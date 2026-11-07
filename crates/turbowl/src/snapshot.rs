//! Snapshots (ticket 19): a reasoner's whole state as one byte string, saved
//! and loaded back without parsing or reasoning.
//!
//! A loaded reasoner is the saved one: the same dictionary (every term at
//! the same id), base, stored closure (each table's batches as they were),
//! canon, findings and switches, and the same run due next. Its reads give
//! the same triples in the same order, and `add`, `remove` and `reason`
//! then do what they would have done on the saved reasoner. Saving the
//! loaded reasoner gives the same bytes.
//!
//! # The format, version 2
//!
//! Integers are little-endian: an *id* is a `u32` term id, a *count* a
//! `u64`. A *string8* is a `u8` byte length then UTF-8; a *string16* the
//! same with a `u16` length.
//!
//! | Bytes | Field |
//! |---|---|
//! | 8 | magic: `89 54 75 72 62 4F 57 4C` (`\x89TurbOWL`; the high first byte catches a transfer as text) |
//! | 4 | `u32` format version: 2 |
//! | 8 | `u64` length of the whole snapshot, in bytes, checksum included |
//! | 2 + n | string16: the producer, `turbowl <version>` |
//! | ... | the sections below, in this order, each a 4-byte ASCII tag, a `u64` payload length, then the payload |
//! | 8 | `u64` checksum: XXH64, seed 0, of every byte before it |
//!
//! The sections:
//!
//! - `CONF`, the configuration: a `u8` count of the switches that are on,
//!   then each one's name as a string8 ([`Switch::name`]); a `u8` state:
//!   0 the closure is the base's, 1 base triples were added since the last
//!   run (an incremental run is due), 2 a full run is due.
//! - `TERM`, the dictionary: a `u32` count of the documents loaded (the
//!   blank-node scopes handed out); a count of terms; a count of text bytes;
//!   then each term in id order: a `u8` kind (0 an IRI, 1 a blank node, 2 a
//!   literal with a datatype, 3 a language-tagged literal), a `u32` (the
//!   blank node's document, the literal's datatype id, the language tag's
//!   length in bytes; 0 for an IRI), a `u32` text length, then the text (a
//!   tagged literal's lexical form, then its tag). The first terms are the
//!   vocabulary and the rules' literals, exactly as every new dictionary
//!   holds them.
//! - `BASE`, the base triples, by predicate: a count of predicates, then
//!   each one, strictly ascending: its id, a count of its pairs (one or
//!   more), then its `(s, o)` pairs as two ids each, strictly ascending.
//!   (Version 1 held `[s, p, o]` triples here.)
//! - `CANO`, the canon of `owl:sameAs`: a `u32` count of the rules'
//!   constants; a count and the ids of the watched constants; a count of
//!   the merges made; a count and the ids of the representative of each id
//!   below it; a count of the classes of two or more, then for each, by
//!   ascending representative: the representative, a count and the ids of
//!   its members, in the canon's order.
//! - `STOR`, the stored closure: a `u64` count of the rounds the store has
//!   run; a count of predicate tables, then each one in the store's order:
//!   the predicate's id; its pairs by subject; a `u8`, 1 when the table is
//!   indexed by object too, then those pairs. Pairs are a `u8` count of
//!   batches, then each batch: a count, then `(u32, u32)` pairs, strictly
//!   ascending.
//! - `READ`, what the reads count and add: a count of the RDF triples the
//!   stored closure stands for, a count of those eq-ref adds, then a count
//!   and the ids of the representatives eq-ref adds `r owl:sameAs r` for.
//! - `FIND`, the findings: a count, then each one, in order: its code as a
//!   string8, a count and the ids of its terms.
//! - `QUEU`, the triples queued for the incremental run that is due: a
//!   count, then `[s, p, o]` ids, in the order they were queued.
//! - `DOCS`, the documents loaded (the keys of "the same document again"):
//!   a `u128` probe, the key of an empty Turtle document; a count, then
//!   each `u128` key, strictly ascending.
//!
//! # Loading checks everything
//!
//! A snapshot is input at the edge: the loader trusts nothing in it until
//! it is checked, and refuses anything else with an [`Error::Snapshot`]
//! (crate::Error::Snapshot). It never panics on any bytes. In order:
//!
//! 1. The magic, then the format version, then the length (a shorter file
//!    is truncated), then the checksum, then the producer: a snapshot of
//!    another TurbOWL version is refused, since its closure follows from
//!    that version's rules.
//! 2. Each section's tag and length, and that its payload is read to the
//!    end; every count fits in the bytes left before anything is allocated
//!    for it.
//! 3. The dictionary: each kind known, the text UTF-8, a tag's length on a
//!    character boundary, a document no later than the documents counted,
//!    a datatype that is an IRI of the dictionary, the first terms those
//!    of a new dictionary, no term twice.
//! 4. Every id below the dictionary's length; every sorted list strictly
//!    ascending.
//! 5. The canon: each class's members pointing at its representative, each
//!    id in at most one class, a class with a constant represented by its
//!    smallest member, the merges one fewer than the members of each class.
//! 6. The store: each predicate once; every batch non-empty, the batches
//!    of a part disjoint and each more than twice the next one's length (as
//!    the engine's tiers are); every stored id its own representative; an
//!    object index holding exactly the pairs by subject, flipped (the same
//!    count, and the same order-free 64-bit fingerprint of the set).
//! 7. The findings' codes known (the W3C rules that conclude `false`).
//! 8. The state: triples queued exactly when an incremental run is due; the
//!    RDF count at least the base when no full run is due.
//!
//! A term's text is not parsed again: an IRI or a language tag is taken as
//! written, as [`Reasoner::insert`](crate::Reasoner::insert) takes an
//! `oxrdf` term built unchecked. The checksum catches corruption, not a
//! forger: a crafted snapshot with a valid checksum can make the closure
//! wrong (an object index with the same fingerprint as its pairs), but not
//! make the reasoner panic or misbehave otherwise.
//!
//! # Choices
//!
//! - **The dictionary as terms, not as its hash table**: the format does
//!   not depend on FxHash or on the table's layout, which may change
//!   (tickets 16 and 18). Loading appends the terms in id order, then
//!   indexes them at once, inserting them in the order of their slots
//!   (`Dictionary::index_appended`), which finds a term listed twice; on
//!   lubm-50's 2.2 M terms that took 54 ms where interning them one by one
//!   took 120 ms (2026-11-06, load average 3).
//! - **The closure as stored**: each table's batches as they are, so the
//!   reads keep their order and the next runs the same tiers and merges.
//!   Between runs every table's recent part is empty (a run ends at a
//!   fixpoint), so only the stable batches are written.
//! - **Recomputed, not stored**: the diagnostics (from the findings), the
//!   base's hash set (the base is written merged, by predicate), the
//!   store's indexes.
//! - **Not stored**: the threads (a setting of the machine: a loaded
//!   reasoner uses [`Threads::ALL`](crate::Threads::ALL)) and the counters
//!   of the last run.
//! - **The document keys** are kept, so that loading a document of the
//!   snapshot again changes nothing, as on the saved reasoner. A key hashes
//!   a document with this build's hashers; the probe, the key of a fixed
//!   document, tells whether the loading build hashes as the saving one
//!   did. If not, the keys are dropped, and the loaded reasoner forgets
//!   which documents were loaded, as after a removal.
//! - **Files behind the `io` feature, bytes always**: the reasoner writes a
//!   snapshot to any [`Write`] and reads one from a byte slice, also in
//!   WebAssembly. Saving to a path writes a temporary file beside it, syncs
//!   it and renames it over the path, so a crash leaves the old snapshot or
//!   the new one, never half of one ([`save`]).
//! - **XXH64** (Yann Collet's xxHash, 64-bit) as the checksum: a published
//!   algorithm with published test vectors, several GB/s in safe Rust, and
//!   no dependency. `std`'s `DefaultHasher` may change between Rust
//!   versions.
//!
//! Writing runs each section's encoder twice, once counting bytes (so the
//! length fields come first) and once writing, and streams: it holds 64 KiB
//! of output at a time, not the whole snapshot. Loading reads the whole
//! snapshot first, to check its checksum before trusting any of it, then
//! decodes and checks the base and each large batch in chunks on every
//! core ([`Workers`], past the engine's threshold for a parallel round).
//!
//! Measured on lubm-50 (6.6 M base and 10.3 M closure triples, 2026-11-06,
//! load average 3.6 on the 12-core machine of the tests), in version 1: a
//! snapshot of 312 MB (30 bytes a closure triple: 107 MB of terms, 79 MB of
//! base, 125 MB of stored pairs), saved in about 0.17 s and loaded in
//! 0.11 s, against 1.69 s to parse and reason. Version 2 holds the base in
//! 8 bytes a triple, not 12: about 26 MB less.

