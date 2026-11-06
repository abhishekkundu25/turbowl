//! The terms a parse holds until it hands a triple over, and the handing over.
//!
//! The grammars keep each term as a [`Node`]: its text, borrowed from the
//! document or from the parse's arena, not yet a term id. A term is interned
//! when the first triple that holds it is handed to the sink, in the order the
//! triples go out and subject, predicate, object within one (a literal's
//! datatype before the literal). That is the order in which the parse through
//! Oxigraph interned them, so a document gives the dictionary the same terms
//! at the same ids as before, and a triple that never goes out (the parse
//! failed first) interns nothing.
//!
//! [`Nodes`] numbers the nodes of one statement, so a term held in several
//! places (an anonymous node, a subject with several predicates) is interned
//! once and its id reused.
//!
//! A grammar's parse ends in a [`Stop`] or a [`Fail`]; [`Out::error`] gives a
//! failure its line and column, which only the caller needs (a parallel
//! parse throws most of its failures away).

use super::error_at;
use super::lexer::{Failure, Text};
use crate::Error;
use crate::term::{Dictionary, DocId, MAX_TERM_BYTES, MAX_TERMS, TermId, vocab};
use std::path::Path;

/// A term before it is interned.
#[derive(Debug, Clone, Copy)]
pub(super) enum Node<'a> {
    /// A term with an id already (the vocabulary: `rdf:type`, `rdf:nil`...).
    Id(TermId),
    Iri(Text<'a>),
    /// A labelled blank node of the document.
    Blank(&'a str),
    /// A new anonymous blank node (`[]`, a list's cells); labelled as Oxigraph
    /// labels them, at random.
    Anon,
    Literal {
        lexical: Text<'a>,
        datatype: Datatype<'a>,
    },
}

/// A literal's datatype, or its language tag.
#[derive(Debug, Clone, Copy)]
pub(super) enum Datatype<'a> {
    Id(TermId),
    Iri(Text<'a>),
    /// A language tag, lowercased (the datatype is `rdf:langString`).
    Lang(Text<'a>),
}

impl<'a> Node<'a> {
    /// A plain literal (`xsd:string`).
    pub(super) fn string(lexical: Text<'a>) -> Self {
        Node::Literal {
            lexical,
            datatype: Datatype::Id(vocab::XSD_STRING),
        }
    }

    /// A language-tagged literal; the tag is lowercased into `arena` when it
    /// has capitals, as Oxigraph's parsers do.
    pub(super) fn tagged(lexical: Text<'a>, tag: &'a str, arena: &mut String) -> Self {
        let tag = if tag.bytes().any(|b| b.is_ascii_uppercase()) {
            let start = arena.len();
            arena.extend(tag.chars().map(|c| c.to_ascii_lowercase()));
            Text::Arena(start, arena.len())
        } else {
            Text::Doc(tag)
        };
        Node::Literal {
            lexical,
            datatype: Datatype::Lang(tag),
        }
    }
}

/// Where a grammar's parse of a span stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stop {
    /// At the end of the document, which ends where a statement may end.
    End,
    /// At this line start, the first at or after the span's end where the
    /// grammar is between statements (between lines for N-Triples): a parse
    /// from here with the same prefixes and base reads on exactly as this one
    /// would have.
    At(usize),
}

/// Why a parse failed.
#[derive(Debug)]
pub(super) enum Fail {
    /// A syntax error at a position.
    Syntax(Failure),
    /// A limit of the dictionary, at no position.
    Limit(String),
    /// An open text ran short ([`Failure::short`]): no error, but the parse
    /// cannot go on until more of the document is read.
    Short,
}

impl From<Failure> for Fail {
    fn from(failure: Failure) -> Self {
        if failure.short {
            Fail::Short
        } else {
            Fail::Syntax(failure)
        }
    }
}

/// An index into [`Nodes`].
pub(super) type NodeId = u32;

const NOT_INTERNED: u32 = u32::MAX;

/// The nodes of one statement, each with its id once interned.
#[derive(Debug, Default)]
pub(super) struct Nodes<'a> {
    nodes: Vec<Node<'a>>,
    ids: Vec<u32>,
}

impl<'a> Nodes<'a> {
    #[inline]
    pub(super) fn add(&mut self, node: Node<'a>) -> NodeId {
        let id = self.nodes.len() as NodeId;
        let interned = match node {
            Node::Id(t) => t.0,
            _ => NOT_INTERNED,
        };
        self.nodes.push(node);
        self.ids.push(interned);
        id
    }

    /// Forgets the statement's nodes (its arena text goes at the same time).
    #[inline]
    pub(super) fn clear(&mut self) {
        self.nodes.clear();
        self.ids.clear();
    }
}

