//! The public `Reasoner`: the base triples, the closure of the last run,
//! and the run loop over the rule table.
//!
//! # State
//!
//! - **The dictionary**: every term seen, as a term id. Terms are never
//!   removed, so a long session of edits keeps the terms it no longer uses
//!   ([`Reasoner::clear`] starts a new one).
//! - **The base**: the triples loaded or inserted, deduplicated, by
//!   predicate ([`Base`]): each predicate's sorted `(s, o)` term ids, as the
//!   store's tables hold them, plus a hash set of the `[s, p, o]` triples
//!   added since the last merge, merged in once it passes an eighth of the
//!   pairs. One addition costs two binary searches and a hash insert, not a
//!   copy of the base. 8 bytes a triple (10 with its vector's spare room at
//!   most), plus the hash set: at most an eighth as many entries, about 15
//!   to 30 bytes each. A full run copies each predicate's pairs into its
//!   table.
//! - **The closure**: an engine [`Store`] holding the base, the families'
//!   facts and every derived triple, generalized triples included (a
//!   literal subject, a predicate that is not an IRI; `docs/specs/v1.md`,
//!   Implementation Decisions). The reads leave the generalized ones out.
//! - **Equality** (ADR 0003): the store's canon says which terms
//!   `owl:sameAs` made equal, and the store holds each triple once, in
//!   representatives. Every read expands a stored triple to each member of
//!   each of its terms' classes (`rules/equality.rs`), which gives the
//!   closure of the W3C eq-* rules, `owl:sameAs` triples among the members
//!   included.
//! - **eq-ref** (switch `trivial`), an output rule: `x owl:sameAs x` for
//!   every term of the closure is added when it is read. A class of two or
//!   more already holds its members' reflexive pairs, so the reads add
//!   `r owl:sameAs r` only for the representatives `rules/trivial.rs` lists
//!   after each run ([`trivial::same_as_itself`]), each expanded like a
//!   stored triple. The store holds eq-ref only once a rule could read it
//!   (a schema that names `owl:sameAs` as a property; `rules/trivial.rs`).
//! - **The count of RDF triples** in the expanded closure, so that
//!   [`Summary::inferred`] needs no pass over it: a run adds what each
//!   round's deltas stand for, and a run that merged a class counts the
//!   store again, since a merge rewrites stored triples (a tuple is in a
//!   delta again, [`Variable::remap`](crate::engine::Variable::remap)) and
//!   changes what the unchanged ones stand for. eq-ref's triples are
//!   counted apart, after each run.
//! - **The findings**, in representatives (dt-not-type's name the literal
//!   itself), sorted and deduplicated after each run, and their public form,
//!   the [`Diagnostic`]s, each term expanded to every member of its class.
//!
//! # Runs
//!
//! [`Reasoner::reason`] does one of three things:
//!
//! - nothing, when the base has not changed since the last run (0 rounds);
//! - after additions only, an incremental run: the new base triples were
//!   queued into the stored closure as they came, and [`Store::run`] joins
//!   them semi-naively against it; the findings of earlier runs stay;
//! - after any removal (and on the first run, after [`Reasoner::clear`],
//!   and after a change of switches), a full run: a fresh store with no
//!   terms equal, the facts of every family that is on (inferred, never
//!   base; an empty base gets them too), then the base.
//!
//! A merge that changes a rule constant (two vocabulary terms made equal,
//! or `rdf:nil` given a class of its own) stops the run, and it starts
//! again from the base with the classes found so far (a *restart*: ADR
//! 0003, `engine/store.rs`). Everything found before the restart follows
//! from the base, so its findings are kept.
//!
//! # Switches
//!
//! The reasoner holds a [`Switches`] configuration, the default unless the
//! caller gives another. A run uses the families of the rule table that the
//! switches leave on ([`Family::is_on`]): the others are not called, and
//! their facts are not inserted. A change of switches makes the next run a
//! full one, so the closure is always the one a fresh reasoner with those
//! switches would give.
//!
//! Each round calls every family that is on, in table order, as
//! `rules/testing.rs` does, and [`Counters`] records the round's delta,
//! what each family emitted and its time, and the store's steps
//! ([`Reasoner::profile`] reads them, with the bytes held).
//!
//! # Threads (ticket 17)
//!
//! The reasoner holds a [`Threads`] setting, [`Threads::ALL`] unless the
//! caller sets another, and hands the store its [`Workers`]. In a round
//! whose delta passes the threshold (`engine/parallel.rs`), each family is
//! a task with its own buffer and findings ([`Round::tasks`]), and the
//! count of the round's delta splits each large recent part into chunks.
//! The buffers, the findings and the counters join in family order, so a
//! run gives the same store, findings, rounds and counters on any number
//! of threads. A change of setting takes effect at the next run, and never
//! makes it a full one.
//!
//! Between a change and the next `reason()`, the reads ([`Reasoner::triples`],
//! [`Reasoner::inferred`], [`Reasoner::diagnostics`], [`Reasoner::write`])
//! show the closure of the last run, told apart from the base as it now is.
//!
//! # Blank nodes
//!
//! Each loaded document has its own blank nodes, written `<label>_<n>` for
//! the n-th document loaded. Inserted blank nodes belong to no document and
//! keep their label, so `insert` then `triples()` gives the labels back; a
//! label that ends in `_` and digits gets `_0` appended, so that it never
//! takes a document node's label (`Dictionary::write_blank_label`).
//! [`Reasoner::remove`] and [`Reasoner::set_base`] name blank nodes as
//! `insert` does: they reach inserted blank nodes, not a document's.
//!
//! Loading the same document again (the same bytes, format and base IRI)
//! changes nothing while the triples it added are still in the base. Its
//! blank nodes would otherwise be new nodes, and the parsers label
//! anonymous ones (`[ ]`, lists) at random on every parse. A removal of any
//! base triple forgets which documents were loaded: loading one again after
//! a removal adds it again.
//!
//! # Snapshots (ticket 19)
//!
//! [`Reasoner::write_snapshot`] writes the whole state above (but the
//! threads and the counters) as one checked byte string, and
//! [`Reasoner::from_snapshot_bytes`] makes the same reasoner from it, with
//! the same run due next; [`Reasoner::save_snapshot`] and
//! [`Reasoner::from_snapshot`] do it with a file, atomically. The format
//! and what loading checks are in `snapshot.rs`.

use crate::Error;
use crate::engine::{Canon, Round, Store, Workers};
use crate::profile::{self, Clock, Memory, Parts, Profile, Step};
use crate::rules::equality::{self, Weights};
use crate::rules::{Context, FAMILIES, Family, Finding, datatype, trivial};
use crate::snapshot::{self, Restored, Saved};
use crate::term::{Dictionary, DocId, Kind, TermId, vocab};
use crate::{Diagnostic, Switch, Switches, Threads};
use oxrdf::{NamedNode, NamedOrBlankNode, Triple, TripleRef};
use rustc_hash::FxHashSet;
use std::borrow::Cow;

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
    /// Inferred triples (not base), after expanding `owl:sameAs`
    /// representatives: the RDF triples [`Reasoner::inferred`] gives.
    pub inferred: usize,
    /// Evaluation rounds this run took: 0 when nothing changed since the last run.
    pub rounds: usize,
    /// Diagnostics after the run.
    pub diagnostics: usize,
}

/// An OWL 2 RL reasoner over an in-memory graph. `Send`.
///
/// Change the base ([`Reasoner::load_str`], [`Reasoner::insert`],
/// [`Reasoner::remove`], [`Reasoner::set_base`]), then [`Reasoner::reason`];
/// the reads show the closure of the last run.
#[derive(Debug)]
pub struct Reasoner {
    /// The rule table: [`FAMILIES`], or a test's own.
    families: &'static [Family],
    /// Which of the table's families run.
    switches: Switches,
    /// The threads a run may use, and where they are.
    threads: Threads,
    workers: Workers,
    terms: Dictionary,
    base: Base,
    /// The closure of the last run, plus the base triples added since,
    /// queued for the next run (unless that run is a full one).
    closure: Store,
    state: State,
    /// The RDF triples of the stored closure once expanded (base and
    /// inferred).
    rdf: usize,
    /// eq-ref on read (switch `trivial`): the representatives whose
    /// `r owl:sameAs r` the reads add ([`trivial::same_as_itself`]), and
    /// the RDF triples they stand for once expanded (all inferred).
    same_as_itself: Vec<u32>,
    same_as_itself_rdf: usize,
    /// The findings of the closure, in representatives, sorted and
    /// deduplicated.
    findings: Vec<Finding>,
    /// The public form of `findings`, expanded to every member of each
    /// term's class, sorted.
    diagnostics: Vec<Diagnostic>,
    counters: Counters,
    /// The documents loaded since the last removal ([`document_key`]).
    #[cfg(feature = "io")]
    loaded: Documents,
}

// The interface promises `Send`: checked here, at compile time.
const _: () = {
    const fn send<T: Send>() {}
    send::<Reasoner>()
};

/// What the next [`Reasoner::reason`] has to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    /// Nothing: the closure is the base's.
    Current,
    /// Base triples were added (and queued in the closure): an incremental run.
    Added,
    /// A run from the base: first, after a removal, after `clear`, or after
    /// a change of switches.
    Full,
}

impl Default for Reasoner {
    fn default() -> Self {
        Self::new()
    }
}

impl Reasoner {
    /// An empty reasoner with the default switches ([`Switches::default`]).
    pub fn new() -> Self {
        Self::with_switches(Switches::default())
    }

    /// An empty reasoner with these switches.
    pub fn with_switches(switches: Switches) -> Self {
        Self::build(FAMILIES, switches)
    }

    /// An empty reasoner over another rule table, with the default switches
    /// (tests use toy families).
    #[cfg(test)]
    pub(crate) fn with_families(families: &'static [Family]) -> Self {
        Self::build(families, Switches::default())
    }

    fn build(families: &'static [Family], switches: Switches) -> Self {
        let terms = new_dictionary();
        let threads = Threads::default();
        let workers = threads.workers();
        let mut closure = Store::new();
        closure.set_workers(workers.clone());
        Self {
            families,
            switches,
            threads,
            workers,
            terms,
            base: Base::default(),
            closure,
            state: State::Full,
            rdf: 0,
            same_as_itself: Vec::new(),
            same_as_itself_rdf: 0,
            findings: Vec::new(),
            diagnostics: Vec::new(),
            counters: Counters::default(),
            #[cfg(feature = "io")]
            loaded: Documents::default(),
        }
    }

    /// Adds a file's triples to the base (format from its extension). See
    /// [`Reasoner::load_str`].
    ///
    /// An N-Triples or Turtle file is read in windows of 32 MiB, never whole
    /// (`io::WINDOW`). It is keyed as it is read, so that a second load of
    /// it is a no-op; when a document of its length was loaded already, it
    /// is read once first to be keyed (and then not parsed if it is that
    /// document).
    #[cfg(feature = "io")]
    pub fn load_file(&mut self, path: &std::path::Path) -> Result<(), Error> {
        use std::io::{Read, Seek};
        let format = crate::io::format_of(path)?;
        let failed = |source| Error::Io {
            path: Some(path.into()),
            source,
        };
        let mut file = std::fs::File::open(path).map_err(failed)?;
        let metadata = file.metadata().map_err(failed)?;
        if !metadata.is_file() {
            // A pipe or a device reads once, and cannot be read again for
            // an error's line: read it whole.
            let mut text = Vec::new();
            file.read_to_end(&mut text).map_err(failed)?;
            return self.load(&text, format, None, Some(path));
        }
        if self.loaded.may_hold(metadata.len()) {
            let key = DocumentKey::read(&mut file, format, None).map_err(failed)?;
            if self.loaded.contains(key) {
                return Ok(());
            }
            file.rewind().map_err(failed)?;
        }
        let mut key = DocumentKey::new(format, None);
        let mut triples = Vec::new();
        crate::io::parse_reader_on(
            &self.workers,
            &mut self.terms,
            &mut file,
            format,
            None,
            Some(path),
            crate::io::WINDOW,
            |bytes| key.feed(bytes),
            |[s, p, o]| triples.push([s.0, p.0, o.0]),
        )?;
        let (key, len) = key.finish();
        // Only if the file changed after it was keyed above.
        if self.loaded.contains(key) {
            return Ok(());
        }
        self.add(triples);
        self.loaded.insert(key, len);
        Ok(())
    }

    /// Adds a document's triples to the base. Blank nodes are scoped to this
    /// document. All or nothing: on an error the base is unchanged. Loading
    /// the same document again changes nothing, unless a base triple was
    /// removed since (see the module doc).
    #[cfg(feature = "io")]
    pub fn load_str(&mut self, text: &str, format: Format, base_iri: Option<&str>) -> Result<(), Error> {
        self.load(text.as_bytes(), format, base_iri, None)
    }

