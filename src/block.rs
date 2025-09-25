use crate::meta::{byte_add_into, byte_sub_into, with_head};
use crate::tlsf::{WORD_BITS, Word};

pub(crate) const BLOCK_ALIGNMENT: Word = 8;
pub(crate) const BLOCK_HEAD_SIZE: Word = size_of::<UsedBlockHead>() as Word;
pub(crate) const BLOCK_TAIL_SIZE: Word = size_of::<BlockTail>() as Word;
pub(crate) const BLOCK_META_SIZE: Word = BLOCK_HEAD_SIZE + BLOCK_TAIL_SIZE;
pub(crate) const PACKED_NONE_PTR: Word = Word::MAX;
pub(crate) const PACKED_NONE_DOUBLE_PTR: u64 = u64::MAX;
const LOW_MASK: u64 = 0xFFFF_FFFF;
const HIGH_MASK: u64 = !LOW_MASK;

pub(crate) struct BitFlags;
impl BitFlags {
    pub const USED: Word = 0b1;
    pub const PREV_USED: Word = 0b10;
    pub const NEXT_USED: Word = 0b100;
    pub const SIZE_MASK: Word = !0b111;
}

pub(crate) trait BlockInterface {
    #[inline(always)]
    fn size(&self) -> Word {
        let ptr = self as *const _ as *const Word;
        unsafe { *ptr & BitFlags::SIZE_MASK }
    }
    #[inline(always)]
    fn flags(&self) -> Word {
        let ptr = self as *const _ as *const Word;
        unsafe { *ptr & !BitFlags::SIZE_MASK }
    }
    #[inline(always)]
    fn set_size_flags(&mut self, word: Word) {
        let ptr = self as *mut _ as *mut Word;
        unsafe { *ptr = word }
    }
    #[inline(always)]
    fn or_flags(&mut self, flags: Word) {
        let ptr = self as *mut _ as *mut Word;
        unsafe { *ptr |= flags }
    }
    #[inline(always)]
    fn clear_or_flags(&mut self, flags: Word) {
        let ptr = self as *mut _ as *mut Word;
        unsafe { *ptr &= !flags }
    }
    #[inline(always)]
    fn used(&self) -> bool {
        let ptr = self as *const _ as *const Word;
        unsafe { (*ptr & BitFlags::USED) != 0 }
    }
    #[inline(always)]
    fn next_used(&self) -> bool {
        let ptr = self as *const _ as *const Word;
        unsafe { (*ptr & BitFlags::NEXT_USED) != 0 }
    }
    #[inline(always)]
    fn prev_used(&self) -> bool {
        let ptr = self as *const _ as *const Word;
        unsafe { (*ptr & BitFlags::PREV_USED) != 0 }
    }
}
pub(crate) trait BlockTailPtrInterface {
    #[inline(always)]
    fn deref<'a>(&mut self) -> &'a mut BlockTail {
        unsafe { &mut **(self as *const _ as *const *mut BlockTail) }
    }
    #[inline(always)]
    fn head_ptr(&self, block_size: Word) -> *mut BlockHead {
        let ptr = unsafe { *(self as *const _ as *const *mut BlockTail) };
        let head_offset = with_head(block_size) as _;
        unsafe { byte_sub_into(ptr, head_offset) }
    }
}
pub(crate) trait BlockHeadPtrInterface {
    #[inline(always)]
    fn deref<'a>(&mut self) -> &'a mut BlockHead {
        unsafe { &mut **(self as *const _ as *const *mut BlockHead) }
    }
    #[inline(always)]
    fn tail_ptr(&self, block_size: Word) -> *mut BlockTail {
        let ptr = unsafe { *(self as *const _ as *const *mut BlockHead) };
        let tail_offset = with_head(block_size) as _;
        unsafe { byte_add_into(ptr, tail_offset) }
    }
}

pub(crate) union BlockHead {
    pub free: FreeBlockHead,
    pub used: UsedBlockHead,
}

impl BlockHead {
    pub fn as_free(&mut self) -> &mut FreeBlockHead {
        unsafe { &mut self.free }
    }
}

#[repr(C, align(8))]
#[derive(Debug, Copy, Clone)]
pub(crate) struct FreeBlockHead {
    size_and_flags: Word,
    links: u64, // 4 bytes prev, 4 bytes next, measured as offset from mem start
}
impl BlockInterface for BlockHead {}
impl BlockHeadPtrInterface for *mut BlockHead {}

#[repr(C, align(8))]
#[derive(Debug, Copy, Clone)]
pub(crate) struct UsedBlockHead {
    size_and_flags: Word,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(crate) struct BlockTail {
    size_and_flags: Word,
}
impl BlockInterface for BlockTail {}
impl BlockTailPtrInterface for *mut BlockTail {}

impl FreeBlockHead {
    #[inline(always)]
    pub fn link_offsets(&self) -> (Word, Word) {
        let next_link = self.links as Word;
        let prev_link = (self.links >> WORD_BITS) as Word;
        (prev_link as Word, next_link as Word)
    }
    #[inline(always)]
    pub fn set_links(&mut self, links: u64) {
        self.links = links;
    }
    #[inline(always)]
    pub fn set_prev_link(&mut self, link: Word) {
        let links_masked = self.links & LOW_MASK;
        self.links = links_masked | ((link as u64) << WORD_BITS);
    }
    #[inline(always)]
    pub fn set_next_link(&mut self, link: Word) {
        let links_masked = self.links & HIGH_MASK;
        self.links = links_masked | (link as u64);
    }
}
