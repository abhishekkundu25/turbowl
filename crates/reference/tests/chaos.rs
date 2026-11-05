//! Chaos: the core's `Reasoner` against the reference evaluator, on
//! generated inputs.
//!
//! Each iteration draws an input from the seed (`generate`, sizes 1 to 48)
//! and compares closures and diagnostics, as sets, with
//! `turtle_reasoner_reference::evaluate`. The full test runs the core once;
//! the incremental test inserts the input in random batches with `reason()`
//! between, then removes a random part and runs again, then puts it back and
//! runs again, and each state must equal a full evaluation of its base.
//!
//! Deterministic: CHAOS_SEED and CHAOS_ITERS override the seed and the
//! count; CHAOS_ONLY=<i> runs iteration i alone. A failure prints the line
//! that replays it and saves the input as N-Triples under `target/chaos/`.
//! An iteration past its budget (CHAOS_ITERATION_BUDGET_MS) is reported the
//! same way by a watchdog, which then exits the process.
//!
//! Inputs whose reference closure would pass `MAX_CLOSURE` triples are
//! skipped and counted: the reference has no representatives, so a large
//! `owl:sameAs` clique is beyond it.
//!
//! Until the equality rules land (ticket 11, [`EQUALITY_PENDING`]), inputs
//! that mention `owl:sameAs`, `owl:FunctionalProperty` or
//! `owl:InverseFunctionalProperty` are not compared, and counted: on every
//! other input the eq-* rules derive nothing, so the core must already
//! equal the reference there (the argument is on the oracle crate's
//! `ours::needs_equality`, the same criterion). Each shard prints how many
//! inputs it compared and how many it left out, and why.

use oxrdf::{Term, Triple};
use std::collections::{BTreeSet, HashSet};
use std::io::Write;
use std::ops::Range;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use turtle_reasoner::Reasoner;
use turtle_reasoner_reference::{Diagnostics, Rng, evaluate_within, generate, to_ntriples};

/// The largest reference closure an iteration is compared on.
const MAX_CLOSURE: usize = 20_000;

/// The largest input `generate` is asked for.
const MAX_SIZE: usize = 48;

/// How long one iteration may run before the watchdog fails the run. The
/// reference takes about 5 ms on an average input (measured
/// 2026-11-05 on an M-series Mac, 5,000 seeds); the budget leaves room for
/// the largest inputs and a loaded machine.
const ITERATION_BUDGET: Duration = Duration::from_secs(30);

/// Each test runs as this many shards, which the harness runs in parallel.
const SHARDS: usize = 2;

fn env_or<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// The iterations shard `k` runs: its share of `0..iters`, or only
/// CHAOS_ONLY when that is set and falls in it.
fn iterations(iters: usize, k: usize) -> Range<usize> {
    let all = iters * k / SHARDS..iters * (k + 1) / SHARDS;
    match std::env::var("CHAOS_ONLY").ok().and_then(|s| s.parse().ok()) {
        Some(i) if all.contains(&i) => i..i + 1,
        Some(_) => 0..0,
        None => all,
    }
}

/// The input of iteration `i`, and the generator its splits draw from.
fn input_of(seed: u64, i: usize) -> (Vec<Triple>, Rng) {
    let mut rng = Rng::at(seed, i);
    let size = 1 + rng.below(MAX_SIZE);
    let input = generate(rng.next_u64(), size);
    (input, rng)
}

/// True while the core has no equality rules (ticket 11 sets it to false).
/// The oracle crate's `ours::EQUALITY_PENDING` is the same switch.
const EQUALITY_PENDING: bool = true;

/// The terms whose presence lets the eq-* rules fire: `owl:sameAs`, and the
/// property types of prp-fp and prp-ifp, which derive it.
const EQUALITY_TERMS: [&str; 3] = [
    "http://www.w3.org/2002/07/owl#sameAs",
    "http://www.w3.org/2002/07/owl#FunctionalProperty",
    "http://www.w3.org/2002/07/owl#InverseFunctionalProperty",
];

/// True when some triple has one of [`EQUALITY_TERMS`] as its subject,
/// predicate or object: the criterion of the oracle crate's
/// `ours::needs_equality` (which says why it is sound), repeated here.
fn needs_equality(input: &[Triple]) -> bool {
    let is = |t: &Term| matches!(t, Term::NamedNode(n) if EQUALITY_TERMS.contains(&n.as_str()));
    input
        .iter()
        .any(|t| EQUALITY_TERMS.contains(&t.predicate.as_str()) || is(&t.subject.clone().into()) || is(&t.object))
}

