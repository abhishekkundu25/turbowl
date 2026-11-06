//! Scaling tests: each operation that should grow linearly with its input
//! (or not at all) is timed at sizes spanning at least 16x, and its growth
//! exponent is fitted on a log-log scale (`docs/specs/v1.md`, Testing
//! Decisions, Scaling). A hidden quadratic cost fails here instead of
//! showing up later in a benchmark. Where a structure's size can be counted
//! (dictionary bytes, stored tuples, a variable's tiers), it is checked
//! exactly instead.
//!
//! # The harness
//!
//! - [`check`]`(name, growth, sizes, once)`: `once(n)` measures the
//!   seconds one operation takes at size `n`, once. [`RUNS`] rounds each
//!   measure every size in turn, and each size keeps its fastest time, so a
//!   burst of load on a shared machine falls on all sizes alike instead of
//!   on a few neighbouring ones. The exponent is the least-squares slope of
//!   ln(time) against ln(size), and must be at most
//!   [`Growth::max_exponent`]: 1.25 for linear (n log n passes), 0.25 for
//!   constant. Over the bound, everything is measured again with
//!   [`RETRY_RUNS`] rounds; only a second failure panics, printing both
//!   tables (size, time, time / size, exponent).
//! - [`per_input`]`(setup, op)`: a `once` for `op(setup(n))`, with a fresh
//!   input per call, built outside the timing.
//! - [`per_call`]`(make, op)`: a `once` for `op(&mut state)` repeated in
//!   place on one state per size made by `make(n)`, for an operation that
//!   changes state and can repeat on it (one incremental insert into a
//!   store).
//! - [`linear`]: [`check`] over [`per_input`], with the linear bound.
//! - [`shared`]: a setup that builds its input once per size and hands out
//!   `Rc`s, for operations that only read their input.
//! - [`lock`]: every scaling test (and every `timing_*` test) holds it, so
//!   two never time each other.
//!
//! A measurement repeats the operation until it lasts at least [`TARGET`],
//! so a microsecond operation is not timed against the clock's resolution;
//! the first measurement at a size sets its repetitions and is not counted.
//! Inputs and results pass through `black_box`, and results are dropped
//! after the timing ends. The sizes keep the whole suite at a few seconds
//! in release.

use std::cell::RefCell;
use std::fmt;
use std::hint::black_box;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// Rounds over the sizes: the fastest measurement of each size counts.
pub(crate) const RUNS: usize = 7;
/// Rounds when the first fit is over its bound.
pub(crate) const RETRY_RUNS: usize = 21;
/// The shortest measurement: a faster operation is repeated until it lasts this long.
pub(crate) const TARGET: Duration = Duration::from_millis(2);
/// The most repetitions in one measurement.
const MAX_REPS: usize = 1 << 20;

/// How an operation may grow with its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Growth {
    /// At most linear; n log n passes.
    Linear,
    /// Not at all.
    Constant,
}

impl Growth {
    /// The largest fitted exponent that passes.
    pub(crate) fn max_exponent(self) -> f64 {
        match self {
            Growth::Linear => 1.25,
            Growth::Constant => 0.25,
        }
    }
}

impl fmt::Display for Growth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Growth::Linear => "linear",
            Growth::Constant => "constant",
        };
        write!(f, "{name} (exponent at most {})", self.max_exponent())
    }
}

static LOCK: Mutex<()> = Mutex::new(());

/// The lock every scaling test holds while it runs. A test that failed
/// while holding it does not stop the others.
pub(crate) fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The least-squares slope of ln(y) against ln(x).
pub(crate) fn exponent(points: &[(f64, f64)]) -> f64 {
    let n = points.len() as f64;
    let logs: Vec<(f64, f64)> = points.iter().map(|&(x, y)| (x.ln(), y.ln())).collect();
    let mx = logs.iter().map(|p| p.0).sum::<f64>() / n;
    let my = logs.iter().map(|p| p.1).sum::<f64>() / n;
    let sxy: f64 = logs.iter().map(|&(x, y)| (x - mx) * (y - my)).sum();
    let sxx: f64 = logs.iter().map(|&(x, _)| (x - mx) * (x - mx)).sum();
    sxy / sxx
}

/// One measured table: seconds per operation at each size, and the fit.
#[derive(Debug, Clone)]
pub(crate) struct Fit {
    pub(crate) rows: Vec<(usize, f64)>,
    pub(crate) exponent: f64,
    pub(crate) runs: usize,
}

impl Fit {
    fn measure(sizes: &[usize], runs: usize, once: &mut impl FnMut(usize) -> f64) -> Self {
        let mut rows: Vec<(usize, f64)> = sizes.iter().map(|&n| (n, f64::INFINITY)).collect();
        for _ in 0..runs {
            for (n, best) in &mut rows {
                *best = best.min(once(*n));
            }
        }
        let points: Vec<(f64, f64)> = rows.iter().map(|&(n, t)| (n as f64, t.max(1e-12))).collect();
        Self {
            exponent: exponent(&points),
            rows,
            runs,
        }
    }
}

/// Seconds in a readable unit.
fn duration(seconds: f64) -> String {
    match seconds {
        s if s >= 1.0 => format!("{s:.3} s"),
        s if s >= 1e-3 => format!("{:.3} ms", s * 1e3),
        s if s >= 1e-6 => format!("{:.3} us", s * 1e6),
        s => format!("{:.2} ns", s * 1e9),
    }
}

impl fmt::Display for Fit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  {:>10}  {:>12}  {:>12}", "size", "time", "time / size")?;
        for &(n, t) in &self.rows {
            writeln!(f, "  {n:>10}  {:>12}  {:>12}", duration(t), duration(t / n as f64))?;
        }
        write!(
            f,
            "  exponent {:.3} (fastest of {} runs per size)",
            self.exponent, self.runs
        )
    }
}

/// Fits how the time `once(n)` measures grows over `sizes`, and panics when
/// it grows faster than `growth` twice in a row. Returns the passing fit.
pub(crate) fn check(name: &str, growth: Growth, sizes: &[usize], mut once: impl FnMut(usize) -> f64) -> Fit {
    assert!(sizes.len() >= 3, "{name}: fit at least 3 sizes");
    let (least, most) = (sizes.iter().min().unwrap(), sizes.iter().max().unwrap());
    assert!(most / least >= 16, "{name}: the sizes span less than 16x");
    let max = growth.max_exponent();
    let first = Fit::measure(sizes, RUNS, &mut once);
    if first.exponent <= max {
        println!("{name}: {growth}: pass\n{first}");
        return first;
    }
    let again = Fit::measure(sizes, RETRY_RUNS, &mut once);
    if again.exponent <= max {
        println!("{name}: {growth}: pass when re-measured\nfirst:\n{first}\nre-measured:\n{again}");
        return again;
    }
    panic!("{name}: grows faster than {growth}\nfirst:\n{first}\nre-measured:\n{again}");
}

/// The repetitions each size needs for a measurement to last [`TARGET`].
#[derive(Debug, Default)]
struct Reps(Vec<(usize, usize)>);

impl Reps {
    /// Seconds per operation at size `n`, from one measurement, where
    /// `batch(reps)` performs `reps` operations and returns how long they
    /// took. The first call at a size sets its repetitions, uncounted.
    fn measure(&mut self, n: usize, mut batch: impl FnMut(usize) -> Duration) -> f64 {
        let reps = match self.0.iter().find(|e| e.0 == n) {
            Some(&(_, reps)) => reps,
            None => {
                let mut reps = 1;
                loop {
                    let elapsed = batch(reps);
                    if elapsed >= TARGET || reps >= MAX_REPS {
                        break;
                    }
                    let per = elapsed.as_secs_f64().max(1e-9) / reps as f64;
                    let want = (TARGET.as_secs_f64() * 1.5 / per).ceil() as usize;
                    reps = want.clamp(reps * 2, MAX_REPS);
                }
                self.0.push((n, reps));
                reps
            }
        };
        batch(reps).as_secs_f64() / reps as f64
    }
}

/// A `once` for `op(setup(n))`. Inputs are built before the clock starts;
/// results are dropped after it stops.
pub(crate) fn per_input<T, R>(
    mut setup: impl FnMut(usize) -> T,
    mut op: impl FnMut(T) -> R,
) -> impl FnMut(usize) -> f64 {
    let mut reps = Reps::default();
    move |n| {
        reps.measure(n, |reps| {
            let inputs: Vec<T> = (0..reps).map(|_| setup(n)).collect();
            let mut outputs = Vec::with_capacity(reps);
            let start = Instant::now();
            for input in inputs {
                outputs.push(black_box(op(black_box(input))));
            }
            let elapsed = start.elapsed();
            drop(outputs);
            elapsed
        })
    }
}

/// A `once` for `op(&mut state)`, repeated in place on one state per size,
/// made by `make(n)` (outside the timing) the first time `n` is measured.
pub(crate) fn per_call<S, R>(
    mut make: impl FnMut(usize) -> S,
    mut op: impl FnMut(&mut S) -> R,
) -> impl FnMut(usize) -> f64 {
    let mut reps = Reps::default();
    let mut states: Vec<(usize, S)> = Vec::new();
    move |n| {
        let i = match states.iter().position(|e| e.0 == n) {
            Some(i) => i,
            None => {
                states.push((n, make(n)));
                states.len() - 1
            }
        };
        let state = &mut states[i].1;
        reps.measure(n, |reps| {
            let mut outputs = Vec::with_capacity(reps);
            let start = Instant::now();
            for _ in 0..reps {
                outputs.push(black_box(op(state)));
            }
            let elapsed = start.elapsed();
            drop(outputs);
            elapsed
        })
    }
}

