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
    allocs.push(suballoc.allocate(1997).unwrap());
    for i in 0..LOOPS {
        if i % 7 == 0 {
            suballoc.deallocate(allocs.pop().unwrap()).unwrap();
        } else {
            allocs.push(suballoc.allocate(i % 76).unwrap());
        }
    }
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
