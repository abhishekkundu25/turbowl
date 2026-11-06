//! The canon: which term ids `owl:sameAs` has made equal, and the one id,
//! the representative, that stands for each class of them (ticket 11, ADR
//! 0003).
//!
//! A union-find with weighted union in which every member points straight
//! at its representative: [`Canon::find`] is one array read, from `&self`,
//! so rules and rewriting read it without locks or path compression. A
//! merge relabels the members of the class that loses, and the larger class
//! wins (so each id is relabelled O(log n) times over n merges), except
//! that a class holding a *constant* (an id below [`Canon::constants`], the
//! terms the rules name) is always represented by its smallest constant.
//! The vocabulary is interned first, so a constant is the smallest id of
//! any class it is in, and RDFox's "smallest id wins" holds wherever a
//! constant is involved (Motik et al., AAAI 2015, appendix A).
//!
//! The canon only answers which ids are equal. The store rewrites its
//! tuples through it ([`super::Store`]); the reasoner expands each
//! representative back to its members on output.

use rustc_hash::FxHashMap;

/// Equal term ids and their representatives.
#[derive(Debug, Clone, Default)]
pub(crate) struct Canon {
    /// The representative of each id below its length; an id past the end
    /// was never merged and is its own. Every id of a class of two or more
    /// is below the length.
    rep: Vec<u32>,
    /// The members of each class of two or more (its representative
    /// included), by representative.
    classes: FxHashMap<u32, Vec<u32>>,
    /// Ids below this are the rules' constants.
    constants: u32,
    /// Constants whose class must hold nothing else for the rules to stay
    /// semi-naive: the rules test them for identity (`rdf:nil`, where a
    /// list ends), so a member joining their class changes those rules.
    watched: Vec<u32>,
    /// Merges made: unions of two different classes.
    merges: usize,
}

/// What [`Canon::merge`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Merged {
    /// The two ids were already equal.
    Nothing,
    /// Two classes became one; the rules' constants are where they were.
    Classes,
    /// Two classes became one, and a rule constant changed: a constant
    /// lost its place as representative, or a watched constant's class
    /// grew. The rules must be evaluated again over everything.
    Constant,
}

/// The members of a class: one id, or a slice.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Members<'a> {
    One(u32),
    Many(&'a [u32]),
}

impl<'a> Members<'a> {
    pub(crate) fn len(self) -> usize {
        match self {
            Members::One(_) => 1,
            Members::Many(m) => m.len(),
        }
    }

    pub(crate) fn iter(self) -> impl Iterator<Item = u32> + Clone + 'a {
        let (one, many) = match self {
            Members::One(id) => (Some(id), &[][..]),
            Members::Many(m) => (None, m),
        };
        one.into_iter().chain(many.iter().copied())
    }
}

impl Canon {
    /// No two ids equal; ids below `constants` are the rules' constants,
    /// and `watched` those that the rules also test for identity.
    pub(crate) fn new(constants: u32, watched: &[u32]) -> Self {
        debug_assert!(watched.iter().all(|&w| w < constants), "a watched id is a constant");
        Self {
            constants,
            watched: watched.to_vec(),
            ..Self::default()
        }
    }

    /// A canon from its parts, as [`Canon::constants`], [`Canon::watched`],
    /// [`Canon::rep`], [`Canon::classes`] and [`Canon::merges`] give them: a
    /// loaded snapshot (`snapshot.rs`), which checks the invariants of the
    /// module doc first.
    pub(crate) fn restore(
        constants: u32,
        watched: Vec<u32>,
        rep: Vec<u32>,
        classes: Vec<(u32, Vec<u32>)>,
        merges: usize,
    ) -> Self {
        debug_assert!(watched.iter().all(|&w| w < constants));
        debug_assert!(
            classes
                .iter()
                .all(|(r, members)| members.iter().all(|&m| rep[m as usize] == *r))
        );
        Self {
            rep,
            classes: classes.into_iter().collect(),
            constants,
            watched,
            merges,
        }
    }

    /// The representative of each id below its length; an id past the end
    /// is its own ([`Canon::find`]).
    pub(crate) fn rep(&self) -> &[u32] {
        &self.rep
    }

    /// The constants the rules also test for identity.
    pub(crate) fn watched(&self) -> &[u32] {
        &self.watched
    }

    /// The representative of `id`.
    #[inline]
    pub(crate) fn find(&self, id: u32) -> u32 {
        self.rep.get(id as usize).copied().unwrap_or(id)
    }

    /// True when no two ids are equal.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }

