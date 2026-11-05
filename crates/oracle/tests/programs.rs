//! The one rule source (`oracles/rules/`) and the programs composed from it
//! (`turbowl_oracle::programs`):
//!
//! - the Soufflé and VLog programs of the default and the complete
//!   configuration equal the golden programs in `tests/programs/`
//!   (`BLESS=1` rewrites them; read the diff before keeping it);
//! - every rule of the spec's switch table is named in exactly one group
//!   file, its own group's, and in the W3C report's map;
//! - every group has its file in the rule source, the reference evaluator,
//!   the generator and the minimal inputs;
//! - the codes and rules a configuration's program holds;
//! - Nemo runs the complete configuration.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use turbowl::{Switch, Switches};
use turbowl_oracle::{Input, Outcome, nemo, programs, w3c, workspace_root};

/// Compares `text` with the golden file `tests/programs/<name>`, or
/// rewrites the file when `BLESS=1`.
fn golden(name: &str, text: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/programs")
        .join(name);
    if std::env::var("BLESS").is_ok_and(|v| v == "1") {
        std::fs::write(&path, text).expect("tests/programs is writable");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    if text != expected {
        let first = text
            .lines()
            .zip(expected.lines())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| text.lines().count().min(expected.lines().count()));
        panic!(
            "{name} differs from the golden program from line {}:\n  emitted: {:?}\n  golden:  {:?}\n\
             (BLESS=1 rewrites the golden programs)",
            first + 1,
            text.lines().nth(first),
            expected.lines().nth(first)
        );
    }
}

#[test]
fn souffle_programs_equal_the_golden_ones() {
    golden("souffle-default.dl", &programs::souffle(Switches::default()));
    golden("souffle-complete.dl", &programs::souffle(Switches::complete()));
}

#[test]
fn vlog_programs_equal_the_golden_ones() {
    golden("vlog-default.rls", &programs::vlog(Switches::default()));
    golden("vlog-complete.rls", &programs::vlog(Switches::complete()));
}

/// The rules of each group in the spec's table ("The switches"), the core
/// as `core`.
fn spec_table() -> BTreeMap<String, Vec<String>> {
    let path = workspace_root().join("docs/specs/v2-1-scoreboard-and-rules.md");
    let spec = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let section = spec
        .split("## The switches")
        .nth(1)
        .expect("the spec has a section \"The switches\"");
    let mut table = BTreeMap::new();
    for row in section
        .lines()
        .skip_while(|l| !l.starts_with('|'))
        .take_while(|l| l.starts_with('|'))
    {
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        let (group, rules) = (cells[1], cells[2]);
        if group == "Switch" || group.starts_with("---") {
            continue;
        }
        let group = if group.starts_with("none") {
            "core".to_string()
        } else {
            group.trim_matches('`').to_string()
        };
        table.insert(group, rules.split(',').map(|r| r.trim().to_string()).collect());
    }
    table
}