/// What one iteration did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Done {
    Compared,
    /// The reference closure would pass `MAX_CLOSURE`.
    TooLarge,
    /// The input needs the equality rules ([`EQUALITY_PENDING`]).
    NeedsEquality,
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// A line on stderr past libtest's capture, so a passing run shows it too.
fn say(line: &str) {
    let _ = std::io::stderr().lock().write_all(format!("{line}\n").as_bytes());
}

/// What the reference says, or `None` when the input is too large for it.
fn reference(base: &HashSet<Triple>) -> Option<(HashSet<Triple>, Diagnostics)> {
    evaluate_within(base, MAX_CLOSURE).map(|e| (e.closure, e.diagnostics))
}

fn core_state(reasoner: &Reasoner) -> (HashSet<Triple>, Diagnostics) {
    let closure = reasoner.triples().collect();
    let diagnostics = reasoner
        .diagnostics()
        .iter()
        .map(|d| (d.code.to_owned(), d.terms.clone()))
        .collect();
    (closure, diagnostics)
}

/// Where the inputs of failing iterations go: `target/chaos/`.
fn chaos_dir() -> PathBuf {
    let tmp = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    tmp.parent().map_or(tmp.clone(), |target| target.join("chaos"))
}

/// Saves the input and returns the report: what failed, the replay line,
/// and where the input is.
fn failure_report(kind: &str, seed: u64, iters: usize, i: usize, input: &[Triple], why: &str) -> String {
    let dir = chaos_dir();
    let path = dir.join(format!("{kind}-seed{seed}-iteration{i}.nt"));
    let saved = std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&path, to_ntriples(input)))
        .map_or_else(
            |e| format!("not saved ({e})"),
            |()| format!("saved to {}", path.display()),
        );
    format!(
        "{kind} iteration {i} (seed {seed}; replay with CHAOS_SEED={seed} CHAOS_ITERS={iters} CHAOS_ONLY={i}) \
         failed: {why}\n  input: {} triples, {saved}",
        input.len()
    )
}

/// The first differences between the reference's answer and the core's.
fn difference(expected: &(HashSet<Triple>, Diagnostics), actual: &(HashSet<Triple>, Diagnostics)) -> Option<String> {
    let mut lines = Vec::new();
    let missing: BTreeSet<String> = expected.0.difference(&actual.0).map(|t| format!("{t} .")).collect();
    let extra: BTreeSet<String> = actual.0.difference(&expected.0).map(|t| format!("{t} .")).collect();
    for (label, items) in [("missing", &missing), ("extra", &extra)] {
        if !items.is_empty() {
            lines.push(format!("{} {label} triples, first:", items.len()));
            lines.extend(items.iter().take(10).map(|t| format!("    {t}")));
        }
    }
    for (label, a, b) in [("missing", &expected.1, &actual.1), ("extra", &actual.1, &expected.1)] {
        let items: BTreeSet<String> = a
            .difference(b)
            .map(|(code, terms)| {
                let terms: Vec<String> = terms.iter().map(ToString::to_string).collect();
                format!("{code} {}", terms.join(" "))
            })
            .collect();
        if !items.is_empty() {
            lines.push(format!("{} {label} diagnostics, first:", items.len()));
            lines.extend(items.iter().take(10).map(|d| format!("    {d}")));
        }
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// The iteration a shard is running: its number, start and input.
type Running = Option<(usize, Instant, Arc<Vec<Triple>>)>;

/// A shard's watchdog: a thread that wakes every 100 ms and, when the
/// iteration in the slot has run past the budget, reports it and exits the
/// process (a hang would otherwise end in a timeout that names no input).
/// Dropped, it prints the shard's five slowest iterations.
struct Watchdog {
    slot: Arc<Mutex<Running>>,
    slowest: Vec<(Duration, usize)>,
    kind: &'static str,
}

impl Watchdog {
    fn new(kind: &'static str, seed: u64, iters: usize) -> Watchdog {
        let budget = Duration::from_millis(env_or("CHAOS_ITERATION_BUDGET_MS", ITERATION_BUDGET.as_millis() as u64));
        let slot: Arc<Mutex<Running>> = Arc::default();
        let weak = Arc::downgrade(&slot);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(100));
                let Some(slot) = weak.upgrade() else { return };
                let running = slot.lock().unwrap_or_else(|e| e.into_inner());
                let Some((i, started, input)) = &*running else { continue };
                if started.elapsed() > budget {
                    let why = format!("still running after {:.2?} (budget {budget:?})", started.elapsed());
                    let report = failure_report(kind, seed, iters, *i, input, &why);
                    // libtest captures `eprintln!` from this thread too, and the exit drops it.
                    let _ = std::io::stderr().lock().write_all(format!("{report}\n").as_bytes());
                    std::process::exit(101);
                }
            }
        });
        Watchdog {
            slot,
            slowest: Vec::new(),
            kind,
        }
    }

    /// Iteration `i` starts on `input`.
    fn start(&mut self, i: usize, input: &[Triple]) {
        let next = Some((i, Instant::now(), Arc::new(input.to_vec())));
        let last = std::mem::replace(&mut *self.slot.lock().unwrap_or_else(|e| e.into_inner()), next);
        self.time(last);
    }

    fn time(&mut self, last: Running) {
        if let Some((i, started, _)) = last {
            self.slowest.push((started.elapsed(), i));
            self.slowest.sort_by(|a, b| b.cmp(a));
            self.slowest.truncate(5);
        }
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        let last = self.slot.lock().unwrap_or_else(|e| e.into_inner()).take();
        self.time(last);
        let slowest: Vec<String> = self
            .slowest
            .iter()
            .map(|(took, i)| format!("{i} ({took:.2?})"))
            .collect();
        // Past libtest's capture, so a passing run's log has them too.
        say(&format!("{} slowest iterations: {}", self.kind, slowest.join(", ")));
    }
}