use crate::engine::{Canon, Pair, Relation, Store, Table, Variable, Workers, gallop};
use crate::reasoner::{ByPredicate, State};
use crate::rules::Finding;
use crate::term::{Dictionary, DocId, Kind, MAX_TERMS, Term, TermId, vocab};
use crate::{Switch, Switches};
use rustc_hash::FxHashSet;
use std::borrow::Cow;
use std::io::{self, Write};

/// The first bytes of every snapshot.
pub(crate) const MAGIC: [u8; 8] = *b"\x89TurbOWL";

/// The format version this build writes and reads.
pub(crate) const VERSION: u32 = 2;

/// Who wrote a snapshot: this build's name and version.
const PRODUCER: &str = concat!("turbowl ", env!("CARGO_PKG_VERSION"));

/// The magic, the version and the length.
const FIXED: usize = MAGIC.len() + 4 + 8;

/// The checksum at the end.
const TRAILER: usize = 8;

/// The bytes the writer holds before it hashes and writes them.
const CHUNK: usize = 1 << 16;

/// The codes a finding can have: the W3C rules that conclude `false` (the
/// table of [`Diagnostic`](crate::Diagnostic)). A loaded finding's code is
/// one of these; a rule that reports a new code adds it here, or saving it
/// fails its load (the oracle crate's round trips cover every code).
const CODES: &[&str] = &[
    "cax-adc",
    "cax-dw",
    "cls-com",
    "cls-maxc1",
    "cls-maxqc1",
    "cls-maxqc2",
    "cls-nothing2",
    "dt-not-type",
    "eq-diff1",
    "eq-diff2",
    "eq-diff3",
    "prp-adp",
    "prp-asyp",
    "prp-irp",
    "prp-npa1",
    "prp-npa2",
    "prp-pdw",
];

/// The sections, in the order they are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Conf,
    Terms,
    Base,
    Canon,
    Store,
    Reads,
    Findings,
    Queue,
    Documents,
}

const SECTIONS: [Section; 9] = [
    Section::Conf,
    Section::Terms,
    Section::Base,
    Section::Canon,
    Section::Store,
    Section::Reads,
    Section::Findings,
    Section::Queue,
    Section::Documents,
];

impl Section {
    fn tag(self) -> [u8; 4] {
        *match self {
            Section::Conf => b"CONF",
            Section::Terms => b"TERM",
            Section::Base => b"BASE",
            Section::Canon => b"CANO",
            Section::Store => b"STOR",
            Section::Reads => b"READ",
            Section::Findings => b"FIND",
            Section::Queue => b"QUEU",
            Section::Documents => b"DOCS",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Section::Conf => "CONF (the configuration)",
            Section::Terms => "TERM (the dictionary)",
            Section::Base => "BASE (the base triples)",
            Section::Canon => "CANO (the canon)",
            Section::Store => "STOR (the stored closure)",
            Section::Reads => "READ (the counts of the reads)",
            Section::Findings => "FIND (the findings)",
            Section::Queue => "QUEU (the queued triples)",
            Section::Documents => "DOCS (the documents loaded)",
        }
    }
}

/// A reasoner's state, borrowed for saving.
#[derive(Debug)]
pub(crate) struct Saved<'a> {
    pub(crate) switches: Switches,
    pub(crate) state: State,
    pub(crate) terms: &'a Dictionary,
    /// Merged, by predicate.
    pub(crate) base: Cow<'a, ByPredicate>,
    /// Settled: between runs ([`Store::is_settled`]).
    pub(crate) closure: &'a Store,
    pub(crate) rdf: usize,
    pub(crate) same_as_itself: &'a [u32],
    pub(crate) same_as_itself_rdf: usize,
    /// Sorted, deduplicated.
    pub(crate) findings: &'a [Finding],
    /// The key of the probe document, and the keys of the documents
    /// loaded, sorted (none without the `io` feature).
    pub(crate) probe: u128,
    pub(crate) documents: Vec<u128>,
}