/// Where a parse's triples go: the dictionary, the document and the sink.
pub(super) struct Out<'a, 'd, 'p, S> {
    dict: &'d mut Dictionary,
    doc: DocId,
    sink: S,
    pub(super) triples: usize,
    path: Option<&'p Path>,
    /// The last subject and predicate taken from the document as they are,
    /// with their ids: consecutive triples mostly share them, and comparing
    /// the text is cheaper than a lookup.
    last: [Option<(&'a str, TermId)>; 2],
}

impl<'a, 'd, 'p, S: FnMut([TermId; 3])> Out<'a, 'd, 'p, S> {
    pub(super) fn new(dict: &'d mut Dictionary, doc: DocId, sink: S, path: Option<&'p Path>) -> Self {
        Out {
            dict,
            doc,
            sink,
            triples: 0,
            path,
            last: [None; 2],
        }
    }

    /// The error a parse failure is, with its line and column.
    pub(super) fn error(&self, text: &[u8], fail: Fail) -> Error {
        error(self.path, text, fail)
    }

    /// Hands a triple of three nodes over (N-Triples: nothing is shared).
    pub(super) fn deliver(&mut self, triple: [Node<'a>; 3], arena: &str) -> Result<(), Fail> {
        self.check(&triple, arena)?;
        let [s, p, o] = triple;
        let ids = [
            self.intern(s, 0, arena),
            self.intern(p, 1, arena),
            self.intern(o, 2, arena),
        ];
        (self.sink)(ids);
        self.triples += 1;
        Ok(())
    }

    /// Hands a triple of a statement's nodes over, interning each node once.
    pub(super) fn deliver_held(&mut self, nodes: &mut Nodes<'a>, triple: [NodeId; 3], arena: &str) -> Result<(), Fail> {
        let held = triple.map(|n| nodes.nodes[n as usize]);
        self.check(&held, arena)?;
        let mut ids = [TermId(0); 3];
        for (position, (&n, node)) in triple.iter().zip(held).enumerate() {
            let cached = nodes.ids[n as usize];
            ids[position] = if cached != NOT_INTERNED {
                TermId(cached)
            } else {
                let id = self.intern(node, position, arena);
                nodes.ids[n as usize] = id.0;
                id
            };
        }
        (self.sink)(ids);
        self.triples += 1;
        Ok(())
    }

    /// The limits the parse through Oxigraph checked before each triple.
    fn check(&self, triple: &[Node<'a>; 3], arena: &str) -> Result<(), Fail> {
        if self.dict.len() > MAX_TERMS - 3 {
            return Err(Fail::Limit(format!("the dictionary is full ({MAX_TERMS} terms)")));
        }
        // A term can be longer than its document (prefixes, a base IRI).
        if triple.iter().any(|n| length(n, arena) > MAX_TERM_BYTES) {
            return Err(Fail::Limit(format!("a term is longer than {MAX_TERM_BYTES} bytes")));
        }
        Ok(())
    }

    /// The id of a node at `position` (0 subject, 1 predicate, 2 object).
    #[inline]
    fn intern(&mut self, node: Node<'a>, position: usize, arena: &str) -> TermId {
        match node {
            Node::Id(id) => id,
            Node::Iri(Text::Doc(iri)) if position < 2 => {
                if let Some((last, id)) = self.last[position]
                    && last == iri
                {
                    return id;
                }
                let id = self.dict.intern_iri(iri);
                self.last[position] = Some((iri, id));
                id
            }
            Node::Iri(text) => self.dict.intern_iri(text.get(arena)),
            Node::Blank(label) => self.dict.intern_blank(self.doc, label),
            Node::Anon => {
                let label = oxrdf::BlankNode::default();
                self.dict.intern_blank(self.doc, label.as_str())
            }
            Node::Literal { lexical, datatype } => {
                let lexical = lexical.get(arena);
                match datatype {
                    Datatype::Id(datatype) => self.dict.intern_typed_literal(lexical, datatype),
                    Datatype::Iri(iri) => {
                        let iri = iri.get(arena);
                        let datatype = if iri == vocab::XSD_STRING_IRI {
                            vocab::XSD_STRING
                        } else {
                            self.dict.intern_iri(iri)
                        };
                        self.dict.intern_typed_literal(lexical, datatype)
                    }
                    Datatype::Lang(tag) => self.dict.intern_lang_literal(lexical, tag.get(arena)),
                }
            }
        }
    }
}

/// The error a parse failure of `text` is, with its line and column.
pub(super) fn error(path: Option<&Path>, text: &[u8], fail: Fail) -> Error {
    match fail {
        Fail::Syntax(failure) => {
            let (line, column) = failure.position(text);
            error_at(path, line + 1, column + 1, failure.message)
        }
        Fail::Limit(message) => error_at(path, 0, 0, message),
        Fail::Short => unreachable!("only an open text runs short, and its parse reads on"),
    }
}

/// The bytes of a node's term, as the dictionary limits them.
fn length(node: &Node<'_>, arena: &str) -> usize {
    match *node {
        Node::Id(_) | Node::Anon => 0,
        Node::Iri(text) => text.get(arena).len(),
        Node::Blank(label) => label.len(),
        Node::Literal { lexical, datatype } => {
            lexical.get(arena).len()
                + match datatype {
                    Datatype::Lang(tag) => tag.get(arena).len(),
                    _ => 0,
                }
        }
    }
}
