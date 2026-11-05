//! The public `Reasoner` (ticket 10 implements it; this is the interface).

use crate::{Diagnostic, Error};
use oxrdf::{Triple, TripleRef};

/// A document format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Turtle,
    NTriples,
    N3,
    RdfXml,
}

impl Format {
    /// The format of a file name's extension (`ttl`, `nt`, `n3`, `rdf`, `owl`, `xml`).
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "ttl" | "turtle" => Some(Self::Turtle),
            "nt" => Some(Self::NTriples),
            "n3" => Some(Self::N3),
            "rdf" | "owl" | "xml" => Some(Self::RdfXml),
            _ => None,
        }
    }
}

/// What a run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Summary {
    /// Base triples.
    pub base: usize,
    /// Inferred triples (not base), after expanding `owl:sameAs` representatives.
    pub inferred: usize,
    /// Evaluation rounds this run took.
    pub rounds: usize,
    /// Diagnostics after the run.
    pub diagnostics: usize,
}

/// An OWL 2 RL reasoner over an in-memory graph. `Send`; not `Sync`.
#[derive(Debug, Default)]
pub struct Reasoner {}

impl Reasoner {
    /// An empty reasoner.
    pub fn new() -> Self {
        Self {}
    }

    /// Adds a file's triples to the base (format from its extension).
    #[cfg(feature = "io")]
    pub fn load_file(&mut self, path: &std::path::Path) -> Result<(), Error> {
        let _ = path;
        unimplemented!("ticket 10")
    }

    /// Adds a document's triples to the base. Blank nodes are scoped to this document.
    #[cfg(feature = "io")]
    pub fn load_str(&mut self, text: &str, format: Format, base_iri: Option<&str>) -> Result<(), Error> {
        let _ = (text, format, base_iri);
        unimplemented!("ticket 10")
    }

    /// Adds triples to the base.
    pub fn insert<'a>(&mut self, triples: impl IntoIterator<Item = TripleRef<'a>>) {
        let _ = triples.into_iter();
        unimplemented!("ticket 10")
    }

    /// Removes triples from the base.
    pub fn remove<'a>(&mut self, triples: impl IntoIterator<Item = TripleRef<'a>>) {
        let _ = triples.into_iter();
        unimplemented!("ticket 10")
    }

    /// Makes the base exactly `triples`. True when a base triple was removed;
    /// an unchanged base costs nothing.
    pub fn set_base<'a>(&mut self, triples: impl IntoIterator<Item = TripleRef<'a>>) -> bool {
        let _ = triples.into_iter();
        unimplemented!("ticket 10")
    }

    /// Empties the base, the closure and the diagnostics.
    pub fn clear(&mut self) {
        unimplemented!("ticket 10")
    }

    /// Runs to the fixpoint: in full, or over what changed since the last run.
    pub fn reason(&mut self) -> Summary {
        unimplemented!("ticket 10")
    }

    /// The closure: base and inferred triples.
    pub fn triples(&self) -> impl Iterator<Item = Triple> + '_ {
        unimplemented!("ticket 10");
        #[allow(unreachable_code)]
        std::iter::empty()
    }

    /// Only the inferred triples.
    pub fn inferred(&self) -> impl Iterator<Item = Triple> + '_ {
        unimplemented!("ticket 10");
        #[allow(unreachable_code)]
        std::iter::empty()
    }

    /// The diagnostics of the last run, deduplicated on (code, terms).
    pub fn diagnostics(&self) -> &[Diagnostic] {
        unimplemented!("ticket 10")
    }

    /// Writes the closure (or only the inferred triples) as `format`.
    #[cfg(feature = "io")]
    pub fn write(&self, out: impl std::io::Write, format: Format, inferred_only: bool) -> Result<(), Error> {
        let _ = (out, format, inferred_only);
        unimplemented!("ticket 10")
    }
}
