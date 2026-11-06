//! Turtle: TurbOWL's parser.
//!
//! Copyright (c) 2018 Oxigraph developers; used under the MIT license (see
//! `THIRD-PARTY-NOTICES.md` at the repository root).
//!
//! The grammar is a port of `oxttl` 0.2.5's Turtle recognizer (`TriGRecognizer`
//! without graphs and without RDF 1.2): the same states on the same stack,
//! fed the same tokens ([`super::lexer`]), so a document gives the same
//! triples in the same order and a malformed one fails at the same token, with
//! the same message. Where oxttl records an error and reads on (a missing
//! dot), this does too, because a later error of the same token is the one
//! oxttl reports; the parse stops at the end of the token.
//!
//! The terms of a statement are [`Node`]s until a triple goes out
//! ([`super::terms`]); the statement's nodes and arena are dropped when it
//! ends.

use super::iri::{self, Base};
use super::lexer::{Event, Failure, Lexer, Mode, Text, Token, resolve_local_name};
use super::terms::{Datatype, Node, NodeId, Nodes, Out};
use crate::Error;
use crate::term::{TermId, vocab};
use rustc_hash::FxHashMap;

const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// Parses a Turtle document into `out`. `base` resolves relative IRIs.
pub(super) fn parse<'a, S: FnMut([TermId; 3])>(
    text: &'a [u8],
    base: Option<Base>,
    out: &mut Out<'a, '_, '_, S>,
) -> Result<(), Error> {
    let mut lexer = Lexer::new(text, Mode::Turtle, base);
    let mut arena = String::new();
    let mut turtle = Turtle::default();
    loop {
        let token = match lexer.next(&mut arena) {
            Ok(Event::Token(token)) => token,
            Ok(Event::LineJump) => continue,
            Ok(Event::End) => {
                return match turtle.end() {
                    Some(message) => Err(out.failure(text, Failure::at(lexer.token_start(), message))),
                    None => Ok(()),
                };
            }
            Err(failure) => return Err(out.failure(text, failure)),
        };
        turtle.step(token, &mut arena, &mut lexer.base);
        if let Some(message) = turtle.error.take() {
            let message = message.replace("TOKEN", &lexer.token_source());
            return Err(out.failure(text, Failure::at(lexer.token_start(), message)));
        }
        // As oxttl, the triples of one token go out last first.
        while let Some(triple) = turtle.results.pop() {
            out.deliver_held(&mut turtle.nodes, triple, &arena)?;
        }
        if turtle.between_statements() {
            turtle.nodes.clear();
            arena.clear();
        }
    }
}

/// oxttl's `TriGState`, the states Turtle reaches.
#[derive(Debug, Clone, Copy)]
enum State<'a> {
    TurtleDoc,
    ExpectDot,
    BaseExpectIri,
    PrefixExpectPrefix,
    PrefixExpectIri { name: &'a str },
    TriplesOrGraph,
    WrappedGraphBlankNodePropertyListCurrent,
    SubjectBlankNodePropertyListEnd,
    SubjectBlankNodePropertyListAfter,
    SubjectCollectionBeginning,
    SubjectCollectionPossibleEnd,
    WrappedGraphOrPredicateObjectList { term: NodeId },
    PredicateObjectList,
    PredicateObjectListEnd,
    PredicateObjectListPossibleContinuation,
    ObjectsList,
    AnnotationBlock,
    Verb,
    Object,
    ObjectBlankNodePropertyListCurrent,
    ObjectBlankNodePropertyListEnd,
    ObjectCollectionBeginning,
    ObjectCollectionPossibleEnd,
    LiteralPossibleSuffix { value: Text<'a>, emit: bool },
    LiteralExpectDatatype { value: Text<'a>, emit: bool },
}

#[derive(Debug)]
struct Turtle<'a> {
    stack: Vec<State<'a>>,
    subjects: Vec<NodeId>,
    predicates: Vec<NodeId>,
    objects: Vec<NodeId>,
    nodes: Nodes<'a>,
    /// Each prefix's namespace, and whether the namespace's path is empty.
    prefixes: FxHashMap<&'a str, (String, bool)>,
    /// The triples of the current token.
    results: Vec<[NodeId; 3]>,
    /// The error of the current token: the last one recorded, as oxttl reports it.
    error: Option<String>,
}

impl Default for Turtle<'_> {
    fn default() -> Self {
        Turtle {
            stack: vec![State::TurtleDoc],
            subjects: Vec::new(),
            predicates: Vec::new(),
            objects: Vec::new(),
            nodes: Nodes::default(),
            prefixes: FxHashMap::default(),
            results: Vec::new(),
            error: None,
        }
    }
}

