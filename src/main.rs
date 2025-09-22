use std::io::ErrorKind::ConnectionAborted;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use crate::test::{test_suballoc, test_suballoc_de, test_tlsf};
use crate::tlsf::{Word, TLSF};

mod block;
mod core;
mod meta;
mod nav;
mod test;
mod tlsf;

fn main() {
    const CAPACITY: usize = 2usize.pow(25); // ~32m
    const LOOPS: usize = 1_000_000;
    // test_suballoc(CAPACITY, LOOPS);
    test_tlsf(CAPACITY, LOOPS);
}