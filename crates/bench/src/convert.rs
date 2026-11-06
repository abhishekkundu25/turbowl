//! A scenario's inputs as the Datalog competitors read them, converted once
//! and cached under `target/bench-input/`, and the closure they write back,
//! counted.
//!
//! The files are parsed with Oxigraph's parsers, each as this reasoner reads
//! it (the format from its extension, `.n3` as Turtle, no base IRI), with
//! blank nodes scoped per file. Every literal and blank node is then renamed
//! to an IRI by the oracle's [`Codec`], as the oracle feeds Nemo: the engines
//! match terms as the W3C rules and this reasoner do, never by value (Nemo
//! would read `"01"^^xsd:integer` as 1), and the literals the rules name get
//! their fixed IRIs. The result is one directory per list of input files:
//!
//! - `input.nt`: the triples, encoded, as N-Triples (Nemo, VLog);
//! - `input.tsv`: the same triples, one per line, the three terms in
//!   N-Triples form separated by tabs (Soufflé);
//! - `literal-types.nt` and `literal-values.nt`, with their `.tsv` forms:
//!   the facts the value-based rules read (dt-type2, dt-eq, dt-diff), as the
//!   oracle gives them to Nemo (`Codec::literal_facts`);
//! - `literals.bin`: a bit per number the codec gave, set where that number
//!   is a literal, so that a closure can be read back without the codec.
//!
//! An encoded closure is counted without decoding it: the codec is a
//! bijection, so distinct encoded triples are distinct triples. Decoding
//! matters only to drop what RDF cannot write (a literal subject, a
//! predicate that is not an IRI), as this reasoner's reads do, and the bits
//! say which encoded terms are literals.

use oxrdf::{BlankNode, NamedOrBlankNode, Term, Triple};
use std::borrow::Cow;
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
use std::io::{BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use turbowl::Format;
use turbowl_oracle::{CODEC_PREFIX, Codec, NAMED_LITERALS};

/// The conversion's version, in its cache key: a change to what it writes
/// changes the version, so old conversions are never read.
const VERSION: u32 = 2;

/// A converted input: the directory holding `input.nt`, `input.tsv` and
/// `literals.bin`.
#[derive(Debug, Clone)]
pub(crate) struct Encoded {
    dir: PathBuf,
}

impl Encoded {
    /// The directory the files are in (Nemo's import directory, Soufflé's
    /// fact directory).
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    /// The N-Triples file.
    pub(crate) fn ntriples(&self) -> PathBuf {
        self.dir.join("input.nt")
    }

    /// Counts a closure an engine wrote: each line is one row, which `split`
    /// turns into its three terms (`None` for a line that is not a row).
    /// Returns the rows RDF can write and how many of them are distinct.
    /// Rows with a literal subject or a predicate that is not an IRI are
    /// generalized triples and are left out of both.
    pub(crate) fn closure(
        &self,
        path: &Path,
        split: impl Fn(&str) -> Option<[Cow<'_, str>; 3]>,
    ) -> Result<(usize, usize), String> {
        let literals =
            std::fs::read(self.dir.join("literals.bin")).map_err(|e| format!("{}: {e}", self.dir.display()))?;
        let err = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
        let file = std::fs::File::open(path).map_err(|e| err(&e))?;
        let hasher = BuildHasherDefault::<DefaultHasher>::default();
        let mut hashes = Vec::new();
        for (n, line) in std::io::BufReader::with_capacity(1 << 20, file).lines().enumerate() {
            let line = line.map_err(|e| err(&e))?;
            if line.trim().is_empty() {
                continue;
            }
            let [s, p, o] = split(&line).ok_or_else(|| err(&format!("line {}: not three terms: {line:?}", n + 1)))?;
            for t in [&s, &p, &o] {
                if !(t.starts_with('<') && t.ends_with('>')) {
                    return Err(err(&format!("line {}: {t} is not an encoded term", n + 1)));
                }
            }
            if is_literal(&s, &literals) || p[1..].starts_with(CODEC_PREFIX) {
                continue;
            }
            hashes.push(hasher.hash_one((&*s, &*p, &*o)));
        }
        let rows = hashes.len();
        Ok((rows, crate::engine::distinct(hashes)))
    }
}

/// Whether an encoded term (`<...>`) stands for a literal.
fn is_literal(term: &str, literals: &[u8]) -> bool {
    let iri = &term[1..term.len() - 1];
    let Some(rest) = iri.strip_prefix(CODEC_PREFIX) else {
        return false;
    };
    match rest.parse::<usize>() {
        Ok(n) => literals.get(n / 8).is_some_and(|b| b & (1 << (n % 8)) != 0),
        Err(_) => NAMED_LITERALS.iter().any(|(_, _, named)| *named == iri),
    }
}

/// The converted form of these files, in this order: from the cache when it
/// is there, else converted now. The cache key is the files' paths, sizes
/// and modification times; a conversion is written under a temporary name
/// and renamed whole, so a crash leaves nothing that is read.
pub(crate) fn encoded(files: &[PathBuf]) -> Result<Encoded, String> {
    let mut sources = format!("version {VERSION}\n");
    for f in files {
        let path = f.canonicalize().map_err(|e| format!("{}: {e}", f.display()))?;
        let meta = std::fs::metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos());
        sources.push_str(&format!("{} {} {modified}\n", path.display(), meta.len()));
    }
    let key = BuildHasherDefault::<DefaultHasher>::default().hash_one(&sources);
    let root = crate::repo_root().join("target/bench-input");
    let dir = root.join(format!("{key:016x}"));
    let done = Encoded { dir: dir.clone() };
    if std::fs::read_to_string(dir.join("sources.txt")).is_ok_and(|s| s == sources) {
        return Ok(done);
    }
    let partial = root.join(format!("{key:016x}.partial"));
    let io = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    let _ = std::fs::remove_dir_all(&partial);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&partial).map_err(|e| io(&partial, e))?;
    let start = std::time::Instant::now();
    let triples = convert(files, &partial)?;
    std::fs::write(partial.join("sources.txt"), &sources).map_err(|e| io(&partial, e))?;
    std::fs::rename(&partial, &dir).map_err(|e| io(&dir, e))?;
    eprintln!(
        "converted {} files ({triples} triples) into {} in {:.1} s",
        files.len(),
        dir.display(),
        start.elapsed().as_secs_f64()
    );
    Ok(done)
}

