//! Parsing and writing documents.
//!
//! Turtle, N-Triples and N3 are read with `oxttl`, RDF/XML with `oxrdfxml`,
//! straight into the [`Dictionary`]: each triple is interned and handed to
//! the caller's sink as `[subject, predicate, object]` term ids, with no
//! intermediate list of owned triples. Turtle and N-Triples are written with
//! `oxttl` (N3 is written as Turtle, which N3 reads) and RDF/XML with
//! `oxrdfxml`, from term ids, language tags kept.
//!
//! # Interface (for `reasoner.rs`; `parse_file` is for the tests, since the
//! reasoner reads a file itself to key the document)
//!
//! ```text
//! format_of(path: &Path) -> Result<Format, Error>
//! parse_file(dict: &mut Dictionary, path: &Path, format: Option<Format>,
//!            base_iri: Option<&str>, sink: impl FnMut([TermId; 3]))
//!     -> Result<Loaded, Error>
//! parse(dict: &mut Dictionary, text: &[u8], format: Format,
//!       base_iri: Option<&str>, path: Option<&Path>, sink: impl FnMut([TermId; 3]))
//!     -> Result<Loaded, Error>
//! write(dict: &Dictionary, triples: impl IntoIterator<Item = [TermId; 3]>,
//!       out: impl Write, format: Format) -> Result<usize, Error>
//! ```
//!
//! - Each `parse` or `parse_file` call is one document: it starts a new
//!   [`DocId`] ([`Loaded::document`]), so its blank nodes are its own.
//! - The sink sees every triple in document order, duplicates included. It
//!   runs while the parse goes on, so after an error it has seen the triples
//!   before the error: a caller that wants all or nothing buffers them and
//!   commits on `Ok`. Terms interned before an error stay in the dictionary
//!   (harmless: unused ids).
//! - `parse_file` reads the whole file, then parses the bytes. Its format is
//!   `format`, or else the file's extension ([`format_of`]). In `parse`,
//!   `path` only labels errors.
//! - `base_iri` resolves relative IRIs (Turtle, N3, RDF/XML; N-Triples has
//!   none). Without one a relative IRI is a parse error.
//! - N3: only statements of the default graph whose terms are RDF terms are
//!   kept (an IRI or blank-node subject, an IRI predicate, no variable).
//!   Statements inside formulas, with variables or with literal subjects are
//!   counted in [`Loaded::skipped`].
//! - Errors: [`Error::Parse`] with the path, the 1-based line and column of
//!   the error and the parser's message (RDF/XML: the position the reader
//!   had reached; line 0 for errors that are not at a position, such as an
//!   invalid base IRI); [`Error::Io`] when a file cannot be read or the
//!   output cannot be written; [`Error::UnknownFormat`] for an extension
//!   that names no format. Nothing here panics on bad input.
//! - `write` writes the triples in the order given and returns how many it
//!   wrote. A triple that is not RDF (a literal subject, or a predicate that
//!   is not an IRI) is skipped. Turtle output declares the `rdf`, `rdfs`,
//!   `owl` and `xsd` prefixes and groups consecutive triples that share a
//!   subject, so sort by subject first for compact output. Blank nodes are
//!   labelled `<label>_<document>` ([`Dictionary::write_blank_label`]).

use crate::term::{Dictionary, DocId, MAX_TERM_BYTES, MAX_TERMS, Term, TermId, vocab};
use crate::{Error, Format};
use oxrdf::{BlankNodeRef, GraphName, NamedNodeRef, NamedOrBlankNodeRef, TermRef, TripleRef};
use oxrdfxml::{RdfXmlParser, RdfXmlSerializer};
use oxttl::n3::{N3Parser, N3Quad, N3Term};
use oxttl::{NTriplesParser, NTriplesSerializer, TurtleParser, TurtleSerializer, TurtleSyntaxError};
use std::fmt::Display;
use std::io::{BufWriter, Write};
use std::path::Path;

/// What one parse added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Loaded {
    /// The document its blank nodes belong to.
    pub(crate) document: DocId,
    /// Triples handed to the sink (duplicates included).
    pub(crate) triples: usize,
    /// N3 statements that are not RDF triples, left out.
    pub(crate) skipped: usize,
}

