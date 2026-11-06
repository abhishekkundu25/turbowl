//! The rule program for a configuration, for each Datalog engine: one rule
//! source, `oracles/rules/`, emitted for Nemo, Soufflé and VLog.
//!
//! The source is one Nemo file per switch group: `core.rls`, always on, and
//! one file named after each [`Switch`] (`Switch::name`). The program for a
//! [`Switches`] value is the core followed by the file of each switch that
//! is on, in [`Switch::ALL`] order. [`nemo`] gives those files as they are;
//! [`souffle`] and [`vlog`] translate them, statement by statement, so a rule
//! is written once, in Nemo's syntax, and every engine runs the same rules.
//!
//! The source keeps to the part of Nemo's language that the translations
//! read: `@prefix`, `@import` and `@export` with a `resource`, facts, and
//! rules whose body holds atoms and `!=` (a variable against a constant or
//! another variable). The translations panic
//! on anything else, and the golden programs in `tests/programs/` cover the
//! complete configuration, so a source they cannot read fails the tests.
//!
//! The programs share the source's conventions: every term is an IRI (the
//! oracle encodes literals and blank nodes as IRIs), the rules run over
//! generalized triples, and each rule that concludes `false` has its own
//! output, named after the rule.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use turbowl::{Switch, Switches};

/// The Nemo program: the group files as they are, each after a comment
/// naming it.
pub fn nemo(switches: Switches) -> String {
    let mut out = header("%", switches);
    for (name, text) in groups(switches) {
        let _ = write!(out, "\n% oracles/rules/{name}.rls\n\n{text}");
    }
    out
}

/// The finding codes the program of a configuration exports: each rule
/// that concludes `false` has a CSV output named after it.
pub fn codes(switches: Switches) -> Vec<String> {
    Program::parse(switches)
        .statements()
        .filter_map(|(statement, _)| match statement {
            Statement::Export { resource, .. } => resource.strip_suffix(".csv").map(str::to_owned),
            _ => None,
        })
        .collect()
}