    #[cfg(feature = "io")]
    fn load(
        &mut self,
        text: &[u8],
        format: Format,
        base_iri: Option<&str>,
        path: Option<&std::path::Path>,
    ) -> Result<(), Error> {
        let key = document_key(text, format, base_iri);
        if self.loaded.contains(key) {
            return Ok(());
        }
        let mut triples = Vec::new();
        crate::io::parse_on(
            &self.workers,
            &mut self.terms,
            text,
            format,
            base_iri,
            path,
            |[s, p, o]| {
                triples.push([s.0, p.0, o.0]);
            },
        )?;
        self.add(triples);
        self.loaded.insert(key, text.len() as u64);
        Ok(())
    }

    /// Adds triples to the base. Blank nodes belong to no document: `_:x`
    /// here is `_:x` in every call, and keeps its label on output.
    pub fn insert<'a>(&mut self, triples: impl IntoIterator<Item = TripleRef<'a>>) {
        let batch: Vec<[u32; 3]> = triples.into_iter().map(|t| self.intern(t)).collect();
        self.add(batch);
    }

    /// Removes triples from the base (blank nodes named as by `insert`).
    /// Triples that are not base triples are ignored; when one was removed,
    /// the next run is a full one.
    pub fn remove<'a>(&mut self, triples: impl IntoIterator<Item = TripleRef<'a>>) {
        let gone: Vec<[u32; 3]> = triples.into_iter().filter_map(|t| self.lookup(t)).collect();
        if self.base.remove(gone) > 0 {
            self.restart();
        }
    }

    /// Makes the base exactly `triples` (blank nodes named as by `insert`).
    /// True when a base triple was removed, so the next run is a full one.
    /// An unchanged base costs nothing: the next run takes no rounds; a base
    /// that only grew runs incrementally.
    pub fn set_base<'a>(&mut self, triples: impl IntoIterator<Item = TripleRef<'a>>) -> bool {
        let new: Vec<[u32; 3]> = triples.into_iter().map(|t| self.intern(t)).collect();
        let new = ByPredicate::from_triples(new);
        let old = self.base.flush();
        if new == *old {
            return false;
        }
        match added(old, &new) {
            Some(more) => {
                self.queue(&more);
                self.base.replace(new);
                false
            }
            None => {
                self.base.replace(new);
                self.restart();
                true
            }
        }
    }

    /// The switches the runs use.
    pub fn switches(&self) -> Switches {
        self.switches
    }

    /// Sets the switches the next runs use. A change makes the next run a
    /// full one: its closure and diagnostics are those a fresh reasoner with
    /// these switches gives. Setting the same switches again changes
    /// nothing. Until the next run, the reads show the last run's closure.
    pub fn set_switches(&mut self, switches: Switches) {
        if switches != self.switches {
            self.switches = switches;
            self.state = State::Full;
        }
    }

    /// The threads the runs may use.
    pub fn threads(&self) -> Threads {
        self.threads
    }

    /// Sets the threads the next runs may use ([`Threads`]). The closure,
    /// the diagnostics and the order of every read do not depend on it, so
    /// a change never makes the next run a full one.
    pub fn set_threads(&mut self, threads: Threads) {
        if threads != self.threads {
            self.threads = threads;
            self.workers = threads.workers();
            self.closure.set_workers(self.workers.clone());
        }
    }

    /// Runs on `workers`, which [`Threads`] cannot all describe (a test
    /// that measures the threshold).
    #[cfg(test)]
    pub(crate) fn set_workers(&mut self, workers: Workers) {
        self.closure.set_workers(workers.clone());
        self.workers = workers;
    }

    /// Empties the base, the closure and the diagnostics (and the
    /// dictionary): as new, with the same switches and threads.
    pub fn clear(&mut self) {
        let (threads, workers) = (self.threads, self.workers.clone());
        *self = Self::build(self.families, self.switches);
        self.threads = threads;
        self.closure.set_workers(workers.clone());
        self.workers = workers;
    }

    /// Runs to the fixpoint: in full, or over what changed since the last run.
    pub fn reason(&mut self) -> Summary {
        let rounds = match self.state {
            State::Current => {
                self.counters.clear();
                0
            }
            State::Added => self.run(false),
            State::Full => self.run(true),
        };
        self.state = State::Current;
        let base = self.base.len();
        debug_assert!(self.rdf >= base, "the closure holds the base");
        Summary {
            base,
            inferred: self.rdf.saturating_sub(base) + self.same_as_itself_rdf,
            rounds,
            diagnostics: self.diagnostics.len(),
        }
    }

    /// The closure: base and inferred triples. Generalized triples (a
    /// literal subject, a predicate that is not an IRI) are left out. Each
    /// term made equal to others by `owl:sameAs` stands for all of them:
    /// a triple about one holds for every one, and they are `owl:sameAs`
    /// each other.
    pub fn triples(&self) -> impl Iterator<Item = Triple> + '_ {
        self.expanded().filter_map(|(s, p, o)| self.rdf_triple(s, p, o))
    }

    /// Only the inferred triples (the closure's triples that are not base).
    pub fn inferred(&self) -> impl Iterator<Item = Triple> + '_ {
        self.expanded()
            .filter(|&(s, p, o)| !self.base.contains(&[s, p, o]))
            .filter_map(|(s, p, o)| self.rdf_triple(s, p, o))
    }

    /// The diagnostics of the last run, deduplicated on (code, terms).
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Writes the closure (or only the inferred triples) as `format`,
    /// sorted by subject so that Turtle groups each subject's triples.
    #[cfg(feature = "io")]
    pub fn write(&self, out: impl std::io::Write, format: Format, inferred_only: bool) -> Result<(), Error> {
        let mut triples: Vec<[TermId; 3]> = self
            .expanded()
            .filter(|&(s, p, _)| self.is_rdf(s, p))
            .filter(|&(s, p, o)| !inferred_only || !self.base.contains(&[s, p, o]))
            .map(|(s, p, o)| [TermId(s), TermId(p), TermId(o)])
            .collect();
        triples.sort_unstable();
        crate::io::write(&self.terms, triples, out, format).map(|_| ())
    }

    /// Writes a snapshot of the reasoner to `out`: its whole state (the
    /// terms, the base, the closure of the last run as stored, the
    /// diagnostics, the switches, the documents loaded, and any change made
    /// since the last run), so that [`Reasoner::from_snapshot_bytes`] gives
    /// it back without parsing or reasoning. Only the threads are not
    /// saved. The format is versioned and checksummed (`snapshot.rs`
    /// documents it); its size is about 8 bytes a stored pair or a base
    /// triple, plus the terms' text.
    ///
    /// For WebAssembly, which has no files, and the oracle crate's round
    /// trips; [`Reasoner::save_snapshot`] writes a file.
    pub fn write_snapshot(&self, out: impl std::io::Write) -> Result<(), Error> {
        snapshot::write(&self.saved(), out).map_err(|source| Error::Io { path: None, source })
    }

    /// The reasoner a snapshot holds ([`Reasoner::write_snapshot`]): the
    /// saved one, with the same closure, diagnostics and switches, and the
    /// same run due next, so that [`Reasoner::reason`] and the changes then
    /// do what they would have done on it. It runs on [`Threads::ALL`].
    ///
    /// # Errors
    /// [`Error::Snapshot`] when `bytes` are not a snapshot this TurbOWL
    /// reads: another format version, another TurbOWL version (its closure
    /// follows from that version's rules), a truncated or damaged snapshot
    /// (the checksum), or malformed contents. Every part is checked before
    /// it is used.
    pub fn from_snapshot_bytes(bytes: &[u8]) -> Result<Self, Error> {
        Self::read_snapshot(bytes, None)
    }

    /// Saves a snapshot ([`Reasoner::write_snapshot`]) to the file at
    /// `path`, atomically: it is written to a temporary file in the same
    /// directory, synced, and renamed over `path`, so that a crash leaves
    /// the old file or the new one, never half a snapshot.
    #[cfg(feature = "io")]
    pub fn save_snapshot(&self, path: &std::path::Path) -> Result<(), Error> {
        snapshot::save(path, |file| snapshot::write(&self.saved(), file)).map_err(|source| Error::Io {
            path: Some(path.into()),
            source,
        })
    }

    /// The reasoner the snapshot file at `path` holds
    /// ([`Reasoner::from_snapshot_bytes`]).
    ///
    /// # Errors
    /// [`Error::Io`] when the file cannot be read; [`Error::Snapshot`] as
    /// for [`Reasoner::from_snapshot_bytes`].
    #[cfg(feature = "io")]
    pub fn from_snapshot(path: &std::path::Path) -> Result<Self, Error> {
        let bytes = std::fs::read(path).map_err(|source| Error::Io {
            path: Some(path.into()),
            source,
        })?;
        Self::read_snapshot(&bytes, Some(path))
    }

    fn read_snapshot(bytes: &[u8], path: Option<&std::path::Path>) -> Result<Self, Error> {
        let restored = snapshot::read(bytes, new_dictionary).map_err(|message| Error::Snapshot {
            path: path.map(Into::into),
            message,
        })?;
        Ok(Self::restored(restored))
    }

    /// The state a snapshot saves, borrowed.
    fn saved(&self) -> Saved<'_> {
        #[cfg(feature = "io")]
        let (probe, documents) = {
            let mut keys: Vec<u128> = self.loaded.keys.iter().copied().collect();
            keys.sort_unstable();
            (probe_key(), keys)
        };
        #[cfg(not(feature = "io"))]
        let (probe, documents) = (0, Vec::new());
        Saved {
            switches: self.switches,
            state: self.state,
            terms: &self.terms,
            base: self.base.sorted_all(),
            closure: &self.closure,
            rdf: self.rdf,
            same_as_itself: &self.same_as_itself,
            same_as_itself_rdf: self.same_as_itself_rdf,
            findings: &self.findings,
            probe,
            documents,
        }
    }

    /// A reasoner in the state a snapshot held, on the default threads.
    fn restored(r: Restored) -> Self {
        let mut out = Self::build(FAMILIES, r.switches);
        out.diagnostics = diagnostics(&r.findings, r.closure.canon(), &r.terms);
        out.terms = r.terms;
        out.base = Base::restored(r.base);
        out.closure = r.closure;
        out.closure.set_workers(out.workers.clone());
        out.state = r.state;
        out.rdf = r.rdf;
        out.same_as_itself = r.same_as_itself;
        out.same_as_itself_rdf = r.same_as_itself_rdf;
        out.findings = r.findings;
        // Keys from a build that hashes documents another way would name
        // no document here: forget them, as a removal does.
        #[cfg(feature = "io")]
        if r.probe == probe_key() {
            out.loaded = Documents::restored(r.documents);
        }
        // Without `io` no document is loaded, and the keys name none.
        #[cfg(not(feature = "io"))]
        let _ = (r.probe, r.documents);
        out
    }

    /// Every triple of the closure, generalized ones included, each stored
    /// triple expanded to the members of its terms' classes; then eq-ref's
    /// (none unless `trivial` is on), expanded the same way.
    fn expanded(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        let canon = self.closure.canon();
        let same = canon.find(vocab::OWL_SAME_AS.0);
        let same_as_itself = self.same_as_itself.iter().map(move |&r| (r, same, r));
        equality::expanded(canon, self.closure.triples().chain(same_as_itself))
    }

    /// The closure as stored, in representatives.
    #[cfg(test)]
    pub(crate) fn store(&self) -> &Store {
        &self.closure
    }

    /// The base, merged, by predicate, and the triples its hash set holds.
    #[cfg(test)]
    pub(crate) fn base_by_predicate(&self) -> (ByPredicate, usize) {
        (self.base.sorted_all().into_owned(), self.base.recent.len())
    }

    /// The dictionary.
    #[cfg(test)]
    pub(crate) fn terms(&self) -> &Dictionary {
        &self.terms
    }

    /// The counters of the last run.
    #[cfg(test)]
    pub(crate) fn counters(&self) -> &Counters {
        &self.counters
    }

    /// What the last [`Reasoner::reason`] did, round by round and step by
    /// step, and the bytes the reasoner holds now, by part: for the bench
    /// (`crates/bench`), which records them beside its times. The counts
    /// are kept by every run at a cost of an integer add per family and
    /// round; the bytes are computed here, by a walk that visits each
    /// table's batches, not its tuples. See [`profile`](crate::profile).
    pub fn profile(&self) -> Profile {
        let c = &self.counters;
        let f = c.families.len();
        let rounds = (0..c.delta.len())
            .map(|r| profile::Round {
                delta: c.delta[r],
                emitted: c.emitted[r * f..(r + 1) * f].to_vec(),
                family_ns: c.family_ns[r * f..(r + 1) * f].to_vec(),
                ns: c.round_ns[r],
            })
            .collect();
        Profile {
            families: c.families.clone(),
            rounds,
            steps: c.steps.clone(),
            memory: self.memory(),
        }
    }

    /// The bytes the reasoner holds, by part ([`Memory`]).
    fn memory(&self) -> Memory {
        let (dictionary_text, dictionary_entries, dictionary_slots) = self.terms.bytes();
        let canon = self.closure.canon();
        let rdf_type = canon.find(vocab::RDF_TYPE.0);
        let (mut store_type, mut store_other) = (Parts::default(), Parts::default());
        let mut tables: Vec<(u32, Parts)> = Vec::new();
        for (p, table) in self.closure.tables() {
            let parts = table.parts();
            if p == rdf_type {
                store_type.add(&parts);
            } else {
                store_other.add(&parts);
            }
            tables.push((p, parts));
        }
        tables.sort_unstable_by_key(|(p, parts)| (std::cmp::Reverse(parts.bytes()), *p));
        let largest = tables
            .iter()
            .take(5)
            .map(|&(p, parts)| (self.terms.to_oxrdf(TermId(p)).to_string(), parts))
            .collect();
        let findings = profile::vec_bytes(&self.findings)
            + self
                .findings
                .iter()
                .map(|f| profile::vec_bytes(&f.terms))
                .sum::<usize>();
        #[cfg(feature = "io")]
        let loaded = self.loaded.bytes();
        #[cfg(not(feature = "io"))]
        let loaded = 0;
        Memory {
            dictionary_text,
            dictionary_entries,
            dictionary_slots,
            base: self.base.bytes(),
            store_type,
            store_other,
            store_rest: self.closure.own_bytes(),
            largest,
            canon: canon.bytes(),
            findings,
            diagnostics: profile::diagnostic_bytes(&self.diagnostics),
            other: profile::vec_bytes(&self.same_as_itself) + loaded + self.counters.bytes(),
        }
    }

    /// A triple of the caller's as term ids, interned (blank nodes of no document).
    fn intern(&mut self, t: TripleRef<'_>) -> [u32; 3] {
        [
            self.terms.intern_subject(t.subject, DocId::NONE).0,
            self.terms.intern_iri(t.predicate.as_str()).0,
            self.terms.intern_term(t.object, DocId::NONE).0,
        ]
    }

    /// A triple of the caller's as term ids, if every term is known.
    fn lookup(&self, t: TripleRef<'_>) -> Option<[u32; 3]> {
        Some([
            self.terms.get_term(t.subject.into(), DocId::NONE)?.0,
            self.terms.get_iri(t.predicate.as_str())?.0,
            self.terms.get_term(t.object, DocId::NONE)?.0,
        ])
    }

    /// Adds triples to the base, and queues the new ones for the next run.
    fn add(&mut self, batch: Vec<[u32; 3]>) {
        let new = self.base.fresh(batch);
        self.queue(&new);
        self.base.extend(new);
    }

    /// Queues new base triples into the closure for an incremental run (a
    /// full run inserts the whole base instead).
    fn queue(&mut self, new: &[[u32; 3]]) {
        if new.is_empty() || self.state == State::Full {
            return;
        }
        self.closure.extend(new.iter().map(|&[s, p, o]| (s, p, o)));
        self.state = State::Added;
    }

    /// A base triple was removed: the next run starts from the base.
    fn restart(&mut self) {
        self.state = State::Full;
        #[cfg(feature = "io")]
        self.loaded.clear();
    }

    /// Runs the rule table to the fixpoint: from the base and the facts in a
    /// fresh store (`full`), or from what was queued; again from the base
    /// after each restart. Returns the rounds.
    fn run(&mut self, full: bool) -> usize {
        if full && self.workers.threads() > 1 {
            self.terms.drop_slots();
        }
        let families: Vec<&'static Family> = self.families.iter().filter(|f| f.is_on(self.switches)).collect();
        self.counters = Counters::new(&families);
        let mut start = full.then(equality::canon);
        if full {
            self.findings.clear();
        }
        let merges = if full { 0 } else { self.closure.canon().merges() };
        let mut restarted = false;
        let mut found = Vec::new();
        let mut rounds = 0;
        loop {
            if let Some(canon) = start.take() {
                self.start(&families, canon);
            }
            rounds += self.rounds(&families, &mut found);
            if !self.closure.restarted() {
                break;
            }
            restarted = true;
            start = Some(std::mem::take(&mut self.closure).into_canon());
        }
        let merged = restarted || self.closure.canon().merges() != merges;
        if merged {
            let weights = Weights::new(self.closure.canon(), &self.terms);
            self.rdf = self
                .closure
                .triples()
                .map(|(s, p, o)| weights.of(&self.terms, s, p, o))
                .sum();
        }
        self.same_as_itself.clear();
        self.same_as_itself_rdf = 0;
        if self.switches.is_on(Switch::Trivial) {
            self.same_as_itself = trivial::same_as_itself(&self.closure, &self.terms);
            let canon = self.closure.canon();
            let same = canon.find(vocab::OWL_SAME_AS.0);
            let weights = Weights::new(canon, &self.terms);
            self.same_as_itself_rdf = self
                .same_as_itself
                .iter()
                .map(|&r| weights.of(&self.terms, r, same, r))
                .sum();
        }
        if full || merged || !found.is_empty() {
            self.findings.append(&mut found);
            let canon = self.closure.canon();
            for f in &mut self.findings {
                equality::canonical(canon, f);
            }
            self.findings.sort_unstable();
            self.findings.dedup();
            self.diagnostics = diagnostics(&self.findings, canon, &self.terms);
        }
        rounds
    }

    /// A fresh store with `canon`'s classes, holding the facts of
    /// `families` (those that are on) and the base.
    fn start(&mut self, families: &[&Family], canon: Canon) {
        // The old closure is dropped before the new one fills.
        self.closure = Store::with_canon(canon);
        self.closure.set_workers(self.workers.clone());
        self.rdf = 0;
        let facts = families.iter().flat_map(|f| f.facts).map(|&[s, p, o]| [s.0, p.0, o.0]);
        if self.closure.canon().is_empty() {
            // Every id is its own representative, so the base's pairs are
            // each table's: the tables get them as sorted batches of exact
            // size, which the first step neither copies nor sorts.
            let facts: Vec<[u32; 3]> = facts.collect();
            let base: Vec<(u32, &[[u32; 2]])> = self.base.flush().iter().collect();
            self.closure.fill(&base, &facts);
        } else {
            // After a restart: the canon rewrites the pairs, and the step
            // sorts them.
            self.closure.extend(facts.map(|[s, p, o]| (s, p, o)));
            let base: Vec<(u32, &[[u32; 2]])> = self.base.flush().iter().collect();
            self.closure.extend_by_predicate(&base);
        }
    }

    /// Runs `families` (those that are on) on the store to its fixpoint (or
    /// to a restart), pushing the findings to `found` and the counters.
    /// Returns the rounds.
    fn rounds(&mut self, families: &[&'static Family], found: &mut Vec<Finding>) -> usize {
        let Self {
            terms,
            switches,
            closure,
            rdf,
            counters,
            ..
        } = self;
        let terms: &Dictionary = terms;
        // What a stored triple stands for, as the run starts: when a merge
        // changes it, the run counts the store again at its end.
        let weights = Weights::new(closure.canon(), terms);
        let switches = *switches;
        let mut tasks: Vec<Task> = families
            .iter()
            .map(|&family| Task {
                family,
                findings: Vec::new(),
                emitted: 0,
                ns: 0,
            })
            .collect();
        let rounds = closure.run(|round| {
            let start = Clock::now();
            let (tuples, triples) = delta(round, terms, &weights);
            counters.delta.push(tuples);
            *rdf += triples;
            round.tasks(&mut tasks, |round, task| {
                let mut cx = Context {
                    terms,
                    switches,
                    findings: &mut task.findings,
                };
                let begin = Clock::now();
                let before = round.emitted();
                (task.family.rules)(round, &mut cx);
                task.emitted = round.emitted() - before;
                task.ns = begin.elapsed_ns();
            });
            for task in &mut tasks {
                counters.emitted.push(task.emitted);
                counters.family_ns.push(task.ns);
                found.append(&mut task.findings);
            }
            counters.round_ns.push(start.elapsed_ns());
        });
        counters.steps.extend(closure.take_steps());
        rounds
    }

    /// True when `(s, p, _)` is an RDF triple: a subject that is not a
    /// literal, an IRI predicate.
    fn is_rdf(&self, s: u32, p: u32) -> bool {
        self.terms.kind(TermId(s)) != Kind::Literal && self.terms.kind(TermId(p)) == Kind::Iri
    }

    /// The `oxrdf` triple of term ids, or `None` when it is not RDF.
    fn rdf_triple(&self, s: u32, p: u32, o: u32) -> Option<Triple> {
        if !self.is_rdf(s, p) {
            return None;
        }
        let subject: NamedOrBlankNode = match self.terms.to_oxrdf(TermId(s)) {
            oxrdf::Term::NamedNode(n) => n.into(),
            oxrdf::Term::BlankNode(b) => b.into(),
            _ => return None,
        };
        let predicate = NamedNode::new_unchecked(self.terms.iri(TermId(p))?);
        Some(Triple::new(subject, predicate, self.terms.to_oxrdf(TermId(o))))
    }
}

