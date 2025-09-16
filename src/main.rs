use fxhash::FxHashMap;
use rand::{rng, Rng};
use std::hint::black_box;

macro_rules! hotloop {
    ($n:expr; $i:ident; $e:block) => {{
        let start = std::time::Instant::now();
        for $i in 0..$n {
            black_box($e);
        }
        start.elapsed()
    }};
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

fn main() {
    let mut rng = rng();
    const LOOPS: usize = 10_000_000;
    const SLOTS: usize = 1000;
    const CAPACITY: usize = 1_000_000;

    let slots: [Slot; SLOTS] = std::array::from_fn(|i| Slot { size: i as u32 });

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

    let map_read_time = hotloop!(LOOPS; i; {
        debug_read_map(&map, i % 1_000_000);
    });

    println!("vec ins: {:?}", vec_ins_time);
    println!("map ins: {:?}", map_ins_time);
    println!();
    println!("vec read: {:?}", vec_read_time);
    println!("map read: {:?}", map_read_time);
}
