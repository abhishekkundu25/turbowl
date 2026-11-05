//! Every oracle on the minimal inputs (`cases::minimal`), triangulated.
//! The table is printed and written to `target/oracle/minimal.md`.

mod support;

use turbowl_oracle::{Table, cases, triangulate};

#[test]
fn oracles_agree_on_the_minimal_inputs() {
    let mut table = Table::default();
    for case in cases::minimal() {
        table.rows.push(triangulate(&case));
    }
    let text = table.markdown();
    let path = support::report("minimal", &text);
    println!("{text}\n(written to {})", path.display());
    let failures: Vec<String> = table
        .failures()
        .iter()
        .map(|(case, c)| format!("{case}: {}\n{}", c.name, c.detail))
        .collect();
    assert!(
        failures.is_empty(),
        "{} checks failed:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
    // A known fault that no longer shows (or did not run) is reported, so
    // KNOWN stays true.
    let stale = table.stale_known();
    if !stale.is_empty() {
        println!("known faults not seen: {stale:?}");
    }
}