/// Writes the converted files into `dir`; returns the number of triples.
fn convert(files: &[PathBuf], dir: &Path) -> Result<u64, String> {
    let io = |p: &Path, e: &dyn std::fmt::Display| format!("{}: {e}", p.display());
    let create = |name: &str| {
        let path = dir.join(name);
        std::fs::File::create(&path)
            .map(|f| BufWriter::with_capacity(1 << 20, f))
            .map_err(|e| io(&path, &e))
    };
    let (mut nt, mut tsv) = (create("input.nt")?, create("input.tsv")?);
    let mut codec = Codec::new(true);
    let mut literals: Vec<u8> = Vec::new();
    let mut count = 0u64;
    for (n, path) in files.iter().enumerate() {
        let mut write = |t: Triple| -> std::io::Result<()> {
            let subject = match t.subject {
                NamedOrBlankNode::BlankNode(b) => scoped(n, &b).into(),
                s => s,
            };
            let object = match t.object {
                Term::BlankNode(b) => scoped(n, &b).into(),
                o => o,
            };
            let t = Triple::new(subject, t.predicate, object);
            let literal = t.object.is_literal();
            let t = codec.encode_triple(&t);
            if literal {
                mark_literal(&t.object, &mut literals);
            }
            writeln!(nt, "{} {} {} .", t.subject, t.predicate, t.object)?;
            writeln!(tsv, "{}\t{}\t{}", t.subject, t.predicate, t.object)?;
            count += 1;
            Ok(())
        };
        parse(path, &mut write)?;
    }
    for (name, w) in [("input.nt", nt), ("input.tsv", tsv)] {
        w.into_inner().map_err(|e| io(&dir.join(name), &e))?;
    }
    let facts = codec.literal_facts();
    for (stem, triples) in [("literal-types", &facts.types), ("literal-values", &facts.values)] {
        let (mut nt, mut tsv) = (create(&format!("{stem}.nt"))?, create(&format!("{stem}.tsv"))?);
        for t in triples {
            writeln!(nt, "{} {} {} .", t.subject, t.predicate, t.object).map_err(|e| io(&dir.join(stem), &e))?;
            writeln!(tsv, "{}\t{}\t{}", t.subject, t.predicate, t.object).map_err(|e| io(&dir.join(stem), &e))?;
        }
        for w in [nt, tsv] {
            w.into_inner().map_err(|e| io(&dir.join(stem), &e))?;
        }
    }
    std::fs::write(dir.join("literals.bin"), &literals).map_err(|e| io(dir, &e))?;
    Ok(count)
}

/// Sets the bit of a literal's encoded term.
fn mark_literal(term: &Term, literals: &mut Vec<u8>) {
    let Term::NamedNode(iri) = term else { return };
    // A literal the rules name has a fixed IRI and no number.
    let Some(n) = iri
        .as_str()
        .strip_prefix(CODEC_PREFIX)
        .and_then(|r| r.parse::<usize>().ok())
    else {
        return;
    };
    if literals.len() <= n / 8 {
        literals.resize(n / 8 + 1, 0);
    }
    literals[n / 8] |= 1 << (n % 8);
}

