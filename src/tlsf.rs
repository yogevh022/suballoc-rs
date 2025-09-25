use crate::block::{
    BLOCK_ALIGNMENT, BLOCK_META_SIZE, BLOCK_TAIL_SIZE, BitFlags, BlockHead, BlockHeadPtrInterface,
    BlockInterface, BlockTail, BlockTailPtrInterface, PACKED_NONE_DOUBLE_PTR, PACKED_NONE_PTR,
};
use crate::meta::{
    align_up, byte_add_into, byte_sub_into, left_mask_from, size_between_meta_ptrs, strip_meta,
    with_meta,
};
use std::fmt::Debug;

pub type AllocResult<T> = Result<T, AllocError>;
pub type Word = u32; // 64bit would require adjusting links to be 64bit
pub(crate) const WORD_BITS: Word = Word::BITS as Word;
pub(crate) const SLI_SIZE: usize = 8;
pub(crate) const SLI_BITS: Word = SLI_SIZE.trailing_zeros() as Word;

#[derive(Debug, Clone, Copy)]
pub enum AllocError {
    OutOfMemory,
    InvalidAllocation,
}

pub struct SubAllocator {
    capacity: Word,
    pub(crate) mem: Box<[u8]>,
    fl_bitmap: Word,
    sl_bitmaps: [Word; WORD_BITS as usize],
    free_blocks: [[Option<*mut BlockHead>; WORD_BITS as usize]; WORD_BITS as usize],
}

impl SubAllocator {
    pub fn new(capacity: Word) -> Self {
        assert_ne!(capacity, 0);
        assert_eq!(capacity % 8, 0);
        let mem = Self::init_mem(capacity);
        let mut instance = Self {
            capacity: strip_meta(mem.len() as Word),
            mem,
            fl_bitmap: 0,
            sl_bitmaps: [0; WORD_BITS as usize],
            free_blocks: std::array::from_fn(|_| std::array::from_fn(|_| None)),
        };
        instance.pushf_free_link(instance.mem.as_ptr() as _);
        instance
    }

    fn init_mem(capacity: Word) -> Box<[u8]> {
        let mem = vec![0u8; capacity as usize].into_boxed_slice();
        let user_size = strip_meta(capacity);

        let mut head_ptr = mem.as_ptr() as *mut BlockHead;
        let head = head_ptr.deref();
        let tail = head_ptr.tail_ptr(user_size).deref();

        let size_flags = user_size | BitFlags::PREV_USED | BitFlags::NEXT_USED;
        head.set_size_flags(size_flags);
        head.as_free().set_links(PACKED_NONE_DOUBLE_PTR);
        tail.set_size_flags(size_flags);

        mem
    }

    fn set_bitmap_index_available(&mut self, fli: Word, sli: Word) {
        let fl_mask = 1 << fli;
        self.fl_bitmap |= fl_mask;

        let sl_idx = sli as usize;
        let sl_mask = 1 << sl_idx;
        self.sl_bitmaps[fli as usize] |= sl_mask;
    }

    fn set_bitmap_index_empty(&mut self, fli: Word, sli: Word) {
        let sl_idx = sli as usize;
        let sl_mask = 1 << sl_idx;
        self.sl_bitmaps[fli as usize] &= !sl_mask;

        if self.sl_bitmaps[fli as usize] == 0 {
            let fl_mask = 1 << fli;
            self.fl_bitmap &= !fl_mask;
        }
    }

