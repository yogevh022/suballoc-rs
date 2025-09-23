use crate::tlsf::{Word, WORD_BITS};

pub(crate) const BLOCK_ALIGNMENT: Word = 8;
pub(crate) const BLOCK_HEAD_SIZE: Word = size_of::<BlockHead>() as Word;
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

pub(crate) trait BlockPtr<T: BlockInterface> {
    unsafe fn deref_mut<'a>(&self) -> &'a mut T;
    unsafe fn block_add<P>(&self, offset: usize) -> *mut P;
    unsafe fn block_sub<P>(&self, offset: usize) -> *mut P;
}

impl<T: BlockInterface> BlockPtr<T> for *mut T {
    unsafe fn deref_mut<'a>(&self) -> &'a mut T {
        unsafe { &mut **self }
    }
    unsafe fn block_add<P>(&self, offset: usize) -> *mut P {
        unsafe { (*self).byte_add(offset) as *mut P }
    }
    unsafe fn block_sub<P>(&self, offset: usize) -> *mut P {
        unsafe { (*self).byte_sub(offset) as *mut P }
    }
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
    links: u64, // 4 bytes prev, 4 bytes next, measured as offset from mem start
}

impl BlockInterface for FreeBlockHead {}

impl FreeBlockHead {
    #[inline(always)]
    pub(crate) fn link_offsets(&self) -> (Word, Word) {
        let next_link = self.links as Word;
        let prev_link = (self.links >> WORD_BITS) as Word;
        (prev_link as Word, next_link as Word)
    }
    #[inline(always)]
    pub(crate) fn set_links(&mut self, links: u64) {
        self.links = links;
    }
    #[inline(always)]
    pub(crate) fn set_prev_link(&mut self, link: Word) {
        let links_masked = self.links & LOW_MASK;
        self.links = links_masked | ((link as u64) << WORD_BITS);
    }
    #[inline(always)]
    pub(crate) fn set_next_link(&mut self, link: Word) {
        let links_masked = self.links & HIGH_MASK;
        self.links = links_masked | (link as u64);
    }
}