/// Checks that `op(setup(n))` grows at most linearly in `n`.
pub(crate) fn linear<T, R>(name: &str, sizes: &[usize], setup: impl FnMut(usize) -> T, op: impl FnMut(T) -> R) -> Fit {
    check(name, Growth::Linear, sizes, per_input(setup, op))
}

/// A setup that builds its input once per size and hands out shared
/// copies, for operations that only read it. Every size's input is kept,
/// since the rounds come back to each size.
pub(crate) fn shared<T>(mut build: impl FnMut(usize) -> T) -> impl FnMut(usize) -> Rc<T> {
    let mut cache: Vec<(usize, Rc<T>)> = Vec::new();
    move |n| {
        if let Some((_, value)) = cache.iter().find(|e| e.0 == n) {
            return Rc::clone(value);
        }
        let value = Rc::new(build(n));
        cache.push((n, Rc::clone(&value)));
        value
    }
}

/// SplitMix64: seeded, reproducible, no dependency.
#[derive(Debug)]
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// A number in `0..n`.
    pub(crate) fn below(&mut self, n: usize) -> u32 {
        (self.next() % n as u64) as u32
    }
}

/// Doubling sizes from `least`: `least`, `2 least`, ..., `16 least`.
fn sizes(least: usize) -> [usize; 5] {
    [least, least * 2, least * 4, least * 8, least * 16]
}

// ---------------------------------------------------------------------------
// The dictionary
// ---------------------------------------------------------------------------

mod dictionary {
    use super::*;
    use crate::term::{Dictionary, DocId, vocab};

    #[derive(Debug, Clone, Copy)]
    enum Kind {
        Iri,
        Blank,
        Lang,
        Typed,
    }

    /// `n` distinct terms: IRIs, blank nodes, language-tagged and typed
    /// literals in turn, of about the same length at every size.
    fn terms(n: usize) -> Vec<(Kind, String)> {
        (0..n)
            .map(|i| match i % 4 {
                0 => (Kind::Iri, format!("http://example.org/term/{i:08}")),
                1 => (Kind::Blank, format!("b{i:08}")),
                2 => (Kind::Lang, format!("label {i:08}")),
                _ => (Kind::Typed, format!("{i:08}")),
            })
            .collect()
    }

    fn intern_all(d: &mut Dictionary, doc: DocId, terms: &[(Kind, String)]) -> u64 {
        let mut sum = 0u64;
        for (kind, text) in terms {
            let id = match kind {
                Kind::Iri => d.intern_iri(text),
                Kind::Blank => d.intern_blank(doc, text),
                Kind::Lang => d.intern_lang_literal(text, "en"),
                Kind::Typed => d.intern_typed_literal(text, vocab::XSD_INTEGER),
            };
            sum = sum.wrapping_add(u64::from(id.0));
        }
        sum
    }

    fn filled(terms: &[(Kind, String)]) -> (Dictionary, DocId, u64) {
        let mut d = Dictionary::new();
        let doc = d.new_document();
        let sum = intern_all(&mut d, doc, terms);
        (d, doc, sum)
    }

    // The timed dictionary tests keep every size in one memory regime, so
    // the fit follows the dictionary and not the cache: a table that leaves
    // the cache between two sizes makes each probe about twice as slow.
    // Interning new terms is timed from 65,536 to 1,048,576 terms (5 to 77
    // MB, past the cache, where fresh memory is touched at every size);
    // re-interning from 2,048 to 32,768 (150 KB to 2.4 MB, in the cache).
    #[test]
    fn scaling_dictionary_intern_new_terms() {
        let _lock = lock();
        linear(
            "scaling_dictionary_intern_new_terms",
            &sizes(65_536),
            shared(terms),
            |terms: Rc<Vec<(Kind, String)>>| filled(&terms),
        );
    }

    #[derive(Debug)]
    struct Known {
        terms: Vec<(Kind, String)>,
        dict: RefCell<Dictionary>,
        doc: DocId,
    }

    #[test]
    fn scaling_dictionary_reintern_known_terms() {
        let _lock = lock();
        let sizes = sizes(2_048);
        // Exactly: re-interning returns the same ids and adds nothing.
        for &n in &sizes {
            let terms = terms(n);
            let (mut d, doc, sum) = filled(&terms);
            let (len, bytes) = (d.len(), d.heap_bytes());
            assert_eq!(intern_all(&mut d, doc, &terms), sum, "n = {n}: the same ids");
            assert_eq!((d.len(), d.heap_bytes()), (len, bytes), "n = {n}: the dictionary grew");
        }
        linear(
            "scaling_dictionary_reintern_known_terms",
            &sizes,
            shared(|n| {
                let terms = terms(n);
                let (dict, doc, _) = filled(&terms);
                Known {
                    terms,
                    dict: RefCell::new(dict),
                    doc,
                }
            }),
            |k: Rc<Known>| intern_all(&mut k.dict.borrow_mut(), k.doc, &k.terms),
        );
    }

    /// Exact, not timed: the heap bytes per term stay under what the layout
    /// allows (the arena and the entries at most double their length, the
    /// slots stay at most 4 per term), from 1,024 to 1,048,576 terms.
    #[test]
    fn scaling_dictionary_bytes_per_term() {
        let _lock = lock();
        let empty = Dictionary::new().heap_bytes();
        let mut table = String::new();
        for shift in 10..=20 {
            let n = 1usize << shift;
            let (d, _, _) = filled(&terms(n));
            let (bytes, text, len) = (d.heap_bytes(), d.text_bytes(), d.len());
            // Entries are 16 bytes and at most doubled; slots are 8 bytes, at
            // most 4 per term once past the initial table (`empty` covers it).
            let bound = 2 * text + 2 * 16 * len + 4 * 8 * len + empty;
            table.push_str(&format!(
                "  {n:>9} terms: {bytes:>10} bytes, {:.1} per term ({:.1} of text), bound {bound}\n",
                bytes as f64 / len as f64,
                text as f64 / len as f64
            ));
            assert!(bytes <= bound, "n = {n}: {bytes} bytes over {bound}\n{table}");
        }
        println!("scaling_dictionary_bytes_per_term:\n{table}");
    }
}

// ---------------------------------------------------------------------------
// Parsing and writing
// ---------------------------------------------------------------------------

#[cfg(feature = "io")]
mod io {
    use super::*;
    use crate::Format;
    use crate::io::{parse, write};
    use crate::term::{Dictionary, TermId};
    use std::fmt::Write as _;

    /// `n` triples as N-Triples: IRIs, blank nodes, typed and tagged
    /// literals, a new subject every two triples.
    fn ntriples(n: usize) -> String {
        let mut text = String::with_capacity(n * 100);
        for i in 0..n {
            let s = i / 2;
            let p = i % 16;
            let _ = match i % 4 {
                0 => writeln!(
                    text,
                    "<http://example.org/s{s}> <http://example.org/p{p}> <http://example.org/o{i}> ."
                ),
                1 => writeln!(
                    text,
                    "<http://example.org/s{s}> <http://example.org/p{p}> \"label {i}\"@en ."
                ),
                2 => writeln!(
                    text,
                    "_:b{s} <http://example.org/p{p}> \"{i}\"^^<http://www.w3.org/2001/XMLSchema#integer> ."
                ),
                _ => writeln!(
                    text,
                    "_:b{s} <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C{}> .",
                    i % 64
                ),
            };
        }
        text
    }

    /// `n` triples (a multiple of 4) as Turtle: prefixes, `;` lists, `a`,
    /// numbers and tagged literals, four triples per subject.
    fn turtle(n: usize) -> String {
        let mut text = String::with_capacity(n * 40);
        text.push_str("@prefix ex: <http://example.org/> .\n");
        for s in 0..n / 4 {
            let _ = writeln!(
                text,
                "ex:s{s} ex:p{} ex:o{s} ; ex:label \"label {s}\"@en ; ex:value {s} ; a ex:C{} .",
                s % 16,
                s % 64
            );
        }
        text
    }

    /// A dictionary and the triples parsed into it.
    type Loaded = (Dictionary, Vec<[TermId; 3]>);

    fn load(text: &str, format: Format) -> Loaded {
        let mut d = Dictionary::new();
        let mut triples = Vec::new();
        parse(&mut d, text.as_bytes(), format, None, None, |t| triples.push(t)).unwrap();
        (d, triples)
    }

    fn parse_counting(text: &str, format: Format) -> (Dictionary, usize) {
        let mut d = Dictionary::new();
        let mut count = 0;
        parse(&mut d, text.as_bytes(), format, None, None, |t| {
            black_box(t);
            count += 1;
        })
        .unwrap();
        (d, count)
    }

    #[test]
    fn scaling_parse_ntriples() {
        let _lock = lock();
        let sizes = sizes(2_048);
        for &n in &sizes {
            assert_eq!(parse_counting(&ntriples(n), Format::NTriples).1, n);
        }
        linear(
            "scaling_parse_ntriples",
            &sizes,
            shared(ntriples),
            |text: Rc<String>| parse_counting(&text, Format::NTriples),
        );
    }

