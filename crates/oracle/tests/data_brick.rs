//! Parity checklist item 1: Brick 1.1 with small1, and with owl.n3 and
//! rdfs.ttl (the variants of reasonable's benches/my_benchmark.rs:26-84; the
//! variants run with `ORACLE_SLOW=1`). See tests/support/data.rs.

mod support;

use support::data::{BRICK, OWL, RDFS, SMALL1, check, check_slow};

#[test]
fn brick_small1() {
    check("brick-small1", "1", &[BRICK, SMALL1]);
}

#[test]
fn brick_small1_owl() {
    check_slow("brick-small1-owl", "1", &[BRICK, SMALL1, OWL]);
}

#[test]
fn brick_small1_rdfs() {
    check_slow("brick-small1-rdfs", "1", &[BRICK, SMALL1, RDFS]);
}

#[test]
fn brick_small1_owl_rdfs() {
    check_slow("brick-small1-owl-rdfs", "1", &[BRICK, SMALL1, OWL, RDFS]);
}
