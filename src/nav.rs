use crate::block::{BlockHead, BlockInterface, BlockTail};
use crate::tlsf::TLSF;

impl TLSF {
    pub(crate) fn block_is_first(&self, block_ptr: *mut BlockHead) -> bool {
        block_ptr as *const _ == self.mem.as_ptr()
    }

    pub(crate) fn block_is_last(&self, block_ptr: *mut BlockHead) -> bool {
        let block_tail_end_ptr = unsafe {
            let block_size = (*block_ptr).size();
            block_ptr.byte_add(Self::add_block_size_meta(block_size) as usize) as *const _
        };
        let mem_end_ptr = unsafe {
            self.mem
                .as_ptr()
                .byte_add(Self::add_block_size_meta(self.capacity) as usize)
        };
        block_tail_end_ptr == mem_end_ptr
    }

    pub(crate) fn head_from_tail(block_tail_ptr: *mut BlockTail) -> *mut BlockHead {
        unsafe {
            let block_size = (*block_tail_ptr).size();
            block_tail_ptr.byte_sub(size_of::<BlockHead>() + block_size as usize) as *mut BlockHead
        }
    }

    pub(crate) fn tail_from_head(block_head_ptr: *mut BlockHead) -> *mut BlockTail {
        unsafe {
            let block_size = (*block_head_ptr).size();
            block_head_ptr.byte_add(size_of::<BlockHead>() + block_size as usize) as *mut BlockTail
        }
    }

    pub(crate) unsafe fn next_block_head(block_ptr: *mut BlockHead) -> *mut BlockHead {
        unsafe {
            let block_size = (*block_ptr).size();
            block_ptr.byte_add(Self::add_block_size_meta(block_size) as usize)
        }
    }

    pub(crate) unsafe fn prev_block_tail(block_ptr: *mut BlockHead) -> *mut BlockTail {
        unsafe { block_ptr.byte_sub(size_of::<BlockTail>()) as *mut BlockTail }
    }
}
