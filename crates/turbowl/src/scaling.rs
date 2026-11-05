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