    #[test]
    fn scaling_parse_turtle() {
        let _lock = lock();
        let sizes = sizes(2_048);
        for &n in &sizes {
            assert_eq!(parse_counting(&turtle(n), Format::Turtle).1, n);
        }
        linear("scaling_parse_turtle", &sizes, shared(turtle), |text: Rc<String>| {
            parse_counting(&text, Format::Turtle)
        });
    }

    /// `n` Turtle statements of 11 triples each that take the parser's
    /// slower paths: a base to resolve against, escapes in IRIs and strings,
    /// a nested blank node property list, a list, a long string, prefixed
    /// names with escapes, and a new language tag every eight statements.
    fn turtle_nested(n: usize) -> String {
        let mut text = String::with_capacity(n * 160);
        text.push_str("@base <http://example.org/base/> .\n@prefix ex: <http://example.org/ns#> .\n");
        for i in 0..n {
            let _ = writeln!(
                text,
                "<s{i}> <p\\u0041> [ ex:q \"esc\\t{i}\" ; ex:r ( ex:a\\-{i} \"\"\"long {i}\nline\"\"\" [ ex:t 1.5 ] ) ] ; \
                 ex:l \"x\"@en-x-{} .",
                i / 8
            );
        }
        text
    }

    #[test]
    fn scaling_parse_turtle_nested() {
        let _lock = lock();
        let sizes = sizes(1_024);
        for &n in &sizes {
            assert_eq!(parse_counting(&turtle_nested(n), Format::Turtle).1, 11 * n);
        }
        linear(
            "scaling_parse_turtle_nested",
            &sizes,
            shared(turtle_nested),
            |text: Rc<String>| parse_counting(&text, Format::Turtle),
        );
    }

    /// One statement of `n` objects, then a list of `n` members: what a
    /// parse holds for one statement grows with it.
    fn turtle_one_statement(n: usize) -> String {
        let mut text = String::with_capacity(n * 40);
        text.push_str("@prefix ex: <http://example.org/> .\nex:s ex:p ex:o0");
        for i in 1..n {
            let _ = write!(text, ", ex:o{i}");
        }
        text.push_str(" ; ex:list (");
        for i in 0..n {
            let _ = write!(text, " \"m{i}\"");
        }
        text.push_str(" ) .\n");
        text
    }

    #[test]
    fn scaling_parse_turtle_one_statement() {
        let _lock = lock();
        let sizes = sizes(4_096);
        for &n in &sizes {
            // n objects, the list's head, and a first and a rest per member.
            assert_eq!(parse_counting(&turtle_one_statement(n), Format::Turtle).1, 3 * n + 1);
        }
        linear(
            "scaling_parse_turtle_one_statement",
            &sizes,
            shared(turtle_one_statement),
            |text: Rc<String>| parse_counting(&text, Format::Turtle),
        );
    }

    /// The line and column of an error at the end of a long document are
    /// found in one pass over it.
    #[test]
    fn scaling_parse_error_at_the_end() {
        let _lock = lock();
        let broken = |n: usize| {
            let mut text = ntriples(n);
            text.push_str("<http://example.org/s> <http://example.org/p> \"\\q\" .\n");
            text
        };
        let fail = |text: Rc<String>| {
            let mut d = Dictionary::new();
            parse(&mut d, text.as_bytes(), Format::NTriples, None, None, |t| {
                black_box(t);
            })
            .unwrap_err()
        };
        for &n in &sizes(2_048) {
            let crate::Error::Parse { line, column, .. } = fail(Rc::new(broken(n))) else {
                panic!("a parse error");
            };
            assert_eq!((line, column), (n as u64 + 1, 48));
        }
        linear("scaling_parse_error_at_the_end", &sizes(2_048), shared(broken), fail);
    }

    fn written(format: Format) -> impl FnMut(Rc<Loaded>) -> Vec<u8> {
        move |input| {
            let (d, triples) = &*input;
            let mut out = Vec::new();
            let n = write(d, triples.iter().copied(), &mut out, format).unwrap();
            assert_eq!(n, triples.len());
            out
        }
    }

    #[test]
    fn scaling_write_ntriples() {
        let _lock = lock();
        linear(
            "scaling_write_ntriples",
            &sizes(4_096),
            shared(|n| load(&ntriples(n), Format::NTriples)),
            written(Format::NTriples),
        );
    }

    #[test]
    fn scaling_write_turtle() {
        let _lock = lock();
        linear(
            "scaling_write_turtle",
            &sizes(4_096),
            shared(|n| {
                let (d, mut triples) = load(&ntriples(n), Format::NTriples);
                triples.sort_unstable();
                (d, triples)
            }),
            written(Format::Turtle),
        );
    }
}

// ---------------------------------------------------------------------------
// The engine: relations, joins, variables, the store
// ---------------------------------------------------------------------------

mod engine {
    use super::*;
    use crate::engine::{
        Relation, Round, Store, Variable, View, antijoin, extend_with, join, join_slices, leapjoin, value_filter,
    };

    type Pair = (u32, u32);

    fn random_pairs(rng: &mut Rng, n: usize) -> Vec<Pair> {
        (0..n).map(|_| (rng.below(n), rng.below(n))).collect()
    }

    /// `from_vec` sorts: n log n, which the linear bound allows.
    #[test]
    fn scaling_relation_from_vec() {
        let _lock = lock();
        let mut rng = Rng::new(1);
        linear(
            "scaling_relation_from_vec",
            &sizes(4_096),
            move |n| random_pairs(&mut rng, n),
            Relation::from_vec,
        );
    }

    /// Two interleaved relations of n tuples, a third of them shared.
    #[test]
    fn scaling_relation_merge() {
        let _lock = lock();
        let mut both = shared(|n| {
            let a = Relation::from_vec((0..n as u32).map(|i| (3 * i, i)).collect());
            let b = Relation::from_vec((0..n as u32).map(|i| (2 * i, 2 * i / 3)).collect());
            (a, b)
        });
        let merged = |(a, b): (Relation<Pair>, Relation<Pair>)| a.merge(b);
        for n in sizes(16_384) {
            let (a, b) = (*both(n)).clone();
            let expected = {
                let mut all: Vec<Pair> = a.iter().chain(b.iter()).copied().collect();
                all.sort_unstable();
                all.dedup();
                all
            };
            assert_eq!(&merged((a, b))[..], &expected[..]);
        }
        linear(
            "scaling_relation_merge",
            &sizes(16_384),
            move |n| (*both(n)).clone(),
            merged,
        );
    }

    /// A merge join with n results: keys `2i` against keys `0..2n`, so the
    /// larger side gallops past every other key.
    #[test]
    fn scaling_join_merge() {
        let _lock = lock();
        let build = |n: usize| {
            let a: Vec<Pair> = (0..n as u32).map(|i| (2 * i, i)).collect();
            let b: Vec<Pair> = (0..2 * n as u32).map(|i| (i, i ^ 1)).collect();
            (a, b)
        };
        let join_all = |ab: Rc<(Vec<Pair>, Vec<Pair>)>| {
            let mut results = 0usize;
            join_slices(&ab.0, &ab.1, &mut |k, x, y| {
                black_box((k, x, y));
                results += 1;
            });
            results
        };
        for n in sizes(16_384) {
            assert_eq!(join_all(Rc::new(build(n))), n);
        }
        linear("scaling_join_merge", &sizes(16_384), shared(build), join_all);
    }

    /// The recent part of n tuples against keys in three parts (a third of
    /// the keys): 2n/3 results.
    #[test]
    fn scaling_antijoin() {
        let _lock = lock();
        let build = |n: usize| {
            let mut input: Variable<Pair> = Variable::new();
            input.extend((0..n as u32).map(|i| (i, i)));
            input.changed();
            let mut keys: Variable<u32> = Variable::new();
            for part in 0..3u32 {
                keys.extend((0..n as u32).filter(|i| i % 9 == 3 * part));
                keys.changed();
            }
            (input, keys)
        };
        let anti = |v: Rc<(Variable<Pair>, Variable<u32>)>| {
            let mut results = 0usize;
            antijoin(&v.0, &v.1.all(), |k, x| {
                black_box((k, x));
                results += 1;
            });
            results
        };
        for n in sizes(16_384) {
            assert_eq!(anti(Rc::new(build(n))), n - n.div_ceil(3));
        }
        linear("scaling_antijoin", &sizes(16_384), shared(build), anti);
    }

    /// n source tuples `(a, b)`, each extended by the at most 4 values `c`
    /// of `s(b, c)`, intersected with `t(a, c)`, and filtered by `c != a`.
    /// Keys are near their tuple's position (`b` near `a`, `c` near `b`):
    /// random keys make each lookup a cache miss, and the fit then follows
    /// the cache hierarchy (a 3x step between 4,096 and 32,768 tuples) instead
    /// of the algorithm. A lookup that scanned instead of searching would
    /// still show as quadratic.
    #[test]
    fn scaling_leapjoin() {
        let _lock = lock();
        let build = |n: usize| {
            let mut rng = Rng::new(n as u64);
            let near = |rng: &mut Rng, x: u32| x + rng.below(16);
            let source = Relation::from_vec((0..n as u32).map(|a| (a, near(&mut rng, a))).collect());
            let s = Relation::from_vec(
                (0..n as u32 + 16)
                    .flat_map(|b| {
                        let k = rng.below(5);
                        (0..k).map(|_| (b, near(&mut rng, b))).collect::<Vec<_>>()
                    })
                    .collect(),
            );
            // Each `a` meets about half its proposals in `t`, plus noise.
            let mut t = Vec::new();
            for &(a, b) in source.iter() {
                let from = s.partition_point(|p| p.0 < b);
                for &(_, c) in s[from..].iter().take_while(|p| p.0 == b).step_by(2) {
                    t.push((a, c));
                }
                t.push((a, near(&mut rng, a)));
            }
            (source, s, Relation::from_vec(t))
        };
        let run = |r: Rc<(Relation<Pair>, Relation<Pair>, Relation<Pair>)>| {
            let (source, s, t) = &*r;
            let (s, t) = (View::from(s), View::from(t));
            let mut results = 0usize;
            leapjoin(
                source,
                &mut (
                    extend_with(&s, |&(_a, b): &Pair| b),
                    extend_with(&t, |&(a, _b): &Pair| a),
                    value_filter(|&(a, _b): &Pair, &c: &u32| c != a),
                ),
                |p, c| {
                    black_box((p, c));
                    results += 1;
                },
            );
            results
        };
        for n in sizes(4_096) {
            let results = run(Rc::new(build(n)));
            assert!(results > n / 3 && results < 3 * n, "n = {n}: {results} results");
        }
        linear("scaling_leapjoin", &sizes(4_096), shared(build), run);
    }