/// A reasoner's state, loaded and checked.
#[derive(Debug)]
pub(crate) struct Restored {
    pub(crate) switches: Switches,
    pub(crate) state: State,
    pub(crate) terms: Dictionary,
    pub(crate) base: ByPredicate,
    /// With the queued triples queued.
    pub(crate) closure: Store,
    pub(crate) rdf: usize,
    pub(crate) same_as_itself: Vec<u32>,
    pub(crate) same_as_itself_rdf: usize,
    pub(crate) findings: Vec<Finding>,
    pub(crate) probe: u128,
    pub(crate) documents: Vec<u128>,
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

/// Writes `saved` as a snapshot to `out`.
pub(crate) fn write(saved: &Saved<'_>, out: impl Write) -> io::Result<()> {
    debug_assert!(saved.closure.is_settled(), "a snapshot is taken between runs");
    let mut sizes = [0u64; SECTIONS.len()];
    for (size, &section) in sizes.iter_mut().zip(&SECTIONS) {
        let mut count = Count(0);
        encode(&mut count, section, saved)?;
        *size = count.0;
    }
    let header = (FIXED + 2 + PRODUCER.len()) as u64;
    let total = header + sizes.iter().map(|s| 4 + 8 + s).sum::<u64>() + TRAILER as u64;

    let mut out = Out::new(out);
    out.put(&MAGIC)?;
    out.u32(VERSION)?;
    out.u64(total)?;
    out.str16(PRODUCER)?;
    for (&section, &size) in SECTIONS.iter().zip(&sizes) {
        out.put(&section.tag())?;
        out.u64(size)?;
        let before = out.written;
        encode(&mut out, section, saved)?;
        if out.written - before != size {
            return Err(io::Error::other(format!(
                "snapshot section {}: wrote {} bytes, counted {size}",
                section.name(),
                out.written - before
            )));
        }
    }
    debug_assert_eq!(out.written + TRAILER as u64, total);
    out.finish()
}

/// Where an encoder puts bytes: a counter or a writer.
trait Sink {
    fn put(&mut self, bytes: &[u8]) -> io::Result<()>;
    fn pairs(&mut self, pairs: &[Pair]) -> io::Result<()>;
    fn ids(&mut self, ids: &[u32]) -> io::Result<()>;
    fn triples(&mut self, triples: &[[u32; 3]]) -> io::Result<()>;

    fn u8(&mut self, v: u8) -> io::Result<()> {
        self.put(&[v])
    }

    fn u32(&mut self, v: u32) -> io::Result<()> {
        self.put(&v.to_le_bytes())
    }

    fn u64(&mut self, v: u64) -> io::Result<()> {
        self.put(&v.to_le_bytes())
    }

    fn u128(&mut self, v: u128) -> io::Result<()> {
        self.put(&v.to_le_bytes())
    }

    fn count(&mut self, n: usize) -> io::Result<()> {
        self.u64(n as u64)
    }

    fn str8(&mut self, s: &str) -> io::Result<()> {
        let len = u8::try_from(s.len()).map_err(|_| io::Error::other(format!("{s:?} is too long")))?;
        self.u8(len)?;
        self.put(s.as_bytes())
    }

    fn str16(&mut self, s: &str) -> io::Result<()> {
        let len = u16::try_from(s.len()).map_err(|_| io::Error::other(format!("{s:?} is too long")))?;
        self.put(&len.to_le_bytes())?;
        self.put(s.as_bytes())
    }
}

/// Counts the bytes an encoder puts.
struct Count(u64);

impl Sink for Count {
    fn put(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.0 += bytes.len() as u64;
        Ok(())
    }

    fn pairs(&mut self, pairs: &[Pair]) -> io::Result<()> {
        self.0 += 8 * pairs.len() as u64;
        Ok(())
    }

    fn ids(&mut self, ids: &[u32]) -> io::Result<()> {
        self.0 += 4 * ids.len() as u64;
        Ok(())
    }

    fn triples(&mut self, triples: &[[u32; 3]]) -> io::Result<()> {
        self.0 += 12 * triples.len() as u64;
        Ok(())
    }
}

/// Writes what an encoder puts, in chunks, and hashes it.
struct Out<W: Write> {
    out: W,
    hash: Xxh64,
    buf: Vec<u8>,
    /// Bytes put so far.
    written: u64,
}

impl<W: Write> Out<W> {
    fn new(out: W) -> Self {
        Self {
            out,
            hash: Xxh64::new(0),
            buf: Vec::with_capacity(CHUNK + 64),
            written: 0,
        }
    }

    /// Hashes and writes what is held.
    fn drain(&mut self) -> io::Result<()> {
        self.hash.update(&self.buf);
        self.out.write_all(&self.buf)?;
        self.buf.clear();
        Ok(())
    }

    /// Puts `items`, each `width` bytes as `encode` writes it.
    fn bulk<T: Copy>(&mut self, items: &[T], width: usize, encode: impl Fn(&mut Vec<u8>, T)) -> io::Result<()> {
        for chunk in items.chunks(CHUNK / width) {
            self.buf.reserve(chunk.len() * width);
            for &item in chunk {
                encode(&mut self.buf, item);
            }
            self.written += (chunk.len() * width) as u64;
            if self.buf.len() >= CHUNK {
                self.drain()?;
            }
        }
        Ok(())
    }

    /// Writes the rest, then the checksum.
    fn finish(mut self) -> io::Result<()> {
        self.drain()?;
        let sum = self.hash.digest();
        self.out.write_all(&sum.to_le_bytes())?;
        self.out.flush()
    }
}

impl<W: Write> Sink for Out<W> {
    fn put(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.written += bytes.len() as u64;
        if bytes.len() >= CHUNK {
            self.drain()?;
            self.hash.update(bytes);
            return self.out.write_all(bytes);
        }
        self.buf.extend_from_slice(bytes);
        if self.buf.len() >= CHUNK {
            self.drain()?;
        }
        Ok(())
    }

    fn pairs(&mut self, pairs: &[Pair]) -> io::Result<()> {
        self.bulk(pairs, 8, |buf, (a, b)| {
            buf.extend_from_slice(&a.to_le_bytes());
            buf.extend_from_slice(&b.to_le_bytes());
        })
    }

    fn ids(&mut self, ids: &[u32]) -> io::Result<()> {
        self.bulk(ids, 4, |buf, id| buf.extend_from_slice(&id.to_le_bytes()))
    }

