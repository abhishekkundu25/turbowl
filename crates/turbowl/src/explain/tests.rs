//! The explainer through the public interface, and the parser and the
//! search inside it. Every proof here is checked for its shape
//! ([`well_formed`]); `turbowl_reference::check_proof` checks each step's
//! rule in the reference crate's tests and in chaos.

use super::program::{self, Kind, T};
use super::{Goal, Proof, Reason, Statement};
use crate::{Error, ExplainCache, Format, Reasoner, Switch, Switches};
use oxrdf::{BlankNode, Literal, NamedNode, NamedOrBlankNode, Term, Triple};

const PREFIXES: &str = "@prefix ex: <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
";

/// The configuration ticket 01 explains: the default, without equality.
fn switches() -> Switches {
    Switches::default().off(Switch::Equality)
}

fn reasoned(turtle: &str, switches: Switches) -> Reasoner {
    let mut r = Reasoner::with_switches(switches);
    r.load_str(&format!("{PREFIXES}{turtle}"), Format::Turtle, None)
        .unwrap_or_else(|e| panic!("{e}"));
    r.reason();
    r
}

fn iri(short: &str) -> NamedNode {
    let (prefix, local) = short.split_once(':').expect("a prefixed name");
    let namespace = PREFIXES
        .lines()
        .find_map(|l| l.strip_prefix(&format!("@prefix {prefix}: <")))
        .and_then(|rest| rest.strip_suffix("> ."))
        .expect("a known prefix");
    NamedNode::new_unchecked(format!("{namespace}{local}"))
}

fn triple(s: &str, p: &str, o: &str) -> Triple {
    Triple::new(iri(s), iri(p), iri(o))
}

fn statement(t: &Triple) -> Statement {
    Statement {
        subject: t.subject.clone().into(),
        predicate: t.predicate.clone().into(),
        object: t.object.clone(),
    }
}

/// The proof's shape: every premise an earlier step, every statement once,
/// every base step a base triple of `r` and every rule step a triple of its
/// closure (generalized ones aside), and the goal last.
fn well_formed(r: &Reasoner, proof: &Proof) {
    let base: Vec<Statement> = r
        .triples()
        .filter(|t| r.inferred().all(|i| i != *t))
        .map(|t| statement(&t))
        .collect();
    let closure: Vec<Statement> = r.triples().map(|t| statement(&t)).collect();
    let mut seen = std::collections::HashSet::new();
    for (i, step) in proof.steps().iter().enumerate() {
        assert!(seen.insert(&step.statement), "{:?} twice in\n{proof}", step.statement);
        match &step.reason {
            Reason::Base => assert!(base.contains(&step.statement), "{:?} is not base", step.statement),
            Reason::Rule { premises, .. } => {
                assert!(premises.iter().all(|&p| p < i), "step {i} reads a later step:\n{proof}");
                if !step.statement.subject.is_literal() {
                    assert!(
                        closure.contains(&step.statement),
                        "{:?} is not in the closure",
                        step.statement
                    );
                }
            }
        }
    }
}

fn explain(r: &Reasoner, t: &Triple) -> Proof {
    let proof = r
        .explain(t.as_ref())
        .unwrap()
        .unwrap_or_else(|| panic!("no proof of {t}"));
    well_formed(r, &proof);
    assert_eq!(proof.goal(), Goal::Statement(&statement(t)));
    proof
}

/// The rule of each step, in order.
fn rules(proof: &Proof) -> Vec<&'static str> {
    proof
        .steps()
        .iter()
        .map(|s| match s.reason {
            Reason::Base => "base",
            Reason::Rule { rule, .. } => rule,
        })
        .collect()
}

#[test]
fn the_program_parses_with_tp_read_as_t() {
    let p = program::program();
    let lines = p.lines();
    assert!(
        lines
            .iter()
            .all(|l| !l.contains("Tp(") && !l.contains("usedProp(") && !l.contains("input("))
    );
    let rdf_type = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
    let dom = "<http://www.w3.org/2000/01/rdf-schema#domain>";
    assert!(
        lines.contains(&format!(
            "[core] T(?0, {rdf_type}, ?1) :- T(?2, {dom}, ?1), T(?0, ?2, ?3) . % prp-dom"
        )),
        "{lines:#?}"
    );
    // The named literals are literals again.
    assert!(lines.iter().any(
        |l| l.contains("\"1\"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger>")
            && l.starts_with("[identity] maxOne(")
    ));
    // Every rule of T and of a finding has a W3C name; helpers have none.
    let findings: Vec<&str> = [
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
        "dt-not-type",
    ]
    .to_vec();
    for code in &findings {
        assert!(p.finding(code).is_some(), "{code}");
    }
    for line in &lines {
        let named = !line.ends_with("% -");
        let head = line.split_once("] ").unwrap().1;
        let derives_t = head.starts_with("T(")
            || findings.iter().any(|c| {
                let rel = p.finding(c).unwrap();
                head.starts_with(&format!("{}(", p.relations[rel as usize].name))
            });
        // valueOf's copy through sameAs is named eq-rep-s (equality.rls).
        assert!(named == derives_t || head.starts_with("valueOf("), "{line}");
    }
    assert_eq!(p.relations[T as usize].kind, Kind::Triple);
    assert!(lines.len() > 100, "{} rules", lines.len());
}

