mod block;
mod meta;
mod tlsf;
fn main() {
    let mut sa = tlsf::SubAllocator::new(1024);
    for i in 0..10 {
        let a = sa.allocate(1).unwrap();
        dbg!(a);
    }
    dbg!(sa.capacity(), sa.free());
}