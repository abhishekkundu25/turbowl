//! The rule program the explainer runs backwards: `oracles/rules/*.rls`,
//! the W3C rules that Nemo, Soufflé and VLog run (and that every oracle
//! holds equal to TurbOWL's), read with `include_str!` and parsed here,
//! plus `literals.rls`, what that program leaves to the oracle crate
//! (`literalType`, `literalValue`) or leaves out (dt-not-type), and
//! `values.rls`, dt-diff in the one shape a proof gives it.
//!
//! The parser reads the part of Nemo's syntax the files use: `%` comments,
//! `@prefix`, `@import`, `@export`, facts and rules of atoms over variables
//! (`?x`), prefixed names and `<iri>`s, and `?x != t`. A rule's W3C name is
//! the comment on the line where it ends (`... .  % cax-sco`). Each file is
//! the group of a switch (or the core), as `crates/oracle` composes them.
//!
//! Two rewrites, sound by `core.rls`'s note on `Tp`:
//!
//! - `Tp(p, x, y)` is read as `T(x, p, y)`. `Tp` holds the triples of `T`
//!   whose predicate some schema triple names (`usedProp`), and every rule
//!   that reads it binds its `p` by such a triple, so the rules derive the
//!   same with `T`, and no unrelated schema triple becomes a premise.
//! - The `usedProp` rules and the `Tp` rule are dropped, and so is
//!   `T(?s, ?p, ?o) :- input(?s, ?p, ?o)`: a base triple is a leaf of every
//!   proof.
//!
//! One more, for dt-diff's proof shape (`docs/specs/explanations.md`): the
//! oracle files' dt-diff rule, which concludes eq-diff1 from the values of
//! two terms made the same (`valueOf`, carried through `owl:sameAs`), is
//! dropped with the `valueOf` rules. `values.rls` states dt-diff as the
//! reference evaluator does: a generalized triple `lt1 unlisted lt2` between
//! two literals of different values, whose predicate
//! ([`UNLISTED`](super::UNLISTED)) is the reference's unlisted
//! `owl:differentFrom`, carried by eq-rep-s and eq-rep-o like any triple,
//! and read by eq-diff1 as `owl:differentFrom` is.
//!
//! The literals the oracle names as IRIs (`turbowl_oracle::NAMED_LITERALS`)
//! are read back as the literals they stand for.

use crate::switch::{Switch, Switches};
use crate::term::{Dictionary, DocId};
use std::collections::HashMap;
use std::sync::OnceLock;

/// The files, each with its group (`None`: always on).
const FILES: [(&str, Option<Switch>, &str); 10] = [
    ("core.rls", None, include_str!("../../../../oracles/rules/core.rls")),
    (
        "equality.rls",
        Some(Switch::Equality),
        include_str!("../../../../oracles/rules/equality.rls"),
    ),
    (
        "property-chains.rls",
        Some(Switch::PropertyChains),
        include_str!("../../../../oracles/rules/property-chains.rls"),
    ),
    (
        "checks.rls",
        Some(Switch::Checks),
        include_str!("../../../../oracles/rules/checks.rls"),
    ),
    (
        "identity.rls",
        Some(Switch::Identity),
        include_str!("../../../../oracles/rules/identity.rls"),
    ),
    (
        "schema.rls",
        Some(Switch::Schema),
        include_str!("../../../../oracles/rules/schema.rls"),
    ),
    (
        "trivial.rls",
        Some(Switch::Trivial),
        include_str!("../../../../oracles/rules/trivial.rls"),
    ),
    (
        "literal-values.rls",
        Some(Switch::LiteralValues),
        include_str!("../../../../oracles/rules/literal-values.rls"),
    ),
    ("literals.rls", None, include_str!("literals.rls")),
    ("values.rls", Some(Switch::LiteralValues), include_str!("values.rls")),
];

/// The IRI `values.rls` writes for dt-diff's unlisted predicate, read as
/// [`UNLISTED`].
const UNLISTED_IRI: &str = "urn:x-turbowl-explain:unlisted";

/// The id of dt-diff's unlisted predicate, which no term of the
/// dictionary has (see the module doc).
pub(crate) const UNLISTED: u32 = u32::MAX - 1;

