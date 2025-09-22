use crate::block::{BLOCK_HEAD_SIZE, BLOCK_META_SIZE, BLOCK_TAIL_SIZE, BlockHead};
use crate::tlsf::{SubAllocator, Word};

impl SubAllocator {
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

    pub(crate) fn offset_from_ptr<T>(&self, ptr: *const T) -> Word {
        let ptr = ptr as *const u8;
        unsafe { ptr.offset_from(self.mem.as_ptr()) as Word }
    }

    pub(crate) fn ptr_from_offset<T>(&self, offset: Word) -> *mut T {
        unsafe { self.mem.as_ptr().offset(offset as isize) as *mut T }
    }

    pub(crate) fn ptr_eq_mem_start<T>(&self, ptr: *mut T) -> bool {
        ptr as *const _ == self.mem.as_ptr()
    }

    pub(crate) fn ptr_eq_mem_end<T>(&self, ptr: *mut T) -> bool {
        unsafe {
            ptr as *const _ == self.mem.as_ptr().add(self.mem.len())
        }
    }

    pub(crate) fn left_mask_from(index: Word) -> Word {
        Word::MAX << index
    }

    pub(crate) fn align_up(x: Word, align: Word) -> Word {
        (x + align - 1) & !(align - 1)
    }
}
