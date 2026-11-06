//! Term ids and the dictionary.
//!
//! Every IRI, blank node and literal is interned once to a [`TermId`], a
//! dense `u32` in first-seen order. Relations, rules and indexes work only on
//! term ids; terms appear only at the edges (input, output, diagnostics).
//!
//! The vocabulary the rules use ([`vocab`]) is interned first, by
//! [`Dictionary::new`], so `vocab::RDF_TYPE` and the rest are constants that
//! never depend on the input.
//!
//! Identity:
//! - an IRI is its text;
//! - a blank node is its label *and* its document ([`DocId`]): `_:b` in two
//!   documents is two blank nodes;
//! - a literal is its lexical form plus its datatype, or plus its language
//!   tag (datatype `rdf:langString`). Literals are never compared by value:
//!   `"1"^^xsd:integer` and `"01"^^xsd:integer` are two terms.
//!
//! On output a blank node of a document is labelled `<label>_<document>`,
//! and one from no document (inserted by a caller) keeps its label, so
//! distinct blank nodes always get distinct labels
//! ([`Dictionary::write_blank_label`] has the rule).
//!
//! Layout, for speed: all term text lives in one arena `String`; an entry
//! per term holds its range, its kind and one 32-bit field (the document of a
//! blank node, the datatype id of a literal, or the length of a language
//! tag). The term-to-id map is an open-addressed table of `u64` slots, each
//! the term's 32-bit FxHash beside its id, so a probe compares hashes inside
//! the table and reads the arena only on a hash match. Interning a term seen
//! before allocates nothing; a new term appends to the arena and the entry
//! list (amortised). Id to term is one index into the entries.
//!
//! This module builds without the `io` feature (WebAssembly).

use oxrdf::{BlankNodeRef, LiteralRef, NamedNodeRef, NamedOrBlankNodeRef, TermRef};
use rustc_hash::FxHasher;
use std::fmt;
use std::hash::Hasher;

/// A term's id in the dictionary. Relations and rules work only on these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TermId(pub(crate) u32);

impl TermId {
    /// The id as an index into per-term tables.
    #[inline]
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

/// The document a blank node comes from. Each parsed document gets a new
/// one from [`Dictionary::new_document`]; [`DocId::NONE`] is for blank nodes
/// that come from no document (triples a caller inserts, terms a rule makes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DocId(pub(crate) u32);

impl DocId {
    /// Blank nodes that come from no document.
    pub(crate) const NONE: DocId = DocId(0);
}

/// A term, read from the dictionary. Borrowed: no allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Term<'a> {
    Iri(&'a str),
    Blank {
        doc: DocId,
        label: &'a str,
    },
    /// `datatype` is `vocab::RDF_LANG_STRING` when `language` is set. A literal
    /// typed `rdf:langString` with no tag is ill-typed, and kept as it came:
    /// no parser makes one, but a caller can insert it (dt-not-type reports it).
    Literal {
        lexical: &'a str,
        datatype: TermId,
        language: Option<&'a str>,
    },
}

/// The kinds of term, as rules test them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    Iri,
    Blank,
    Literal,
}

/// The most terms a dictionary holds: ids `0..MAX_TERMS`. `u32::MAX` is never
/// an id.
pub(crate) const MAX_TERMS: usize = u32::MAX as usize;

/// The longest term text, in bytes (a language-tagged literal counts its
/// lexical form plus its tag).
pub(crate) const MAX_TERM_BYTES: usize = u32::MAX as usize;

// Entry tags, in the top two bits of `Entry::start`.
const TAG_IRI: u64 = 0;
const TAG_BLANK: u64 = 1;
const TAG_TYPED: u64 = 2;
const TAG_LANG: u64 = 3;
const TAG_SHIFT: u32 = 62;
const START_MASK: u64 = (1 << TAG_SHIFT) - 1;

/// An empty slot: hash `u32::MAX` beside id `u32::MAX`, which is never an id.
const EMPTY: u64 = u64::MAX;

/// One term: `start` is its offset in the arena with the tag in the top two
/// bits; `aux` is the blank node's document, the typed literal's datatype id,
/// or the language tag's length (the tag follows the lexical form).
#[derive(Clone, Copy)]
struct Entry {
    start: u64,
    len: u32,
    aux: u32,
}

impl Entry {
    #[inline]
    fn tag(self) -> u64 {
        self.start >> TAG_SHIFT
    }

    #[inline]
    fn range(self) -> std::ops::Range<usize> {
        let start = (self.start & START_MASK) as usize;
        start..start + self.len as usize
    }
}

/// The two-way map between terms and term ids. Interning is idempotent.
#[derive(Clone)]
pub(crate) struct Dictionary {
    text: String,
    entries: Vec<Entry>,
    /// Open addressing, linear probing, at most half full: `hash << 32 | id`.
    slots: Vec<u64>,
    documents: u32,
}

impl fmt::Debug for Dictionary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dictionary")
            .field("terms", &self.entries.len())
            .field("text_bytes", &self.text.len())
            .field("documents", &self.documents)
            .finish()
    }
}

impl Default for Dictionary {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn hash(tag: u64, aux: u32, text: &str, tail: &str) -> u32 {
    let mut h = FxHasher::default();
    h.write_u64((tag << 32) | u64::from(aux));
    h.write(text.as_bytes());
    if !tail.is_empty() {
        h.write(tail.as_bytes());
    }
    let x = h.finish();
    ((x >> 32) as u32) ^ (x as u32)
}

impl Dictionary {
    /// A dictionary holding the vocabulary at its fixed ids.
    pub(crate) fn new() -> Self {
        let mut d = Self {
            text: String::with_capacity(vocab::IRIS.iter().map(|i| i.len()).sum()),
            entries: Vec::with_capacity(vocab::IRIS.len()),
            slots: vec![EMPTY; (vocab::IRIS.len() * 4).next_power_of_two()],
            documents: 0,
        };
        for (i, iri) in vocab::IRIS.iter().enumerate() {
            let id = d.intern_iri(iri);
            debug_assert_eq!(id.index(), i, "vocabulary IRI {iri} is listed twice");
        }
        d
    }