/// The literals `crates/oracle` names as IRIs, so that the files can write
/// them: (IRI, lexical form, datatype).
const NAMED_LITERALS: [(&str, &str, &str); 2] = [
    (
        "urn:x-turbowl-oracle:term:nonNegativeInteger-0",
        "0",
        "http://www.w3.org/2001/XMLSchema#nonNegativeInteger",
    ),
    (
        "urn:x-turbowl-oracle:term:nonNegativeInteger-1",
        "1",
        "http://www.w3.org/2001/XMLSchema#nonNegativeInteger",
    ),
];

/// The most arguments an atom has (`prpPdw`, `sameKeyFrom`, ...).
pub(crate) const MAX_ARITY: usize = 4;

/// A relation's id: its index in [`Program::relations`].
pub(crate) type RelId = u16;

/// `T`, the closure: always relation 0.
pub(crate) const T: RelId = 0;

/// A test on terms that TurbOWL's datatype code answers (`literals.rls`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Builtin {
    /// `ruleLiteral(x)`: `x` is a literal the rules name.
    RuleLiteral,
    /// `typesOf(lt, dt)`: `lt` has a value in `dt`'s value space.
    TypesOf,
    /// `illTyped(lt, dt)`: `lt` is ill-typed, and `dt` is its datatype.
    IllTyped,
    /// `outsideValueSpace(lt, dt)`: `dt` is a checked datatype whose value
    /// space does not hold `lt`.
    OutsideValueSpace,
    /// `valueId(lt, v)`: `lt` has a value, and `v` names it: two literals
    /// have one `v` when they have one value.
    ValueId,
}

const BUILTINS: [(&str, Builtin, usize); 5] = [
    ("ruleLiteral", Builtin::RuleLiteral, 1),
    ("typesOf", Builtin::TypesOf, 2),
    ("illTyped", Builtin::IllTyped, 2),
    ("outsideValueSpace", Builtin::OutsideValueSpace, 2),
    ("valueId", Builtin::ValueId, 2),
];

/// What a relation is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// `T`: the closure, generalized triples included.
    Triple,
    /// A relation the rules derive: a helper (`listCell`, `chainFrom`, ...)
    /// or a finding (`caxDw`, ...).
    Derived,
    Builtin(Builtin),
}

#[derive(Debug, Clone)]
pub(crate) struct Relation {
    pub(crate) name: &'static str,
    pub(crate) arity: usize,
    pub(crate) kind: Kind,
}

/// A term of an atom as written.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Written {
    Var(u8),
    Iri(String),
    /// A literal the oracle names as an IRI: lexical form, datatype.
    Literal(&'static str, &'static str),
    /// dt-diff's unlisted predicate ([`UNLISTED`]).
    Unlisted,
}

#[derive(Debug, Clone)]
struct WrittenAtom {
    rel: RelId,
    args: Vec<Written>,
}

#[derive(Debug, Clone)]
enum WrittenBody {
    Atom(WrittenAtom),
    NotEqual(Written, Written),
}

/// A rule as parsed (after the rewrites).
#[derive(Debug, Clone)]
struct WrittenRule {
    /// The W3C name; `None` for a helper relation's rule.
    name: Option<&'static str>,
    group: Option<Switch>,
    head: WrittenAtom,
    body: Vec<WrittenBody>,
    vars: usize,
}

/// The parsed program: every file, before a configuration picks its groups.
#[derive(Debug)]
pub(crate) struct Program {
    pub(crate) relations: Vec<Relation>,
    rules: Vec<WrittenRule>,
    /// Each finding relation, by the code of its diagnostic (`cax-dw`).
    findings: Vec<(String, RelId)>,
}

/// The parsed program, once per process.
pub(crate) fn program() -> &'static Program {
    static PROGRAM: OnceLock<Program> = OnceLock::new();
    PROGRAM.get_or_init(|| parse(&FILES).unwrap_or_else(|e| panic!("the rule program: {e}")))
}

/// An argument of a rule ready to run: a variable or a term id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arg {
    Var(u8),
    /// A term id, or [`NONE`] for a term the dictionary does not hold
    /// (which matches nothing).
    Id(u32),
}

/// The id of a constant the dictionary does not hold.
pub(crate) const NONE: u32 = u32::MAX;

#[derive(Debug, Clone)]
pub(crate) struct Atom {
    pub(crate) rel: RelId,
    pub(crate) args: Vec<Arg>,
}

#[derive(Debug, Clone)]
pub(crate) enum Body {
    Atom(Atom),
    NotEqual(Arg, Arg),
}

