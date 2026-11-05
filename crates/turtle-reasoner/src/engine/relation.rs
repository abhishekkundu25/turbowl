//! `Relation`: a sorted, duplicate-free `Vec` of tuples, and `gallop`, the
//! exponential search every join and merge here uses to skip ahead.

use std::cmp::Ordering;
use std::ops::Deref;

/// A sorted, duplicate-free set of tuples. Built by `sort_unstable` and
/// `dedup`, or by merging two relations; never edited in place otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Relation<T> {
    elements: Vec<T>,
}

impl<T> Default for Relation<T> {
    fn default() -> Self {
        Self { elements: Vec::new() }
    }
}

impl<T: Ord + Copy> Relation<T> {
    /// An empty relation.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Sorts and deduplicates `elements`, keeping its allocation.
    pub(crate) fn from_vec(mut elements: Vec<T>) -> Self {
        elements.sort_unstable();
        elements.dedup();
        Self { elements }
    }

    /// Wraps a vector that is already strictly increasing (checked in debug builds).
    pub(crate) fn from_sorted(elements: Vec<T>) -> Self {
        debug_assert!(
            elements.windows(2).all(|w| w[0] < w[1]),
            "from_sorted: not strictly increasing"
        );
        Self { elements }
    }

    /// The union of two relations. Merges into the larger allocation, from the
    /// back, so the common case (a small batch into a large one) moves the
    /// large one at most once.
    pub(crate) fn merge(self, other: Self) -> Self {
        let (big, small) = if self.elements.capacity() >= other.elements.capacity() {
            (self.elements, other.elements)
        } else {
            (other.elements, self.elements)
        };
        if small.is_empty() {
            return Self { elements: big };
        }
        Self {
            elements: merge_into(big, &small),
        }
    }

    /// True when `x` is in the relation.
    pub(crate) fn contains(&self, x: &T) -> bool {
        self.elements.binary_search(x).is_ok()
    }

    /// The tuples, as a vector.
    pub(crate) fn into_vec(self) -> Vec<T> {
        self.elements
    }
}

impl<T> Deref for Relation<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.elements
    }
}

impl<T: Ord + Copy> FromIterator<T> for Relation<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_vec(iter.into_iter().collect())
    }
}

/// Merges sorted, duplicate-free `b` into sorted, duplicate-free `a`, keeping
/// `a`'s allocation. Writes from the back; a tuple in both is written once,
/// and the gap the duplicates leave is closed with one move at the end.
fn merge_into<T: Ord + Copy>(mut a: Vec<T>, b: &[T]) -> Vec<T> {
    let (mut i, mut j) = (a.len(), b.len());
    a.extend_from_slice(b);
    let mut k = a.len();
    while i > 0 && j > 0 {
        let (x, y) = (a[i - 1], b[j - 1]);
        k -= 1;
        match x.cmp(&y) {
            Ordering::Greater => {
                a[k] = x;
                i -= 1;
            }
            Ordering::Less => {
                a[k] = y;
                j -= 1;
            }
            Ordering::Equal => {
                a[k] = x;
                i -= 1;
                j -= 1;
            }
        }
    }
    // What is left is a prefix of one input, already sorted: `a[..i]` is in
    // place; `b[..j]` goes to the front. Either way the gap is `filled..k`.
    let filled = if j > 0 {
        a[..j].copy_from_slice(&b[..j]);
        j
    } else {
        i
    };
    if k > filled {
        a.drain(filled..k);
    }
    a
}

/// Skips the prefix of `slice` whose elements satisfy `before` (which must be
/// true on a prefix and false after it), by doubling then halving steps:
/// O(log d) for a skip of d elements, so a short skip stays cheap.
pub(crate) fn gallop<T>(mut slice: &[T], mut before: impl FnMut(&T) -> bool) -> &[T] {
    if slice.first().is_some_and(&mut before) {
        let mut step = 1;
        while step < slice.len() && before(&slice[step]) {
            slice = &slice[step..];
            step <<= 1;
        }
        step >>= 1;
        while step > 0 {
            if step < slice.len() && before(&slice[step]) {
                slice = &slice[step..];
            }
            step >>= 1;
        }
        slice = &slice[1..];
    }
    slice
}