/// The format of a file, from its extension.
pub(crate) fn format_of(path: &Path) -> Result<Format, Error> {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(Format::from_extension)
        .ok_or_else(|| Error::UnknownFormat { path: path.into() })
}

/// Reads and parses a file: one document.
#[cfg(test)]
pub(crate) fn parse_file(
    dict: &mut Dictionary,
    path: &Path,
    format: Option<Format>,
    base_iri: Option<&str>,
    sink: impl FnMut([TermId; 3]),
) -> Result<Loaded, Error> {
    let format = match format {
        Some(format) => format,
        None => format_of(path)?,
    };
    let text = std::fs::read(path).map_err(|source| Error::Io {
        path: Some(path.into()),
        source,
    })?;
    parse(dict, &text, format, base_iri, Some(path), sink)
}

/// Parses a document held in memory: one document. `path` only labels errors.
pub(crate) fn parse(
    dict: &mut Dictionary,
    text: &[u8],
    format: Format,
    base_iri: Option<&str>,
    path: Option<&Path>,
    sink: impl FnMut([TermId; 3]),
) -> Result<Loaded, Error> {
    let mut load = Load {
        dict,
        document: DocId::NONE,
        sink,
        triples: 0,
        path,
    };
    load.document = load.dict.new_document();
    let mut skipped = 0;
    let syntax = |e: TurtleSyntaxError| {
        let at = e.location().start;
        error_at(path, at.line + 1, at.column + 1, e.message())
    };
    let bad_base = |e: &dyn Display| error_at(path, 0, 0, format_args!("invalid base IRI: {e}"));
    match format {
        Format::NTriples => {
            for triple in NTriplesParser::new().for_slice(text) {
                load.triple(triple.map_err(syntax)?.as_ref())?;
            }
        }
        Format::Turtle => {
            let mut parser = TurtleParser::new();
            if let Some(base) = base_iri {
                parser = parser.with_base_iri(base).map_err(|e| bad_base(&e))?;
            }
            for triple in parser.for_slice(text) {
                load.triple(triple.map_err(syntax)?.as_ref())?;
            }
        }
        Format::N3 => {
            let mut parser = N3Parser::new();
            if let Some(base) = base_iri {
                parser = parser.with_base_iri(base).map_err(|e| bad_base(&e))?;
            }
            for quad in parser.for_slice(text) {
                match n3_triple(&quad.map_err(syntax)?) {
                    Some(triple) => load.triple(triple)?,
                    None => skipped += 1,
                }
            }
        }
        Format::RdfXml => {
            let mut parser = RdfXmlParser::new();
            if let Some(base) = base_iri {
                parser = parser.with_base_iri(base).map_err(|e| bad_base(&e))?;
            }
            let mut triples = parser.for_slice(text);
            while let Some(triple) = triples.next() {
                match triple {
                    Ok(triple) => load.triple(triple.as_ref())?,
                    Err(e) => {
                        let (line, column) = line_column(text, triples.buffer_position());
                        return Err(error_at(path, line, column, e));
                    }
                }
            }
        }
    }
    Ok(Loaded {
        document: load.document,
        triples: load.triples,
        skipped,
    })
}

/// One document's parse into the dictionary.
struct Load<'a, 'p, S> {
    dict: &'a mut Dictionary,
    document: DocId,
    sink: S,
    triples: usize,
    path: Option<&'p Path>,
}

impl<S: FnMut([TermId; 3])> Load<'_, '_, S> {
    #[inline]
    fn triple(&mut self, t: TripleRef<'_>) -> Result<(), Error> {
        if self.dict.len() > MAX_TERMS - 3 {
            return Err(error_at(
                self.path,
                0,
                0,
                format_args!("the dictionary is full ({MAX_TERMS} terms)"),
            ));
        }
        // A term can be longer than its document (prefixes, a base IRI,
        // RDF/XML entities), so every triple is checked.
        if too_long(t) {
            return Err(error_at(
                self.path,
                0,
                0,
                format_args!("a term is longer than {MAX_TERM_BYTES} bytes"),
            ));
        }
        let ids = [
            self.dict.intern_subject(t.subject, self.document),
            self.dict.intern_iri(t.predicate.as_str()),
            self.dict.intern_term(t.object, self.document),
        ];
        (self.sink)(ids);
        self.triples += 1;
        Ok(())
    }
}