/// The rules the program of a configuration holds: the W3C names that end
/// its rules as comments (`% cax-sco`), not those listed as to come or left
/// out.
pub fn rules(switches: Switches) -> std::collections::BTreeSet<String> {
    let is_rule = |r: &str| {
        r.contains('-')
            && r.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    Program::parse(switches)
        .statements()
        .filter_map(|(statement, trailing)| match statement {
            Statement::Rule(_) => trailing.as_deref(),
            _ => None,
        })
        .flat_map(|comment| comment.split(',').map(str::trim))
        .filter(|name| is_rule(name))
        .map(str::to_owned)
        .collect()
}

/// The Soufflé program (souffle-lang.github.io): a `.decl` for each relation
/// (every attribute a `symbol`), then the rules.
///
/// - A constant is the symbol of its IRI in N-Triples form,
///   `"<http://www.w3.org/2002/07/owl#sameAs>"`; Soufflé has no prefixes.
/// - A variable `?x` is `x`, or `_` when it occurs once in its rule.
/// - A relation or variable named by a word Soufflé reserves (`input`,
///   `output`, `count`, ...: `SOUFFLE_KEYWORDS`) takes a final `_`:
///   `input_`.
/// - `@import` and `@export` become `.input` and `.output` of tab-separated
///   files, the resource's name with the extension `.tsv` (`input.tsv`,
///   `closure.tsv`, `cax-dw.tsv`), each term in N-Triples form, so a line of
///   `closure.tsv` followed by ` .` is an N-Triples line.
pub fn souffle(switches: Switches) -> String {
    let program = Program::parse(switches);
    for relation in program.arities.keys() {
        assert!(
            souffle_ident(relation) == *relation || !program.arities.contains_key(&souffle_ident(relation)),
            "{relation} and {} would be one relation in Soufflé",
            souffle_ident(relation)
        );
    }
    let mut out = header("//", switches);
    out.push('\n');
    for (relation, arity) in &program.arities {
        let attributes: Vec<String> = (0..*arity).map(|i| format!("a{i}: symbol")).collect();
        let _ = writeln!(out, ".decl {}({})", souffle_ident(relation), attributes.join(", "));
    }
    for (name, items) in &program.groups {
        let _ = writeln!(out, "\n// oracles/rules/{name}.rls\n");
        for item in items {
            match item {
                Item::Blank => out.push('\n'),
                Item::Comment(text) => {
                    let _ = writeln!(out, "//{text}");
                }
                Item::Statement(Statement::Prefix { .. }, _) => {}
                Item::Statement(Statement::Import { relation, resource, .. }, trailing) => {
                    let line = format!(
                        ".input {}(IO=file, filename=\"{}\", delimiter=\"\\t\")",
                        souffle_ident(relation),
                        tsv(resource)
                    );
                    line_with(&mut out, &line, trailing, "//");
                }
                Item::Statement(Statement::Export { relation, resource, .. }, trailing) => {
                    let line = format!(
                        ".output {}(IO=file, filename=\"{}\", delimiter=\"\\t\")",
                        souffle_ident(relation),
                        tsv(resource)
                    );
                    line_with(&mut out, &line, trailing, "//");
                }
                Item::Statement(Statement::Rule(rule), trailing) => {
                    line_with(&mut out, &souffle_rule(rule, &program.prefixes), trailing, "//");
                }
            }
        }
    }
    single_blank_lines(out)
}

/// The VLog program, in the rule syntax of Rulewerk
/// (github.com/knowsys/rulewerk, wiki "Rule syntax by examples"), through
/// which VLog runs.
///
/// - The prefixes and the `@source` declarations come first, then the rules
///   in the source's order.
/// - `@import r :- ntriples{resource="f"}` (or `turtle`) is
///   `@source r[3]: load-rdf("f") .`; a `csv` import is `load-csv`.
/// - Rulewerk has no comparison, so `?x != c` is `~sameTerm(?x, c)`, with a
///   fact `sameTerm(c, c) .` for each such constant: stratified negation
///   over facts alone. `?x != ?y` is `~sameBound(?x, ?y)`, with a rule
///   `sameBound(?x, ?x) :- A .` for each variable and `A` the first atom of
///   its rule's body that binds it: stratified while no relation that `A`
///   reads depends on the rule's head (the translation checks that `A` is
///   not the head's relation).
/// - A rule file has no exports. They close the program as comments that
///   hold the Rulewerk client's command for each (`@query` with
///   `EXPORTCSV`, the resource's name with the extension `.csv`), for the
///   runner to issue after `@reason .`.
pub fn vlog(switches: Switches) -> String {
    let program = Program::parse(switches);
    for name in [SAME_TERM, SAME_BOUND] {
        assert!(
            !program.arities.contains_key(name),
            "the rule source names `{name}`, which the VLog program uses for !="
        );
    }
    let mut out = header("%", switches);
    out.push('\n');
    let mut exports = Vec::new();
    let mut distinct = Distinct::default();
    for (_, items) in &program.groups {
        for item in items {
            match item {
                Item::Statement(Statement::Prefix { name, iri }, _) => {
                    let _ = writeln!(out, "@prefix {name}: <{iri}> .");
                }
                Item::Statement(
                    Statement::Import {
                        relation,
                        format,
                        resource,
                    },
                    _,
                ) => {
                    let loader = match format.as_str() {
                        "ntriples" | "turtle" => "load-rdf",
                        "csv" => "load-csv",
                        other => panic!("no Rulewerk source for the Nemo format {other}"),
                    };
                    let arity = program.arities[relation];
                    let _ = writeln!(out, "@source {relation}[{arity}]: {loader}(\"{resource}\") .");
                }
                _ => {}
            }
        }
    }
    for (name, items) in &program.groups {
        let _ = writeln!(out, "\n% oracles/rules/{name}.rls\n");
        for item in items {
            match item {
                Item::Blank => out.push('\n'),
                Item::Comment(text) => {
                    let _ = writeln!(out, "%{text}");
                }
                Item::Statement(Statement::Prefix { .. } | Statement::Import { .. }, _) => {}
                Item::Statement(Statement::Export { relation, resource, .. }, _) => {
                    exports.push((relation, resource));
                }
                Item::Statement(Statement::Rule(rule), trailing) => {
                    line_with(&mut out, &vlog_rule(rule, &mut distinct), trailing, "%");
                }
            }
        }
    }
    if !distinct.constants.is_empty() {
        out.push_str("\n% `?x != c` in the source: sameTerm(c, c) for each such constant.\n");
        for c in &distinct.constants {
            let _ = writeln!(out, "{SAME_TERM}({c}, {c}) .");
        }
    }
    if !distinct.bound.is_empty() {
        out.push_str("\n% `?x != ?y` in the source: sameBound(?x, ?x) for each term an atom binds ?x to.\n");
        for rule in &distinct.bound {
            let _ = writeln!(out, "{rule}");
        }
    }
    if !exports.is_empty() {
        out.push_str("\n% The exports, as Rulewerk client commands to issue after `@reason .`:\n");
        for (relation, resource) in exports {
            let terms: Vec<String> = (1..=program.arities[relation]).map(|i| format!("?x{i}")).collect();
            let file = with_extension(resource, "csv");
            let _ = writeln!(out, "% @query {relation}({}) EXPORTCSV \"{file}\" .", terms.join(", "));
        }
    }
    single_blank_lines(out)
}

/// The bare words Soufflé's scanner reads as keywords
/// (`src/parser/scanner.ll`), which cannot name a relation or a variable.
const SOUFFLE_KEYWORDS: &[&str] = &[
    "debug_delta",
    "autoinc",
    "band",
    "bor",
    "bxor",
    "bnot",
    "bshl",
    "bshr",
    "bshru",
    "land",
    "lor",
    "lxor",
    "lnot",
    "match",
    "mean",
    "cat",
    "ord",
    "range",
    "strlen",
    "substr",
    "stateful",
    "contains",
    "output",
    "input",
    "overridable",
    "printsize",
    "eqrel",
    "inline",
    "no_inline",
    "magic",
    "no_magic",
    "brie",
    "btree_delete",
    "btree",
    "min",
    "max",
    "as",
    "nil",
    "count",
    "sum",
    "true",
    "false",
    "to_float",
    "to_number",
    "to_string",
    "to_unsigned",
    "recursive_iteration_cnt",
];

/// A relation's or variable's name in Soufflé: the name, with a final `_`
/// when Soufflé reserves it.
fn souffle_ident(name: &str) -> String {
    if SOUFFLE_KEYWORDS.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

/// The relation the VLog program uses for `!=` against a constant.
const SAME_TERM: &str = "sameTerm";

/// The relation the VLog program uses for `!=` between two variables.
const SAME_BOUND: &str = "sameBound";

/// What the VLog program adds for `!=`: the constants compared, and the
/// rules that bind the variables compared.
#[derive(Default)]
struct Distinct {
    constants: Vec<String>,
    bound: Vec<String>,
}

/// The text of a group's file: the core for `None`. A match, so a new
/// switch has no program until it has a file.
fn source(group: Option<Switch>) -> &'static str {
    match group {
        None => include_str!("../../../oracles/rules/core.rls"),
        Some(Switch::Equality) => include_str!("../../../oracles/rules/equality.rls"),
        Some(Switch::PropertyChains) => include_str!("../../../oracles/rules/property-chains.rls"),
        Some(Switch::Checks) => include_str!("../../../oracles/rules/checks.rls"),
        Some(Switch::Identity) => include_str!("../../../oracles/rules/identity.rls"),
        Some(Switch::Schema) => include_str!("../../../oracles/rules/schema.rls"),
        Some(Switch::Trivial) => include_str!("../../../oracles/rules/trivial.rls"),
        Some(Switch::LiteralValues) => include_str!("../../../oracles/rules/literal-values.rls"),
    }
}

/// The groups of the program, in order: each file's name (without `.rls`)
/// and text.
fn groups(switches: Switches) -> Vec<(&'static str, &'static str)> {
    let on = Switch::ALL.into_iter().filter(|&s| switches.is_on(s));
    std::iter::once(("core", source(None)))
        .chain(on.map(|s| (s.name(), source(Some(s)))))
        .collect()
}

/// The program's first lines: where it comes from and its groups.
fn header(comment: &str, switches: Switches) -> String {
    let names: Vec<&str> = groups(switches).into_iter().map(|(n, _)| n).collect();
    format!(
        "{comment} Composed by crates/oracle (src/programs.rs) from oracles/rules/: do not edit.\n\
         {comment} Groups: {}.\n",
        names.join(", ")
    )
}

/// Writes a statement on one line, with the comment that ended it in the
/// source.
fn line_with(out: &mut String, line: &str, trailing: &Option<String>, comment: &str) {
    match trailing {
        Some(text) => {
            let _ = writeln!(out, "{line}  {comment}{text}");
        }
        None => {
            let _ = writeln!(out, "{line}");
        }
    }
}

/// The text with no two blank lines in a row (a directive moved or dropped
/// leaves its blank lines behind).
fn single_blank_lines(mut text: String) -> String {
    while text.contains("\n\n\n") {
        text = text.replace("\n\n\n", "\n\n");
    }
    text
}

/// `input.nt` → `input.tsv`.
fn tsv(resource: &str) -> String {
    with_extension(resource, "tsv")
}

fn with_extension(resource: &str, extension: &str) -> String {
    let stem = resource.rsplit_once('.').map_or(resource, |(stem, _)| stem);
    format!("{stem}.{extension}")
}

fn souffle_rule(rule: &Rule, prefixes: &BTreeMap<String, String>) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for term in rule.terms() {
        if let Term::Var(v) = term {
            *counts.entry(v).or_default() += 1;
        }
    }
    let term = |t: &Term| match t {
        Term::Var(v) if counts[v.as_str()] == 1 => "_".to_string(),
        Term::Var(v) => souffle_ident(v),
        Term::Const(c) => {
            let iri = resolve(c, prefixes);
            assert!(!iri.contains(['"', '\\']), "{iri}: a quote or backslash in an IRI");
            format!("\"<{iri}>\"")
        }
    };
    let atom = |a: &Atom| {
        let terms: Vec<String> = a.terms.iter().map(&term).collect();
        format!("{}({})", souffle_ident(&a.relation), terms.join(", "))
    };
    let head: Vec<String> = rule.head.iter().map(&atom).collect();
    if rule.body.is_empty() {
        return format!("{}.", head.join(", "));
    }
    let body: Vec<String> = rule
        .body
        .iter()
        .map(|l| match l {
            Literal::Atom(a) => atom(a),
            Literal::NotEqual(x, y) => format!("{} != {}", term(x), term(y)),
        })
        .collect();
    format!("{} :- {}.", head.join(", "), body.join(", "))
}

