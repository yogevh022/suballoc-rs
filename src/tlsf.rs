const USED_BIT_MASK: u32 = 0b1;
const PREV_USED_BIT_MASK: u32 = 0b10;
const SIZE_MASK: u32 = !0b11;
const SLI_SIZE: usize = 8;
const SLI_BITS: u32 = SLI_SIZE.trailing_zeros();

#[derive(Debug, Clone, Copy)]
pub enum AllocError {
    OutOfMemory,
    InvalidAllocation,
}

struct BlockHead {
    size_and_flags: u32,
}

struct FreeBlockHead {
    size_and_flags: u32,
    prev_free: *mut FreeBlockHead,
    next_free: *mut FreeBlockHead,
}

fn size(ptr: *const BlockHead) -> u32 {
    unsafe { (*ptr).size_and_flags & SIZE_MASK }
}

fn is_used(ptr: *const BlockHead) -> bool {
    unsafe { ((*ptr).size_and_flags & USED_BIT_MASK) != 0 }
}

fn is_prev_used(ptr: *const BlockHead) -> bool {
    unsafe { ((*ptr).size_and_flags & PREV_USED_BIT_MASK) != 0 }
}

pub struct TLSF {
    mem: Box<[u8]>,
    fl_bitmap: u32,
    sl_bitmaps: [u32; 32],
}

impl TLSF {
    pub fn new(capacity: u32) -> Self {
        Self {
            mem: vec![0; capacity as usize].into_boxed_slice(),
            fl_bitmap: 0,
            sl_bitmaps: [0; 32],
        }
    }

    fn left_mask_from(index: u32) -> u32 {
        u32::MAX << index
    }

    fn calc_sl_index_for_fl(size: u32, fl: u32) -> u32 {
        let base = 1 << fl;
        let offset = size - base;
        (offset << SLI_BITS) >> fl
    }

    pub fn mapping(&self, size: u32) -> Result<(u32, u32), AllocError> {
        let fl_idx = 31 - size.leading_zeros();
        let available_fl_mask = self.fl_bitmap & (!0u32 << fl_idx);
        if available_fl_mask == 0 {
            return Err(AllocError::OutOfMemory);
        }

        let first_fl = available_fl_mask.trailing_zeros();
        let sl_idx = if first_fl == fl_idx {
            Self::calc_sl_index_for_fl(size, first_fl)
        } else {
            0
        };

        let available_sl_mask = self.sl_bitmaps[sl_idx as usize] & Self::left_mask_from(sl_idx);
        if available_sl_mask != 0 {
            let first_sl = available_sl_mask.trailing_zeros();
            return Ok((first_fl, first_sl));
        }

        let higher_fl_mask = self.fl_bitmap & Self::left_mask_from(fl_idx + 1);
        if higher_fl_mask != 0 {
            let next_fl = higher_fl_mask.trailing_zeros();
            return Ok((next_fl, 0));
        }

        Err(AllocError::OutOfMemory)
    }
}