#[derive(Debug, Clone)]
pub(crate) struct Rule {
    pub(crate) name: Option<&'static str>,
    pub(crate) head: Atom,
    pub(crate) body: Vec<Body>,
    pub(crate) vars: usize,
}

/// The program of one configuration, its constants read with one
/// dictionary.
#[derive(Debug)]
pub(crate) struct Compiled {
    pub(crate) relations: &'static [Relation],
    pub(crate) rules: Vec<Rule>,
    /// The rules of each relation's head, by relation id.
    pub(crate) by_head: Vec<Vec<usize>>,
    pub(crate) findings: Vec<(&'static str, RelId)>,
    /// The eq-* rules of `T`, when `equality` is on.
    pub(crate) eq: Option<EqRules>,
    /// By rule: one of [`Compiled::eq`]'s, which the search replaces by
    /// steps through the canon unless it falls back to them.
    pub(crate) is_eq: Vec<bool>,
}

/// The indices of the eq-* rules that conclude `T`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EqRules {
    pub(crate) sym: usize,
    pub(crate) trans: usize,
    /// eq-rep-s, eq-rep-p, eq-rep-o: by the position they rewrite.
    pub(crate) rep: [usize; 3],
}

impl Program {
    /// The groups `switches` leave on, with their constants as term ids.
    pub(crate) fn compile(&'static self, switches: Switches, terms: &Dictionary) -> Compiled {
        let id = |w: &Written| -> Arg {
            match w {
                Written::Var(v) => Arg::Var(*v),
                Written::Iri(iri) => Arg::Id(terms.get_iri(iri).map_or(NONE, |t| t.0)),
                Written::Literal(lexical, datatype) => {
                    let literal =
                        oxrdf::LiteralRef::new_typed_literal(lexical, oxrdf::NamedNodeRef::new_unchecked(datatype));
                    Arg::Id(terms.get_term(literal.into(), DocId::NONE).map_or(NONE, |t| t.0))
                }
                Written::Unlisted => Arg::Id(UNLISTED),
            }
        };
        let atom = |a: &WrittenAtom| Atom {
            rel: a.rel,
            args: a.args.iter().map(id).collect(),
        };
        let rules: Vec<Rule> = self
            .rules
            .iter()
            .filter(|r| r.group.is_none_or(|g| switches.is_on(g)))
            .map(|r| Rule {
                name: r.name,
                head: atom(&r.head),
                body: r
                    .body
                    .iter()
                    .map(|b| match b {
                        WrittenBody::Atom(a) => Body::Atom(atom(a)),
                        WrittenBody::NotEqual(x, y) => Body::NotEqual(id(x), id(y)),
                    })
                    .collect(),
                vars: r.vars,
            })
            .collect();
        let mut by_head = vec![Vec::new(); self.relations.len()];
        for (i, r) in rules.iter().enumerate() {
            by_head[r.head.rel as usize].push(i);
        }
        let named = |name: &str| {
            by_head[T as usize]
                .iter()
                .copied()
                .find(|&i| rules[i].name == Some(name))
        };
        let eq = (|| {
            Some(EqRules {
                sym: named("eq-sym")?,
                trans: named("eq-trans")?,
                rep: [named("eq-rep-s")?, named("eq-rep-p")?, named("eq-rep-o")?],
            })
        })();
        let is_eq = (0..rules.len())
            .map(|i| eq.is_some_and(|e| i == e.sym || i == e.trans || e.rep.contains(&i)))
            .collect();
        Compiled {
            relations: &self.relations,
            rules,
            by_head,
            findings: self.findings.iter().map(|(code, rel)| (code.as_str(), *rel)).collect(),
            eq,
            is_eq,
        }
    }

    /// The finding relation of a diagnostic code.
    #[cfg(test)]
    pub(crate) fn finding(&self, code: &str) -> Option<RelId> {
        self.findings.iter().find(|(c, _)| c == code).map(|&(_, r)| r)
    }

