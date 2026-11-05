//! The one rule source (`oracles/rules/`) and the programs composed from it
//! (`turbowl_oracle::programs`):
//!
//! - the Soufflé and VLog programs of the default and the complete
//!   configuration equal the golden programs in `tests/programs/`
//!   (`BLESS=1` rewrites them; read the diff before keeping it);
//! - every rule of the spec's switch table is named in exactly one group
//!   file, its own group's;
//! - Nemo runs the complete configuration.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use turbowl::{Switch, Switches};
use turbowl_oracle::{Input, Outcome, nemo, programs, workspace_root};

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