/// Runs `check` on every iteration of shard `k`: a returned error or a
/// panic fails the test with the replay line. An input that needs the
/// equality rules is counted and left out while [`EQUALITY_PENDING`].
fn run(kind: &'static str, default_seed: u64, k: usize, check: impl Fn(&[Triple], &mut Rng) -> Result<Done, String>) {
    let seed = env_or("CHAOS_SEED", default_seed);
    let iters = env_or("CHAOS_ITERS", 400usize);
    let mut watchdog = Watchdog::new(kind, seed, iters);
    let (mut compared, mut too_large, mut equality) = (0, 0, 0);
    for i in iterations(iters, k) {
        let (input, mut rng) = input_of(seed, i);
        watchdog.start(i, &input);
        let checked = if EQUALITY_PENDING && needs_equality(&input) {
            Ok(Ok(Done::NeedsEquality))
        } else {
            catch_unwind(AssertUnwindSafe(|| check(&input, &mut rng)))
        };
        match checked {
            Ok(Ok(Done::Compared)) => compared += 1,
            Ok(Ok(Done::TooLarge)) => too_large += 1,
            Ok(Ok(Done::NeedsEquality)) => equality += 1,
            Ok(Err(why)) => panic!("{}", failure_report(kind, seed, iters, i, &input, &why)),
            Err(payload) => panic!(
                "{}",
                failure_report(
                    kind,
                    seed,
                    iters,
                    i,
                    &input,
                    &format!("panicked: {}", panic_message(&*payload))
                )
            ),
        }
    }
    say(&format!(
        "{kind} shard {k}: {compared} compared; not compared: {too_large} too large for the reference, \
         {equality} need the equality rules (ticket 11)"
    ));
}

/// One full run equals the reference.
fn full(input: &[Triple], _rng: &mut Rng) -> Result<Done, String> {
    let base: HashSet<Triple> = input.iter().cloned().collect();
    let Some(expected) = reference(&base) else {
        return Ok(Done::TooLarge);
    };
    let mut reasoner = Reasoner::new();
    reasoner.insert(input.iter().map(Triple::as_ref));
    reasoner.reason();
    if let Some(diff) = difference(&expected, &core_state(&reasoner)) {
        return Err(format!("one full run:\n{diff}"));
    }
    Ok(Done::Compared)
}

