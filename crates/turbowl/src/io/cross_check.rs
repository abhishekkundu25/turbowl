//! The parity contract of TurbOWL's N-Triples and Turtle parsers: each test
//! document is parsed twice, by `io::parse` and by Oxigraph's `oxttl` with the
//! interning the reasoner used before (`Load::triple`), and the two must give
//!
//! - the same triples, as term ids, in the same order (also those handed over
//!   before an error);
//! - the same dictionary: the same term at every id, except that anonymous
//!   blank nodes (`[]`, list cells) have random labels in both;
//! - the same error: line, column and message.
//!
//! And the parse in pieces (`io/parallel.rs`) must give exactly what the parse
//! on one thread gives: each document is also parsed cut in about 32 pieces
//! (every line a piece, for the small ones), on four threads or on one, and
//! compared with the same three rules.
//!
//! The documents: every `.ttl` and `.nt` file of the repository and of the
//! fetched data (`data/`, `target/bench-data/`, up to [`MAX_FILE`] bytes, or
//! as many as `IO_CROSS_CHECK_MAX_BYTES` says),
//! every string literal of the Rust and Python sources (the suites' inline
//! documents, and much that is not RDF), each read as both formats and with
//! and without a base; then generated documents and mutations of them.

use super::*;
use oxttl::{NTriplesParser, TurtleParser};
use std::path::PathBuf;

/// Files larger than this are left to the benchmarks, unless
/// `IO_CROSS_CHECK_MAX_BYTES` raises the limit.
const MAX_FILE: u64 = 20 << 20;

const BASE: &str = "http://example.org/base/dir/file.ttl?q#f";

#[derive(Debug)]
struct Parsed {
    triples: Vec<[TermId; 3]>,
    dict: Dictionary,
    error: Option<(u64, u64, String)>,
}

fn ours(text: &[u8], format: Format, base: Option<&str>) -> Parsed {
    on(&Workers::one(), text, format, base)
}

/// The parse on `workers`.
fn on(workers: &Workers, text: &[u8], format: Format, base: Option<&str>) -> Parsed {
    let mut dict = Dictionary::new();
    let mut triples = Vec::new();
    let result = parse_on(workers, &mut dict, text, format, base, None, |t| triples.push(t));
    let error = match result {
        Ok(loaded) => {
            assert_eq!(loaded.triples, triples.len());
            None
        }
        Err(Error::Parse {
            line, column, message, ..
        }) => Some((line, column, message)),
        Err(e) => panic!("not a parse error: {e:?}"),
    };
    Parsed { triples, dict, error }
}

/// The parse before TurbOWL's own parsers: `oxttl`, interned by `Load::triple`.
fn oxigraph(text: &[u8], format: Format, base: Option<&str>) -> Parsed {
    let mut dict = Dictionary::new();
    let mut triples = Vec::new();
    let document = dict.new_document();
    let mut load = Load {
        dict: &mut dict,
        document,
        sink: |t| triples.push(t),
        triples: 0,
        path: None,
    };
    let syntax = |e: TurtleSyntaxError| {
        let at = e.location().start;
        Some((at.line + 1, at.column + 1, e.message().to_owned()))
    };
    let mut error = None;
    match format {
        Format::NTriples => {
            for t in NTriplesParser::new().for_slice(text) {
                match t {
                    Ok(t) => load.triple(t.as_ref()).unwrap(),
                    Err(e) => {
                        error = syntax(e);
                        break;
                    }
                }
            }
        }
        _ => {
            let mut parser = TurtleParser::new();
            if let Some(base) = base {
                parser = parser.with_base_iri(base).unwrap();
            }
            for t in parser.for_slice(text) {
                match t {
                    Ok(t) => load.triple(t.as_ref()).unwrap(),
                    Err(e) => {
                        error = syntax(e);
                        break;
                    }
                }
            }
        }
    }
    Parsed { triples, dict, error }
}

/// The label Oxigraph gives an anonymous blank node: a random 128-bit number
/// in lowercase hex, the first digit a letter.
fn anonymous(label: &str) -> bool {
    label.len() <= 32
        && label.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        && matches!(label.as_bytes()[0], b'a'..=b'f')
}

