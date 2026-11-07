//! Explanations: why a triple is in the closure, or why a diagnostic is
//! reported, as a [`Proof`] down to base triples
//! (`docs/specs/explanations.md`).
//!
//! A proof is computed on demand, after reasoning, from the closure of the
//! last run: nothing is recorded while reasoning runs. The [`Explainer`]
//! runs the declarative rule program (`oracles/rules/`, the rules Nemo,
//! Soufflé and VLog run, which every oracle holds equal to TurbOWL's)
//! backwards over the closure (`explain/program.rs` reads it,
//! `explain/search.rs` searches it): it finds the rule instances relevant
//! to the goal, ranks the facts by derivation depth from the base, and
//! keeps for each fact an instance whose premises all rank lower. So every
//! proof is acyclic, grounded in base triples, and as shallow as the
//! closure allows. Only the rules the reasoner's switches leave on are
//! used. A helper relation of the program (the cells of a list, a chain's
//! partial paths) is folded into the step that reads it, so each step's
//! premises are triples.
//!
//! The explainer's indexes (the closure sorted three ways, the program with
//! its constants as term ids) are built on first use, and the rule
//! instances its searches find are kept, so each fact is explained once.
//! Both live in an [`ExplainCache`], which the caller may keep between
//! explainers while the reasoner does not change.
//!
//! # Limits
//!
//! Ticket 01 of `docs/tickets/explain/`: a closure in which `owl:sameAs`
//! made two terms one (the canon has a class) gets no proofs yet, and
//! neither do the triples of `literal-values` (dt-eq): ticket 02 adds them.

mod program;
mod search;
#[cfg(all(test, feature = "io"))]
mod tests;

use crate::reasoner::Reasoner;
use crate::rules::datatype;
use crate::switch::Switch;
use crate::term::{Dictionary, DocId, TermId, vocab};
use crate::{Diagnostic, Switches};
use oxrdf::{Term, TermRef, TripleRef};
use program::{Compiled, MAX_ARITY, T};
use search::{Built, Facts, Graph, Search, World};
use std::fmt;
use std::sync::{Mutex, OnceLock};

/// A triple of a proof: three terms, a generalized triple, since a proof
/// may pass through one that RDF cannot write (a literal subject, as
/// prp-rng gives a literal object a type).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Statement {
    pub subject: Term,
    pub predicate: Term,
    pub object: Term,
}

/// Why a step holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// A base triple: loaded or inserted.
    Base,
    /// A W3C rule (`cax-sco`, `prp-trp`, `dt-type2`, ...) over premises,
    /// each the index of an earlier step of the proof. A rule with an
    /// empty body (cls-thing, cls-nothing1, prp-ap, dt-type1) has none.
    Rule { rule: &'static str, premises: Vec<usize> },
}

/// One step of a proof: a statement and why it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub statement: Statement,
    pub reason: Reason,
}

/// What a proof proves: a statement (its last step), or a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal<'a> {
    Statement(&'a Statement),
    Diagnostic(&'a Diagnostic),
}

/// A proof: a DAG of steps in topological order, each premise before the
/// steps that read it, every statement once.
///
/// The goal is the last step, or for a diagnostic, a root apart from the
/// steps (only the root can be a diagnostic). Its leaves are base triples,
/// and it is acyclic. `Display` writes it as an indented tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof {
    steps: Vec<Step>,
    /// The diagnostic proved, and its rule (always [`Reason::Rule`]).
    diagnostic: Option<(Diagnostic, Reason)>,
}

impl Proof {
    /// The steps, each premise before the steps that read it; for a
    /// statement, the goal is the last.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// What the proof proves.
    pub fn goal(&self) -> Goal<'_> {
        match &self.diagnostic {
            Some((d, _)) => Goal::Diagnostic(d),
            None => Goal::Statement(&self.steps.last().expect("a proof has a step").statement),
        }
    }

    /// Why the goal holds.
    pub fn reason(&self) -> &Reason {
        match &self.diagnostic {
            Some((_, reason)) => reason,
            None => &self.steps.last().expect("a proof has a step").reason,
        }
    }

    /// The base triples the proof rests on, each once, in step order.
    pub fn leaves(&self) -> impl Iterator<Item = &Statement> + '_ {
        self.steps
            .iter()
            .filter(|s| s.reason == Reason::Base)
            .map(|s| &s.statement)
    }

    /// The most rule steps on a path from the goal to a leaf: 0 for a base
    /// triple, 1 for a rule over base triples (or with no premises).
    pub fn depth(&self) -> usize {
        let mut depth = Vec::with_capacity(self.steps.len());
        let of = |reason: &Reason, depth: &[usize]| match reason {
            Reason::Base => 0,
            Reason::Rule { premises, .. } => 1 + premises.iter().map(|&p| depth[p]).max().unwrap_or(0),
        };
        for step in &self.steps {
            let d = of(&step.reason, &depth);
            depth.push(d);
        }
        match &self.diagnostic {
            Some((_, reason)) => of(reason, &depth),
            None => depth.last().copied().unwrap_or(0),
        }
    }
}