    /// Distinct values in a scattered order (an odd multiplier is a bijection on `u32`).
    fn scattered(i: usize) -> u32 {
        (i as u32).wrapping_mul(0x9e37_79b1)
    }

    fn batches(n: usize, batch: usize) -> Vec<Relation<u32>> {
        (0..n / batch)
            .map(|r| Relation::from_vec((r * batch..(r + 1) * batch).map(scattered).collect()))
            .collect()
    }

    /// The inserts of a run, one batch per round.
    fn insert_all<T: Ord + Copy>(batches: Vec<Relation<T>>) -> Variable<T> {
        let mut var = Variable::new();
        for batch in batches {
            var.insert(batch);
            var.changed();
        }
        var
    }

    /// N scattered tuples in 16 batches over 16 rounds: the regime real
    /// reasoning runs in (Brick and LUBM take few rounds, with large
    /// batches). Timed, linear.
    #[test]
    fn scaling_variable_large_batches() {
        let _lock = lock();
        const ROUNDS: usize = 16;
        for n in sizes(16_384) {
            let mut var = insert_all(batches(n, n / ROUNDS));
            var.changed();
            assert_eq!(var.len(), n);
        }
        linear(
            "scaling_variable_large_batches",
            &sizes(16_384),
            |n| batches(n, n / ROUNDS),
            insert_all,
        );
    }

    thread_local! {
        static ORDERINGS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }

    /// A `u32` that counts its orderings (`cmp`, `<`, ...) in [`ORDERINGS`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Counted(u32);

    impl PartialOrd for Counted {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for Counted {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            ORDERINGS.with(|c| c.set(c.get() + 1));
            self.0.cmp(&other.0)
        }
    }

    /// Many small scattered batches, counted, not timed: N tuples in batches
    /// of b = 64 over N/b rounds. The tiered design costs N log²(N/b) here,
    /// not N log N: each round gallops its b fresh tuples through about
    /// log2(N/b) stable tiers, b log2(T/b) orderings per tier of T tuples,
    /// so about log2(N/b)² / 2 orderings per tuple (the tier merges add
    /// O(log(N/b)), a lower-order term). That is the accepted cost of
    /// datafrog's tiered sorted runs; experiment E10 (a Bloom filter per
    /// tier, `docs/specs/experiments.md`) is the change that would make it
    /// N log N. Timing it would fail the linear bound on the log² alone, so
    /// this test pins the class instead, by counting: orderings per tuple
    /// over log2(N/b)² must stay flat (at most 1.15 times its value at the
    /// smallest size) and below 1.0, twice the leading term's coefficient
    /// of 1/2. A step to log³ or N^1.5 fails; load and cache cannot.
    #[test]
    fn scaling_variable_small_batches_comparisons() {
        let _lock = lock();
        const BATCH: usize = 64;
        const MAX_RATIO: f64 = 1.0;
        const MAX_GROWTH: f64 = 1.15;
        let mut rows: Vec<(usize, f64, f64, f64)> = Vec::new();
        for n in sizes(16_384) {
            let batches: Vec<Relation<Counted>> = batches(n, BATCH)
                .into_iter()
                .map(|b| Relation::from_sorted(b.iter().map(|&x| Counted(x)).collect()))
                .collect();
            ORDERINGS.with(|c| c.set(0));
            let mut var = insert_all(batches);
            let orderings = ORDERINGS.with(|c| c.get());
            var.changed();
            assert_eq!(var.len(), n);
            let per_tuple = orderings as f64 / n as f64;
            let log2 = ((n / BATCH) as f64).log2().powi(2);
            rows.push((n, per_tuple, log2, per_tuple / log2));
        }
        let mut table = format!(
            "  {:>8}  {:>18}  {:>12}  {:>6}\n",
            "N", "orderings / tuple", "log2(N/b)^2", "ratio"
        );
        for &(n, per_tuple, log2, ratio) in &rows {
            table.push_str(&format!("  {n:>8}  {per_tuple:>18.2}  {log2:>12.0}  {ratio:>6.3}\n"));
        }
        let first = rows[0].3;
        let ok = rows.iter().all(|r| r.3 <= MAX_GROWTH * first && r.3 <= MAX_RATIO);
        assert!(
            ok,
            "scaling_variable_small_batches_comparisons: orderings per tuple grow faster than \
             log2(N/b)^2 (ratio over {MAX_GROWTH} x the first, or over {MAX_RATIO})\n{table}"
        );
        println!("scaling_variable_small_batches_comparisons: N log2(N/b)^2, pass\n{table}");
    }

    /// Exact, not timed: after every round, a variable of `len` tuples has at
    /// most floor(log2(len)) + 1 stable tiers (each tier more than twice
    /// the next), for batches of 1 and of 64, up to 1,048,576 tuples.
    #[test]
    fn scaling_variable_tiers() {
        let _lock = lock();
        let mut table = String::new();
        for (n, batch) in [(1usize << 16, 1usize), (1 << 20, 64)] {
            let mut var: Variable<u32> = Variable::new();
            let mut most = 0;
            let mut next_report = 1024;
            for b in batches(n, batch) {
                var.insert(b);
                var.changed();
                let (tiers, len) = (var.stable().len(), var.len());
                let bound = len.ilog2() as usize + 1;
                assert!(
                    tiers <= bound,
                    "{len} tuples in batches of {batch}: {tiers} tiers, over {bound}"
                );
                for w in var.stable().windows(2) {
                    assert!(w[0].len() > 2 * w[1].len(), "tiers {} and {}", w[0].len(), w[1].len());
                }
                most = most.max(tiers);
                if len >= next_report {
                    table.push_str(&format!(
                        "  batches of {batch:>2}: {len:>8} tuples, at most {most:>2} tiers so far (bound {bound})\n"
                    ));
                    next_report *= 4;
                }
            }
        }
        println!("scaling_variable_tiers:\n{table}");
    }

    const EDGE: u32 = 1;
    const PATH: u32 = 2;

    /// path(x, z) :- path(x, y), edge(y, z), as a rule family writes it.
    fn reach(round: &mut Round<'_>) {
        let (Some(path_os), Some(edge_so)) = (round.by_object(PATH), round.by_subject(EDGE)) else {
            return;
        };
        join(path_os, edge_so, |_y, x, z| round.emit(x, PATH, z));
    }

    fn path_pairs(store: &Store) -> usize {
        store.table(PATH).map_or(0, |t| t.by_subject().len())
    }

    /// `k` disjoint chains of `length` edges, as `edge` and `path` triples.
    fn chains(k: usize, length: u32) -> Store {
        let mut store = Store::new();
        for c in 0..k as u32 {
            let base = c * (length + 1);
            for i in base..base + length {
                store.insert(i, EDGE, i + 1);
                store.insert(i, PATH, i + 1);
            }
        }
        store
    }

    /// The closure of k disjoint chains of 16 edges: 16 rounds whatever k,
    /// 136 pairs per chain.
    #[test]
    fn scaling_store_disjoint_chains() {
        let _lock = lock();
        const LENGTH: u32 = 16;
        let close = |mut store: Store| {
            let rounds = store.run(reach);
            (rounds, store)
        };
        for k in sizes(256) {
            let (_, store) = close(chains(k, LENGTH));
            assert_eq!(path_pairs(&store), k * 136);
        }
        linear(
            "scaling_store_disjoint_chains",
            &sizes(256),
            |k| chains(k, LENGTH),
            close,
        );
    }

    /// Reachability from the head of a chain of n edges: n + 2 rounds, one
    /// new pair each. Linear total time means a round's cost does not grow
    /// with what is already stored: the semi-naive property.
    #[test]
    fn scaling_store_chain_rounds() {
        let _lock = lock();
        let chain = |n: usize| {
            let mut store = Store::new();
            store.extend((0..n as u32).map(|i| (i, EDGE, i + 1)));
            store.insert(0, PATH, 0);
            store
        };
        let close = |mut store: Store| {
            let rounds = store.run(reach);
            (rounds, store)
        };
        for n in sizes(4_096) {
            let (rounds, store) = close(chain(n));
            assert_eq!((rounds, path_pairs(&store)), (n + 2, n + 1));
        }
        linear("scaling_store_chain_rounds", &sizes(4_096), chain, close);
    }