    fn triples(&mut self, triples: &[[u32; 3]]) -> io::Result<()> {
        self.bulk(triples, 12, |buf, [s, p, o]| {
            buf.extend_from_slice(&s.to_le_bytes());
            buf.extend_from_slice(&p.to_le_bytes());
            buf.extend_from_slice(&o.to_le_bytes());
        })
    }
}

/// Puts one section's payload.
fn encode(s: &mut impl Sink, section: Section, saved: &Saved<'_>) -> io::Result<()> {
    match section {
        Section::Conf => {
            let on: Vec<Switch> = Switch::ALL.into_iter().filter(|&w| saved.switches.is_on(w)).collect();
            s.u8(on.len() as u8)?;
            for switch in on {
                s.str8(switch.name())?;
            }
            s.u8(state_code(saved.state))
        }
        Section::Terms => encode_terms(s, saved.terms),
        Section::Base => {
            let predicates: Vec<(u32, &[[u32; 2]])> = saved.base.iter().collect();
            s.count(predicates.len())?;
            for (p, pairs) in predicates {
                s.u32(p)?;
                s.count(pairs.len())?;
                s.ids(pairs.as_flattened())?;
            }
            Ok(())
        }
        Section::Canon => encode_canon(s, saved.closure.canon()),
        Section::Store => {
            s.u64(saved.closure.rounds() as u64)?;
            let tables: Vec<(u32, &Table)> = saved.closure.tables().collect();
            s.count(tables.len())?;
            for (p, table) in tables {
                s.u32(p)?;
                encode_variable(s, table.by_subject())?;
                match table.by_object() {
                    None => s.u8(0)?,
                    Some(os) => {
                        s.u8(1)?;
                        encode_variable(s, os)?;
                    }
                }
            }
            Ok(())
        }
        Section::Reads => {
            s.u64(saved.rdf as u64)?;
            s.u64(saved.same_as_itself_rdf as u64)?;
            s.count(saved.same_as_itself.len())?;
            s.ids(saved.same_as_itself)
        }
        Section::Findings => {
            s.count(saved.findings.len())?;
            for f in saved.findings {
                s.str8(f.code)?;
                s.count(f.terms.len())?;
                s.ids(&f.terms)?;
            }
            Ok(())
        }
        Section::Queue => {
            // A full run starts from a new store: what an earlier change
            // queued in this one is never read.
            let queued: Vec<[u32; 3]> = match saved.state {
                State::Added => saved.closure.queued().map(|(s, p, o)| [s, p, o]).collect(),
                State::Current | State::Full => Vec::new(),
            };
            s.count(queued.len())?;
            s.triples(&queued)
        }
        Section::Documents => {
            s.u128(saved.probe)?;
            s.count(saved.documents.len())?;
            for &key in &saved.documents {
                s.u128(key)?;
            }
            Ok(())
        }
    }
}

fn state_code(state: State) -> u8 {
    match state {
        State::Current => 0,
        State::Added => 1,
        State::Full => 2,
    }
}

const KIND_IRI: u8 = 0;
const KIND_BLANK: u8 = 1;
const KIND_TYPED: u8 = 2;
const KIND_LANG: u8 = 3;
/// A triple term: its text is its N-Triples form (`Dictionary` has the rule).
#[cfg(feature = "rdf-12")]
const KIND_TRIPLE: u8 = 4;

fn encode_terms(s: &mut impl Sink, terms: &Dictionary) -> io::Result<()> {
    let n = terms.len();
    let text: usize = (0..n)
        .map(|i| match terms.term(TermId(i as u32)) {
            Term::Iri(t) | Term::Blank { label: t, .. } => t.len(),
            Term::Literal { lexical, language, .. } => lexical.len() + language.map_or(0, str::len),
            #[cfg(feature = "rdf-12")]
            Term::Triple(triple) => triple.to_string().len(),
        })
        .sum();
    s.u32(terms.documents())?;
    s.count(n)?;
    s.count(text)?;
    for i in 0..n {
        #[cfg(feature = "rdf-12")]
        let written: String;
        let (kind, aux, head, tail) = match terms.term(TermId(i as u32)) {
            Term::Iri(iri) => (KIND_IRI, 0, iri, ""),
            Term::Blank { doc, label } => (KIND_BLANK, doc.0, label, ""),
            Term::Literal {
                lexical,
                datatype,
                language: None,
            } => (KIND_TYPED, datatype.0, lexical, ""),
            Term::Literal {
                lexical,
                language: Some(tag),
                ..
            } => (KIND_LANG, tag.len() as u32, lexical, tag),
            #[cfg(feature = "rdf-12")]
            Term::Triple(triple) => {
                written = triple.to_string();
                (KIND_TRIPLE, 0, written.as_str(), "")
            }
        };
        s.u8(kind)?;
        s.u32(aux)?;
        s.u32((head.len() + tail.len()) as u32)?;
        s.put(head.as_bytes())?;
        s.put(tail.as_bytes())?;
    }
    Ok(())
}

fn encode_canon(s: &mut impl Sink, canon: &Canon) -> io::Result<()> {
    s.u32(canon.constants())?;
    s.count(canon.watched().len())?;
    s.ids(canon.watched())?;
    s.count(canon.merges())?;
    s.count(canon.rep().len())?;
    s.ids(canon.rep())?;
    let mut classes: Vec<(u32, &[u32])> = canon.classes().collect();
    classes.sort_unstable_by_key(|c| c.0);
    s.count(classes.len())?;
    for (rep, members) in classes {
        s.u32(rep)?;
        s.count(members.len())?;
        s.ids(members)?;
    }
    Ok(())
}

fn encode_variable(s: &mut impl Sink, v: &Variable<Pair>) -> io::Result<()> {
    debug_assert!(v.recent().is_empty() && !v.is_active(), "settled between runs");
    let stable = v.stable();
    // At most 64 batches: each is more than twice the next one's length.
    s.u8(stable.len() as u8)?;
    for batch in stable {
        s.count(batch.len())?;
        s.pairs(batch)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// The state a snapshot holds, checked; or why it is refused, as a
/// sentence.
pub(crate) fn read(bytes: &[u8], new_dictionary: impl FnOnce() -> Dictionary) -> Result<Restored, String> {
    if bytes.len() < MAGIC.len() || bytes[..MAGIC.len()] != MAGIC {
        return Err("not a TurbOWL snapshot".into());
    }
    let truncated = |total: Option<u64>| match total {
        Some(total) => format!("the snapshot is truncated: {} of its {total} bytes", bytes.len()),
        None => format!("the snapshot is truncated: {} bytes", bytes.len()),
    };
    let word = |at: usize, len: usize| bytes.get(at..at + len);
    let version = word(8, 4).ok_or_else(|| truncated(None))?;
    let version = u32::from_le_bytes(version.try_into().expect("4 bytes"));
    if version != VERSION {
        return Err(format!(
            "the snapshot has format version {version}; this TurbOWL reads version {VERSION}"
        ));
    }
    let total = word(12, 8).ok_or_else(|| truncated(None))?;
    let total = u64::from_le_bytes(total.try_into().expect("8 bytes"));
    let len = bytes.len() as u64;
    if len < total {
        return Err(truncated(Some(total)));
    }
    if len > total {
        return Err(format!(
            "the snapshot has {} bytes past its end (at byte {total})",
            len - total
        ));
    }
    if bytes.len() < FIXED + TRAILER {
        return Err(format!("the snapshot is malformed: a length of {total} bytes"));
    }
    let (body, sum) = bytes.split_at(bytes.len() - TRAILER);
    let sum = u64::from_le_bytes(sum.try_into().expect("8 bytes"));
    if xxh64(body, 0) != sum {
        return Err("the snapshot is damaged: its checksum does not match its contents".into());
    }

    let mut rest = Reader::new(&body[FIXED..], "the header");
    let producer = rest.str16().map_err(malformed)?;
    if producer != PRODUCER {
        return Err(format!(
            "the snapshot was saved by {producer}, and this is {PRODUCER}: a snapshot holds a closure of its version's rules"
        ));
    }
    // Large sections decode in chunks on every core, as large rounds run.
    restore(rest, new_dictionary, &Workers::all()).map_err(malformed)
}

fn malformed(detail: String) -> String {
    format!("the snapshot is malformed: {detail}")
}

/// The sections after the header, checked.
fn restore(
    mut rest: Reader<'_>,
    new_dictionary: impl FnOnce() -> Dictionary,
    workers: &Workers,
) -> Result<Restored, String> {
    let mut r = rest.section(Section::Conf)?;
    let (switches, state) = read_conf(&mut r)?;
    r.end()?;

    let mut r = rest.section(Section::Terms)?;
    let terms = read_terms(&mut r, new_dictionary())?;
    r.end()?;
    let n = terms.len();

    let mut r = rest.section(Section::Base)?;
    let base = read_base(&mut r, n, workers)?;
    r.end()?;
    let base_len = base.len();

    let mut r = rest.section(Section::Canon)?;
    let canon = read_canon(&mut r, n)?;
    r.end()?;

    let mut r = rest.section(Section::Store)?;
    let mut closure = read_store(&mut r, n, canon, workers)?;
    r.end()?;

    let mut r = rest.section(Section::Reads)?;
    let rdf = r.u64()?;
    let same_as_itself_rdf = r.u64()?;
    let count = r.count(4)?;
    let same_as_itself = r.ids(count, n)?;
    r.end()?;
    let (Ok(rdf), Ok(same_as_itself_rdf)) = (usize::try_from(rdf), usize::try_from(same_as_itself_rdf)) else {
        return Err(format!(
            "{}: a count of {rdf} or {same_as_itself_rdf} triples",
            Section::Reads.name()
        ));
    };

    let mut r = rest.section(Section::Findings)?;
    let findings = read_findings(&mut r, n)?;
    r.end()?;

    let mut r = rest.section(Section::Queue)?;
    let count = r.count(12)?;
    let queued = r.triples(count, n, workers)?;
    r.end()?;

    let mut r = rest.section(Section::Documents)?;
    let probe = r.u128()?;
    let count = r.count(16)?;
    let mut documents = Vec::with_capacity(count);
    for _ in 0..count {
        documents.push(r.u128()?);
    }
    if documents.windows(2).any(|w| w[0] >= w[1]) {
        return Err(r.bad("the keys are not strictly ascending".into()));
    }
    r.end()?;
    rest.end()?;

    // The state, against what the sections hold.
    if (state == State::Added) != !queued.is_empty() {
        return Err(format!(
            "{} triples are queued, and the state says an incremental run is {}due",
            queued.len(),
            if state == State::Added { "" } else { "not " }
        ));
    }
    let counted = match state {
        State::Current => Some(rdf),
        State::Added => Some(rdf.saturating_add(queued.len())),
        State::Full => None,
    };
    if counted.is_some_and(|c| c < base_len) {
        return Err(format!(
            "the closure stands for {rdf} RDF triples, fewer than the {base_len} base triples"
        ));
    }
    closure.extend(queued.iter().map(|&[s, p, o]| (s, p, o)));

    Ok(Restored {
        switches,
        state,
        terms,
        base,
        closure,
        rdf,
        same_as_itself,
        same_as_itself_rdf,
        findings,
        probe,
        documents,
    })
}

fn read_conf(r: &mut Reader<'_>) -> Result<(Switches, State), String> {
    let count = r.u8()?;
    let mut switches = Switches::complete();
    for s in Switch::ALL {
        switches = switches.off(s);
    }
    for _ in 0..count {
        let name = r.str8()?;
        let switch = Switch::from_name(name).ok_or_else(|| r.bad(format!("an unknown switch {name:?}")))?;
        if switches.is_on(switch) {
            return Err(r.bad(format!("the switch {name:?} is listed twice")));
        }
        switches = switches.on(switch);
    }
    let state = match r.u8()? {
        0 => State::Current,
        1 => State::Added,
        2 => State::Full,
        other => return Err(r.bad(format!("an unknown state {other}"))),
    };
    Ok((switches, state))
}

fn read_terms(r: &mut Reader<'_>, mut terms: Dictionary) -> Result<Dictionary, String> {
    let documents = r.u32()?;
    // Each term takes at least its kind and two `u32`s.
    let count = r.count(9)?;
    let text = r.count(1)?;
    let known = terms.len();
    if count > MAX_TERMS {
        return Err(r.bad(format!("{count} terms, more than a dictionary holds")));
    }
    if count < known {
        return Err(r.bad(format!(
            "{count} terms, fewer than the {known} every dictionary starts with"
        )));
    }
    terms.reserve(count - known, text);
    let mut seen = 0usize;
    for i in 0..count {
        let kind = r.u8()?;
        let aux = r.u32()?;
        let len = r.u32()? as usize;
        let bytes = r.take(len)?;
        let text = std::str::from_utf8(bytes).map_err(|_| r.bad(format!("term {i} is not UTF-8")))?;
        seen += len;
        #[cfg(feature = "rdf-12")]
        let triple: oxrdf::Triple;
        let term = match kind {
            KIND_IRI => Term::Iri(text),
            KIND_BLANK if aux <= documents => Term::Blank {
                doc: DocId(aux),
                label: text,
            },
            KIND_BLANK => {
                return Err(r.bad(format!("term {i} is a blank node of document {aux}, of {documents}")));
            }
            KIND_TYPED => Term::Literal {
                lexical: text,
                datatype: TermId(aux),
                language: None,
            },
            KIND_LANG => {
                let at = len.checked_sub(aux as usize).filter(|&at| text.is_char_boundary(at));
                let Some(at) = at else {
                    return Err(r.bad(format!(
                        "term {i}: a language tag of {aux} bytes in {len} bytes of text"
                    )));
                };
                let (lexical, tag) = text.split_at(at);
                Term::Literal {
                    lexical,
                    datatype: vocab::RDF_LANG_STRING,
                    language: Some(tag),
                }
            }
            #[cfg(feature = "rdf-12")]
            KIND_TRIPLE => {
                triple = text
                    .parse()
                    .ok()
                    .filter(|t: &oxrdf::Triple| aux == 0 && t.to_string() == text)
                    .ok_or_else(|| r.bad(format!("term {i} is not a triple term in N-Triples")))?;
                Term::Triple(&triple)
            }
            other => return Err(r.bad(format!("term {i} has an unknown kind {other}"))),
        };
        if i < known {
            if terms.term(TermId(i as u32)) != term {
                return Err(r.bad(format!(
                    "term {i} is not the one a new dictionary holds there: another vocabulary"
                )));
            }
            continue;
        }
        terms.append(term);
    }
    if seen != text {
        return Err(r.bad(format!("{seen} bytes of text, not the {text} announced")));
    }
    terms
        .index_appended(known)
        .map_err(|(id, earlier)| r.bad(format!("term {id} repeats term {earlier}")))?;
    for i in known..count {
        if let Term::Literal {
            datatype,
            language: None,
            ..
        } = terms.term(TermId(i as u32))
            && (datatype.index() >= count || terms.kind(datatype) != Kind::Iri)
        {
            return Err(r.bad(format!("term {i} has a datatype {} that is not an IRI", datatype.0)));
        }
    }
    terms.set_documents(documents);
    Ok(terms)
}

fn read_base(r: &mut Reader<'_>, n: usize, workers: &Workers) -> Result<ByPredicate, String> {
    // Each predicate takes at least its id, a count and a pair.
    let count = r.count(20)?;
    let mut base = ByPredicate::default();
    let mut last: Option<u32> = None;
    for _ in 0..count {
        let p = r.u32()?;
        if p as usize >= n {
            return Err(r.bad(format!("an id {p}, of {n}")));
        }
        if last.is_some_and(|q| q >= p) {
            return Err(r.bad(format!("predicate {p} is not after the one before it")));
        }
        last = Some(p);
        let len = r.count(8)?;
        if len == 0 {
            return Err(r.bad(format!("predicate {p} has no pairs")));
        }
        let pairs = r
            .id_pairs(len, n, workers)
            .map_err(|e| format!("{e} (predicate {p})"))?;
        base.push(p, &pairs);
    }
    Ok(base)
}

fn read_canon(r: &mut Reader<'_>, n: usize) -> Result<Canon, String> {
    let constants = r.u32()?;
    if constants as usize > n {
        return Err(r.bad(format!("{constants} constants, of {n} terms")));
    }
    let count = r.count(4)?;
    let watched = r.ids(count, constants as usize)?;
    let merges = r.u64()?;
    let len = r.count(4)?;
    if len > n {
        return Err(r.bad(format!("representatives for {len} ids, of {n} terms")));
    }
    let rep = r.ids(len, len)?;
    let count = r.count(12)?;
    let mut classes: Vec<(u32, Vec<u32>)> = Vec::with_capacity(count);
    let mut placed = vec![false; len];
    let mut members_moved = 0u64;
    for _ in 0..count {
        let id = r.u32()?;
        let size = r.count(4)?;
        let members = r.ids(size, len)?;
        if (id as usize) >= len || rep[id as usize] != id {
            return Err(r.bad(format!("class {id} is not its own representative")));
        }
        if classes.last().is_some_and(|(last, _)| *last >= id) {
            return Err(r.bad(format!("class {id} is not after the class before it")));
        }
        if members.len() < 2 || !members.contains(&id) {
            return Err(r.bad(format!(
                "class {id} has {} members, itself not among them",
                members.len()
            )));
        }
        for &m in &members {
            if placed[m as usize] || rep[m as usize] != id {
                return Err(r.bad(format!(
                    "{m} is in class {id}, and its representative is {}",
                    rep[m as usize]
                )));
            }
            placed[m as usize] = true;
        }
        let low = *members.iter().min().expect("two or more");
        if low < constants && low != id {
            return Err(r.bad(format!("class {id} holds the constant {low}, which must represent it")));
        }
        members_moved += members.len() as u64 - 1;
        classes.push((id, members));
    }
    if let Some(i) = (0..len).find(|&i| rep[i] as usize != i && !placed[i]) {
        return Err(r.bad(format!("{i} has the representative {}, and no class", rep[i])));
    }
    if merges != members_moved {
        return Err(r.bad(format!("{merges} merges for classes that took {members_moved}")));
    }
    Ok(Canon::restore(constants, watched, rep, classes, members_moved as usize))
}

fn read_store(r: &mut Reader<'_>, n: usize, canon: Canon, workers: &Workers) -> Result<Store, String> {
    let rounds = r.u64()?;
    let rounds = usize::try_from(rounds).map_err(|_| r.bad(format!("{rounds} rounds")))?;
    // Each table takes at least its predicate and two `u8`s.
    let count = r.count(6)?;
    let mut tables = Vec::with_capacity(count);
    let mut predicates = FxHashSet::default();
    for _ in 0..count {
        let p = r.u32()?;
        if p as usize >= n {
            return Err(r.bad(format!("a table of predicate {p}, of {n} terms")));
        }
        if !predicates.insert(p) {
            return Err(r.bad(format!("predicate {p} has two tables")));
        }
        let so = read_variable(r, n, &canon, workers)?;
        if !so.is_empty() && canon.find(p) != p {
            return Err(r.bad(format!("predicate {p} has pairs, and is not its own representative")));
        }
        let os = match r.u8()? {
            0 => None,
            1 => {
                let os = read_variable(r, n, &canon, workers)?;
                if os.len() != so.len() || fingerprint(&so, false, workers) != fingerprint(&os, true, workers) {
                    return Err(r.bad(format!(
                        "predicate {p}: its pairs by object are not its pairs by subject"
                    )));
                }
                Some(os)
            }
            other => return Err(r.bad(format!("predicate {p}: an index flag of {other}"))),
        };
        tables.push((p, Table::from_parts(so, os)));
    }
    Ok(Store::restore(tables, canon, rounds))
}

/// One part's batches (by subject or by object), checked.
fn read_variable(r: &mut Reader<'_>, n: usize, canon: &Canon, workers: &Workers) -> Result<Variable<Pair>, String> {
    let count = r.u8()?;
    let mut batches: Vec<Relation<Pair>> = Vec::with_capacity(count.into());
    for b in 0..count {
        let len = r.count(8)?;
        if len == 0 {
            return Err(r.bad(format!("batch {b} is empty")));
        }
        if let Some(last) = batches.last()
            && last.len() <= 2 * len
        {
            return Err(r.bad(format!(
                "batch {b} has {len} pairs, and the one before it {}: the tiers each halve",
                last.len()
            )));
        }
        let pairs = r.pairs(len, n, workers).map_err(|e| format!("{e} (batch {b})"))?;
        if !canon.is_empty() {
            let outside = on_chunks(workers, &pairs, |chunk| {
                chunk
                    .iter()
                    .find(|&&(s, o)| canon.find(s) != s || canon.find(o) != o)
                    .copied()
            });
            if let Some((s, o)) = outside.into_iter().flatten().next() {
                return Err(r.bad(format!("pair ({s}, {o}) is not in representatives")));
            }
        }
        if let Some(earlier) = batches.iter().position(|e| !disjoint(e, &pairs)) {
            return Err(r.bad(format!("batches {earlier} and {b} share a pair")));
        }
        batches.push(Relation::from_sorted(pairs));
    }
    Ok(Variable::from_stable(batches))
}

/// True when the sorted slices share no item.
fn disjoint(a: &[Pair], b: &[Pair]) -> bool {
    let (small, mut big) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    for x in small {
        big = gallop(big, |y| y < x);
        match big.first() {
            None => return true,
            Some(y) if y == x => return false,
            Some(_) => {}
        }
    }
    true
}

/// An order-free fingerprint of a part's pairs (flipped first when
/// `flip`): a sum of their mixed bits, so that a set and its index flipped
/// back give the same one.
fn fingerprint(v: &Variable<Pair>, flip: bool, workers: &Workers) -> u64 {
    let mix = |&(a, b): &Pair| {
        let (s, o) = if flip { (b, a) } else { (a, b) };
        // SplitMix64's finalizer.
        let mut z = (u64::from(s) << 32) | u64::from(o);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };
    v.parts()
        .flat_map(|part| {
            on_chunks(workers, part, |chunk| {
                chunk.iter().map(mix).fold(0u64, u64::wrapping_add)
            })
        })
        .fold(0u64, u64::wrapping_add)
}

/// `f` of each chunk of `items`, in order: on `workers` when there are
/// many, else `f` of all of them, on this thread.
fn on_chunks<T: Sync, R: Send>(workers: &Workers, items: &[T], f: impl Fn(&[T]) -> R + Sync + Send) -> Vec<R> {
    if workers.engage(items.len()) {
        workers.chunks(items, f)
    } else {
        vec![f(items)]
    }
}

fn read_findings(r: &mut Reader<'_>, n: usize) -> Result<Vec<Finding>, String> {
    // Each finding takes at least its code's length and a count.
    let count = r.count(9)?;
    let mut findings: Vec<Finding> = Vec::with_capacity(count);
    for i in 0..count {
        let code = r.str8()?;
        let code = CODES
            .iter()
            .copied()
            .find(|&c| c == code)
            .ok_or_else(|| r.bad(format!("finding {i} has an unknown code {code:?}")))?;
        let size = r.count(4)?;
        let terms = r.ids(size, n)?;
        let finding = Finding { code, terms };
        if findings.last().is_some_and(|last| *last >= finding) {
            return Err(r.bad(format!("finding {i} is not after the one before it")));
        }
        findings.push(finding);
    }
    Ok(findings)
}

/// Reads a snapshot's fields from the front of a byte slice, each read
/// checked against the bytes left.
#[derive(Debug)]
struct Reader<'a> {
    rest: &'a [u8],
    /// What is being read, for the messages.
    what: &'static str,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], what: &'static str) -> Self {
        Self { rest: bytes, what }
    }

    /// A message about what is being read.
    fn bad(&self, detail: String) -> String {
        format!("{}: {detail}", self.what)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if n > self.rest.len() {
            return Err(self.bad(format!("{n} bytes wanted, {} left", self.rest.len())));
        }
        let (head, rest) = self.rest.split_at(n);
        self.rest = rest;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        Ok(self.take(N)?.try_into().expect("N bytes"))
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.array::<1>()?[0])
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    fn u128(&mut self) -> Result<u128, String> {
        Ok(u128::from_le_bytes(self.array()?))
    }

    /// A count of items of at least `each` bytes, which the bytes left
    /// can hold.
    fn count(&mut self, each: usize) -> Result<usize, String> {
        let n = self.u64()?;
        let most = (self.rest.len() / each) as u64;
        if n > most {
            return Err(self.bad(format!("a count of {n}, and {} bytes left", self.rest.len())));
        }
        Ok(n as usize)
    }

    fn str8(&mut self) -> Result<&'a str, String> {
        let len = self.u8()?;
        let bytes = self.take(len.into())?;
        std::str::from_utf8(bytes).map_err(|_| self.bad("a name that is not UTF-8".into()))
    }

