use crate::core::SubAllocator;
use rand::Rng;
use std::hint::black_box;
use crate::tlsf;

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
            let alloc = allocs.swap_remove(rng.random_range(0..allocs.len()));
            suballoc.deallocate(alloc);
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

pub(crate) fn test_suballoc() {
    const CAPACITY: usize = 10_000_000;
    const LOOPS: usize = 10_000_000;

    let mut suballoc = SubAllocator::new(CAPACITY);
    let mut allocs = Vec::default();

    let alloc_time = hotloop!(LOOPS; i; {
        allocs.push(suballoc.allocate(1).unwrap());
    });
    let free_time = hotloop!(LOOPS; i; {
        suballoc.deallocate(allocs.pop().unwrap());
    });

    println!("alloc: {:?} free: {:?}", alloc_time, free_time);
}

pub(crate) fn test_tlsf() {
    const CAPACITY: usize = 100;
    let sa = tlsf::TLSF::new(CAPACITY as u32);
    let m = sa.mapping(1);
    dbg!(m);
}