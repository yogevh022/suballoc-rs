use crate::tlsf::{NEXT_USED_BIT_MASK, PREV_USED_BIT_MASK, SIZE_MASK, USED_BIT_MASK, Word};
use std::ptr::NonNull;

pub(crate) trait BlockInterface {
    #[inline(always)]
    fn size(&self) -> Word {
        let ptr = self as *const _ as *const BlockHead;
        unsafe { (*ptr).size_and_flags & SIZE_MASK }
    }
    #[inline(always)]
    fn set_size(&mut self, size: Word) {
        let ptr = self as *mut _ as *mut BlockHead;
        unsafe { (*ptr).size_and_flags = size }
    }
    #[inline(always)]
    fn used(&self) -> bool {
        let ptr = self as *const _ as *const BlockHead;
        unsafe { ((*ptr).size_and_flags & USED_BIT_MASK) == 0 }
    }
    #[inline(always)]
    fn set_free(&mut self) {
        let ptr = self as *mut _ as *mut BlockHead;
        unsafe { (*ptr).size_and_flags |= USED_BIT_MASK }
    }
    #[inline(always)]
    fn set_used(&mut self) {
        let ptr = self as *mut _ as *mut BlockHead;
        unsafe { (*ptr).size_and_flags &= !USED_BIT_MASK }
    }
    #[inline(always)]
    fn prev_used(&self) -> bool {
        let ptr = self as *const _ as *const BlockHead;
        unsafe { ((*ptr).size_and_flags & PREV_USED_BIT_MASK) == 0 }
    }
    #[inline(always)]
    fn set_prev_free(&mut self) {
        let ptr = self as *mut _ as *mut BlockHead;
        unsafe { (*ptr).size_and_flags |= PREV_USED_BIT_MASK }
    }
    #[inline(always)]
    fn set_prev_used(&mut self) {
        let ptr = self as *mut _ as *mut BlockHead;
        unsafe { (*ptr).size_and_flags &= !PREV_USED_BIT_MASK }
    }
    #[inline(always)]
    fn next_used(&self) -> bool {
        let ptr = self as *const _ as *const BlockHead;
        unsafe { ((*ptr).size_and_flags & NEXT_USED_BIT_MASK) == 0 }
    }
    #[inline(always)]
    fn set_next_used(&mut self) {
        let ptr = self as *mut _ as *mut BlockHead;
        unsafe { (*ptr).size_and_flags &= !NEXT_USED_BIT_MASK }
    }
    #[inline(always)]
    fn set_next_free(&mut self) {
        let ptr = self as *mut _ as *mut BlockHead;
        unsafe { (*ptr).size_and_flags |= NEXT_USED_BIT_MASK }
    }
}

pub(crate) trait FreeBlockInterface {
    #[inline(always)]
    fn next(&self) -> Option<NonNull<FreeBlockHead>> {
        let ptr = self as *const _ as *const FreeBlockHead;
        unsafe { (*ptr).next }
    }

    #[inline(always)]
    fn set_next(&mut self, next: Option<NonNull<FreeBlockHead>>) {
        let ptr = self as *mut _ as *mut FreeBlockHead;
        unsafe { (*ptr).next = next }
    }

    #[inline(always)]
    fn prev(&self) -> Option<NonNull<FreeBlockHead>> {
        let ptr = self as *const _ as *const FreeBlockHead;
        unsafe { (*ptr).prev }
    }

    #[inline(always)]
    fn set_prev(&mut self, prev: Option<NonNull<FreeBlockHead>>) {
        let ptr = self as *mut _ as *mut FreeBlockHead;
        unsafe { (*ptr).prev = prev }
    }
}

#[repr(C, align(8))]
pub(crate) struct BlockHead {
    size_and_flags: Word,
}

impl BlockInterface for BlockHead {}

#[repr(C, align(8))]
pub(crate) struct BlockTail {
    size_and_flags: Word,
}

impl BlockInterface for BlockTail {}

#[repr(C, align(8))]
pub(crate) struct FreeBlockHead {
    size_and_flags: Word,
    pub(crate) prev: Option<NonNull<FreeBlockHead>>,
    pub(crate) next: Option<NonNull<FreeBlockHead>>,
}

impl BlockInterface for FreeBlockHead {}
impl FreeBlockInterface for FreeBlockHead {}