/// A blank node of file `n`, labelled apart from every other file's (this
/// reasoner scopes blank nodes per document).
fn scoped(n: usize, b: &BlankNode) -> BlankNode {
    BlankNode::new_unchecked(format!("d{n}_{}", b.as_str()))
}

/// Parses a file as this reasoner's `load_file` does, giving each triple to `sink`.
pub(crate) fn parse(path: &Path, sink: &mut dyn FnMut(Triple) -> std::io::Result<()>) -> Result<(), String> {
    let err = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
    let format = Format::from_extension(ext).ok_or_else(|| err(&"unknown format"))?;
    let file = std::fs::File::open(path).map_err(|e| err(&e))?;
    let reader = std::io::BufReader::with_capacity(1 << 20, file);
    let triples: Box<dyn Iterator<Item = Result<Triple, String>>> = match format {
        Format::NTriples => Box::new(
            oxttl::NTriplesParser::new()
                .for_reader(reader)
                .map(|t| t.map_err(|e| e.to_string())),
        ),
        // The `.n3` models under data/ are Turtle (the oracle reads them so too).
        Format::Turtle | Format::N3 => Box::new(
            oxttl::TurtleParser::new()
                .for_reader(reader)
                .map(|t| t.map_err(|e| e.to_string())),
        ),
        Format::RdfXml => Box::new(
            oxrdfxml::RdfXmlParser::new()
                .for_reader(reader)
                .map(|t| t.map_err(|e| e.to_string())),
        ),
    };
    for t in triples {
        sink(t.map_err(|e| err(&e))?).map_err(|e| err(&e))?;
    }
    Ok(())
}

/// The three terms of an N-Triples line of IRIs (Nemo's closure).
pub(crate) fn ntriples_terms(line: &str) -> Option<[Cow<'_, str>; 3]> {
    let body = line.trim_end().strip_suffix('.')?.trim_end();
    let mut it = body.splitn(3, ' ');
    Some([it.next()?.into(), it.next()?.into(), it.next()?.trim().into()])
}

/// The three terms of a tab-separated line (Soufflé's closure).
pub(crate) fn tsv_terms(line: &str) -> Option<[Cow<'_, str>; 3]> {
    let mut it = line.split('\t');
    let terms = [it.next()?.into(), it.next()?.into(), it.next()?.into()];
    it.next().is_none().then_some(terms)
}