/// Compares the two parses of one document; the error says how they differ.
fn compare(text: &[u8], format: Format, base: Option<&str>) -> Result<(), String> {
    let a = ours(text, format, base);
    same(&a, &oxigraph(text, format, base), "oxttl")?;
    let pieces = if text.len().is_multiple_of(2) {
        threads()
    } else {
        Workers::one().every_round()
    };
    same(&on(&pieces, text, format, base), &a, "one thread")
}

/// Four threads, every document cut in pieces.
fn threads() -> Workers {
    static WORKERS: std::sync::OnceLock<Workers> = std::sync::OnceLock::new();
    WORKERS.get_or_init(|| Workers::exactly(4).every_round()).clone()
}

/// Compares two parses of one document, `a` ours, `b` the `other` parse.
fn same(a: &Parsed, b: &Parsed, other: &str) -> Result<(), String> {
    if a.error != b.error {
        return Err(format!("error: ours {:?}, {other} {:?}", a.error, b.error));
    }
    if a.triples != b.triples {
        let at = a.triples.iter().zip(&b.triples).position(|(x, y)| x != y);
        return Err(format!(
            "triples: ours {}, {other} {}, first difference at {at:?}",
            a.triples.len(),
            b.triples.len()
        ));
    }
    if a.dict.len() != b.dict.len() {
        return Err(format!("terms: ours {}, {other} {}", a.dict.len(), b.dict.len()));
    }
    for id in (0..a.dict.len()).map(|i| TermId(i as u32)) {
        let (x, y) = (a.dict.term(id), b.dict.term(id));
        let same = match (x, y) {
            (Term::Blank { doc: d, label: l }, Term::Blank { doc: e, label: m }) => {
                d == e && (l == m || (anonymous(l) && anonymous(m)))
            }
            _ => x == y,
        };
        if !same {
            return Err(format!("term {}: ours {x:?}, {other} {y:?}", id.0));
        }
    }
    Ok(())
}

/// Every way the corpus reads a document: its formats, with and without a base.
fn check(name: &str, text: &[u8], formats: &[Format], failures: &mut Vec<String>) -> usize {
    let mut cases = 0;
    for &format in formats {
        for base in [None, Some(BASE)] {
            if format == Format::NTriples && base.is_some() {
                continue;
            }
            cases += 1;
            if let Err(e) = compare(text, format, base) {
                failures.push(format!("{name} as {format:?} (base {base:?}): {e}"));
            }
        }
    }
    cases
}

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == "target" || name == ".git" || name == ".venv" || name == "node_modules" {
            continue;
        }
        // Follows links: `data/` is one.
        match std::fs::metadata(&path) {
            Ok(m) if m.is_dir() => files(&path, out),
            Ok(m) if m.is_file() => out.push(path),
            _ => {}
        }
    }
}

/// The contents of the string literals of a Rust or Python source: raw
/// strings, plain strings with their escapes decoded, triple-quoted strings.
fn literals(source: &str, out: &mut Vec<String>) {
    let b = source.as_bytes();
    let mut i = 0;
    while i < b.len() {
        // Rust raw strings: r"..." and r#"..."#.
        if b[i] == b'r' && matches!(b.get(i + 1), Some(b'"' | b'#')) {
            let hashes = b[i + 1..].iter().take_while(|&&c| c == b'#').count();
            if b.get(i + 1 + hashes) == Some(&b'"') {
                let start = i + 2 + hashes;
                let close = format!("\"{}", "#".repeat(hashes));
                if let Some(end) = source[start..].find(&close) {
                    out.push(source[start..start + end].to_owned());
                    i = start + end + close.len();
                    continue;
                }
            }
        }
        // Python triple-quoted strings.
        if source[i..].starts_with("\"\"\"") || source[i..].starts_with("'''") {
            let quote = &source[i..i + 3];
            if let Some(end) = source[i + 3..].find(quote) {
                out.push(source[i + 3..i + 3 + end].to_owned());
                i += 6 + end;
                continue;
            }
        }
        if b[i] == b'"' {
            let mut s = String::new();
            let mut rest = &source[i + 1..];
            while let Some(c) = rest.chars().next() {
                rest = &rest[c.len_utf8()..];
                match c {
                    '"' => break,
                    '\\' => {
                        let Some(e) = rest.chars().next() else { break };
                        rest = &rest[e.len_utf8()..];
                        match e {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            'r' => s.push('\r'),
                            '0' => s.push('\0'),
                            // A Rust line continuation skips the next line's indentation.
                            '\n' => rest = rest.trim_start(),
                            other => s.push(other),
                        }
                    }
                    c => s.push(c),
                }
            }
            out.push(s);
            i = source.len() - rest.len();
            continue;
        }
        i += source[i..].chars().next().map_or(1, char::len_utf8);
    }
}

