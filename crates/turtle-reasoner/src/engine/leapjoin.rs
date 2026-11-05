//! Leapjoin: extend each source tuple by one value, the values every leaper
//! agrees on. The leaper with the fewest candidates proposes; the others
//! intersect. This is datafrog's single-variable extension over sorted
//! relations (ADR 0001), not a trie iterator.

use super::relation::gallop;
use super::variable::View;

/// One constraint on the value that extends a prefix `P`.
pub(crate) trait Leaper<P, V> {
    /// How many values it would propose (`usize::MAX`: it only filters).
    fn count(&mut self, prefix: &P) -> usize;
    /// Pushes its values, sorted and distinct.
    fn propose(&mut self, prefix: &P, values: &mut Vec<V>);
    /// Keeps the values (sorted) it allows.
    fn intersect(&mut self, prefix: &P, values: &mut Vec<V>);
}

/// A tuple of leapers, used together.
pub(crate) trait Leapers<P, V> {
    /// Calls `f(index, count)` for each leaper.
    fn for_each_count(&mut self, prefix: &P, f: impl FnMut(usize, usize));
    /// The leaper at `index` proposes.
    fn propose(&mut self, prefix: &P, index: usize, values: &mut Vec<V>);
    /// Every leaper except the one at `index` intersects.
    fn intersect(&mut self, prefix: &P, index: usize, values: &mut Vec<V>);
}

macro_rules! leapers_tuple {
    ($($name:ident $idx:tt),+) => {
        impl<P, V, $($name: Leaper<P, V>),+> Leapers<P, V> for ($($name,)+) {
            fn for_each_count(&mut self, prefix: &P, mut f: impl FnMut(usize, usize)) {
                $(f($idx, self.$idx.count(prefix));)+
            }
            fn propose(&mut self, prefix: &P, index: usize, values: &mut Vec<V>) {
                match index {
                    $($idx => self.$idx.propose(prefix, values),)+
                    _ => unreachable!("no leaper {index}"),
                }
            }
            fn intersect(&mut self, prefix: &P, index: usize, values: &mut Vec<V>) {
                $(if index != $idx && !values.is_empty() { self.$idx.intersect(prefix, values); })+
            }
        }
    };
}

leapers_tuple!(A 0);
leapers_tuple!(A 0, B 1);
leapers_tuple!(A 0, B 1, C 2);
leapers_tuple!(A 0, B 1, C 2, D 3);
leapers_tuple!(A 0, B 1, C 2, D 3, E 4);

/// For each tuple of `source`, `f(prefix, value)` for every value all
/// `leapers` accept. At least one leaper must propose (count below
/// `usize::MAX`); a prefix where every leaper only filters gets nothing.
pub(crate) fn leapjoin<P, V: Copy>(source: &[P], leapers: &mut impl Leapers<P, V>, mut f: impl FnMut(&P, V)) {
    let mut values = Vec::new();
    for prefix in source {
        let (mut index, mut least) = (usize::MAX, usize::MAX);
        leapers.for_each_count(prefix, |i, count| {
            if count < least {
                least = count;
                index = i;
            }
        });
        if least == 0 || index == usize::MAX {
            continue;
        }
        leapers.propose(prefix, index, &mut values);
        leapers.intersect(prefix, index, &mut values);
        for &v in &values {
            f(prefix, v);
        }
        values.clear();
    }
}

/// Values `v` such that `(key(prefix), v)` is in the relation.
#[derive(Debug)]
pub(crate) struct ExtendWith<'a, K, V, F> {
    parts: Vec<&'a [(K, V)]>,
    ranges: Vec<&'a [(K, V)]>,
    key: F,
}

/// Values `v` such that `(key(prefix), v)` is in the relation.
pub(crate) fn extend_with<'a, P, K, V, F>(view: &View<'a, (K, V)>, key: F) -> ExtendWith<'a, K, V, F>
where
    K: Ord + Copy,
    V: Ord + Copy,
    F: Fn(&P) -> K,
{
    ExtendWith {
        parts: view.parts().to_vec(),
        ranges: Vec::new(),
        key,
    }
}

/// The run of tuples with key `k` in a sorted part.
fn key_range<K: Ord + Copy, V>(part: &[(K, V)], k: K) -> &[(K, V)] {
    let start = part.partition_point(|t| t.0 < k);
    let rest = &part[start..];
    let len = gallop(rest, |t| t.0 <= k);
    &rest[..rest.len() - len.len()]
}

/// Keeps the sorted `values` found (or, with `keep_found` false, not found)
/// as a value in one of `ranges`. Advances the ranges as cursors.
fn retain_in<K, V: Ord + Copy>(ranges: &mut [&[(K, V)]], values: &mut Vec<V>, keep_found: bool) {
    match ranges {
        [] => {
            if keep_found {
                values.clear();
            }
        }
        [one] => {
            let slice = one;
            values.retain(|v| {
                *slice = gallop(slice, |t| t.1 < *v);
                (slice.first().map(|t| &t.1) == Some(v)) == keep_found
            });
        }
        cursors => {
            values.retain(|v| {
                let mut found = false;
                for slice in cursors.iter_mut() {
                    *slice = gallop(slice, |t| t.1 < *v);
                    found |= slice.first().map(|t| &t.1) == Some(v);
                }
                found == keep_found
            });
        }
    }
}

