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
//! Everything else (term ids, the dictionary, relations, the engine, the
//! rule families) is internal: see `CONTEXT.md` for the words.

mod diagnostics;
mod engine;
mod error;
#[cfg(feature = "io")]
mod io;
mod reasoner;
mod rules;
#[cfg(test)]
mod scaling;
mod snapshot;
mod switch;
mod term;
mod threads;

pub use diagnostics::{Diagnostic, Severity};
pub use error::Error;
pub use reasoner::{Format, Reasoner, Summary};
pub use switch::{Switch, Switches};
pub use threads::Threads;