/// The three fields of a CSV line, RFC 4180 quoting (VLog's closure).
pub(crate) fn csv_terms(line: &str) -> Option<[Cow<'_, str>; 3]> {
    if !line.contains('"') {
        let mut it = line.split(',');
        let terms = [it.next()?.into(), it.next()?.into(), it.next()?.into()];
        return it.next().is_none().then_some(terms);
    }
    let mut fields = vec![String::new()];
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                fields.last_mut()?.push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => fields.push(String::new()),
            c => fields.last_mut()?.push(c),
        }
    }
    let [s, p, o]: [String; 3] = fields.try_into().ok()?;
    Some([s.into(), p.into(), o.into()])
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The fixture's two documents: a literal and a blank node in the first,
    /// the same label in the second (another node).
    const FIRST: &str = "<http://example.org/a> <http://example.org/p> \"lit\" .\n\
                         <http://example.org/a> <http://example.org/q> _:b .\n";
    const SECOND: &str = "_:b <http://example.org/q> <http://example.org/a> .\n";

    /// A closure an engine could write for the fixture: two RDF triples, one
    /// with a literal subject (the literal is term 0) and one whose predicate
    /// is a blank node (term 1).
    pub(crate) const CLOSURE: [[&str; 3]; 4] = [
        [
            "<http://example.org/a>",
            "<http://example.org/p>",
            "<urn:x-turbowl-oracle:term:0>",
        ],
        [
            "<urn:x-turbowl-oracle:term:1>",
            "<http://example.org/p>",
            "<http://example.org/a>",
        ],
        [
            "<urn:x-turbowl-oracle:term:0>",
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>",
            "<http://example.org/C>",
        ],
        [
            "<http://example.org/a>",
            "<urn:x-turbowl-oracle:term:1>",
            "<http://example.org/a>",
        ],
    ];

    /// The fixture converted, and an empty directory to run an engine in,
    /// under `target/bench-test/<name>-<pid>/`.
    pub(crate) fn fixture(name: &str) -> (Encoded, PathBuf) {
        let dir = crate::repo_root().join(format!("target/bench-test/{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("run")).unwrap();
        std::fs::write(dir.join("first.ttl"), FIRST).unwrap();
        std::fs::write(dir.join("second.nt"), SECOND).unwrap();
        let encoded = encoded(&[dir.join("first.ttl"), dir.join("second.nt")]).unwrap();
        (encoded, dir.join("run"))
    }

    /// Removes what [`fixture`] made.
    pub(crate) fn remove(input: &Encoded, run: &Path) {
        let _ = std::fs::remove_dir_all(input.dir());
        let _ = std::fs::remove_dir_all(run.parent().unwrap());
    }

    /// A shell command, standing in for an engine.
    pub(crate) fn sh(script: &str) -> std::process::Command {
        let mut cmd = std::process::Command::new("sh");
        cmd.args(["-c", script]);
        cmd
    }

    /// Literals and blank nodes become IRIs, blank nodes apart per file, and
    /// the literal is marked; converting again reads the cache.
    #[test]
    fn encodes_once_and_marks_the_literals() {
        let (input, run) = fixture("convert");
        let nt = std::fs::read_to_string(input.ntriples()).unwrap();
        let term = |n: usize| format!("<{CODEC_PREFIX}{n}>");
        assert_eq!(
            nt,
            format!(
                "<http://example.org/a> <http://example.org/p> {} .\n\
                 <http://example.org/a> <http://example.org/q> {} .\n\
                 {} <http://example.org/q> <http://example.org/a> .\n",
                term(0),
                term(1),
                term(2)
            )
        );
        let tsv = std::fs::read_to_string(input.dir().join("input.tsv")).unwrap();
        assert_eq!(
            tsv.lines().next().unwrap(),
            format!("<http://example.org/a>\t<http://example.org/p>\t{}", term(0))
        );
        let literals = std::fs::read(input.dir().join("literals.bin")).unwrap();
        assert_eq!(literals, [0b001]);
        assert!(is_literal(&term(0), &literals) && !is_literal(&term(1), &literals));
        let named = format!("<{}>", NAMED_LITERALS[0].2);
        assert!(is_literal(&named, &literals) && !is_literal("<http://example.org/a>", &literals));

        let dir = run.parent().unwrap();
        let again = encoded(&[dir.join("first.ttl"), dir.join("second.nt")]).unwrap();
        assert_eq!(again.dir(), input.dir());
        remove(&input, &run);
    }

    /// The closure counts the RDF triples, drops the generalized ones, and
    /// counts a repeated row once as distinct; a line that is not a row is
    /// an error.
    #[test]
    fn counts_a_closure() {
        let (input, run) = fixture("closure");
        let mut lines: Vec<String> = CLOSURE.iter().map(|t| t.join("\t")).collect();
        lines.push(lines[0].clone());
        std::fs::write(run.join("closure.tsv"), lines.join("\n") + "\n").unwrap();
        assert_eq!(input.closure(&run.join("closure.tsv"), tsv_terms), Ok((3, 2)));
        std::fs::write(
            run.join("bad.tsv"),
            "<http://example.org/a>\t\"x\"\t<http://example.org/b>\n",
        )
        .unwrap();
        assert!(
            input
                .closure(&run.join("bad.tsv"), tsv_terms)
                .unwrap_err()
                .contains("not an encoded term")
        );
        std::fs::write(run.join("short.tsv"), "<http://example.org/a>\n").unwrap();
        assert!(
            input
                .closure(&run.join("short.tsv"), tsv_terms)
                .unwrap_err()
                .contains("not three terms")
        );
        remove(&input, &run);
    }

    #[test]
    fn splits_each_engines_rows() {
        let row = |t: Option<[Cow<'_, str>; 3]>| t.map(|t| t.map(|s| s.into_owned()));
        let terms = Some(["<a>".to_string(), "<b>".into(), "<c,d>".into()]);
        assert_eq!(row(ntriples_terms("<a> <b> <c,d> .")), terms);
        assert_eq!(row(tsv_terms("<a>\t<b>\t<c,d>")), terms);
        assert_eq!(row(csv_terms("<a>,<b>,\"<c,d>\"")), terms);
        assert_eq!(
            row(csv_terms("<a>,<b>,\"<c\"\"d>\"")),
            Some(["<a>".to_string(), "<b>".into(), "<c\"d>".into()])
        );
        assert_eq!(row(csv_terms("<a>,<b>")), None);
        assert_eq!(row(tsv_terms("<a>\t<b>\t<c>\t<d>")), None);
        assert_eq!(row(ntriples_terms("<a> <b>")), None);
    }
}
