use crate::block::BlockInterface;
use crate::tlsf::{SubAllocator, Word};
use rand::Rng;
use std::hint::black_box;
use std::ptr::NonNull;
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

    let free_time = hotloop!(loops / 3; i; {
        sa.deallocate(allocs.swap_remove(rng.random_range(0..allocs.len()))).unwrap();
    });

    (alloc_time, free_time)
}

fn dbg_free_blocks(sa: &SubAllocator) -> Vec<u64> {
    let mut bv = Vec::new();
    for block in sa.free_blocks.iter().flatten() {
        if let Some(block) = block {
            let b = unsafe {
                (**block).size() as u64
            };
            bv.push(b);
        }
    }
    bv
}

fn test_tlsf2(capacity: usize, loops: usize) -> Duration {
    let mut sa = SubAllocator::new(capacity as Word);
    let mut allocs = Vec::default();
    let mut rng = rand::rng();

    // println!("initial cap: {}", sa.capacity);
    // println!("initial FL: {:b}", sa.fl_bitmap);
    // println!(
    //     "initial SL: {:b}",
    //     sa.sl_bitmaps[sa.fl_bitmap.trailing_zeros() as usize]
    // );

    let time = hotloop!(loops; i; {
        if rng.random_bool(0.49) {
            let size = rng.random_range(1..100);
            allocs.push(sa.allocate(size).unwrap());
        } else if !allocs.is_empty() {
            let al = allocs.swap_remove(rng.random_range(0..allocs.len()));
            sa.deallocate(al).unwrap();
        }
        let casted: &[u64] = bytemuck::cast_slice(&sa.mem);
        let fl = sa.fl_bitmap;
        let sl = sa.sl_bitmaps[sa.fl_bitmap.trailing_zeros() as usize];
        let mapping = (fl + sl.trailing_zeros()*(fl/8))..(fl + (sl.trailing_zeros()+1)*(fl/8)) - 1;
        let fb = dbg_free_blocks(&sa);

        println!("{:?}", &casted);
        println!("Mapping: fl: {:<3} sl: {:<3} {:<3} - {:<3}", fl, sl, mapping.start, mapping.end);
        println!("Free blocks: {:?}", fb);
        println!("FL: {:<4} {:>12b}", fl, fl);
        println!("SL: {:<4} {:>12b}", sl.trailing_zeros(), sl);
    });

    time
}

fn main() {
    // const CAPACITY: usize = 2usize.pow(25); // ~32m
    // const LOOPS: usize = 10_000_000;
    const CAPACITY: usize = 1024;
    const LOOPS: usize = 20;

    let mut t = Duration::new(0, 0);
    let (mut al, mut de) = (Duration::new(0, 0), Duration::new(0, 0));

    const BENCH_COUNT: usize = 1;
    for i in 0..BENCH_COUNT {
        t += test_tlsf2(CAPACITY, LOOPS);
        // al += a;
        // de += d;
    }

    println!("TLSF:");
    println!("total: {:?}", t);
    println!("avg: {:?}", t / BENCH_COUNT as u32);

    // let al_avg = al / BENCH_COUNT as u32;
    // let de_avg = de / BENCH_COUNT as u32;
    //
    // println!("TLSF:");
    // println!("total:");
    // println!("alloc: {:?} free: {:?}", al, de);
    // println!("avg:");
    // println!("alloc: {:?} free: {:?}", al_avg, de_avg);
}