    /// After a run on k chains of 8 edges (44 triples each), one new edge
    /// into a chain's head, then `run` again: 9 new pairs whatever k. The
    /// second run's time must not grow with the base (ticket 10's budget,
    /// at the engine level).
    #[test]
    fn scaling_store_incremental_insert() {
        let _lock = lock();
        const LENGTH: u32 = 8;
        /// A closed base of `k` chains, and the inserts made since.
        struct Base {
            store: Store,
            k: usize,
            added: usize,
        }
        let base = |k: usize| {
            let mut store = chains(k, LENGTH);
            store.run(reach);
            Base { store, k, added: 0 }
        };
        // A new node with an edge into the next chain's head (cycling): 9
        // new pairs, in 9 rounds, whatever the base and the inserts before.
        let insert_one = |b: &mut Base| {
            let head = (b.added % b.k) as u32 * (LENGTH + 1);
            let fresh = b.k as u32 * (LENGTH + 1) + b.added as u32;
            b.store.insert(fresh, EDGE, head);
            b.store.insert(fresh, PATH, head);
            b.added += 1;
            b.store.run(reach)
        };
        for k in sizes(1_024) {
            let mut b = base(k);
            assert_eq!(path_pairs(&b.store), k * 36);
            assert_eq!(insert_one(&mut b), LENGTH as usize + 1, "rounds");
            assert_eq!(insert_one(&mut b), LENGTH as usize + 1, "rounds");
            let first = k as u32 * (LENGTH + 1);
            assert!(
                b.store.contains(first, PATH, LENGTH),
                "the first new node reaches chain 0's tail"
            );
            assert!(
                b.store.contains(first + 1, PATH, 2 * LENGTH + 1),
                "the second reaches chain 1's"
            );
            assert_eq!(path_pairs(&b.store), k * 36 + 2 * (LENGTH as usize + 1));
        }
        check(
            "scaling_store_incremental_insert",
            Growth::Constant,
            &sizes(1_024),
            per_call(base, insert_one),
        );
    }
}

// ---------------------------------------------------------------------------
// The reasoner
// ---------------------------------------------------------------------------

#[cfg(feature = "io")]
mod reasoner {
    use super::*;
    use crate::{Format, Reasoner};
    use oxrdf::{NamedNode, Triple};
    use std::fmt::Write as _;

    const EX: &str = "http://example.org/";

    /// Brick's shape at about `n` triples, as Turtle: forests of class trees
    /// of a fixed depth (a root, 4 children each, 3 levels: 85 classes a
    /// tree, one tree per 1,024 triples), and typed instances, each of a
    /// leaf class, linked to the next by a property with a domain and a
    /// range, with a language-tagged label. With the class and property
    /// rules each instance gets a fixed number of types, so the closure
    /// grows with the input, as Brick's does with its building models.
    fn brick_like(n: usize) -> String {
        let trees = (n / 1024).max(1);
        let mut text = String::with_capacity(n * 60);
        let mut triples = 0;
        text.push_str(
            "@prefix ex: <http://example.org/> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
             ex:feeds a owl:ObjectProperty ; rdfs:domain ex:Equipment ; rdfs:range ex:Equipment .\n",
        );
        triples += 3;
        for t in 0..trees {
            for c in 0..85 {
                let _ = write!(text, "ex:t{t}c{c} a owl:Class");
                if c > 0 {
                    let _ = write!(text, " ; rdfs:subClassOf ex:t{t}c{}", (c - 1) / 4);
                    triples += 1;
                } else {
                    text.push_str(" ; rdfs:subClassOf ex:Equipment");
                    triples += 1;
                }
                text.push_str(" .\n");
                triples += 1;
            }
        }
        let mut i = 0;
        while triples < n {
            let _ = writeln!(
                text,
                "ex:i{i} a ex:t{}c{} ; ex:feeds ex:i{} ; rdfs:label \"point {i}\"@en .",
                i % trees,
                21 + (i / trees) % 64,
                i + 1
            );
            triples += 3;
            i += 1;
        }
        text
    }

    fn loaded(text: &str) -> Reasoner {
        let mut r = Reasoner::new();
        r.load_str(text, Format::Turtle, None).unwrap();
        r
    }

    /// Parsing a document into the base: interning, the all-or-nothing
    /// buffer, sorting and deduplicating it into the base.
    #[test]
    fn scaling_reasoner_load() {
        let _lock = lock();
        let sizes = sizes(2_048);
        for &n in &sizes {
            let mut r = loaded(&brick_like(n));
            let base = r.reason().base;
            assert!(base >= n && base < n + 3, "n = {n}: {base} base triples");
        }
        linear(
            "scaling_reasoner_load",
            &sizes,
            shared(brick_like),
            |text: Rc<String>| loaded(&text),
        );
    }

    /// The first `reason()` on a loaded base: the facts and the base into a
    /// fresh store, the rule table to the fixpoint, the findings.
    #[test]
    fn scaling_reasoner_full_run() {
        let _lock = lock();
        let sizes = sizes(4_096);
        let mut text = shared(brick_like);
        linear(
            "scaling_reasoner_full_run",
            &sizes,
            move |n| loaded(&text(n)),
            |mut r: Reasoner| {
                let summary = r.reason();
                (summary, r)
            },
        );
    }

    /// A reasoned base, and the additions made since.
    struct Edited {
        r: Reasoner,
        n: usize,
        added: usize,
    }

    /// One added triple, then `reason()`: a new instance's type, or a link
    /// from it to an existing instance, in turn. Its cost must not grow
    /// with the base (`docs/specs/v1.md`, budgets: an incremental addition
    /// under 10% of a full run on Brick).
    fn add_one(e: &mut Edited) -> usize {
        let k = e.added / 2;
        let subject = NamedNode::new_unchecked(format!("{EX}new{k}"));
        let triple = if e.added.is_multiple_of(2) {
            let class = NamedNode::new_unchecked(format!("{EX}t0c{}", 21 + k % 64));
            Triple::new(subject, oxrdf::vocab::rdf::TYPE, class)
        } else {
            let object = NamedNode::new_unchecked(format!("{EX}i{}", k % (e.n / 4)));
            Triple::new(subject, NamedNode::new_unchecked(format!("{EX}feeds")), object)
        };
        e.added += 1;
        e.r.insert([triple.as_ref()]);
        e.r.reason().rounds
    }

    /// One `owl:sameAs` clique of `n` aliases, as the bench's `sameas-clique`
    /// draws it: a chain `a0 owl:sameAs a1`, ..., and 10 triples on each
    /// alias with objects of its own.
    fn clique(n: usize) -> String {
        let mut text = String::with_capacity(n * 400);
        text.push_str("@prefix ex: <http://example.org/> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n");
        for i in 0..n {
            if i + 1 < n {
                let _ = writeln!(text, "ex:a{i} owl:sameAs ex:a{} .", i + 1);
            }
            for j in 0..10 {
                let _ = writeln!(text, "ex:a{i} ex:p{j} ex:v{i}-{j} .");
            }
        }
        text
    }

    /// The eq-* closure of a clique of n aliases holds n² `owl:sameAs`
    /// triples and 10 n² others; the store holds one class: its triples are
    /// rewritten to the representative, `r owl:sameAs r` once and each
    /// alias's 10 triples once (ADR 0003). Counted exactly, and the run
    /// timed: both linear in n.
    #[test]
    fn scaling_reasoner_same_as_clique() {
        let _lock = lock();
        let sizes = sizes(256);
        for &n in &sizes {
            let mut r = loaded(&clique(n));
            let s = r.reason();
            // The base, then 1 `owl:sameAs` and 10 n triples stored, and
            // the two facts of cls-thing and cls-nothing1.
            assert_eq!(s.base, 11 * n - 1, "n = {n}");
            assert_eq!(r.store().len(), 10 * n + 1 + 2, "n = {n}: stored tuples");
            assert_eq!(s.base + s.inferred, 11 * n * n + 2, "n = {n}: the expanded closure");
        }
        let mut text = shared(clique);
        linear(
            "scaling_reasoner_same_as_clique",
            &sizes,
            move |n| loaded(&text(n)),
            |mut r: Reasoner| {
                let summary = r.reason();
                (summary, r)
            },
        );
    }

    /// Expanding a clique's closure on output (`triples()`) is n² triples by
    /// definition: its time per output triple must not grow with n.
    #[test]
    fn scaling_reasoner_same_as_clique_expansion_per_triple() {
        let _lock = lock();
        let sizes = sizes(16);
        let mut read = per_call(
            |n| {
                let mut r = loaded(&clique(n));
                r.reason();
                r
            },
            |r: &mut Reasoner| r.triples().count(),
        );
        let per_triple = move |n: usize| read(n) / (11 * n * n + 2) as f64;
        check(
            "scaling_reasoner_same_as_clique_expansion_per_triple",
            Growth::Constant,
            &sizes,
            per_triple,
        );
    }

    #[test]
    fn scaling_reasoner_incremental_addition() {
        let _lock = lock();
        let make = |n: usize| {
            let mut r = loaded(&brick_like(n));
            r.reason();
            Edited { r, n, added: 0 }
        };
        for n in sizes(2_048) {
            let mut e = make(n);
            let before = e.r.reason().base;
            assert!(add_one(&mut e) >= 1, "an addition runs");
            assert!(add_one(&mut e) >= 1, "an addition runs");
            assert_eq!(
                e.r.counters().delta[0],
                1,
                "incremental: the run starts from the addition"
            );
            let again = e.r.reason();
            assert_eq!((again.rounds, again.base), (0, before + 2), "nothing changed: no round");
        }
        check(
            "scaling_reasoner_incremental_addition",
            Growth::Constant,
            &sizes(2_048),
            per_call(make, add_one),
        );
    }