    /// The bytes the dictionary holds: its text (the arena), its entries
    /// and its hash slots. Capacities, not lengths.
    pub(crate) fn bytes(&self) -> (usize, usize, usize) {
        use crate::profile::vec_bytes;
        (self.text.capacity(), vec_bytes(&self.entries), vec_bytes(&self.slots))
    }

    /// Makes room for `terms` more terms and `bytes` more bytes of term text.
    #[allow(dead_code)] // for a loader that knows its input's size (an experiment)
    pub(crate) fn reserve(&mut self, terms: usize, bytes: usize) {
        self.text.reserve(bytes);
        self.entries.reserve(terms);
        let want = ((self.entries.len() + terms) * 2 + 1).next_power_of_two();
        if want > self.slots.len() {
            self.rehash(want);
        }
    }

    /// The number of terms.
    #[inline]
    #[cfg_attr(not(feature = "io"), allow(dead_code))] // the parser uses it
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// The bytes the dictionary holds on the heap: the arena's, the
    /// entries' and the slots' capacities (the scaling tests check its growth).
    #[cfg(test)]
    pub(crate) fn heap_bytes(&self) -> usize {
        self.text.capacity()
            + self.entries.capacity() * std::mem::size_of::<Entry>()
            + self.slots.capacity() * std::mem::size_of::<u64>()
    }

    /// The bytes of term text in the arena.
    #[cfg(test)]
    pub(crate) fn text_bytes(&self) -> usize {
        self.text.len()
    }

    /// Starts a document: its blank nodes are its own.
    #[cfg_attr(not(feature = "io"), allow(dead_code))] // the parser uses it
    pub(crate) fn new_document(&mut self) -> DocId {
        self.documents += 1;
        DocId(self.documents)
    }

    /// The id of an IRI, interned if new.
    ///
    /// # Panics
    /// When the dictionary already holds [`MAX_TERMS`] terms or the term is
    /// longer than [`MAX_TERM_BYTES`] (the parsers check both first). The
    /// same holds for every `intern_*`.
    #[inline]
    pub(crate) fn intern_iri(&mut self, iri: &str) -> TermId {
        self.intern(TAG_IRI, 0, iri, "")
    }

    /// The id of the blank node `label` of document `doc`, interned if new.
    #[inline]
    pub(crate) fn intern_blank(&mut self, doc: DocId, label: &str) -> TermId {
        self.intern(TAG_BLANK, doc.0, label, "")
    }

    /// The id of a literal with a datatype and no language tag, interned if
    /// new. With `rdf:langString` it is the ill-typed literal of [`Term::Literal`].
    #[inline]
    pub(crate) fn intern_typed_literal(&mut self, lexical: &str, datatype: TermId) -> TermId {
        self.intern(TAG_TYPED, datatype.0, lexical, "")
    }

    /// The id of a language-tagged literal, interned if new.
    #[inline]
    pub(crate) fn intern_lang_literal(&mut self, lexical: &str, language: &str) -> TermId {
        self.intern(TAG_LANG, lang_len(language), lexical, language)
    }

    /// The id of an `oxrdf` literal, interned if new.
    #[inline]
    pub(crate) fn intern_literal(&mut self, literal: LiteralRef<'_>) -> TermId {
        match literal.language() {
            Some(language) => self.intern_lang_literal(literal.value(), language),
            None => {
                let datatype = literal.datatype().as_str();
                // Most literals are plain strings: skip the table for their datatype.
                let datatype = if datatype == vocab::XSD_STRING_IRI {
                    vocab::XSD_STRING
                } else {
                    self.intern_iri(datatype)
                };
                self.intern_typed_literal(literal.value(), datatype)
            }
        }
    }

    /// The id of an `oxrdf` term, interned if new; its blank nodes belong to `doc`.
    #[inline]
    pub(crate) fn intern_term(&mut self, term: TermRef<'_>, doc: DocId) -> TermId {
        match term {
            TermRef::NamedNode(n) => self.intern_iri(n.as_str()),
            TermRef::BlankNode(b) => self.intern_blank(doc, b.as_str()),
            TermRef::Literal(l) => self.intern_literal(l),
        }
    }

    /// The id of an `oxrdf` subject, interned if new; a blank node belongs to `doc`.
    #[inline]
    pub(crate) fn intern_subject(&mut self, subject: NamedOrBlankNodeRef<'_>, doc: DocId) -> TermId {
        match subject {
            NamedOrBlankNodeRef::NamedNode(n) => self.intern_iri(n.as_str()),
            NamedOrBlankNodeRef::BlankNode(b) => self.intern_blank(doc, b.as_str()),
        }
    }

    /// The id of an `oxrdf` term if it is interned; nothing is added.
    pub(crate) fn get_term(&self, term: TermRef<'_>, doc: DocId) -> Option<TermId> {
        match term {
            TermRef::NamedNode(n) => self.get_iri(n.as_str()),
            TermRef::BlankNode(b) => self.get(TAG_BLANK, doc.0, b.as_str(), ""),
            TermRef::Literal(l) => match l.language() {
                Some(language) => self.get(TAG_LANG, lang_len(language), l.value(), language),
                None => {
                    let datatype = self.get_iri(l.datatype().as_str())?;
                    self.get(TAG_TYPED, datatype.0, l.value(), "")
                }
            },
        }
    }

    /// The id of an IRI if it is interned; nothing is added.
    pub(crate) fn get_iri(&self, iri: &str) -> Option<TermId> {
        self.get(TAG_IRI, 0, iri, "")
    }

    /// The term of an id.
    ///
    /// # Panics
    /// When `id` is not from this dictionary.
    #[inline]
    pub(crate) fn term(&self, id: TermId) -> Term<'_> {
        let e = self.entries[id.index()];
        let text = &self.text[e.range()];
        match e.tag() {
            TAG_IRI => Term::Iri(text),
            TAG_BLANK => Term::Blank {
                doc: DocId(e.aux),
                label: text,
            },
            TAG_TYPED => Term::Literal {
                lexical: text,
                datatype: TermId(e.aux),
                language: None,
            },
            _ => {
                let (lexical, language) = text.split_at(text.len() - e.aux as usize);
                Term::Literal {
                    lexical,
                    datatype: vocab::RDF_LANG_STRING,
                    language: Some(language),
                }
            }
        }
    }

