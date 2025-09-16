use std::hint::black_box;
use crate::core::SubAllocator;

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
    const CAPACITY: usize = 1_500_000;
    const LOOPS: usize = 10_000;
    let mut suballoc = SubAllocator::new(CAPACITY);
    let mut allocs = Vec::default();
    for i in 0..10 {
        if i == 0 {
            allocs.push(suballoc.allocate(i*100).unwrap());
        } else {
            suballoc.allocate(i*100).unwrap();
        }
    }
    suballoc.deallocate(allocs.pop().unwrap()).unwrap();
    println!("free: {} used: {} frag: {}", suballoc.free(), suballoc.used(), suballoc.fragment_count());
}

pub(crate) fn test_suballoc() {
    const CAPACITY: usize = 10_000_000;
    const LOOPS: usize = 10_000_000;

    let mut suballoc = SubAllocator::new(CAPACITY);
    let mut allocs = Vec::default();

    let alloc_time = hotloop!(LOOPS; i; {
        allocs.push(suballoc.allocate(1).unwrap());
    });
    let free_time = hotloop!(LOOPS; i; {
        suballoc.deallocate(allocs.pop().unwrap()).unwrap();
    });

    println!("alloc: {:?} free: {:?}", alloc_time, free_time);
}