    /// A symmetric, transitive property over one chain of `n` people, as
    /// OWL2Bench's and UOBM's `hasSameHomeTownWith` (ticket 25): its closure
    /// is all n² pairs. Joined on its middle term, prp-trp derives each pair
    /// once per middle node, n³ in all, and the round buffers every
    /// derivation: 9,300 bytes per closure triple on `owl2bench-rl-1`. What
    /// the property family emits over the run is counted exactly against
    /// the closure, and the run's time per closure pair must not grow with n.
    #[test]
    fn scaling_reasoner_symmetric_transitive_per_closure_pair() {
        let _lock = lock();
        let hometown = |n: usize| {
            let mut text = String::with_capacity(n * 80);
            text.push_str(
                "@prefix ex: <http://example.org/> .\n\
                 @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                 ex:town a owl:SymmetricProperty , owl:TransitiveProperty .\n",
            );
            for i in 1..n {
                let _ = writeln!(text, "ex:p{} ex:town ex:p{i} .", i - 1);
            }
            text
        };
        let sizes = sizes(32);
        for &n in &sizes {
            let mut r = loaded(&hometown(n));
            r.reason();
            let town = r.terms().get_iri(&format!("{EX}town")).unwrap().0;
            let pairs = r.store().table(town).map_or(0, |t| t.by_subject().len());
            assert_eq!(pairs, n * n, "n = {n}: every pair, each person with itself included");
            let emitted: usize = r.counters().emitted_by("property").iter().sum();
            assert!(
                emitted <= 4 * n * n,
                "n = {n}: the property rules emitted {emitted} triples for a closure of {pairs} pairs"
            );
        }
        let mut text = shared(hometown);
        let mut run = per_input(move |n| loaded(&text(n)), |mut r: Reasoner| (r.reason(), r));
        check(
            "scaling_reasoner_symmetric_transitive_per_closure_pair",
            Growth::Constant,
            &sizes,
            move |n| run(n) / (n * n) as f64,
        );
    }
}

// ---------------------------------------------------------------------------
// The identity rules (v2 ticket 06)
// ---------------------------------------------------------------------------

#[cfg(feature = "io")]
mod identity {
    use super::*;
    use crate::{Format, Reasoner, Switch, Switches};
    use std::fmt::Write as _;

    fn loaded(text: &str) -> Reasoner {
        let mut r = Reasoner::with_switches(Switches::default().on(Switch::Identity));
        r.load_str(text, Format::Turtle, None).unwrap();
        r
    }

    /// `n` individuals under a key of two properties: all share their
    /// first value, and pairs share their last, so the key makes `n / 2`
    /// pairs the same. Joining on the first property would compare every
    /// two individuals.
    fn keyed(n: usize) -> String {
        let mut text = String::with_capacity(n * 80);
        text.push_str(
            "@prefix ex: <http://example.org/> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
             ex:Person owl:hasKey ( ex:first ex:last ) .\n",
        );
        for i in 0..n {
            let _ = writeln!(text, "ex:i{i} a ex:Person ; ex:first ex:f ; ex:last ex:l{} .", i / 2);
        }
        text
    }

    /// prp-key over many individuals: each pair merged once, in time linear
    /// in the individuals.
    #[test]
    fn scaling_identity_keys_over_many_individuals() {
        let _lock = lock();
        let sizes = sizes(1_024);
        for &n in &sizes {
            let mut r = loaded(&keyed(n));
            r.reason();
            assert_eq!(r.store().canon().merges(), n / 2, "n = {n}: one merge per pair");
        }
        let mut text = shared(keyed);
        linear(
            "scaling_identity_keys_over_many_individuals",
            &sizes,
            move |n| loaded(&text(n)),
            |mut r: Reasoner| {
                let summary = r.reason();
                (summary, r)
            },
        );
    }

    /// A key list of `n` cells, each with two firsts, `ex:p{i}` and
    /// `ex:q{i}`, so 2^n paths: `ex:a` and `ex:b` share every `ex:p{i}`,
    /// `ex:c` and `ex:d` every `ex:q{i}` but the last. Only `ex:a` and
    /// `ex:b` are the same.
    fn branching(n: usize) -> String {
        let mut text = String::with_capacity(n * 200);
        text.push_str(
            "@prefix ex: <http://example.org/> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
             @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
             ex:K owl:hasKey ex:cell0 .\n\
             ex:a a ex:K . ex:b a ex:K . ex:c a ex:K . ex:d a ex:K .\n",
        );
        for i in 0..n {
            let rest = if i + 1 == n {
                "rdf:nil".to_owned()
            } else {
                format!("ex:cell{}", i + 1)
            };
            let _ = writeln!(text, "ex:cell{i} rdf:first ex:p{i} , ex:q{i} ; rdf:rest {rest} .");
            let _ = writeln!(text, "ex:a ex:p{i} \"v{i}\" . ex:b ex:p{i} \"v{i}\" .");
            if i + 1 == n {
                let _ = writeln!(text, "ex:c ex:q{i} \"x\" . ex:d ex:q{i} \"y\" .");
            } else {
                let _ = writeln!(text, "ex:c ex:q{i} \"w{i}\" . ex:d ex:q{i} \"w{i}\" .");
            }
        }
        text
    }

    /// prp-key on a key list with 2^n paths (a chaos find, where
    /// `owl:equivalentProperty owl:sameAs rdf:rest` closed a list's rests):
    /// linear in the cells, as the pairs are read, not the paths.
    #[test]
    fn scaling_identity_keys_on_a_branching_list() {
        let _lock = lock();
        let sizes = sizes(1_024);
        for &n in &sizes {
            let mut r = loaded(&branching(n));
            r.reason();
            assert_eq!(r.store().canon().merges(), 1, "n = {n}: ex:a and ex:b only");
        }
        let mut text = shared(branching);
        linear(
            "scaling_identity_keys_on_a_branching_list",
            &sizes,
            move |n| loaded(&text(n)),
            |mut r: Reasoner| {
                let summary = r.reason();
                (summary, r)
            },
        );
    }

    /// `n` individuals of a restriction to one value of `ex:p`, each with
    /// two values, the second the next one's first: cls-maxc2 makes all
    /// `n + 1` values one class.
    fn at_most_one(n: usize) -> String {
        let mut text = String::with_capacity(n * 80);
        text.push_str(
            "@prefix ex: <http://example.org/> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
             @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
             ex:R owl:maxCardinality \"1\"^^xsd:nonNegativeInteger ; owl:onProperty ex:p .\n",
        );
        for i in 0..n {
            let _ = writeln!(text, "ex:u{i} a ex:R ; ex:p ex:v{i} , ex:v{} .", i + 1);
        }
        text
    }

    /// cls-maxc2 over many values: `n + 1` values merged into one class
    /// (n merges), in time linear in the values.
    #[test]
    fn scaling_identity_max_cardinality_over_many_values() {
        let _lock = lock();
        let sizes = sizes(1_024);
        for &n in &sizes {
            let mut r = loaded(&at_most_one(n));
            r.reason();
            assert_eq!(r.store().canon().merges(), n, "n = {n}: the values are one class");
        }
        let mut text = shared(at_most_one);
        linear(
            "scaling_identity_max_cardinality_over_many_values",
            &sizes,
            move |n| loaded(&text(n)),
            |mut r: Reasoner| {
                let summary = r.reason();
                (summary, r)
            },
        );
    }
}

// ---------------------------------------------------------------------------
// The datatype map
// ---------------------------------------------------------------------------

#[cfg(feature = "io")]
mod datatypes {
    use super::*;
    use crate::{Format, Reasoner};
    use std::fmt::Write as _;

    /// The ranges of the eight data properties: each literal is typed with
    /// one (dt-not-type's typed half), often outside its value space.
    const RANGES: [&str; 8] = [
        "xsd:integer",
        "xsd:decimal",
        "xsd:string",
        "xsd:token",
        "xsd:double",
        "xsd:dateTime",
        "rdfs:Literal",
        "owl:rational",
    ];

    /// `n` triples, each with a literal of its own, of the datatypes of the
    /// map in turn (numbers, floats, strings, binaries, date-times), a
    /// quarter of them ill-typed, on eight data properties with ranges.
    fn literals(n: usize) -> String {
        let mut text = String::with_capacity(n * 80);
        text.push_str(
            "@prefix ex: <http://example.org/> .\n\
             @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
        );
        for (j, range) in RANGES.iter().enumerate() {
            let _ = writeln!(text, "ex:d{j} rdfs:range {range} .");
        }
        for i in 0..n {
            let k = i / 12;
            let literal = match i % 12 {
                0 => format!("\"{k}\"^^xsd:int"),
                1 => format!("\"-{k}.25\"^^xsd:decimal"),
                2 => format!("\"{k}e-3\"^^xsd:double"),
                3 => format!("\"a{k} b\"^^xsd:token"),
                4 => format!("\"{k}/7\"^^owl:rational"),
                5 => format!("\"{}-01-01T00:00:00Z\"^^xsd:dateTime", 10_000 + k),
                6 => format!("\"{k:08X}\"^^xsd:hexBinary"),
                7 => format!("\"{k} \"^^xsd:integer"),
                8 => format!("\"x{k}\"^^xsd:boolean"),
                9 => format!("\"{}\"^^xsd:unsignedByte", 256 + k),
                10 => format!("\"s{k}\"@en"),
                _ => format!("\"{k}.5\"^^xsd:float"),
            };
            let _ = writeln!(text, "ex:i{i} ex:d{} {literal} .", i % 8);
        }
        text
    }

