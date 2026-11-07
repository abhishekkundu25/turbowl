//! The proof checker: a `turbowl::Proof` judged by the reference rules
//! (`docs/specs/explanations.md`, Implementation Decisions). It shares no
//! code with the core's explainer or its rules.
//!
//! For each rule step, the families of the reference that hold the step's
//! rule (and that the switches leave on) run one round over the step's
//! premises alone, recording every (triple, rule) pair they derive, not
//! only the first rule of each triple: two rules can share a premise set
//! (prp-inv1 and prp-inv2 on `p owl:inverseOf p`), and either one passes.
//! The step's statement must be among them under its rule. A diagnostic
//! root is judged the same way by the family's diagnostic rules. Each base
//! step must be a base triple, each premise an earlier step (so the steps
//! are acyclic), and each statement must appear once.

use crate::evaluate::{Fact, Round, View};
use crate::rules::families_of;
use oxrdf::{NamedNode, NamedOrBlankNode, Triple};
use std::collections::HashSet;
use turbowl::{Diagnostic, Goal, Proof, Reason, Statement, Step, Switches};

/// Checks `proof` against the reference rules of `switches`, with `base`
/// the base triples: `Err` names the first step that fails and why.
pub fn check_proof(proof: &Proof, base: &HashSet<Triple>, switches: Switches) -> Result<(), String> {
    let root = match proof.goal() {
        Goal::Diagnostic(d) => Some((d, proof.reason())),
        Goal::Statement(_) => None,
    };
    check_steps(proof.steps(), root, base, switches).map_err(|e| format!("{e}\nin the proof\n{proof}"))
}

/// [`check_proof`] over the parts of a proof.
fn check_steps(
    steps: &[Step],
    root: Option<(&Diagnostic, &Reason)>,
    base: &HashSet<Triple>,
    switches: Switches,
) -> Result<(), String> {
    if steps.is_empty() && root.is_none() {
        return Err("a proof with no step".into());
    }
    let mut seen = HashSet::new();
    for (i, step) in steps.iter().enumerate() {
        let s = &step.statement;
        let shown = || format!("step {i} ({} {} {})", s.subject, s.predicate, s.object);
        if !seen.insert(s) {
            return Err(format!("{} appears twice", shown()));
        }
        match &step.reason {
            Reason::Base => {
                if !rdf(s).is_some_and(|t| base.contains(&t)) {
                    return Err(format!("{} is a leaf but not a base triple", shown()));
                }
            }
            Reason::Rule { rule, premises } => {
                let view = premises_of(steps, i, premises).map_err(|e| format!("{}: {e}", shown()))?;
                let derived = derive(rule, &view, switches);
                if !derived.contains(&(fact(s), *rule)) {
                    return Err(format!(
                        "{}: {rule} does not derive it from its {} premises",
                        shown(),
                        premises.len()
                    ));
                }
            }
        }
    }
    if let Some((d, reason)) = root {
        let Reason::Rule { rule, premises } = reason else {
            return Err("a diagnostic is not a base triple".into());
        };
        if *rule != d.code {
            return Err(format!("the diagnostic {} is proved by {rule}", d.code));
        }
        let view = premises_of(steps, steps.len(), premises).map_err(|e| format!("the diagnostic: {e}"))?;
        let mut found = Vec::new();
        let view = View::new(&view);
        for family in families_of(rule, switches) {
            (family.diagnose)(&view, &mut found);
        }
        if !found.iter().any(|(code, terms)| code == rule && *terms == d.terms) {
            return Err(format!(
                "{rule} does not find the diagnostic {:?} in its {} premises",
                d.terms,
                premises.len()
            ));
        }
    }
    Ok(())
}

/// The premises of step `i` as facts, each an earlier step.
fn premises_of(steps: &[Step], i: usize, premises: &[usize]) -> Result<HashSet<Fact>, String> {
    premises
        .iter()
        .map(|&p| {
            if p < i {
                Ok(fact(&steps[p].statement))
            } else {
                Err(format!("premise {p} is not an earlier step"))
            }
        })
        .collect()
}

/// Every (triple, rule) one round of `rule`'s families derives over `premises`.
fn derive(rule: &str, premises: &HashSet<Fact>, switches: Switches) -> HashSet<(Fact, &'static str)> {
    let view = View::new(premises);
    let mut derived = HashSet::new();
    for family in families_of(rule, switches) {
        let mut round = Round::logged(premises);
        (family.derive)(&view, &mut round);
        derived.extend(round.into_log());
    }
    derived
}

fn fact(s: &Statement) -> Fact {
    (s.subject.clone(), s.predicate.clone(), s.object.clone())
}