    fn str16(&mut self) -> Result<&'a str, String> {
        let len = u16::from_le_bytes(self.array()?);
        let bytes = self.take(len.into())?;
        std::str::from_utf8(bytes).map_err(|_| self.bad("a name that is not UTF-8".into()))
    }

    /// `count` ids, each below `n`.
    fn ids(&mut self, count: usize, n: usize) -> Result<Vec<u32>, String> {
        let bytes = self.take(count * 4)?;
        let ids: Vec<u32> = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&c| u32::from_le_bytes(c))
            .collect();
        match ids.iter().find(|&&id| id as usize >= n) {
            Some(id) => Err(self.bad(format!("an id {id}, of {n}"))),
            None => Ok(ids),
        }
    }

    /// `count` pairs of ids, each below `n`, strictly ascending.
    fn pairs(&mut self, count: usize, n: usize, workers: &Workers) -> Result<Vec<Pair>, String> {
        let pair = |c: &[u8]| (word32(&c[..4]), word32(&c[4..]));
        self.items(count, 8, n, ("pair", true), workers, pair, |&(s, o)| s.max(o))
    }

    /// `count` pairs of ids as arrays, each below `n`, strictly ascending.
    fn id_pairs(&mut self, count: usize, n: usize, workers: &Workers) -> Result<Vec<[u32; 2]>, String> {
        let pair = |c: &[u8]| [word32(&c[..4]), word32(&c[4..])];
        self.items(count, 8, n, ("pair", true), workers, pair, |&[s, o]| s.max(o))
    }

    /// `count` triples of ids, each below `n`, in any order.
    fn triples(&mut self, count: usize, n: usize, workers: &Workers) -> Result<Vec<[u32; 3]>, String> {
        let triple = |c: &[u8]| [word32(&c[..4]), word32(&c[4..8]), word32(&c[8..])];
        let top = |t: &[u32; 3]| t[0].max(t[1]).max(t[2]);
        self.items(count, 12, n, ("triple", false), workers, triple, top)
    }

    /// `count` items of `width` bytes each, made by `item`, the largest id
    /// of each (`top`) below `n`, and strictly ascending when
    /// `order.1` (the items are called `order.0` in the messages). Many
    /// items decode in chunks on `workers`.
    #[allow(clippy::too_many_arguments)]
    fn items<T: Copy + Default + Ord + Send + Sync>(
        &mut self,
        count: usize,
        width: usize,
        n: usize,
        (noun, ascending): (&str, bool),
        workers: &Workers,
        item: impl Fn(&[u8]) -> T + Sync + Send,
        top: impl Fn(&T) -> u32 + Sync + Send,
    ) -> Result<Vec<T>, String> {
        /// One chunk: its bytes, its items, the largest id, the first item
        /// not after the one before it.
        struct Chunk<'b, T> {
            bytes: &'b [u8],
            items: &'b mut [T],
            top: u32,
            unsorted: Option<usize>,
        }
        let bytes = self.take(count * width)?;
        let mut out = vec![T::default(); count];
        let size = if workers.engage(count) {
            workers.chunk(count)
        } else {
            count.max(1)
        };
        let mut chunks: Vec<Chunk<'_, T>> = bytes
            .chunks(size * width)
            .zip(out.chunks_mut(size))
            .map(|(bytes, items)| Chunk {
                bytes,
                items,
                top: 0,
                unsorted: None,
            })
            .collect();
        workers.for_each(&mut chunks, |chunk| {
            for (slot, b) in chunk.items.iter_mut().zip(chunk.bytes.chunks_exact(width)) {
                *slot = item(b);
            }
            chunk.top = chunk.items.iter().map(&top).max().unwrap_or(0);
            if ascending {
                chunk.unsorted = chunk.items.windows(2).position(|w| w[0] >= w[1]).map(|i| i + 1);
            }
        });
        let mut at = 0;
        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.top as usize >= n {
                return Err(self.bad(format!("an id {}, of {n}", chunk.top)));
            }
            let across = ascending && i > 0 && chunks[i - 1].items.last() >= chunk.items.first();
            if let Some(k) = chunk.unsorted.or(across.then_some(0)) {
                return Err(self.bad(format!("{noun} {} is not after the one before it", at + k)));
            }
            at += chunk.items.len();
        }
        drop(chunks);
        Ok(out)
    }

    /// The payload of the next section, which must be `section`.
    fn section(&mut self, section: Section) -> Result<Reader<'a>, String> {
        let tag: [u8; 4] = self.array().map_err(|_| format!("{} is missing", section.name()))?;
        if tag != section.tag() {
            return Err(format!(
                "{} expected, found the tag {:?}",
                section.name(),
                String::from_utf8_lossy(&tag)
            ));
        }
        let len = self.u64()?;
        let len = usize::try_from(len)
            .ok()
            .filter(|&len| len <= self.rest.len())
            .ok_or_else(|| {
                format!(
                    "{}: a length of {len}, and {} bytes left",
                    section.name(),
                    self.rest.len()
                )
            })?;
        Ok(Reader::new(self.take(len)?, section.name()))
    }

    /// Checks that everything was read.
    fn end(&self) -> Result<(), String> {
        match self.rest.len() {
            0 => Ok(()),
            left => Err(self.bad(format!("{left} bytes left over"))),
        }
    }
}