/// A new dictionary: the vocabulary, then the literals the rules name, at
/// the ids every reasoner (and every snapshot) has them.
pub(crate) fn new_dictionary() -> Dictionary {
    let mut terms = Dictionary::new();
    datatype::intern_rule_literals(&mut terms);
    terms
}

/// The public form of `findings` (in representatives): each expanded to
/// every member of its terms' classes, sorted, deduplicated.
fn diagnostics(findings: &[Finding], canon: &Canon, terms: &Dictionary) -> Vec<Diagnostic> {
    let mut expanded = Vec::with_capacity(findings.len());
    for f in findings {
        equality::expand_finding(canon, f, &mut expanded);
    }
    expanded.sort_unstable();
    expanded.dedup();
    expanded.iter().map(|f| Diagnostic::from_finding(f, terms)).collect()
}

/// The key of a fixed document (an empty Turtle one): a snapshot saves it
/// with the keys of the documents loaded, and keeps those keys on load only
/// when this build gives the same, as a sign that it hashes documents as the
/// saving build did.
#[cfg(feature = "io")]
fn probe_key() -> u128 {
    document_key(b"", Format::Turtle, None)
}

/// One family's task in a round ([`Round::tasks`]): what it found and
/// emitted.
#[derive(Debug)]
struct Task {
    family: &'static Family,
    findings: Vec<Finding>,
    emitted: usize,
    /// Its time in the round.
    ns: u64,
}

/// The tuples in the deltas of the store's tables, and the RDF triples
/// they stand for once expanded: called once a round, it counts each stored
/// triple once over a run in which no class changes. A large delta is
/// counted in chunks ([`Round::split`]).
fn delta(round: &mut Round<'_>, terms: &Dictionary, weights: &Weights) -> (usize, usize) {
    let (mut tuples, mut rdf) = (0, 0);
    for (p, table) in round.store().tables() {
        let recent = table.by_subject().recent();
        if recent.is_empty() {
            continue;
        }
        tuples += recent.len();
        if weights.is_trivial() && terms.kind(TermId(p)) != Kind::Iri {
            continue;
        }
        let counts = round.split(recent, |_, chunk, count: &mut usize| {
            *count = rdf_triples(chunk, p, terms, weights);
        });
        rdf += counts.iter().sum::<usize>();
    }
    (tuples, rdf)
}

/// The RDF triples that `p`'s `pairs` stand for once expanded.
fn rdf_triples(pairs: &[(u32, u32)], p: u32, terms: &Dictionary, weights: &Weights) -> usize {
    if !weights.is_trivial() {
        return pairs.iter().map(|&(s, o)| weights.of(terms, s, p, o)).sum();
    }
    // Sorted by subject: one lookup per subject.
    let mut rdf = 0;
    let mut last: Option<(u32, bool)> = None;
    for &(s, _) in pairs {
        let literal = match last {
            Some((l, literal)) if l == s => literal,
            _ => {
                let literal = terms.kind(TermId(s)) == Kind::Literal;
                last = Some((s, literal));
                literal
            }
        };
        rdf += usize::from(!literal);
    }
    rdf
}

