//! Brick 1.1 with small1, and with owl.n3 and rdfs.ttl (the variants of the
//! Brick benchmark; the variants run with `ORACLE_SLOW=1`). See
//! tests/support/data.rs.

mod support;

use support::data::{BRICK, OWL, RDFS, SMALL1, check, check_slow};

#[test]
fn brick_small1() {
    check("brick-small1", "Brick + small1", &[BRICK, SMALL1]);
}

#[test]
fn brick_small1_owl() {
    check_slow("brick-small1-owl", "Brick + small1 + owl.n3", &[BRICK, SMALL1, OWL]);
}

#[test]
fn brick_small1_rdfs() {
    check_slow("brick-small1-rdfs", "Brick + small1 + rdfs.ttl", &[BRICK, SMALL1, RDFS]);
}

#[test]
fn brick_small1_owl_rdfs() {
    check_slow(
        "brick-small1-owl-rdfs",
        "Brick + small1 + owl.n3 + rdfs.ttl",
        &[BRICK, SMALL1, OWL, RDFS],
    );
}
