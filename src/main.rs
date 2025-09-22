use crate::test::{test_suballoc, test_suballoc_de, test_tlsf};
mod block;
mod core;
mod meta;
mod nav;
mod test;
mod tlsf;

fn main() {
    test_tlsf();
    // test_suballoc_de();
    // test_suballoc();
}
