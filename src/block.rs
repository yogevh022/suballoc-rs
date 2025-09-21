use std::ptr::NonNull;
use crate::tlsf::{Word, PREV_USED_BIT_MASK, SIZE_MASK, USED_BIT_MASK};

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
}

pub(crate) trait FreeBlockInterface {
    #[inline(always)]
    fn link(&self) -> NonNull<Option<NonNull<FreeBlockLink>>> {
        let ptr = self as *const _ as *const FreeBlockHead;
        unsafe { (*ptr).link }
    }
    #[inline(always)]
    fn set_link(&mut self, link: NonNull<Option<NonNull<FreeBlockLink>>>) {
        let ptr = self as *mut _ as *mut FreeBlockHead;
        unsafe { (*ptr).link = link }
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
    link: NonNull<Option<NonNull<FreeBlockLink>>>,
}

impl BlockInterface for FreeBlockHead {}
impl FreeBlockInterface for FreeBlockHead {}

#[derive(Debug)]
pub(crate) struct FreeBlockLink {
    pub(crate) head: NonNull<FreeBlockHead>,
    pub(crate) prev: Option<NonNull<FreeBlockLink>>,
    pub(crate) next: Option<NonNull<FreeBlockLink>>,
}