#[test]
fn a_base_triple_is_its_own_proof() {
    let r = reasoned("ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog .", switches());
    let t = triple("ex:rex", "rdf:type", "ex:Dog");
    let proof = explain(&r, &t);
    assert_eq!(rules(&proof), ["base"]);
    assert_eq!(proof.depth(), 0);
    assert_eq!(proof.leaves().collect::<Vec<_>>(), [&statement(&t)]);
    assert_eq!(
        proof.to_string(),
        "<http://example.org/rex> rdf:type <http://example.org/Dog>  [base]\n"
    );
}

#[test]
fn a_rule_step_names_its_rule_and_premises() {
    let r = reasoned("ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog .", switches());
    let proof = explain(&r, &triple("ex:rex", "rdf:type", "ex:Animal"));
    assert_eq!(rules(&proof), ["base", "base", "cax-sco"]);
    assert_eq!(
        proof.reason(),
        &Reason::Rule {
            rule: "cax-sco",
            premises: vec![0, 1]
        }
    );
    assert_eq!(proof.depth(), 1);
    assert_eq!(proof.leaves().count(), 2);
    assert_eq!(
        proof.to_string(),
        "<http://example.org/rex> rdf:type <http://example.org/Animal>  [cax-sco]
  <http://example.org/Dog> rdfs:subClassOf <http://example.org/Animal>  [base]
  <http://example.org/rex> rdf:type <http://example.org/Dog>  [base]
"
    );
}

#[test]
fn an_axiom_step_has_no_premises() {
    let r = reasoned("ex:a ex:p ex:b .", switches());
    let proof = explain(&r, &triple("owl:Thing", "rdf:type", "owl:Class"));
    assert_eq!(rules(&proof), ["cls-thing"]);
    assert_eq!(proof.depth(), 1);
    assert_eq!(proof.leaves().count(), 0);
    let r = reasoned(
        "ex:a ex:p ex:b .",
        Switches::complete().off(Switch::Equality).off(Switch::LiteralValues),
    );
    let proof = explain(&r, &triple("rdfs:label", "rdf:type", "owl:AnnotationProperty"));
    assert_eq!(rules(&proof), ["prp-ap"]);
}

#[test]
fn a_triple_outside_the_closure_has_no_proof() {
    let r = reasoned("ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog .", switches());
    for t in [
        triple("ex:Animal", "rdfs:subClassOf", "ex:Dog"),
        triple("ex:unknown", "rdf:type", "ex:Dog"),
    ] {
        assert_eq!(r.explain(t.as_ref()).unwrap(), None, "{t}");
    }
}

/// The ranks: the shallowest proof is kept, whichever derivation the
/// search meets first.
#[test]
fn the_shallowest_proof_is_kept() {
    // ex:x a ex:D by cax-sco over a chain of four, or by prp-dom at once.
    let r = reasoned(
        "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C . ex:C rdfs:subClassOf ex:D .
         ex:x a ex:A . ex:p rdfs:domain ex:D . ex:x ex:p ex:y .",
        switches(),
    );
    let proof = explain(&r, &triple("ex:x", "rdf:type", "ex:D"));
    assert_eq!(proof.depth(), 1);
    assert_eq!(rules(&proof).last(), Some(&"prp-dom"));
    // A transitive chain of eight proves its ends in log2(8) = 3 steps.
    let chain: String = (0..8).map(|i| format!("ex:n{i} ex:next ex:n{} . ", i + 1)).collect();
    let r = reasoned(&format!("ex:next a owl:TransitiveProperty . {chain}"), switches());
    let proof = explain(&r, &triple("ex:n0", "ex:next", "ex:n8"));
    assert_eq!(proof.depth(), 3, "{proof}");
    assert!(rules(&proof).iter().all(|&r| r == "base" || r == "prp-trp"));
}

