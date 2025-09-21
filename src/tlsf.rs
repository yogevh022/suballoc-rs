use crate::block::{
    BlockHead, BlockInterface, BlockTail, FreeBlockHead, FreeBlockInterface, FreeBlockLink,
};
use std::fmt::Debug;
use std::ptr::NonNull;

pub(crate) type AllocResult<T> = Result<T, AllocError>;
pub(crate) type Word = u32;
pub(crate) const ALIGNMENT: Word = 8;
pub(crate) const SLI_SIZE: usize = 8;
pub(crate) const WORD_BITS: Word = Word::BITS as Word;
pub(crate) const USED_BIT_MASK: Word = 0b1;
pub(crate) const PREV_USED_BIT_MASK: Word = 0b10;
pub(crate) const SIZE_MASK: Word = !0b11;
pub(crate) const SLI_BITS: Word = SLI_SIZE.trailing_zeros() as Word;
pub(crate) const BLOCK_META_SIZE: Word = (size_of::<BlockHead>() + size_of::<BlockTail>()) as Word;

#[derive(Debug, Clone, Copy)]
pub enum AllocError {
    OutOfMemory,
    InvalidAllocation,
}

pub struct TLSF {
    pub capacity: Word,

    pub(crate) mem: Box<[u8]>,
    fl_bitmap: Word,
    sl_bitmaps: [Word; WORD_BITS as usize],
    free_blocks: [[Option<NonNull<FreeBlockLink>>; WORD_BITS as usize]; WORD_BITS as usize],
}