fn vlog_rule(rule: &Rule, distinct: &mut Distinct) -> String {
    let term = |t: &Term| match t {
        Term::Var(v) => format!("?{v}"),
        Term::Const(c) => c.clone(),
    };
    let atom = |a: &Atom| {
        let terms: Vec<String> = a.terms.iter().map(term).collect();
        format!("{}({})", a.relation, terms.join(", "))
    };
    let head: Vec<String> = rule.head.iter().map(atom).collect();
    if rule.body.is_empty() {
        return format!("{} .", head.join(", "));
    }
    // The first atom of the body that binds `v`.
    let binding = |v: &str| {
        rule.body
            .iter()
            .find_map(|l| match l {
                Literal::Atom(a) if a.terms.iter().any(|t| matches!(t, Term::Var(x) if x == v)) => Some(a),
                _ => None,
            })
            .unwrap_or_else(|| panic!("?{v} is compared but no atom binds it"))
    };
    let body: Vec<String> = rule
        .body
        .iter()
        .map(|l| match l {
            Literal::Atom(a) => atom(a),
            Literal::NotEqual(x @ Term::Var(_), Term::Const(c))
            | Literal::NotEqual(Term::Const(c), x @ Term::Var(_)) => {
                if !distinct.constants.contains(c) {
                    distinct.constants.push(c.clone());
                }
                format!("~{SAME_TERM}({}, {c})", term(x))
            }
            Literal::NotEqual(Term::Var(x), Term::Var(y)) => {
                for v in [x, y] {
                    let a = binding(v);
                    assert!(
                        rule.head.iter().all(|h| h.relation != a.relation),
                        "?{v} != ...: {} binds it and is the rule's head, which Rulewerk cannot stratify",
                        a.relation
                    );
                    let bound = format!("{SAME_BOUND}(?{v}, ?{v}) :- {} .", atom(a));
                    if !distinct.bound.contains(&bound) {
                        distinct.bound.push(bound);
                    }
                }
                format!("~{SAME_BOUND}(?{x}, ?{y})")
            }
            Literal::NotEqual(x, y) => panic!("{} != {}: two constants", term(x), term(y)),
        })
        .collect();
    format!("{} :- {} .", head.join(", "), body.join(", "))
}