impl<'a> Turtle<'a> {
    /// True between two statements: nothing refers to a node or the arena.
    fn between_statements(&self) -> bool {
        self.stack.len() == 1 && self.subjects.is_empty() && self.predicates.is_empty() && self.objects.is_empty()
    }

    /// An error that ends the statement (oxttl clears its stacks).
    fn fail(&mut self, message: impl Into<String>) {
        self.error = Some(message.into());
        self.stack.clear();
        self.subjects.clear();
        self.predicates.clear();
        self.objects.clear();
    }

    fn emit(&mut self) {
        let last = |v: &Vec<NodeId>| *v.last().expect("the grammar holds a term here");
        self.results
            .push([last(&self.subjects), last(&self.predicates), last(&self.objects)]);
    }

    fn constant(&mut self, id: TermId) -> NodeId {
        self.nodes.add(Node::Id(id))
    }

    /// The IRI of a prefixed name, in the arena.
    fn prefixed(
        &self,
        prefix: &str,
        local: Text<'a>,
        might_be_invalid_iri: bool,
        arena: &mut String,
    ) -> Result<Text<'a>, String> {
        resolve_local_name(prefix, local, might_be_invalid_iri, self.prefixes.get(prefix), arena)
    }

    /// The node of an IRI, a prefixed name or a blank node label, if the token is one.
    fn term(&mut self, token: &Token<'a>, arena: &mut String) -> Option<Result<NodeId, String>> {
        let node = match *token {
            Token::IriRef(iri) => Node::Iri(iri),
            Token::PrefixedName {
                prefix,
                local,
                might_be_invalid_iri,
            } => match self.prefixed(prefix, local, might_be_invalid_iri, arena) {
                Ok(iri) => Node::Iri(iri),
                Err(e) => return Some(Err(e)),
            },
            Token::BlankNodeLabel(label) => Node::Blank(label),
            _ => return None,
        };
        Some(Ok(self.nodes.add(node)))
    }

    /// One token: oxttl's `recognize_next`, with each re-dispatch of the
    /// token a turn of the loop.
    fn step(&mut self, token: Token<'a>, arena: &mut String, base: &mut Option<Base>) {
        let punctuation = |p: &str| token.is_punctuation(p);
        while let Some(state) = self.stack.pop() {
            match state {
                State::TurtleDoc => {
                    self.stack.push(State::TurtleDoc);
                    match token {
                        Token::PlainKeyword(k) if k.eq_ignore_ascii_case("base") => {
                            self.stack.push(State::BaseExpectIri);
                            return;
                        }
                        Token::PlainKeyword(k) if k.eq_ignore_ascii_case("prefix") => {
                            self.stack.push(State::PrefixExpectPrefix);
                            return;
                        }
                        Token::LangTag("prefix") => {
                            self.stack.push(State::ExpectDot);
                            self.stack.push(State::PrefixExpectPrefix);
                            return;
                        }
                        Token::LangTag("base") => {
                            self.stack.push(State::ExpectDot);
                            self.stack.push(State::BaseExpectIri);
                            return;
                        }
                        _ => self.stack.push(State::TriplesOrGraph),
                    }
                }
                State::ExpectDot => {
                    self.subjects.pop();
                    if punctuation(".") {
                        return;
                    }
                    self.error = Some("A dot is expected at the end of statements".into());
                }
                State::BaseExpectIri => {
                    if let Token::IriRef(iri) = token {
                        // Valid already: the lexer validated (or resolved) it.
                        match Base::new(iri.get(arena)) {
                            Ok(new) => *base = Some(new),
                            Err(e) => self.fail(e),
                        }
                    } else {
                        self.fail("The BASE keyword should be followed by an IRI");
                    }
                    return;
                }
                State::PrefixExpectPrefix => {
                    match token {
                        Token::PrefixedName { prefix, local, .. } if local.get(arena).is_empty() => {
                            self.stack.push(State::PrefixExpectIri { name: prefix });
                        }
                        _ => self.fail("The PREFIX keyword should be followed by a prefix like 'ex:'"),
                    }
                    return;
                }
                State::PrefixExpectIri { name } => {
                    if let Token::IriRef(iri) = token {
                        let iri = iri.get(arena);
                        self.prefixes.insert(name, (iri.to_owned(), iri::has_empty_path(iri)));
                    } else {
                        self.fail("The PREFIX declaration should be followed by a prefix and its value as an IRI");
                    }
                    return;
                }
                State::TriplesOrGraph => {
                    match self.term(&token, arena) {
                        Some(Ok(term)) => self.stack.push(State::WrappedGraphOrPredicateObjectList { term }),
                        Some(Err(e)) => self.fail(e),
                        None if punctuation("[") => self.stack.push(State::WrappedGraphBlankNodePropertyListCurrent),
                        None if punctuation("(") => {
                            self.stack.push(State::ExpectDot);
                            self.stack.push(State::PredicateObjectList);
                            self.stack.push(State::SubjectCollectionBeginning);
                        }
                        None => self.fail("TOKEN is not a valid subject or graph name"),
                    }
                    return;
                }
                State::WrappedGraphOrPredicateObjectList { term } => {
                    self.subjects.push(term);
                    self.stack.push(State::ExpectDot);
                    self.stack.push(State::PredicateObjectList);
                }
                State::WrappedGraphBlankNodePropertyListCurrent => {
                    let term = self.nodes.add(Node::Anon);
                    if punctuation("]") {
                        self.stack.push(State::WrappedGraphOrPredicateObjectList { term });
                        return;
                    }
                    self.subjects.push(term);
                    self.stack.push(State::ExpectDot);
                    self.stack.push(State::SubjectBlankNodePropertyListEnd);
                    self.stack.push(State::PredicateObjectList);
                }
                State::SubjectBlankNodePropertyListEnd => {
                    self.stack.push(State::SubjectBlankNodePropertyListAfter);
                    if punctuation("]") {
                        return;
                    }
                    self.error = Some("blank node property lists should end with a ']'".into());
                }
                State::SubjectBlankNodePropertyListAfter => {
                    if !(punctuation(".") || punctuation("}")) {
                        self.stack.push(State::PredicateObjectList);
                    }
                }
                State::SubjectCollectionBeginning => {
                    if punctuation(")") {
                        let nil = self.constant(vocab::RDF_NIL);
                        self.subjects.push(nil);
                        return;
                    }
                    let root = self.nodes.add(Node::Anon);
                    self.subjects.push(root);
                    self.subjects.push(root);
                    let first = self.constant(vocab::RDF_FIRST);
                    self.predicates.push(first);
                    self.stack.push(State::SubjectCollectionPossibleEnd);
                    self.stack.push(State::Object);
                }
                State::SubjectCollectionPossibleEnd => {
                    let old = self.subjects.pop().expect("a list cell");
                    self.objects.pop();
                    let rest = self.constant(vocab::RDF_REST);
                    if punctuation(")") {
                        self.predicates.pop();
                        let nil = self.constant(vocab::RDF_NIL);
                        self.results.push([old, rest, nil]);
                        return;
                    }
                    let new = self.nodes.add(Node::Anon);
                    self.results.push([old, rest, new]);
                    self.subjects.push(new);
                    self.stack.push(State::ObjectCollectionPossibleEnd);
                    self.stack.push(State::Object);
                }
                State::PredicateObjectList => {
                    self.stack.push(State::PredicateObjectListEnd);
                    self.stack.push(State::ObjectsList);
                    self.stack.push(State::Verb);
                }
                State::PredicateObjectListEnd => {
                    self.predicates.pop();
                    if punctuation(";") {
                        self.stack.push(State::PredicateObjectListPossibleContinuation);
                        return;
                    }
                }
                State::PredicateObjectListPossibleContinuation => {
                    if punctuation(";") {
                        self.stack.push(State::PredicateObjectListPossibleContinuation);
                        return;
                    }
                    if !(punctuation(".") || punctuation("}") || punctuation("]") || punctuation("|}")) {
                        self.stack.push(State::PredicateObjectListEnd);
                        self.stack.push(State::ObjectsList);
                        self.stack.push(State::Verb);
                    }
                }
                State::ObjectsList => {
                    self.stack.push(State::AnnotationBlock);
                    self.stack.push(State::Object);
                }
                State::AnnotationBlock => {
                    self.objects.pop();
                    if punctuation(",") {
                        self.stack.push(State::AnnotationBlock);
                        self.stack.push(State::Object);
                        return;
                    }
                }
                State::Verb => {
                    match token {
                        Token::PlainKeyword("a") => {
                            let a = self.constant(vocab::RDF_TYPE);
                            self.predicates.push(a);
                        }
                        Token::BlankNodeLabel(_) => self.fail("TOKEN is not a valid predicate"),
                        _ => match self.term(&token, arena) {
                            Some(Ok(predicate)) => self.predicates.push(predicate),
                            Some(Err(e)) => self.fail(e),
                            None => self.fail("TOKEN is not a valid predicate"),
                        },
                    }
                    return;
                }
                State::Object => {
                    let literal = |lexical, datatype| Node::Literal {
                        lexical: Text::Doc(lexical),
                        datatype: Datatype::Id(datatype),
                    };
                    let node = match token {
                        Token::String(value) | Token::LongString(value) => {
                            self.stack.push(State::LiteralPossibleSuffix { value, emit: true });
                            return;
                        }
                        Token::Integer(v) => literal(v, vocab::XSD_INTEGER),
                        Token::Decimal(v) => literal(v, vocab::XSD_DECIMAL),
                        Token::Double(v) => literal(v, vocab::XSD_DOUBLE),
                        Token::PlainKeyword(v @ ("true" | "false")) => literal(v, vocab::XSD_BOOLEAN),
                        Token::Punctuation("[") => {
                            self.stack.push(State::ObjectBlankNodePropertyListCurrent);
                            return;
                        }
                        Token::Punctuation("(") => {
                            self.stack.push(State::ObjectCollectionBeginning);
                            return;
                        }
                        _ => {
                            match self.term(&token, arena) {
                                Some(Ok(object)) => {
                                    self.objects.push(object);
                                    self.emit();
                                }
                                Some(Err(e)) => self.fail(e),
                                None => self.fail("TOKEN is not a valid RDF object"),
                            }
                            return;
                        }
                    };
                    let object = self.nodes.add(node);
                    self.objects.push(object);
                    self.emit();
                    return;
                }
                State::ObjectBlankNodePropertyListCurrent => {
                    let node = self.nodes.add(Node::Anon);
                    if punctuation("]") {
                        self.objects.push(node);
                        self.emit();
                        return;
                    }
                    self.subjects.push(node);
                    self.stack.push(State::ObjectBlankNodePropertyListEnd);
                    self.stack.push(State::PredicateObjectList);
                }
                State::ObjectBlankNodePropertyListEnd => {
                    if punctuation("]") {
                        let node = self.subjects.pop().expect("the list's node");
                        self.objects.push(node);
                        self.emit();
                    } else {
                        self.fail("blank node property lists should end with a ']'");
                    }
                    return;
                }
                State::ObjectCollectionBeginning => {
                    if punctuation(")") {
                        let nil = self.constant(vocab::RDF_NIL);
                        self.objects.push(nil);
                        self.emit();
                        return;
                    }
                    let root = self.nodes.add(Node::Anon);
                    self.objects.push(root);
                    self.emit();
                    self.subjects.push(root);
                    let first = self.constant(vocab::RDF_FIRST);
                    self.predicates.push(first);
                    self.stack.push(State::ObjectCollectionPossibleEnd);
                    self.stack.push(State::Object);
                }
                State::ObjectCollectionPossibleEnd => {
                    let old = self.subjects.pop().expect("a list cell");
                    self.objects.pop();
                    let rest = self.constant(vocab::RDF_REST);
                    if punctuation(")") {
                        self.predicates.pop();
                        let nil = self.constant(vocab::RDF_NIL);
                        self.results.push([old, rest, nil]);
                        return;
                    }
                    let new = self.nodes.add(Node::Anon);
                    self.results.push([old, rest, new]);
                    self.subjects.push(new);
                    self.stack.push(State::ObjectCollectionPossibleEnd);
                    self.stack.push(State::Object);
                }
                State::LiteralPossibleSuffix { value, emit } => match token {
                    Token::LangTag(tag) => {
                        let node = self.nodes.add(Node::tagged(value, tag, arena));
                        self.objects.push(node);
                        if emit {
                            self.emit();
                        }
                        return;
                    }
                    Token::Punctuation("^^") => {
                        self.stack.push(State::LiteralExpectDatatype { value, emit });
                        return;
                    }
                    _ => {
                        let node = self.nodes.add(Node::string(value));
                        self.objects.push(node);
                        if emit {
                            self.emit();
                        }
                    }
                },
                State::LiteralExpectDatatype { value, emit } => {
                    let datatype = match token {
                        Token::IriRef(iri) => Ok(iri),
                        Token::PrefixedName {
                            prefix,
                            local,
                            might_be_invalid_iri,
                        } => self.prefixed(prefix, local, might_be_invalid_iri, arena),
                        _ => {
                            self.fail("Expecting a datatype IRI after ^^, found TOKEN");
                            return;
                        }
                    };
                    match datatype {
                        Ok(datatype) => {
                            if datatype.get(arena) == RDF_LANG_STRING {
                                self.error = Some(
                                    "The datatype of a literal without a language tag must not be rdf:langString"
                                        .into(),
                                );
                            }
                            let node = self.nodes.add(Node::Literal {
                                lexical: value,
                                datatype: Datatype::Iri(datatype),
                            });
                            self.objects.push(node);
                            if emit {
                                self.emit();
                            }
                        }
                        Err(e) => self.fail(e),
                    }
                    return;
                }
            }
        }
    }

    /// oxttl's `recognize_end`: the error of a document that stops mid-statement.
    fn end(&self) -> Option<String> {
        match self.stack.as_slice() {
            [] | [State::TurtleDoc] => None,
            [.., State::LiteralPossibleSuffix { emit: true, .. }] => Some("Triples should be followed by a dot".into()),
            _ => Some("Unexpected end".into()),
        }
    }
}
