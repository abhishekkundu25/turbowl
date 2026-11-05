//! Brick 1.1 with soda_hall, and each building model (from `reasonable`'s
//! repository, benches/python/buildings) with Brick, owl.n3 and rdfs.ttl.
//! Both run with `ORACLE_SLOW=1` only. See tests/support/data.rs.

mod support;

use support::data::{BRICK, OWL, RDFS, check, check_slow};

#[test]
fn brick_soda_hall() {
    check_slow(
        "brick-soda-hall",
        "Brick + soda_hall",
        &[BRICK, "example_models/soda_hall.n3"],
    );
}

#[test]
fn building_models() {
    if !support::data::slow() {
        println!("building models: slow tier, not run (ORACLE_SLOW=1 runs them)");
        return;
    }
    let dir = turtle_reasoner_oracle::data_dir().join("reasonable/benches/python/buildings");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        let o =
            turtle_reasoner_oracle::Outcome::skipped("data", "data/reasonable is missing: run scripts/fetch-data.sh");
        println!("{o:?}");
        return;
    };
    let mut buildings: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".ttl"))
        .collect();
    buildings.sort();
    for b in buildings {
        let file = format!("benches/python/buildings/{b}");
        let name = format!("building-{}", b.trim_end_matches(".ttl").replace(['(', ')', '.'], ""));
        check(
            &name,
            "Brick + owl.n3 + rdfs.ttl + building",
            &[BRICK, OWL, RDFS, &file],
        );
    }
}
