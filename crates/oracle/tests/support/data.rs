//! The data inputs of the parity checklist (items 1 and 2): Brick 1.1 with
//! `reasonable`'s example models and building models, fetched into `data/`
//! by `scripts/fetch-data.sh`. Reports go to `target/oracle/data-*.md`.
//!
//! By default only Brick with small1 runs, so `cargo test -p
//! turtle-reasoner-oracle` stays under two minutes on a busy machine.
//! `ORACLE_SLOW=1` adds the owl.n3 and rdfs.ttl variants, Brick with
//! soda_hall, every building model, and OWL-RL (minutes on these inputs);
//! such a run takes longer, so run it one test at a time:
//!
//! ```sh
//! ORACLE_SLOW=1 timeout 600 cargo test --release -p turtle-reasoner-oracle --test data_brick -- brick_small1_owl
//! ```

use std::path::Path;
use turtle_reasoner_oracle::{Input, Options, Outcome, Table, data_dir, fetched_version, triangulate_input};

pub const BRICK: &str = "example_models/ontologies/Brick.n3";
pub const OWL: &str = "example_models/ontologies/owl.n3";
pub const RDFS: &str = "example_models/ontologies/rdfs.ttl";
pub const SMALL1: &str = "example_models/small1.n3";

/// `ORACLE_SLOW=1`: the variants, every building model, and OWL-RL.
pub fn slow() -> bool {
    std::env::var("ORACLE_SLOW").is_ok_and(|v| v == "1")
}

/// A scenario of the slow tier: runs only with `ORACLE_SLOW=1`.
pub fn check_slow(name: &str, item: &str, files: &[&str]) {
    if slow() {
        check(name, item, files);
    } else {
        println!("{name}: slow tier, not run (ORACLE_SLOW=1 runs it)");
    }
}

/// The files under `data/reasonable/`, or `None` (skipped) when not fetched.
pub fn input(name: &str, files: &[&str]) -> Option<Input> {
    let root = data_dir().join("reasonable");
    if fetched_version(&root).is_none() {
        let o = Outcome::skipped("data", "data/reasonable is missing: run scripts/fetch-data.sh");
        println!("{o:?}");
        return None;
    }
    let paths: Vec<_> = files.iter().map(|f| root.join(f)).collect();
    let refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();
    Some(Input::files(name, &refs).unwrap_or_else(|e| panic!("{e}")))
}

/// One scenario at a time: Nemo and HermiT use several cores each, and
/// running the scenarios side by side makes the file slower, not faster.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Every oracle on the files under `data/reasonable/`, compared; panics on a failed check.
pub fn check(name: &str, item: &str, files: &[&str]) {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let Some(input) = input(name, files) else { return };
    let start = std::time::Instant::now();
    let reasonable = super::reasonable(&input);
    let reasonable_secs = start.elapsed().as_secs_f64();
    // HermiT cannot load owl.n3, OWL's own vocabulary (the OWL API rejects
    // its axioms about built-in terms), so inputs with it are not classified.
    let options = Options {
        owlrl: slow(),
        hermit: !files.contains(&OWL),
    };
    let start = std::time::Instant::now();
    let row = triangulate_input(name, item, &input, None, reasonable, options);
    let table = Table { rows: vec![row] };
    let text = format!(
        "{}\nInput: {} triples from {}. reasonable took {reasonable_secs:.1} s, the other oracles {:.1} s.\n",
        table.markdown(),
        input.triples().len(),
        files.join(" + "),
        start.elapsed().as_secs_f64()
    );
    let path = super::report(&format!("data-{name}"), &text);
    println!("{text}\n(written to {})", path.display());
    let failures: Vec<String> = table
        .failures()
        .iter()
        .map(|(_, c)| format!("{}\n{}", c.name, c.detail))
        .collect();
    assert!(
        failures.is_empty(),
        "{name}: {} checks failed:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
