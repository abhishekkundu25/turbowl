//! The minimal inputs: one per rule and per diagnostic, and a few inputs
//! that test how lists, equality and literals are read, each with what the
//! closures must and must not hold under the W3C OWL 2 RL rules.
//!
//! # Groups and files
//!
//! The inputs are split as the rules are (ADR 0005): one file per switch
//! group (`core.rs`, then [`Switch::name`] with `_` for `-`), and
//! `datatypes.rs` for dt-not-type, which is in the `checks` group. Each case
//! knows its group ([`Case::group`], the `GROUP` of its file). Its written
//! expectations are checked only in a configuration where that group is on
//! ([`Case::checked_in`]); the oracles are compared on it in every
//! configuration. An expected finding counts only where its code's group is
//! on (the `codes` given to [`Case::check`]). A case may lean on the groups
//! of the default configuration besides its own: the suite checks
//! expectations in the default and the complete configuration only.
//!
//! Expectations are lines `<mark> <triple or finding>` in a Turtle-like short
//! form (prefixes `ex:`, `rdf:`, `rdfs:`, `owl:`, `xsd:`):
//!
//! | Mark | Meaning |
//! |---|---|
//! | `+` | in every closure, OWL-RL's included |
//! | `+V` | in every closure of the configuration's rules (Nemo, TurbOWL, the reference evaluator) |
//! | `-` | in no closure of the configuration's rules (OWL-RL implements more rules) |
//! | `=` | a closure of the configuration's rules is exactly the `=` lines |
//! | `!` | a finding (code, then terms) every oracle covering that code reports; no other finding is reported |

mod checks;
mod core;
mod datatypes;
mod equality;
mod identity;
mod literal_values;
mod property_chains;
mod schema;
mod trivial;

use crate::Input;
use crate::closure::{terms_in, triple};
use crate::{Closure, Finding};
use oxrdf::Triple;
use turbowl::{Switch, Switches};

/// One minimal input.
#[derive(Debug, Clone, Copy)]
pub struct Case {
    /// The group of the rules it is about: `None` for the core, or the
    /// switch.
    pub group: Option<Switch>,
    /// A short name (`prp-dom`).
    pub name: &'static str,
    /// The W3C rules (or the reading of lists and literals) it covers.
    pub covers: &'static str,
    /// The input, in Turtle, without prefixes.
    pub turtle: &'static str,
    pub expect: &'static [&'static str],
}

const PREFIXES: &str = "@prefix ex: <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

const NAMESPACES: [(&str, &str); 5] = [
    ("ex:", "http://example.org/"),
    ("rdf:", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("rdfs:", "http://www.w3.org/2000/01/rdf-schema#"),
    ("owl:", "http://www.w3.org/2002/07/owl#"),
    ("xsd:", "http://www.w3.org/2001/XMLSchema#"),
];

/// What kind of closure an oracle gives, for [`Case::check`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// OWL-RL: the full W3C rule set, and axiomatic triples.
    FullRl,
    /// The rules of one configuration: Nemo, TurbOWL, the reference
    /// evaluator, each run with the same switches.
    RuleSet,
}

impl Case {
    /// The input.
    pub fn input(&self) -> Input {
        Input::turtle(self.name, &format!("{PREFIXES}{}", self.turtle))
    }

    /// True when its group is on in `switches`: its expectations are checked
    /// in that configuration.
    pub fn checked_in(&self, switches: Switches) -> bool {
        self.group.is_none_or(|g| switches.is_on(g))
    }

    /// Checks a closure of this input against the expectations. `codes`: the
    /// finding codes the oracle covers in the configuration. Returns the
    /// failed expectations.
    pub fn check(&self, closure: &Closure, kind: Kind, codes: &[&str]) -> Vec<String> {
        let mut failed = Vec::new();
        let mut exact: Vec<Triple> = Vec::new();
        let mut findings: Vec<Finding> = Vec::new();
        for line in self.expect {
            let (mark, rest) = line.split_once(' ').unwrap_or((line, ""));
            let (want, all) = match mark {
                "+" => (true, true),
                "+V" => (true, false),
                "-" => (false, false),
                "=" => {
                    exact.push(triple(&expand(rest)));
                    continue;
                }
                "!" => {
                    let (code, terms) = rest.split_once(' ').unwrap_or((rest, ""));
                    findings.push(Finding::new(code, terms_in(&expand(terms))));
                    continue;
                }
                _ => panic!("{}: unknown mark in {line:?}", self.name),
            };
            let applies = all || kind == Kind::RuleSet;
            if applies && closure.triples.contains(&triple(&expand(rest))) != want {
                failed.push((*line).to_owned());
            }
        }
        if kind == Kind::RuleSet && !exact.is_empty() {
            let want: std::collections::HashSet<Triple> = exact.into_iter().collect();
            if want != closure.triples {
                failed.push(format!(
                    "= closure has {} triples, expected exactly {}",
                    closure.triples.len(),
                    want.len()
                ));
            }
        }
        let want: std::collections::HashSet<&Finding> =
            findings.iter().filter(|f| codes.contains(&f.code.as_str())).collect();
        let got: std::collections::HashSet<&Finding> = closure
            .findings
            .iter()
            .filter(|f| codes.contains(&f.code.as_str()))
            .collect();
        for f in want.difference(&got) {
            failed.push(format!("! missing {f}"));
        }
        for f in got.difference(&want) {
            failed.push(format!("! unexpected {f}"));
        }
        failed
    }
}