/// The rules a group file names: the comment that ends a rule (`% cax-sco`),
/// and the lists of rules not written yet (`% to come: ...`) or left out
/// (`% left out: ...`). A comment is such a marker when everything after
/// the label is a list of rule names.
fn named_rules(text: &str) -> Vec<String> {
    let is_rule = |r: &str| {
        r.contains('-')
            && r.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    let mut names = Vec::new();
    for line in text.lines() {
        let Some((_, comment)) = line.split_once('%') else {
            continue;
        };
        let comment = comment.trim();
        let list = comment
            .strip_prefix("to come:")
            .or_else(|| comment.strip_prefix("left out:"))
            .unwrap_or(comment);
        let rules: Vec<&str> = list.split(',').map(str::trim).collect();
        if rules.iter().all(|r| is_rule(r)) {
            names.extend(rules.into_iter().map(String::from));
        }
    }
    names
}

#[test]
fn every_rule_of_the_switch_table_is_in_exactly_one_group_file() {
    let table = spec_table();
    let groups: BTreeSet<String> = std::iter::once("core")
        .chain(Switch::ALL.into_iter().map(Switch::name))
        .map(String::from)
        .collect();
    assert_eq!(
        table.keys().cloned().collect::<BTreeSet<_>>(),
        groups,
        "the spec's table and turbowl::Switch name the same groups"
    );

    let dir = workspace_root().join("oracles/rules");
    let mut files: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("a directory entry").path();
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("a UTF-8 name")
            .to_string();
        assert_eq!(
            path.extension().and_then(|e| e.to_str()),
            Some("rls"),
            "{}",
            path.display()
        );
        let text = std::fs::read_to_string(&path).expect("readable");
        files.insert(name, named_rules(&text));
    }
    assert_eq!(
        files.keys().cloned().collect::<BTreeSet<_>>(),
        groups,
        "oracles/rules/ holds one file per group"
    );

    let mut problems = Vec::new();
    for (group, rules) in &table {
        for rule in rules {
            let holders: Vec<&String> = files.iter().filter(|(_, n)| n.contains(rule)).map(|(f, _)| f).collect();
            if holders != [group] {
                problems.push(format!("{rule} (group {group}) is named in {holders:?}"));
            }
        }
    }
    let known: BTreeSet<&String> = table.values().flatten().collect();
    for (file, names) in &files {
        for name in names.iter().filter(|n| !known.contains(n)) {
            problems.push(format!("{file}.rls names {name}, which the spec's table does not list"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn nemo_runs_the_complete_configuration() {
    // A chain, a sameAs and a finding: a rule of each group with rules.
    let input = Input::turtle(
        "complete",
        "@prefix ex: <http://example.org/> .\n\
         @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         ex:p owl:propertyChainAxiom (ex:q ex:q) .\n\
         ex:a ex:q ex:b . ex:b ex:q ex:c .\n\
         ex:c owl:sameAs ex:d .\n\
         ex:A owl:disjointWith ex:B . ex:x a ex:A, ex:B .\n",
    );
    let closure = match nemo::run_with(&input, Switches::complete()) {
        Outcome::Ran(c) => c,
        Outcome::Skipped(why) => {
            println!("skipped: {why}");
            return;
        }
        Outcome::Failed(e) => panic!("Nemo failed on the complete configuration: {e}"),
    };
    assert!(closure.has("<http://example.org/a> <http://example.org/p> <http://example.org/d>"));
    assert_eq!(closure.findings.len(), 1, "{:?}", closure.findings);
}

/// The groups: the core, then every switch, by name.
fn group_names() -> Vec<&'static str> {
    std::iter::once("core")
        .chain(Switch::ALL.into_iter().map(Switch::name))
        .collect()
}

/// Each group has a file in each place it is split: the rule source
/// (`oracles/rules/<name>.rls`), the reference evaluator's rules and the
/// generator's shapes (`crates/reference/src/{rules,generate}/`), and the
/// minimal inputs (`crates/oracle/src/cases/`), a Rust file named with `_`
/// for `-` and declared by its `mod.rs`. Besides those, the Rust
/// directories hold only `mod.rs` and `datatypes.rs` (dt-not-type, in the
/// `checks` group).
#[test]
fn every_group_has_its_file_everywhere() {
    let root = workspace_root();
    let mut problems = Vec::new();
    for dir in [
        "crates/reference/src/rules",
        "crates/reference/src/generate",
        "crates/oracle/src/cases",
    ] {
        let dir = root.join(dir);
        let module = std::fs::read_to_string(dir.join("mod.rs")).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        let mut expected: BTreeSet<String> = group_names()
            .into_iter()
            .map(|g| format!("{}.rs", g.replace('-', "_")))
            .collect();
        expected.insert("datatypes.rs".to_owned());
        for file in &expected {
            let stem = file.trim_end_matches(".rs");
            if !dir.join(file).is_file() {
                problems.push(format!("{} has no {file}", dir.display()));
            } else if !module.lines().any(|l| l.trim() == format!("mod {stem};")) {
                problems.push(format!("{}/mod.rs does not declare mod {stem}", dir.display()));
            }
        }
        expected.insert("mod.rs".to_owned());
        for entry in std::fs::read_dir(&dir).expect("a directory") {
            let name = entry.expect("an entry").file_name().to_string_lossy().into_owned();
            if !expected.contains(&name) {
                problems.push(format!("{} holds {name}, which is no group's file", dir.display()));
            }
        }
    }
    let rules = root.join("oracles/rules");
    for group in group_names() {
        if !rules.join(format!("{group}.rls")).is_file() {
            problems.push(format!("oracles/rules/ has no {group}.rls"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_w3c_reports_map_names_every_rule() {
    let mapped: BTreeSet<&str> = w3c::LIKELY_RULES
        .iter()
        .flat_map(|(_, _, rules)| rules.iter().copied())
        .collect();
    let table = spec_table();
    let missing: Vec<&String> = table
        .values()
        .flatten()
        .filter(|r| !mapped.contains(r.as_str()))
        .collect();
    assert!(missing.is_empty(), "w3c::LIKELY_RULES does not map {missing:?}");
    let known: BTreeSet<&str> = table.values().flatten().map(String::as_str).collect();
    let unknown: Vec<&&str> = mapped.iter().filter(|r| !known.contains(**r)).collect();
    assert!(
        unknown.is_empty(),
        "w3c::LIKELY_RULES names {unknown:?}, which the spec's table does not list"
    );
}

/// The codes a configuration's program exports and the rules it holds.
#[test]
fn a_configurations_program_lists_its_codes_and_rules() {
    let default = Switches::default();
    let checks = [
        "cax-dw",
        "prp-pdw",
        "cls-nothing2",
        "prp-asyp",
        "prp-irp",
        "cls-com",
        "eq-diff1",
        "eq-diff2",
        "eq-diff3",
        "cax-adc",
        "prp-adp",
        "prp-npa1",
        "prp-npa2",
        "cls-maxc1",
        "cls-maxqc1",
        "cls-maxqc2",
    ];
    assert_eq!(programs::codes(default), checks);
    assert_eq!(nemo::codes(default), checks);
    let no_checks = default.off(Switch::Checks);
    assert!(programs::codes(no_checks).is_empty());
    assert!(nemo::codes(no_checks).is_empty());
    let rules = programs::rules(default);
    for rule in ["cax-sco", "cls-thing", "prp-spo2", "eq-rep-o", "cax-dw", "prp-key"] {
        assert!(rules.contains(rule), "{rule}");
    }
    // dt-not-type is left out: Nemo reads literals as IRIs.
    assert!(!rules.contains("dt-not-type"), "{rules:?}");
    let chains = programs::rules(default.off(Switch::PropertyChains));
    assert!(!chains.contains("prp-spo2"));
}
