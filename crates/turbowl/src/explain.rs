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
//! The explainer reads the reasoner's own store, in the terms the user
//! wrote: no copy of the closure is made. What the store cannot answer (a
//! predicate's triples by object where it keeps none, eq-ref's triples)
//! is built on first use, as is the program with its constants as term ids,
//! and the rule instances its searches find are kept, so each fact is
//! explained once. All of it lives in an [`ExplainCache`], which the
//! caller may keep between explainers while the reasoner does not change.
//!
//! # Equality and literal values
//!
//! A triple about terms `owl:sameAs` made equal is proved in the user's
//! terms with the eq-* rules: eq-rep-s, eq-rep-p and eq-rep-o from a triple
//! as stated, and each `owl:sameAs` needed as the shortest chain of eq-sym
//! and eq-trans over the class's links (`explain/search.rs`). dt-eq cites a
//! closure triple that names each literal; dt-diff, which the closure never
//! lists, is a generalized step whose predicate is the reference
//! evaluator's unlisted `owl:differentFrom` ([`UNLISTED_LEXICAL`] typed
//! [`UNLISTED_DATATYPE`]), carried by eq-rep-s and eq-rep-o to the terms
//! equal to its literals, where eq-diff1 reads it.
//!
//! # A cache that outlives its closure
//!
//! A cache is matched to its reasoner by the switches and a fingerprint of
//! its state (the counts of terms, base triples, stored triples and
//! diagnostics, each predicate's triples, the rounds run and the merges
//! made), and every proof's steps are checked against the reasoner before
//! it is returned: each base step is a base triple, each other a triple of
//! the closure. A proof that fails starts the cache over, and the search
//! runs again.

mod closure;
mod program;
mod search;
#[cfg(all(test, feature = "io"))]
mod tests;

use crate::reasoner::Reasoner;
use crate::rules::datatype;
use crate::term::{Dictionary, DocId, TermId};
use crate::{Cancel, Diagnostic, Error, Switches};
use closure::{Closure, Indexes};
use oxrdf::{Literal, NamedNode, Term, TermRef, TripleRef};
use program::{Compiled, MAX_ARITY, T, UNLISTED};
use search::{Built, Graph, Search, World};
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
/// stored triples or diagnostics, or other tables, rounds or merges)
/// starts it over, and a proof whose steps the reasoner does not hold is
/// never returned from it: the search runs again on a fresh one.
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
            .field("bytes", &self.bytes())
            .field("nodes", &nodes)
            .field("instances", &instances)
            .finish()
    }
}

impl ExplainCache {
    /// The bytes the cache holds: its indexes, the compiled program and the
    /// searches' graph (an estimate from each vector's room and each map's
    /// table; `profile::explain_cache_bytes` gives it to the bench).
    pub(crate) fn bytes(&self) -> usize {
        let parts = self.parts.get().map_or(0, Parts::bytes);
        let graph = self.graph.lock().map_or(0, |g| g.bytes());
        parts + graph
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
    /// Each predicate's triples, the rounds run and the merges made,
    /// hashed.
    fingerprint: u64,
}

impl Key {
    fn of(r: &Reasoner) -> Self {
        use std::hash::{Hash, Hasher};
        let store = r.store();
        let mut h = rustc_hash::FxHasher::default();
        for (p, table) in store.tables() {
            (p, table.by_subject().len()).hash(&mut h);
        }
        (store.rounds(), store.canon().merges()).hash(&mut h);
        Self {
            switches: r.switches(),
            terms: r.terms().len(),
            base: r.base_len(),
            stored: store.len(),
            diagnostics: r.diagnostics().len(),
            fingerprint: h.finish(),
        }
    }
}

/// The indexes, built on first use.
#[derive(Debug)]
struct Parts {
    indexes: Indexes,
    program: Compiled,
    rule_literals: Vec<u32>,
}

impl Parts {
    fn new(r: &Reasoner) -> Self {
        let terms = r.terms();
        Self {
            indexes: Indexes::new(r),
            program: program::program().compile(r.switches(), terms),
            rule_literals: datatype::rule_literals(terms).collect(),
        }
    }

    fn bytes(&self) -> usize {
        use crate::profile::vec_bytes;
        let program = vec_bytes(&self.program.rules)
            + self
                .program
                .rules
                .iter()
                .map(|r| vec_bytes(&r.body) + vec_bytes(&r.head.args))
                .sum::<usize>()
            + vec_bytes(&self.program.by_head)
            + vec_bytes(&self.program.is_eq);
        self.indexes.bytes() + program + vec_bytes(&self.rule_literals)
    }
}

/// The lexical form of dt-diff's unlisted predicate in a proof: the
/// reference evaluator's (`crates/reference/src/vocab.rs`), a literal, so
/// that a step with it is a generalized triple RDF cannot write.
pub(crate) const UNLISTED_LEXICAL: &str = "owl:differentFrom";

/// The datatype of dt-diff's unlisted predicate ([`UNLISTED_LEXICAL`]).
pub(crate) const UNLISTED_DATATYPE: &str = "urn:x-turbowl-reference:unlisted";

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

    fn closure<'a>(&'a self, parts: &'a Parts) -> Closure<'a> {
        Closure::new(self.reasoner, &parts.indexes)
    }

    /// The proof of a triple of the closure: base, or derived by the rules
    /// the switches leave on. `None` when the triple is not in the closure.
    /// Blank nodes are named as the reads write them ([`Reasoner::triples`]).
    pub fn explain(&self, triple: TripleRef<'_>) -> Option<Proof> {
        self.explain_until(triple, None).expect("a search nobody cancels ends")
    }