#[inline]
fn too_long(t: TripleRef<'_>) -> bool {
    let subject = match t.subject {
        NamedOrBlankNodeRef::NamedNode(n) => n.as_str().len(),
        NamedOrBlankNodeRef::BlankNode(b) => b.as_str().len(),
    };
    let object = match t.object {
        TermRef::NamedNode(n) => n.as_str().len(),
        TermRef::BlankNode(b) => b.as_str().len(),
        TermRef::Literal(l) => l.value().len() + l.language().map_or(0, str::len),
    };
    subject.max(object).max(t.predicate.as_str().len()) > MAX_TERM_BYTES
}

/// An N3 statement as an RDF triple, if it is one.
fn n3_triple(quad: &N3Quad) -> Option<TripleRef<'_>> {
    if quad.graph_name != GraphName::DefaultGraph {
        return None;
    }
    let subject: NamedOrBlankNodeRef<'_> = match &quad.subject {
        N3Term::NamedNode(n) => n.into(),
        N3Term::BlankNode(b) => b.into(),
        _ => return None,
    };
    let N3Term::NamedNode(predicate) = &quad.predicate else {
        return None;
    };
    let object: TermRef<'_> = match &quad.object {
        N3Term::NamedNode(n) => n.into(),
        N3Term::BlankNode(b) => b.into(),
        N3Term::Literal(l) => l.into(),
        _ => return None,
    };
    Some(TripleRef::new(subject, predicate, object))
}

fn error_at(path: Option<&Path>, line: u64, column: u64, message: impl Display) -> Error {
    Error::Parse {
        path: path.map(Into::into),
        line,
        column,
        message: message.to_string(),
    }
}

/// The 1-based line and column (in characters) of a byte offset.
fn line_column(text: &[u8], offset: u64) -> (u64, u64) {
    let before = &text[..usize::try_from(offset).unwrap_or(usize::MAX).min(text.len())];
    let line_start = before.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    let line = before.iter().filter(|&&b| b == b'\n').count() as u64 + 1;
    // Count characters, not bytes: skip UTF-8 continuation bytes.
    let column = before[line_start..].iter().filter(|&&b| b & 0xC0 != 0x80).count() as u64 + 1;
    (line, column)
}

/// Writes triples as `format`; returns how many were written. A triple that
/// is not RDF (a literal subject, a predicate that is not an IRI) is skipped.
pub(crate) fn write(
    dict: &Dictionary,
    triples: impl IntoIterator<Item = [TermId; 3]>,
    out: impl Write,
    format: Format,
) -> Result<usize, Error> {
    let io = |source| Error::Io { path: None, source };
    let mut out = BufWriter::with_capacity(1 << 16, out);
    let mut scratch = Scratch::default();
    let mut written = 0;
    match format {
        Format::NTriples => {
            let mut writer = NTriplesSerializer::new().for_writer(&mut out);
            for t in triples {
                if let Some(triple) = scratch.triple(dict, t) {
                    writer.serialize_triple(triple).map_err(io)?;
                    written += 1;
                }
            }
            writer.finish();
        }
        Format::Turtle | Format::N3 => {
            let mut serializer = TurtleSerializer::new();
            for (prefix, iri) in vocab::PREFIXES {
                serializer = serializer
                    .with_prefix(*prefix, *iri)
                    .map_err(|e| io(std::io::Error::other(e)))?;
            }
            let mut writer = serializer.for_writer(&mut out);
            for t in triples {
                if let Some(triple) = scratch.triple(dict, t) {
                    writer.serialize_triple(triple).map_err(io)?;
                    written += 1;
                }
            }
            writer.finish().map_err(io)?;
        }
        Format::RdfXml => {
            let mut serializer = RdfXmlSerializer::new();
            for (prefix, iri) in vocab::PREFIXES {
                serializer = serializer
                    .with_prefix(*prefix, *iri)
                    .map_err(|e| io(std::io::Error::other(e)))?;
            }
            let mut writer = serializer.for_writer(&mut out);
            for t in triples {
                if let Some(triple) = scratch.triple(dict, t) {
                    writer.serialize_triple(triple).map_err(io)?;
                    written += 1;
                }
            }
            writer.finish().map_err(io)?;
        }
    }
    out.flush().map_err(io)?;
    Ok(written)
}

