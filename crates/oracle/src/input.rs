//! The input every oracle receives: documents parsed once with Oxigraph's
//! parsers, blank nodes scoped per document, triples deduplicated.

use oxrdf::{BlankNode, NamedNode, NamedOrBlankNode, Term, Triple};
use oxrdfxml::RdfXmlParser;
use oxttl::{NTriplesParser, TurtleParser};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;
use turbowl::Format;

/// One parsed document.
#[derive(Debug, Clone)]
pub struct Document {
    /// A file name or a fixture name.
    pub name: String,
    pub format: Format,
    /// The document's own text, as given.
    pub text: String,
}

/// The documents of one oracle run and their triples.
#[derive(Debug, Clone, Default)]
pub struct Input {
    pub name: String,
    documents: Vec<Document>,
    triples: Vec<Triple>,
    seen: HashSet<Triple>,
}

impl Input {
    /// An input with no documents.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// One Turtle document (fixtures; panics on a syntax error).
    pub fn turtle(name: impl Into<String>, text: &str) -> Self {
        let name = name.into();
        Self::new(name.clone())
            .with_document(name, text, Format::Turtle)
            .unwrap_or_else(|e| panic!("{e}"))
    }

    /// The files, in order (format from each extension; `.n3` is read as Turtle,
    /// a superset of the N-Triples that the `.n3` models under `data/` hold).
    pub fn files(name: impl Into<String>, paths: &[&Path]) -> Result<Self, String> {
        let mut input = Self::new(name);
        for path in paths {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
            let format = Format::from_extension(ext).ok_or_else(|| format!("{}: unknown format", path.display()))?;
            let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            input = input.with_document(path.display().to_string(), &text, format)?;
        }
        Ok(input)
    }

    /// One document of triples given as they are (generated inputs): blank
    /// nodes keep their labels, duplicates are dropped.
    pub fn from_triples(name: impl Into<String>, triples: impl IntoIterator<Item = Triple>) -> Self {
        let name = name.into();
        let mut input = Self::new(name.clone());
        for t in triples {
            if input.seen.insert(t.clone()) {
                input.triples.push(t);
            }
        }
        let text = input.to_ntriples();
        input.documents.push(Document {
            name,
            format: Format::NTriples,
            text,
        });
        input
    }

    /// Adds a document. Its blank nodes are scoped to it and relabelled in
    /// order of appearance (`b0`, `b1`, ... in the first document, `d1b0`, ...
    /// in the second), so every parse of the same text gives the same labels
    /// (the parsers label anonymous nodes at random).
    pub fn with_document(mut self, name: impl Into<String>, text: &str, format: Format) -> Result<Self, String> {
        let name = name.into();
        let n = self.documents.len();
        let mut labels: HashMap<BlankNode, BlankNode> = HashMap::new();
        let mut scope = |b: BlankNode| {
            let next = labels.len();
            labels
                .entry(b)
                .or_insert_with(|| {
                    BlankNode::new_unchecked(if n == 0 {
                        format!("b{next}")
                    } else {
                        format!("d{n}b{next}")
                    })
                })
                .clone()
        };
        let parsed: Vec<Triple> = match format {
            Format::Turtle | Format::N3 => TurtleParser::new()
                .for_slice(text)
                .collect::<Result<_, _>>()
                .map_err(|e| format!("{name}: {e}"))?,
            Format::NTriples => NTriplesParser::new()
                .for_slice(text)
                .collect::<Result<_, _>>()
                .map_err(|e| format!("{name}: {e}"))?,
            Format::RdfXml => RdfXmlParser::new()
                .for_slice(text)
                .collect::<Result<_, _>>()
                .map_err(|e| format!("{name}: {e}"))?,
        };
        for t in parsed {
            let subject = match t.subject {
                NamedOrBlankNode::BlankNode(b) => scope(b).into(),
                s => s,
            };
            let object = match t.object {
                Term::BlankNode(b) => scope(b).into(),
                o => o,
            };
            let t = Triple::new(subject, t.predicate, object);
            if self.seen.insert(t.clone()) {
                self.triples.push(t);
            }
        }
        self.documents.push(Document {
            name,
            format,
            text: text.to_owned(),
        });
        Ok(self)
    }

    /// The documents, in order.
    pub fn documents(&self) -> &[Document] {
        &self.documents
    }

    /// The triples of every document, deduplicated, in input order.
    pub fn triples(&self) -> &[Triple] {
        &self.triples
    }

    /// The triples as N-Triples text.
    pub fn to_ntriples(&self) -> String {
        ntriples(self.triples.iter())
    }
}

/// Triples as N-Triples text, one per line.
pub(crate) fn ntriples<'a>(triples: impl Iterator<Item = &'a Triple>) -> String {
    let mut out = String::new();
    for t in triples {
        let _ = writeln!(out, "{t} .");
    }
    out
}

/// Renames terms an external tool would read differently, and back.
///
/// Nemo gets every literal and blank node as an IRI (it normalises typed
/// literals and labels blank nodes its own way); OWL-RL gets blank nodes as
/// IRIs (rdflib relabels them). The rules only compare terms, so the
/// renaming does not change what they derive.
#[derive(Debug)]
pub(crate) struct Codec {
    literals: bool,
    to: HashMap<Term, NamedNode>,
    from: HashMap<String, Term>,
}

const CODEC_PREFIX: &str = "urn:x-turbowl-oracle:term:";

impl Codec {
    /// `literals`: rename literals too, not only blank nodes.
    pub(crate) fn new(literals: bool) -> Self {
        Self {
            literals,
            to: HashMap::new(),
            from: HashMap::new(),
        }
    }

    fn encode(&mut self, term: Term) -> Term {
        let rename = match &term {
            Term::BlankNode(_) => true,
            Term::Literal(_) => self.literals,
            _ => false,
        };
        if !rename {
            return term;
        }
        let next = self.to.len();
        let iri = self
            .to
            .entry(term.clone())
            .or_insert_with(|| NamedNode::new_unchecked(format!("{CODEC_PREFIX}{next}")))
            .clone();
        self.from.insert(iri.as_str().to_owned(), term);
        iri.into()
    }

    /// The triple with its blank nodes (and literals) renamed to IRIs.
    pub(crate) fn encode_triple(&mut self, t: &Triple) -> Triple {
        let s = self.encode(t.subject.clone().into());
        let o = self.encode(t.object.clone());
        let Term::NamedNode(s) = s else {
            unreachable!("subjects encode to IRIs")
        };
        Triple::new(s, t.predicate.clone(), o)
    }

    /// The original term of an encoded one.
    pub(crate) fn decode(&self, term: Term) -> Term {
        match &term {
            Term::NamedNode(n) if n.as_str().starts_with(CODEC_PREFIX) => {
                self.from.get(n.as_str()).cloned().unwrap_or(term)
            }
            _ => term,
        }
    }

    /// The original triple, or `None` when it has no RDF form (a literal
    /// subject, or a predicate that is not an IRI): the rules derive such
    /// generalized triples, and the closure leaves them out.
    pub(crate) fn decode_triple(&self, s: Term, p: Term, o: Term) -> Option<Triple> {
        let s = match self.decode(s) {
            Term::NamedNode(n) => NamedOrBlankNode::from(n),
            Term::BlankNode(b) => NamedOrBlankNode::from(b),
            _ => return None,
        };
        let Term::NamedNode(p) = self.decode(p) else {
            return None;
        };
        Some(Triple::new(s, p, self.decode(o)))
    }
}
