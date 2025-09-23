use crate::block::{
    BLOCK_HEAD_SIZE, BLOCK_META_SIZE, BLOCK_TAIL_SIZE, BitFlags, BlockHead, BlockInterface,
    BlockPtr, BlockTail, FreeBlockHead,
};
use crate::tlsf::{SubAllocator, Word};

impl SubAllocator {
    pub(crate) fn tail_from_head_ptr(head_ptr: *mut BlockHead, block_size: Word) -> *mut BlockTail {
        let tail_offset = Self::with_head(block_size) as _;
        unsafe { head_ptr.block_add::<BlockTail>(tail_offset) }
    }

    pub(crate) fn head_from_tail_ptr(tail_ptr: *mut BlockTail, block_size: Word) -> *mut BlockHead {
        let head_offset = Self::with_head(block_size) as _;
        unsafe { tail_ptr.block_sub::<BlockHead>(head_offset) }
    }

    pub(crate) unsafe fn next_block_meta<'a>(
        head_ptr: *mut FreeBlockHead,
        block_size: Word,
    ) -> (&'a mut BlockHead, &'a mut BlockTail) {
        let next_head_ptr = unsafe {
            let next_head_offset = Self::with_meta(block_size) as _;
            head_ptr.block_add::<BlockHead>(next_head_offset)
        };
        let next_head = unsafe { next_head_ptr.deref_mut() };
        let next_size = next_head.size();
        let next_tail_ptr = Self::tail_from_head_ptr(next_head_ptr, next_size);
        unsafe { (next_head_ptr.deref_mut(), next_tail_ptr.deref_mut()) }
    }

    pub(crate) unsafe fn prev_block_meta<'a>(
        head_ptr: *mut FreeBlockHead,
    ) -> (&'a mut BlockHead, &'a mut BlockTail) {
        unsafe {
            let prev_tail_ptr = head_ptr.block_sub::<BlockTail>(BLOCK_TAIL_SIZE as _);
            let prev_tail = prev_tail_ptr.deref_mut();
            let prev_head_ptr = Self::head_from_tail_ptr(prev_tail_ptr, prev_tail.size());
            (prev_head_ptr.deref_mut(), prev_tail)
        }
    }

    fn ptr_eq_mem_start<T>(&self, ptr: *mut T) -> bool {
        ptr as *const _ == self.mem.as_ptr()
    }

    fn ptr_eq_mem_end<T>(&self, ptr: *mut T) -> bool {
        unsafe { ptr as *const _ == self.mem.as_ptr().add(self.mem.len()) }
    }

    pub(crate) fn is_block_last(&self, head_ptr: *mut BlockHead, block_size: Word) -> bool {
        let block_end_ptr = unsafe {
            let block_end_offset = Self::with_meta(block_size) as _;
            head_ptr.block_add::<u8>(block_end_offset)
        };
        self.ptr_eq_mem_end(block_end_ptr)
    }

    pub(crate) fn is_block_first(&self, head_ptr: *mut BlockHead) -> bool {
        self.ptr_eq_mem_start(head_ptr)
    }

    pub(crate) const fn strip_meta(size: Word) -> Word {
        size - BLOCK_META_SIZE
    }

    pub(crate) const fn with_meta(size: Word) -> Word {
        size + BLOCK_META_SIZE
    }

    pub(crate) const fn with_head(size: Word) -> Word {
        size + BLOCK_HEAD_SIZE
    }

    pub(crate) const fn with_tail(size: Word) -> Word {
        size + BLOCK_TAIL_SIZE
    }

    pub(crate) fn size_between_meta_ptrs<H, T>(head_ptr: *const H, tail_ptr: *const T) -> Word {
        unsafe { tail_ptr.byte_offset_from(head_ptr) as Word - BLOCK_HEAD_SIZE }
    }

    pub(crate) fn mem_offset_from_ptr<T>(&self, ptr: *const T) -> Word {
        let ptr = ptr as *const u8;
        unsafe { ptr.offset_from(self.mem.as_ptr()) as Word }
    }

    pub(crate) fn ptr_from_mem_offset_unchecked<T>(&self, offset: Word) -> *mut T {
        unsafe { self.mem.as_ptr().offset(offset as isize) as *mut T }
    }

    pub(crate) fn ptr_from_mem_offset<T>(&self, ptr_offset: Word) -> Option<*mut T> {
        match ptr_offset {
            Word::MAX => None,
            _ => unsafe { Some(self.mem.as_ptr().byte_add(ptr_offset as usize) as _) },
        }
    }

    pub(crate) fn left_mask_from(index: Word) -> Word {
        Word::MAX << index
    }

    pub(crate) fn align_up(x: Word, align: Word) -> Word {
        (x + align - 1) & !(align - 1)
    }
}
