use crate::block::{
    BLOCK_HEAD_SIZE, BLOCK_META_SIZE, BLOCK_TAIL_SIZE, BlockHead, BlockHeadPtrInterface,
    BlockInterface, BlockTail, BlockTailPtrInterface, PACKED_NONE_PTR,
};
use crate::tlsf::{SubAllocator, Word};

impl SubAllocator {
    pub(crate) unsafe fn next_block_meta<'a>(
        head_ptr: *mut BlockHead,
        block_size: Word,
    ) -> (&'a mut BlockHead, &'a mut BlockTail) {
        let mut next_head_ptr: *mut BlockHead =
            unsafe { byte_add_into(head_ptr, with_meta(block_size) as _) };
        let next_head = next_head_ptr.deref();
        let next_size = next_head.size();
        let mut next_tail_ptr = next_head_ptr.tail_ptr(next_size);
        (next_head_ptr.deref(), next_tail_ptr.deref())
    }

    pub(crate) unsafe fn prev_block_meta<'a>(
        head_ptr: *mut BlockHead,
    ) -> (&'a mut BlockHead, &'a mut BlockTail) {
        let mut prev_tail_ptr: *mut BlockTail =
            unsafe { byte_sub_into(head_ptr, BLOCK_TAIL_SIZE as _) };
        let prev_tail = prev_tail_ptr.deref();
        let mut prev_head_ptr = prev_tail_ptr.head_ptr(prev_tail.size());
        (prev_head_ptr.deref(), prev_tail)
    }

    fn ptr_eq_mem_start<T>(&self, ptr: *mut T) -> bool {
        ptr as *const _ == self.mem.as_ptr()
    }

    fn ptr_eq_mem_end<T>(&self, ptr: *mut T) -> bool {
        unsafe { ptr as *const _ == self.mem.as_ptr().add(self.mem.len()) }
    }

    pub(crate) fn is_block_last(&self, head_ptr: *mut BlockHead, block_size: Word) -> bool {
        let block_end_ptr: *mut u8 = unsafe { byte_add_into(head_ptr, with_meta(block_size) as _) };
        self.ptr_eq_mem_end(block_end_ptr)
    }

    pub(crate) fn is_block_first(&self, head_ptr: *mut BlockHead) -> bool {
        self.ptr_eq_mem_start(head_ptr)
    }

    pub(crate) fn mem_offset_from_ptr<T>(&self, ptr: *const T) -> Word {
        (ptr as u64 - self.mem.as_ptr() as u64) as Word
    }

    pub(crate) fn ptr_from_mem_offset<T>(&self, ptr_offset: Word) -> Option<*mut T> {
        match ptr_offset {
            PACKED_NONE_PTR => None,
            _ => unsafe { Some(self.mem.as_ptr().byte_add(ptr_offset as usize) as _) },
        }
    }

    pub(crate) fn ptr_from_mem_offset_unchecked<T>(&self, offset: Word) -> *mut T {
        unsafe { self.mem.as_ptr().offset(offset as isize) as *mut T }
    }
}

pub(crate) fn left_mask_from(index: Word) -> Word {
    Word::MAX << index
}

pub(crate) fn align_up(x: Word, align: Word) -> Word {
    (x + align - 1) & !(align - 1)
}

pub(crate) const unsafe fn byte_add_into<B, R>(block_ptr: *const B, offset: usize) -> *mut R {
    unsafe { block_ptr.byte_add(offset) as *mut R }
}
pub(crate) const unsafe fn byte_sub_into<B, R>(block_ptr: *const B, offset: usize) -> *mut R {
    unsafe { block_ptr.byte_sub(offset) as *mut R }
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

pub(crate) const fn size_between_meta_ptrs(
    head_ptr: *const BlockHead,
    tail_ptr: *const BlockTail,
) -> Word {
    unsafe { tail_ptr.byte_offset_from(head_ptr) as Word - BLOCK_HEAD_SIZE }
}