    /// The kind of an id's term, without reading its text.
    #[inline]
    pub(crate) fn kind(&self, id: TermId) -> Kind {
        match self.entries[id.index()].tag() {
            TAG_IRI => Kind::Iri,
            TAG_BLANK => Kind::Blank,
            _ => Kind::Literal,
        }
    }

    /// The IRI of an id, if its term is an IRI.
    #[inline]
    pub(crate) fn iri(&self, id: TermId) -> Option<&str> {
        match self.term(id) {
            Term::Iri(iri) => Some(iri),
            _ => None,
        }
    }

    /// Writes the label a blank node has on output into `out` (cleared
    /// first). A term that is not a blank node leaves `out` empty.
    ///
    /// - A blank node of a document is `<label>_<document>`: `_:b` of the
    ///   first document is `b_1`, of the second `b_2`.
    /// - A blank node from no document ([`DocId::NONE`], inserted by a
    ///   caller) keeps its label, so inserting a triple and reading the
    ///   closure gives the label back.
    /// - Except when that label already looks like a document label (it ends
    ///   in `_` and digits, as `b_1` does): it is then written with `_0`
    ///   appended (`b_1_0`). No document label ends in `_0` (documents are
    ///   numbered from 1), so such a node never takes a document node's label.
    ///
    /// Distinct blank nodes get distinct labels, valid in every format.
    pub(crate) fn write_blank_label(&self, id: TermId, out: &mut String) {
        out.clear();
        if let Term::Blank { doc, label } = self.term(id) {
            out.push_str(label);
            if doc != DocId::NONE {
                out.push('_');
                push_decimal(out, doc.0);
            } else if looks_like_document_label(label) {
                out.push_str("_0");
            }
        }
    }

    /// The owned `oxrdf` term of an id (blank nodes labelled as by
    /// [`Self::write_blank_label`]).
    pub(crate) fn to_oxrdf(&self, id: TermId) -> oxrdf::Term {
        let mut scratch = String::new();
        self.to_oxrdf_ref(id, &mut scratch).into_owned()
    }

    /// The borrowed `oxrdf` term of an id; a blank node's label is written
    /// into `scratch`. Allocates nothing once `scratch` is large enough.
    pub(crate) fn to_oxrdf_ref<'a>(&'a self, id: TermId, scratch: &'a mut String) -> TermRef<'a> {
        match self.term(id) {
            Term::Iri(iri) => NamedNodeRef::new_unchecked(iri).into(),
            Term::Blank { .. } => {
                self.write_blank_label(id, scratch);
                BlankNodeRef::new_unchecked(scratch).into()
            }
            Term::Literal {
                lexical,
                language: Some(language),
                ..
            } => LiteralRef::new_language_tagged_literal_unchecked(lexical, language).into(),
            Term::Literal { lexical, datatype, .. } => {
                let datatype = self.iri(datatype).unwrap_or(vocab::XSD_STRING_IRI);
                LiteralRef::new_typed_literal(lexical, NamedNodeRef::new_unchecked(datatype)).into()
            }
        }
    }

    #[inline]
    fn matches(&self, id: u32, tag: u64, aux: u32, text: &str, tail: &str) -> bool {
        let e = self.entries[id as usize];
        if e.tag() != tag || e.aux != aux || e.len as usize != text.len() + tail.len() {
            return false;
        }
        let (head, rest) = self.text.as_bytes()[e.range()].split_at(text.len());
        head == text.as_bytes() && rest == tail.as_bytes()
    }

    /// The id of the key, or the empty slot where it goes.
    #[inline]
    fn find(&self, h: u32, tag: u64, aux: u32, text: &str, tail: &str) -> Result<TermId, usize> {
        let mask = self.slots.len() - 1;
        let mut i = h as usize & mask;
        loop {
            let slot = self.slots[i];
            if slot == EMPTY {
                return Err(i);
            }
            if (slot >> 32) as u32 == h && self.matches(slot as u32, tag, aux, text, tail) {
                return Ok(TermId(slot as u32));
            }
            i = (i + 1) & mask;
        }
    }

    #[inline]
    fn get(&self, tag: u64, aux: u32, text: &str, tail: &str) -> Option<TermId> {
        self.find(hash(tag, aux, text, tail), tag, aux, text, tail).ok()
    }

    #[inline]
    fn intern(&mut self, tag: u64, aux: u32, text: &str, tail: &str) -> TermId {
        let h = hash(tag, aux, text, tail);
        match self.find(h, tag, aux, text, tail) {
            Ok(id) => id,
            Err(slot) => self.insert(h, slot, tag, aux, text, tail),
        }
    }

    fn insert(&mut self, h: u32, slot: usize, tag: u64, aux: u32, text: &str, tail: &str) -> TermId {
        let id = self.entries.len();
        assert!(id < MAX_TERMS, "the dictionary is full ({MAX_TERMS} terms)");
        let len = text.len() + tail.len();
        assert!(len <= MAX_TERM_BYTES, "a term is longer than {MAX_TERM_BYTES} bytes");
        let start = self.text.len() as u64;
        self.text.push_str(text);
        self.text.push_str(tail);
        self.entries.push(Entry {
            start: start | (tag << TAG_SHIFT),
            len: len as u32,
            aux,
        });
        let id = id as u32;
        self.slots[slot] = (u64::from(h) << 32) | u64::from(id);
        if self.entries.len() * 2 > self.slots.len() {
            self.rehash(self.slots.len() * 2);
        }
        TermId(id)
    }

    #[cold]
    fn rehash(&mut self, size: usize) {
        let old = std::mem::replace(&mut self.slots, vec![EMPTY; size]);
        let mask = size - 1;
        for slot in old.into_iter().filter(|&s| s != EMPTY) {
            let mut i = (slot >> 32) as usize & mask;
            while self.slots[i] != EMPTY {
                i = (i + 1) & mask;
            }
            self.slots[i] = slot;
        }
    }
}

