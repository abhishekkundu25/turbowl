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
//! A family is a `RuleFn` (`rules/mod.rs`): `fn(&mut Round<'_>, &mut
//! Context<'_>)`. [`Store::run`] takes any closure over a [`Round`]; each
//! round the reasoner (ticket 10) calls every family's function with the
//! same [`Round`]: the store as the last round left it, read-only, and a
//! buffer for derived triples ([`Round::emit`]). A family reads predicate tables
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
//! # Equality (ticket 11)
//!
//! [`Canon`] is a union-find of term ids. A rule asks for a merge
//! ([`Round::merge`]); the next step merges, rewrites every stored tuple
//! that mentions a moved id ([`Variable::remap`] moves the rewritten tuples
//! back into the deltas), moves a merged-away predicate's table into its
//! representative's, and rewrites the round's derived triples before they
//! are sorted. Rules read the constants they name through the canon
//! ([`Round::constant`]). A merge that changes a constant stops the run
//! ([`Store::restarted`]); see `store.rs`.

// The engine offers more than version 1's rules use: leapjoin, antijoin and
// `join_static` for the experiments. Its tests use all of it.
#![allow(dead_code, unused_imports)]

mod canon;
mod closure;
mod join;
mod leapjoin;
mod relation;
mod store;
mod variable;

#[cfg(test)]
mod tests;

pub(crate) use canon::{Canon, Members, Merged};
pub(crate) use closure::transitive_closure;
pub(crate) use join::{
    antijoin, antijoin_slices, join, join_count, join_slices, join_slices_count, join_static, join_views,
    join_views_count,
};
pub(crate) use leapjoin::{
    ExtendAnti, ExtendWith, FilterAnti, FilterWith, Leaper, Leapers, ValueFilter, extend_anti, extend_with,
    filter_anti, filter_with, leapjoin, value_filter,
};
pub(crate) use relation::{Relation, gallop};
pub(crate) use store::{Pair, Round, Store, Table};
pub(crate) use variable::{Variable, View};