/// The triples in `new` and not in `old`, sorted by predicate, when `old`
/// is a subset of `new`; `None` when it is not.
fn added(old: &ByPredicate, new: &ByPredicate) -> Option<Vec<[u32; 3]>> {
    let mut more = Vec::new();
    let mut old = old.iter().peekable();
    for (p, pairs) in new.iter() {
        if old.peek().is_some_and(|&(q, _)| q < p) {
            return None;
        }
        let mine: &[[u32; 2]] = old.next_if(|&(q, _)| q == p).map_or(&[], |(_, mine)| mine);
        let mut mine = mine.iter().peekable();
        for pair in pairs {
            if mine.next_if_eq(&pair).is_none() {
                more.push([pair[0], p, pair[1]]);
            }
        }
        if mine.peek().is_some() {
            return None;
        }
    }
    old.peek().is_none().then_some(more)
}

/// Identifies a document for [`Reasoner::load_str`]'s "the same document
/// again": its bytes, format and base IRI, hashed twice (FxHash and
/// SipHash, 128 bits together), so that two different documents share a key
/// with a probability near 2^-128 ([`DocumentKey`]).
#[cfg(feature = "io")]
fn document_key(text: &[u8], format: Format, base_iri: Option<&str>) -> u128 {
    let mut key = DocumentKey::new(format, base_iri);
    key.feed(text);
    key.finish().0
}

/// A document's key, fed its bytes as they are read: the format and base
/// IRI, then the bytes in blocks of [`KEY_BLOCK`] (the last one shorter),
/// then their count, each into both hashers. The blocks fall at the same
/// offsets however the bytes come, so a file read in windows and the same
/// text in memory get the same key.
#[cfg(feature = "io")]
struct DocumentKey {
    fx: rustc_hash::FxHasher,
    sip: std::hash::DefaultHasher,
    /// The bytes of the block being filled.
    block: Vec<u8>,
    len: u64,
}

/// The bytes hashed at a time by a [`DocumentKey`].
#[cfg(feature = "io")]
const KEY_BLOCK: usize = 1 << 16;

#[cfg(feature = "io")]
impl DocumentKey {
    fn new(format: Format, base_iri: Option<&str>) -> Self {
        use std::hash::Hash;
        let mut key = DocumentKey {
            fx: rustc_hash::FxHasher::default(),
            sip: std::hash::DefaultHasher::new(),
            block: Vec::new(),
            len: 0,
        };
        (format as u8).hash(&mut key.fx);
        base_iri.hash(&mut key.fx);
        (format as u8).hash(&mut key.sip);
        base_iri.hash(&mut key.sip);
        key
    }

    /// The key of the document `reader` holds.
    fn read(reader: &mut impl std::io::Read, format: Format, base_iri: Option<&str>) -> std::io::Result<u128> {
        let mut key = DocumentKey::new(format, base_iri);
        let mut buffer = vec![0; 1 << 20];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => return Ok(key.finish().0),
                Ok(n) => key.feed(&buffer[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
    }

    /// The next bytes of the document.
    fn feed(&mut self, mut bytes: &[u8]) {
        self.len += bytes.len() as u64;
        if !self.block.is_empty() {
            let more = (KEY_BLOCK - self.block.len()).min(bytes.len());
            self.block.extend_from_slice(&bytes[..more]);
            bytes = &bytes[more..];
            if self.block.len() < KEY_BLOCK {
                return;
            }
            Self::write(&mut self.fx, &mut self.sip, &self.block);
            self.block.clear();
        }
        let (blocks, rest) = bytes.as_chunks::<KEY_BLOCK>();
        for block in blocks {
            Self::write(&mut self.fx, &mut self.sip, block);
        }
        self.block.extend_from_slice(rest);
    }

    fn write(fx: &mut impl std::hash::Hasher, sip: &mut impl std::hash::Hasher, block: &[u8]) {
        fx.write(block);
        sip.write(block);
    }

    /// The key, and the document's length in bytes.
    fn finish(mut self) -> (u128, u64) {
        use std::hash::{Hash, Hasher};
        if !self.block.is_empty() {
            Self::write(&mut self.fx, &mut self.sip, &self.block);
        }
        self.len.hash(&mut self.fx);
        self.len.hash(&mut self.sip);
        let key = (u128::from(self.fx.finish()) << 64) | u128::from(self.sip.finish());
        (key, self.len)
    }
}

/// The documents loaded since the last removal: their keys, and their
/// lengths, which say whether a file must be keyed before it is parsed (a
/// file of no loaded length is no loaded document).
#[cfg(feature = "io")]
#[derive(Debug)]
struct Documents {
    keys: FxHashSet<u128>,
    /// `None` when some lengths are not known: keys restored from a snapshot.
    lengths: Option<FxHashSet<u64>>,
}

#[cfg(feature = "io")]
impl Default for Documents {
    fn default() -> Self {
        Documents {
            keys: FxHashSet::default(),
            lengths: Some(FxHashSet::default()),
        }
    }
}

#[cfg(feature = "io")]
impl Documents {
    /// The documents of these keys, of lengths not known.
    fn restored(keys: Vec<u128>) -> Self {
        Documents {
            lengths: if keys.is_empty() {
                Some(FxHashSet::default())
            } else {
                None
            },
            keys: keys.into_iter().collect(),
        }
    }

    fn contains(&self, key: u128) -> bool {
        self.keys.contains(&key)
    }

    /// True when a loaded document may be `len` bytes long.
    fn may_hold(&self, len: u64) -> bool {
        !self.keys.is_empty() && self.lengths.as_ref().is_none_or(|lengths| lengths.contains(&len))
    }

    fn insert(&mut self, key: u128, len: u64) {
        self.keys.insert(key);
        if let Some(lengths) = &mut self.lengths {
            lengths.insert(len);
        }
    }

    fn clear(&mut self) {
        *self = Documents::default();
    }

    /// The bytes the sets take.
    fn bytes(&self) -> usize {
        profile::hash_bytes(self.keys.capacity(), size_of::<u128>())
            + self
                .lengths
                .as_ref()
                .map_or(0, |l| profile::hash_bytes(l.capacity(), size_of::<u64>()))
    }
}

/// The fewest triples the base's hash set holds before it is merged into
/// the pairs.
const MERGE_AT_LEAST: usize = 4096;

/// Triples as sorted `(s, o)` pairs by predicate, in one vector: the
/// predicates ascending, each with the place of its first pair, and every
/// pair as two ids, predicate after predicate, each predicate's strictly
/// ascending and none without pairs. 8 bytes a triple.
///
/// One vector, not one a predicate, so that the triples of a load become
/// the pairs in their own allocation ([`ByPredicate::from_sorted`]): a
/// load's buffer freed beside new vectors is memory the allocator keeps
/// (on lubm-50, a base of one vector a predicate raised the all-cores
/// footprint by about 70 MB).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ByPredicate {
    /// Each predicate, and the index of its first pair.
    starts: Vec<(u32, usize)>,
    /// The pairs, `s` then `o`.
    ids: Vec<u32>,
}

impl ByPredicate {
    /// The triples of `triples`, deduplicated.
    fn from_triples(mut triples: Vec<[u32; 3]>) -> Self {
        triples.sort_unstable_by_key(by_predicate);
        triples.dedup();
        Self::from_sorted(triples)
    }

    /// The triples of `triples` (sorted by predicate, [`by_predicate`],
    /// deduplicated), made pairs in place: in the triples' own allocation,
    /// which then gives back the room it no longer needs.
    fn from_sorted(triples: Vec<[u32; 3]>) -> Self {
        let n = triples.len();
        let mut starts: Vec<(u32, usize)> = Vec::new();
        for (i, t) in triples.iter().enumerate() {
            if starts.last().is_none_or(|&(p, _)| p != t[1]) {
                starts.push((t[1], i));
            }
        }
        let mut ids = triples.into_flattened();
        // Pair i's ids go to 2i and 2i + 1, read from 3i and 3i + 2: never
        // from where an earlier pair was written.
        for i in 0..n {
            ids[2 * i] = ids[3 * i];
            ids[2 * i + 1] = ids[3 * i + 2];
        }
        ids.truncate(2 * n);
        trim(&mut ids);
        Self { starts, ids }
    }

    /// The pairs.
    pub(crate) fn len(&self) -> usize {
        self.ids.len() / 2
    }

    /// Every pair, predicate after predicate.
    fn pairs(&self) -> &[[u32; 2]] {
        self.ids.as_chunks::<2>().0
    }

    /// Each predicate, ascending, and its pairs.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (u32, &[[u32; 2]])> + '_ {
        let pairs = self.pairs();
        self.starts.iter().enumerate().map(move |(i, &(p, start))| {
            let end = self.starts.get(i + 1).map_or(pairs.len(), |next| next.1);
            (p, &pairs[start..end])
        })
    }

    /// Appends predicate `p` and its pairs (sorted, deduplicated, not
    /// empty), `p` after every predicate held: a loaded snapshot.
    pub(crate) fn push(&mut self, p: u32, pairs: &[[u32; 2]]) {
        debug_assert!(self.starts.last().is_none_or(|&(q, _)| q < p) && !pairs.is_empty());
        self.starts.push((p, self.len()));
        self.ids.extend_from_slice(pairs.as_flattened());
    }

    fn contains(&self, &[s, p, o]: &[u32; 3]) -> bool {
        let Ok(i) = self.starts.binary_search_by_key(&p, |&(q, _)| q) else {
            return false;
        };
        let pairs = self.pairs();
        let end = self.starts.get(i + 1).map_or(pairs.len(), |next| next.1);
        pairs[self.starts[i].1..end].binary_search(&[s, o]).is_ok()
    }

    /// Each predicate, the range of its pairs.
    fn ranges(&self) -> Vec<(u32, usize, usize)> {
        let len = self.len();
        self.starts
            .iter()
            .enumerate()
            .map(|(i, &(p, start))| (p, start, self.starts.get(i + 1).map_or(len, |next| next.1)))
            .collect()
    }

    /// Merges triples that are not held, sorted by predicate, into the
    /// pairs: in place, from the back, the vector growing by exactly what
    /// they need (with no pairs yet, in the triples' allocation).
    fn merge(&mut self, more: Vec<[u32; 3]>) {
        if more.is_empty() {
            return;
        }
        if self.ids.is_empty() {
            *self = Self::from_sorted(more);
            return;
        }
        // Each predicate of either, ascending, with its range of pairs held
        // and its run of `more`.
        let mut old = self.ranges().into_iter().peekable();
        let mut entries: Vec<(u32, usize, usize, &[[u32; 3]])> = Vec::with_capacity(self.starts.len());
        for run in more.chunk_by(|a, b| a[1] == b[1]) {
            let p = run[0][1];
            entries.extend(std::iter::from_fn(|| old.next_if(|r| r.0 < p)).map(|(q, lo, hi)| (q, lo, hi, &[][..])));
            let (lo, hi) = old.next_if(|r| r.0 == p).map_or((0, 0), |(_, lo, hi)| (lo, hi));
            entries.push((p, lo, hi, run));
        }
        entries.extend(old.map(|(q, lo, hi)| (q, lo, hi, &[][..])));

        let total = self.len() + more.len();
        self.ids.reserve_exact(2 * more.len());
        self.ids.resize(2 * total, 0);
        let ids = &mut self.ids;
        // Written at k, read at i - 1 < k, or copied onto itself when
        // every pair of `more` before it is placed.
        let mut k = total;
        let mut starts = Vec::with_capacity(entries.len());
        for &(p, lo, hi, run) in entries.iter().rev() {
            let (mut i, mut j) = (hi, run.len());
            while i > lo || j > 0 {
                k -= 1;
                let held = i > lo && (j == 0 || [ids[2 * i - 2], ids[2 * i - 1]] > [run[j - 1][0], run[j - 1][2]]);
                let [s, o] = if held {
                    i -= 1;
                    [ids[2 * i], ids[2 * i + 1]]
                } else {
                    j -= 1;
                    [run[j][0], run[j][2]]
                };
                ids[2 * k] = s;
                ids[2 * k + 1] = o;
            }
            starts.push((p, k));
        }
        debug_assert_eq!(k, 0, "every pair placed");
        starts.reverse();
        self.starts = starts;
    }

    /// Removes the triples of `gone` (held, sorted by predicate,
    /// deduplicated): in place, the predicates left with no pair dropped.
    fn remove(&mut self, gone: &[[u32; 3]]) {
        let mut gone = gone.iter().map(by_predicate).peekable();
        let ranges = self.ranges();
        let ids = &mut self.ids;
        let mut starts = Vec::with_capacity(ranges.len());
        let mut w = 0;
        for (p, lo, hi) in ranges {
            let begin = w;
            for r in lo..hi {
                let (s, o) = (ids[2 * r], ids[2 * r + 1]);
                if gone.next_if_eq(&(p, s, o)).is_some() {
                    continue;
                }
                ids[2 * w] = s;
                ids[2 * w + 1] = o;
                w += 1;
            }
            if w > begin {
                starts.push((p, begin));
            }
        }
        debug_assert!(gone.next().is_none(), "every triple removed was held");
        ids.truncate(2 * w);
        trim(ids);
        self.starts = starts;
    }