    fn pushf_free_link(&mut self, mut head_ptr: *mut BlockHead) {
        let head = head_ptr.deref();
        let (fli, sli) = self.mapping_insert(head.size());
        let head_free = head.as_free();

        let slot = &mut self.free_blocks[fli as usize][sli as usize];
        let last_head_opt = std::mem::replace(slot, Some(head_ptr));

        match last_head_opt {
            Some(mut last_head_ptr) => {
                // pack links
                let packed_block_head_ptr = self.mem_offset_from_ptr(head_ptr);
                let packed_last_head_ptr = self.mem_offset_from_ptr(last_head_ptr) as u64;
                let last_head_free = last_head_ptr.deref().as_free();
                last_head_free.set_prev_link(packed_block_head_ptr);
                head_free.set_links(
                    ((PACKED_NONE_PTR as u64) << (WORD_BITS as u64)) | packed_last_head_ptr,
                );
            }
            None => head_free.set_links(PACKED_NONE_DOUBLE_PTR),
        }
        self.set_bitmap_index_available(fli, sli);
    }

    fn popf_free_link(&mut self, fli: Word, sli: Word) -> *mut BlockHead {
        let slot_ptr: *mut Option<*mut BlockHead> =
            &mut self.free_blocks[fli as usize][sli as usize] as *mut _;
        let mut block_head_ptr = unsafe { (*slot_ptr).take().unwrap() };

        // unpack and set the next link as head
        let (_, next_link_offset) = block_head_ptr.deref().as_free().link_offsets();
        let next_link = self.ptr_from_mem_offset::<BlockHead>(next_link_offset);
        unsafe { *slot_ptr = next_link };

        if let Some(mut next) = next_link {
            next.deref().as_free().set_prev_link(PACKED_NONE_PTR);
        } else {
            self.set_bitmap_index_empty(fli, sli);
        }

        block_head_ptr
    }

    fn remove_free_link(&mut self, fli: Word, sli: Word, head: &mut BlockHead) {
        // unpack links
        let (prev_link_offset, next_link_offset) = head.as_free().link_offsets();
        let prev_link_opt = self.ptr_from_mem_offset::<BlockHead>(prev_link_offset);
        let next_link_opt = self.ptr_from_mem_offset::<BlockHead>(next_link_offset);

        // remove head from linked list
        if let Some(mut next) = next_link_opt {
            next.deref().as_free().set_prev_link(prev_link_offset);
        }
        if let Some(mut prev) = prev_link_opt {
            prev.deref().as_free().set_next_link(next_link_offset);
        }

        let slot = unsafe {
            self.free_blocks
                .get_unchecked_mut(fli as usize)
                .get_unchecked_mut(sli as usize)
        };
        if slot.map_or(false, |x| x == head) {
            *slot = next_link_opt;
        }
        if slot.is_none() {
            self.set_bitmap_index_empty(fli, sli);
        }
    }

    fn calc_sl_index_for_fl(size: Word, fl: Word) -> Word {
        let base = 1 << fl;
        let offset = size - base;
        (offset << SLI_BITS) >> fl
    }

    fn mapping_search(&self, size: Word) -> AllocResult<(Word, Word)> {
        let fl_idx = (WORD_BITS - 1) - size.leading_zeros() as Word;
        let available_fl_mask = self.fl_bitmap & left_mask_from(fl_idx);
        if available_fl_mask == 0 {
            return Err(AllocError::OutOfMemory);
        }

        #[inline(always)]
        fn find_sl_for_fl(this: &SubAllocator, fl_idx: Word, size: Word) -> Option<Word> {
            let sl_idx = SubAllocator::calc_sl_index_for_fl(size, fl_idx);
            let available_sl_mask = this.sl_bitmaps[fl_idx as usize] & left_mask_from(sl_idx + 1);
            if available_sl_mask != 0 {
                let first_sl = available_sl_mask.trailing_zeros() as Word;
                return Some(first_sl);
            }
            None
        }

        let first_fl = available_fl_mask.trailing_zeros() as Word;
        if first_fl == fl_idx {
            if let Some(first_sl) = find_sl_for_fl(self, first_fl, size) {
                return Ok((first_fl, first_sl));
            }
        }

        let higher_fl_mask = self.fl_bitmap & left_mask_from(fl_idx + 1);
        if higher_fl_mask != 0 {
            let next_fl = higher_fl_mask.trailing_zeros();
            let first_sl = self.sl_bitmaps[next_fl as usize].trailing_zeros() as Word;
            return Ok((next_fl as Word, first_sl));
        }

        Err(AllocError::OutOfMemory)
    }