// ---------------------------------------------------------------------------
// Files
// ---------------------------------------------------------------------------

/// Writes a file at `path` with `write`, atomically: into a temporary file
/// beside it (`.<name>.<process>-<n>.tmp`, new), synced to the disk, then
/// renamed over `path`. A crash leaves `path` as it was or wholly written,
/// never half; it can leave the temporary file, which nothing reads. On
/// an error the temporary file is removed and `path` is unchanged.
/// Concurrent saves to one path each write their own temporary file, and
/// the last rename wins.
#[cfg(feature = "io")]
pub(crate) fn save(path: &std::path::Path, write: impl FnOnce(&mut std::fs::File) -> io::Result<()>) -> io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "a snapshot is saved to a file name"))?;
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => std::path::Path::new("."),
    };
    let (temp, mut file) = loop {
        let mut temp_name = std::ffi::OsString::from(".");
        temp_name.push(name);
        temp_name.push(format!(
            ".{}-{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let temp = dir.join(temp_name);
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => break (temp, file),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    };
    let written = write(&mut file).and_then(|()| file.sync_all()).and_then(|()| {
        drop(file);
        std::fs::rename(&temp, path)
    });
    if written.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    written
}

// ---------------------------------------------------------------------------
// The checksum: XXH64
// ---------------------------------------------------------------------------

const P1: u64 = 0x9E37_79B1_85EB_CA87;
const P2: u64 = 0xC2B2_AE3D_27D4_EB4F;
const P3: u64 = 0x1656_67B1_9E37_79F9;
const P4: u64 = 0x85EB_CA77_C2B2_AE63;
const P5: u64 = 0x27D4_EB2F_1656_67C5;

/// XXH64 (the xxHash specification, `doc/xxhash_spec.md` in
/// <https://github.com/Cyan4973/xxHash>), fed in pieces.
#[derive(Debug, Clone)]
struct Xxh64 {
    seed: u64,
    lanes: [u64; 4],
    /// The bytes of a stripe not yet full.
    held: [u8; 32],
    holding: usize,
    total: u64,
}

#[inline]
fn lane_round(acc: u64, input: u64) -> u64 {
    acc.wrapping_add(input.wrapping_mul(P2))
        .rotate_left(31)
        .wrapping_mul(P1)
}

#[inline]
fn word32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes[..4].try_into().expect("4 bytes"))
}

