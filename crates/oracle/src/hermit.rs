//! The DL reasoner oracle: HermiT 1.3.8 on a Temurin 21 JRE, both pinned and
//! fetched into `tools/` by `scripts/fetch-tools.sh`. It classifies the input
//! (classes, object and data properties) and checks that every class and
//! property subsumption a closure inferred is entailed: full OWL 2 DL must
//! entail everything a sound OWL 2 RL reasoner infers between named terms.
//!
//! Konclude was the other candidate; it has no arm64 macOS build and reads
//! only OWL/XML and functional syntax (see `scripts/fetch-tools.sh`).

use crate::process::{run as run_process, scratch_dir, timeout};
use crate::{Closure, Input, Outcome, fetched_version, tools_dir};
use oxrdf::vocab::rdfs;
use oxrdf::{NamedNode, NamedNodeRef, Term, Triple};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The pinned versions.
pub const VERSION: &str = "HermiT 1.3.8 on Temurin JRE 21.0.13+9";

const OWL: &str = "http://www.w3.org/2002/07/owl#";
const EQUIVALENT_CLASS: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/2002/07/owl#equivalentClass");
const EQUIVALENT_PROPERTY: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2002/07/owl#equivalentProperty");

/// The result of checking a closure's subsumptions.
#[derive(Debug, Clone, Default)]
pub struct Subsumptions {
    /// Inferred subsumptions between named terms that were checked.
    pub checked: usize,
    /// Inferred subsumptions not judged, as `sub ⊑ sup`: a term of each is
    /// not declared in the input (as `owl:Class`, `owl:ObjectProperty` or
    /// `owl:DatatypeProperty`).
    pub not_judged: Vec<String>,
    /// The ones HermiT does not entail, as `sub ⊑ sup`.
    pub not_entailed: Vec<String>,
    /// HermiT found the input inconsistent (so it entails everything).
    pub inconsistent: bool,
}

fn java() -> PathBuf {
    tools_dir().join("jre/bin/java")
}

fn jar() -> PathBuf {
    tools_dir().join("hermit/HermiT.jar")
}

/// The namespaces of the reserved vocabulary (OWL 2 Structural Specification,
/// section 2.4): IRIs in them cannot be individuals in OWL 2 DL.
const RESERVED: [&str; 4] = [
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "http://www.w3.org/2000/01/rdf-schema#",
    "http://www.w3.org/2002/07/owl#",
    "http://www.w3.org/2001/XMLSchema#",
];

/// An input triple that makes a reserved IRI `owl:sameAs` another term,
/// if there is one. Such an input is not in OWL 2 DL (the reserved IRI
/// would be an individual), so a DL reasoner has nothing to say about what
/// the OWL 2 RL rules infer from it, as with `owl.n3`.
pub fn reserved_same_as(input: &Input) -> Option<&Triple> {
    let reserved = |t: &Term| matches!(t, Term::NamedNode(n) if RESERVED.iter().any(|ns| n.as_str().starts_with(ns)));
    input.triples().iter().find(|t| {
        t.predicate.as_str() == "http://www.w3.org/2002/07/owl#sameAs"
            && (reserved(&t.subject.clone().into()) || reserved(&t.object))
    })
}

/// The terms the input declares, by kind: classes typed `owl:Class`,
/// properties typed `owl:ObjectProperty` or `owl:DatatypeProperty`, and the
/// built-in classes and properties of OWL. OWL 2 DL needs these
/// declarations to tell what a term is: without them the OWL API drops the
/// axioms that name it (`p owl:equivalentProperty q` on undeclared p and q
/// gives HermiT nothing), so HermiT has nothing to say about a subsumption
/// between undeclared terms.
fn declared(input: &Input) -> HashSet<(Kind, String)> {
    let mut terms: HashSet<(Kind, String)> = ["Thing", "Nothing"]
        .into_iter()
        .map(|c| (Kind::Class, format!("{OWL}{c}")))
        .chain(
            [
                "topObjectProperty",
                "bottomObjectProperty",
                "topDataProperty",
                "bottomDataProperty",
            ]
            .into_iter()
            .map(|p| (Kind::Property, format!("{OWL}{p}"))),
        )
        .collect();
    for t in input.triples() {
        let (oxrdf::NamedOrBlankNode::NamedNode(s), Term::NamedNode(o)) = (&t.subject, &t.object) else {
            continue;
        };
        if t.predicate.as_str() != "http://www.w3.org/1999/02/22-rdf-syntax-ns#type" {
            continue;
        }
        let kind = match o.as_str().strip_prefix(OWL) {
            Some("Class") => Kind::Class,
            Some("ObjectProperty" | "DatatypeProperty") => Kind::Property,
            _ => continue,
        };
        terms.insert((kind, s.as_str().to_owned()));
    }
    terms
}