/// Every document of the repository and of the fetched data, and every
/// string literal of the sources, parse the same with both.
#[test]
fn every_test_document_parses_as_oxttl_does() {
    let root = workspace();
    let mut paths = Vec::new();
    for dir in ["crates", "data", "target/bench-data", "oracles", "scripts", "docs"] {
        files(&root.join(dir), &mut paths);
    }
    paths.sort();
    let max_file = std::env::var("IO_CROSS_CHECK_MAX_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(MAX_FILE);
    let (mut documents, mut cases, mut skipped) = (0, 0, Vec::new());
    let mut failures = Vec::new();
    let mut strings = Vec::new();
    for path in &paths {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let format = match ext {
            "ttl" => Format::Turtle,
            "nt" => Format::NTriples,
            "rs" | "py" => {
                if let Ok(source) = std::fs::read_to_string(path) {
                    literals(&source, &mut strings);
                }
                continue;
            }
            _ => continue,
        };
        let size = std::fs::metadata(path).map_or(0, |m| m.len());
        if size > max_file {
            skipped.push(path.strip_prefix(&root).unwrap_or(path).display().to_string());
            continue;
        }
        let text = std::fs::read(path).unwrap();
        let name = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        // A small file is also read as the other format: errors must agree too.
        let formats: &[Format] = if size < 1 << 20 {
            &[Format::Turtle, Format::NTriples]
        } else {
            std::slice::from_ref(&format)
        };
        documents += 1;
        cases += check(&name, &text, formats, &mut failures);
    }
    strings.sort();
    strings.dedup();
    for (i, s) in strings.iter().enumerate() {
        cases += check(
            &format!("string {i} {:?}", s.chars().take(60).collect::<String>()),
            s.as_bytes(),
            &[Format::Turtle, Format::NTriples],
            &mut failures,
        );
    }
    println!(
        "every_test_document_parses_as_oxttl_does: {documents} files, {} strings, {cases} parses compared; \
         left out (over {max_file} bytes): {skipped:?}",
        strings.len()
    );
    assert!(documents >= 4, "the fixtures at least");
    assert!(
        failures.is_empty(),
        "{} of {cases} differ:\n{}",
        failures.len(),
        failures.iter().take(30).cloned().collect::<Vec<_>>().join("\n")
    );
}

/// A deterministic generator (xorshift64*).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }

    /// Mostly one of `valid`, one time in 150 one of `invalid`.
    fn mostly<'a>(&mut self, valid: &[&'a str], invalid: &[&'a str]) -> &'a str {
        if self.below(150) == 0 {
            self.pick(invalid)
        } else {
            self.pick(valid)
        }
    }
}

fn space(r: &mut Rng, out: &mut String) {
    out.push_str(r.mostly(
        &[
            " ",
            " ",
            " ",
            "\n",
            "  ",
            "\t",
            "\r\n",
            "\r",
            " # c\n",
            "\n# comment é\n",
        ],
        &[""],
    ));
}

fn iri(r: &mut Rng, out: &mut String) {
    out.push_str(r.mostly(
        &[
            "<http://example.org/a>",
            "<http://example.org/b#c>",
            "<http://example.org/s/../t/./u>",
            "<http://example.org/\\u00e9t\\U0001F600>",
            "<http://example.org/%41?q=1>",
            "<urn:x:y>",
            "<http://[::1]:80/p>",
            "<http://example.org/é>",
        ],
        &[
            "<a>",
            "<../up>",
            "<#frag>",
            "<>",
            "<//example.org/x>",
            "<http://ex ample.org/>",
            "<http://example.org/\\u0020>",
        ],
    ));
}

