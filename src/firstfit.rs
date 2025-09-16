#[derive(Debug)]
pub enum MallocError {
    OutOfMemory,
    InvalidAllocation,
}

#[derive(Debug, Clone, Copy)]
pub struct MemBlock {
    size: usize,
    prev_space: usize,
}

impl MemBlock {
    pub fn new(size: usize, prev_space: usize) -> Self {
        MemBlock { size, prev_space }
    }
}

#[derive(Debug)]
pub struct Malloc {
    capacity: usize,

    free_blocks_indices: Vec<usize>,
    free_blocks: Vec<MemBlock>,
    used_blocks: Vec<MemBlock>,
}

impl Malloc {
    pub fn new(capacity: usize) -> Self {
        let mut free_blocks = vec![MemBlock::new(0, 0); capacity];
        free_blocks[0] = MemBlock::new(capacity, 0);
        let mut free_blocks_indices = Vec::with_capacity(capacity);
        free_blocks_indices.push(0);
        Malloc {
            capacity,
            free_blocks_indices,
            free_blocks,
            used_blocks: vec![MemBlock::new(0, 0); capacity],
        }
    }

    fn partition_leftover(block: &mut MemBlock, target_size: usize) -> Option<MemBlock> {
        // subtract the leftover size from mem_block and return the resulting block
        let leftover_size = block.size - target_size;
        if leftover_size > 0 {
            block.size = target_size;
            let leftover_block = MemBlock::new(leftover_size, 0);
            Some(leftover_block)
        } else {
            None
        }
    }

    pub fn alloc(&mut self, requested_size: usize) -> Result<(usize, MemBlock), MallocError> {
        for (idx_i, &idx) in self.free_blocks_indices.iter().enumerate() {
            let mut block = self.free_blocks[idx];
            if block.size < requested_size {
                continue;
            }

            // update the next used block prev_space
            let next_idx = idx + block.size;
            if next_idx != self.capacity {
                self.used_blocks[next_idx].prev_space = block.size;
            }

            // add leftover space back
            let leftover_block = Self::partition_leftover(&mut block, requested_size);
            match leftover_block {
                Some(leftover_block) => {
                    let leftover_idx = idx + block.size;
                    self.free_blocks[leftover_idx] = leftover_block;
                    self.free_blocks_indices[idx_i] = leftover_idx;
                }
                None => {
                    self.free_blocks_indices.swap_remove(idx_i);
                }
            }

            self.used_blocks[idx] = block;
            return Ok((idx, block));
        }
        Err(MallocError::OutOfMemory)
    }

    pub fn free(&mut self, addr: usize) -> Result<(), MallocError> {
        let mut block = self.used_blocks[addr];

        let next_block_idx = addr + block.size;
        let greedy_idx = addr - block.prev_space;

        if next_block_idx == self.capacity {
            block.size += block.prev_space;
            block.prev_space = 0;
            self.free_blocks[greedy_idx] = block;
            self.free_blocks_indices.push(greedy_idx);
            return Ok(());
        }

        let mut next_block = self.free_blocks[next_block_idx];
        if next_block.prev_space == 0 {
            // coalesce with prev free and next free blocks
            next_block.size += block.size + block.prev_space;
            self.free_blocks[greedy_idx] = next_block;
            let free_block_index = self
                .free_blocks_indices
                .iter_mut()
                .find(|i| **i == next_block_idx);
            *free_block_index.unwrap() = greedy_idx;
        } else {
            // coalesce with prev free block and update the next used block prev_space
            block.size += block.prev_space;
            block.prev_space = 0;
            self.used_blocks[next_block_idx].prev_space = block.size;
            self.free_blocks[greedy_idx] = block;
            self.free_blocks_indices.push(greedy_idx);
        }
        Ok(())
    }
}
