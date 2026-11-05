//! Shared by the test files: a small Turtle reader for hand-written inputs.

#![allow(dead_code)]

use oxrdf::{BlankNode, Literal, NamedNode, NamedOrBlankNode, Term, Triple};
use std::collections::{BTreeSet, HashSet};

const PREFIXES: &[(&str, &str)] = &[
    ("rdf:", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("rdfs:", "http://www.w3.org/2000/01/rdf-schema#"),
    ("owl:", "http://www.w3.org/2002/07/owl#"),
    ("xsd:", "http://www.w3.org/2001/XMLSchema#"),
    ("ex:", "http://example.org/"),
];

/// The Turtle these tests write, with the prefixes above built in:
/// prefixed names, `a`, `_:label`, `"text"`, `"text"@lang`,
/// `"text"^^pfx:name`, `;`, `,` and `( ... )` lists. Written here so that
/// the crate needs no Turtle parser dependency.
pub fn parse(turtle: &str) -> HashSet<Triple> {
    let mut parser = Parser {
        tokens: tokenize(turtle),
        at: 0,
        cells: 0,
        out: HashSet::new(),
    };
    while parser.at < parser.tokens.len() {
        let subject = parser.term();
        loop {
            let predicate = match parser.term() {
                Term::NamedNode(p) => p,
                other => panic!("predicate {other}"),
            };
            loop {
                let object = parser.term();
                parser.add(subject.clone(), predicate.clone(), object);
                if parser.peek() != "," {
                    break;
                }
                parser.at += 1;
            }
            if parser.peek() != ";" {
                break;
            }
            parser.at += 1;
        }
        assert_eq!(parser.next(), ".", "in {turtle}");
    }
    parser.out
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '"' {
            let mut token = String::from(chars.next().unwrap());
            while let Some(c) = chars.next() {
                token.push(c);
                if c == '\\' {
                    token.push(chars.next().unwrap());
                } else if c == '"' {
                    break;
                }
            }
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || ",;.()".contains(c) && !token.ends_with('^') {
                    break;
                }
                token.push(c);
                chars.next();
            }
            tokens.push(token);
        } else if ",;()".contains(c) {
            tokens.push(chars.next().unwrap().to_string());
        } else {
            let mut token = String::new();
            while let Some(&c) = chars.peek() {
                // A `.` ends a name only when whitespace or the end follows.
                if c.is_whitespace() || ",;()".contains(c) {
                    break;
                }
                token.push(c);
                chars.next();
                if c == '.' && chars.peek().is_none_or(|n| n.is_whitespace()) {
                    break;
                }
            }
            if token.len() > 1 && token.ends_with('.') {
                token.pop();
                tokens.push(token);
                tokens.push(".".to_owned());
            } else {
                tokens.push(token);
            }
        }
    }
    tokens
}

struct Parser {
    tokens: Vec<String>,
    at: usize,
    cells: usize,
    out: HashSet<Triple>,
}

impl Parser {
    fn peek(&self) -> &str {
        self.tokens.get(self.at).map_or("", String::as_str)
    }

    fn next(&mut self) -> String {
        self.at += 1;
        self.tokens[self.at - 1].clone()
    }

    fn add(&mut self, s: Term, p: NamedNode, o: Term) {
        let s = NamedOrBlankNode::try_from(s).expect("subject");
        self.out.insert(Triple::new(s, p, o));
    }

    fn term(&mut self) -> Term {
        let token = self.next();
        if token == "(" {
            let mut members = Vec::new();
            while self.peek() != ")" {
                members.push(self.term());
            }
            self.at += 1;
            let mut list = Term::from(name("rdf:nil"));
            for member in members.into_iter().rev() {
                self.cells += 1;
                let cell = Term::from(BlankNode::new_unchecked(format!("cell{}", self.cells)));
                self.add(cell.clone(), name("rdf:first"), member);
                self.add(cell.clone(), name("rdf:rest"), list);
                list = cell;
            }
            list
        } else if token == "a" {
            Term::from(name("rdf:type"))
        } else if let Some(label) = token.strip_prefix("_:") {
            Term::from(BlankNode::new_unchecked(label))
        } else if let Some(quoted) = token.strip_prefix('"') {
            let end = quoted.rfind('"').expect("closing quote");
            let value = quoted[..end].replace("\\\"", "\"");
            let suffix = &quoted[end + 1..];
            if let Some(language) = suffix.strip_prefix('@') {
                Term::from(Literal::new_language_tagged_literal_unchecked(value, language))
            } else if let Some(datatype) = suffix.strip_prefix("^^") {
                Term::from(Literal::new_typed_literal(value, name(datatype)))
            } else {
                Term::from(Literal::new_simple_literal(value))
            }
        } else {
            Term::from(name(&token))
        }
    }
}

pub fn name(prefixed: &str) -> NamedNode {
    for (prefix, namespace) in PREFIXES {
        if let Some(local) = prefixed.strip_prefix(prefix) {
            return NamedNode::new_unchecked(format!("{namespace}{local}"));
        }
    }
    panic!("unknown prefix in {prefixed}")
}

/// One term, written as `parse` reads an object: `ex:a`, `"5"^^xsd:int`,
/// `"x"@en`, `_:b`.
pub fn term(token: &str) -> Term {
    let triples = parse(&format!("ex:s ex:p {token} ."));
    assert_eq!(triples.len(), 1, "one term: {token}");
    triples.into_iter().next().unwrap().object
}

pub fn lines<'a>(triples: impl IntoIterator<Item = &'a Triple>) -> BTreeSet<String> {
    triples.into_iter().map(|t| format!("{t} .")).collect()
}
