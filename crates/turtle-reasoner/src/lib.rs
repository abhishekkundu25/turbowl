//! turtle-reasoner: a fast OWL 2 RL reasoner.
//!
//! One deep module behind a small interface. Load documents or insert
//! triples, [`Reasoner::reason`] to the fixpoint, then read the closure,
//! the inferred triples and the [`Diagnostic`]s. Runs after a change reuse
//! the previous closure. Two [`Profile`]s: the W3C OWL 2 RL rule set, and the
//! conventions of `reasonable` (see `docs/adr/0002-parity-with-reasonable.md`).
//!
//! ```no_run
//! use turtle_reasoner::{Format, Profile, Reasoner};
//! let mut r = Reasoner::new(Profile::OwlRl);
//! r.load_str("@prefix ex: <http://example.org/> .
//!             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
//!             ex:Dog rdfs:subClassOf ex:Animal . ex:rex a ex:Dog .", Format::Turtle, None).unwrap();
//! let summary = r.reason();
//! assert!(summary.inferred > 0);
//! ```
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
mod term;

pub use diagnostics::{Diagnostic, Severity};
pub use error::Error;
pub use reasoner::{Format, Profile, Reasoner, Summary};
