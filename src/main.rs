use crate::tlsf::{SubAllocator, Word};
use std::time::Duration;
use std::hint::black_box;
use rand::Rng;

mod block;
mod meta;
mod old;
mod tlsf;

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

fn test_tlsf(capacity: usize, loops: usize) -> (Duration, Duration) {
    let mut sa = SubAllocator::new(capacity as Word);
    let mut allocs = Vec::default();
    let mut rng = rand::rng();

    let alloc_time = hotloop!(loops; i; {
        allocs.push(sa.allocate(1).unwrap());
    });

    let free_time = hotloop!(loops / 3; i; {
        sa.deallocate(allocs.swap_remove(rng.random_range(0..allocs.len()))).unwrap();
    });

    (alloc_time, free_time)
}

fn main() {
    const CAPACITY: usize = 2usize.pow(25); // ~32m
    const LOOPS: usize = 10_000_000;

    let (mut al, mut de) = (Duration::new(0, 0), Duration::new(0, 0));

    const BENCH_COUNT: usize = 200;
    for i in 0..BENCH_COUNT {
        let (a, d) = test_tlsf(CAPACITY, LOOPS);
        al += a;
        de += d;
    }

    let al_avg = al / BENCH_COUNT as u32;
    let de_avg = de / BENCH_COUNT as u32;

    println!("TLSF:");
    println!("total:");
    println!("alloc: {:?} free: {:?}", al, de);
    println!("avg:");
    println!("alloc: {:?} free: {:?}", al_avg, de_avg);
}
