//! Merge joins and antijoins on the first field of `(key, value)` tuples.
//!
//! Results go to a callback, not into a relation: a rule pushes them where it
//! wants (a round's output, a temporary vector), and nothing is allocated
//! here. No hashing: both sides are sorted, and a mismatch gallops.

use super::relation::gallop;
use super::variable::{Variable, View};

/// Joins two sorted slices on their keys: `f(key, a_value, b_value)` for
/// every pair of tuples with equal keys.
pub(crate) fn join_slices<K, A, B>(a: &[(K, A)], b: &[(K, B)], f: &mut impl FnMut(K, A, B))
where
    K: Ord + Copy,
    A: Copy,
    B: Copy,
{
    groups(a, b, |ka, ga, gb| {
        for &(_, va) in ga {
            for &(_, vb) in gb {
                f(ka, va, vb);
            }
        }
    });
}

/// How many results [`join_slices`] gives, counted without making them.
pub(crate) fn join_slices_count<K, A, B>(a: &[(K, A)], b: &[(K, B)]) -> usize
where
    K: Ord + Copy,
{
    let mut count = 0;
    groups(a, b, |_, ga, gb| count += ga.len() * gb.len());
    count
}

/// Calls `f(key, a_group, b_group)` for each key both sorted slices hold,
/// with the run of tuples each holds under it.
fn groups<'a, 'b, K, A, B>(mut a: &'a [(K, A)], mut b: &'b [(K, B)], mut f: impl FnMut(K, &'a [(K, A)], &'b [(K, B)]))
where
    K: Ord + Copy,
{
    while let (Some(&(ka, _)), Some(&(kb, _))) = (a.first(), b.first()) {
        match ka.cmp(&kb) {
            std::cmp::Ordering::Less => a = gallop(a, |t| t.0 < kb),
            std::cmp::Ordering::Greater => b = gallop(b, |t| t.0 < ka),
            std::cmp::Ordering::Equal => {
                let na = a.iter().take_while(|t| t.0 == ka).count();
                let nb = b.iter().take_while(|t| t.0 == ka).count();
                f(ka, &a[..na], &b[..nb]);
                a = &a[na..];
                b = &b[nb..];
            }
        }
    }
}

/// Joins every part of `a` with every part of `b`.
pub(crate) fn join_views<K, A, B>(a: &View<'_, (K, A)>, b: &View<'_, (K, B)>, mut f: impl FnMut(K, A, B))
where
    K: Ord + Copy,
    A: Ord + Copy,
    B: Ord + Copy,
{
    for pa in a.parts() {
        for pb in b.parts() {
            join_slices(pa, pb, &mut f);
        }
    }
}

/// The semi-naive join of two variables: every pair where at least one side
/// is recent, each pair once over a run (recent × stable, stable × recent,
/// recent × recent). `a` and `b` may be the same variable.
pub(crate) fn join<K, A, B>(a: &Variable<(K, A)>, b: &Variable<(K, B)>, mut f: impl FnMut(K, A, B))
where
    K: Ord + Copy,
    A: Ord + Copy,
    B: Ord + Copy,
{
    let (ra, rb) = (a.recent(), b.recent());
    if !ra.is_empty() {
        for sb in b.stable() {
            join_slices(ra, sb, &mut f);
        }
    }
    if !rb.is_empty() {
        for sa in a.stable() {
            join_slices(sa, rb, &mut f);
        }
        join_slices(ra, rb, &mut f);
    }
}

/// How many results [`join_views`] gives, counted without making them.
pub(crate) fn join_views_count<K, A, B>(a: &View<'_, (K, A)>, b: &View<'_, (K, B)>) -> usize
where
    K: Ord + Copy,
    A: Ord + Copy,
    B: Ord + Copy,
{
    let mut count = 0;
    for pa in a.parts() {
        for pb in b.parts() {
            count += join_slices_count(pa, pb);
        }
    }
    count
}

/// How many results [`join`] gives this round, counted without making them.
pub(crate) fn join_count<K, A, B>(a: &Variable<(K, A)>, b: &Variable<(K, B)>) -> usize
where
    K: Ord + Copy,
    A: Ord + Copy,
    B: Ord + Copy,
{
    let (ra, rb) = (a.recent(), b.recent());
    let mut count = 0;
    if !ra.is_empty() {
        count += b.stable().iter().map(|sb| join_slices_count(ra, sb)).sum::<usize>();
    }
    if !rb.is_empty() {
        count += a.stable().iter().map(|sa| join_slices_count(sa, rb)).sum::<usize>();
        count += join_slices_count(ra, rb);
    }
    count
}

/// The semi-naive join of a variable with a relation that does not change
/// during the run: only `a`'s recent part is joined.
pub(crate) fn join_static<K, A, B>(a: &Variable<(K, A)>, b: &View<'_, (K, B)>, mut f: impl FnMut(K, A, B))
where
    K: Ord + Copy,
    A: Ord + Copy,
    B: Ord + Copy,
{
    let ra = a.recent();
    if ra.is_empty() {
        return;
    }
    for pb in b.parts() {
        join_slices(ra, pb, &mut f);
    }
}

/// `f(key, value)` for every tuple of `input` whose key is in no part of
/// `keys`.
pub(crate) fn antijoin_slices<K, V>(input: &[(K, V)], keys: &View<'_, K>, mut f: impl FnMut(K, V))
where
    K: Ord + Copy,
    V: Copy,
{
    let mut cursors: Vec<&[K]> = keys.parts().to_vec();
    let mut rest = input;
    while let Some(&(k, _)) = rest.first() {
        let n = rest.iter().take_while(|t| t.0 == k).count();
        let mut found = false;
        for cursor in &mut cursors {
            *cursor = gallop(cursor, |x| *x < k);
            found |= cursor.first() == Some(&k);
        }
        if !found {
            for &(_, v) in &rest[..n] {
                f(k, v);
            }
        }
        rest = &rest[n..];
    }
}

/// The semi-naive antijoin: `input`'s recent tuples whose key is not in
/// `keys`. `keys` must not change during the run (stratified negation).
pub(crate) fn antijoin<K, V>(input: &Variable<(K, V)>, keys: &View<'_, K>, f: impl FnMut(K, V))
where
    K: Ord + Copy,
    V: Ord + Copy,
{
    antijoin_slices(input.recent(), keys, f);
}