/// Batches of insertions, then a removal, then the removed triples put
/// back, each followed by `reason()`, equal a full evaluation of the base.
fn incremental(input: &[Triple], rng: &mut Rng) -> Result<Done, String> {
    let base: HashSet<Triple> = input.iter().cloned().collect();
    let batches = 1 + rng.below(4);
    let mut split: Vec<Vec<&Triple>> = vec![Vec::new(); batches];
    for triple in input {
        split[rng.below(batches)].push(triple);
    }
    let removed: Vec<&Triple> = input.iter().filter(|_| rng.one_in(4)).collect();
    let kept: HashSet<Triple> = base.iter().filter(|t| !removed.contains(t)).cloned().collect();

    let (Some(whole), Some(after_removal)) = (reference(&base), reference(&kept)) else {
        return Ok(Done::TooLarge);
    };
    let mut reasoner = Reasoner::new();
    for batch in &split {
        reasoner.insert(batch.iter().map(|t| t.as_ref()));
        reasoner.reason();
    }
    if let Some(diff) = difference(&whole, &core_state(&reasoner)) {
        let sizes: Vec<usize> = split.iter().map(Vec::len).collect();
        return Err(format!("inserted in batches of {sizes:?}:\n{diff}"));
    }
    reasoner.remove(removed.iter().map(|t| t.as_ref()));
    reasoner.reason();
    if let Some(diff) = difference(&after_removal, &core_state(&reasoner)) {
        let lines: Vec<String> = removed.iter().map(|t| format!("    {t} .")).collect();
        return Err(format!("after removing:\n{}\n{diff}", lines.join("\n")));
    }
    reasoner.insert(removed.iter().map(|t| t.as_ref()));
    reasoner.reason();
    if let Some(diff) = difference(&whole, &core_state(&reasoner)) {
        return Err(format!("after putting the removed triples back:\n{diff}"));
    }
    Ok(Done::Compared)
}

const FULL_SEED: u64 = 0x5EED_0005;
const INCREMENTAL_SEED: u64 = 0x5EED_0105;

#[test]
fn full_runs_match_the_reference_0() {
    run("full", FULL_SEED, 0, full);
}

#[test]
fn full_runs_match_the_reference_1() {
    run("full", FULL_SEED, 1, full);
}

#[test]
fn incremental_runs_match_a_full_run_0() {
    run("incremental", INCREMENTAL_SEED, 0, incremental);
}

#[test]
fn incremental_runs_match_a_full_run_1() {
    run("incremental", INCREMENTAL_SEED, 1, incremental);
}

/// The equality terms in any position, and only those, leave an input out.
#[test]
fn needs_equality_looks_at_every_position() {
    let iri = |s: &str| oxrdf::NamedNode::new_unchecked(s);
    let (a, p) = (iri("http://example.org/a"), iri("http://example.org/p"));
    let triple = |s: &oxrdf::NamedNode, p: &oxrdf::NamedNode, o: Term| Triple::new(s.clone(), p.clone(), o);
    for term in EQUALITY_TERMS.map(iri) {
        assert!(needs_equality(&[triple(&term, &p, a.clone().into())]));
        assert!(needs_equality(&[triple(&a, &term, a.clone().into())]));
        assert!(needs_equality(&[triple(&a, &p, term.clone().into())]));
    }
    let literal = oxrdf::Literal::new_simple_literal(EQUALITY_TERMS[0]);
    assert!(!needs_equality(&[
        triple(&a, &p, literal.into()),
        triple(&a, &p, a.clone().into())
    ]));
}

/// The shards cover `0..iters` once, and CHAOS_ONLY picks one shard.
#[test]
fn shards_cover_every_iteration_once() {
    for iters in [0, 1, 7, 400] {
        let mut all: Vec<usize> = (0..SHARDS)
            .flat_map(|k| iters * k / SHARDS..iters * (k + 1) / SHARDS)
            .collect();
        all.sort_unstable();
        assert_eq!(all, (0..iters).collect::<Vec<_>>());
    }
}

/// An iteration past its budget: the child (this test binary, run again on
/// this test alone) starts an iteration that sleeps past a 100 ms budget;
/// the watchdog must name the iteration and the replay line, save the input
/// and exit 101.
#[test]
fn watchdog_reports_a_stuck_iteration() {
    if std::env::var_os("CHAOS_WATCHDOG_CHILD").is_some() {
        let mut watchdog = Watchdog::new("watchdog", 7, 2000);
        let (input, _) = input_of(7, 5);
        watchdog.start(5, &input);
        std::thread::sleep(Duration::from_secs(5));
        unreachable!("the watchdog exits first");
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "watchdog_reports_a_stuck_iteration",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("CHAOS_WATCHDOG_CHILD", "1")
        .env("CHAOS_ITERATION_BUDGET_MS", "100")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(101), "{stderr}");
    assert!(
        stderr.contains("CHAOS_SEED=7 CHAOS_ITERS=2000 CHAOS_ONLY=5"),
        "{stderr}"
    );
    assert!(stderr.contains("budget 100ms"), "{stderr}");
    let saved = chaos_dir().join("watchdog-seed7-iteration5.nt");
    let text = std::fs::read_to_string(&saved).unwrap();
    assert_eq!(text, to_ntriples(&input_of(7, 5).0));
    std::fs::remove_file(saved).unwrap();
}
