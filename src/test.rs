use crate::firstfit::Malloc;
use crate::old::{SimpleAllocationRequest, VMallocFirstFit, VirtualMalloc};
use rand::{Rng, rng};
use rustc_hash::{FxHashMap, FxHashSet};
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

pub fn test_malloc() {
    const CAPACITY: usize = 10_000_000;
    const LOOPS: usize = 10_000_000;
    let mut malloc = Malloc::new(CAPACITY);
    let mut old_malloc = VMallocFirstFit::new(CAPACITY, 0);

    let mut new_allocs = Vec::default();
    let mut old_allocs = Vec::default();

    for _ in 0..10 {
        let allocation_request = SimpleAllocationRequest { size: 1 };
        let old_malloc_time = hotloop!(LOOPS; i; {
            old_allocs.push(old_malloc.alloc(allocation_request).unwrap());
        });
            let old_free_time = hotloop!(LOOPS; i; {
            old_malloc.free(old_allocs.pop().unwrap()).unwrap();
        });

        let new_malloc_time = hotloop!(LOOPS; i; {
            new_allocs.push(malloc.alloc(1).unwrap().0);
        });
        let new_free_time = hotloop!(LOOPS; i; {
            malloc.free(new_allocs.pop().unwrap()).unwrap();
        });

        println!("new alloc: {:?} free: {:?}", new_malloc_time, new_free_time);
        println!("old alloc: {:?} free: {:?}", old_malloc_time, old_free_time);
    }


    //
    // println!("old count: {:?}\nnew count: {:?}\n", old_allocs.len(), new_allocs.len());
    // println!();
}

#[derive(Default, Clone, Copy, Debug)]
struct Slot {
    size: u32,
}

#[inline(always)]
fn debug_ins_vec(vec: &mut Vec<Slot>, i: usize, s: Slot) {
    vec[i] = s;
}

#[inline(always)]
fn debug_read_vec(vec: &Vec<Slot>, i: usize) -> Slot {
    vec[i]
}

#[inline(always)]
fn debug_ins_map(map: &mut FxHashMap<usize, Slot>, i: usize, s: Slot) {
    map.insert(i, s);
}

#[inline(always)]
fn debug_read_map(map: &FxHashMap<usize, Slot>, i: usize) -> Slot {
    map[&i]
}

pub fn test_rw() {
    let mut rng = rng();
    const LOOPS: usize = 10_000_000;
    const SLOTS: usize = 1000;
    const CAPACITY: usize = 1_000_000;

    let slots: [Slot; SLOTS] = std::array::from_fn(|i| Slot { size: i as u32 });
    let next_indices = (0..CAPACITY).collect::<Vec<_>>();

    let mut map = FxHashMap::<usize, Slot>::default();
    let mut vec = vec![Slot::default(); CAPACITY];

    let vec_ins_time = hotloop!(LOOPS; i; {
        debug_ins_vec(&mut vec, i % 1_000_000, slots[rng.random_range(0..SLOTS)]);
    } );
    let map_ins_time = hotloop!(LOOPS; i; {
        debug_ins_map(&mut map, i % 1_000_000, slots[rng.random_range(0..SLOTS)]);
    } );

    let vec_read_time = hotloop!(LOOPS; i; {
        debug_read_vec(&vec, i % 1_000_000);
    });

    let vec_ll_read_time = hotloop!(LOOPS; i; {
        let q = vec[next_indices[i % 1_000_000]];
        black_box(q);
    });

    let map_read_time = hotloop!(LOOPS; i; {
        debug_read_map(&map, i % 1_000_000);
    });

    println!("vec ins: {:?}", vec_ins_time);
    println!("map ins: {:?}", map_ins_time);
    println!();
    println!("vec read: {:?}", vec_read_time);
    println!("vec ll read: {:?}", vec_ll_read_time);
    println!("map read: {:?}", map_read_time);
}
