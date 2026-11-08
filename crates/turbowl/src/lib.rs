//! TurbOWL: a fast OWL 2 RL reasoner.
//!
//! One deep module behind a small interface. Load documents or insert
//! triples, [`Reasoner::reason`] to the fixpoint, then read the closure,
//! the inferred triples and the [`Diagnostic`]s. Runs after a change reuse
//! the previous closure. The rules are those of the W3C OWL 2 RL rule
//! tables (OWL 2 Profiles §4.3), grouped by [`Switch`]
//! (see `docs/adr/0004-w3c-rules-are-the-target.md` and
//! `docs/adr/0005-switches.md`).
//!
//! ```
//! use turbowl::{Format, Reasoner};
//! let mut r = Reasoner::new();
//! r.load_str("@prefix ex: <http://example.org/> .
//!             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
//!             ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog .", Format::Turtle, None).unwrap();
//! let summary = r.reason();
//! assert!(summary.inferred > 0);
//! ```
//!
//! # Switches
//!
//! Every rule belongs to the always-on core or to exactly one switch, a
//! named group of rules the caller turns on or off ([`Switches`], given to
//! [`Reasoner::with_switches`] or [`Reasoner::set_switches`]). The core
//! holds the class and property rules: cax-sco, cax-eqc1/2, cls-thing,
//! cls-nothing1, cls-int1/2, cls-uni, cls-svf1/2, cls-avf, cls-hv1/2,
//! cls-oo, prp-dom, prp-rng, prp-fp, prp-ifp, prp-symp, prp-trp, prp-spo1,
//! prp-eqp1/2, prp-inv1/2, scm-sco and scm-eqc1.
//!
//! A switch is off by default when other reasoners rarely implement its
//! rules, or when turning it on slows reasoning by more than 5% on a
//! scoreboard scenario (ADR 0005). [`Switches::complete`] turns every
//! switch on: the complete W3C rule set.
//!
//! | Switch | Default | Why |
//! |---|---|---|
//! | `equality` | on | Version 1's behaviour: `owl:sameAs` makes terms equal, as in other OWL 2 RL reasoners. |
//! | `property-chains` | on | Version 1's behaviour: chains were on by default there. |
//! | `checks` | on | Version 1's behaviour: contradictions and ill-typed literals are reported. |
//! | `identity` | off | Slower by more than 5%: 26% on uobm-010-u, where it adds 18 M of the closure's 39 M triples (the phase 1 close-out, 2026-11-06). Common elsewhere: owlrl, GraphDB and the Nemo program have it. |
//! | `schema` | off | Slower by more than 5%: 22% to 39% on every editor-size scenario and up to 62% on the data-size ones, a cost that grows with the data (the phase 1 close-out). Common elsewhere, as `identity`. |
//! | `trivial` | off | Rarely implemented elsewhere, and filler: `x owl:sameAs x` for every term, four triples for every class, its datatypes for every literal. |
//! | `literal-values` | off | Rarely implemented elsewhere, and it reads every literal's value; dt-diff only checks, and lists no `owl:differentFrom` triples. |
//!
//! The rules of each switch are listed on its [`Switch`] variant.
//!
//! # Threads
//!
//! A run uses every core by default ([`Threads::ALL`]), round by round: a
//! round whose work passes a measured size (65,536 tuples) runs its rule
//! families as parallel tasks, splits large deltas into chunks, and sorts
//! and advances each predicate's table on its own thread. Smaller rounds,
//! such as those after a small change in an editor, run on the caller's
//! thread and never wait on a thread pool. [`Reasoner::set_threads`] takes
//! [`Threads::ONE`] for one thread, and [`Threads::every_round`] makes every
//! round parallel however small (for tests). The closure, the diagnostics,
//! the [`Summary`] and the order of every read are the same at any thread
//! count. The threads are rayon's global pool, whose size the
//! `RAYON_NUM_THREADS` variable sets: `RAYON_NUM_THREADS=1` runs every
//! round on the caller's thread, as [`Threads::ONE`] does. [`Threads`] is
//! in the interface for the chaos tests (`crates/reference`), which run
//! each input at one thread and at all cores and compare the two.
//!
//! The `parallel` cargo feature, on by default, brings the threads. Without
//! it, and always on WebAssembly, every round runs on the caller's thread.
//!
//! # Snapshots
//!
//! A snapshot is a reasoner's whole state in one versioned, checksummed
//! byte string: the terms, the base, the closure as stored, the
//! diagnostics, the switches, the documents loaded, and any change made
//! since the last run. Loading one gives the saved reasoner back without
//! parsing or reasoning, and the next changes and runs do what they would
//! have done on it. A snapshot of another format version or another
//! TurbOWL version, or one truncated, damaged or malformed, is refused with
//! [`Error::Snapshot`].
//!
//! ```
//! use turbowl::{Format, Reasoner};
//! let mut r = Reasoner::new();
//! r.load_str("@prefix ex: <http://example.org/> .
//!             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
//!             ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog .", Format::Turtle, None).unwrap();
//! let summary = r.reason();
//! let mut snapshot = Vec::new();
//! r.write_snapshot(&mut snapshot).unwrap();
//! let mut again = Reasoner::from_snapshot_bytes(&snapshot).unwrap();
//! assert_eq!(again.reason().inferred, summary.inferred);
//! assert!(Reasoner::from_snapshot_bytes(&snapshot[..100]).is_err());
//! ```
//!
//! The interface gained four functions and an error variant for them, each
//! for an adapter:
//!
//! - [`Reasoner::save_snapshot`] (written atomically) and
//!   [`Reasoner::from_snapshot`], with files (the `io` feature): for the
//!   CLI's `--save-snapshot` and `--load-snapshot`, and Python's
//!   `Reasoner.save_snapshot` and `Reasoner.from_snapshot`.
//! - [`Reasoner::write_snapshot`] and [`Reasoner::from_snapshot_bytes`],
//!   with any writer and a byte slice (always built): for the oracle
//!   crate's round trips, and for WebAssembly, which has no files.
//! - [`Error::Snapshot`]: the refusal, which the CLI and Python report.
//!
//! The format, and what loading checks, are documented in `src/snapshot.rs`.
//!
//! # Explanations
//!
//! [`Reasoner::explainer`] gives an [`Explainer`] over the closure of the
//! last run. [`Explainer::explain`] proves a triple of the closure and
//! [`Explainer::explain_diagnostic`] a diagnostic, as a [`Proof`]: a DAG of
//! [`Step`]s in topological order, each a [`Statement`] (three terms: a
//! generalized triple, since a proof may pass through a literal subject)
//! with its [`Reason`], [`Reason::Base`] or a W3C rule over earlier steps
//! (an axiom rule such as cls-thing has none). Only the goal can be a
//! diagnostic ([`Proof::goal`], [`Goal`]). [`Proof::leaves`] gives the base
//! triples, [`Proof::depth`] the most rule steps to a leaf, and `Display`
//! an indented tree. A triple outside the closure, or a diagnostic not
//! reported, has no proof (`None`).
//!
//! ```
//! use oxrdf::{NamedNode, Triple, vocab::rdf};
//! use turbowl::{Format, Reason, Reasoner, Switch, Switches};
//! let mut r = Reasoner::with_switches(Switches::default().off(Switch::Equality));
//! r.load_str("@prefix ex: <http://example.org/> .
//!             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
//!             ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog .", Format::Turtle, None).unwrap();
//! r.reason();
//! let ex = |l: &str| NamedNode::new_unchecked(format!("http://example.org/{l}"));
//! let proof = r.explain(Triple::new(ex("rex"), rdf::TYPE, ex("Animal")).as_ref()).unwrap().unwrap();
//! assert!(matches!(proof.reason(), Reason::Rule { rule: "cax-sco", .. }));
//! assert_eq!(proof.leaves().count(), 2);
//! ```
//!
//! A proof is computed on demand: the explainer runs the declarative rule
//! program (`oracles/rules/`, which every oracle holds equal to TurbOWL's
//! rules) backwards over the closure, with the rules the switches leave
//! on, and keeps the shallowest proof. Nothing is recorded while reasoning
//! runs, so reasoning costs what it did. Every configuration is explained,
//! `equality` and `literal-values` included: a proof is written in the
//! terms the user wrote, and a triple that holds through an `owl:sameAs`
//! class gets the eq-rep step and the chain of sameAs links it rests on.
//! The search reads the store's tables; the indexes it needs are built on
//! first use and kept in the [`ExplainCache`], never in the reasoner.
//!
//! The interface gained these for the adapters, the CLI's `--explain` and
//! Python's `Reasoner.explain`, and for the proof checks of the oracle and
//! reference crates:
//!
//! - [`Reasoner::explainer`] and [`Explainer`], with
//!   [`Reasoner::explain`] and [`Reasoner::explain_diagnostic`] as one-call
//!   forms.
//! - [`ExplainCache`]: the explainer's indexes and what its searches
//!   found, owned and `Send + Sync`, which [`Reasoner::explainer_with`]
//!   reuses and [`Explainer::into_cache`] gives back, so that an editor or
//!   Python pays for them once while the reasoner does not change.
//! - [`Proof`], [`Step`], [`Statement`], [`Reason`] and [`Goal`].
//! - [`Error::NotReasoned`]: there is no closure to explain while the base
//!   or the switches changed since the last run.
//! - [`profile::explain_cache_bytes`]: the bytes an [`ExplainCache`] holds,
//!   for the bench's `--explain`, which cannot tell them apart from
//!   reasoning in the process's peak memory.
//!
//! # Cancellation
//!
//! A [`Cancel`] stops a run or a proof search from any thread:
//! [`Reasoner::reason_cancellable`] checks it before each round, and
//! [`Explainer::explain_cancellable`] and
//! [`Explainer::explain_diagnostic_cancellable`] between the search's
//! steps; a tripped handle gives [`Error::Cancelled`]. [`Reasoner::reason`]
//! runs the same path without a handle. A cancelled run keeps no
//! partial result: the reasoner is as loaded and not yet reasoned (no
//! closure, no diagnostics, [`Error::NotReasoned`] from the explainer), and
//! the next run is a full one. A cancelled search keeps the explainer's
//! indexes. A round, an index build and a ranking of the whole search graph
//! are not interrupted.
//!
//! ```
//! use turbowl::{Cancel, Error, Format, Reasoner};
//! let mut r = Reasoner::new();
//! r.load_str("<http://example.org/a> <http://example.org/p> <http://example.org/b> .", Format::NTriples, None).unwrap();
//! let cancel = Cancel::new();
//! cancel.cancel(); // from another thread, as an editor does on the next keystroke
//! assert!(matches!(r.reason_cancellable(&cancel), Err(Error::Cancelled)));
//! assert_eq!(r.triples().count(), 0);
//! r.reason();
//! ```
//!
//! The interface gained these for an editor's event loop (turtle-lsp) and
//! Python's `cancel=` and Ctrl+C:
//!
//! - [`Cancel`], [`Reasoner::reason_cancellable`],
//!   [`Explainer::explain_cancellable`] and
//!   [`Explainer::explain_diagnostic_cancellable`].
//! - [`Error::Cancelled`].
//! - [`profile::cancel_after`]: a handle that trips itself at its n-th
//!   check, for chaos and the tests.
//!
//! # Counters
//!
//! [`Reasoner::profile`] tells what the last run did and what the reasoner
//! holds ([`profile::Profile`]): per round, the delta and each family's
//! emitted tuples and time; per step, the time in sorting and in finding
//! what changed, and the distinct and new tuples; and the bytes held by the
//! dictionary, the base, the store (`rdf:type`'s table, the others, the
//! five largest), the canon and the findings. Every run keeps the counts,
//! at an integer add per family and round and a few clock readings per
//! round and step; the bytes are computed only when asked.
//!
//! The method and the [`profile`] module are in the interface for one
//! adapter, the bench (`crates/bench`), which records them in its results
//! file so that every claim of the improvement loop
//! (`docs/specs/experiments.md`) comes with what bounds it. They are kept
//! out of the CLI and Python. Their fields may change with what the loop
//! needs to measure.
//!
//! # Cargo features
//!
//! - `io`, on by default: parsing and writing documents ([`Format`]).
//! - `parallel`, on by default: rounds on every core (see Threads).
//! - `rdf-12`, off by default: RDF 1.2 triple terms (`<<( s p o )>>`), as
//!   oxrdf's `rdf-12` feature has them. A triple term is an opaque term,
//!   interned whole as a literal is: a rule carries it (as the object of
//!   `rdf:reifies`, or through `eq-rep-o`) but never looks inside it, and
//!   the triple inside is not asserted. Two triple terms are one term when
//!   oxrdf's terms are equal; the blank nodes inside one belong to its
//!   document and are written as other blank nodes are. A triple whose
//!   subject is a triple term (from `eq-sym`, say) is not RDF: it is neither
//!   read nor counted, as one with a literal subject is not. An embedder
//!   that turns on `oxrdf/rdf-12` must turn this on too, or the crate does
//!   not compile. With `io`, it turns on the parsers' `rdf-12` features
//!   too (they match on oxrdf's terms as well). Of the formats, RDF/XML
//!   reads triple terms (`rdf:parseType="Triple"`); Turtle, N-Triples and
//!   N3 reject them as syntax errors.
//!
//! Everything else (term ids, the dictionary, relations, the engine, the
//! rule families) is internal: see `CONTEXT.md` for the words.

mod cancel;
mod diagnostics;
mod engine;
mod error;
mod explain;
#[cfg(feature = "io")]
mod io;
pub mod profile;
mod reasoner;
mod rules;
#[cfg(test)]
mod scaling;
mod snapshot;
mod switch;
mod term;
mod threads;

pub use cancel::Cancel;
pub use diagnostics::{Diagnostic, Severity};
pub use error::Error;
pub use explain::{ExplainCache, Explainer, Goal, Proof, Reason, Statement, Step};
pub use reasoner::{Format, Reasoner, Summary};
pub use switch::{Switch, Switches};
pub use threads::Threads;
