//! Term ids and the dictionary (ticket 02).

/// A term's id in the dictionary. Relations and rules work only on these.
#[allow(dead_code)] // used from ticket 02 on; remove the allow then
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TermId(pub(crate) u32);