    /// [`Explainer::explain`], stopped by `cancel`: it is checked between
    /// the search's steps. A cancelled search drops what it built and keeps
    /// the indexes, so the next question is answered as if nothing had been
    /// cancelled. An index build is not interrupted.
    ///
    /// # Errors
    /// [`Error::Cancelled`] when `cancel` was tripped.
    pub fn explain_cancellable(&self, triple: TripleRef<'_>, cancel: &Cancel) -> Result<Option<Proof>, Error> {
        self.explain_until(triple, Some(cancel))
    }

    fn explain_until(&self, triple: TripleRef<'_>, cancel: Option<&Cancel>) -> Result<Option<Proof>, Error> {
        if cancel.is_some_and(Cancel::is_cancelled) {
            return Err(Error::Cancelled);
        }
        let terms = self.reasoner.terms();
        let (Some(s), Some(p), Some(o)) = (
            id_of(terms, triple.subject.into()),
            terms.get_iri(triple.predicate.as_str()),
            id_of(terms, triple.object),
        ) else {
            return Ok(None);
        };
        let t = [s, p.0, o];
        let parts = self.parts();
        if !self.closure(parts).contains(t) {
            return Ok(None);
        }
        let built = self.prove(parts, T, [t[0], t[1], t[2], 0], cancel)?;
        Ok(built.map(|b| self.proof(b, None)))
    }

    /// The proof of a diagnostic of the last run: the rule that reports it
    /// and the proofs of its premises. `None` when it is not reported.
    pub fn explain_diagnostic(&self, diagnostic: &Diagnostic) -> Option<Proof> {
        self.explain_diagnostic_until(diagnostic, None)
            .expect("a search nobody cancels ends")
    }

    /// [`Explainer::explain_diagnostic`], stopped by `cancel`, as
    /// [`Explainer::explain_cancellable`] is.
    ///
    /// # Errors
    /// [`Error::Cancelled`] when `cancel` was tripped.
    pub fn explain_diagnostic_cancellable(
        &self,
        diagnostic: &Diagnostic,
        cancel: &Cancel,
    ) -> Result<Option<Proof>, Error> {
        self.explain_diagnostic_until(diagnostic, Some(cancel))
    }

    fn explain_diagnostic_until(
        &self,
        diagnostic: &Diagnostic,
        cancel: Option<&Cancel>,
    ) -> Result<Option<Proof>, Error> {
        if cancel.is_some_and(Cancel::is_cancelled) {
            return Err(Error::Cancelled);
        }
        let Some((rel, tuple)) = self.goal_of(diagnostic) else {
            return Ok(None);
        };
        let built = self.prove(self.parts(), rel, tuple, cancel)?;
        Ok(built.map(|b| self.proof(b, Some(diagnostic))))
    }

    /// The finding a reported diagnostic is, as a goal of the program.
    fn goal_of(&self, diagnostic: &Diagnostic) -> Option<(program::RelId, [u32; MAX_ARITY])> {
        if !self.reasoner.diagnostics().contains(diagnostic) {
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
        Some((rel, tuple))
    }

    /// The search for one goal, over the graph earlier goals left, and its
    /// proof checked against the reasoner ([`Explainer::holds`]): a graph
    /// that gives a proof the closure does not hold (a cache from another
    /// state that matched its key) is dropped, and the search runs again
    /// on a fresh one. A search that panicked leaves a graph that may lack
    /// instances: it is dropped too, and so is one that `cancel` stopped.
    fn prove(
        &self,
        parts: &Parts,
        rel: program::RelId,
        tuple: [u32; MAX_ARITY],
        cancel: Option<&Cancel>,
    ) -> Result<Option<Built>, Error> {
        let mut graph = self.cache.graph.lock().unwrap_or_else(|poisoned| {
            let mut g = poisoned.into_inner();
            *g = Graph::default();
            g
        });
        let world = World {
            program: &parts.program,
            closure: self.closure(parts),
            rule_literals: &parts.rule_literals,
            cancel,
        };
        let search = |graph: &mut Graph| {
            let built = Search::new(world, graph).prove(rel, tuple);
            if cancel.is_some_and(Cancel::is_cancelled) {
                *graph = Graph::default();
                return Err(Error::Cancelled);
            }
            Ok(built)
        };
        let built = search(&mut graph)?;
        if built.as_ref().is_none_or(|b| self.holds(parts, b)) {
            return Ok(built);
        }
        *graph = Graph::default();
        Ok(search(&mut graph)?.filter(|b| self.holds(parts, b)))
    }

    /// True when every step of `built` holds of the reasoner: a base step
    /// is a base triple, any other a triple of the closure (dt-diff's
    /// unlisted ones between terms equal to literals of different values).
    fn holds(&self, parts: &Parts, built: &Built) -> bool {
        let closure = self.closure(parts);
        built.steps.iter().all(|s| {
            let t = s.triple;
            match s.rule {
                None => self.reasoner.is_base(&t),
                Some(_) if t[1] == UNLISTED => closure.unlisted(t[0], t[2]),
                Some(_) => closure.contains(t),
            }
        })
    }

    /// The public form of a proof in term ids.
    fn proof(&self, built: Built, diagnostic: Option<&Diagnostic>) -> Proof {
        let terms = self.reasoner.terms();
        let term = |id: u32| match id {
            UNLISTED => Term::from(Literal::new_typed_literal(
                UNLISTED_LEXICAL,
                NamedNode::new_unchecked(UNLISTED_DATATYPE),
            )),
            id => terms.to_oxrdf(TermId(id)),
        };
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
        TermRef::NamedNode(_) | TermRef::Literal(_) => terms.get_term(term, DocId::NONE),
        #[cfg(feature = "rdf-12")]
        TermRef::Triple(t) => terms.get_written_triple(t.as_ref()),
    }
    .map(|t| t.0)
}