/// A symmetric property makes the closure cyclic (`a p b` gives `b p a`,
/// which gives `a p b`); the proof is not.
#[test]
fn a_cyclic_closure_gives_an_acyclic_proof() {
    let r = reasoned(
        "ex:p a owl:SymmetricProperty , owl:TransitiveProperty . ex:a ex:p ex:b . ex:b ex:p ex:c .",
        switches(),
    );
    for (s, o) in [("ex:b", "ex:a"), ("ex:a", "ex:a"), ("ex:c", "ex:a")] {
        let proof = explain(&r, &triple(s, "ex:p", o));
        assert!(proof.depth() <= 3, "{proof}");
    }
    let proof = explain(&r, &triple("ex:b", "ex:p", "ex:a"));
    assert_eq!(rules(&proof), ["base", "base", "prp-symp"]);
}

/// Lists are read through helper relations (`listCell`, `allTypes`, ...),
/// which fold into the step that reads them: its premises are the list's
/// triples.
#[test]
fn a_list_folds_into_the_step_that_reads_it() {
    let r = reasoned(
        "ex:C owl:intersectionOf ( ex:A ex:B ) . ex:x a ex:A , ex:B .",
        switches(),
    );
    let proof = explain(&r, &triple("ex:x", "rdf:type", "ex:C"));
    assert_eq!(rules(&proof).last(), Some(&"cls-int1"));
    let Reason::Rule { premises, .. } = proof.reason() else {
        panic!()
    };
    // C intersectionOf h, both cells' first and rest, and both typings.
    assert_eq!(premises.len(), 7, "{proof}");
    assert!(
        proof
            .steps()
            .iter()
            .all(|s| s.reason == Reason::Base || s.statement == statement(&triple("ex:x", "rdf:type", "ex:C")))
    );
    // A chain: prp-spo2 over the chain's list.
    let r = reasoned(
        "ex:uncle owl:propertyChainAxiom ( ex:parent ex:brother ) . ex:a ex:parent ex:b . ex:b ex:brother ex:c .",
        switches(),
    );
    let proof = explain(&r, &triple("ex:a", "ex:uncle", "ex:c"));
    assert_eq!(rules(&proof).last(), Some(&"prp-spo2"));
    assert_eq!(proof.leaves().count(), 7, "{proof}");
}

#[test]
fn a_diagnostic_is_the_root_of_its_proof() {
    let r = reasoned(
        "ex:A owl:disjointWith ex:B . ex:C rdfs:subClassOf ex:B . ex:x a ex:A , ex:C .",
        switches(),
    );
    let d = r
        .diagnostics()
        .iter()
        .find(|d| d.code == "cax-dw")
        .expect("cax-dw")
        .clone();
    let proof = r.explain_diagnostic(&d).unwrap().expect("a proof");
    well_formed(&r, &proof);
    assert_eq!(proof.goal(), Goal::Diagnostic(&d));
    assert_eq!(proof.depth(), 2);
    let Reason::Rule { rule, premises } = proof.reason() else {
        panic!()
    };
    assert_eq!((*rule, premises.len()), ("cax-dw", 3));
    assert!(proof.to_string().ends_with("[cax-sco]\n    <http://example.org/C> rdfs:subClassOf <http://example.org/B>  [base]\n    <http://example.org/x> rdf:type <http://example.org/C>  [base]\n"), "{proof}");
    // A diagnostic that is not reported has none.
    let mut other = d.clone();
    other.code = "cls-com";
    assert_eq!(r.explain_diagnostic(&other).unwrap(), None);
}

#[test]
fn dt_type2_and_dt_not_type_cite_a_triple_that_names_the_literal() {
    let r = reasoned(
        "ex:age rdfs:range xsd:string . ex:x ex:age \"12\"^^xsd:integer . ex:y ex:p \"abc\"^^xsd:integer .",
        Switches::complete().off(Switch::Equality).off(Switch::LiteralValues),
    );
    let lit = Term::from(Literal::new_typed_literal(
        "12",
        NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#integer"),
    ));
    let explainer = r.explainer().unwrap();
    for d in r.diagnostics().iter().filter(|d| d.code == "dt-not-type") {
        let proof = explainer.explain_diagnostic(d).expect("a proof");
        well_formed(&r, &proof);
        let Reason::Rule { rule, .. } = proof.reason() else {
            panic!()
        };
        assert_eq!(*rule, "dt-not-type");
        if d.terms[0] == lit {
            // Typed by prp-rng: the typing's proof.
            assert_eq!(rules(&proof), ["base", "base", "prp-rng"], "{proof}");
        } else {
            // Ill-typed: the base triple that names it.
            assert_eq!(rules(&proof), ["base"], "{proof}");
        }
    }
    assert_eq!(r.diagnostics().iter().filter(|d| d.code == "dt-not-type").count(), 2);
}

