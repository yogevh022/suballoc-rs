use crate::block::BlockHead;
use crate::tlsf::{BLOCK_META_SIZE, TLSF, Word};

impl TLSF {
    pub(crate) const fn strip_block_size_meta(size: Word) -> Word {
        size - BLOCK_META_SIZE
    }

    pub(crate) const fn add_block_size_meta(size: Word) -> Word {
        size + BLOCK_META_SIZE
    }

    pub(crate) fn block_ptr_to_offset(&self, block_ptr: *const u8) -> Word {
        unsafe { block_ptr.offset_from(self.mem.as_ptr() as *const _) as Word }
    }

    pub(crate) fn offset_to_block_ptr(&self, offset: Word) -> *mut BlockHead {
        unsafe { self.mem.as_ptr().offset(offset as isize) as *mut BlockHead }
    }
}