/// Expands the short prefixes outside quoted strings into full IRIs.
pub fn expand(short: &str) -> String {
    let mut out = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut i = 0;
    while i < short.len() {
        let rest = &short[i..];
        let c = rest.chars().next().expect("in bounds");
        if quoted {
            out.push(c);
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => quoted = false,
                _ => {}
            }
            i += c.len_utf8();
            continue;
        }
        let boundary = out.is_empty() || out.ends_with([' ', '^', '(']);
        if let Some((prefix, ns)) = NAMESPACES.iter().find(|(p, _)| boundary && rest.starts_with(p)) {
            let local: String = rest[prefix.len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
                .collect();
            out.push_str(&format!("<{ns}{local}>"));
            i += prefix.len() + local.len();
            continue;
        }
        if c == '"' {
            quoted = true;
        }
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// The files of a group (`None`: the core), each with the `GROUP` it
/// declares. A match, so a new switch has no inputs until it has its file.
fn files(group: Option<Switch>) -> &'static [(Option<Switch>, &'static [Case])] {
    match group {
        None => &[(self::core::GROUP, self::core::CASES)],
        Some(Switch::Equality) => &[(equality::GROUP, equality::CASES)],
        Some(Switch::PropertyChains) => &[(property_chains::GROUP, property_chains::CASES)],
        Some(Switch::Checks) => &[(checks::GROUP, checks::CASES), (datatypes::GROUP, datatypes::CASES)],
        Some(Switch::Identity) => &[(identity::GROUP, identity::CASES)],
        Some(Switch::Schema) => &[(schema::GROUP, schema::CASES)],
        Some(Switch::Trivial) => &[(trivial::GROUP, trivial::CASES)],
        Some(Switch::LiteralValues) => &[(literal_values::GROUP, literal_values::CASES)],
    }
}

/// The groups, in order: the core, then [`Switch::ALL`].
fn groups() -> impl Iterator<Item = Option<Switch>> {
    std::iter::once(None).chain(Switch::ALL.into_iter().map(Some))
}

/// The minimal inputs of every group, the core first, then each switch in
/// [`Switch::ALL`] order.
pub fn minimal() -> Vec<Case> {
    groups()
        .flat_map(files)
        .flat_map(|(_, cases)| cases.iter().copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_prefixes_outside_strings() {
        assert_eq!(
            expand("ex:a rdfs:label \"ex:b\"^^xsd:string"),
            "<http://example.org/a> <http://www.w3.org/2000/01/rdf-schema#label> \"ex:b\"^^<http://www.w3.org/2001/XMLSchema#string>"
        );
    }

    /// Each file declares the group it is filed under, each case is of its
    /// file's group, and names are unique.
    #[test]
    fn every_case_is_of_its_files_group() {
        for group in groups() {
            for (declared, cases) in files(group) {
                assert_eq!(*declared, group);
                for case in *cases {
                    assert_eq!(case.group, group, "{}", case.name);
                }
            }
        }
        let mut names: Vec<&str> = minimal().iter().map(|c| c.name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "two cases share a name");
    }

    #[test]
    fn expectations_are_checked_where_the_group_is_on() {
        let case = |name| minimal().into_iter().find(|c| c.name == name).expect("a case");
        let no_checks = Switches::default().off(Switch::Checks);
        assert!(case("prp-dom").checked_in(no_checks));
        assert!(!case("cax-dw").checked_in(no_checks));
        assert!(!case("dt-not-type").checked_in(no_checks));
        assert!(case("cax-dw").checked_in(Switches::default()));
        assert!(!case("not-eq-ref").checked_in(Switches::default()));
        assert!(case("not-eq-ref").checked_in(Switches::complete()));
    }

    #[test]
    fn every_case_parses() {
        for case in minimal() {
            let _ = case.input();
            let _ = case.check(&Closure::default(), Kind::RuleSet, &[]);
        }
    }
}