impl TLSF {
    pub fn new(capacity: Word) -> Self {
        assert!(capacity > 0);
        assert_eq!(capacity % 8, 0);
        let mem = Self::init_mem(capacity);
        let mut tlsf_instance = Self {
            capacity: Self::strip_block_size_meta(mem.len() as Word),
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
        let user_size = Self::strip_block_size_meta(capacity);
        unsafe {
            (*(mem_ptr as *mut BlockHead)).set_size(user_size);
        }
        let mem_tail = mem_ptr.wrapping_add(capacity as usize - size_of::<BlockTail>());
        unsafe {
            (*(mem_tail as *mut BlockTail)).set_size(user_size);
        }
        mem
    }

    fn init_bitmaps(&mut self) {
        // this function assumes that the memory is already initialized with initial free block
        let head_ptr = unsafe { NonNull::new_unchecked(self.mem.as_ptr() as *mut FreeBlockHead) };
        self.pushf_free_link(head_ptr);
    }

    pub(crate) fn calc_sl_index_for_fl(size: Word, fl: Word) -> Word {
        let base = 1 << fl;
        let offset = size - base;
        (offset << SLI_BITS) >> fl
    }

    fn left_mask_from(index: Word) -> Word {
        Word::MAX << index
    }


    fn set_bitmap_index_used(&mut self, fli: Word, sli: Word) {
        let fl_mask = 1 << fli;
        self.fl_bitmap |= fl_mask;

        let sl_idx = sli as usize;
        let sl_mask = 1 << sl_idx;
        self.sl_bitmaps[fli as usize] |= sl_mask;
    }

    fn set_bitmap_index_free(&mut self, fli: Word, sli: Word) {
        let sl_idx = sli as usize;
        let sl_mask = 1 << sl_idx;
        self.sl_bitmaps[fli as usize] &= !sl_mask;

        if self.sl_bitmaps[fli as usize] == 0 {
            let fl_mask = 1 << fli;
            self.fl_bitmap &= !fl_mask;
        }
    }

    fn pushf_free_link(&mut self, block_head: NonNull<FreeBlockHead>) {
        let size = unsafe { block_head.as_ref().size() };
        let (fli, sli) = self.mapping_insert(size);

        let slot = &mut self.free_blocks[fli as usize][sli as usize];

        let free_link_ptr = unsafe {
            let ptr = Box::into_raw(Box::new(FreeBlockLink {
                head: block_head,
                prev: None,
                next: slot.take(),
            }));
            NonNull::new_unchecked(ptr)
        };
        *slot = Some(free_link_ptr);

        unsafe {
            (*block_head.as_ptr()).set_link(Some(free_link_ptr));
        }

        self.set_bitmap_index_used(fli, sli);
    }

    fn popf_free_link(&mut self, fli: Word, sli: Word) -> NonNull<FreeBlockHead> {
        let slot = &mut self.free_blocks[fli as usize][sli as usize];
        let link = slot.take().unwrap();

        let head = unsafe { (*link.as_ptr()).head };
        let next = unsafe { (*link.as_ptr()).next };
        *slot = next;

        if slot.is_none() {
            self.set_bitmap_index_free(fli, sli);
        }
        unsafe {
            let _ = Box::from_raw(link.as_ptr()); // mark for dropping
        }
        head
    }

    fn mapping_search(&self, size: Word) -> AllocResult<(Word, Word)> {
        let fl_idx = (WORD_BITS - 1) - size.leading_zeros() as Word;
        let available_fl_mask = self.fl_bitmap & Self::left_mask_from(fl_idx);
        if available_fl_mask == 0 {
            return Err(AllocError::OutOfMemory);
        }

        #[inline]
        fn find_sl_for_fl(this: &TLSF, fl_idx: Word, size: Word) -> Option<Word> {
            let sl_idx = TLSF::calc_sl_index_for_fl(size, fl_idx);
            let sl_mask = this.sl_bitmaps[fl_idx as usize] & TLSF::left_mask_from(sl_idx);
            if sl_mask != 0 {
                return Some(sl_idx);
            }
            None
        }

        let first_fl = available_fl_mask.trailing_zeros() as Word;

        if first_fl == fl_idx {
            if let Some(sl_idx) = find_sl_for_fl(self, first_fl, size) {
                return Ok((first_fl, sl_idx));
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

    fn part_leftover_block(
        &self,
        block_ptr: *mut BlockHead,
        total_leftover: Word,
    ) -> *mut FreeBlockHead {
        let leftover_use_size = Self::strip_block_size_meta(total_leftover);

        let leftover_tail_ptr = Self::tail_from_head(block_ptr);
        unsafe {
            let leftover_tail = &mut (*leftover_tail_ptr);
            leftover_tail.set_size(leftover_use_size);
            leftover_tail.set_free();
            leftover_tail.set_prev_used();
        }

        let leftover_head_ptr = Self::head_from_tail(leftover_tail_ptr) as *mut FreeBlockHead;
        unsafe {
            let leftover_head = &mut (*leftover_head_ptr);
            leftover_head.set_size(leftover_use_size);
            leftover_head.set_free();
            leftover_head.set_prev_used();
        }
        leftover_head_ptr
    }

    fn use_entire_block(&mut self, block_ptr: *mut BlockHead) {
        unsafe {
            if !self.block_is_last(block_ptr) {
                let next_head = Self::next_block_head(block_ptr);
                (*next_head).set_prev_used();
            }
            (*block_ptr).set_used();
        }
    }

    fn use_part_of_block(
        &mut self,
        block_ptr: *mut BlockHead,
        used_size: Word,
        leftover_total_size: Word,
    ) {
        let leftover_head = self.part_leftover_block(block_ptr, leftover_total_size);
        self.pushf_free_link(unsafe { NonNull::new_unchecked(leftover_head) });

        let used_tail_ptr =
            unsafe { block_ptr.byte_add(size_of::<BlockHead>() + used_size as usize) };

        unsafe {
            let used_head = &mut (*block_ptr);
            used_head.set_size(used_size);
            used_head.set_prev_used();
            used_head.set_used();

            let used_tail = &mut (*used_tail_ptr);
            used_tail.set_size(used_size);
            used_tail.set_prev_used();
            used_tail.set_used();
        }
    }

    fn use_block(&mut self, block_ptr: *mut BlockHead, used_size: Word) {
        let block_size = unsafe { (*block_ptr).size() };

        let leftover_total_size = block_size - used_size;
        if leftover_total_size < align_up(BLOCK_META_SIZE + 1, ALIGNMENT) {
            // if the leftover size is less= than block meta (min block size), include leftover.
            self.use_entire_block(block_ptr);
        } else {
            // add leftover block metadata to mem and to free list.
            self.use_part_of_block(block_ptr, used_size, leftover_total_size);
        }
    }

    pub fn allocate(&mut self, size: Word) -> AllocResult<Word> {
        let aligned_size = align_up(size, ALIGNMENT);
        let (fli, sli) = self.mapping_search(aligned_size)?;
        let block_head = self.popf_free_link(fli, sli);
        let block_ptr = block_head.as_ptr() as *mut BlockHead;
        self.use_block(block_ptr, aligned_size);
        Ok(self.block_ptr_to_offset(block_ptr as *const u8))
    }
}

impl Debug for TLSF {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (flr, slr) = bitmap_bin_repr(self);
        write!(f, "user cap: {}, FL: {}\n SL: {}", self.capacity, flr, slr)
    }
}

fn bitmap_bin_repr(tlsf: &TLSF) -> (String, String) {
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

fn align_up(x: Word, align: Word) -> Word {
    (x + align - 1) & !(align - 1)
}