    /// The bytes held.
    fn bytes(&self) -> usize {
        profile::vec_bytes(&self.starts) + profile::vec_bytes(&self.ids)
    }
}

/// The base triples, deduplicated: sorted pairs by predicate
/// ([`ByPredicate`]), as the store's tables hold them, and the triples
/// added since the last merge in a hash set (none of them among the pairs).
/// The set is merged in when it passes [`MERGE_AT_LEAST`] or an eighth of
/// the pairs, so an addition costs O(log n) plus O(1) amortized, and the
/// memory stays at 8 bytes a triple (10 with the vector's slack at most),
/// plus the hash set: at most an eighth as many entries.
#[derive(Debug, Default)]
struct Base {
    merged: ByPredicate,
    recent: FxHashSet<[u32; 3]>,
}

impl Base {
    /// The base of these pairs, with no triple in the hash set.
    fn restored(merged: ByPredicate) -> Self {
        Base {
            merged,
            recent: FxHashSet::default(),
        }
    }

    fn len(&self) -> usize {
        self.merged.len() + self.recent.len()
    }

    fn contains(&self, t: &[u32; 3]) -> bool {
        self.merged.contains(t) || self.recent.contains(t)
    }

    /// Every base triple, by predicate: the pairs themselves when the hash
    /// set is empty, else a merged copy.
    fn sorted_all(&self) -> Cow<'_, ByPredicate> {
        if self.recent.is_empty() {
            return Cow::Borrowed(&self.merged);
        }
        let mut more: Vec<[u32; 3]> = self.recent.iter().copied().collect();
        more.sort_unstable_by_key(by_predicate);
        let mut all = self.merged.clone();
        all.merge(more);
        Cow::Owned(all)
    }

    /// The base triples, in no order.
    #[cfg(test)]
    fn iter(&self) -> impl Iterator<Item = [u32; 3]> + '_ {
        self.merged
            .iter()
            .flat_map(|(p, pairs)| pairs.iter().map(move |&[s, o]| [s, p, o]))
            .chain(self.recent.iter().copied())
    }

    /// The triples of `batch` that are not in the base: sorted by
    /// predicate, then subject, then object ([`by_predicate`]),
    /// deduplicated.
    fn fresh(&self, mut batch: Vec<[u32; 3]>) -> Vec<[u32; 3]> {
        batch.sort_unstable_by_key(by_predicate);
        batch.dedup();
        batch.retain(|t| !self.contains(t));
        batch
    }

    /// Adds triples that are not in the base (as [`Base::fresh`] gives them).
    fn extend(&mut self, new: Vec<[u32; 3]>) {
        if new.is_empty() {
            return;
        }
        if self.recent.len() + new.len() <= MERGE_AT_LEAST.max(self.merged.len() / 8) {
            self.recent.extend(new);
            return;
        }
        let mut more = new;
        more.extend(self.recent.drain());
        more.sort_unstable_by_key(by_predicate);
        self.merged.merge(more);
    }

    /// Merges the hash set into the pairs and returns the whole base.
    fn flush(&mut self) -> &ByPredicate {
        if !self.recent.is_empty() {
            let mut more: Vec<[u32; 3]> = self.recent.drain().collect();
            more.sort_unstable_by_key(by_predicate);
            self.merged.merge(more);
        }
        &self.merged
    }

    /// Makes the base exactly these pairs.
    fn replace(&mut self, merged: ByPredicate) {
        *self = Base::restored(merged);
    }

    /// Removes the base triples among `gone`; returns how many. The vector
    /// gives back its room when more than a quarter of it is spare.
    fn remove(&mut self, mut gone: Vec<[u32; 3]>) -> usize {
        gone.retain(|t| self.contains(t));
        if gone.is_empty() {
            return 0;
        }
        gone.sort_unstable_by_key(by_predicate);
        gone.dedup();
        self.flush();
        self.merged.remove(&gone);
        gone.len()
    }

    /// The bytes the base holds: the pairs and the hash set.
    fn bytes(&self) -> usize {
        self.merged.bytes() + profile::hash_bytes(self.recent.capacity(), size_of::<[u32; 3]>())
    }
}

/// Gives back the spare room of the base's vector when it is more than a
/// quarter of its length (a load's buffer, made pairs, is a third larger at
/// least; a merge leaves none; a removal can leave any).
fn trim(ids: &mut Vec<u32>) {
    if ids.capacity() > ids.len() + ids.len() / 4 {
        ids.shrink_to_fit();
    }
}

/// The order of the base: by predicate, then subject, then object.
fn by_predicate(&[s, p, o]: &[u32; 3]) -> (u32, u32, u32) {
    (p, s, o)
}

/// What the last run did, round by round: the counters experiment P0
/// (`docs/specs/experiments.md`) reads. Kept by every run, in flat vectors
/// that cost a push each; [`Reasoner::profile`] gives them out by round.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Counters {
    /// The names of the families that ran (those the switches left on), in
    /// table order.
    pub(crate) families: Vec<&'static str>,
    /// Each round's delta: the tuples new in the store when the round
    /// starts (its tables' recent parts; a tuple a merge rewrote is new
    /// again). The first
    /// round's is the run's input (the base and the facts in a full run,
    /// the additions in an incremental one); round r + 1's is what round r
    /// derived that was new.
    pub(crate) delta: Vec<usize>,
    /// The tuples each family emitted, duplicates included: round by round,
    /// one entry per family in table order (`emitted[round * families +
    /// family]`).
    pub(crate) emitted: Vec<usize>,
    /// Each family's time, laid out as `emitted`.
    pub(crate) family_ns: Vec<u64>,
    /// Each round's wall time.
    pub(crate) round_ns: Vec<u64>,
    /// The store's steps, in order: one more than the rounds per attempt.
    pub(crate) steps: Vec<Step>,
}

impl Counters {
    fn new(families: &[&Family]) -> Self {
        Self {
            families: families.iter().map(|f| f.name).collect(),
            ..Self::default()
        }
    }

    fn clear(&mut self) {
        self.delta.clear();
        self.emitted.clear();
        self.family_ns.clear();
        self.round_ns.clear();
        self.steps.clear();
    }

    /// The bytes the counters hold.
    fn bytes(&self) -> usize {
        profile::vec_bytes(&self.families)
            + profile::vec_bytes(&self.delta)
            + profile::vec_bytes(&self.emitted)
            + profile::vec_bytes(&self.family_ns)
            + profile::vec_bytes(&self.round_ns)
            + profile::vec_bytes(&self.steps)
    }

    /// The counters with every time set to 0: the counts alone, which do
    /// not change from run to run.
    #[cfg(test)]
    pub(crate) fn counts(&self) -> Counters {
        let mut c = self.clone();
        c.family_ns.fill(0);
        c.round_ns.fill(0);
        for s in &mut c.steps {
            (s.ns, s.sort_ns, s.changed_ns) = (0, 0, 0);
        }
        c
    }

    /// Rounds the last run took.
    #[cfg(test)]
    pub(crate) fn rounds(&self) -> usize {
        self.delta.len()
    }

    /// What the family called `name` emitted in each round.
    #[cfg(test)]
    pub(crate) fn emitted_by(&self, name: &str) -> Vec<usize> {
        let f = self
            .families
            .iter()
            .position(|n| *n == name)
            .unwrap_or_else(|| panic!("no family {name:?}"));
        self.emitted
            .iter()
            .skip(f)
            .step_by(self.families.len())
            .copied()
            .collect()
    }
}

/// The reasoner over a toy rule table, so that incremental runs, findings
/// and generalized triples are tested while the real families are being
/// written. The tests that need only the public interface are in
/// `tests/reasoner.rs`.
#[cfg(all(test, feature = "io"))]
mod tests {
    use super::*;
    use crate::Switch;
    use crate::engine::{Pair, Round, Variable};
    use crate::term::vocab;
    use std::collections::BTreeSet;

    const PREFIXES: &str = "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .\n";

    fn ex(cx: &Context<'_>, local: &str) -> Option<u32> {
        cx.terms.get_iri(&format!("http://example.org/{local}")).map(|t| t.0)
    }

    fn all(v: &Variable<Pair>) -> Vec<Pair> {
        v.parts().flatten().copied().collect()
    }

    /// A document's key is the same however its bytes come: whole, a byte
    /// at a time, in blocks of the key's size and around it, or as a file
    /// read in windows hands them over.
    #[test]
    fn a_document_key_does_not_depend_on_how_the_bytes_come() {
        let mut text = String::from("@prefix ex: <http://example.org/> .\n");
        let mut i = 0;
        while text.len() < 5 * KEY_BLOCK / 2 {
            text.push_str(&format!("ex:s{i} ex:p ex:o{i} ; ex:q \"label {i}\"@en .\n"));
            i += 1;
        }
        let bytes = text.as_bytes();
        let whole = document_key(bytes, Format::Turtle, None);
        for chunk in [
            1,
            7,
            KEY_BLOCK - 1,
            KEY_BLOCK,
            KEY_BLOCK + 1,
            3 * KEY_BLOCK,
            bytes.len(),
        ] {
            let mut key = DocumentKey::new(Format::Turtle, None);
            for part in bytes.chunks(chunk) {
                key.feed(part);
            }
            assert_eq!(key.finish(), (whole, bytes.len() as u64), "in parts of {chunk} bytes");
        }
        // As a file read in windows of a few KiB hands them over.
        for window in [4 << 10, 50_000] {
            let mut key = DocumentKey::new(Format::Turtle, None);
            crate::io::parse_reader_on(
                &Workers::one(),
                &mut Dictionary::new(),
                &mut std::io::Cursor::new(bytes),
                Format::Turtle,
                None,
                None,
                window,
                |part| key.feed(part),
                |_| {},
            )
            .unwrap();
            assert_eq!(key.finish().0, whole, "in windows of {window} bytes");
        }
        // The format, the base IRI and the bytes each change the key.
        assert_ne!(whole, document_key(bytes, Format::NTriples, None));
        assert_ne!(whole, document_key(bytes, Format::Turtle, Some("http://example.org/")));
        assert_ne!(whole, document_key(&bytes[1..], Format::Turtle, None));
        assert_ne!(
            document_key(b"", Format::Turtle, None),
            document_key(b"\0", Format::Turtle, None)
        );
    }

    /// A file and the same text in memory are one document, whichever is
    /// loaded first, and so is a file loaded again after a snapshot.
    #[test]
    fn a_file_and_its_text_are_one_document() {
        let text = format!("{PREFIXES}ex:a ex:p [ ex:q ex:b ] , ( ex:c ex:d ) .\nex:a a ex:C .\n");
        let path = std::env::temp_dir().join(format!("turbowl-one-document-{}.ttl", std::process::id()));
        std::fs::write(&path, &text).unwrap();
        let mut r = Reasoner::new();
        r.load_file(&path).unwrap();
        let (base, terms) = (r.reason().base, r.terms.len());
        r.load_str(&text, Format::Turtle, None).unwrap();
        r.load_file(&path).unwrap();
        assert_eq!((r.reason().rounds, r.reason().base, r.terms.len()), (0, base, terms));

        let mut r = Reasoner::new();
        r.load_str(&text, Format::Turtle, None).unwrap();
        r.load_file(&path).unwrap();
        assert_eq!((r.reason().base, r.terms.len()), (base, terms));

        // A snapshot keeps the keys but not the lengths: the file is keyed
        // before it is parsed, and is the same document.
        let mut bytes = Vec::new();
        r.write_snapshot(&mut bytes).unwrap();
        let mut r = Reasoner::from_snapshot_bytes(&bytes).unwrap();
        assert!(r.loaded.lengths.is_none());
        r.load_file(&path).unwrap();
        assert_eq!((r.reason().rounds, r.reason().base, r.terms.len()), (0, base, terms));

        // A file of a loaded document's length but other bytes is another
        // document.
        let other = text.replace("ex:C", "ex:D");
        assert_eq!(other.len(), text.len());
        std::fs::write(&path, &other).unwrap();
        r.load_file(&path).unwrap();
        assert_eq!(
            r.reason().base,
            base + 8,
            "its own blank nodes' 7 triples, and ex:a a ex:D"
        );
        std::fs::remove_file(path).unwrap();
    }