    /// The members of the class whose representative is `rep` (`rep` alone
    /// when its class has one member).
    #[inline]
    pub(crate) fn members(&self, rep: u32) -> Members<'_> {
        if self.classes.is_empty() {
            return Members::One(rep);
        }
        match self.classes.get(&rep) {
            Some(m) => Members::Many(m),
            None => Members::One(rep),
        }
    }

    /// Every class of two or more: its representative and its members.
    pub(crate) fn classes(&self) -> impl Iterator<Item = (u32, &[u32])> {
        self.classes.iter().map(|(&r, m)| (r, &m[..]))
    }

    /// Merges made so far.
    pub(crate) fn merges(&self) -> usize {
        self.merges
    }

    /// The bytes the canon holds: the representatives, the classes'
    /// members and their map, and the watched constants.
    pub(crate) fn bytes(&self) -> usize {
        use crate::profile::{hash_bytes, vec_bytes};
        vec_bytes(&self.rep)
            + hash_bytes(self.classes.capacity(), std::mem::size_of::<(u32, Vec<u32>)>())
            + self.classes.values().map(vec_bytes).sum::<usize>()
            + vec_bytes(&self.watched)
    }

    /// Ids below this are the rules' constants.
    pub(crate) fn constants(&self) -> u32 {
        self.constants
    }

    /// Makes `a` and `b` equal. Every id whose representative changed is
    /// pushed to `moved`.
    pub(crate) fn merge(&mut self, a: u32, b: u32, moved: &mut Vec<u32>) -> Merged {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return Merged::Nothing;
        }
        let size = |r: u32| self.classes.get(&r).map_or(1, Vec::len);
        let (sa, sb) = (size(ra), size(rb));
        let low = ra.min(rb);
        let winner = if low < self.constants || sa == sb {
            low
        } else if sa > sb {
            ra
        } else {
            rb
        };
        let (loser, winner_size) = if winner == ra { (rb, sa) } else { (ra, sb) };
        let members = self.classes.remove(&loser).unwrap_or_else(|| vec![loser]);
        let top = members.iter().copied().chain([winner]).max().expect("not empty") as usize;
        if top >= self.rep.len() {
            let from = self.rep.len() as u32;
            self.rep.extend(from..=top as u32);
        }
        for &m in &members {
            self.rep[m as usize] = winner;
        }
        moved.extend_from_slice(&members);
        self.classes
            .entry(winner)
            .or_insert_with(|| vec![winner])
            .extend(members);
        self.merges += 1;
        let watched_grew = winner_size == 1 && self.watched.contains(&winner);
        if loser < self.constants || watched_grew {
            Merged::Constant
        } else {
            Merged::Classes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn members(c: &Canon, id: u32) -> Vec<u32> {
        let mut m: Vec<u32> = c.members(c.find(id)).iter().collect();
        m.sort_unstable();
        m
    }

    #[test]
    fn the_larger_class_wins_and_ties_go_to_the_smaller_id() {
        let mut c = Canon::new(10, &[]);
        let mut moved = Vec::new();
        assert_eq!(c.merge(20, 21, &mut moved), Merged::Classes);
        assert_eq!((c.find(20), c.find(21)), (20, 20));
        assert_eq!(moved, [21]);
        moved.clear();
        // {20, 21} is larger than {15}: 15 moves, though its id is smaller.
        assert_eq!(c.merge(15, 21, &mut moved), Merged::Classes);
        assert_eq!(c.find(15), 20);
        assert_eq!(moved, [15]);
        assert_eq!(members(&c, 15), [15, 20, 21]);
        assert_eq!(c.merge(21, 15, &mut moved), Merged::Nothing);
        assert_eq!(c.merges(), 2);
        // Ids never merged are their own, past the end of the array too.
        assert_eq!((c.find(3), c.find(1_000)), (3, 1_000));
        assert_eq!(c.members(1_000).len(), 1);
    }

    #[test]
    fn a_constant_represents_its_class() {
        let mut c = Canon::new(10, &[]);
        let mut moved = Vec::new();
        for i in 21..30 {
            c.merge(20, i, &mut moved);
        }
        moved.clear();
        // A class of ten against the constant 4: 4 wins, and the ten move.
        assert_eq!(c.merge(25, 4, &mut moved), Merged::Classes);
        assert_eq!(c.find(29), 4);
        assert_eq!(moved.len(), 10);
        // Two constants: the smaller wins, and the larger one moved.
        moved.clear();
        assert_eq!(c.merge(2, 29, &mut moved), Merged::Constant);
        assert_eq!(c.find(4), 2);
        assert_eq!(members(&c, 20).len(), 12);
    }

    #[test]
    fn a_watched_constant_reports_its_class_growing_once() {
        let mut c = Canon::new(10, &[7]);
        let mut moved = Vec::new();
        assert_eq!(c.merge(30, 7, &mut moved), Merged::Constant);
        assert_eq!(c.find(30), 7);
        assert_eq!(c.merge(31, 30, &mut moved), Merged::Classes);
        assert_eq!(members(&c, 31), [7, 30, 31]);
    }
}
