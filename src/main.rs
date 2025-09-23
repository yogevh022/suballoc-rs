use crate::tlsf::{SubAllocator, Word};
use rand::Rng;
use std::hint::black_box;
use std::time::Duration;

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

    let free_time = hotloop!(loops; i; {
        sa.deallocate(allocs.swap_remove(rng.random_range(0..allocs.len()))).unwrap();
    });

    (alloc_time, free_time)
}

fn test_tlsf2(capacity: usize, loops: usize) -> Duration {
    let mut sa = SubAllocator::new(capacity as Word);
    let mut allocs = Vec::default();
    let mut rng = rand::rng();

    let time = hotloop!(loops; i; {
        if rng.random_bool(0.49) {
            let size = rng.random_range(1..100);
            allocs.push(sa.allocate(size).unwrap());
        } else if !allocs.is_empty() {
            let al = allocs.swap_remove(rng.random_range(0..allocs.len()));
            sa.deallocate(al).unwrap();
        }
    });

    time
}

fn main() {
    const CAPACITY: usize = 2usize.pow(25); // ~32m
    const LOOPS: usize = 1_000_000;
    // const CAPACITY: usize = 1024;
    // const LOOPS: usize = 20;

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
