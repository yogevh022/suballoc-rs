use crate::test::{test_suballoc, test_suballoc_de, test_tlsf};
mod tlsf;
mod core;
mod test;


fn main() {
    test_tlsf();
    // test_suballoc_de();
    // test_suballoc();
}