/// A prefixed name (`owl:Thing`) or a full IRI (`<...>`), as an IRI.
fn resolve(constant: &str, prefixes: &BTreeMap<String, String>) -> String {
    if let Some(iri) = constant.strip_prefix('<').and_then(|c| c.strip_suffix('>')) {
        return iri.to_string();
    }
    let (prefix, local) = constant
        .split_once(':')
        .unwrap_or_else(|| panic!("{constant}: not an IRI or a prefixed name"));
    let iri = prefixes
        .get(prefix)
        .unwrap_or_else(|| panic!("{constant}: the prefix {prefix}: is not declared in core.rls"));
    format!("{iri}{local}")
}

/// The source of a configuration, parsed: each group's items, the prefixes
/// and each relation's arity (in name order).
struct Program {
    groups: Vec<(&'static str, Vec<Item>)>,
    prefixes: BTreeMap<String, String>,
    arities: BTreeMap<String, usize>,
}

/// A line of a group file: a blank line, a comment line (its text after
/// `%`), or a statement and the comment on the line it ends.
enum Item {
    Blank,
    Comment(String),
    Statement(Statement, Option<String>),
}

enum Statement {
    Prefix {
        name: String,
        iri: String,
    },
    Import {
        relation: String,
        format: String,
        resource: String,
    },
    Export {
        relation: String,
        resource: String,
    },
    /// A rule, or a fact when its body is empty.
    Rule(Rule),
}

struct Rule {
    head: Vec<Atom>,
    body: Vec<Literal>,
}

impl Rule {
    fn terms(&self) -> impl Iterator<Item = &Term> {
        let head = self.head.iter().flat_map(|a| &a.terms);
        let body = self.body.iter().flat_map(|l| match l {
            Literal::Atom(a) => a.terms.iter().collect::<Vec<_>>(),
            Literal::NotEqual(x, y) => vec![x, y],
        });
        head.chain(body)
    }
}

struct Atom {
    relation: String,
    terms: Vec<Term>,
}

enum Literal {
    Atom(Atom),
    NotEqual(Term, Term),
}

enum Term {
    /// The name after `?`.
    Var(String),
    /// A prefixed name or `<IRI>`, as written.
    Const(String),
}

impl Program {
    fn parse(switches: Switches) -> Self {
        let mut program = Program {
            groups: Vec::new(),
            prefixes: BTreeMap::new(),
            arities: BTreeMap::new(),
        };
        for (name, text) in groups(switches) {
            let items = parse(text).unwrap_or_else(|e| panic!("oracles/rules/{name}.rls: {e}"));
            for item in &items {
                program.note(name, item);
            }
            program.groups.push((name, items));
        }
        program
    }

