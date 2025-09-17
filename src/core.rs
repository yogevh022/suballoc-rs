#[derive(Debug, Clone, Copy)]
pub enum SubAllocatorError {
    OutOfMemory,
    InvalidAllocation,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MemBlock {
    size: usize,
    prev_space: usize,
}

impl MemBlock {
    pub fn new(size: usize) -> Self {
        MemBlock {
            size,
            prev_space: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubAllocator {
    capacity: usize,
    pub free_blocks_indices: Vec<usize>,
    pub free_blocks: Vec<Option<MemBlock>>,
    pub used_blocks: Vec<MemBlock>,
}

impl SubAllocator {
    pub fn new(capacity: usize) -> Self {
        let mut free_blocks = vec![None; capacity];
        free_blocks[0] = Some(MemBlock::new(capacity));
        let mut free_blocks_indices = Vec::with_capacity(capacity);
        free_blocks_indices.push(0);
        SubAllocator {
            capacity,
            free_blocks_indices,
            free_blocks,
            used_blocks: vec![MemBlock::default(); capacity],
        }
    }

    fn partition_leftover(block: &mut MemBlock, target_size: usize) -> Option<MemBlock> {
        // subtract the leftover size from mem_block and return the resulting block
        let leftover_size = block.size - target_size;
        if leftover_size > 0 {
            block.size = target_size;
            let leftover_block = MemBlock::new(leftover_size);
            Some(leftover_block)
        } else {
            None
        }
    }

    /// allocate the requested size, return allocation start index, error if out of memory
    pub fn allocate(&mut self, requested_size: usize) -> Result<usize, SubAllocatorError> {
        debug_assert!(requested_size > 0);
        for (idx_i, &idx) in self.free_blocks_indices.iter().enumerate() {
            let mut block = self.free_blocks[idx].unwrap();
            if block.size < requested_size {
                continue;
            }
            self.free_blocks[idx] = None;

            // update the next used block prev_space
            let next_idx = idx + block.size;
            if next_idx != self.capacity {
                self.used_blocks[next_idx].prev_space = block.size - requested_size;
            }

            // add leftover space back
            let leftover_block = Self::partition_leftover(&mut block, requested_size);
            match leftover_block {
                Some(_) => {
                    let leftover_idx = idx + block.size;
                    self.free_blocks[leftover_idx] = leftover_block;
                    self.free_blocks_indices[idx_i] = leftover_idx;
                }
                None => {
                    self.free_blocks_indices.swap_remove(idx_i);
                }
            }

            self.used_blocks[idx] = block;
            return Ok(idx);
        }
        Err(SubAllocatorError::OutOfMemory)
    }

    /// deallocate by allocation start index
    pub fn deallocate(&mut self, alloc_start: usize) {
        debug_assert!(alloc_start < self.capacity);
        let mut block = self.used_blocks[alloc_start];
        block.size += block.prev_space;

        let next_idx = alloc_start + block.size;
        let greedy_idx = alloc_start - block.prev_space;
        if greedy_idx == alloc_start {
            println!("no prev_space: {}", alloc_start);
            self.free_blocks_indices.push(alloc_start);
        }

        if next_idx != self.capacity {
            if let Some(next_block) = self.free_blocks[next_idx].take() {
                // coalesce with prev free and next free blocks
                block.size += next_block.size;
                let next_next_idx = next_idx + next_block.size;
                if next_next_idx != self.capacity {
                    self.used_blocks[next_next_idx].prev_space = block.size; // update block next to next_block
                }
                println!("coalescing p+n: {} .. {}, updated nn: {}", greedy_idx, next_idx, next_next_idx);
                let idx_i = self.free_blocks_indices.iter().position(|&i| i == next_idx);
                self.free_blocks_indices.swap_remove(idx_i.unwrap());
            } else {
                // coalesce with prev free block and update the next used block prev_space
                self.used_blocks[next_idx].prev_space = block.size;
                println!("coalescing p: g: {} .. s: {}, updated n: {}", greedy_idx, alloc_start, next_idx);
            };
        }

        block.prev_space = 0;
        self.free_blocks[greedy_idx] = Some(block);
    }

    /// total capacity of the arena
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// total free space
    pub fn free(&self) -> usize {
        self.free_blocks_indices
            .iter()
            .map(|&i| self.free_blocks[i].unwrap().size)
            .sum()
    }

    /// total used space
    pub fn used(&self) -> usize {
        self.capacity - self.free()
    }

    /// number of fragments free space is split into
    pub fn fragment_count(&self) -> usize {
        self.free_blocks_indices.len()
    }
}
