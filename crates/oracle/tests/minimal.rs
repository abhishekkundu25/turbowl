//! Every oracle on the parity checklist's minimal inputs, triangulated.
//! The table is printed and written to `target/oracle/minimal.md`.

mod support;

use turtle_reasoner_oracle::{Table, cases, triangulate};

#[test]
fn oracles_agree_on_the_minimal_inputs() {
    let mut table = Table::default();
    for case in cases::minimal() {
        let reasonable = support::reasonable(&case.input());
        table.rows.push(triangulate(&case, reasonable));
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
    let stale = table.stale_known();
    // A known disagreement that no longer shows (or did not run) is reported,
    // so KNOWN stays true; it fails only when every oracle ran.
    if !stale.is_empty() {
        println!("known disagreements not seen: {stale:?}");
    }
}
