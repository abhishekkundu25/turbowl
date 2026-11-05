//! The harness the rule families' tests share: parse a small Turtle
//! document, run families to the fixpoint the way the reasoner does (their
//! facts, then rounds until nothing is new), and read the closure and the
//! findings back as text.
//!
//! ```text
//! let run = testing::run(&["class"], "ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog .");
//! assert_eq!(run.inferred(), testing::expand("ex:rex rdf:type ex:Animal"));
//! ```
//!
//! Text forms, so expectations stay short and diffs readable:
//! - a term as N-Triples writes it, except a blank node: `_:` and the label
//!   the input gave it (tests use explicit labels such as `_:l1`; Turtle's
//!   `( ... )` and `[ ... ]` get generated labels, fine in inputs but not in
//!   expectations);
//! - a triple: `s p o`, no final dot; generalized triples (a literal
//!   subject) are written the same way;
//! - a finding: `code t1 t2 ...`, its terms in the rule's order.
//!
//! [`expand`] turns lines such as `ex:rex rdf:type ex:Animal` or
//! `cax-dw ex:x ex:A ex:B` into those forms. Inputs get the prefixes `rdf:`,
//! `rdfs:`, `owl:`, `xsd:` and `ex:` (`http://example.org/`) declared.

// The families' tests (tickets 07 to 09) use what the harness's own tests do not.
#![allow(dead_code)]

use super::{Context, FAMILIES, Family, Finding};
use crate::Format;
use crate::engine::Store;
use crate::term::{Dictionary, Term, TermId};
use std::collections::BTreeSet;

const NAMESPACES: &[(&str, &str)] = &[
    ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
    ("owl", "http://www.w3.org/2002/07/owl#"),
    ("xsd", "http://www.w3.org/2001/XMLSchema#"),
    ("ex", "http://example.org/"),
];

/// The registered family called `name` (panics when there is none).
pub(crate) fn family(name: &str) -> &'static Family {
    FAMILIES
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no rule family {name:?}"))
}

/// A run of some families over a base, kept so that a test can add more
/// triples and run again (an incremental run).
#[derive(Debug)]
pub(crate) struct Run {
    pub(crate) dict: Dictionary,
    pub(crate) store: Store,
    /// The base triples, `[s, p, o]`.
    pub(crate) base: BTreeSet<[u32; 3]>,
    pub(crate) findings: Vec<Finding>,
    /// Rounds run so far, over every call.
    pub(crate) rounds: usize,
    families: Vec<&'static Family>,
}

/// Runs the registered families `names` over a Turtle document to the fixpoint.
pub(crate) fn run(names: &[&str], turtle: &str) -> Run {
    run_families(names.iter().map(|n| family(n)).collect(), turtle)
}

/// Runs every registered family over a Turtle document to the fixpoint.
pub(crate) fn run_all(turtle: &str) -> Run {
    run_families(FAMILIES.iter().collect(), turtle)
}

/// Runs `families` (registered or a test's own) over a Turtle document.
pub(crate) fn run_families(families: Vec<&'static Family>, turtle: &str) -> Run {
    let mut run = Run {
        dict: Dictionary::new(),
        store: Store::new(),
        base: BTreeSet::new(),
        findings: Vec::new(),
        rounds: 0,
        families,
    };
    for f in &run.families {
        for [s, p, o] in f.facts {
            run.store.insert(s.0, p.0, o.0);
        }
    }
    run.add(turtle);
    run
}

impl Run {
    /// Adds a Turtle document (a new document: its blank nodes are its own)
    /// to the base and runs again from where the last run stopped.
    pub(crate) fn add(&mut self, turtle: &str) {
        let mut text = String::new();
        for (prefix, ns) in NAMESPACES {
            text.push_str(&format!("@prefix {prefix}: <{ns}> .\n"));
        }
        text.push_str(turtle);
        let mut triples = Vec::new();
        crate::io::parse(&mut self.dict, text.as_bytes(), Format::Turtle, None, None, |t| {
            triples.push(t)
        })
        .unwrap_or_else(|e| panic!("the test input is not valid Turtle: {e}"));
        for [s, p, o] in triples {
            if self.base.insert([s.0, p.0, o.0]) {
                self.store.insert(s.0, p.0, o.0);
            }
        }
        let Run {
            dict,
            store,
            findings,
            families,
            ..
        } = self;
        let mut cx = Context { terms: dict, findings };
        self.rounds += store.run(|round| {
            for f in families.iter() {
                (f.rules)(round, &mut cx);
            }
        });
    }

    /// A term in the text form (see the module doc).
    pub(crate) fn term(&self, id: u32) -> String {
        match self.dict.term(TermId(id)) {
            Term::Blank { label, .. } => format!("_:{label}"),
            _ => self.dict.to_oxrdf(TermId(id)).to_string(),
        }
    }

    fn line(&self, ids: &[u32]) -> String {
        ids.iter().map(|&id| self.term(id)).collect::<Vec<_>>().join(" ")
    }

    /// Every triple the store holds (base, facts and derived), generalized
    /// triples included.
    pub(crate) fn closure(&self) -> BTreeSet<String> {
        self.store.triples().map(|(s, p, o)| self.line(&[s, p, o])).collect()
    }

    /// The closure without the base triples.
    pub(crate) fn inferred(&self) -> BTreeSet<String> {
        self.store
            .triples()
            .filter(|&(s, p, o)| !self.base.contains(&[s, p, o]))
            .map(|(s, p, o)| self.line(&[s, p, o]))
            .collect()
    }