    fn mapping_insert(&mut self, size: Word) -> (Word, Word) {
        let fl_idx = (WORD_BITS - 1) - size.leading_zeros() as Word;
        let sl_idx = Self::calc_sl_index_for_fl(size, fl_idx);
        (fl_idx, sl_idx)
    }

    fn push_leftover_block(
        &mut self,
        mut leftover_tail_ptr: *mut BlockTail,
        leftover_total_size: Word,
    ) {
        let leftover_use_size = strip_meta(leftover_total_size);
        let mut leftover_head_ptr = leftover_tail_ptr.head_ptr(leftover_use_size);

        let size_flags = leftover_use_size | BitFlags::PREV_USED | BitFlags::NEXT_USED;
        leftover_head_ptr.deref().set_size_flags(size_flags);
        leftover_tail_ptr.deref().set_size_flags(size_flags);

        self.pushf_free_link(leftover_head_ptr as _);
    }

    fn set_next_prev_used(&mut self, head_ptr: *mut BlockHead, block_size: Word) {
        if self.is_block_last(head_ptr as _, block_size) {
            return;
        }
        let (next_head, next_tail) = unsafe { Self::next_block_meta(head_ptr, block_size) };
        next_head.or_flags(BitFlags::PREV_USED);
        next_tail.or_flags(BitFlags::PREV_USED);
    }

    fn set_prev_next_used(&mut self, head_ptr: *mut BlockHead) {
        if self.is_block_first(head_ptr as _) {
            return;
        }
        let (prev_head, prev_tail) = unsafe { Self::prev_block_meta(head_ptr) };
        prev_head.or_flags(BitFlags::NEXT_USED);
        prev_tail.or_flags(BitFlags::NEXT_USED);
    }

    fn set_block_used(&mut self, mut head_ptr: *mut BlockHead, used_size: Word) {
        let head = head_ptr.deref();
        let block_size = head.size();
        let leftover_total_size = block_size - used_size;
        let mut initial_tail_ptr = head_ptr.tail_ptr(block_size);

        let (head, tail, size_flags) =
            if leftover_total_size <= align_up(BLOCK_META_SIZE + 1, BLOCK_ALIGNMENT) {
                self.set_next_prev_used(head_ptr, block_size);
                (
                    head,
                    initial_tail_ptr.deref(),
                    block_size | BitFlags::USED | BitFlags::PREV_USED | BitFlags::NEXT_USED,
                )
            } else {
                self.push_leftover_block(initial_tail_ptr, leftover_total_size);
                let mut tail_ptr = head_ptr.tail_ptr(used_size);
                (
                    head,
                    tail_ptr.deref(),
                    used_size | BitFlags::USED | BitFlags::PREV_USED,
                )
            };
        head.set_size_flags(size_flags);
        tail.set_size_flags(size_flags);
        self.set_prev_next_used(head_ptr);
    }

    pub fn allocate(&mut self, size: Word) -> AllocResult<Word> {
        debug_assert!(size > 0);
        let aligned_size = align_up(size, BLOCK_ALIGNMENT);
        let (fli, sli) = self.mapping_search(aligned_size)?;
        let block_head_ptr = self.popf_free_link(fli, sli);
        self.set_block_used(block_head_ptr, aligned_size);
        Ok(self.mem_offset_from_ptr(block_head_ptr))
    }

