//! N-Triples: TurbOWL's parser.
//!
//! Copyright (c) 2018 Oxigraph developers; used under the MIT license (see
//! `THIRD-PARTY-NOTICES.md` at the repository root).
//!
//! The grammar is a port of `oxttl` 0.2.5's N-Triples recognizer
//! (`NQuadsRecognizer` without graph names and without RDF 1.2), fed the same
//! tokens ([`super::lexer`]): a triple per line, line jumps significant. A
//! triple goes out at the token after its object, as in oxttl, so a triple
//! whose line then fails never reaches the sink.

use super::lexer::{Event, Failure, Lexer, Mode, Text, Token};
use super::terms::{Datatype, Node, Out};
use crate::Error;
use crate::term::TermId;

const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// Parses an N-Triples document into `out`.
pub(super) fn parse<'a, S: FnMut([TermId; 3])>(text: &'a [u8], out: &mut Out<'a, '_, '_, S>) -> Result<(), Error> {
    let mut lexer = Lexer::new(text, Mode::NTriples, None);
    let mut arena = String::new();
    let mut line = Line::default();
    loop {
        let event = lexer.next(&mut arena).map_err(|failure| out.failure(text, failure))?;
        if let Event::End = event {
            return match line.end() {
                Some(message) => Err(out.failure(text, Failure::at(lexer.token_start(), message))),
                None => Ok(()),
            };
        }
        line.step(event, &mut arena);
        if let Some(message) = line.error.take() {
            let message = message.replace("TOKEN", &lexer.token_source());
            return Err(out.failure(text, Failure::at(lexer.token_start(), message)));
        }
        if let Some(triple) = line.result.take() {
            out.deliver(triple, &arena)?;
            arena.clear();
        }
    }
}

/// oxttl's `NQuadsState`, the states N-Triples reaches. Its stack never holds
/// more than one, so it is an `Option` here (`None` is an empty stack).
#[derive(Debug, Clone, Copy)]
enum State<'a> {
    ExpectSubject,
    ExpectPredicate,
    ExpectedObject,
    ExpectPossibleGraphOrEndOfQuotedTriple,
    ExpectDot,
    ExpectLiteralAnnotationOrGraphNameOrDot { value: Text<'a> },
    ExpectLiteralDatatype { value: Text<'a> },
    ExpectLineJump,
}

#[derive(Debug)]
struct Line<'a> {
    state: Option<State<'a>>,
    subject: Option<Node<'a>>,
    predicate: Option<Node<'a>>,
    object: Option<Node<'a>>,
    /// The triple the current token completes.
    result: Option<[Node<'a>; 3]>,
    /// The error of the current token: the last one recorded, as oxttl reports it.
    error: Option<String>,
}

impl Default for Line<'_> {
    fn default() -> Self {
        Line {
            state: Some(State::ExpectSubject),
            subject: None,
            predicate: None,
            object: None,
            result: None,
            error: None,
        }
    }
}

const MID_TRIPLE: &str = "line jumps are not allowed in the middle of triples";
const NO_DOT: &str = "Quads must be followed by a dot";

impl<'a> Line<'a> {
    /// An error after which oxttl skips to the next line.
    fn fail(&mut self, message: &str) {
        self.error = Some(message.into());
        self.state = None;
    }