    /// Every statement, in order, with the comment on the line it ends.
    fn statements(&self) -> impl Iterator<Item = (&Statement, &Option<String>)> {
        self.groups.iter().flat_map(|(_, items)| {
            items.iter().filter_map(|item| match item {
                Item::Statement(statement, trailing) => Some((statement, trailing)),
                _ => None,
            })
        })
    }

    /// Records a statement's prefix or relations, checking each relation
    /// keeps one arity.
    fn note(&mut self, group: &str, item: &Item) {
        let Item::Statement(statement, _) = item else { return };
        let mut arity = |relation: &str, n: usize| {
            let known = *self.arities.entry(relation.to_string()).or_insert(n);
            assert_eq!(
                known, n,
                "oracles/rules/{group}.rls: {relation} has arity {known} and {n}"
            );
        };
        match statement {
            Statement::Prefix { name, iri } => {
                self.prefixes.insert(name.clone(), iri.clone());
            }
            // An imported relation holds triples.
            Statement::Import { relation, .. } => arity(relation, 3),
            Statement::Export { .. } => {}
            Statement::Rule(rule) => {
                for a in rule.head.iter().chain(rule.body.iter().filter_map(|l| match l {
                    Literal::Atom(a) => Some(a),
                    Literal::NotEqual(..) => None,
                })) {
                    arity(&a.relation, a.terms.len());
                }
            }
        }
    }
}

/// A token of the source.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// `?name`
    Var(String),
    /// `@name`
    Directive(String),
    /// `<...>`, with the brackets.
    Iri(String),
    /// `"..."`, without the quotes.
    Str(String),
    /// A relation name, a format name or a prefixed name (`owl:Thing`, `rdf:`).
    Name(String),
    /// `(`, `)`, `,`, `.`, `:-`, `!=`, `{`, `}`, `=`.
    Punct(&'static str),
}

/// The items of a group file.
fn parse(text: &str) -> Result<Vec<Item>, String> {
    let mut items = Vec::new();
    let mut statement: Vec<Token> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let at = |e: String| format!("line {}: {e}", n + 1);
        let (code, comment) = split_comment(line);
        let tokens = lex(code).map_err(at)?;
        if tokens.is_empty() && statement.is_empty() {
            match comment {
                Some(c) => items.push(Item::Comment(c.to_string())),
                None => items.push(Item::Blank),
            }
            continue;
        }
        // True from a statement's `.` to the end of its line.
        let mut just_ended = false;
        for token in tokens {
            if just_ended {
                return Err(at("a line holds more than one statement".into()));
            }
            if token == Token::Punct(".") {
                let s = parse_statement(&std::mem::take(&mut statement)).map_err(at)?;
                items.push(Item::Statement(s, None));
                just_ended = true;
            } else {
                statement.push(token);
            }
        }
        if let Some(c) = comment {
            match items.last_mut() {
                Some(Item::Statement(_, trailing)) if just_ended => *trailing = Some(c.to_string()),
                _ => return Err(at("a comment inside a statement".into())),
            }
        }
    }
    if !statement.is_empty() {
        return Err("the last statement has no final `.`".into());
    }
    Ok(items)
}