#[test]
fn an_explainer_needs_a_reasoned_closure() {
    let mut r = Reasoner::with_switches(switches());
    assert!(matches!(r.explainer(), Err(Error::NotReasoned)));
    r.reason();
    assert!(r.explainer().is_ok());
    r.insert([triple("ex:a", "ex:p", "ex:b").as_ref()]);
    assert!(matches!(
        r.explain(triple("ex:a", "ex:p", "ex:b").as_ref()),
        Err(Error::NotReasoned)
    ));
    r.reason();
    assert_eq!(rules(&explain(&r, &triple("ex:a", "ex:p", "ex:b"))), ["base"]);
    r.set_switches(Switches::complete());
    assert!(matches!(r.explainer(), Err(Error::NotReasoned)));
    assert_eq!(
        Error::NotReasoned.to_string(),
        "the reasoner changed since its last run: reason() first"
    );
}

/// A blank node of a loaded document, named as the reads write it (`b_1`).
#[test]
fn a_blank_node_of_a_document_is_found_by_its_written_label() {
    let r = reasoned("ex:p rdfs:domain ex:C . _:b ex:p ex:o .", switches());
    let typed: Triple = r
        .inferred()
        .find(|t| t.object == Term::from(iri("ex:C")))
        .expect("prp-dom");
    assert_eq!(typed.subject, NamedOrBlankNode::from(BlankNode::new_unchecked("b_1")));
    let proof = explain(&r, &typed);
    assert_eq!(rules(&proof), ["base", "base", "prp-dom"]);
    let inserted = Triple::new(BlankNode::new_unchecked("b"), iri("ex:p"), iri("ex:o"));
    assert_eq!(r.explain(inserted.as_ref()).unwrap(), None, "no inserted _:b");
}

#[test]
fn a_cache_is_reused_while_the_closure_stays() {
    let mut r = reasoned("ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog .", switches());
    let t = triple("ex:rex", "rdf:type", "ex:Animal");
    let explainer = r.explainer().unwrap();
    let first = explainer.explain(t.as_ref());
    let cache: ExplainCache = explainer.into_cache();
    assert!(format!("{cache:?}").contains("built: true"));
    let again = r.explainer_with(cache).unwrap();
    assert_eq!(again.explain(t.as_ref()), first);
    let cache = again.into_cache();
    assert!(format!("{cache:?}").contains("built: true"));
    // Another closure starts the cache over.
    r.insert([triple("ex:Animal", "rdfs:subClassOf", "ex:Thing").as_ref()]);
    r.reason();
    let other = r.explainer_with(cache).unwrap();
    assert!(format!("{:?}", other).contains("built: false"));
    let proof = other
        .explain(triple("ex:rex", "rdf:type", "ex:Thing").as_ref())
        .expect("a proof");
    well_formed(&r, &proof);
    assert_eq!(proof.depth(), 2);
}

#[test]
fn a_reasoner_from_a_snapshot_explains_as_the_saved_one() {
    let r = reasoned("ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog .", switches());
    let mut bytes = Vec::new();
    r.write_snapshot(&mut bytes).unwrap();
    let restored = Reasoner::from_snapshot_bytes(&bytes).unwrap();
    let t = triple("ex:rex", "rdf:type", "ex:Animal");
    assert_eq!(restored.explain(t.as_ref()).unwrap(), r.explain(t.as_ref()).unwrap());
    assert!(restored.explain(t.as_ref()).unwrap().is_some());
}

/// Every inferred triple and diagnostic of a small mixed input has a proof
/// of the right shape, in the complete configuration without equality and
/// literal values.
#[test]
fn every_inferred_triple_and_diagnostic_has_a_proof() {
    let r = reasoned(
        "ex:p a owl:TransitiveProperty ; rdfs:domain ex:A ; owl:inverseOf ex:q .
         ex:A owl:equivalentClass ex:B . ex:B owl:intersectionOf ( ex:C ex:D ) .
         ex:C owl:disjointWith ex:E . ex:r owl:propertyChainAxiom ( ex:p ex:q ) .
         ex:x ex:p ex:y . ex:y ex:p ex:z . ex:x a ex:E .
         ex:R owl:onProperty ex:p ; owl:someValuesFrom ex:A . ex:k a owl:Class ; owl:hasKey ( ex:p ) .
         ex:u a ex:k ; ex:p ex:z . ex:v a ex:k ; ex:p ex:z .",
        Switches::complete().off(Switch::Equality).off(Switch::LiteralValues),
    );
    let explainer = r.explainer().unwrap();
    let mut n = 0;
    for t in r.inferred() {
        let proof = explainer
            .explain(t.as_ref())
            .unwrap_or_else(|| panic!("no proof of {t}"));
        well_formed(&r, &proof);
        n += 1;
    }
    for d in r.diagnostics() {
        let proof = explainer
            .explain_diagnostic(d)
            .unwrap_or_else(|| panic!("no proof of {d:?}"));
        well_formed(&r, &proof);
    }
    assert!(n > 100 && !r.diagnostics().is_empty(), "{n}");
}
