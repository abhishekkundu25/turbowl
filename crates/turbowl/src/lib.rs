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
//! | `identity` | on | Until the close-out of phase 1 (ticket 15) measures it against the rule above. |
//! | `schema` | on | Until the close-out of phase 1 (ticket 15) measures it against the rule above. |
//! | `trivial` | off | Rarely implemented elsewhere, and filler: `x owl:sameAs x` for every term, four triples for every class, a type for every literal. |
//! | `literal-values` | off | Rarely implemented elsewhere, and it reads every literal's value; dt-diff only checks, and lists no `owl:differentFrom` triples. |
//!
//! The rules of each switch are listed on its [`Switch`] variant. While
//! version 2 is built, a group whose rule ticket has not landed derives
//! nothing (`docs/tickets/v2/README.md` says which have).
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
mod switch;
mod term;

pub use diagnostics::{Diagnostic, Severity};
pub use error::Error;
pub use reasoner::{Format, Reasoner, Summary};
pub use switch::{Switch, Switches};