    /// The rules of every group, written back as text (one per line, with
    /// its group and name), for the tests.
    #[cfg(test)]
    pub(crate) fn lines(&self) -> Vec<String> {
        let term = |w: &Written| match w {
            Written::Var(v) => format!("?{v}"),
            Written::Iri(i) => format!("<{i}>"),
            Written::Literal(l, d) => format!("\"{l}\"^^<{d}>"),
            Written::Unlisted => format!("<{UNLISTED_IRI}>"),
        };
        let atom = |a: &WrittenAtom| {
            let args: Vec<String> = a.args.iter().map(term).collect();
            format!("{}({})", self.relations[a.rel as usize].name, args.join(", "))
        };
        self.rules
            .iter()
            .map(|r| {
                let body: Vec<String> = r
                    .body
                    .iter()
                    .map(|b| match b {
                        WrittenBody::Atom(a) => atom(a),
                        WrittenBody::NotEqual(x, y) => format!("{} != {}", term(x), term(y)),
                    })
                    .collect();
                format!(
                    "[{}] {} :- {} . % {}",
                    r.group.map_or("core", Switch::name),
                    atom(&r.head),
                    body.join(", "),
                    r.name.unwrap_or("-")
                )
            })
            .collect()
    }
}

/// A token of a file, with its line.
#[derive(Debug, Clone, PartialEq)]
enum Token<'a> {
    /// A word: a relation, a prefixed name, a directive (`@prefix`).
    Word(&'a str),
    Var(&'a str),
    Iri(&'a str),
    Str(&'a str),
    Punct(&'a str),
    /// A comment's text, after the `%`.
    Comment(&'a str),
}

fn tokens(text: &str) -> Result<Vec<(Token<'_>, usize)>, String> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let (mut i, mut line) = (0, 1);
    let word = |c: u8| c.is_ascii_alphanumeric() || matches!(c, b'_' | b':' | b'-' | b'@' | b'#');
    while i < bytes.len() {
        let c = bytes[i];
        let start = i;
        match c {
            b'\n' => {
                line += 1;
                i += 1;
            }
            c if c.is_ascii_whitespace() => i += 1,
            b'%' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                out.push((Token::Comment(text[start + 1..i].trim()), line));
            }
            b'<' => {
                while i < bytes.len() && bytes[i] != b'>' {
                    i += 1;
                }
                if i == bytes.len() {
                    return Err(format!("line {line}: an IRI is not closed"));
                }
                out.push((Token::Iri(&text[start + 1..i]), line));
                i += 1;
            }
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += 1;
                }
                if i == bytes.len() {
                    return Err(format!("line {line}: a string is not closed"));
                }
                out.push((Token::Str(&text[start + 1..i]), line));
                i += 1;
            }
            b'?' => {
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                out.push((Token::Var(&text[start + 1..i]), line));
            }
            b':' if bytes.get(i + 1) == Some(&b'-') => {
                out.push((Token::Punct(":-"), line));
                i += 2;
            }
            b'!' if bytes.get(i + 1) == Some(&b'=') => {
                out.push((Token::Punct("!="), line));
                i += 2;
            }
            b'(' | b')' | b',' | b'.' | b'{' | b'}' | b'=' => {
                out.push((Token::Punct(&text[i..i + 1]), line));
                i += 1;
            }
            c if word(c) => {
                while i < bytes.len() && word(bytes[i]) {
                    i += 1;
                }
                out.push((Token::Word(&text[start..i]), line));
            }
            _ => return Err(format!("line {line}: unexpected {:?}", c as char)),
        }
    }
    Ok(out)
}

/// The parser's state over all files.
struct Parser {
    prefixes: HashMap<String, String>,
    relations: Vec<Relation>,
    rules: Vec<WrittenRule>,
    findings: Vec<(String, RelId)>,
}

impl Parser {
    fn relation(&mut self, name: &'static str, arity: usize) -> Result<RelId, String> {
        if let Some(i) = self.relations.iter().position(|r| r.name == name) {
            if self.relations[i].arity != arity {
                return Err(format!("{name} has {} arguments and {arity}", self.relations[i].arity));
            }
            return Ok(i as RelId);
        }
        let kind = match BUILTINS.iter().find(|(n, _, _)| *n == name) {
            Some(&(_, b, a)) if a == arity => Kind::Builtin(b),
            Some(_) => return Err(format!("the test {name} takes other arguments")),
            None if name == "T" => Kind::Triple,
            None => Kind::Derived,
        };
        self.relations.push(Relation { name, arity, kind });
        Ok((self.relations.len() - 1) as RelId)
    }

    fn constant(&self, token: &Token<'static>) -> Result<Written, String> {
        let iri = match token {
            Token::Iri(iri) => (*iri).to_owned(),
            Token::Word(name) => {
                let (prefix, local) = name.split_once(':').ok_or_else(|| format!("{name} is not a term"))?;
                let namespace = self
                    .prefixes
                    .get(prefix)
                    .ok_or_else(|| format!("the prefix {prefix}: is not declared"))?;
                format!("{namespace}{local}")
            }
            other => return Err(format!("{other:?} is not a term")),
        };
        if iri == UNLISTED_IRI {
            return Ok(Written::Unlisted);
        }
        Ok(match NAMED_LITERALS.iter().find(|(named, _, _)| *named == iri) {
            Some(&(_, lexical, datatype)) => Written::Literal(lexical, datatype),
            None => Written::Iri(iri),
        })
    }
}

fn parse(files: &[(&'static str, Option<Switch>, &'static str)]) -> Result<Program, String> {
    let mut p = Parser {
        prefixes: HashMap::new(),
        relations: Vec::new(),
        rules: Vec::new(),
        findings: Vec::new(),
    };
    p.relation("T", 3)?;
    for &(file, group, text) in files {
        let from = p.rules.len();
        parse_file(&mut p, group, text).map_err(|e| format!("{file}: {e}"))?;
        // dt-diff as the oracle files write it, and the `valueOf` it reads
        // (see the module doc): `values.rls` states it as a proof shows it.
        if file != "values.rls" {
            let value_of = p.relations.iter().position(|r| r.name == "valueOf").map(|i| i as RelId);
            let mut k = 0;
            p.rules.retain(|r| {
                k += 1;
                k <= from || (r.name != Some("dt-diff") && Some(r.head.rel) != value_of)
            });
        }
    }
    // The rewrites: Tp to T, and the rules that only steer Nemo's joins (or
    // read the base) dropped.
    let named = |p: &Parser, name: &str| p.relations.iter().position(|r| r.name == name).map(|i| i as RelId);
    let (tp, used, input) = (named(&p, "Tp"), named(&p, "usedProp"), named(&p, "input"));
    p.rules.retain(|r| {
        let reads_input = r
            .body
            .iter()
            .any(|b| matches!(b, WrittenBody::Atom(a) if Some(a.rel) == input));
        Some(r.head.rel) != tp && Some(r.head.rel) != used && !reads_input
    });
    for rule in &mut p.rules {
        for b in &mut rule.body {
            if let WrittenBody::Atom(a) = b
                && Some(a.rel) == tp
            {
                let [pr, x, y]: [Written; 3] = std::mem::take(&mut a.args)
                    .try_into()
                    .map_err(|_| "Tp takes three arguments".to_owned())?;
                *a = WrittenAtom {
                    rel: T,
                    args: vec![x, pr, y],
                };
            }
        }
        if let Some(b) = rule
            .body
            .iter()
            .find(|b| matches!(b, WrittenBody::Atom(a) if Some(a.rel) == used))
        {
            return Err(format!("a rule reads usedProp: {b:?}"));
        }
    }
    Ok(Program {
        relations: p.relations,
        rules: p.rules,
        findings: p.findings,
    })
}

fn parse_file(p: &mut Parser, group: Option<Switch>, text: &'static str) -> Result<(), String> {
    let tokens = tokens(text)?;
    let mut i = 0;
    let at = |i: usize| tokens.get(i).map(|(t, _)| t.clone());
    while i < tokens.len() {
        let (token, line) = tokens[i].clone();
        if let Token::Comment(_) = token {
            i += 1;
            continue;
        }
        // A statement: up to its `.`, outside IRIs and strings.
        let end = (i..tokens.len())
            .find(|&j| tokens[j].0 == Token::Punct("."))
            .ok_or_else(|| format!("line {line}: a statement has no end"))?;
        let statement: Vec<Token<'static>> = tokens[i..end]
            .iter()
            .map(|(t, _)| t.clone())
            .filter(|t| !matches!(t, Token::Comment(_)))
            .collect();
        let end_line = tokens[end].1;
        // The rule's name: the comment on the line of its `.`.
        let name = match at(end + 1) {
            Some(Token::Comment(text)) if tokens[end + 1].1 == end_line => text.split_whitespace().next(),
            _ => None,
        };
        i = end + 1;
        match statement.first() {
            Some(Token::Word("@prefix")) => match &statement[..] {
                [_, Token::Word(prefix), Token::Iri(iri)] => {
                    let prefix = prefix.strip_suffix(':').ok_or("a prefix ends in ':'")?;
                    p.prefixes.insert(prefix.to_owned(), (*iri).to_owned());
                }
                other => return Err(format!("line {line}: a prefix: {other:?}")),
            },
            Some(Token::Word("@import")) => {}
            Some(Token::Word("@export")) => {
                if let [
                    _,
                    Token::Word(rel),
                    Token::Punct(":-"),
                    Token::Word("csv"),
                    ..,
                    Token::Str(resource),
                    _,
                ] = &statement[..]
                {
                    let code = resource.strip_suffix(".csv").ok_or("a finding's file is a .csv")?;
                    let id = p
                        .relations
                        .iter()
                        .position(|r| r.name == *rel)
                        .ok_or_else(|| format!("line {line}: {rel} is exported before its rules"))?;
                    p.findings.push((code.to_owned(), id as RelId));
                }
            }
            Some(_) => {
                let rule = parse_rule(p, &statement, group, name).map_err(|e| format!("line {line}: {e}"))?;
                p.rules.push(rule);
            }
            None => return Err(format!("line {line}: an empty statement")),
        }
    }
    Ok(())
}

fn parse_rule(
    p: &mut Parser,
    tokens: &[Token<'static>],
    group: Option<Switch>,
    name: Option<&'static str>,
) -> Result<WrittenRule, String> {
    let mut vars: Vec<&str> = Vec::new();
    let mut i = 0;
    let mut term = |p: &Parser, i: &mut usize| -> Result<Written, String> {
        let t = tokens.get(*i).ok_or("a term is missing")?;
        *i += 1;
        match t {
            Token::Var(v) => {
                let n = vars.iter().position(|w| w == v).unwrap_or_else(|| {
                    vars.push(v);
                    vars.len() - 1
                });
                Ok(Written::Var(u8::try_from(n).map_err(|_| "too many variables")?))
            }
            other => p.constant(other),
        }
    };
    let atom = |p: &mut Parser, i: &mut usize, term: &mut dyn FnMut(&Parser, &mut usize) -> Result<Written, String>| {
        let Some(Token::Word(rel)) = tokens.get(*i) else {
            return Err(format!("an atom starts with a relation, not {:?}", tokens.get(*i)));
        };
        *i += 1;
        if tokens.get(*i) != Some(&Token::Punct("(")) {
            return Err(format!("{rel} has no arguments"));
        }
        *i += 1;
        let mut args = Vec::new();
        loop {
            args.push(term(p, i)?);
            match tokens.get(*i) {
                Some(Token::Punct(",")) => *i += 1,
                Some(Token::Punct(")")) => {
                    *i += 1;
                    break;
                }
                other => return Err(format!("{rel}: {other:?} in its arguments")),
            }
        }
        if args.len() > MAX_ARITY {
            return Err(format!("{rel} has more than {MAX_ARITY} arguments"));
        }
        let rel = p.relation(rel, args.len())?;
        Ok(WrittenAtom { rel, args })
    };
    let head = atom(p, &mut i, &mut term)?;
    let mut body = Vec::new();
    if i < tokens.len() {
        if tokens[i] != Token::Punct(":-") {
            return Err(format!("{:?} after the head", tokens[i]));
        }
        i += 1;
        loop {
            if matches!(tokens.get(i + 1), Some(Token::Punct("!="))) {
                let x = term(p, &mut i)?;
                i += 1;
                let y = term(p, &mut i)?;
                body.push(WrittenBody::NotEqual(x, y));
            } else {
                body.push(WrittenBody::Atom(atom(p, &mut i, &mut term)?));
            }
            match tokens.get(i) {
                Some(Token::Punct(",")) => i += 1,
                None => break,
                Some(other) => return Err(format!("{other:?} in a body")),
            }
        }
    }
    // Safety: every head variable is bound by a body atom.
    let bound = |v: u8| {
        body.iter().any(|b| match b {
            WrittenBody::Atom(a) => a.args.contains(&Written::Var(v)),
            WrittenBody::NotEqual(..) => false,
        })
    };
    if let Some(Written::Var(v)) = head.args.iter().find(|a| matches!(a, Written::Var(v) if !bound(*v))) {
        return Err(format!("the head variable ?{} is not bound", vars[*v as usize]));
    }
    Ok(WrittenRule {
        name,
        group,
        head,
        body,
        vars: vars.len(),
    })
}