/// A line's code and its comment (the text after `%`, outside an IRI or a
/// string).
fn split_comment(line: &str) -> (&str, Option<&str>) {
    let mut in_iri = false;
    let mut in_str = false;
    for (i, c) in line.char_indices() {
        match c {
            '<' if !in_str => in_iri = true,
            '>' if !in_str => in_iri = false,
            '"' if !in_iri => in_str = !in_str,
            '%' if !in_iri && !in_str => return (&line[..i], Some(&line[i + 1..])),
            _ => {}
        }
    }
    (line, None)
}

fn lex(code: &str) -> Result<Vec<Token>, String> {
    let name_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    // The length in bytes of the run of `pred` characters at the start of `s`.
    let run = |s: &str, pred: &dyn Fn(char) -> bool| s.find(|c: char| !pred(c)).unwrap_or(s.len());
    let mut tokens = Vec::new();
    let mut i = 0;
    while let Some(c) = code[i..].chars().next() {
        let rest = &code[i..];
        let (token, len) = match c {
            c if c.is_whitespace() => {
                i += c.len_utf8();
                continue;
            }
            '?' | '@' => {
                let name = &rest[1..1 + run(&rest[1..], &|c| c.is_ascii_alphanumeric() || c == '_')];
                if name.is_empty() {
                    return Err(format!("`{c}` with no name"));
                }
                let token = if c == '?' {
                    Token::Var(name.to_string())
                } else {
                    Token::Directive(name.to_string())
                };
                (token, 1 + name.len())
            }
            '<' => {
                let end = rest.find('>').ok_or("an IRI with no `>`")?;
                (Token::Iri(rest[..=end].to_string()), end + 1)
            }
            '"' => {
                let end = rest[1..].find('"').ok_or("a string with no closing quote")? + 1;
                if rest[1..end].contains('\\') {
                    return Err("an escape in a string".into());
                }
                (Token::Str(rest[1..end].to_string()), end + 1)
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut len = run(rest, &name_char);
                // A prefixed name: `owl:Thing`, or `rdf:` in a @prefix, but not `input :-`.
                if rest[len..].starts_with(':') && !rest[len..].starts_with(":-") {
                    len += 1 + run(&rest[len + 1..], &name_char);
                }
                (Token::Name(rest[..len].to_string()), len)
            }
            _ => {
                let punct = ["(", ")", ",", ".", ":-", "!=", "{", "}", "="]
                    .into_iter()
                    .find(|p| rest.starts_with(p))
                    .ok_or_else(|| format!("unexpected `{c}`"))?;
                (Token::Punct(punct), punct.len())
            }
        };
        tokens.push(token);
        i += len;
    }
    Ok(tokens)
}