    fn coalesce_next(
        &mut self,
        head_ptr: *mut BlockHead,
        tail_ptr: *mut BlockTail,
        head: &mut BlockHead,
        head_size: Word,
    ) -> *mut BlockTail {
        let mut next_head_ptr: *mut BlockHead =
            unsafe { byte_add_into(head_ptr, with_meta(head_size) as _) };
        let next_head = next_head_ptr.deref();
        let next_head_size = next_head.size();
        let mut next_tail_ptr = next_head_ptr.tail_ptr(next_head_size);

        match head.next_used() {
            true => {
                let next_tail = next_tail_ptr.deref();
                next_head.clear_or_flags(BitFlags::PREV_USED);
                next_tail.clear_or_flags(BitFlags::PREV_USED);
                tail_ptr
            }
            false => {
                let (fli, sli) = self.mapping_insert(next_head_size);
                self.remove_free_link(fli, sli, next_head);
                next_tail_ptr
            }
        }
    }

    fn coalesce_prev(&mut self, head_ptr: *mut BlockHead, head: &mut BlockHead) -> *mut BlockHead {
        let mut prev_tail_ptr: *mut BlockTail =
            unsafe { byte_sub_into(head_ptr, BLOCK_TAIL_SIZE as usize) };
        let prev_tail = prev_tail_ptr.deref();
        let prev_size = prev_tail.size();
        let mut prev_head_ptr = prev_tail_ptr.head_ptr(prev_size);
        let prev_head = prev_head_ptr.deref();

        match head.prev_used() {
            true => {
                prev_tail.clear_or_flags(BitFlags::NEXT_USED);
                prev_head.clear_or_flags(BitFlags::NEXT_USED);
                head_ptr
            }
            false => {
                let (fli, sli) = self.mapping_insert(prev_size);
                self.remove_free_link(fli, sli, prev_head);
                prev_head_ptr
            }
        }
    }

    pub fn deallocate(&mut self, addr: Word) -> AllocResult<()> {
        let mut head_ptr: *mut BlockHead = self.ptr_from_mem_offset_unchecked(addr);
        let head = head_ptr.deref();
        debug_assert!(head.flags() & BitFlags::USED == BitFlags::USED);

        dbg!(addr);

        let head_size = head.size();
        let tail_ptr = head_ptr.tail_ptr(head_size);
        let mut coalesced_tail_ptr = match self.is_block_last(head_ptr, head_size) {
            true => tail_ptr,
            false => self.coalesce_next(head_ptr, tail_ptr, head, head_size),
        };

        let mut coalesced_head_ptr = match self.is_block_first(head_ptr) {
            true => head_ptr,
            false => self.coalesce_prev(head_ptr, head),
        };

        let coalesced_size = size_between_meta_ptrs(coalesced_head_ptr, coalesced_tail_ptr);
        let size_flags = coalesced_size | BitFlags::PREV_USED | BitFlags::NEXT_USED;
        coalesced_head_ptr.deref().set_size_flags(size_flags);
        coalesced_tail_ptr.deref().set_size_flags(size_flags);

        self.pushf_free_link(coalesced_head_ptr as _);

        Ok(())
    }

    pub fn capacity(&self) -> Word {
        self.capacity
    }

    pub fn free(&self) -> Word {
        let mut total_free: Word = 0;
        for bin in self.free_blocks.iter().flatten() {
            let mut link = *bin;
            while let Some(mut head_ptr) = link {
                let head = head_ptr.deref();
                total_free += head.size();
                let (_, next_link_offset) = head.as_free().link_offsets();
                let next_link = self.ptr_from_mem_offset::<BlockHead>(next_link_offset);
                link = next_link;
            }
        }
        total_free
    }
}

impl Debug for SubAllocator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (flr, slr) = bitmap_bin_repr(self);
        write!(f, "user cap: {}, FL: {}\n SL: {}", self.capacity, flr, slr)
    }
}

fn bitmap_bin_repr(tlsf: &SubAllocator) -> (String, String) {
    const BIN_WIDTH: usize = WORD_BITS as usize;
    let fl_repr = format!("{:0BIN_WIDTH$b}", tlsf.fl_bitmap);
    let sl_repr = tlsf
        .sl_bitmaps
        .iter()
        .map(|x| format!("{:0BIN_WIDTH$b}", x))
        .collect::<Vec<_>>()
        .join("\n");
    (fl_repr, sl_repr)
}
