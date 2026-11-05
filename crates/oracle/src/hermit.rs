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

/// Checks every inferred `rdfs:subClassOf`, `owl:equivalentClass`,
/// `rdfs:subPropertyOf` and `owl:equivalentProperty` triple between IRIs in
/// the closure (not in the input) against HermiT's classification of the
/// input. `Err` is the outcome when HermiT is missing or fails, or when the
/// input is not OWL 2 DL ([`reserved_same_as`]: not run).
pub fn check(input: &Input, closure: &Closure) -> Result<Subsumptions, Outcome> {
    if let Some(t) = reserved_same_as(input) {
        return Err(Outcome::Skipped(format!(
            "HermiT: not OWL 2 DL, not judged ({t} makes a reserved IRI equal to another term)"
        )));
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
    let asserted: HashSet<&Triple> = input.triples().iter().collect();
    let wanted: Vec<(Kind, NamedNode, NamedNode)> = closure
        .triples
        .iter()
        .filter(|t| !asserted.contains(t))
        .flat_map(subsumptions)
        .collect();
    if wanted.is_empty() {
        return Ok(Subsumptions::default());
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
}
