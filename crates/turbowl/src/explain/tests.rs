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

/// The default configuration without equality: no class forms.
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
                // A generalized triple (a literal subject, dt-diff's unlisted
                // predicate) is not one the reads give.
                if !step.statement.subject.is_literal() && step.statement.predicate.is_named_node() {
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

fn literal(lexical: &str, datatype: &str) -> Term {
    Term::from(Literal::new_typed_literal(lexical, iri(datatype)))
}

/// `ex:a owl:sameAs ex:b`, `ex:b owl:sameAs ex:c`, `ex:c owl:sameAs ex:d`:
/// `ex:a owl:sameAs ex:d` is the chain of three links, by eq-trans, and a
/// triple about `ex:d` holds of `ex:a` by one eq-rep-s over it.
#[test]
fn a_same_as_chain_of_three_hops() {
    let r = reasoned(
        "ex:a owl:sameAs ex:b . ex:b owl:sameAs ex:c . ex:c owl:sameAs ex:d . ex:d ex:p ex:o .",
        Switches::default(),
    );
    let proof = explain(&r, &triple("ex:a", "owl:sameAs", "ex:d"));
    assert_eq!(
        rules(&proof),
        ["base", "base", "eq-trans", "base", "eq-trans"],
        "{proof}"
    );
    assert_eq!(proof.depth(), 2);
    assert_eq!(proof.leaves().count(), 3);
    // Backwards: each link turned by eq-sym.
    let proof = explain(&r, &triple("ex:d", "owl:sameAs", "ex:a"));
    assert_eq!(rules(&proof).iter().filter(|&&r| r == "eq-sym").count(), 3, "{proof}");
    let proof = explain(&r, &triple("ex:a", "ex:p", "ex:o"));
    assert_eq!(rules(&proof).last(), Some(&"eq-rep-s"), "{proof}");
    assert_eq!(proof.leaves().count(), 4, "{proof}");
    // A term is the same as itself through a link.
    let proof = explain(&r, &triple("ex:b", "owl:sameAs", "ex:b"));
    assert_eq!(rules(&proof).last(), Some(&"eq-trans"), "{proof}");
}

/// The store holds `ex:x ex:p ex:o` once, in representatives; a triple of
/// the user's terms that no rule derives as stated holds only through
/// them: eq-rep-s, eq-rep-p and eq-rep-o from the base triple.
#[test]
fn a_fact_that_holds_only_through_a_representative() {
    let r = reasoned(
        "ex:x ex:p ex:o . ex:x owl:sameAs ex:y . ex:p owl:sameAs ex:q . ex:o owl:sameAs ex:z .
         ex:C rdfs:subClassOf ex:D . ex:y a ex:C .",
        Switches::default(),
    );
    let canon = r.store().canon();
    assert!(!canon.is_empty());
    let proof = explain(&r, &triple("ex:y", "ex:q", "ex:z"));
    assert_eq!(
        rules(&proof).iter().filter(|r| r.starts_with("eq-rep")).count(),
        3,
        "{proof}"
    );
    assert_eq!(proof.leaves().count(), 4, "{proof}");
    // A rule over a triple about another member: cax-sco on ex:y, carried
    // to ex:x.
    let proof = explain(&r, &triple("ex:x", "rdf:type", "ex:D"));
    assert!(rules(&proof).contains(&"cax-sco"), "{proof}");
    // Every inferred triple and diagnostic has one.
    let explainer = r.explainer().unwrap();
    for t in r.inferred() {
        let proof = explainer
            .explain(t.as_ref())
            .unwrap_or_else(|| panic!("no proof of {t}"));
        well_formed(&r, &proof);
    }
}

/// dt-eq: two literals of one value are the same, each named by a closure
/// triple, which is the step's premise. (Its triple has a literal subject,
/// so it is asked for through what eq-rep-o carries.)
#[test]
fn dt_eq_cites_a_triple_naming_each_literal() {
    {
        let r = reasoned(
            "ex:x ex:p \"1\"^^xsd:integer . ex:y ex:p \"01\"^^xsd:integer .",
            Switches::complete(),
        );
        let one = literal("1", "xsd:integer");
        let other = literal("01", "xsd:integer");
        let proof = r
            .explain(oxrdf::TripleRef::new(
                oxrdf::NamedOrBlankNodeRef::from(iri("ex:x").as_ref()),
                iri("ex:p").as_ref(),
                other.as_ref(),
            ))
            .unwrap()
            .expect("a proof");
        well_formed(&r, &proof);
        // ex:x ex:p "01": eq-rep-o over dt-eq's "1" owl:sameAs "01".
        let dt_eq = proof
            .steps()
            .iter()
            .find(|s| s.reason != Reason::Base && matches!(s.reason, Reason::Rule { rule: "dt-eq", .. }))
            .unwrap_or_else(|| panic!("{proof}"));
        assert_eq!(dt_eq.statement.subject, one, "{proof}");
        let Reason::Rule { premises, .. } = &dt_eq.reason else {
            panic!()
        };
        assert_eq!(premises.len(), 2, "{proof}");
    }
}

/// dt-diff: two literals of different values made the same (by a
/// functional property) are reported by eq-diff1, whose proof reads a
/// dt-diff step between them, with the reference's unlisted predicate.
#[test]
fn dt_diff_goes_into_eq_diff1() {
    for switches in [Switches::complete(), Switches::complete().off(Switch::Equality)] {
        let r = reasoned(
            "ex:f a owl:FunctionalProperty . ex:x ex:f \"1\"^^xsd:integer , \"2\"^^xsd:integer .",
            switches,
        );
        let explainer = r.explainer().unwrap();
        let found: Vec<_> = r.diagnostics().iter().filter(|d| d.code == "eq-diff1").collect();
        assert!(!found.is_empty(), "{:?}", r.diagnostics());
        for d in found {
            let proof = explainer
                .explain_diagnostic(d)
                .unwrap_or_else(|| panic!("no proof of {d:?}"));
            well_formed(&r, &proof);
            let Reason::Rule { rule, premises } = proof.reason() else {
                panic!()
            };
            assert_eq!((*rule, premises.len()), ("eq-diff1", 2), "{proof}");
            let unlisted = Term::from(Literal::new_typed_literal(
                super::UNLISTED_LEXICAL,
                NamedNode::new_unchecked(super::UNLISTED_DATATYPE),
            ));
            let diff = proof
                .steps()
                .iter()
                .find(|s| matches!(s.reason, Reason::Rule { rule: "dt-diff", .. }))
                .unwrap_or_else(|| panic!("{proof}"));
            assert_eq!(diff.statement.predicate, unlisted, "{proof}");
            assert!(diff.statement.subject.is_literal() && diff.statement.object.is_literal());
            // The root reads the unlisted triple between its own terms.
            let read = &proof.steps()[premises[1]].statement;
            assert_eq!(read.predicate, unlisted, "{proof}");
            assert_eq!([&read.subject, &read.object], [&d.terms[0], &d.terms[1]], "{proof}");
        }
    }
}

/// dt-not-type's two cases: a literal ill-typed for its own datatype cites
/// a closure triple that names it, and a literal typed by a triple outside
/// that type's value space cites the typing's proof, through equality too.
#[test]
fn dt_not_type_has_two_cases() {
    let r = reasoned(
        "ex:y ex:p \"abc\"^^xsd:integer .
         ex:age rdfs:range xsd:boolean . ex:x ex:years \"12\"^^xsd:integer . ex:x owl:sameAs ex:x2 .
         ex:x2 ex:age \"12\"^^xsd:integer .",
        Switches::default(),
    );
    let explainer = r.explainer().unwrap();
    let mut seen = (false, false);
    for d in r.diagnostics().iter().filter(|d| d.code == "dt-not-type") {
        let proof = explainer
            .explain_diagnostic(d)
            .unwrap_or_else(|| panic!("no proof of {d:?}"));
        well_formed(&r, &proof);
        if d.terms[0] == literal("abc", "xsd:integer") {
            assert_eq!(rules(&proof), ["base"], "{proof}");
            seen.0 = true;
        } else {
            assert_eq!(d.terms[0], literal("12", "xsd:integer"));
            assert!(rules(&proof).contains(&"prp-rng"), "{proof}");
            seen.1 = true;
        }
    }
    assert_eq!(seen, (true, true), "{:?}", r.diagnostics());
}

/// A cache from another reasoner whose counts happen to match is never
/// trusted blindly: a proof whose steps the closure does not hold is not
/// returned, and the search runs again on a fresh cache.
#[test]
fn a_stale_cache_gives_no_wrong_proof() {
    let a = reasoned("ex:A rdfs:subClassOf ex:B . ex:x a ex:A .", switches());
    let b = reasoned("ex:C rdfs:subClassOf ex:D . ex:y a ex:C .", switches());
    let explainer = a.explainer().unwrap();
    let t = triple("ex:x", "rdf:type", "ex:B");
    assert!(explainer.explain(t.as_ref()).is_some());
    let cache = explainer.into_cache();
    // Same counts: the key alone may match, but the proofs must be b's.
    let other = b.explainer_with(cache).unwrap();
    let proof = other
        .explain(triple("ex:y", "rdf:type", "ex:D").as_ref())
        .expect("a proof");
    well_formed(&b, &proof);
    assert_eq!(other.explain(t.as_ref()), None);
}

/// A chaos input (reduced from seed 1592590341, iteration 336 of the full
/// test) whose run restarts: `owl:hasValue owl:sameAs owl:sameAs` makes
/// `owl:hasValue` a spelling of `owl:sameAs`, so `_:g42 owl:hasValue
/// "plain"` is a link of a class that ends up holding `rdf:type` and most
/// terms, literals of different values among them. Its eq-diff1
/// diagnostics on `"a"` need the program's eq-* rules (the last level of
/// the search): each took over a second in chaos, and the ten take about
/// half a second here with one explainer.
#[test]
fn a_restarted_closure_is_explained_in_time() {
    const INPUT: &str = r##"<http://example.org/C0> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C2> .
<http://example.org/C0> <http://www.w3.org/2002/07/owl#hasKey> _:g38 .
<http://example.org/C1> <http://www.w3.org/2000/01/rdf-schema#subClassOf> _:g41 .
<http://example.org/C1> <http://www.w3.org/2002/07/owl#equivalentClass> _:g47 .
<http://example.org/C2> <http://www.w3.org/2000/01/rdf-schema#subClassOf> _:g6 .
<http://example.org/C3> <http://www.w3.org/2000/01/rdf-schema#subClassOf> _:g46 .
<http://example.org/d1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#FunctionalProperty> .
<http://example.org/d1> <http://www.w3.org/2002/07/owl#equivalentProperty> <http://example.org/d0> .
<http://example.org/i0> <http://example.org/p5> <http://example.org/i8> .
<http://example.org/i0> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C0> .
<http://example.org/i0> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> _:g35 .
<http://example.org/i0> <http://www.w3.org/2002/07/owl#sameAs> <http://example.org/i6> .
<http://example.org/i1> <http://example.org/d0> "a@"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral> .
<http://example.org/i7> <http://example.org/p4> <http://example.org/i2> .
<http://example.org/i7> <http://example.org/p5> <http://example.org/i1> .
<http://example.org/i7> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C0> .
<http://example.org/i7> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Nothing> .
<http://example.org/i9> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C0> .
<http://example.org/p4> <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> <http://example.org/p5> .
<http://www.w3.org/2002/07/owl#hasValue> <http://www.w3.org/2002/07/owl#sameAs> <http://www.w3.org/2002/07/owl#sameAs> .
_:g1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Restriction> .
_:g1 <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C1> .
_:g1 <http://www.w3.org/2002/07/owl#onProperty> <http://example.org/p5> .
_:g1 <http://www.w3.org/2002/07/owl#someValuesFrom> <http://www.w3.org/2002/07/owl#Thing> .
_:g35 <http://www.w3.org/2002/07/owl#maxCardinality> "1"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger> .
_:g35 <http://www.w3.org/2002/07/owl#onProperty> <http://example.org/p5> .
_:g36 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/p2> .
_:g38 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/p5> .
_:g38 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> <http://www.w3.org/1999/02/22-rdf-syntax-ns#nil> .
_:g39 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/p1> .
_:g41 <http://www.w3.org/2002/07/owl#onProperty> <http://example.org/d0> .
_:g42 <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Restriction> .
_:g42 <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C4> .
_:g42 <http://www.w3.org/2002/07/owl#hasValue> "plain"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral> .
_:g42 <http://www.w3.org/2002/07/owl#onProperty> <http://example.org/d2> .
_:g47 <http://www.w3.org/2002/07/owl#onProperty> <http://example.org/p4> .
_:g6 <http://www.w3.org/2002/07/owl#onProperty> <http://example.org/p4> .
_:g6 <http://www.w3.org/2002/07/owl#someValuesFrom> <http://example.org/C4> .
_:g8 <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C5> .
_:g8 <http://www.w3.org/2002/07/owl#onProperty> <http://example.org/p5> .
_:g8 <http://www.w3.org/2002/07/owl#someValuesFrom> <http://example.org/C0> .
_:g9 <http://www.w3.org/1999/02/22-rdf-syntax-ns#first> <http://example.org/p2> .
_:g9 <http://www.w3.org/1999/02/22-rdf-syntax-ns#rest> _:g10 .
_:x0 <http://example.org/d0> "+5"^^<http://www.w3.org/2001/XMLSchema#integer> .
_:x0 <http://example.org/d0> "- 1"^^<http://www.w3.org/2001/XMLSchema#double> .
_:x0 <http://example.org/d0> "99999999999999999999999999999999999999999"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger> .
_:x0 <http://example.org/d0> "QQ=="^^<http://www.w3.org/2001/XMLSchema#base64Binary> .
_:x0 <http://example.org/d0> "plain"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral> .
_:x0 <http://example.org/p4> <http://example.org/i2> .
_:x0 <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C0> .
_:x1 <http://example.org/p4> <http://example.org/i6> .
_:x1 <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> _:g44 .
_:x2 <http://example.org/d1> "hello" .
_:x2 <http://example.org/d2> "a" .
"##;
    let mut r = Reasoner::with_switches(Switches::complete());
    r.load_str(INPUT, Format::NTriples, None).unwrap();
    r.reason();
    let explainer = r.explainer().unwrap();
    let a = Term::from(Literal::new_simple_literal("a"));
    let start = std::time::Instant::now();
    let found: Vec<_> = r
        .diagnostics()
        .iter()
        .filter(|d| d.code == "eq-diff1" && d.terms[0] == a)
        .collect();
    assert_eq!(found.len(), 10);
    for d in found {
        let proof = explainer
            .explain_diagnostic(d)
            .unwrap_or_else(|| panic!("no proof of {d:?}"));
        well_formed(&r, &proof);
    }
    // About 0.5 s (release, an M-series Mac at load 3, 2026-11-07).
    assert!(
        start.elapsed() < std::time::Duration::from_secs(5),
        "{:?}",
        start.elapsed()
    );
}

/// A class of many members that many triples reach, as Claros's time spans
/// in the complete configuration: one production with 100 time spans makes
/// them one class by cls-maxc2 (at most one `ex:span`), and 3,000 other
/// productions each name one of them. Read member by member, the class's
/// links read every production once per member and every member once per
/// production (spans × productions × spans matches: on claros-l one
/// explanation ran for over 20 minutes); read once for the class, each
/// triple is read once and a member's partners form a star. Each proof is
/// checked against the base, and the whole is timed.
#[test]
fn a_class_that_many_triples_reach_is_linked_in_time() {
    use std::collections::HashSet;
    use std::fmt::Write;
    const SPANS: usize = 100;
    const PRODUCTIONS: usize = 3_000;
    let mut text = String::from(
        "ex:Production rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:span ;
           owl:maxCardinality \"1\"^^xsd:nonNegativeInteger ] .
         ex:hub a ex:Production .\n",
    );
    for k in 0..SPANS {
        let _ = writeln!(text, "ex:hub ex:span ex:s{k} .");
    }
    for i in 0..PRODUCTIONS {
        let _ = writeln!(text, "ex:u{i} a ex:Production ; ex:span ex:s{} .", i % SPANS);
    }
    let r = reasoned(&text, Switches::complete());
    let inferred: HashSet<Triple> = r.inferred().collect();
    let base: HashSet<Statement> = r
        .triples()
        .filter(|t| !inferred.contains(t))
        .map(|t| statement(&t))
        .collect();
    let goals = [
        triple("ex:u7", "ex:span", "ex:s42"),
        triple("ex:hub", "ex:span", "ex:s3"),
        triple("ex:s3", "owl:sameAs", "ex:s99"),
        triple("ex:s99", "owl:sameAs", "ex:s3"),
        triple("ex:s5", "owl:sameAs", "ex:s5"),
        triple("ex:u1234", "ex:span", "ex:s0"),
    ];
    let start = std::time::Instant::now();
    let explainer = r.explainer().unwrap();
    for t in &goals {
        let proof = explainer
            .explain(t.as_ref())
            .unwrap_or_else(|| panic!("no proof of {t}"));
        assert_eq!(proof.goal(), Goal::Statement(&statement(t)));
        for (i, step) in proof.steps().iter().enumerate() {
            match &step.reason {
                Reason::Base => assert!(
                    base.contains(&step.statement),
                    "{:?} is not base\n{proof}",
                    step.statement
                ),
                Reason::Rule { premises, .. } => {
                    assert!(premises.iter().all(|&p| p < i), "step {i} reads a later step:\n{proof}");
                }
            }
        }
        assert!(proof.depth() <= 8, "{proof}");
    }
    // About 8 ms (release, an M-series Mac at load 2, 2026-11-07); read
    // member by member, 30 s.
    assert!(
        start.elapsed() < std::time::Duration::from_secs(2),
        "{:?}",
        start.elapsed()
    );
}
