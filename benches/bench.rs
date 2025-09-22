use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use rand::Rng;
use std::hint::black_box;
use suballoc::SubAllocator;
use suballoc::tlsf::TLSF;

type Word = u32;
fn tlsf_bench_alloc(c: &mut Criterion) {
    const SIZE: usize = 2usize.pow(24);
    let mut tlsf_alloc = TLSF::new(SIZE as Word);
    let mut suballoc = SubAllocator::new(SIZE);
    let mut count = 0;
    const SIZES: [usize; 5] = [1, 2, 3, 4, 9];

    let mut rng = rand::rng();

    let mut group = c.benchmark_group("alloc_group");

    group
        .bench_function("tlsf", |b| {
            b.iter(|| {
                let size = SIZES[count % 5];
                let q = tlsf_alloc.allocate(size as Word).unwrap();
                if rng.random_bool(0.5) {
                    tlsf_alloc.deallocate(q).unwrap();
                }
                count += 1;
            });
        })
        .sample_size(1);

    group
        .bench_function("suballoc", |b| {
            b.iter(|| {
                let size = SIZES[count % 5];
                let q = suballoc.allocate(size).unwrap();
                if rng.random_bool(0.5) {
                    suballoc.deallocate(q);
                }
                count += 1;
            });
        })
        .sample_size(1);

    group.finish();
}

fn tlsf_bench_dealloc(c: &mut Criterion) {
    c.bench_function("dealloc", |b| {
        b.iter_batched(
            || {
                const SIZE: usize = 2usize.pow(24);
                let mut sa = TLSF::new(SIZE as Word);
                for _ in 0..100_000 {
                    black_box(sa.allocate(1).unwrap());
                }
                sa
            },
            |mut sa| {
                for i in 0..100_000 {
                    black_box(sa.deallocate(i * 32).unwrap());
                }
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, tlsf_bench_alloc);
criterion_main!(benches);