#[inline]
fn word64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes[..8].try_into().expect("8 bytes"))
}

impl Xxh64 {
    fn new(seed: u64) -> Self {
        Self {
            seed,
            lanes: [
                seed.wrapping_add(P1).wrapping_add(P2),
                seed.wrapping_add(P2),
                seed,
                seed.wrapping_sub(P1),
            ],
            held: [0; 32],
            holding: 0,
            total: 0,
        }
    }

    #[inline]
    fn stripe(lanes: &mut [u64; 4], stripe: &[u8]) {
        for (i, lane) in lanes.iter_mut().enumerate() {
            *lane = lane_round(*lane, word64(&stripe[i * 8..]));
        }
    }

    fn update(&mut self, mut bytes: &[u8]) {
        self.total += bytes.len() as u64;
        if self.holding > 0 {
            let take = (32 - self.holding).min(bytes.len());
            self.held[self.holding..self.holding + take].copy_from_slice(&bytes[..take]);
            self.holding += take;
            bytes = &bytes[take..];
            if self.holding < 32 {
                return;
            }
            let held = self.held;
            Self::stripe(&mut self.lanes, &held);
            self.holding = 0;
        }
        let (stripes, rest) = bytes.as_chunks::<32>();
        for stripe in stripes {
            Self::stripe(&mut self.lanes, stripe);
        }
        self.held[..rest.len()].copy_from_slice(rest);
        self.holding = rest.len();
    }