/// Buffers for the output labels of a triple's blank nodes.
#[derive(Default)]
struct Scratch {
    subject: String,
    object: String,
}

impl Scratch {
    /// A triple of ids as an `oxrdf` triple borrowing the dictionary and
    /// these buffers; `None` when it is not an RDF triple.
    #[inline]
    fn triple<'a>(&'a mut self, dict: &'a Dictionary, [s, p, o]: [TermId; 3]) -> Option<TripleRef<'a>> {
        let subject: NamedOrBlankNodeRef<'a> = match dict.term(s) {
            Term::Iri(iri) => NamedNodeRef::new_unchecked(iri).into(),
            Term::Blank { .. } => {
                dict.write_blank_label(s, &mut self.subject);
                BlankNodeRef::new_unchecked(&self.subject).into()
            }
            Term::Literal { .. } => return None,
        };
        let predicate = NamedNodeRef::new_unchecked(dict.iri(p)?);
        let object = dict.to_oxrdf_ref(o, &mut self.object);
        Some(TripleRef::new(subject, predicate, object))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxrdf::{Graph, Triple};
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/io")
            .join(name)
    }

    fn load(dict: &mut Dictionary, text: &str, format: Format) -> Result<Vec<[TermId; 3]>, Error> {
        let mut out = Vec::new();
        parse(dict, text.as_bytes(), format, None, None, |t| out.push(t))?;
        Ok(out)
    }

    /// The triples as an `oxrdf` graph with canonical blank-node labels.
    fn graph(dict: &Dictionary, triples: &[[TermId; 3]]) -> Graph {
        let mut graph = Graph::new();
        let mut scratch = Scratch::default();
        for &t in triples {
            let triple: Triple = scratch.triple(dict, t).expect("an RDF triple").into_owned();
            graph.insert(&triple);
        }
        graph.canonicalize(oxrdf::dataset::CanonicalizationAlgorithm::Unstable);
        graph
    }

    fn written(dict: &Dictionary, triples: &[[TermId; 3]], format: Format) -> String {
        let mut out = Vec::new();
        let n = write(dict, triples.iter().copied(), &mut out, format).unwrap();
        assert_eq!(n, triples.len());
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn round_trip_every_format() {
        for (name, format) in [
            ("sample.ttl", Format::Turtle),
            ("sample.nt", Format::NTriples),
            ("sample.n3", Format::N3),
            ("sample.rdf", Format::RdfXml),
        ] {
            let mut dict = Dictionary::new();
            let mut base = Vec::new();
            let loaded = parse_file(&mut dict, &fixture(name), Some(format), None, |t| base.push(t)).unwrap();
            assert_eq!(loaded.triples, base.len());
            assert!(base.len() >= 8, "{name}: {} triples", base.len());
            let expected = graph(&dict, &base);
            for out in [Format::Turtle, Format::NTriples, Format::N3, Format::RdfXml] {
                let text = written(&dict, &base, out);
                let mut again = Vec::new();
                parse(&mut dict, text.as_bytes(), out, None, None, |t| again.push(t))
                    .unwrap_or_else(|e| panic!("{name} as {out:?}: {e}\n{text}"));
                assert_eq!(graph(&dict, &again), expected, "{name} as {out:?}:\n{text}");
            }
        }
    }

    #[test]
    fn the_four_formats_read_the_same_graph() {
        let mut dict = Dictionary::new();
        let mut graphs = Vec::new();
        for name in ["sample.ttl", "sample.nt", "sample.n3", "sample.rdf"] {
            let mut triples = Vec::new();
            parse_file(&mut dict, &fixture(name), None, None, |t| triples.push(t)).unwrap();
            graphs.push((name, graph(&dict, &triples)));
        }
        for (name, g) in &graphs[1..] {
            assert_eq!(g, &graphs[0].1, "{name} and sample.ttl");
        }
    }

    #[test]
    fn language_tags_and_datatypes_survive_output() {
        let mut dict = Dictionary::new();
        let triples = load(
            &mut dict,
            r#"<http://example.org/a> <http://example.org/p> "chat"@fr, "7"^^<http://www.w3.org/2001/XMLSchema#integer>, "plain" ."#,
            Format::Turtle,
        )
        .unwrap();
        let nt = written(&dict, &triples, Format::NTriples);
        assert!(nt.contains("\"chat\"@fr"), "{nt}");
        assert!(nt.contains("\"7\"^^<http://www.w3.org/2001/XMLSchema#integer>"), "{nt}");
        assert!(nt.contains("\"plain\" ."), "plain literals stay plain: {nt}");
        let ttl = written(&dict, &triples, Format::Turtle);
        assert!(ttl.contains("\"chat\"@fr"), "{ttl}");
    }

    #[test]
    fn two_documents_blank_nodes_stay_apart() {
        let mut dict = Dictionary::new();
        let text = "_:b <http://example.org/p> <http://example.org/o> .";
        let one = load(&mut dict, text, Format::NTriples).unwrap();
        let two = load(&mut dict, text, Format::NTriples).unwrap();
        assert_ne!(one[0][0], two[0][0]);
        assert_eq!(one[0][1], two[0][1]);
        assert_eq!(one[0][2], two[0][2]);
        // Within a document a label is one node.
        let same = load(
            &mut dict,
            &format!("{text}\n_:b <http://example.org/q> _:b ."),
            Format::NTriples,
        )
        .unwrap();
        assert_eq!(same[0][0], same[1][0]);
        assert_eq!(same[1][0], same[1][2]);
        // On output the two documents' nodes keep distinct labels.
        let all: Vec<_> = one.iter().chain(&two).copied().collect();
        let nt = written(&dict, &all, Format::NTriples);
        let mut again = Dictionary::new();
        let back = load(&mut again, &nt, Format::NTriples).unwrap();
        assert_ne!(back[0][0], back[1][0], "{nt}");
    }

    #[test]
    fn a_malformed_file_names_the_file_and_position() {
        for (name, line, column) in [
            ("malformed.ttl", 4, 28),
            ("malformed.nt", 2, 47),
            // RDF/XML: where the reader stopped, the end of the bad end tag.
            ("malformed.rdf", 9, 14),
        ] {
            let path = fixture(name);
            let mut seen = 0;
            let err = parse_file(&mut Dictionary::new(), &path, None, None, |_| seen += 1).unwrap_err();
            let Error::Parse {
                path: Some(ref at),
                line: l,
                column: c,
                ref message,
            } = err
            else {
                panic!("{name}: {err:?}")
            };
            assert_eq!(at, &path);
            assert!(!message.is_empty());
            assert_eq!((l, c), (line, column), "{name}: {message}");
            assert!(err.to_string().starts_with(&path.display().to_string()), "{err}");
            assert!(seen >= 1, "{name}: the triples before the error reach the sink");
        }
    }

    #[test]
    fn other_errors() {
        let mut dict = Dictionary::new();
        let none = |_| {};
        let missing = fixture("missing.ttl");
        assert!(matches!(
            parse_file(&mut dict, &missing, None, None, none),
            Err(Error::Io { path: Some(p), .. }) if p == missing
        ));
        assert!(matches!(
            parse_file(&mut dict, Path::new("data.csv"), None, None, none),
            Err(Error::UnknownFormat { .. })
        ));
        assert!(matches!(
            parse(
                &mut dict,
                b"<a> <b> <c> .",
                Format::Turtle,
                Some("not an iri"),
                None,
                none
            ),
            Err(Error::Parse { line: 0, .. })
        ));
        // A relative IRI needs a base.
        assert!(matches!(
            parse(&mut dict, b"<a> <b> <c> .", Format::Turtle, None, None, none),
            Err(Error::Parse { line: 1, .. })
        ));
        let mut triples = Vec::new();
        parse(
            &mut dict,
            b"<a> <b> <c> .",
            Format::Turtle,
            Some("http://example.org/"),
            None,
            |t| triples.push(t),
        )
        .unwrap();
        assert_eq!(dict.iri(triples[0][0]), Some("http://example.org/a"));
        // Bytes that are not UTF-8.
        assert!(matches!(
            parse(
                &mut dict,
                b"<http://e/\xff> <http://e/p> <http://e/o> .",
                Format::NTriples,
                None,
                None,
                none
            ),
            Err(Error::Parse { .. })
        ));
        assert_eq!(format_of(Path::new("x.OWL")).unwrap(), Format::RdfXml);
    }

    #[test]
    fn n3_keeps_only_rdf_triples() {
        let mut dict = Dictionary::new();
        let mut triples = Vec::new();
        let text = "@prefix : <http://example.org/> .
            :a :p :b .
            { ?x :p ?y } => { ?y :q ?x } .
            :c = :d .";
        let loaded = parse(&mut dict, text.as_bytes(), Format::N3, None, None, |t| triples.push(t)).unwrap();
        // `:a :p :b`, `:c owl:sameAs :d`, and the rule's `_:f1 log:implies _:f2`.
        assert_eq!(loaded.triples, 3, "{triples:?}");
        assert_eq!(loaded.skipped, 2, "the two statements inside formulas");
        assert!(triples.iter().any(|t| t[1] == vocab::OWL_SAME_AS));
    }

    #[test]
    fn non_rdf_triples_are_not_written() {
        let mut dict = Dictionary::new();
        let lit = dict.intern_lang_literal("x", "en");
        let iri = dict.intern_iri("http://example.org/a");
        let mut out = Vec::new();
        let n = write(
            &dict,
            [[lit, vocab::RDF_TYPE, iri], [iri, lit, iri]],
            &mut out,
            Format::NTriples,
        )
        .unwrap();
        assert_eq!(n, 0);
        assert!(out.is_empty());
    }

    #[test]
    fn line_and_column_count_characters() {
        let text = "ab\ncdé\nfg".as_bytes();
        assert_eq!(line_column(text, 0), (1, 1));
        assert_eq!(line_column(text, 3), (2, 1));
        assert_eq!(line_column(text, 7), (2, 4), "é is one character");
        assert_eq!(line_column(text, 1000), (3, 3));
    }

    /// Loads a generated 1,000,000-triple N-Triples document into the
    /// dictionary and prints triples per second, beside the parser alone.
    #[test]
    fn timing_intern_load() {
        // Not beside a scaling test: neither would time alone.
        let _lock = crate::scaling::lock();
        const TRIPLES: usize = 1_000_000;
        let mut text = String::with_capacity(TRIPLES * 90);
        for i in 0..TRIPLES {
            use std::fmt::Write as _;
            let s = i % 200_000;
            let p = i % 50;
            let _ = match i % 4 {
                0 | 1 => writeln!(
                    text,
                    "<http://example.org/s{s}> <http://example.org/p{p}> <http://example.org/s{}> .",
                    (i * 7) % 200_000
                ),
                2 => writeln!(
                    text,
                    "<http://example.org/s{s}> <http://example.org/p{p}> \"{i}\"^^<http://www.w3.org/2001/XMLSchema#integer> ."
                ),
                _ => writeln!(text, "_:b{s} <http://example.org/p{p}> \"label {}\"@en .", i % 1000),
            };
        }
        let bytes = text.len();

        let start = std::time::Instant::now();
        let mut count = 0;
        for triple in NTriplesParser::new().for_slice(&text) {
            triple.unwrap();
            count += 1;
        }
        let parse_only = start.elapsed();
        assert_eq!(count, TRIPLES);

        let start = std::time::Instant::now();
        let mut dict = Dictionary::new();
        let mut triples = Vec::with_capacity(TRIPLES);
        let loaded = parse(&mut dict, text.as_bytes(), Format::NTriples, None, None, |t| {
            triples.push(t)
        })
        .unwrap();
        let full = start.elapsed();
        assert_eq!(loaded.triples, TRIPLES);

        let rate = |d: std::time::Duration| TRIPLES as f64 / d.as_secs_f64();
        println!(
            "timing_intern_load: {TRIPLES} triples, {:.1} MB, {} terms: \
             parse+intern {:.0} ms = {:.2} M triples/s; oxttl parse alone {:.0} ms = {:.2} M triples/s; \
             interning is {:.0}% of the load",
            bytes as f64 / 1e6,
            dict.len(),
            full.as_secs_f64() * 1e3,
            rate(full) / 1e6,
            parse_only.as_secs_f64() * 1e3,
            rate(parse_only) / 1e6,
            100.0 * (full.as_secs_f64() - parse_only.as_secs_f64()).max(0.0) / full.as_secs_f64(),
        );
    }
}