/// The tree: the goal first, each premise indented under the step that
/// reads it, a step shown before written again as `(shown above)`.
impl fmt::Display for Proof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut shown = vec![false; self.steps.len()];
        let label = |reason: &Reason| match reason {
            Reason::Base => "base",
            Reason::Rule { rule, .. } => rule,
        };
        match &self.diagnostic {
            Some((d, reason)) => {
                writeln!(f, "{}  [{}]", d.message, label(reason))?;
                if let Reason::Rule { premises, .. } = reason {
                    for &p in premises {
                        self.tree(f, p, 1, &mut shown)?;
                    }
                }
                Ok(())
            }
            None => self.tree(f, self.steps.len() - 1, 0, &mut shown),
        }
    }
}

impl Proof {
    fn tree(&self, f: &mut fmt::Formatter<'_>, i: usize, indent: usize, shown: &mut [bool]) -> fmt::Result {
        let step = &self.steps[i];
        let s = &step.statement;
        let names = [&s.subject, &s.predicate, &s.object].map(crate::diagnostics::name);
        write!(
            f,
            "{:indent$}{} {} {}",
            "",
            names[0],
            names[1],
            names[2],
            indent = 2 * indent
        )?;
        match &step.reason {
            Reason::Base => writeln!(f, "  [base]"),
            Reason::Rule { rule, .. } if shown[i] => writeln!(f, "  [{rule}, shown above]"),
            Reason::Rule { rule, premises } => {
                writeln!(f, "  [{rule}]")?;
                shown[i] = true;
                for &p in premises {
                    self.tree(f, p, indent + 1, shown)?;
                }
                Ok(())
            }
        }
    }
}

/// The explainer's lazy indexes, owned, so that a caller can keep them
/// between explainers: [`Reasoner::explainer_with`] takes one,
/// [`Explainer::into_cache`] gives it back. `Send + Sync`.
///
/// A cache belongs to the closure it was built on. Keep it only while the
/// reasoner does not change; an explainer given a cache made for another
/// state (another configuration, or another count of terms, base triples,
/// stored triples or diagnostics) starts it over.
#[derive(Default)]
pub struct ExplainCache {
    key: Option<Key>,
    parts: OnceLock<Parts>,
    /// What the searches found: each fact is explained once.
    graph: Mutex<Graph>,
}

impl fmt::Debug for ExplainCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (nodes, instances) = self.graph.lock().map_or((0, 0), |g| g.len());
        f.debug_struct("ExplainCache")
            .field("built", &self.parts.get().is_some())
            .field("bytes", &self.parts.get().map_or(0, |p| p.facts.bytes()))
            .field("nodes", &nodes)
            .field("instances", &instances)
            .finish()
    }
}

// The interface promises `Send + Sync`: checked here, at compile time.
const _: () = {
    const fn send_sync<T: Send + Sync>() {}
    send_sync::<ExplainCache>()
};

/// What a cache was built on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Key {
    switches: Switches,
    terms: usize,
    base: usize,
    stored: usize,
    diagnostics: usize,
}

impl Key {
    fn of(r: &Reasoner) -> Self {
        Self {
            switches: r.switches(),
            terms: r.terms().len(),
            base: r.base_len(),
            stored: r.store().len(),
            diagnostics: r.diagnostics().len(),
        }
    }
}

/// The indexes, built on first use.
#[derive(Debug)]
struct Parts {
    facts: Facts,
    program: Compiled,
    rule_literals: Vec<u32>,
}

impl Parts {
    fn new(r: &Reasoner) -> Self {
        let terms = r.terms();
        let mut triples: Vec<[u32; 3]> = r.store().triples().map(|(s, p, o)| [s, p, o]).collect();
        // eq-ref (switch `trivial`) for every term of the closure, literals
        // included, as the program has it: the reads add it, and the store
        // holds it only where a rule could read it (`rules/trivial.rs`).
        if r.switches().is_on(Switch::Trivial) && !triples.is_empty() {
            let same = vocab::OWL_SAME_AS.0;
            let mut all: Vec<u32> = triples.iter().flatten().copied().collect();
            all.push(same);
            all.sort_unstable();
            all.dedup();
            triples.extend(all.into_iter().map(|t| [t, same, t]));
        }
        Self {
            facts: Facts::new(triples),
            program: program::program().compile(r.switches(), terms),
            rule_literals: datatype::rule_literals(terms).collect(),
        }
    }
}

/// Explains triples and diagnostics of a reasoner's last run
/// ([`Reasoner::explainer`]). It borrows the reasoner, so the closure
/// cannot change while it lives; its indexes are built on first use and
/// kept ([`ExplainCache`]).
#[derive(Debug)]
pub struct Explainer<'r> {
    reasoner: &'r Reasoner,
    cache: ExplainCache,
}

