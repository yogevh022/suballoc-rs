use crate::block::{
    BLOCK_ALIGNMENT, BLOCK_META_SIZE, BLOCK_TAIL_SIZE, BitFlags, BlockHead, BlockInterface,
    BlockPtr, BlockTail, FreeBlockHead,
};
use std::fmt::Debug;
use std::ptr::NonNull;

pub type AllocResult<T> = Result<T, AllocError>;
pub type Word = u32;
pub(crate) const WORD_BITS: Word = Word::BITS as Word;
pub(crate) const SLI_SIZE: usize = 8;
pub(crate) const SLI_BITS: Word = SLI_SIZE.trailing_zeros() as Word;

#[derive(Debug, Clone, Copy)]
pub enum AllocError {
    OutOfMemory,
    InvalidAllocation,
}

pub struct SubAllocator {
    pub capacity: Word,
    pub mem: Box<[u8]>, // fixme not pub
    pub fl_bitmap: Word,
    pub sl_bitmaps: [Word; WORD_BITS as usize],
    pub free_blocks: [[Option<NonNull<FreeBlockHead>>; WORD_BITS as usize]; WORD_BITS as usize],
}

impl SubAllocator {
    pub fn new(capacity: Word) -> Self {
        assert!(capacity > 0);
        assert_eq!(capacity % 8, 0);
        let mem = Self::init_mem(capacity);
        let mut tlsf_instance = Self {
            capacity: Self::strip_meta(mem.len() as Word),
            mem,
            fl_bitmap: 0,
            sl_bitmaps: [0; WORD_BITS as usize],
            free_blocks: std::array::from_fn(|_| std::array::from_fn(|_| None)),
        };
        tlsf_instance.init_bitmaps();
        tlsf_instance
    }

    fn init_mem(capacity: Word) -> Box<[u8]> {
        let mut mem = vec![0u8; capacity as usize].into_boxed_slice();
        let mem_ptr = mem.as_mut_ptr();
        let user_size = Self::strip_meta(capacity);

        let size_flags = user_size | BitFlags::PREV_USED | BitFlags::NEXT_USED;
        unsafe {
            let initial_head = (mem_ptr as *mut FreeBlockHead).deref_mut();
            initial_head.set_size_flags(size_flags);
            initial_head.set_prev_link(None);
            initial_head.set_next_link(None);
        }
        let mem_tail = mem_ptr.wrapping_add(capacity as usize - size_of::<BlockTail>());
        unsafe {
            let initial_tail = (mem_tail as *mut BlockTail).deref_mut();
            initial_tail.set_size_flags(size_flags);
        }
        mem
    }