fn parse_statement(tokens: &[Token]) -> Result<Statement, String> {
    use Token::{Directive, Iri, Name, Punct, Str};
    match tokens {
        [Directive(d), Name(prefix), Iri(iri)] if d == "prefix" => {
            let name = prefix.strip_suffix(':').ok_or("@prefix: a name ending in `:`")?;
            Ok(Statement::Prefix {
                name: name.to_string(),
                iri: iri[1..iri.len() - 1].to_string(),
            })
        }
        [
            Directive(d),
            Name(relation),
            Punct(":-"),
            Name(format),
            Punct("{"),
            Name(key),
            Punct("="),
            Str(resource),
            Punct("}"),
        ] if key == "resource" && (d == "import" || d == "export") => {
            let (relation, format, resource) = (relation.clone(), format.clone(), resource.clone());
            Ok(if d == "import" {
                Statement::Import {
                    relation,
                    format,
                    resource,
                }
            } else {
                Statement::Export { relation, resource }
            })
        }
        [Directive(d), ..] => Err(format!("@{d}: not a directive the translations read")),
        _ => {
            let (head, body) = match tokens.iter().position(|t| *t == Punct(":-")) {
                Some(i) => (&tokens[..i], &tokens[i + 1..]),
                None => (tokens, &[][..]),
            };
            let head = split_commas(head)
                .into_iter()
                .map(|a| match parse_literal(a)? {
                    Literal::Atom(a) => Ok(a),
                    Literal::NotEqual(..) => Err("a comparison in a rule's head".to_string()),
                })
                .collect::<Result<_, _>>()?;
            let body = if body.is_empty() {
                Vec::new()
            } else {
                split_commas(body)
                    .into_iter()
                    .map(parse_literal)
                    .collect::<Result<_, _>>()?
            };
            Ok(Statement::Rule(Rule { head, body }))
        }
    }
}

