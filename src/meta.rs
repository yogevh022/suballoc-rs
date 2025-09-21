use crate::tlsf::{Word, BLOCK_META_SIZE, TLSF};

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
}