/// Checks every inferred `rdfs:subClassOf`, `owl:equivalentClass`,
/// `rdfs:subPropertyOf` and `owl:equivalentProperty` triple between IRIs in
/// the closure (not in the input) against HermiT's classification of the
/// input. A subsumption is judged only when the input declares both its
/// terms ([`declared`]); the others are listed as not judged. `Err` is the
/// outcome when HermiT is missing or fails, or when the input is not OWL 2
/// DL ([`reserved_same_as`], or no inferred subsumption between declared
/// terms: not run).
pub fn check(input: &Input, closure: &Closure) -> Result<Subsumptions, Outcome> {
    if let Some(t) = reserved_same_as(input) {
        return Err(Outcome::Skipped(format!(
            "HermiT: not OWL 2 DL, not judged ({t} makes a reserved IRI equal to another term)"
        )));
    }
    let asserted: HashSet<&Triple> = input.triples().iter().collect();
    let declared = declared(input);
    let is_declared = |kind: Kind, n: &NamedNode| declared.contains(&(kind, n.as_str().to_owned()));
    let (wanted, undeclared): (Vec<(Kind, NamedNode, NamedNode)>, Vec<_>) = closure
        .triples
        .iter()
        .filter(|t| !asserted.contains(t))
        .flat_map(subsumptions)
        .partition(|(kind, sub, sup)| is_declared(*kind, sub) && is_declared(*kind, sup));
    let mut not_judged: Vec<String> = undeclared
        .iter()
        .map(|(_, sub, sup)| format!("{sub} ⊑ {sup}"))
        .collect();
    not_judged.sort();
    not_judged.dedup();
    if wanted.is_empty() {
        if let Some(first) = not_judged.first() {
            return Err(Outcome::Skipped(format!(
                "HermiT: not OWL 2 DL, not judged ({} inferred subsumptions, such as {first}, name a term the \
                 input does not declare as owl:Class, owl:ObjectProperty or owl:DatatypeProperty)",
                not_judged.len()
            )));
        }
        return Ok(Subsumptions::default());
    }
    if fetched_version(&tools_dir().join("jre")).is_none() || !java().exists() {
        return Err(Outcome::skipped(
            "HermiT",
            "tools/jre is missing (no pinned JRE for this platform, or scripts/fetch-tools.sh not run)",
        ));
    }
    if fetched_version(&tools_dir().join("hermit")).as_deref() != Some("1.3.8") || !jar().exists() {
        return Err(Outcome::skipped(
            "HermiT",
            "tools/hermit/HermiT.jar 1.3.8 is missing: run scripts/fetch-tools.sh",
        ));
    }
    let dir = scratch_dir("hermit").map_err(Outcome::Failed)?;
    // N-Triples is Turtle; the bundled OWL API picks its parser by content.
    let in_path = dir.join("input.ttl");
    std::fs::write(&in_path, input.to_ntriples()).map_err(|e| Outcome::Failed(e.to_string()))?;
    let mut taxonomy = Taxonomy::default();
    // Classes and object properties first; data properties (another JVM run)
    // only when a property subsumption is still unexplained.
    for (flags, out) in [(&["-c", "-O"][..], "objects.txt"), (&["-D"][..], "data.txt")] {
        if flags == ["-D"]
            && wanted
                .iter()
                .all(|(k, sub, sup)| taxonomy.entails(*k, sub.as_str(), sup.as_str()))
        {
            break;
        }
        match classify(&dir, &in_path, flags, &dir.join(out)) {
            Ok(text) => taxonomy.read(&text),
            Err(e) if e.contains("nconsistent") => {
                return Ok(Subsumptions {
                    checked: wanted.len(),
                    not_judged,
                    not_entailed: Vec::new(),
                    inconsistent: true,
                });
            }
            Err(e) => return Err(Outcome::Failed(format!("HermiT: {e}"))),
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    let mut result = Subsumptions {
        checked: wanted.len(),
        not_judged,
        ..Subsumptions::default()
    };
    for (kind, sub, sup) in wanted {
        if !taxonomy.entails(kind, sub.as_str(), sup.as_str()) {
            result.not_entailed.push(format!("{sub} ⊑ {sup}"));
        }
    }
    result.not_entailed.sort();
    Ok(result)
}

fn classify(dir: &Path, input: &Path, flags: &[&str], out: &Path) -> Result<String, String> {
    let mut cmd = Command::new(java());
    cmd.arg("-jar")
        .arg(jar())
        .args(flags)
        .arg("-o")
        .arg(out)
        .arg(format!("file://{}", input.display()));
    run_process(&mut cmd, dir, timeout())?;
    std::fs::read_to_string(out).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Class,
    Property,
}

/// The subsumptions a triple states between two IRIs.
fn subsumptions(t: &Triple) -> Vec<(Kind, NamedNode, NamedNode)> {
    let (oxrdf::NamedOrBlankNode::NamedNode(s), Term::NamedNode(o)) = (&t.subject, &t.object) else {
        return Vec::new();
    };
    let (s, o) = (s.clone(), o.clone());
    match t.predicate.as_ref() {
        p if p == rdfs::SUB_CLASS_OF => vec![(Kind::Class, s, o)],
        p if p == rdfs::SUB_PROPERTY_OF => vec![(Kind::Property, s, o)],
        p if p == EQUIVALENT_CLASS => vec![(Kind::Class, s.clone(), o.clone()), (Kind::Class, o, s)],
        p if p == EQUIVALENT_PROPERTY => vec![(Kind::Property, s.clone(), o.clone()), (Kind::Property, o, s)],
        _ => Vec::new(),
    }
}

/// HermiT's classification: direct subsumptions, by kind.
#[derive(Debug, Default)]
struct Taxonomy {
    up: HashMap<(Kind, String), HashSet<String>>,
}

impl Taxonomy {
    /// Reads `SubClassOf( <a> <b> )`, `EquivalentClasses( <a> <b> ... )` and
    /// the object and data property forms.
    fn read(&mut self, text: &str) {
        for line in text.lines() {
            let Some((head, rest)) = line.trim().split_once('(') else {
                continue;
            };
            let kind = match head.trim() {
                "SubClassOf" | "EquivalentClasses" => Kind::Class,
                "SubObjectPropertyOf"
                | "EquivalentObjectProperties"
                | "SubDataPropertyOf"
                | "EquivalentDataProperties" => Kind::Property,
                _ => continue,
            };
            let names: Vec<String> = rest
                .trim_end()
                .trim_end_matches(')')
                .split_whitespace()
                .map(|n| match n.strip_prefix("owl:") {
                    Some(local) => format!("{OWL}{local}"),
                    None => n.trim_start_matches('<').trim_end_matches('>').to_owned(),
                })
                .collect();
            if head.trim().starts_with("Sub") {
                if let [a, b] = &names[..] {
                    self.edge(kind, a, b);
                }
            } else {
                for a in &names {
                    for b in &names {
                        self.edge(kind, a, b);
                    }
                }
            }
        }
    }

    fn edge(&mut self, kind: Kind, a: &str, b: &str) {
        self.up.entry((kind, a.to_owned())).or_default().insert(b.to_owned());
    }

    fn entails(&self, kind: Kind, sub: &str, sup: &str) -> bool {
        let (top, bottom) = match kind {
            Kind::Class => ("Thing", "Nothing"),
            Kind::Property => ("topObjectProperty", "bottomObjectProperty"),
        };
        let (top, bottom) = (format!("{OWL}{top}"), format!("{OWL}{bottom}"));
        if sub == sup
            || sup == top
            || sub == bottom
            || (kind == Kind::Property && sup == format!("{OWL}topDataProperty"))
        {
            return true;
        }
        let mut seen: HashSet<&str> = HashSet::new();
        let mut stack = vec![sub];
        while let Some(n) = stack.pop() {
            if !seen.insert(n) {
                continue;
            }
            if n == sup || n == bottom {
                return true;
            }
            if let Some(ups) = self.up.get(&(kind, n.to_owned())) {
                stack.extend(ups.iter().map(String::as_str));
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_taxonomy() {
        let mut t = Taxonomy::default();
        t.read(
            "SubClassOf( <http://example.org/A> <http://example.org/B> )\n\
             EquivalentClasses( <http://example.org/B> <http://example.org/C> )\n\
             EquivalentClasses( <http://example.org/U> owl:Nothing )\n\
             SubObjectPropertyOf( <http://example.org/p> <http://example.org/q> )\n",
        );
        let ex = |n: &str| format!("http://example.org/{n}");
        assert!(t.entails(Kind::Class, &ex("A"), &ex("C")));
        assert!(!t.entails(Kind::Class, &ex("C"), &ex("A")));
        assert!(t.entails(Kind::Class, &ex("U"), &ex("A")));
        assert!(t.entails(Kind::Property, &ex("p"), &ex("q")));
        assert!(!t.entails(Kind::Class, &ex("p"), &ex("q")));
    }

    #[test]
    fn an_input_equating_reserved_iris_is_not_judged() {
        let input = |turtle: &str| {
            Input::turtle(
                "t",
                &format!(
                    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                     @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
                     @prefix ex: <http://example.org/> .\n{turtle}"
                ),
            )
        };
        for turtle in [
            "rdfs:subClassOf owl:sameAs owl:equivalentClass .",
            "ex:t owl:sameAs rdfs:subClassOf .",
        ] {
            let i = input(turtle);
            assert!(reserved_same_as(&i).is_some(), "{turtle}");
            assert!(matches!(check(&i, &Closure::default()), Err(Outcome::Skipped(_))));
        }
        for turtle in ["ex:a owl:sameAs ex:b .", "ex:a rdfs:subClassOf owl:Thing ."] {
            assert!(reserved_same_as(&input(turtle)).is_none(), "{turtle}");
        }
    }

    /// prp-eqp's input: `p owl:equivalentProperty q` on undeclared
    /// properties, whose subproperty pairs the rules infer. Undeclared, the
    /// OWL API drops the axiom, so they are not judged; declared, HermiT
    /// judges and entails them.
    #[test]
    fn only_subsumptions_between_declared_terms_are_judged() {
        let prefixes = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix ex: <http://example.org/> .\n";
        let body = "ex:p owl:equivalentProperty ex:q . ex:a ex:p ex:b . ex:c ex:q ex:d .";
        let ex = |n: &str| format!("<http://example.org/{n}>");
        let spo = "<http://www.w3.org/2000/01/rdf-schema#subPropertyOf>";
        let closure = |input: &Input| {
            let inferred = [
                format!("{} {spo} {}", ex("p"), ex("q")),
                format!("{} {spo} {}", ex("q"), ex("p")),
            ];
            Closure::new(
                input
                    .triples()
                    .iter()
                    .cloned()
                    .chain(inferred.iter().map(|t| crate::closure::triple(t))),
                [],
            )
        };

        let undeclared = Input::turtle("prp-eqp", &format!("{prefixes}{body}"));
        match check(&undeclared, &closure(&undeclared)) {
            Err(Outcome::Skipped(why)) => assert!(why.contains("not judged"), "{why}"),
            other => panic!("undeclared properties were judged: {other:?}"),
        }
        let half = Input::turtle("half", &format!("{prefixes}ex:p a owl:ObjectProperty . {body}"));
        assert!(matches!(check(&half, &closure(&half)), Err(Outcome::Skipped(_))));

        let declared = Input::turtle(
            "prp-eqp-declared",
            &format!("{prefixes}ex:p a owl:ObjectProperty . ex:q a owl:ObjectProperty . {body}"),
        );
        match check(&declared, &closure(&declared)) {
            Ok(s) => {
                assert_eq!(s.checked, 2, "{s:?}");
                assert!(s.not_judged.is_empty(), "{s:?}");
                assert!(s.not_entailed.is_empty(), "{s:?}");
            }
            Err(Outcome::Skipped(why)) => println!("skipped: {why}"),
            Err(other) => panic!("HermiT: {other:?}"),
        }
    }
}