/// The RDF triple a statement is, if it is one.
fn rdf(s: &Statement) -> Option<Triple> {
    Some(Triple::new(
        NamedOrBlankNode::try_from(s.subject.clone()).ok()?,
        NamedNode::try_from(s.predicate.clone()).ok()?,
        s.object.clone(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocab::*;
    use oxrdf::Term;
    use turbowl::{Reasoner, Severity, Switch};

    const EX: &str = "http://example.org/";

    fn ex(local: &str) -> Term {
        iri(&format!("{EX}{local}"))
    }

    fn statement(s: Term, p: Term, o: Term) -> Statement {
        Statement {
            subject: s,
            predicate: p,
            object: o,
        }
    }

    fn base(s: Statement) -> Step {
        Step {
            statement: s,
            reason: Reason::Base,
        }
    }

    fn rule(s: Statement, rule: &'static str, premises: Vec<usize>) -> Step {
        Step {
            statement: s,
            reason: Reason::Rule { rule, premises },
        }
    }

    fn triples(steps: &[Step]) -> HashSet<Triple> {
        steps
            .iter()
            .filter(|s| s.reason == Reason::Base)
            .filter_map(|s| rdf(&s.statement))
            .collect()
    }

    /// `ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog`, and its cax-sco step.
    fn dog() -> Vec<Step> {
        vec![
            base(statement(ex("Dog"), iri(RDFS_SUB_CLASS_OF), ex("Animal"))),
            base(statement(ex("rex"), iri(RDF_TYPE), ex("Dog"))),
            rule(statement(ex("rex"), iri(RDF_TYPE), ex("Animal")), "cax-sco", vec![0, 1]),
        ]
    }

    fn check(steps: &[Step], base: &HashSet<Triple>) -> Result<(), String> {
        check_steps(steps, None, base, Switches::default())
    }

    #[test]
    fn a_valid_proof_passes() {
        let steps = dog();
        assert_eq!(check(&steps, &triples(&steps)), Ok(()));
        // An axiom: no premises.
        let thing = [rule(
            statement(iri(OWL_THING), iri(RDF_TYPE), iri(OWL_CLASS)),
            "cls-thing",
            vec![],
        )];
        assert_eq!(check(&thing, &HashSet::new()), Ok(()));
    }

    #[test]
    fn a_proof_from_the_core_passes() {
        let mut r = Reasoner::with_switches(Switches::default().off(Switch::Equality));
        let steps = dog();
        let base = triples(&steps);
        r.insert(base.iter().map(Triple::as_ref));
        r.reason();
        let goal = rdf(&steps[2].statement).unwrap();
        let proof = r.explain(goal.as_ref()).unwrap().expect("a proof");
        assert_eq!(check_proof(&proof, &base, r.switches()), Ok(()));
    }

    #[test]
    fn a_wrong_rule_name_fails() {
        let mut steps = dog();
        steps[2].reason = Reason::Rule {
            rule: "cax-eqc1",
            premises: vec![0, 1],
        };
        let e = check(&steps, &triples(&steps)).unwrap_err();
        assert!(e.contains("cax-eqc1 does not derive it"), "{e}");
        // A rule of a switch that is off.
        steps[2].reason = Reason::Rule {
            rule: "scm-sco",
            premises: vec![0, 1],
        };
        assert!(check(&steps, &triples(&steps)).is_err());
    }

    #[test]
    fn a_missing_premise_fails() {
        let mut steps = dog();
        steps[2].reason = Reason::Rule {
            rule: "cax-sco",
            premises: vec![1],
        };
        let e = check(&steps, &triples(&steps)).unwrap_err();
        assert!(e.contains("from its 1 premises"), "{e}");
    }

    #[test]
    fn a_leaf_that_is_not_base_fails() {
        let steps = dog();
        let mut base = triples(&steps);
        base.remove(&rdf(&steps[1].statement).unwrap());
        let e = check(&steps, &base).unwrap_err();
        assert!(e.contains("step 1") && e.contains("not a base triple"), "{e}");
    }

    #[test]
    fn a_cycle_fails() {
        // ex:a ex:p ex:b and ex:b ex:p ex:a, each by prp-symp from the other.
        let p = ex("p");
        let ty = Step {
            statement: statement(p.clone(), iri(RDF_TYPE), iri(OWL_SYMMETRIC_PROPERTY)),
            reason: Reason::Base,
        };
        let steps = vec![
            ty,
            rule(statement(ex("a"), p.clone(), ex("b")), "prp-symp", vec![0, 2]),
            rule(statement(ex("b"), p.clone(), ex("a")), "prp-symp", vec![0, 1]),
        ];
        let e = check(&steps, &triples(&steps)).unwrap_err();
        assert!(e.contains("premise 2 is not an earlier step"), "{e}");
        // A statement twice.
        let mut twice = dog();
        twice.push(twice[0].clone());
        assert!(check(&twice, &triples(&twice)).unwrap_err().contains("twice"));
    }

    /// `p owl:inverseOf p`: prp-inv1 and prp-inv2 derive `b p a` from the
    /// same premises, and each passes.
    #[test]
    fn two_rules_on_one_premise_set_both_pass() {
        let p = ex("p");
        let premises = [
            base(statement(p.clone(), iri(OWL_INVERSE_OF), p.clone())),
            base(statement(ex("a"), p.clone(), ex("b"))),
        ];
        for name in ["prp-inv1", "prp-inv2"] {
            let mut steps = premises.to_vec();
            steps.push(rule(statement(ex("b"), p.clone(), ex("a")), name, vec![0, 1]));
            assert_eq!(check(&steps, &triples(&steps)), Ok(()), "{name}");
        }
    }

    #[test]
    fn a_diagnostic_root_is_judged_by_its_rule() {
        let steps = vec![
            base(statement(ex("A"), iri(OWL_DISJOINT_WITH), ex("B"))),
            base(statement(ex("x"), iri(RDF_TYPE), ex("A"))),
            base(statement(ex("x"), iri(RDF_TYPE), ex("B"))),
        ];
        let d = Diagnostic {
            code: "cax-dw",
            severity: Severity::Error,
            message: String::new(),
            terms: vec![ex("x"), ex("A"), ex("B")],
        };
        let base = triples(&steps);
        let ok = Reason::Rule {
            rule: "cax-dw",
            premises: vec![0, 1, 2],
        };
        assert_eq!(check_steps(&steps, Some((&d, &ok)), &base, Switches::default()), Ok(()));
        let short = Reason::Rule {
            rule: "cax-dw",
            premises: vec![0, 1],
        };
        assert!(check_steps(&steps, Some((&d, &short)), &base, Switches::default()).is_err());
        let off = Switches::default().off(Switch::Checks);
        assert!(check_steps(&steps, Some((&d, &ok)), &base, off).is_err());
    }
}