impl<'a, P, K, V, F> Leaper<P, V> for ExtendWith<'a, K, V, F>
where
    K: Ord + Copy,
    V: Ord + Copy,
    F: Fn(&P) -> K,
{
    fn count(&mut self, prefix: &P) -> usize {
        let k = (self.key)(prefix);
        self.ranges.clear();
        let mut total = 0;
        for &part in &self.parts {
            let range = key_range(part, k);
            if !range.is_empty() {
                total += range.len();
                self.ranges.push(range);
            }
        }
        total
    }

    fn propose(&mut self, _prefix: &P, values: &mut Vec<V>) {
        for range in &self.ranges {
            values.extend(range.iter().map(|t| t.1));
        }
        if self.ranges.len() > 1 {
            values.sort_unstable();
            values.dedup();
        }
    }

    fn intersect(&mut self, _prefix: &P, values: &mut Vec<V>) {
        retain_in(&mut self.ranges, values, true);
    }
}

/// Removes values `v` such that `(key(prefix), v)` is in the relation.
#[derive(Debug)]
pub(crate) struct ExtendAnti<'a, K, V, F> {
    parts: Vec<&'a [(K, V)]>,
    ranges: Vec<&'a [(K, V)]>,
    key: F,
}

/// Removes values `v` such that `(key(prefix), v)` is in the relation.
pub(crate) fn extend_anti<'a, P, K, V, F>(view: &View<'a, (K, V)>, key: F) -> ExtendAnti<'a, K, V, F>
where
    K: Ord + Copy,
    V: Ord + Copy,
    F: Fn(&P) -> K,
{
    ExtendAnti {
        parts: view.parts().to_vec(),
        ranges: Vec::new(),
        key,
    }
}

impl<'a, P, K, V, F> Leaper<P, V> for ExtendAnti<'a, K, V, F>
where
    K: Ord + Copy,
    V: Ord + Copy,
    F: Fn(&P) -> K,
{
    fn count(&mut self, _prefix: &P) -> usize {
        usize::MAX
    }

    fn propose(&mut self, _prefix: &P, _values: &mut Vec<V>) {
        unreachable!("ExtendAnti only filters")
    }

    fn intersect(&mut self, prefix: &P, values: &mut Vec<V>) {
        let k = (self.key)(prefix);
        self.ranges.clear();
        self.ranges
            .extend(self.parts.iter().map(|&p| key_range(p, k)).filter(|r| !r.is_empty()));
        if !self.ranges.is_empty() {
            retain_in(&mut self.ranges, values, false);
        }
    }
}

/// Keeps a prefix only when `tuple(prefix)` is in the relation.
#[derive(Debug)]
pub(crate) struct FilterWith<'a, T, F> {
    view: View<'a, T>,
    tuple: F,
}

/// Keeps a prefix only when `tuple(prefix)` is in the relation.
pub(crate) fn filter_with<'a, P, T, F>(view: &View<'a, T>, tuple: F) -> FilterWith<'a, T, F>
where
    T: Ord + Copy,
    F: Fn(&P) -> T,
{
    FilterWith {
        view: view.clone(),
        tuple,
    }
}

impl<'a, P, V, T, F> Leaper<P, V> for FilterWith<'a, T, F>
where
    T: Ord + Copy,
    F: Fn(&P) -> T,
{
    fn count(&mut self, prefix: &P) -> usize {
        if self.view.contains(&(self.tuple)(prefix)) {
            usize::MAX
        } else {
            0
        }
    }
    fn propose(&mut self, _prefix: &P, _values: &mut Vec<V>) {
        unreachable!("FilterWith only filters")
    }
    fn intersect(&mut self, _prefix: &P, _values: &mut Vec<V>) {}
}

/// Drops a prefix when `tuple(prefix)` is in the relation.
#[derive(Debug)]
pub(crate) struct FilterAnti<'a, T, F> {
    view: View<'a, T>,
    tuple: F,
}

/// Drops a prefix when `tuple(prefix)` is in the relation.
pub(crate) fn filter_anti<'a, P, T, F>(view: &View<'a, T>, tuple: F) -> FilterAnti<'a, T, F>
where
    T: Ord + Copy,
    F: Fn(&P) -> T,
{
    FilterAnti {
        view: view.clone(),
        tuple,
    }
}

impl<'a, P, V, T, F> Leaper<P, V> for FilterAnti<'a, T, F>
where
    T: Ord + Copy,
    F: Fn(&P) -> T,
{
    fn count(&mut self, prefix: &P) -> usize {
        if self.view.contains(&(self.tuple)(prefix)) {
            0
        } else {
            usize::MAX
        }
    }
    fn propose(&mut self, _prefix: &P, _values: &mut Vec<V>) {
        unreachable!("FilterAnti only filters")
    }
    fn intersect(&mut self, _prefix: &P, _values: &mut Vec<V>) {}
}

/// Keeps the values `keep(prefix, value)` accepts.
#[derive(Debug)]
pub(crate) struct ValueFilter<F> {
    keep: F,
}

/// Keeps the values `keep(prefix, value)` accepts.
pub(crate) fn value_filter<P, V, F: Fn(&P, &V) -> bool>(keep: F) -> ValueFilter<F> {
    ValueFilter { keep }
}

impl<P, V, F: Fn(&P, &V) -> bool> Leaper<P, V> for ValueFilter<F> {
    fn count(&mut self, _prefix: &P) -> usize {
        usize::MAX
    }
    fn propose(&mut self, _prefix: &P, _values: &mut Vec<V>) {
        unreachable!("ValueFilter only filters")
    }
    fn intersect(&mut self, prefix: &P, values: &mut Vec<V>) {
        values.retain(|v| (self.keep)(prefix, v));
    }
}