    /// `ex:anc` is transitive: recent pairs joined with all, on both sides.
    fn transitive(round: &mut Round<'_>, cx: &mut Context<'_>) {
        let Some(anc) = ex(cx, "anc") else { return };
        let Some(v) = round.by_subject(anc) else { return };
        let (recent, all) = (v.recent().to_vec(), all(v));
        for &(x, y) in &recent {
            for &(_, z) in all.iter().filter(|p| p.0 == y) {
                round.emit(x, anc, z);
            }
        }
        for &(w, x) in &all {
            for &(_, y) in recent.iter().filter(|p| p.0 == x) {
                round.emit(w, anc, y);
            }
        }
    }

    /// cax-sco's shape, and a finding for every member of `ex:Bad`, made in
    /// every round and twice each time: duplicates on purpose.
    fn typing(round: &mut Round<'_>, cx: &mut Context<'_>) {
        let (ty, sub) = (vocab::RDF_TYPE.0, vocab::RDFS_SUB_CLASS_OF.0);
        let Some(types) = round.by_subject(ty) else { return };
        let (t_recent, t_all) = (types.recent().to_vec(), all(types));
        if let Some(subs) = round.by_subject(sub) {
            let (s_recent, s_all) = (subs.recent().to_vec(), all(subs));
            for &(x, c) in &t_recent {
                for &(_, d) in s_all.iter().filter(|p| p.0 == c) {
                    round.emit(x, ty, d);
                }
            }
            for &(x, c) in &t_all {
                for &(_, d) in s_recent.iter().filter(|p| p.0 == c) {
                    round.emit(x, ty, d);
                }
            }
        }
        if let Some(bad) = ex(cx, "Bad") {
            for &(x, _) in t_all.iter().filter(|p| p.1 == bad) {
                cx.report("toy-bad", &[x]);
                cx.report("toy-bad", &[x]);
            }
        }
    }