/// The parts of a list separated by commas outside parentheses.
fn split_commas(tokens: &[Token]) -> Vec<&[Token]> {
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0, 0);
    for (i, t) in tokens.iter().enumerate() {
        match t {
            Token::Punct("(") => depth += 1,
            Token::Punct(")") => depth -= 1,
            Token::Punct(",") if depth == 0 => {
                parts.push(&tokens[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&tokens[start..]);
    parts
}

fn parse_literal(tokens: &[Token]) -> Result<Literal, String> {
    match tokens {
        [x, Token::Punct("!="), y] => Ok(Literal::NotEqual(parse_term(x)?, parse_term(y)?)),
        [Token::Name(relation), Token::Punct("("), terms @ .., Token::Punct(")")] if !relation.contains(':') => {
            let terms = split_commas(terms)
                .into_iter()
                .map(|t| match t {
                    [t] => parse_term(t),
                    _ => Err(format!("{relation}: an argument is not one term: {t:?}")),
                })
                .collect::<Result<_, _>>()?;
            Ok(Literal::Atom(Atom {
                relation: relation.clone(),
                terms,
            }))
        }
        _ => Err(format!("not an atom or a comparison: {tokens:?}")),
    }
}

fn parse_term(token: &Token) -> Result<Term, String> {
    match token {
        Token::Var(v) => Ok(Term::Var(v.clone())),
        Token::Iri(c) => Ok(Term::Const(c.clone())),
        Token::Name(c) if c.contains(':') && !c.ends_with(':') => Ok(Term::Const(c.clone())),
        t => Err(format!("not a term: {t:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_statements_and_their_comments() {
        let text = "% head\n\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                    @import input :- ntriples{resource=\"input.nt\"} .\n\
                    T(?y, owl:sameAs, ?x) :- T(?x, owl:sameAs, ?y),\n    T(?y, ?p, ?z), ?z != owl:Nothing .  % eq-sym\n\
                    T(owl:Thing, <http://e/a%20b>, owl:Class) .\n";
        let items = parse(text).unwrap();
        assert!(matches!(&items[0], Item::Comment(c) if c == " head"));
        assert!(matches!(items[1], Item::Blank));
        assert!(
            matches!(&items[2], Item::Statement(Statement::Prefix { name, iri }, None)
            if name == "owl" && iri == "http://www.w3.org/2002/07/owl#")
        );
        assert!(
            matches!(&items[3], Item::Statement(Statement::Import { relation, format, resource }, None)
            if relation == "input" && format == "ntriples" && resource == "input.nt")
        );
        let Item::Statement(Statement::Rule(rule), Some(trailing)) = &items[4] else {
            panic!("a rule with its comment")
        };
        assert_eq!(trailing, " eq-sym");
        assert_eq!((rule.head.len(), rule.body.len()), (1, 3));
        assert!(
            matches!(&rule.body[2], Literal::NotEqual(Term::Var(z), Term::Const(c)) if z == "z" && c == "owl:Nothing")
        );
        let Item::Statement(Statement::Rule(fact), None) = &items[5] else {
            panic!("a fact")
        };
        assert!(fact.body.is_empty());
        assert!(matches!(&fact.head[0].terms[1], Term::Const(c) if c == "<http://e/a%20b>"));
    }

    #[test]
    fn refuses_what_it_cannot_translate() {
        assert!(parse("T(?x) :- \n% inside\nU(?x) .\n").is_err());
        assert!(parse("T(?x) :- U(?x) . U(?x) :- V(?x) .\n").is_err());
        assert!(parse("T(?x) :- U(?x)\n").is_err());
        assert!(parse("@parameter $x = 1 .\n").is_err());
        assert!(parse("T(?x) :- U(?x), ?x > 1 .\n").is_err());
    }

    #[test]
    fn souffle_names_singletons_and_spells_constants() {
        let mut prefixes = BTreeMap::new();
        prefixes.insert("owl".to_string(), "http://www.w3.org/2002/07/owl#".to_string());
        let Item::Statement(Statement::Rule(rule), _) =
            parse("U(?p) :- input(?p, owl:inverseOf, ?q), ?p != owl:Thing .\n")
                .unwrap()
                .remove(0)
        else {
            panic!("a rule")
        };
        assert_eq!(
            souffle_rule(&rule, &prefixes),
            "U(p) :- input_(p, \"<http://www.w3.org/2002/07/owl#inverseOf>\", _), p != \"<http://www.w3.org/2002/07/owl#Thing>\"."
        );
        let mut distinct = Distinct::default();
        assert_eq!(
            vlog_rule(&rule, &mut distinct),
            "U(?p) :- input(?p, owl:inverseOf, ?q), ~sameTerm(?p, owl:Thing) ."
        );
        assert_eq!(distinct.constants, ["owl:Thing"]);
    }

    /// `?x != ?y` translates for each engine: as it is for Soufflé, and
    /// through a relation the binding atoms fill for VLog.
    #[test]
    fn two_variables_compared() {
        let Item::Statement(Statement::Rule(rule), _) =
            parse("D(?x, ?y) :- T(?x, ?p, ?y), V(?x, ?a), V(?y, ?b), ?a != ?b .\n")
                .unwrap()
                .remove(0)
        else {
            panic!("a rule")
        };
        assert_eq!(
            souffle_rule(&rule, &BTreeMap::new()),
            "D(x, y) :- T(x, _, y), V(x, a), V(y, b), a != b."
        );
        let mut distinct = Distinct::default();
        assert_eq!(
            vlog_rule(&rule, &mut distinct),
            "D(?x, ?y) :- T(?x, ?p, ?y), V(?x, ?a), V(?y, ?b), ~sameBound(?a, ?b) ."
        );
        assert_eq!(
            distinct.bound,
            ["sameBound(?a, ?a) :- V(?x, ?a) .", "sameBound(?b, ?b) :- V(?y, ?b) ."]
        );
    }

    #[test]
    fn the_core_comes_first_then_the_switches_in_order() {
        let names: Vec<&str> = groups(Switches::complete()).into_iter().map(|(n, _)| n).collect();
        assert_eq!(
            names,
            [
                "core",
                "equality",
                "property-chains",
                "checks",
                "identity",
                "schema",
                "trivial",
                "literal-values"
            ]
        );
        let none = Switch::ALL.into_iter().fold(Switches::complete(), Switches::off);
        assert_eq!(groups(none).len(), 1);
    }
}