    fn loaded(text: &str) -> Reasoner {
        let mut r = Reasoner::new();
        r.load_str(text, Format::Turtle, None).unwrap();
        r
    }

    /// dt-not-type over `n` literals, both halves, in a full run: linear.
    #[test]
    fn scaling_datatype_check_literals() {
        let _lock = lock();
        let sizes = sizes(4_096);
        for &n in &sizes {
            let mut r = loaded(&literals(n));
            r.reason();
            let findings = r.diagnostics().len();
            assert!(findings > n / 3 && findings < 2 * n, "n = {n}: {findings} findings");
        }
        let mut text = shared(literals);
        linear(
            "scaling_datatype_check_literals",
            &sizes,
            move |n| loaded(&text(n)),
            |mut r: Reasoner| {
                let summary = r.reason();
                (summary, r)
            },
        );
    }
}

// ---------------------------------------------------------------------------
// The schema rules
// ---------------------------------------------------------------------------

#[cfg(feature = "io")]
mod schema {
    use super::*;
    use crate::{Format, Reasoner, Switch, Switches};
    use oxrdf::{NamedNode, Triple};
    use std::fmt::Write as _;

    const EX: &str = "http://example.org/";

    /// Classes per tree: a binary tree of nine levels.
    const CLASSES: usize = 511;
    /// Properties per tree: a binary tree of eight levels.
    const PROPERTIES: usize = 128;

    /// Deep class and property hierarchies, as Turtle: `n / 512` trees of
    /// 511 classes and as many of 128 properties, binary (node `c` below
    /// `(c - 1) / 2`), so the depth is fixed and the closure grows with
    /// `n`. Every restriction shares one property or one filler with all the
    /// others of its kind, so a rule that joined them pairwise would grow as
    /// n²:
    /// - for each class, a someValuesFrom and an allValuesFrom restriction
    ///   with it as filler, all on one property (scm-svf1, scm-avf1 follow
    ///   the class trees);
    /// - for each property, a someValuesFrom, an allValuesFrom and a
    ///   hasValue restriction, all with one filler (scm-svf2, scm-avf2,
    ///   scm-hv follow the property trees);
    /// - for every fourth class, an intersection and a union of it and the
    ///   next (scm-int, scm-uni);
    /// - on each property tree's root, a domain and a range of a leaf class
    ///   (scm-dom1 and scm-rng1 up the class tree, scm-dom2 and scm-rng2
    ///   down the property tree).
    fn hierarchy(n: usize) -> String {
        let trees = (n / 512).max(1);
        let mut text = String::with_capacity(n * 600);
        text.push_str(
            "@prefix ex: <http://example.org/> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
        );
        for t in 0..trees {
            for c in 0..CLASSES {
                if c > 0 {
                    let _ = writeln!(text, "ex:C{t}_{c} rdfs:subClassOf ex:C{t}_{} .", (c - 1) / 2);
                }
                let _ = writeln!(
                    text,
                    "ex:S{t}_{c} owl:someValuesFrom ex:C{t}_{c} ; owl:onProperty ex:p ."
                );
                let _ = writeln!(
                    text,
                    "ex:A{t}_{c} owl:allValuesFrom ex:C{t}_{c} ; owl:onProperty ex:p ."
                );
                if c % 4 == 1 {
                    let d = c + 1;
                    let _ = writeln!(text, "ex:I{t}_{c} owl:intersectionOf ( ex:C{t}_{c} ex:C{t}_{d} ) .");
                    let _ = writeln!(text, "ex:U{t}_{c} owl:unionOf ( ex:C{t}_{c} ex:C{t}_{d} ) .");
                }
            }
            for k in 0..PROPERTIES {
                if k > 0 {
                    let _ = writeln!(text, "ex:P{t}_{k} rdfs:subPropertyOf ex:P{t}_{} .", (k - 1) / 2);
                }
                let _ = writeln!(
                    text,
                    "ex:T{t}_{k} owl:someValuesFrom ex:F ; owl:onProperty ex:P{t}_{k} ."
                );
                let _ = writeln!(
                    text,
                    "ex:V{t}_{k} owl:allValuesFrom ex:F ; owl:onProperty ex:P{t}_{k} ."
                );
                let _ = writeln!(text, "ex:H{t}_{k} owl:hasValue ex:v ; owl:onProperty ex:P{t}_{k} .");
            }
            let (leaf, other) = (CLASSES - 1, CLASSES - 2);
            let _ = writeln!(
                text,
                "ex:P{t}_0 rdfs:domain ex:C{t}_{leaf} ; rdfs:range ex:C{t}_{other} ."
            );
        }
        text
    }

    fn loaded(text: &str) -> Reasoner {
        let mut r = Reasoner::with_switches(Switches::default().on(Switch::Schema));
        r.load_str(text, Format::Turtle, None).unwrap();
        r
    }

    /// The first `reason()` over the hierarchy, in the default
    /// configuration with the `schema` switch on.
    #[test]
    fn scaling_schema_deep_hierarchy() {
        let _lock = lock();
        let sizes = sizes(512);
        // Each rule reaches the far end of its tree.
        let mut r = loaded(&hierarchy(sizes[1]));
        r.reason();
        let closure: std::collections::HashSet<Triple> = r.triples().collect();
        let ex = |local: &str| NamedNode::new_unchecked(format!("{EX}{local}"));
        let sub_class_of = |a: &str, b: &str| Triple::new(ex(a), oxrdf::vocab::rdfs::SUB_CLASS_OF, ex(b));
        let (leaf, last) = (CLASSES - 1, PROPERTIES - 1);
        for t in [
            sub_class_of(&format!("S1_{leaf}"), "S1_0"),
            sub_class_of(&format!("A1_{leaf}"), "A1_0"),
            sub_class_of(&format!("T1_{last}"), "T1_0"),
            sub_class_of("V1_0", &format!("V1_{last}")),
            sub_class_of(&format!("H1_{last}"), "H1_0"),
            sub_class_of("I1_1", "C1_0"),
            sub_class_of("C1_2", "U1_1"),
            Triple::new(ex(&format!("P1_{last}")), oxrdf::vocab::rdfs::DOMAIN, ex("C1_0")),
            Triple::new(ex(&format!("P1_{last}")), oxrdf::vocab::rdfs::RANGE, ex("C1_0")),
        ] {
            assert!(closure.contains(&t), "missing {t}");
        }
        assert!(!closure.contains(&sub_class_of("S0_1", "S1_0")), "trees stay apart");
        let mut text = shared(hierarchy);
        linear(
            "scaling_schema_deep_hierarchy",
            &sizes,
            move |n| loaded(&text(n)),
            |mut r: Reasoner| {
                let summary = r.reason();
                (summary, r)
            },
        );
    }
}

// ---------------------------------------------------------------------------
// The trivial group (v2 ticket 08)
// ---------------------------------------------------------------------------

#[cfg(feature = "io")]
mod trivial {
    use super::*;
    use crate::{Format, Reasoner, Switches};
    use std::fmt::Write as _;

    /// `n` classes, each declared and with an instance linked to the next:
    /// scm-cls gives four triples per class, eq-ref one per term. With
    /// `read_by_a_rule`, `owl:sameAs rdfs:domain ex:C` makes the trivial
    /// family store eq-ref, and prp-dom types every term.
    fn declared(n: usize, read_by_a_rule: bool) -> String {
        let mut text = String::with_capacity(n * 90);
        text.push_str(
            "@prefix ex: <http://example.org/> .\n\
             @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
        );
        if read_by_a_rule {
            text.push_str("owl:sameAs rdfs:domain ex:C .\n");
        }
        for i in 0..n {
            let _ = writeln!(
                text,
                "ex:C{i} a owl:Class . ex:i{i} a ex:C{i} ; ex:next ex:i{} .",
                i + 1
            );
        }
        text
    }

    fn loaded(text: &str) -> Reasoner {
        let mut r = Reasoner::with_switches(Switches::complete());
        r.load_str(text, Format::Turtle, None).unwrap();
        r
    }