    /// Rules over generalized triples: `x ex:value v` gives `v ex:valueOf x`
    /// (a literal subject when `v` is a literal), which gives `x ex:seen v`;
    /// `x ex:predOf y` gives `x y x` (`y` as a predicate).
    fn general(round: &mut Round<'_>, cx: &mut Context<'_>) {
        let rule = |round: &mut Round<'_>, from: Option<u32>, to: Option<u32>| {
            let (Some(from), Some(to)) = (from, to) else { return };
            if let Some(v) = round.by_subject(from) {
                for &(x, y) in v.recent().iter() {
                    round.emit(y, to, x);
                }
            }
        };
        rule(round, ex(cx, "value"), ex(cx, "valueOf"));
        rule(round, ex(cx, "valueOf"), ex(cx, "seen"));
        if let Some(pred_of) = ex(cx, "predOf")
            && let Some(v) = round.by_subject(pred_of)
        {
            for &(x, y) in v.recent().iter() {
                round.emit(x, y, x);
            }
        }
    }

    static TOY: &[Family] = &[
        Family {
            name: "transitive",
            switch: None,
            rules: transitive,
            facts: &[],
        },
        Family {
            name: "typing",
            switch: None,
            rules: typing,
            facts: &[[vocab::OWL_THING, vocab::RDF_TYPE, vocab::OWL_CLASS]],
        },
        Family {
            name: "general",
            switch: None,
            rules: general,
            facts: &[],
        },
    ];

    /// A reasoner over [`TOY`]. The toy rules' IRIs are interned first, as
    /// the real vocabulary is: a rule that found its head's IRI missing in
    /// one run would miss its derivation for good.
    fn toy() -> Reasoner {
        let mut r = Reasoner::with_families(TOY);
        for local in ["anc", "Bad", "value", "valueOf", "seen", "predOf"] {
            r.terms.intern_iri(&format!("http://example.org/{local}"));
        }
        r
    }

    fn load(r: &mut Reasoner, turtle: &str) {
        r.load_str(&format!("{PREFIXES}{turtle}"), Format::Turtle, None)
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// Triples of a short Turtle text, as `oxrdf` triples (for `insert`).
    fn parse(turtle: &str) -> Vec<Triple> {
        oxttl::TurtleParser::new()
            .for_slice(format!("{PREFIXES}{turtle}").as_bytes())
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn short(term: &str) -> String {
        [
            ("http://example.org/", "ex:"),
            ("http://www.w3.org/1999/02/22-rdf-syntax-ns#", "rdf:"),
            ("http://www.w3.org/2000/01/rdf-schema#", "rdfs:"),
            ("http://www.w3.org/2002/07/owl#", "owl:"),
        ]
        .iter()
        .find_map(|(ns, prefix)| {
            term.strip_prefix('<')
                .and_then(|t| t.strip_prefix(ns))
                .map(|local| format!("{prefix}{}", local.trim_end_matches('>')))
        })
        .unwrap_or_else(|| term.to_owned())
    }

    /// Triples as `s p o` lines with short IRIs.
    fn lines(triples: impl Iterator<Item = Triple>) -> BTreeSet<String> {
        triples
            .map(|t| {
                format!(
                    "{} {} {}",
                    short(&t.subject.to_string()),
                    short(&t.predicate.to_string()),
                    short(&t.object.to_string())
                )
            })
            .collect()
    }

    /// The diagnostics as `code term ...` lines.
    fn findings(r: &Reasoner) -> BTreeSet<String> {
        r.diagnostics()
            .iter()
            .map(|d| {
                let terms: Vec<String> = d.terms.iter().map(|t| short(&t.to_string())).collect();
                format!("{} {}", d.code, terms.join(" "))
            })
            .collect()
    }

    fn set(text: &str) -> BTreeSet<String> {
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn the_facts_hold_on_an_empty_base() {
        let mut r = toy();
        let summary = r.reason();
        assert_eq!(
            summary,
            Summary {
                base: 0,
                inferred: 1,
                rounds: 1,
                diagnostics: 0
            }
        );
        assert_eq!(lines(r.triples()), set("owl:Thing rdf:type owl:Class"));
        assert_eq!(lines(r.inferred()), set("owl:Thing rdf:type owl:Class"));
        // And again after `clear`, and after a removal empties the base.
        r.clear();
        assert_eq!(r.triples().count(), 0, "cleared, not yet run");
        assert_eq!(r.reason().inferred, 1);
        load(&mut r, "ex:a ex:anc ex:b .");
        r.reason();
        r.remove(parse("ex:a ex:anc ex:b .").iter().map(Triple::as_ref));
        assert_eq!(r.reason().base, 0);
        assert_eq!(lines(r.triples()), set("owl:Thing rdf:type owl:Class"));
    }

    const CHAIN: &str = "ex:a ex:anc ex:b . ex:b ex:anc ex:c . ex:c ex:anc ex:d .";

    #[test]
    fn counters_record_each_round() {
        let mut r = toy();
        load(&mut r, CHAIN);
        let s = r.reason();
        assert_eq!((s.base, s.inferred, s.rounds), (3, 3 + 1, 3));
        let c = r.counters();
        assert_eq!(c.families, ["transitive", "typing", "general"]);
        // Round 1 starts with the base and the fact; it derives a-c and b-d
        // (each twice), round 2 derives a-d (twice), round 3 nothing.
        assert_eq!(c.delta, [4, 2, 1]);
        assert_eq!(c.emitted_by("transitive"), [4, 2, 0]);
        assert_eq!(c.emitted_by("typing"), [0, 0, 0]);
        assert_eq!(c.rounds(), 3);

        // An incremental run starts from the addition alone.
        load(&mut r, "ex:d ex:anc ex:e .");
        let s = r.reason();
        assert_eq!((s.base, s.inferred, s.rounds), (4, 6 + 1, 2));
        let c = r.counters();
        assert_eq!(c.delta, [1, 3]);
        assert_eq!(c.emitted_by("transitive"), [3, 3]);

        // Nothing changed: no round.
        assert_eq!(r.reason().rounds, 0);
        assert_eq!(r.counters().rounds(), 0);
    }

    /// The profile gives the counters out by round, with one step more than
    /// rounds; each step's new tuples are the next round's delta. On one
    /// thread a round's families and a step's sorts and changes take part
    /// of its wall time.
    #[test]
    fn the_profile_gives_each_round_and_step() {
        let mut r = toy();
        r.set_threads(Threads::ONE);
        load(&mut r, CHAIN);
        r.reason();
        let p = r.profile();
        assert_eq!(p.families, ["transitive", "typing", "general"]);
        let delta: Vec<usize> = p.rounds.iter().map(|r| r.delta).collect();
        assert_eq!(delta, [4, 2, 1]);
        let emitted: Vec<&[usize]> = p.rounds.iter().map(|r| &r.emitted[..]).collect();
        assert_eq!(emitted, [&[4, 0, 0][..], &[2, 0, 0], &[0, 0, 0]]);
        // The input (the base and the fact), then what each round emitted,
        // distinct: a-c and b-d, then a-d, then nothing.
        let funnel: Vec<(usize, usize)> = p.steps.iter().map(|s| (s.pairs, s.new)).collect();
        assert_eq!(funnel, [(4, 4), (2, 2), (1, 1), (0, 0)]);
        for (step, round) in p.steps.iter().zip(&p.rounds) {
            assert_eq!(step.new, round.delta);
        }
        for step in &p.steps {
            assert!(step.sort_ns + step.changed_ns <= step.ns, "{step:?}");
        }
        for round in &p.rounds {
            assert_eq!(round.family_ns.len(), 3);
            assert!(round.family_ns.iter().sum::<u64>() <= round.ns, "{round:?}");
        }
        #[cfg(not(target_family = "wasm"))]
        assert!(p.steps.iter().any(|s| s.ns > 0), "the steps are timed");

        // Nothing changed: no round, no step.
        r.reason();
        let p = r.profile();
        assert!(p.rounds.is_empty() && p.steps.is_empty());
    }

    /// The memory breakdown: the parts add up to the total, the tables to
    /// the closure as stored, and a larger input holds more in each part
    /// that grows with it.
    #[test]
    fn the_memory_parts_add_up_and_grow() {
        use std::fmt::Write as _;
        let input = |n: usize| {
            let mut text = String::from("ex:A rdfs:subClassOf ex:B . ex:p rdfs:domain ex:A .\n");
            for i in 0..n {
                let _ = writeln!(text, "ex:i{i} ex:p ex:i{} . ex:i{i} ex:q \"v{i}\" .", i + 1);
            }
            text
        };
        let measure = |n: usize| {
            let mut r = Reasoner::new();
            load(&mut r, &input(n));
            let s = r.reason();
            (r.profile().memory, s, r.closure.len())
        };
        let (small, s, stored) = measure(100);
        let (large, _, _) = measure(10_000);
        for (m, stored) in [(&small, stored), (&large, 0)] {
            assert_eq!(
                m.total(),
                m.dictionary_text
                    + m.dictionary_entries
                    + m.dictionary_slots
                    + m.base
                    + m.store_type.bytes()
                    + m.store_other.bytes()
                    + m.store_rest
                    + m.canon
                    + m.findings
                    + m.diagnostics
                    + m.other
            );
            assert!(m.largest.len() <= 5 && !m.largest.is_empty());
            assert!(m.largest.windows(2).all(|w| w[0].1.bytes() >= w[1].1.bytes()));
            let largest: usize = m.largest.iter().map(|(_, p)| p.bytes()).sum();
            assert!(largest <= m.store_type.bytes() + m.store_other.bytes());
            if stored > 0 {
                assert_eq!(m.store_type.tuples + m.store_other.tuples, stored);
                // Each stored pair takes 8 bytes at least.
                assert!(m.store_type.bytes() + m.store_other.bytes() >= 8 * stored);
                // 8 bytes a base triple, merged.
                assert!(m.base >= 8 * s.base, "{m:?}");
            }
        }
        // ex:i0 ... are typed ex:A by prp-dom: rdf:type is a large table.
        assert!(
            large.largest.iter().any(|(p, _)| p.contains("22-rdf-syntax-ns#type")),
            "{large:?}"
        );
        assert!(large.total() > small.total());
        assert!(large.dictionary() > small.dictionary());
        assert!(large.base > small.base);
        assert!(large.store_type.bytes() > small.store_type.bytes());
        assert!(large.store_other.bytes() > small.store_other.bytes());
    }

    const INPUT: &str = "
        ex:a ex:anc ex:b . ex:b ex:anc ex:c . ex:c ex:anc ex:d . ex:d ex:anc ex:e .
        ex:C1 rdfs:subClassOf ex:C2 . ex:C2 rdfs:subClassOf ex:C3 . ex:C3 rdfs:subClassOf ex:Bad .
        ex:x a ex:C1 . ex:y a ex:C2 . _:n a ex:C3 . _:n ex:anc ex:a .
        ex:x ex:value \"five\" .
        ex:x ex:predOf ex:link . ex:y ex:predOf \"no\" . ex:y ex:predOf _:p .
    ";

    #[test]
    fn additions_in_batches_equal_one_run() {
        let triples = parse(INPUT);
        let mut one = toy();
        one.insert(triples.iter().map(Triple::as_ref));
        let whole = one.reason();
        let expected = (lines(one.triples()), lines(one.inferred()), findings(&one));
        assert!(whole.inferred > 15 && whole.diagnostics == 3, "{whole:?}");

        for batches in 1..=5 {
            for offset in 0..batches {
                let mut r = toy();
                let mut last = Summary::default();
                for b in 0..batches {
                    let batch = triples.iter().enumerate().filter(|(i, _)| (i + offset) % batches == b);
                    r.insert(batch.map(|(_, t)| t.as_ref()));
                    last = r.reason();
                }
                let got = (lines(r.triples()), lines(r.inferred()), findings(&r));
                assert_eq!(got, expected, "{batches} batches, offset {offset}");
                assert_eq!(
                    (last.base, last.inferred, last.diagnostics),
                    (whole.base, whole.inferred, whole.diagnostics)
                );
            }
        }
    }

    #[test]
    fn a_removal_removes_its_inferences() {
        let mut r = toy();
        load(&mut r, CHAIN);
        r.reason();
        assert!(lines(r.inferred()).contains("ex:a ex:anc ex:d"));

        // Removing what is not a base triple (here an inferred one) changes nothing.
        r.remove(
            parse("ex:a ex:anc ex:d . ex:q ex:anc ex:r .")
                .iter()
                .map(Triple::as_ref),
        );
        assert_eq!(r.reason().rounds, 0);

        r.remove(parse("ex:b ex:anc ex:c .").iter().map(Triple::as_ref));
        let s = r.reason();
        assert_eq!(
            r.counters().delta[0],
            2 + 1,
            "a full run starts from the base and the facts"
        );
        assert_eq!((s.base, s.inferred), (2, 1));
        assert_eq!(
            lines(r.triples()),
            set("ex:a ex:anc ex:b\nex:c ex:anc ex:d\nowl:Thing rdf:type owl:Class")
        );

        // Put back: the same closure as before the removal.
        r.insert(parse("ex:b ex:anc ex:c .").iter().map(Triple::as_ref));
        let s = r.reason();
        assert_eq!((s.base, s.inferred), (3, 4));
        assert_eq!(r.counters().delta[0], 1, "incremental again");
    }

    #[test]
    fn an_unchanged_base_costs_nothing() {
        let triples = parse(INPUT);
        let mut r = toy();
        assert!(!r.set_base(triples.iter().map(Triple::as_ref)));
        let first = r.reason();
        assert!(first.rounds > 0);

        // The same base, in another order and with duplicates: no round.
        let again: Vec<&Triple> = triples.iter().rev().chain(&triples).collect();
        assert!(!r.set_base(again.iter().map(|t| t.as_ref())));
        assert_eq!(r.reason(), Summary { rounds: 0, ..first });
        assert_eq!(r.counters().rounds(), 0);

        // A base that only grew runs incrementally.
        let more = parse("ex:e ex:anc ex:f .");
        assert!(!r.set_base(triples.iter().chain(&more).map(Triple::as_ref)));
        let grown = r.reason();
        assert_eq!(r.counters().delta[0], 1);
        assert_eq!(grown.base, first.base + 1);

        // A base that lost a triple runs in full.
        assert!(r.set_base(triples.iter().map(Triple::as_ref)));
        assert_eq!(r.reason(), first);
    }

    #[test]
    fn generalized_triples_feed_rules_but_stay_inside() {
        let mut r = toy();
        load(&mut r, INPUT);
        let s = r.reason();
        let closure = lines(r.triples());
        // `"five" ex:valueOf ex:x` has a literal subject: kept inside, where
        // it gives `ex:x ex:seen "five"`.
        assert!(closure.contains("ex:x ex:seen \"five\""), "{closure:#?}");
        assert!(!closure.iter().any(|t| t.starts_with('"')), "{closure:#?}");
        // `ex:x ex:link ex:x` is RDF; `ex:y "no" ex:y` and `ex:y _:p ex:y` are not.
        assert!(closure.contains("ex:x ex:link ex:x"));
        assert!(
            !closure
                .iter()
                .any(|t| t.starts_with("ex:y \"no\"") || t.starts_with("ex:y _:"))
        );
        let hidden = r.closure.len() - closure.len();
        assert_eq!(hidden, 3, "one literal subject, two predicates that are not IRIs");
        // The summary and the reads agree.
        assert_eq!(s.inferred, r.inferred().count());
        assert_eq!(s.base + s.inferred, closure.len());
        let mut out = Vec::new();
        r.write(&mut out, Format::NTriples, false).unwrap();
        assert_eq!(String::from_utf8(out).unwrap().lines().count(), closure.len());

        // Counted the same way by an incremental run.
        load(&mut r, "ex:z ex:value \"six\" . ex:z ex:predOf \"also no\" .");
        let s = r.reason();
        assert_eq!(s.inferred, r.inferred().count());
        assert_eq!(r.closure.len() - r.triples().count(), 5);
    }

    #[test]
    fn findings_are_deduplicated() {
        let mut r = toy();
        load(&mut r, INPUT);
        let s = r.reason();
        // ex:x, ex:y and _:n are in ex:Bad (through ex:C3): each reported
        // in several rounds, twice a round.
        let reported: Vec<String> = r.diagnostics().iter().map(|d| d.terms[0].to_string()).collect();
        assert_eq!(s.diagnostics, reported.len());
        let mut unique = reported.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(reported.len(), unique.len(), "{reported:?}");
        assert_eq!(s.diagnostics, 3, "{reported:?}");

        // An incremental run reports the old ones again, and one more.
        load(&mut r, "ex:w a ex:C1 .");
        assert_eq!(r.reason().diagnostics, 4);
        // A removal that takes ex:Bad away takes them all.
        r.remove(parse("ex:C3 rdfs:subClassOf ex:Bad .").iter().map(Triple::as_ref));
        assert_eq!(r.reason().diagnostics, 0);
        assert!(r.diagnostics().is_empty());
    }

    #[test]
    fn write_sorts_by_subject_and_can_leave_the_base_out() {
        let mut r = toy();
        load(&mut r, CHAIN);
        r.reason();
        let written = |inferred_only| {
            let mut out = Vec::new();
            r.write(&mut out, Format::NTriples, inferred_only).unwrap();
            String::from_utf8(out).unwrap()
        };
        let all = written(false);
        let read: Vec<Triple> = oxttl::NTriplesParser::new()
            .for_slice(all.as_bytes())
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(lines(read.iter().cloned()), lines(r.triples()));
        // Sorted by subject id: each subject's lines are together.
        let subjects: Vec<&str> = all.lines().map(|l| l.split(' ').next().unwrap()).collect();
        let mut groups = subjects.clone();
        groups.dedup();
        let distinct: BTreeSet<&str> = subjects.iter().copied().collect();
        assert_eq!(groups.len(), distinct.len(), "{all}");

        let inferred: Vec<Triple> = oxttl::NTriplesParser::new()
            .for_slice(written(true).as_bytes())
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(lines(inferred.into_iter()), lines(r.inferred()));
    }

    /// The toy rules under two switches: `transitive` under
    /// `literal-values` (off by default), with a fact, and `typing` under
    /// `checks`.
    static SWITCHED: &[Family] = &[
        Family {
            name: "transitive",
            switch: Some(Switch::LiteralValues),
            rules: transitive,
            facts: &[[vocab::OWL_NOTHING, vocab::RDF_TYPE, vocab::OWL_CLASS]],
        },
        Family {
            name: "typing",
            switch: Some(Switch::Checks),
            rules: typing,
            facts: &[[vocab::OWL_THING, vocab::RDF_TYPE, vocab::OWL_CLASS]],
        },
    ];

    #[test]
    fn a_family_runs_and_states_its_facts_only_when_its_switch_is_on() {
        let mut r = Reasoner::with_families(SWITCHED);
        r.terms.intern_iri("http://example.org/anc");
        load(&mut r, CHAIN);
        let s = r.reason();
        assert_eq!((s.base, s.inferred), (3, 1));
        assert_eq!(lines(r.inferred()), set("owl:Thing rdf:type owl:Class"));
        assert_eq!(r.counters().families, ["typing"]);

        // Turned on: a full run, with the family's fact.
        r.set_switches(Switches::default().on(Switch::LiteralValues));
        let s = r.reason();
        assert_eq!((s.base, s.inferred), (3, 3 + 2));
        assert_eq!(r.counters().delta[0], 3 + 2, "the base and both facts");
        assert!(lines(r.inferred()).contains("owl:Nothing rdf:type owl:Class"));
        assert_eq!(r.counters().families, ["transitive", "typing"]);

        // Both off: the base alone.
        r.set_switches(Switches::default().off(Switch::Checks));
        let s = r.reason();
        assert_eq!((s.base, s.inferred), (3, 0));
        assert!(r.counters().families.is_empty());
    }

    /// A seeded xorshift generator: reproducible, no dependency.
    struct Rng(u64);

    impl Rng {
        fn new(seed: u64) -> Self {
            Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
        }

        fn below(&mut self, n: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % n as u64) as usize
        }
    }

    /// The base's pairs keep their shape: the predicates strictly
    /// ascending, each with sorted pairs, none empty; and the vector holds
    /// at most a quarter of spare room.
    fn well_formed(pairs: &ByPredicate) {
        assert!(pairs.starts.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 < w[1].1));
        assert!(pairs.starts.first().is_none_or(|&(_, start)| start == 0));
        for (p, run) in pairs.iter() {
            assert!(!run.is_empty(), "predicate {p} has no pairs");
            assert!(run.windows(2).all(|w| w[0] < w[1]), "predicate {p}");
        }
        assert_eq!(pairs.ids.len() % 2, 0);
        let len = pairs.ids.len();
        assert!(
            pairs.ids.capacity() <= len + len / 4,
            "{} of {len}",
            pairs.ids.capacity()
        );
    }

    #[test]
    fn the_base_merges_its_recent_triples() {
        let mut base = Base::default();
        let t = |i: u32| [i, i % 7, i % 5];
        // Small batches stay in the hash set until it passes the bound.
        for i in 0..MERGE_AT_LEAST as u32 {
            let new = base.fresh(vec![t(i), t(i)]);
            assert_eq!(new.len(), 1);
            base.extend(new);
        }
        assert_eq!((base.merged.len(), base.recent.len()), (0, MERGE_AT_LEAST));
        base.extend(base.fresh(vec![t(MERGE_AT_LEAST as u32)]));
        assert_eq!((base.merged.len(), base.recent.len()), (MERGE_AT_LEAST + 1, 0));
        assert_eq!(base.merged.starts.len(), 7);
        well_formed(&base.merged);
        assert!(base.fresh((0..=MERGE_AT_LEAST as u32).map(t).collect()).is_empty());
        // Removal finds triples in either part.
        base.extend(base.fresh(vec![t(10_000)]));
        assert_eq!(base.remove(vec![t(3), t(10_000), t(20_000), t(3)]), 2);
        assert_eq!(base.len(), MERGE_AT_LEAST);
        assert!(!base.contains(&t(3)) && !base.contains(&t(10_000)) && base.contains(&t(4)));
        assert_eq!(base.iter().count(), base.len());
        well_formed(&base.merged);
        // A predicate whose last pair goes is gone; one before every other
        // and one after are merged in at their places.
        base.extend(base.fresh(vec![[1, 100, 1], [2, 0, 2], [0, 0, 9]]));
        base.flush();
        well_formed(&base.merged);
        assert_eq!(base.merged.starts.len(), 8);
        assert_eq!(base.remove(vec![[1, 100, 1]]), 1);
        assert_eq!(base.merged.starts.len(), 7);
        assert!(base.contains(&[2, 0, 2]) && base.contains(&[0, 0, 9]));
        well_formed(&base.merged);
    }

    /// A load's triples become the pairs, with the room of a parse
    /// buffer given back (here half of it is spare), and the same pairs as
    /// an addition to a base gives.
    #[test]
    fn a_load_becomes_pairs() {
        let mut triples: Vec<[u32; 3]> = Vec::with_capacity(40_000);
        triples.extend((0..20_000u32).map(|i| [i % 1_000, i % 3, i / 3]));
        let pairs = ByPredicate::from_triples(triples);
        assert_eq!(pairs.len(), 20_000);
        assert_eq!(pairs.iter().map(|(p, _)| p).collect::<Vec<_>>(), [0, 1, 2]);
        well_formed(&pairs);
        let mut base = Base::default();
        base.extend(base.fresh((0..20_000u32).map(|i| [i % 1_000, i % 3, i / 3]).collect()));
        assert_eq!(base.merged, pairs);
    }

    /// The base answers as a sorted set of triples (the base before it was
    /// kept by predicate) does, on random additions and removals, small
    /// and large (a load into an empty base, and into one with pairs), and
    /// holds 8 bytes a merged triple and at most a quarter more.
    #[test]
    fn the_base_answers_as_a_set_of_triples() {
        for seed in 0..12u64 {
            let mut rng = Rng::new(seed);
            let triple = |rng: &mut Rng, predicates: usize| {
                [rng.below(500), 20 + rng.below(predicates), rng.below(500)].map(|id| id as u32)
            };
            let mut base = Base::default();
            let mut model: BTreeSet<[u32; 3]> = BTreeSet::new();
            for step in 0..40 {
                // Some steps add many triples (a load), most add or remove a
                // few.
                let large = step % 13 == 0;
                let (count, predicates) = if large { (30_000, 6) } else { (rng.below(300), 12) };
                if large || rng.below(3) > 0 {
                    let batch: Vec<[u32; 3]> = (0..count).map(|_| triple(&mut rng, predicates)).collect();
                    let fresh = base.fresh(batch.clone());
                    let expected: BTreeSet<[u32; 3]> = batch.iter().copied().filter(|t| !model.contains(t)).collect();
                    let mut sorted: Vec<[u32; 3]> = expected.iter().copied().collect();
                    sorted.sort_unstable_by_key(by_predicate);
                    assert_eq!(fresh, sorted, "seed {seed}, step {step}");
                    model.extend(expected);
                    base.extend(fresh);
                } else {
                    let mut gone: Vec<[u32; 3]> = (0..count).map(|_| triple(&mut rng, predicates)).collect();
                    gone.extend(model.iter().filter(|_| rng.below(4) == 0).copied());
                    let removed = gone
                        .iter()
                        .filter(|t| model.contains(*t))
                        .collect::<BTreeSet<_>>()
                        .len();
                    assert_eq!(base.remove(gone.clone()), removed, "seed {seed}, step {step}");
                    for t in &gone {
                        model.remove(t);
                    }
                }
                assert_eq!(base.len(), model.len(), "seed {seed}, step {step}");
                let mut all: Vec<[u32; 3]> = base.iter().collect();
                all.sort_unstable();
                assert_eq!(
                    all,
                    model.iter().copied().collect::<Vec<_>>(),
                    "seed {seed}, step {step}"
                );
                for _ in 0..50 {
                    let t = triple(&mut rng, 14);
                    assert_eq!(base.contains(&t), model.contains(&t), "seed {seed}, step {step}");
                }
                // A copy merged, then the base itself: the same pairs.
                let copy = base.sorted_all().into_owned();
                well_formed(&base.merged);
                if step % 5 == 0 {
                    assert_eq!(base.flush(), &copy, "seed {seed}, step {step}");
                    assert!(base.recent.is_empty());
                    well_formed(&base.merged);
                }
                let flat: Vec<[u32; 3]> = copy
                    .iter()
                    .flat_map(|(p, pairs)| pairs.iter().map(move |&[s, o]| [s, p, o]))
                    .collect();
                let mut expected: Vec<[u32; 3]> = model.iter().copied().collect();
                expected.sort_unstable_by_key(by_predicate);
                assert_eq!(flat, expected, "seed {seed}, step {step}");
            }
        }
    }

    #[test]
    fn added_is_the_difference_of_a_superset() {
        let base = |triples: &[[u32; 3]]| ByPredicate::from_triples(triples.to_vec());
        let (a, b, c, d) = ([1, 1, 1], [2, 1, 2], [3, 3, 3], [0, 2, 0]);
        assert_eq!(added(&base(&[a, c]), &base(&[a, b, c])), Some(vec![b]));
        assert_eq!(added(&base(&[]), &base(&[a])), Some(vec![a]));
        assert_eq!(added(&base(&[a, b]), &base(&[a, b])), Some(vec![]));
        assert_eq!(added(&base(&[a, b]), &base(&[a, c])), None);
        assert_eq!(added(&base(&[c]), &base(&[a, b])), None);
        // By predicate: a new predicate before, between and after.
        assert_eq!(added(&base(&[d]), &base(&[a, d, c])), Some(vec![a, c]));
        assert_eq!(added(&base(&[a, c]), &base(&[a, d, c])), Some(vec![d]));
        assert_eq!(added(&base(&[a, d]), &base(&[a, b, c])), None);
        assert_eq!(added(&base(&[a, d]), &base(&[d])), None);
    }

    /// Random insertions, removals and new bases give, run by run, the
    /// summary, the closure (in the order the reads give it), the inferred
    /// triples and the diagnostics of a fresh reasoner given the same base
    /// at once; on one thread and on four with every round parallel.
    #[test]
    fn edits_give_the_closure_of_their_base() {
        let names = ["anc", "value", "valueOf", "predOf", "q"];
        let classes = ["C1", "C2", "C3", "Bad"];
        for threads in [Threads::ONE, Threads::exactly(4).every_round()] {
            for seed in 0..6u64 {
                let mut rng = Rng::new(seed);
                let triple = |rng: &mut Rng| {
                    if rng.below(4) == 0 {
                        if rng.below(2) == 0 {
                            let (c, d) = (classes[rng.below(4)], classes[rng.below(4)]);
                            format!("ex:{c} rdfs:subClassOf ex:{d} .")
                        } else {
                            let (i, c) = (rng.below(8), classes[rng.below(4)]);
                            format!("ex:i{i} a ex:{c} .")
                        }
                    } else {
                        let (i, p, j) = (rng.below(8), names[rng.below(names.len())], rng.below(8));
                        format!("ex:i{i} ex:{p} ex:i{j} .")
                    }
                };
                let mut r = toy();
                r.set_threads(threads);
                let mut model: BTreeSet<String> = BTreeSet::new();
                for step in 0..30 {
                    let edit: Vec<String> = (0..1 + rng.below(6)).map(|_| triple(&mut rng)).collect();
                    let triples = parse(&edit.join("\n"));
                    match rng.below(5) {
                        0 | 1 => {
                            r.insert(triples.iter().map(Triple::as_ref));
                            model.extend(edit);
                        }
                        2 => {
                            // Known triples as well as new ones.
                            let known: Vec<String> = model.iter().filter(|_| rng.below(3) == 0).cloned().collect();
                            r.remove(parse(&known.join("\n")).iter().map(Triple::as_ref));
                            r.remove(triples.iter().map(Triple::as_ref));
                            for t in known.iter().chain(&edit) {
                                model.remove(t);
                            }
                        }
                        3 => {
                            r.insert(triples.iter().map(Triple::as_ref));
                            model.extend(edit);
                            let kept: Vec<String> = model.iter().filter(|_| rng.below(8) > 0).cloned().collect();
                            r.set_base(parse(&kept.join("\n")).iter().map(Triple::as_ref));
                            model = kept.into_iter().collect();
                        }
                        _ => {
                            let all: Vec<String> = model.iter().cloned().chain(edit.clone()).collect();
                            r.set_base(parse(&all.join("\n")).iter().map(Triple::as_ref));
                            model.extend(edit);
                        }
                    }
                    let got = r.reason();
                    let mut fresh = toy();
                    fresh.insert(
                        parse(&model.iter().cloned().collect::<Vec<_>>().join("\n"))
                            .iter()
                            .map(Triple::as_ref),
                    );
                    let expected = fresh.reason();
                    let context = format!("{threads:?}, seed {seed}, step {step}");
                    assert_eq!(
                        (got.base, got.inferred, got.diagnostics),
                        (expected.base, expected.inferred, expected.diagnostics),
                        "{context}"
                    );
                    assert_eq!(lines(r.triples()), lines(fresh.triples()), "{context}");
                    assert_eq!(lines(r.inferred()), lines(fresh.inferred()), "{context}");
                    assert_eq!(findings(&r), findings(&fresh), "{context}");
                    assert_eq!(r.base.len(), model.len(), "{context}");
                }
            }
        }
    }

    /// A run gives the same closure (in the order the reads give it), the
    /// same diagnostics, summary and counters on one thread as on all cores
    /// with every round in parallel, through merges, a restart and
    /// findings, in full and incremental runs.
    #[test]
    fn the_thread_count_changes_nothing() {
        let text = "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C .
            ex:A owl:disjointWith ex:D .
            ex:p rdfs:domain ex:A ; rdfs:range ex:B .
            ex:x ex:p ex:y . ex:y ex:p ex:z . ex:x a ex:D .
            ex:x owl:sameAs ex:x2 . ex:x2 owl:sameAs ex:x3 .
            ex:q owl:sameAs ex:p .
            ex:z ex:age \"twelve\"^^<http://www.w3.org/2001/XMLSchema#integer> .";
        // A merge of a vocabulary term restarts the run.
        let more = "ex:w ex:p ex:x . ex:z owl:sameAs ex:w . ex:C owl:sameAs ex:E .
            ex:sub owl:sameAs rdfs:subClassOf . ex:E ex:sub ex:F .";
        let state = |r: &Reasoner| {
            (
                r.triples().collect::<Vec<_>>(),
                r.diagnostics().to_vec(),
                r.counters().counts(),
            )
        };
        let mut runs = Vec::new();
        for threads in [
            Threads::ONE,
            Threads::ALL.every_round(),
            Threads::exactly(3).every_round(),
            Threads::ALL,
        ] {
            let mut r = Reasoner::with_switches(Switches::complete());
            r.set_threads(threads);
            assert_eq!(r.threads(), threads);
            load(&mut r, text);
            let full = r.reason();
            let after_full = state(&r);
            load(&mut r, more);
            let incremental = r.reason();
            runs.push((full, after_full, incremental, state(&r)));
        }
        assert!(runs[0].1.1.len() > 1, "the input has findings: {:?}", runs[0].1.1);
        for run in &runs[1..] {
            assert_eq!(run, &runs[0]);
        }
    }

    /// A full run hands each table its base triples and the facts as one
    /// sorted batch of exactly their size: in the first round each table
    /// holds them and nothing else, in one part with no spare room. A fact
    /// that is in the base too is held once. On one thread, and on three
    /// with every round parallel.
    #[test]
    fn a_full_run_starts_with_exact_tables() {
        for threads in [Threads::ONE, Threads::exactly(3).every_round()] {
            let mut r = toy();
            r.set_threads(threads);
            load(&mut r, INPUT);
            load(&mut r, "owl:Thing a owl:Class .");
            let facts = TOY.iter().flat_map(|f| f.facts).map(|&[s, p, o]| [s.0, p.0, o.0]);
            let mut expected: Vec<(u32, u32, u32)> = r.base.iter().chain(facts).map(|[s, p, o]| (p, s, o)).collect();
            expected.sort_unstable();
            expected.dedup();
            assert_eq!(expected.len(), r.base.len(), "the fact is in the base");
            let families: Vec<&'static Family> = TOY.iter().collect();
            r.start(&families, equality::canon());
            assert_eq!(r.closure.buffered(), 0, "nothing goes through the round buffer");
            let mut got = Vec::new();
            r.closure.run(|round| {
                if round.number() == 1 {
                    for (p, table) in round.store().tables() {
                        let so = table.by_subject();
                        assert!(so.stable().is_empty());
                        let recent = so.recent();
                        assert_eq!(recent.bytes(), recent.len() * size_of::<Pair>(), "{p}");
                        got.extend(recent.iter().map(|&(s, o)| (p, s, o)));
                    }
                }
            });
            assert_eq!(got, expected);
        }
    }

    /// On every core (the default), a full run on a large base runs its
    /// large rounds in parallel, and a one-triple change then runs every
    /// round on the caller's thread.
    #[test]
    fn a_small_change_waits_on_no_thread_pool() {
        use std::fmt::Write as _;
        let mut text = String::from("ex:A rdfs:subClassOf ex:B . ex:p rdfs:domain ex:A .\n");
        for i in 0..crate::engine::PARALLEL_FROM {
            let _ = writeln!(text, "ex:i{i} ex:p ex:i{} .", i + 1);
        }
        let mut r = Reasoner::new();
        assert_eq!(r.threads(), Threads::ALL);
        load(&mut r, &text);
        r.reason();
        let engaged = r.store().engaged();
        if Workers::all().threads() > 1 {
            assert!(engaged > 0, "the full run's first round is large");
        }
        load(&mut r, "ex:new ex:p ex:i0 .");
        let s = r.reason();
        assert!(s.rounds > 1);
        assert_eq!(r.store().engaged(), engaged, "no round of the change ran in parallel");
    }
}