fn pname(r: &mut Rng, out: &mut String) {
    out.push_str(r.mostly(
        &[
            "ex:a", "ex:b", ":c", "ex:", "ex:a.b", "ex:a\\-b", "ex:%41b", "ex:1a", "ex::x", "ex:a\\/b", "ex:é", "x:y",
        ],
        &["e.x:a", "ex:a.", "nope:a", "ex:a\\b", "ex:%4"],
    ));
}

fn blank(r: &mut Rng, out: &mut String) {
    out.push_str(r.mostly(&["_:b", "_:b1", "_:x.y", "_:1", "_:é"], &["_:", "_:-a", "_:b."]));
}

fn literal(r: &mut Rng, out: &mut String) {
    out.push_str(r.mostly(
        &[
            "\"x\"",
            "\"\"",
            "\"a\\\"b\\n\\u00e9\"",
            "'single'",
            "\"\"\"long \" \"\" \n line\"\"\"",
            "'''long ' '' \r\n'''",
            "\"chat\"@fr",
            "\"chat\"@EN-gb",
            "\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>",
            "\"1\"^^ex:dt",
            "\"x\"^^xsd:string",
            "1",
            "-1.5",
            "+.5",
            "1e3",
            "1.E-2",
            "true",
            "false",
        ],
        &[
            "\"x\"@en-",
            "\"x\"@1",
            "\"x\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#langString>",
            ".e1",
            "\"bad \\q\"",
            "\"line\njump\"",
            "\"x\"@en--ltr",
        ],
    ));
}

fn object(r: &mut Rng, depth: usize, out: &mut String) {
    match r.below(if depth > 2 { 4 } else { 6 }) {
        0 => iri(r, out),
        1 => pname(r, out),
        2 => blank(r, out),
        3 => literal(r, out),
        4 => {
            out.push('[');
            if r.below(3) > 0 {
                space(r, out);
                predicate_objects(r, depth + 1, out);
            }
            space(r, out);
            out.push(']');
        }
        _ => {
            out.push('(');
            for _ in 0..r.below(4) {
                space(r, out);
                object(r, depth + 1, out);
            }
            space(r, out);
            out.push(')');
        }
    }
}

fn predicate_objects(r: &mut Rng, depth: usize, out: &mut String) {
    for i in 0..1 + r.below(3) {
        if i > 0 {
            space(r, out);
            out.push_str(r.mostly(&[";", ";", "; ;"], &[","]));
            space(r, out);
        }
        match r.below(4) {
            0 => out.push('a'),
            1 => pname(r, out),
            _ => iri(r, out),
        }
        out.push(' ');
        for j in 0..1 + r.below(3) {
            if j > 0 {
                space(r, out);
                out.push(',');
                space(r, out);
            }
            object(r, depth, out);
        }
    }
    if r.below(5) == 0 {
        out.push_str(" ;");
    }
    // Relative IRIs and anything else invalid come from the mutations too.
}

/// A Turtle document of a few statements: directives, triples, lists,
/// blank node property lists, every kind of literal; valid pieces mostly,
/// invalid ones sometimes.
fn turtle(r: &mut Rng) -> String {
    let mut out = String::new();
    if r.below(10) > 0 {
        out.push_str(
            "@prefix ex: <http://example.org/> .\n@prefix : <http://example.org/empty> .\n\
             PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n@prefix x: <http://example.org> .\n",
        );
    }
    for _ in 0..1 + r.below(5) {
        match r.below(8) {
            0 => out.push_str(r.mostly(
                &[
                    "@prefix ex: <http://example.org/> .",
                    "PREFIX ex: <http://example.org/ns#>",
                    "@base <http://example.org/b/c> .",
                    "BASE <http://example.org/d/>",
                    "base <http://example.org/e/f>",
                ],
                &[
                    "@prefix ex: <rel/> .",
                    "BASE <../d/>",
                    "@PREFIX ex: <http://example.org/> .",
                    "@prefix ex <http://example.org/> .",
                    "@base <a> ",
                ],
            )),
            1 => {
                out.push('[');
                space(r, &mut out);
                predicate_objects(r, 0, &mut out);
                out.push(']');
                if r.below(2) == 0 {
                    out.push(' ');
                    predicate_objects(r, 0, &mut out);
                }
                out.push_str(" .");
            }
            2 => {
                out.push('(');
                object(r, 1, &mut out);
                out.push_str(") ");
                predicate_objects(r, 0, &mut out);
                out.push_str(" .");
            }
            _ => {
                match r.below(3) {
                    0 => iri(r, &mut out),
                    1 => pname(r, &mut out),
                    _ => blank(r, &mut out),
                }
                out.push(' ');
                predicate_objects(r, 0, &mut out);
                space(r, &mut out);
                out.push('.');
            }
        }
        space(r, &mut out);
        out.push('\n');
    }
    out
}