    /// The complete configuration's full run and a read of its closure,
    /// with eq-ref on read and with eq-ref stored: linear in the terms.
    #[test]
    fn scaling_trivial_eq_ref_and_scm_cls() {
        let _lock = lock();
        let sizes = sizes(1_024);
        for read_by_a_rule in [false, true] {
            for &n in &sizes {
                let mut r = loaded(&declared(n, read_by_a_rule));
                let s = r.reason();
                assert_eq!(s.base + s.inferred, r.triples().count(), "n = {n}");
                assert!(s.inferred > 6 * n, "n = {n}: {s:?}");
            }
            let mut text = shared(move |n| declared(n, read_by_a_rule));
            linear(
                if read_by_a_rule {
                    "scaling_trivial_eq_ref_stored"
                } else {
                    "scaling_trivial_eq_ref_on_read"
                },
                &sizes,
                move |n| loaded(&text(n)),
                |mut r: Reasoner| {
                    let summary = r.reason();
                    let read = r.triples().count();
                    (summary, read, r)
                },
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The literal-values group (v2 ticket 10)
// ---------------------------------------------------------------------------

#[cfg(feature = "io")]
mod literal_values {
    use super::*;
    use crate::{Format, Reasoner, Switches};
    use std::fmt::Write as _;

    /// `n` individuals, each with three literals of one value in three
    /// forms (`"i"^^xsd:integer`, `"i.0"^^xsd:decimal`, `"0i"^^xsd:int`):
    /// dt-eq makes `n` classes of three, and eq-rep-o gives each individual
    /// six more triples; dt-type2 types each literal.
    fn literals(n: usize) -> String {
        let mut text = String::with_capacity(n * 140);
        text.push_str(
            "@prefix ex: <http://example.org/> .\n\
             @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n",
        );
        // From 2: the cardinalities 0 and 1 the rules name would join.
        for i in 2..n + 2 {
            let _ = writeln!(
                text,
                "ex:x{i} ex:p \"{i}\"^^xsd:integer ; ex:q \"{i}.0\"^^xsd:decimal ; ex:r \"0{i}\"^^xsd:int ."
            );
        }
        text
    }

    fn loaded(text: &str) -> Reasoner {
        let mut r = Reasoner::with_switches(Switches::complete());
        r.load_str(text, Format::Turtle, None).unwrap();
        r
    }

    /// dt-eq over `n` literals, in the complete configuration: the full run
    /// and a read of its closure are linear in the literals.
    #[test]
    fn scaling_literal_values_dt_eq() {
        let _lock = lock();
        let sizes = sizes(2_048);
        for &n in &sizes {
            let mut r = loaded(&literals(n));
            let s = r.reason();
            assert_eq!(s.base, 3 * n, "n = {n}");
            assert_eq!(s.base + s.inferred, r.triples().count(), "n = {n}");
            let copied = r
                .triples()
                .filter(|t| t.predicate.as_str() == "http://example.org/p")
                .count();
            assert_eq!(copied, 3 * n, "n = {n}: each individual has the three literals by ex:p");
        }
        let mut text = shared(literals);
        linear(
            "scaling_literal_values_dt_eq",
            &sizes,
            move |n| loaded(&text(n)),
            |mut r: Reasoner| {
                let summary = r.reason();
                let read = r.triples().count();
                (summary, read, r)
            },
        );
    }
}

// ---------------------------------------------------------------------------
// Parallel rounds (v2 ticket 17)
// ---------------------------------------------------------------------------

mod parallel {
    use super::*;
    use crate::engine::{PARALLEL_FROM, Workers};
    use crate::{Reasoner, Threads};
    use oxrdf::vocab::{rdf, rdfs};
    use oxrdf::{NamedNode, Triple};

    fn ex(local: String) -> NamedNode {
        NamedNode::new_unchecked(format!("http://example.org/{local}"))
    }

    /// LUBM's shape at about `n` triples: a class hierarchy, properties
    /// with domains, ranges and a super-property, and students, each of a
    /// leaf class, taking a course and advised by a professor. The rules
    /// give each student and course a fixed number of types, and each link
    /// a copy, so every round's delta grows with `n`.
    fn university(n: usize) -> Vec<Triple> {
        let iri = |s: &str| ex(s.to_owned());
        let mut triples = Vec::with_capacity(n + 16);
        let schema = [
            ("Graduate", rdfs::SUB_CLASS_OF, "Student"),
            ("Undergraduate", rdfs::SUB_CLASS_OF, "Student"),
            ("Student", rdfs::SUB_CLASS_OF, "Person"),
            ("Professor", rdfs::SUB_CLASS_OF, "Faculty"),
            ("Faculty", rdfs::SUB_CLASS_OF, "Person"),
            ("takes", rdfs::DOMAIN, "Student"),
            ("takes", rdfs::RANGE, "Course"),
            ("advisor", rdfs::DOMAIN, "Person"),
            ("advisor", rdfs::RANGE, "Professor"),
            ("advisor", rdfs::SUB_PROPERTY_OF, "knows"),
        ];
        for (s, p, o) in schema {
            triples.push(Triple::new(iri(s), p, iri(o)));
        }
        let students = n / 3;
        let (courses, professors) = (students / 20 + 1, students / 50 + 1);
        for i in 0..students {
            let student = ex(format!("student{i}"));
            let class = if i % 4 == 0 { "Graduate" } else { "Undergraduate" };
            triples.push(Triple::new(student.clone(), rdf::TYPE, iri(class)));
            triples.push(Triple::new(
                student.clone(),
                iri("takes"),
                ex(format!("course{}", i % courses)),
            ));
            triples.push(Triple::new(
                student,
                iri("advisor"),
                ex(format!("professor{}", i % professors)),
            ));
        }
        triples
    }

    /// A reasoned base of about `n` triples, on `threads`.
    fn reasoned(n: usize, threads: Threads) -> (Reasoner, Triple) {
        on(n, threads.workers())
    }

    /// A reasoned base of about `n` triples, on `workers`.
    fn on(n: usize, workers: Workers) -> (Reasoner, Triple) {
        let triples = university(n);
        let mut r = Reasoner::new();
        r.set_workers(workers);
        r.insert(triples.iter().map(Triple::as_ref));
        r.reason();
        let last = triples.last().cloned().expect("triples");
        (r, last)
    }

    /// A full run of the same base: a triple removed and put back, then
    /// `reason()`.
    fn full_run((r, last): &mut (Reasoner, Triple)) -> usize {
        r.remove([last.as_ref()]);
        r.insert([last.as_ref()]);
        r.reason().inferred
    }

    /// The rounds of a full run, every one with a delta past the parallel
    /// threshold at the largest sizes: linear on one thread.
    #[test]
    fn scaling_parallel_rounds_one_thread() {
        let _lock = lock();
        let sizes = sizes(PARALLEL_FROM);
        check(
            "scaling_parallel_rounds_one_thread",
            Growth::Linear,
            &sizes,
            per_call(|n| reasoned(n, Threads::ONE), full_run),
        );
    }

    /// The same rounds on twelve threads: linear too, and the closure the
    /// one thread gives.
    #[test]
    fn scaling_parallel_rounds_twelve_threads() {
        let _lock = lock();
        let sizes = sizes(PARALLEL_FROM);
        let n = sizes[0];
        let (mut one, mut twelve) = (reasoned(n, Threads::ONE), reasoned(n, Threads::exactly(12)));
        assert_eq!(full_run(&mut one), full_run(&mut twelve));
        assert!(one.0.triples().eq(twelve.0.triples()));
        let threads = cfg!(all(feature = "parallel", not(target_family = "wasm")));
        assert_eq!(twelve.0.store().engaged() > 0, threads && n >= PARALLEL_FROM);
        check(
            "scaling_parallel_rounds_twelve_threads",
            Growth::Linear,
            &sizes,
            per_call(|n| reasoned(n, Threads::exactly(12)), full_run),
        );
    }

    /// Measures where parallel rounds start to pay: a full run on one
    /// thread against all cores with no threshold (every round parallel,
    /// chunks as large as usual), at bases from
    /// 256 to 262,144 triples, the fastest of `RUNS` measurements each. It
    /// prints the table and the smallest size from which every size is
    /// faster on all cores; [`PARALLEL_FROM`] is set from it. It fails only
    /// if, on four or more threads, the largest size is not faster on all
    /// cores.
    ///
    /// Measured 2026-11-06 on the 12-core Apple machine the tests run on
    /// (cores of three speeds: 2, 4 and 6), shared with other jobs (load
    /// about 2), twice, with the same result: all cores were slower up to
    /// 32,768 triples (0.10 times as fast at 256, 0.55 at 16,384, 0.75 at
    /// 32,768) and faster from 65,536 (1.19 to 1.23 times), 1.9 times at
    /// 131,072 and 2.6 to 2.8 times at 262,144. So [`PARALLEL_FROM`] is
    /// 65,536.
    #[test]
    fn timing_parallel_round_threshold() {
        let _lock = lock();
        let sizes = [256, 1_024, 4_096, 16_384, 32_768, 65_536, 131_072, 262_144];
        let mut one = per_call(|n| reasoned(n, Threads::ONE), full_run);
        let mut all = per_call(|n| on(n, Workers::all().from(0)), full_run);
        let mut rows: Vec<(usize, f64, f64)> = sizes.iter().map(|&n| (n, f64::INFINITY, f64::INFINITY)).collect();
        for _ in 0..RUNS {
            for row in &mut rows {
                row.1 = row.1.min(one(row.0));
                row.2 = row.2.min(all(row.0));
            }
        }
        let threads = Workers::all().threads();
        let mut table = format!(
            "timing_parallel_round_threshold ({threads} threads):\n  {:>8}  {:>10}  {:>10}  {:>6}\n",
            "triples", "one", "all", "gain"
        );
        for &(n, a, b) in &rows {
            let _ = std::fmt::Write::write_fmt(
                &mut table,
                format_args!("  {n:>8}  {:>10}  {:>10}  {:>6.2}\n", duration(a), duration(b), a / b),
            );
        }
        let from = rows
            .iter()
            .rposition(|&(_, a, b)| b >= a)
            .map_or(Some(rows[0].0), |i| rows.get(i + 1).map(|r| r.0));
        println!("{table}  all cores win from: {from:?}; PARALLEL_FROM = {PARALLEL_FROM}");
        let &(_, a, b) = rows.last().expect("sizes");
        if threads >= 4 {
            assert!(b < a, "all cores are not faster on the largest base\n{table}");
        }
    }
}