    fn init_bitmaps(&mut self) {
        // this function assumes that the memory is already initialized with initial free block
        let head_ptr = unsafe { NonNull::new_unchecked(self.mem.as_ptr() as *mut FreeBlockHead) };
        self.pushf_free_link(head_ptr);
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

    fn pushf_free_link(&mut self, block_head_ptr: NonNull<FreeBlockHead>) {
        let block_head = unsafe { block_head_ptr.as_ptr().deref_mut() };
        let (fli, sli) = self.mapping_insert(block_head.size());

        let slot = &mut self.free_blocks[fli as usize][sli as usize];
        let last_head_opt = std::mem::replace(slot, Some(block_head_ptr));
        if let Some(last_head) = last_head_opt {
            unsafe {
                (*last_head.as_ptr()).set_prev_link(Some(block_head_ptr));
            }
        }
        block_head.set_next_link(last_head_opt);
        block_head.set_prev_link(None);

        self.set_bitmap_index_available(fli, sli);
    }

    fn popf_free_link(&mut self, fli: Word, sli: Word) -> NonNull<FreeBlockHead> {
        let slot = &mut self.free_blocks[fli as usize][sli as usize];
        let block_head_ptr = slot.take().unwrap();

        *slot = unsafe { (*block_head_ptr.as_ptr()).next_link() };
        if let Some(next) = slot {
            unsafe { (*next.as_ptr()).set_prev_link(None) };
        } else {
            self.set_bitmap_index_empty(fli, sli);
        }

        block_head_ptr
    }

    fn remove_free_link(&mut self, fli: Word, sli: Word, head: *mut FreeBlockHead) {
        let head = unsafe { head.deref_mut() };

        if let Some(next) = head.next_link() {
            unsafe { (*next.as_ptr()).set_prev_link(head.prev_link()) };
        }
        if let Some(prev) = head.prev_link() {
            unsafe { (*prev.as_ptr()).set_next_link(head.next_link()) };
        }

        let slot = unsafe {
            self.free_blocks
                .get_unchecked_mut(fli as usize)
                .get_unchecked_mut(sli as usize)
        };
        if slot.map_or(false, |x| x.as_ptr() == head) {
            *slot = head.next_link();
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
        let available_fl_mask = self.fl_bitmap & Self::left_mask_from(fl_idx);
        if available_fl_mask == 0 {
            return Err(AllocError::OutOfMemory);
        }

        #[inline(always)]
        fn find_sl_for_fl(this: &SubAllocator, fl_idx: Word, size: Word) -> Option<Word> {
            let sl_idx = SubAllocator::calc_sl_index_for_fl(size, fl_idx);
            let available_sl_mask =
                this.sl_bitmaps[fl_idx as usize] & SubAllocator::left_mask_from(sl_idx);
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

        let higher_fl_mask = self.fl_bitmap & Self::left_mask_from(fl_idx + 1);
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
        leftover_tail_ptr: *mut BlockTail,
        leftover_total_size: Word,
    ) {
        let leftover_use_size = Self::strip_meta(leftover_total_size);
        let size_flags = leftover_use_size | BitFlags::PREV_USED | BitFlags::NEXT_USED;

        unsafe {
            let leftover_head_ptr = Self::head_from_tail_ptr(leftover_tail_ptr, leftover_use_size);
            leftover_head_ptr.deref_mut().set_size_flags(size_flags);
            leftover_tail_ptr.deref_mut().set_size_flags(size_flags);

            self.pushf_free_link(NonNull::new_unchecked(leftover_head_ptr as _));
        }
    }

    fn set_next_prev_used(&mut self, head_ptr: *mut FreeBlockHead, block_size: Word) {
        if self.is_block_last(head_ptr as _, block_size) {
            return;
        }
        let (next_head, next_tail) = unsafe { Self::next_block_meta(head_ptr, block_size) };
        next_head.or_flags(BitFlags::PREV_USED);
        next_tail.or_flags(BitFlags::PREV_USED);
    }

    fn set_prev_next_used(&mut self, head_ptr: *mut FreeBlockHead) {
        if self.is_block_first(head_ptr as _) {
            return;
        }
        let (prev_head, prev_tail) = unsafe { Self::prev_block_meta(head_ptr) };
        prev_head.or_flags(BitFlags::NEXT_USED);
        prev_tail.or_flags(BitFlags::NEXT_USED);
    }

    fn set_block_used(&mut self, head_ptr: *mut FreeBlockHead, used_size: Word) {
        let block_size = unsafe { (*head_ptr).size() };
        let leftover_total_size = block_size - used_size;
        let initial_tail_ptr = Self::tail_from_head_ptr(head_ptr as _, block_size);

        let (head, tail, size_flags) =
            if leftover_total_size <= Self::align_up(BLOCK_META_SIZE + 1, BLOCK_ALIGNMENT) {
                self.set_next_prev_used(head_ptr, block_size);
                (
                    unsafe { head_ptr.deref_mut() },
                    unsafe { initial_tail_ptr.deref_mut() },
                    block_size | BitFlags::USED | BitFlags::PREV_USED | BitFlags::NEXT_USED,
                )
            } else {
                self.push_leftover_block(initial_tail_ptr, leftover_total_size);
                let tail_ptr = Self::tail_from_head_ptr(head_ptr as _, used_size);
                (
                    unsafe { head_ptr.deref_mut() },
                    unsafe { tail_ptr.deref_mut() },
                    used_size | BitFlags::USED | BitFlags::PREV_USED,
                )
            };
        head.set_size_flags(size_flags);
        tail.set_size_flags(size_flags);
        self.set_prev_next_used(head_ptr);
    }

    pub fn allocate(&mut self, size: Word) -> AllocResult<Word> {
        let aligned_size = Self::align_up(size, BLOCK_ALIGNMENT);
        let (fli, sli) = self.mapping_search(aligned_size)?;
        let block_head_ptr = self.popf_free_link(fli, sli).as_ptr();
        self.set_block_used(block_head_ptr, aligned_size);
        Ok(self.offset_from_ptr(block_head_ptr))
    }

    fn coalesce_next(
        &mut self,
        head_ptr: *mut BlockHead,
        tail_ptr: *mut BlockTail,
        head: &mut BlockHead,
        head_size: Word,
    ) -> *mut BlockTail {
        let next_head_ptr = unsafe {
            let next_head_offset = Self::with_meta(head_size) as _;
            head_ptr.block_add::<BlockHead>(next_head_offset)
        };
        let next_head = unsafe { next_head_ptr.deref_mut() };
        let next_head_size = next_head.size();
        let next_tail_ptr = Self::tail_from_head_ptr(next_head_ptr, next_head_size);

        match head.next_used() {
            true => {
                next_head.clear_or_flags(BitFlags::PREV_USED);
                let next_tail = unsafe { next_tail_ptr.deref_mut() };
                next_tail.clear_or_flags(BitFlags::PREV_USED);
                tail_ptr
            }
            false => {
                let (fli, sli) = self.mapping_insert(next_head_size);
                self.remove_free_link(fli, sli, next_head_ptr as _);
                next_tail_ptr
            }
        }
    }

    fn coalesce_prev(&mut self, head_ptr: *mut BlockHead, head: &mut BlockHead) -> *mut BlockHead {
        let prev_tail_ptr = unsafe { head_ptr.block_sub::<BlockTail>(BLOCK_TAIL_SIZE as usize) };
        let prev_tail = unsafe { prev_tail_ptr.deref_mut() };
        let prev_size = prev_tail.size();
        let prev_head_ptr = Self::head_from_tail_ptr(prev_tail_ptr, prev_size);

        match head.prev_used() {
            true => {
                prev_tail.clear_or_flags(BitFlags::NEXT_USED);
                let prev_head = unsafe { prev_head_ptr.deref_mut() };
                prev_head.clear_or_flags(BitFlags::NEXT_USED);
                head_ptr
            }
            false => {
                let (fli, sli) = self.mapping_insert(prev_size);
                self.remove_free_link(fli, sli, prev_head_ptr as _);
                prev_head_ptr
            }
        }
    }

    pub fn deallocate(&mut self, addr: Word) -> AllocResult<()> {
        let head_ptr: *mut BlockHead = self.ptr_from_offset(addr);
        let head = unsafe { &mut *(head_ptr) };
        debug_assert!(head.flags() & BitFlags::USED == BitFlags::USED);

        let head_size = head.size();
        let tail_ptr = Self::tail_from_head_ptr(head_ptr, head_size);
        let coalesced_tail_ptr = match self.is_block_last(head_ptr, head_size) {
            true => tail_ptr,
            false => self.coalesce_next(head_ptr, tail_ptr, head, head_size),
        };

        let coalesced_head_ptr = match self.is_block_first(head_ptr) {
            true => head_ptr,
            false => self.coalesce_prev(head_ptr, head),
        };

        let coalesced_size = Self::size_between_meta_ptrs(coalesced_head_ptr, coalesced_tail_ptr);
        let size_flags = coalesced_size | BitFlags::PREV_USED | BitFlags::NEXT_USED;
        unsafe {
            let coalesced_head = coalesced_head_ptr.deref_mut();
            coalesced_head.set_size_flags(size_flags);
            let coalesced_tail = coalesced_tail_ptr.deref_mut();
            coalesced_tail.set_size_flags(size_flags);
        }
        self.pushf_free_link(unsafe { NonNull::new_unchecked(coalesced_head_ptr as _) });

        Ok(())
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
