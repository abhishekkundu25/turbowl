//! `Relation`: a sorted, duplicate-free `Vec` of tuples, and `gallop`, the
//! exponential search every join and merge here uses to skip ahead.
//!
//! `gallop` is adapted from datafrog 2.0.1 (`src/join.rs`). Copyright (c)
//! Frank McSherry, The Rust Project Developers and the Datafrog Developers;
//! used under the MIT license (see `THIRD-PARTY-NOTICES.md` at the repository
//! root).
//!
//! A relation holds little spare room ([`fit`]): a store keeps its tuples
//! for the whole run, and room left over from building them would be kept
//! with them.

use std::cmp::Ordering;
use std::ops::Deref;

/// A sorted, duplicate-free set of tuples. Built by `sort_unstable` and
/// `dedup`, or by merging two relations; never edited in place otherwise.
/// Both leave at most an eighth of its length in spare room ([`fit`]);
/// `from_sorted` keeps what its caller gives.
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

    /// Sorts and deduplicates `elements`, keeping its allocation unless
    /// duplicates or growth left it too large ([`fit`]).
    pub(crate) fn from_vec(mut elements: Vec<T>) -> Self {
        elements.sort_unstable();
        elements.dedup();
        fit(&mut elements);
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
    /// large one at most once. The allocation grows by exactly what the
    /// merge needs, and gives back the room of the duplicates ([`fit`]).
    pub(crate) fn merge(self, other: Self) -> Self {
        let (big, small) = if self.elements.capacity() >= other.elements.capacity() {
            (self.elements, other.elements)
        } else {
            (other.elements, self.elements)
        };
        if small.is_empty() {
            let mut elements = big;
            fit(&mut elements);
            return Self { elements };
        }
        Self {
            elements: merge_into(big, &small),
        }
    }

    /// True when `x` is in the relation.
    pub(crate) fn contains(&self, x: &T) -> bool {
        self.elements.binary_search(x).is_ok()
    }

    /// The bytes the relation holds: its capacity, not its length.
    pub(crate) fn bytes(&self) -> usize {
        self.elements.capacity() * std::mem::size_of::<T>()
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

/// Gives back the spare room of `elements` when it is more than an eighth
/// of its length; an empty vector keeps none.
///
/// Why an eighth: a relation then holds at most 9/8 of its tuples' bytes,
/// so a store sized by its contents stays within an eighth of them. A
/// smaller bound would reallocate for slack that hardly counts. The
/// callers give room back only after work that already reads every tuple
/// (a sort, a merge, a filter), so the copy a shrink may cost is paid by
/// that work, whatever the bound.
pub(crate) fn fit<T>(elements: &mut Vec<T>) {
    if elements.capacity() - elements.len() > elements.len() / 8 {
        elements.shrink_to_fit();
    }
}

/// Merges sorted, duplicate-free `b` into sorted, duplicate-free `a`, keeping
/// `a`'s allocation. Writes from the back; a tuple in both is written once,
/// and the gap the duplicates leave is closed with one move at the end.
/// `a` grows by exactly `b`'s length, not by doubling: a stable batch is
/// merged into rarely and kept, so room for later pushes would only be
/// held.
fn merge_into<T: Ord + Copy>(mut a: Vec<T>, b: &[T]) -> Vec<T> {
    let (mut i, mut j) = (a.len(), b.len());
    a.reserve_exact(b.len());
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
    fit(&mut a);
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
