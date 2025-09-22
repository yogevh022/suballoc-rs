use std::alloc::alloc;
use crate::core::SubAllocator;
use crate::tlsf;
use crate::tlsf::{NEXT_USED_BIT_MASK, PREV_USED_BIT_MASK, SIZE_MASK, TLSF, USED_BIT_MASK, Word};
use rand::Rng;
use std::hint::black_box;

#[macro_export]
macro_rules! hotloop {
    ($n:expr; $i:ident; $e:block) => {{
        let start = std::time::Instant::now();
        for $i in 0..$n {
            black_box($e);
        }
        start.elapsed()
    }};
}

pub(crate) fn test_suballoc_de() {
    const CAPACITY: usize = 30_000_000;
    const LOOPS: usize = 2_000_000;
    let mut suballoc = SubAllocator::new(CAPACITY);
    let mut allocs = Vec::default();
    let mut rng = rand::rng();

    let time = hotloop!(LOOPS; i; {
        let p: f32 = rng.random();
        if p > 0.49 {
            let size = rng.random_range(1..=496*1) as usize;
            let alloc = suballoc.allocate(size).unwrap();
            allocs.push(alloc);
        } else if !allocs.is_empty() {
            // let alloc = allocs.swap_remove(rng.random_range(0..allocs.len()));
            // suballoc.deallocate(alloc);
        }
    });
    println!("time: {:?}", time);
    println!(
        "free: {} used: {} frag: {}",
        suballoc.free(),
        suballoc.used(),
        suballoc.fragment_count()
    );
}

pub(crate) fn test_suballoc(capacity: usize, loops: usize) {
    let mut suballoc = SubAllocator::new(capacity);
    let mut allocs = Vec::default();
    let mut rng = rand::rng();

    let alloc_time = hotloop!(loops; i; {
        allocs.push(suballoc.allocate(1).unwrap());
    });
    let free_time = hotloop!(loops / 2; i; {
        suballoc.deallocate(allocs.swap_remove(rng.random_range(0..allocs.len())));
    });
    let alloc_time2 = hotloop!(loops; i; {
        allocs.push(suballoc.allocate(1).unwrap());
    });
    let free_time2 = hotloop!(loops; i; {
        suballoc.deallocate(allocs.swap_remove(rng.random_range(0..allocs.len())));
    });

    println!("SUBALLOC:");
    println!("alloc: {:?} free: {:?}", alloc_time, free_time);
    println!("alloc 2: {:?} free 2: {:?}", alloc_time2, free_time2);
    println!("alloc t: {:?} free t: {:?}", alloc_time + alloc_time2, free_time + free_time2);
}

pub(crate) fn test_tlsf(capacity: usize, loops: usize) {
    let mut sa = TLSF::new(capacity as Word);
    let mut allocs = Vec::default();
    let mut rng = rand::rng();

    let alloc_time = hotloop!(loops; i; {
        allocs.push(sa.allocate(1).unwrap());
    });
    let free_time = hotloop!(loops / 2; i; {
        sa.deallocate(allocs.swap_remove(rng.random_range(0..allocs.len()))).unwrap();
    });

    let alloc_time_2 = hotloop!(loops; i; {
        allocs.push(sa.allocate(1).unwrap());
    });
    let free_time_2 = hotloop!(loops; i; {
        sa.deallocate(allocs.swap_remove(rng.random_range(0..allocs.len()))).unwrap();
    });

    println!("TLSF:");
    println!("alloc: {:?} free: {:?}", alloc_time, free_time);
    println!("alloc 2: {:?} free 2: {:?}", alloc_time_2, free_time_2);
    println!("alloc t: {:?} free t: {:?}", alloc_time + alloc_time_2, free_time + free_time_2);
}

pub fn print_mem_casted(sa: &tlsf::TLSF) {
    let casted: &[u64] = bytemuck::cast_slice(&sa.mem);
    let repr = casted
        .iter()
        .map(|&x| {
            let size = x as Word & SIZE_MASK;
            let used = x as Word & USED_BIT_MASK;
            let prev_used = (x as Word & PREV_USED_BIT_MASK) >> 1;
            let next_used = (x as Word & NEXT_USED_BIT_MASK) >> 2;
            format!("{}[{} {} {}]", size, prev_used, used, next_used)
        })
        .collect::<Vec<_>>()
        .join(", ");
    println!("MEM: \n{:?}", repr);
}