impl<'r> Explainer<'r> {
    /// An explainer over `reasoner`'s closure, which must be its last
    /// run's (the caller checks).
    pub(crate) fn new(reasoner: &'r Reasoner, cache: ExplainCache) -> Self {
        let key = Key::of(reasoner);
        let cache = if cache.key == Some(key) {
            cache
        } else {
            ExplainCache {
                key: Some(key),
                ..ExplainCache::default()
            }
        };
        Self { reasoner, cache }
    }

    /// Gives the cache back, to be passed to [`Reasoner::explainer_with`]
    /// while the reasoner does not change.
    pub fn into_cache(self) -> ExplainCache {
        self.cache
    }

    fn parts(&self) -> &Parts {
        self.cache.parts.get_or_init(|| Parts::new(self.reasoner))
    }

    /// True when the closure is one the search reads as it is (ticket 01:
    /// no two terms made one by `owl:sameAs`).
    fn readable(&self) -> bool {
        self.reasoner.store().canon().is_empty()
    }

    /// The proof of a triple of the closure: base, or derived by the rules
    /// the switches leave on. `None` when the triple is not in the closure.
    /// Blank nodes are named as the reads write them ([`Reasoner::triples`]).
    pub fn explain(&self, triple: TripleRef<'_>) -> Option<Proof> {
        if !self.readable() {
            return None;
        }
        let terms = self.reasoner.terms();
        let t = [
            id_of(terms, triple.subject.into())?,
            terms.get_iri(triple.predicate.as_str())?.0,
            id_of(terms, triple.object)?,
        ];
        let parts = self.parts();
        if !parts.facts.contains(t) {
            return None;
        }
        let built = self.prove(parts, T, [t[0], t[1], t[2], 0])?;
        Some(self.proof(built, None))
    }

    /// The proof of a diagnostic of the last run: the rule that reports it
    /// and the proofs of its premises. `None` when it is not reported.
    pub fn explain_diagnostic(&self, diagnostic: &Diagnostic) -> Option<Proof> {
        if !self.readable() || !self.reasoner.diagnostics().contains(diagnostic) {
            return None;
        }
        let terms = self.reasoner.terms();
        let parts = self.parts();
        let &(_, rel) = parts
            .program
            .findings
            .iter()
            .find(|(code, _)| *code == diagnostic.code)?;
        if diagnostic.terms.len() != parts.program.relations[rel as usize].arity {
            return None;
        }
        let mut tuple = [0; MAX_ARITY];
        for (slot, term) in tuple.iter_mut().zip(&diagnostic.terms) {
            *slot = id_of(terms, term.as_ref())?;
        }
        let built = self.prove(parts, rel, tuple)?;
        Some(self.proof(built, Some(diagnostic)))
    }

    /// The search for one goal, over the graph earlier goals left. A search
    /// that panicked leaves a graph that may lack instances: it is dropped.
    fn prove(&self, parts: &Parts, rel: program::RelId, tuple: [u32; MAX_ARITY]) -> Option<Built> {
        let mut graph = self.cache.graph.lock().unwrap_or_else(|poisoned| {
            let mut g = poisoned.into_inner();
            *g = Graph::default();
            g
        });
        let world = World {
            program: &parts.program,
            facts: &parts.facts,
            reasoner: self.reasoner,
            rule_literals: &parts.rule_literals,
        };
        Search::new(world, &mut graph).prove(rel, tuple)
    }

    /// The public form of a proof in term ids.
    fn proof(&self, built: Built, diagnostic: Option<&Diagnostic>) -> Proof {
        let terms = self.reasoner.terms();
        let term = |id: u32| terms.to_oxrdf(TermId(id));
        let steps = built
            .steps
            .into_iter()
            .map(|s| Step {
                statement: Statement {
                    subject: term(s.triple[0]),
                    predicate: term(s.triple[1]),
                    object: term(s.triple[2]),
                },
                reason: match s.rule {
                    None => Reason::Base,
                    Some(rule) => Reason::Rule {
                        rule,
                        premises: s.premises,
                    },
                },
            })
            .collect();
        let diagnostic = diagnostic
            .zip(built.root)
            .map(|(d, (rule, premises))| (d.clone(), Reason::Rule { rule, premises }));
        Proof { steps, diagnostic }
    }
}

/// The id of a term as the reads write it: a blank node by its written
/// label (`Dictionary::get_written_blank`).
fn id_of(terms: &Dictionary, term: TermRef<'_>) -> Option<u32> {
    match term {
        TermRef::BlankNode(b) => terms.get_written_blank(b.as_str()),
        other => terms.get_term(other, DocId::NONE),
    }
    .map(|t| t.0)
}
