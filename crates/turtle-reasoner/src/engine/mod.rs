//! The engine: sorted term-id relations, semi-naive rounds, joins (ticket 03,
//! ADR 0001). It knows no rules; the rule families in `rules/` drive it.
//!
//! The design follows datafrog (<https://github.com/rust-lang/datafrog>, by
//! Frank McSherry and the Rust compiler team): relations as sorted,
//! duplicate-free vectors; variables with stable batches, a recent delta and
//! queued additions; merge joins that gallop; leapjoin by proposing and
//! intersecting one variable's values. The code is written for this crate,
//! with these differences:
//!
//! - No `Rc<RefCell<…>>`: joins borrow their inputs and hand each result to a
//!   callback, and new tuples are queued after the reads end. The borrow
//!   checker sees every read and write.
//! - [`View`]s let leapers and joins read a variable's stable batches and
//!   recent part in place, so n-way semi-naive rules need no merged copy.
//! - [`Store`] keeps one predicate table per predicate term id
//!   ([`Table`]: `(s, o)` pairs, plus `(o, s)` pairs once a rule asks), and
//!   drives the rounds.
//!
//! # Interface for the rule families
//!
//! A family is a [`RuleFn`], `fn(&mut Round<'_>)`. Each round, the reasoner
//! (ticket 10) calls every family's function with the same [`Round`]: the
//! store as the last round left it, read-only, and a buffer for derived
//! triples ([`Round::emit`]). A family reads predicate tables
//! ([`Round::by_subject`], [`Round::by_object`]), joins their recent parts
//! with the rest ([`join`], [`leapjoin`]) and emits what it derives. When all
//! families have run, [`Store::run`] sorts the buffer by predicate, queues
//! each group in its table, and ends the round; a round with nothing new is
//! the fixpoint.
//!
//! Why a plain `fn` pointer: families are stateless code in a `static`
//! table, which `fn` pointers allow and trait objects with lifetimes make
//! awkward; a call per family per round costs nothing next to the joins;
//! and with no state in the families, everything a run knows is in the
//! [`Store`], so an incremental run (more inserts, then `run` again) and the
//! canonicalisation of ticket 11 see all of it.
//!
//! Term ids are plain `u32` here (`TermId.0`): the engine is generic over
//! tuples of `Ord + Copy`, and `(u32, u32)` pairs sort and compare as fast as
//! anything can.
//!
//! # Where equality rewriting goes (ticket 11)
//!
//! `Store::step` has the place marked: rewrite the round's derived triples
//! through the canon map before they are sorted, and call
//! [`Variable::remap`] on the tables that mention a merged term, which moves
//! the rewritten tuples back into the deltas.

// The rule families (tickets 07-09) and the reasoner (ticket 10) are the
// users; until they land, only the tests are.
#![allow(dead_code, unused_imports)]

mod join;
mod leapjoin;
mod relation;
mod store;
mod variable;

#[cfg(test)]
mod tests;

pub(crate) use join::{antijoin, antijoin_slices, join, join_slices, join_static, join_views};
pub(crate) use leapjoin::{
    ExtendAnti, ExtendWith, FilterAnti, FilterWith, Leaper, Leapers, ValueFilter, extend_anti, extend_with,
    filter_anti, filter_with, leapjoin, value_filter,
};
pub(crate) use relation::{Relation, gallop};
pub(crate) use store::{Pair, Round, Store, Table};
pub(crate) use variable::{Variable, View};

/// What a rule family provides: one round of its rules.
pub(crate) type RuleFn = fn(&mut Round<'_>);