    fn digest(&self) -> u64 {
        let mut h = if self.total >= 32 {
            let [v1, v2, v3, v4] = self.lanes;
            let mut h = v1
                .rotate_left(1)
                .wrapping_add(v2.rotate_left(7))
                .wrapping_add(v3.rotate_left(12))
                .wrapping_add(v4.rotate_left(18));
            for v in self.lanes {
                h = (h ^ lane_round(0, v)).wrapping_mul(P1).wrapping_add(P4);
            }
            h
        } else {
            self.seed.wrapping_add(P5)
        };
        h = h.wrapping_add(self.total);
        let mut rest = &self.held[..self.holding];
        while rest.len() >= 8 {
            h ^= lane_round(0, word64(rest));
            h = h.rotate_left(27).wrapping_mul(P1).wrapping_add(P4);
            rest = &rest[8..];
        }
        if rest.len() >= 4 {
            let word = u32::from_le_bytes(rest[..4].try_into().expect("4 bytes"));
            h ^= u64::from(word).wrapping_mul(P1);
            h = h.rotate_left(23).wrapping_mul(P2).wrapping_add(P3);
            rest = &rest[4..];
        }
        for &byte in rest {
            h ^= u64::from(byte).wrapping_mul(P5);
            h = h.rotate_left(11).wrapping_mul(P1);
        }
        h ^= h >> 33;
        h = h.wrapping_mul(P2);
        h ^= h >> 29;
        h = h.wrapping_mul(P3);
        h ^ (h >> 32)
    }
}

/// XXH64 of `bytes`.
fn xxh64(bytes: &[u8], seed: u64) -> u64 {
    let mut h = Xxh64::new(seed);
    h.update(bytes);
    h.digest()
}

#[cfg(all(test, feature = "io"))]
mod tests;
