mod block;
mod meta;
mod tlsf;
fn main() {
    let mut sa = tlsf::SubAllocator::new(1024);
    let mut alocs = Vec::new();
    for i in 0..10 {
        let a = sa.allocate(1).unwrap();
        alocs.push(a);
    }

    for i in alocs {
        sa.deallocate(i).unwrap();
    }
    dbg!(sa.capacity(), sa.free());
}