/// A language tag's length as the entry stores it. Longer tags than
/// `u32::MAX` bytes cannot be stored; `insert` rejects the term first.
#[inline]
fn lang_len(language: &str) -> u32 {
    u32::try_from(language.len()).unwrap_or(u32::MAX)
}

/// True when `label` ends in `_` and one or more digits, the shape of a
/// document blank node's output label.
fn looks_like_document_label(label: &str) -> bool {
    let digits = label.len() - label.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    digits > 0 && label[..label.len() - digits].ends_with('_')
}

fn push_decimal(out: &mut String, mut n: u32) {
    let mut digits = [0u8; 10];
    let mut i = digits.len();
    loop {
        i -= 1;
        digits[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    for &d in &digits[i..] {
        out.push(char::from(d));
    }
}

/// The vocabulary: the IRIs the OWL 2 RL rules (W3C OWL 2 Profiles, Tables
/// 4 to 9, and the RL datatypes of Section 4.2) and the RDFS rules name,
/// interned first at fixed ids.
/// A constant's id is its position in [`IRIS`](vocab::IRIS). It lists every
/// IRI of those tables, the annotation properties of prp-ap included, so a
/// rule family never adds one (the phase 1 spec, "The vocabulary"). The
/// canon treats every id here as a rule constant (`rules/equality.rs`).
#[allow(dead_code)]
pub(crate) mod vocab {
    use super::TermId;

    /// `xsd:string`, the datatype of plain literals.
    pub(crate) const XSD_STRING_IRI: &str = "http://www.w3.org/2001/XMLSchema#string";

    /// The namespaces, for writers that declare prefixes.
    pub(crate) const PREFIXES: &[(&str, &str)] = &[
        ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
        ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
        ("owl", "http://www.w3.org/2002/07/owl#"),
        ("xsd", "http://www.w3.org/2001/XMLSchema#"),
    ];

    // `concat!` takes only literals, so each namespace is spelled here.
    macro_rules! iri {
        (rdf $local:literal) => {
            concat!("http://www.w3.org/1999/02/22-rdf-syntax-ns#", $local)
        };
        (rdfs $local:literal) => {
            concat!("http://www.w3.org/2000/01/rdf-schema#", $local)
        };
        (owl $local:literal) => {
            concat!("http://www.w3.org/2002/07/owl#", $local)
        };
        (xsd $local:literal) => {
            concat!("http://www.w3.org/2001/XMLSchema#", $local)
        };
    }

    macro_rules! vocabulary {
        ($($name:ident = $ns:ident $local:literal,)*) => {
            #[allow(non_camel_case_types, clippy::upper_case_acronyms)]
            #[repr(u32)]
            enum Position { $($name,)* }

            $(pub(crate) const $name: TermId = TermId(Position::$name as u32);)*

            /// The vocabulary IRIs, in id order.
            pub(crate) const IRIS: &[&str] = &[$(iri!($ns $local),)*];

            /// The constants' names, in id order (for tests and debugging).
            pub(crate) const NAMES: &[&str] = &[$(stringify!($name),)*];
        };
    }

    vocabulary! {
        // RDF
        RDF_TYPE = rdf "type",
        RDF_PROPERTY = rdf "Property",
        RDF_LIST = rdf "List",
        RDF_FIRST = rdf "first",
        RDF_REST = rdf "rest",
        RDF_NIL = rdf "nil",
        RDF_STATEMENT = rdf "Statement",
        RDF_SUBJECT = rdf "subject",
        RDF_PREDICATE = rdf "predicate",
        RDF_OBJECT = rdf "object",
        RDF_VALUE = rdf "value",
        RDF_BAG = rdf "Bag",
        RDF_SEQ = rdf "Seq",
        RDF_ALT = rdf "Alt",
        RDF_LANG_STRING = rdf "langString",
        RDF_PLAIN_LITERAL = rdf "PlainLiteral",
        RDF_XML_LITERAL = rdf "XMLLiteral",
        // RDFS
        RDFS_RESOURCE = rdfs "Resource",
        RDFS_CLASS = rdfs "Class",
        RDFS_LITERAL = rdfs "Literal",
        RDFS_DATATYPE = rdfs "Datatype",
        RDFS_CONTAINER = rdfs "Container",
        RDFS_CONTAINER_MEMBERSHIP_PROPERTY = rdfs "ContainerMembershipProperty",
        RDFS_SUB_CLASS_OF = rdfs "subClassOf",
        RDFS_SUB_PROPERTY_OF = rdfs "subPropertyOf",
        RDFS_DOMAIN = rdfs "domain",
        RDFS_RANGE = rdfs "range",
        RDFS_MEMBER = rdfs "member",
        RDFS_LABEL = rdfs "label",
        RDFS_COMMENT = rdfs "comment",
        RDFS_SEE_ALSO = rdfs "seeAlso",
        RDFS_IS_DEFINED_BY = rdfs "isDefinedBy",
        // OWL: individuals and equality (Table 4), the top and bottom classes
        OWL_THING = owl "Thing",
        OWL_NOTHING = owl "Nothing",
        OWL_CLASS = owl "Class",
        OWL_SAME_AS = owl "sameAs",
        OWL_DIFFERENT_FROM = owl "differentFrom",
        OWL_ALL_DIFFERENT = owl "AllDifferent",
        OWL_MEMBERS = owl "members",
        OWL_DISTINCT_MEMBERS = owl "distinctMembers",
        OWL_NAMED_INDIVIDUAL = owl "NamedIndividual",
        // OWL: properties (Table 5, scm-op, scm-dp)
        OWL_OBJECT_PROPERTY = owl "ObjectProperty",
        OWL_DATATYPE_PROPERTY = owl "DatatypeProperty",
        OWL_ANNOTATION_PROPERTY = owl "AnnotationProperty",
        OWL_ONTOLOGY_PROPERTY = owl "OntologyProperty",
        OWL_FUNCTIONAL_PROPERTY = owl "FunctionalProperty",
        OWL_INVERSE_FUNCTIONAL_PROPERTY = owl "InverseFunctionalProperty",
        OWL_IRREFLEXIVE_PROPERTY = owl "IrreflexiveProperty",
        OWL_REFLEXIVE_PROPERTY = owl "ReflexiveProperty",
        OWL_SYMMETRIC_PROPERTY = owl "SymmetricProperty",
        OWL_ASYMMETRIC_PROPERTY = owl "AsymmetricProperty",
        OWL_TRANSITIVE_PROPERTY = owl "TransitiveProperty",
        OWL_PROPERTY_CHAIN_AXIOM = owl "propertyChainAxiom",
        OWL_EQUIVALENT_PROPERTY = owl "equivalentProperty",
        OWL_PROPERTY_DISJOINT_WITH = owl "propertyDisjointWith",
        OWL_ALL_DISJOINT_PROPERTIES = owl "AllDisjointProperties",
        OWL_INVERSE_OF = owl "inverseOf",
        OWL_HAS_KEY = owl "hasKey",
        OWL_NEGATIVE_PROPERTY_ASSERTION = owl "NegativePropertyAssertion",
        OWL_SOURCE_INDIVIDUAL = owl "sourceIndividual",
        OWL_ASSERTION_PROPERTY = owl "assertionProperty",
        OWL_TARGET_INDIVIDUAL = owl "targetIndividual",
        OWL_TARGET_VALUE = owl "targetValue",
        OWL_TOP_OBJECT_PROPERTY = owl "topObjectProperty",
        OWL_BOTTOM_OBJECT_PROPERTY = owl "bottomObjectProperty",
        OWL_TOP_DATA_PROPERTY = owl "topDataProperty",
        OWL_BOTTOM_DATA_PROPERTY = owl "bottomDataProperty",
        // OWL: class expressions (Table 6)
        OWL_RESTRICTION = owl "Restriction",
        OWL_INTERSECTION_OF = owl "intersectionOf",
        OWL_UNION_OF = owl "unionOf",
        OWL_COMPLEMENT_OF = owl "complementOf",
        OWL_ONE_OF = owl "oneOf",
        OWL_ON_PROPERTY = owl "onProperty",
        OWL_ON_PROPERTIES = owl "onProperties",
        OWL_ON_CLASS = owl "onClass",
        OWL_ON_DATA_RANGE = owl "onDataRange",
        OWL_SOME_VALUES_FROM = owl "someValuesFrom",
        OWL_ALL_VALUES_FROM = owl "allValuesFrom",
        OWL_HAS_VALUE = owl "hasValue",
        OWL_HAS_SELF = owl "hasSelf",
        OWL_MAX_CARDINALITY = owl "maxCardinality",
        OWL_MIN_CARDINALITY = owl "minCardinality",
        OWL_CARDINALITY = owl "cardinality",
        OWL_MAX_QUALIFIED_CARDINALITY = owl "maxQualifiedCardinality",
        OWL_MIN_QUALIFIED_CARDINALITY = owl "minQualifiedCardinality",
        OWL_QUALIFIED_CARDINALITY = owl "qualifiedCardinality",
        // OWL: class axioms (Table 7)
        OWL_EQUIVALENT_CLASS = owl "equivalentClass",
        OWL_DISJOINT_WITH = owl "disjointWith",
        OWL_ALL_DISJOINT_CLASSES = owl "AllDisjointClasses",
        OWL_DISJOINT_UNION_OF = owl "disjointUnionOf",
        // OWL: datatypes and data ranges (Table 8)
        OWL_REAL = owl "real",
        OWL_RATIONAL = owl "rational",
        OWL_DATA_RANGE = owl "DataRange",
        OWL_ON_DATATYPE = owl "onDatatype",
        OWL_WITH_RESTRICTIONS = owl "withRestrictions",
        OWL_DATATYPE_COMPLEMENT_OF = owl "datatypeComplementOf",
        // OWL: ontologies and annotations
        OWL_ONTOLOGY = owl "Ontology",
        OWL_IMPORTS = owl "imports",
        OWL_VERSION_IRI = owl "versionIRI",
        OWL_VERSION_INFO = owl "versionInfo",
        OWL_PRIOR_VERSION = owl "priorVersion",
        OWL_BACKWARD_COMPATIBLE_WITH = owl "backwardCompatibleWith",
        OWL_INCOMPATIBLE_WITH = owl "incompatibleWith",
        OWL_DEPRECATED = owl "deprecated",
        OWL_DEPRECATED_CLASS = owl "DeprecatedClass",
        OWL_DEPRECATED_PROPERTY = owl "DeprecatedProperty",
        OWL_AXIOM = owl "Axiom",
        OWL_ANNOTATION = owl "Annotation",
        OWL_ANNOTATED_SOURCE = owl "annotatedSource",
        OWL_ANNOTATED_PROPERTY = owl "annotatedProperty",
        OWL_ANNOTATED_TARGET = owl "annotatedTarget",
        // XSD: the OWL 2 RL datatypes (Section 4.2)
        XSD_STRING = xsd "string",
        XSD_NORMALIZED_STRING = xsd "normalizedString",
        XSD_TOKEN = xsd "token",
        XSD_LANGUAGE = xsd "language",
        XSD_NAME = xsd "Name",
        XSD_NCNAME = xsd "NCName",
        XSD_NMTOKEN = xsd "NMTOKEN",
        XSD_BOOLEAN = xsd "boolean",
        XSD_DECIMAL = xsd "decimal",
        XSD_INTEGER = xsd "integer",
        XSD_NON_NEGATIVE_INTEGER = xsd "nonNegativeInteger",
        XSD_NON_POSITIVE_INTEGER = xsd "nonPositiveInteger",
        XSD_POSITIVE_INTEGER = xsd "positiveInteger",
        XSD_NEGATIVE_INTEGER = xsd "negativeInteger",
        XSD_LONG = xsd "long",
        XSD_INT = xsd "int",
        XSD_SHORT = xsd "short",
        XSD_BYTE = xsd "byte",
        XSD_UNSIGNED_LONG = xsd "unsignedLong",
        XSD_UNSIGNED_INT = xsd "unsignedInt",
        XSD_UNSIGNED_SHORT = xsd "unsignedShort",
        XSD_UNSIGNED_BYTE = xsd "unsignedByte",
        XSD_FLOAT = xsd "float",
        XSD_DOUBLE = xsd "double",
        XSD_HEX_BINARY = xsd "hexBinary",
        XSD_BASE64_BINARY = xsd "base64Binary",
        XSD_ANY_URI = xsd "anyURI",
        XSD_DATE_TIME = xsd "dateTime",
        XSD_DATE_TIME_STAMP = xsd "dateTimeStamp",
    }
}

/// What a snapshot (`snapshot.rs`, ticket 19) reads and restores: the count
/// of documents, and the terms in bulk, appended in id order and then
/// indexed at once.
impl Dictionary {
    /// The documents started so far ([`Dictionary::new_document`]).
    pub(crate) fn documents(&self) -> u32 {
        self.documents
    }

    /// Restores the count of documents started, so that the next document
    /// gets a scope of its own.
    pub(crate) fn set_documents(&mut self, documents: u32) {
        self.documents = documents;
    }

    /// Appends `term` at the next id without looking it up or indexing it.
    /// [`Dictionary::index_appended`] must follow before any other call.
    ///
    /// # Panics
    /// As `intern_*`.
    pub(crate) fn append(&mut self, term: Term<'_>) {
        let (tag, aux, text, tail) = match term {
            Term::Iri(iri) => (TAG_IRI, 0, iri, ""),
            Term::Blank { doc, label } => (TAG_BLANK, doc.0, label, ""),
            Term::Literal {
                lexical,
                language: Some(language),
                ..
            } => (TAG_LANG, lang_len(language), lexical, language),
            Term::Literal {
                lexical,
                datatype,
                language: None,
            } => (TAG_TYPED, datatype.0, lexical, ""),
        };
        let id = self.entries.len();
        assert!(id < MAX_TERMS, "the dictionary is full ({MAX_TERMS} terms)");
        let len = text.len() + tail.len();
        assert!(len <= MAX_TERM_BYTES, "a term is longer than {MAX_TERM_BYTES} bytes");
        let start = self.text.len() as u64;
        self.text.push_str(text);
        self.text.push_str(tail);
        self.entries.push(Entry {
            start: start | (tag << TAG_SHIFT),
            len: len as u32,
            aux,
        });
    }

    /// Indexes the terms [`Dictionary::append`] added from id `from` on, so
    /// that interning and lookups find them. Hashes them first, then inserts
    /// them in the order of their slots, so that the inserts walk the table
    /// instead of jumping about it. `Err((id, earlier))` when a term repeats
    /// an earlier one: the dictionary is then not to be used.
    pub(crate) fn index_appended(&mut self, from: usize) -> Result<(), (u32, u32)> {
        let want = (self.entries.len() * 2 + 1).next_power_of_two();
        if want > self.slots.len() {
            self.rehash(want);
        }
        let mask = self.slots.len() - 1;
        let hashed: Vec<(u32, u32)> = (from..self.entries.len())
            .map(|id| {
                let e = self.entries[id];
                let text = &self.text[e.range()];
                let (text, tail) = match e.tag() {
                    TAG_LANG => text.split_at(text.len() - e.aux as usize),
                    _ => (text, ""),
                };
                (hash(e.tag(), e.aux, text, tail), id as u32)
            })
            .collect();
        // A counting sort on the top 12 bits of each home slot.
        let bits = mask.count_ones();
        let shift = bits.saturating_sub(12);
        let bucket = |h: u32| (h as usize & mask) >> shift;
        let mut starts = vec![0usize; (mask >> shift) + 2];
        for &(h, _) in &hashed {
            starts[bucket(h) + 1] += 1;
        }
        for b in 1..starts.len() {
            starts[b] += starts[b - 1];
        }
        let mut sorted = vec![(0u32, 0u32); hashed.len()];
        for &(h, id) in &hashed {
            let at = &mut starts[bucket(h)];
            sorted[*at] = (h, id);
            *at += 1;
        }
        drop(hashed);
        for (h, id) in sorted {
            let mut i = h as usize & mask;
            loop {
                let slot = self.slots[i];
                if slot == EMPTY {
                    self.slots[i] = (u64::from(h) << 32) | u64::from(id);
                    break;
                }
                if (slot >> 32) as u32 == h && self.same_entry(slot as u32, id) {
                    return Err((id, slot as u32));
                }
                i = (i + 1) & mask;
            }
        }
        Ok(())
    }

    /// True when ids `a` and `b` hold the same term.
    fn same_entry(&self, a: u32, b: u32) -> bool {
        let (ea, eb) = (self.entries[a as usize], self.entries[b as usize]);
        ea.tag() == eb.tag() && ea.aux == eb.aux && self.text[ea.range()] == self.text[eb.range()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interning_is_idempotent() {
        let mut d = Dictionary::new();
        let doc = d.new_document();
        let a = d.intern_iri("http://example.org/a");
        let b = d.intern_blank(doc, "b");
        let l = d.intern_lang_literal("chat", "fr");
        let t = d.intern_typed_literal("1", vocab::XSD_INTEGER);
        let len = d.len();
        assert_eq!(d.intern_iri("http://example.org/a"), a);
        assert_eq!(d.intern_blank(doc, "b"), b);
        assert_eq!(d.intern_lang_literal("chat", "fr"), l);
        assert_eq!(d.intern_typed_literal("1", vocab::XSD_INTEGER), t);
        assert_eq!(d.len(), len, "no new terms");
        assert_eq!(d.term(a), Term::Iri("http://example.org/a"));
    }

    #[test]
    fn terms_of_different_kinds_with_the_same_text_differ() {
        let mut d = Dictionary::new();
        let text = "http://example.org/x";
        let ids = [
            d.intern_iri(text),
            d.intern_blank(DocId::NONE, text),
            d.intern_typed_literal(text, vocab::XSD_STRING),
            d.intern_typed_literal(text, vocab::XSD_ANY_URI),
            d.intern_lang_literal(text, "en"),
            // The same bytes, split differently between lexical form and tag.
            d.intern_lang_literal("http://example.org/xe", "n"),
        ];
        let mut distinct = ids.to_vec();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), ids.len(), "{ids:?}");
        assert_eq!(d.kind(ids[0]), Kind::Iri);
        assert_eq!(d.kind(ids[1]), Kind::Blank);
        assert_eq!(d.kind(ids[4]), Kind::Literal);
    }

    #[test]
    fn two_documents_blank_nodes_differ() {
        let mut d = Dictionary::new();
        let one = d.new_document();
        let two = d.new_document();
        assert_ne!(one, two);
        let b1 = d.intern_blank(one, "b");
        let b2 = d.intern_blank(two, "b");
        assert_ne!(b1, b2);
        assert_eq!(d.intern_blank(one, "b"), b1);
        let (mut l1, mut l2) = (String::new(), String::new());
        d.write_blank_label(b1, &mut l1);
        d.write_blank_label(b2, &mut l2);
        assert_eq!((l1.as_str(), l2.as_str()), ("b_1", "b_2"), "output labels stay apart");
        assert_eq!(d.term(b2), Term::Blank { doc: two, label: "b" });
    }

    #[test]
    fn blank_nodes_from_no_document_keep_their_label() {
        let mut d = Dictionary::new();
        let doc = d.new_document();
        let label = |d: &Dictionary, id| {
            let mut out = String::new();
            d.write_blank_label(id, &mut out);
            out
        };
        let from_doc = d.intern_blank(doc, "b");
        let inserted = d.intern_blank(DocId::NONE, "b");
        assert_eq!(
            (label(&d, from_doc).as_str(), label(&d, inserted).as_str()),
            ("b_1", "b")
        );
        // A label shaped like a document label gets `_0`, so it never takes one.
        let shaped = d.intern_blank(DocId::NONE, "b_1");
        assert_eq!(label(&d, shaped), "b_1_0");
        for (text, written) in [
            ("b_", "b_"),
            ("b1", "b1"),
            ("_7", "_7_0"),
            ("x_1_0", "x_1_0_0"),
            ("1", "1"),
        ] {
            let id = d.intern_blank(DocId::NONE, text);
            assert_eq!(label(&d, id), written, "{text}");
        }
    }

    #[test]
    fn a_lang_string_without_a_tag_is_kept_as_it_came() {
        use oxrdf::{Literal, vocab::rdf};
        let mut d = Dictionary::new();
        let untagged = Literal::new_typed_literal("x", rdf::LANG_STRING);
        let id = d.intern_literal(untagged.as_ref());
        assert_eq!(
            d.term(id),
            Term::Literal {
                lexical: "x",
                datatype: vocab::RDF_LANG_STRING,
                language: None
            }
        );
        assert_eq!(d.to_oxrdf(id), untagged.clone().into());
        assert_ne!(id, d.intern_lang_literal("x", "en"));
        assert_eq!(d.intern_literal(untagged.as_ref()), id);
    }

    #[test]
    fn a_literal_keeps_datatype_and_language() {
        use oxrdf::vocab::xsd;
        let mut d = Dictionary::new();
        let fr = d.intern_literal(LiteralRef::new_language_tagged_literal_unchecked("chat", "fr"));
        let en = d.intern_literal(LiteralRef::new_language_tagged_literal_unchecked("chat", "en"));
        let plain = d.intern_literal(LiteralRef::new_simple_literal("chat"));
        let int01 = d.intern_literal(LiteralRef::new_typed_literal("01", xsd::INTEGER));
        let int1 = d.intern_literal(LiteralRef::new_typed_literal("1", xsd::INTEGER));
        let custom = d.intern_literal(LiteralRef::new_typed_literal(
            "chat",
            NamedNodeRef::new_unchecked("http://example.org/dt"),
        ));
        let all = [fr, en, plain, int01, int1, custom];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
        let lang = |lexical, language| Term::Literal {
            lexical,
            datatype: vocab::RDF_LANG_STRING,
            language: Some(language),
        };
        assert_eq!(d.term(fr), lang("chat", "fr"));
        assert_eq!(d.term(en), lang("chat", "en"));
        let typed = |lexical, datatype| Term::Literal {
            lexical,
            datatype,
            language: None,
        };
        assert_eq!(d.term(plain), typed("chat", vocab::XSD_STRING));
        assert_eq!(d.term(int01), typed("01", vocab::XSD_INTEGER));
        let Term::Literal { datatype, .. } = d.term(custom) else {
            panic!("a literal")
        };
        assert_eq!(d.iri(datatype), Some("http://example.org/dt"));
        // Back to oxrdf unchanged.
        assert_eq!(
            d.to_oxrdf(fr),
            oxrdf::Literal::new_language_tagged_literal_unchecked("chat", "fr").into()
        );
        assert_eq!(d.to_oxrdf(plain), oxrdf::Literal::new_simple_literal("chat").into());
        assert_eq!(
            d.to_oxrdf(int01),
            oxrdf::Literal::new_typed_literal("01", xsd::INTEGER).into()
        );
        // Lookup without interning.
        let fr_ref = LiteralRef::new_language_tagged_literal_unchecked("chat", "fr");
        assert_eq!(d.get_term(fr_ref.into(), DocId::NONE), Some(fr));
        let de = LiteralRef::new_language_tagged_literal_unchecked("chat", "de");
        assert_eq!(d.get_term(de.into(), DocId::NONE), None);
    }

    #[test]
    fn every_vocabulary_id_is_fixed() {
        let d = Dictionary::new();
        assert_eq!(d.len(), vocab::IRIS.len(), "the vocabulary lists each IRI once");
        assert_eq!(vocab::NAMES.len(), vocab::IRIS.len());
        for (i, iri) in vocab::IRIS.iter().enumerate() {
            assert_eq!(d.get_iri(iri), Some(TermId(i as u32)), "{}", vocab::NAMES[i]);
        }
        let check = [
            (vocab::RDF_TYPE, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            (
                vocab::RDF_LANG_STRING,
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString",
            ),
            (
                vocab::RDFS_SUB_CLASS_OF,
                "http://www.w3.org/2000/01/rdf-schema#subClassOf",
            ),
            (vocab::OWL_SAME_AS, "http://www.w3.org/2002/07/owl#sameAs"),
            (vocab::OWL_THING, "http://www.w3.org/2002/07/owl#Thing"),
            (
                vocab::OWL_PROPERTY_CHAIN_AXIOM,
                "http://www.w3.org/2002/07/owl#propertyChainAxiom",
            ),
            (vocab::XSD_STRING, vocab::XSD_STRING_IRI),
            (vocab::XSD_INT, "http://www.w3.org/2001/XMLSchema#int"),
        ];
        for (id, iri) in check {
            assert_eq!(d.iri(id), Some(iri));
        }
        assert_eq!(vocab::RDF_TYPE, TermId(0));
        // Whatever is interned later, the vocabulary keeps its ids.
        let mut other = Dictionary::new();
        other.intern_iri("http://example.org/first");
        for (i, iri) in vocab::IRIS.iter().enumerate() {
            assert_eq!(other.intern_iri(iri), TermId(i as u32));
        }
    }

    /// Every IRI the rules of version 2 name (the phase 1 spec, "The
    /// vocabulary"), every OWL 2 RL datatype (OWL 2 Profiles §4.2) and the
    /// annotation properties of prp-ap are vocabulary constants.
    #[test]
    fn the_vocabulary_holds_every_iri_the_rules_name() {
        const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
        const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
        const OWL: &str = "http://www.w3.org/2002/07/owl#";
        const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
        let rules = [
            (OWL, "hasKey"),
            (OWL, "maxCardinality"),
            (OWL, "maxQualifiedCardinality"),
            (OWL, "onClass"),
            (OWL, "onDataRange"),
            (OWL, "oneOf"),
            (OWL, "differentFrom"),
            (OWL, "AllDifferent"),
            (OWL, "members"),
            (OWL, "distinctMembers"),
            (OWL, "AllDisjointClasses"),
            (OWL, "AllDisjointProperties"),
            (OWL, "NegativePropertyAssertion"),
            (OWL, "sourceIndividual"),
            (OWL, "assertionProperty"),
            (OWL, "targetIndividual"),
            (OWL, "targetValue"),
            (OWL, "ObjectProperty"),
            (OWL, "DatatypeProperty"),
            (OWL, "AnnotationProperty"),
            (RDFS, "Datatype"),
        ];
        let annotation_properties = [
            (RDFS, "label"),
            (RDFS, "comment"),
            (RDFS, "seeAlso"),
            (RDFS, "isDefinedBy"),
            (OWL, "deprecated"),
            (OWL, "versionInfo"),
            (OWL, "priorVersion"),
            (OWL, "backwardCompatibleWith"),
            (OWL, "incompatibleWith"),
        ];
        let datatypes = [
            (RDF, "PlainLiteral"),
            (RDF, "XMLLiteral"),
            (RDFS, "Literal"),
            (XSD, "decimal"),
            (XSD, "integer"),
            (XSD, "nonNegativeInteger"),
            (XSD, "nonPositiveInteger"),
            (XSD, "positiveInteger"),
            (XSD, "negativeInteger"),
            (XSD, "long"),
            (XSD, "int"),
            (XSD, "short"),
            (XSD, "byte"),
            (XSD, "unsignedLong"),
            (XSD, "unsignedInt"),
            (XSD, "unsignedShort"),
            (XSD, "unsignedByte"),
            (XSD, "float"),
            (XSD, "double"),
            (XSD, "string"),
            (XSD, "normalizedString"),
            (XSD, "token"),
            (XSD, "language"),
            (XSD, "Name"),
            (XSD, "NCName"),
            (XSD, "NMTOKEN"),
            (XSD, "boolean"),
            (XSD, "hexBinary"),
            (XSD, "base64Binary"),
            (XSD, "anyURI"),
            (XSD, "dateTime"),
            (XSD, "dateTimeStamp"),
        ];
        let d = Dictionary::new();
        for (ns, local) in rules.iter().chain(&annotation_properties).chain(&datatypes) {
            let iri = format!("{ns}{local}");
            assert!(
                d.get_iri(&iri).is_some_and(|id| id.index() < vocab::IRIS.len()),
                "{iri} is not in the vocabulary"
            );
        }
    }

    #[test]
    fn many_terms_survive_growth() {
        let mut d = Dictionary::new();
        let doc = d.new_document();
        let ids: Vec<_> = (0..100_000)
            .map(|i| {
                let s = format!("http://example.org/{i}");
                (
                    d.intern_iri(&s),
                    d.intern_blank(doc, &s),
                    d.intern_lang_literal(&s, "en"),
                )
            })
            .collect();
        assert_eq!(d.len(), vocab::IRIS.len() + 300_000);
        for (i, &(iri, blank, lit)) in ids.iter().enumerate() {
            let s = format!("http://example.org/{i}");
            assert_eq!(d.intern_iri(&s), iri);
            assert_eq!(d.intern_blank(doc, &s), blank);
            assert_eq!(d.intern_lang_literal(&s, "en"), lit);
            assert_eq!(d.term(iri), Term::Iri(&s));
        }
        let mut reserved = Dictionary::new();
        reserved.reserve(1000, 10_000);
        assert_eq!(
            reserved.intern_iri("http://example.org/a"),
            TermId(vocab::IRIS.len() as u32)
        );
        assert_eq!(reserved.get_iri(vocab::XSD_STRING_IRI), Some(vocab::XSD_STRING));
    }

    #[test]
    fn decimal_labels() {
        for n in [0, 7, 10, 4_294_967_295] {
            let mut s = String::new();
            push_decimal(&mut s, n);
            assert_eq!(s, n.to_string());
        }
    }
}