    /// One token or line jump: oxttl's `recognize_next`, with each
    /// re-dispatch a turn of the loop.
    fn step(&mut self, event: Event<'a>, arena: &mut String) {
        loop {
            let state = self.state.take().unwrap_or(State::ExpectSubject);
            let token = match event {
                Event::Token(token) => Some(token),
                _ => None,
            };
            match state {
                State::ExpectSubject => {
                    match token {
                        None => {}
                        Some(Token::IriRef(iri)) => {
                            self.subject = Some(Node::Iri(iri));
                            self.state = Some(State::ExpectPredicate);
                        }
                        Some(Token::BlankNodeLabel(label)) => {
                            self.subject = Some(Node::Blank(label));
                            self.state = Some(State::ExpectPredicate);
                        }
                        Some(_) => self.fail("The subject of a triple must be an IRI or a blank node"),
                    }
                    return;
                }
                State::ExpectPredicate => {
                    match token {
                        None => self.fail(MID_TRIPLE),
                        Some(Token::IriRef(iri)) => {
                            self.predicate = Some(Node::Iri(iri));
                            self.state = Some(State::ExpectedObject);
                        }
                        Some(_) => self.fail("The predicate of a triple must be an IRI"),
                    }
                    return;
                }
                State::ExpectedObject => {
                    match token {
                        None => self.fail(MID_TRIPLE),
                        Some(Token::IriRef(iri)) => {
                            self.object = Some(Node::Iri(iri));
                            self.state = Some(State::ExpectPossibleGraphOrEndOfQuotedTriple);
                        }
                        Some(Token::BlankNodeLabel(label)) => {
                            self.object = Some(Node::Blank(label));
                            self.state = Some(State::ExpectPossibleGraphOrEndOfQuotedTriple);
                        }
                        Some(Token::String(value)) => {
                            self.state = Some(State::ExpectLiteralAnnotationOrGraphNameOrDot { value });
                        }
                        Some(_) => self.fail("The object of a triple must be an IRI, a blank node or a literal"),
                    }
                    return;
                }
                State::ExpectLiteralAnnotationOrGraphNameOrDot { value } => {
                    self.state = Some(State::ExpectPossibleGraphOrEndOfQuotedTriple);
                    match token {
                        Some(Token::LangTag(tag)) => {
                            self.object = Some(Node::tagged(value, tag, arena));
                            return;
                        }
                        Some(Token::Punctuation("^^")) => {
                            self.state = Some(State::ExpectLiteralDatatype { value });
                            return;
                        }
                        _ => self.object = Some(Node::string(value)),
                    }
                }
                State::ExpectLiteralDatatype { value } => {
                    match token {
                        None => self.fail(MID_TRIPLE),
                        Some(Token::IriRef(datatype)) => {
                            if datatype.get(arena) == RDF_LANG_STRING {
                                self.error = Some(
                                    "The datatype of a literal without a language tag must not be rdf:langString"
                                        .into(),
                                );
                            }
                            self.object = Some(Node::Literal {
                                lexical: value,
                                datatype: Datatype::Iri(datatype),
                            });
                            self.state = Some(State::ExpectPossibleGraphOrEndOfQuotedTriple);
                        }
                        Some(_) => self.fail("A literal datatype must be an IRI"),
                    }
                    return;
                }
                State::ExpectPossibleGraphOrEndOfQuotedTriple => {
                    self.emit();
                    self.state = Some(State::ExpectDot);
                }
                State::ExpectDot => match token {
                    None => {
                        self.fail(NO_DOT);
                        return;
                    }
                    Some(Token::Punctuation(".")) => {
                        self.state = Some(State::ExpectLineJump);
                        return;
                    }
                    Some(_) => self.error = Some(NO_DOT.into()),
                },
                State::ExpectLineJump => match token {
                    None => return,
                    Some(token) => {
                        self.error = Some(format!(
                            "Only a single triple or quad can be written in a line, found {}",
                            token.describe(arena)
                        ));
                    }
                },
            }
        }
    }

    fn emit(&mut self) {
        if let (Some(s), Some(p), Some(o)) = (self.subject.take(), self.predicate.take(), self.object.take()) {
            self.result = Some([s, p, o]);
        }
    }

    /// oxttl's `recognize_end`: the error of a document that stops mid-line.
    fn end(&self) -> Option<String> {
        match self.state {
            None | Some(State::ExpectSubject | State::ExpectLineJump) => None,
            Some(
                State::ExpectDot
                | State::ExpectPossibleGraphOrEndOfQuotedTriple
                | State::ExpectLiteralAnnotationOrGraphNameOrDot { .. },
            ) => Some("Triples must be followed by a dot".into()),
            _ => Some("Unexpected end".into()),
        }
    }
}