    /// The findings, deduplicated: `code t1 t2 ...`.
    pub(crate) fn findings(&self) -> BTreeSet<String> {
        self.findings
            .iter()
            .map(|f| {
                let mut line = f.code.to_owned();
                for &t in &f.terms {
                    line.push(' ');
                    line.push_str(&self.term(t));
                }
                line
            })
            .collect()
    }

    /// True when the closure holds the triple written as `expand` reads it.
    pub(crate) fn has(&self, triple: &str) -> bool {
        let wanted = expand(triple);
        assert_eq!(wanted.len(), 1, "has() takes one triple");
        self.closure().is_superset(&wanted)
    }
}

/// Lines of triples or findings in the text form, from a short notation:
/// one per line, tokens separated by spaces (so no spaces inside a
/// literal), an optional final ` .`; `a` is `rdf:type`; `prefix:local`
/// with a declared prefix is an IRI; `"lex"^^prefix:local` is a typed
/// literal (`xsd:string` written as a plain literal, as N-Triples does);
/// `<...>`, `_:label`, `"lex"`, `"lex"@lang` and finding codes stay as they are.
pub(crate) fn expand(lines: &str) -> BTreeSet<String> {
    lines
        .lines()
        .map(|l| l.trim().trim_end_matches('.').trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.split_whitespace().map(expand_token).collect::<Vec<_>>().join(" "))
        .collect()
}

fn expand_token(token: &str) -> String {
    if token == "a" {
        return format!("<{}type>", NAMESPACES[0].1);
    }
    if token.starts_with('"') {
        if let Some((lexical, datatype)) = token.rsplit_once("^^") {
            let datatype = expand_token(datatype);
            if datatype == format!("<{}string>", NAMESPACES[3].1) {
                return lexical.to_owned();
            }
            return format!("{lexical}^^{datatype}");
        }
        return token.to_owned();
    }
    if token.starts_with('<') || token.starts_with("_:") {
        return token.to_owned();
    }
    match token.split_once(':') {
        Some((prefix, local)) => match NAMESPACES.iter().find(|(p, _)| *p == prefix) {
            Some((_, ns)) => format!("<{ns}{local}>"),
            None => token.to_owned(),
        },
        None => token.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Round;
    use crate::term::vocab;

    /// x rdf:type y gives y ex:typeOf x (a toy rule), and a finding for
    /// every literal subject.
    fn toy(round: &mut Round<'_>, cx: &mut Context<'_>) {
        let type_of = cx
            .terms
            .get_iri("http://example.org/typeOf")
            .expect("interned by the input");
        if let Some(types) = round.by_subject(vocab::RDF_TYPE.0) {
            let recent: Vec<(u32, u32)> = types.recent().iter().copied().collect();
            for (x, y) in recent {
                round.emit(y, type_of.0, x);
                if matches!(cx.terms.term(TermId(x)), Term::Literal { .. }) {
                    cx.report("toy", &[x]);
                }
            }
        }
    }

    static TOY: Family = Family {
        name: "toy",
        rules: toy,
        facts: &[[vocab::OWL_THING, vocab::RDF_TYPE, vocab::OWL_CLASS]],
    };

    #[test]
    fn the_harness_runs_a_family_and_reads_back_text() {
        let mut run = run_families(vec![&TOY], "ex:x a ex:C . _:b1 ex:typeOf ex:y .");
        assert_eq!(
            run.inferred(),
            expand(
                "ex:C ex:typeOf ex:x .
                 owl:Thing rdf:type owl:Class
                 owl:Class ex:typeOf owl:Thing"
            )
        );
        assert!(run.has("_:b1 ex:typeOf ex:y"));
        assert!(run.findings().is_empty());
        // An incremental run continues from the stored state.
        run.add("ex:z a ex:C .");
        assert!(run.has("ex:C ex:typeOf ex:z"));
        assert_eq!(run.base.len(), 3);
    }

    #[test]
    fn expand_reads_the_short_notation() {
        assert_eq!(
            expand("ex:a a \"1\"^^xsd:integer .\n\"s\"^^xsd:string rdfs:label \"x\"@en\ncax-dw _:b <http://e/> ex:C"),
            BTreeSet::from([
                "<http://example.org/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \"1\"^^<http://www.w3.org/2001/XMLSchema#integer>".to_owned(),
                "\"s\" <http://www.w3.org/2000/01/rdf-schema#label> \"x\"@en".to_owned(),
                "cax-dw _:b <http://e/> <http://example.org/C>".to_owned(),
            ])
        );
    }

    #[test]
    fn findings_and_generalized_triples_are_text_too() {
        // A literal typed by a base triple cannot be written in Turtle, so
        // the toy rule's finding comes from an inserted generalized triple.
        let mut run = run_families(vec![&TOY], "ex:x ex:p \"5\"^^xsd:integer . ex:typeOf a rdf:Property .");
        let five = run
            .dict
            .get_term(
                oxrdf::LiteralRef::new_typed_literal("5", oxrdf::vocab::xsd::INTEGER).into(),
                crate::term::DocId::NONE,
            )
            .expect("interned")
            .0;
        run.store.insert(five, vocab::RDF_TYPE.0, vocab::OWL_THING.0);
        run.add("");
        assert!(!run.has("\"5\"^^xsd:integer ex:typeOf ex:x"));
        assert!(run.has("owl:Thing ex:typeOf \"5\"^^xsd:integer"));
        assert_eq!(run.findings(), expand("toy \"5\"^^xsd:integer"));
    }
}