/// An N-Triples document of a few lines, mostly valid.
fn ntriples(r: &mut Rng) -> String {
    let mut out = String::new();
    for _ in 0..1 + r.below(4) {
        if r.below(2) == 0 {
            iri(r, &mut out);
        } else {
            blank(r, &mut out);
        }
        out.push(' ');
        iri(r, &mut out);
        out.push(' ');
        match r.below(3) {
            0 => iri(r, &mut out),
            1 => blank(r, &mut out),
            _ => out.push_str(r.mostly(
                &[
                    "\"x\"",
                    "\"\"",
                    "\"a\\\"b\\n\\u00e9\"",
                    "\"chat\"@fr",
                    "\"chat\"@EN-gb",
                    "\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>",
                ],
                &["1", "'single'", "\"1\"^^ex:dt", "\"\"\"long\"\"\""],
            )),
        }
        out.push_str(r.mostly(&[" .", ".", " . # c", " .  "], &["", " . <http://example.org/x>"]));
        out.push_str(r.mostly(&["\n", "\r\n", "\n\n", "\r"], &[""]));
    }
    out
}

/// Breaks a document: deletes, inserts or replaces a byte or a few.
fn mutate(r: &mut Rng, text: &str) -> Vec<u8> {
    const BYTES: &[u8] = b"<>\"'.;,[]()@^_:#\\ \n\rax0eE+-%{}|~\xc3\xa9\xff\xe2\x82";
    let mut b = text.as_bytes().to_vec();
    for _ in 0..1 + r.below(3) {
        let at = r.below(b.len() + 1);
        match r.below(3) {
            0 if at < b.len() => {
                b.remove(at);
            }
            1 if at < b.len() => b[at] = BYTES[r.below(BYTES.len())],
            _ => b.insert(at, BYTES[r.below(BYTES.len())]),
        }
    }
    b
}

/// Generated documents, valid and broken, parse the same with both.
#[test]
fn generated_documents_parse_as_oxttl_does() {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let mut failures = Vec::new();
    let (mut cases, mut rejected, mut originals_rejected) = (0, 0, 0);
    for i in 0..30_000 {
        let (text, format) = if i % 3 == 0 {
            (ntriples(&mut r), Format::NTriples)
        } else {
            (turtle(&mut r), Format::Turtle)
        };
        let mut documents = vec![text.clone().into_bytes()];
        for _ in 0..3 {
            documents.push(mutate(&mut r, &text));
        }
        for (k, document) in documents.into_iter().enumerate() {
            if k == 0 && ours(&document, format, Some(BASE)).error.is_some() {
                originals_rejected += 1;
            }
            let base = (format == Format::Turtle && r.below(2) == 0).then_some(BASE);
            cases += 1;
            match compare(&document, format, base) {
                Ok(()) => rejected += usize::from(ours(&document, format, base).error.is_some()),
                Err(e) => failures.push(format!(
                    "{:?} as {format:?} (base {base:?}): {e}",
                    String::from_utf8_lossy(&document)
                )),
            }
        }
    }
    println!(
        "generated_documents_parse_as_oxttl_does: {cases} documents, {rejected} rejected, {originals_rejected} of the unbroken ones"
    );
    assert!(
        // Of the 30,000 unbroken documents, most are valid.
        originals_rejected < 15_000 && rejected > cases / 2,
        "{rejected} rejected of {cases}, {originals_rejected} unbroken"
    );
    assert!(
        failures.is_empty(),
        "{} of {cases} differ:\n{}",
        failures.len(),
        failures.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
}
