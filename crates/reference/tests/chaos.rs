//! Chaos: the core's `Reasoner` against the reference evaluator, on
//! generated inputs.
//!
//! Each iteration draws an input from the seed (`generate`, sizes 1 to 48)
//! and a configuration (ADR 0005): the default, the complete one, or a
//! random one (each switch on or off), about a third each, from a stream of
//! its own so that the input and the incremental test's splits stay as they
//! were. It runs the core's `Reasoner` with those switches and compares
//! closures and diagnostics, as sets, with
//! `turbowl_reference::evaluate_within` in the same configuration; each
//! run's `Summary` must count what the reads give. The full test runs the
//! core once; the incremental test inserts the input in random batches with
//! `reason()` between, then removes a random part and runs again, then puts
//! it back and runs again, and each state must equal a full evaluation of
//! its base.
//!
//! Every iteration runs the core twice, side by side: at one thread
//! (`Threads::ONE`) and at all cores with every round in parallel however
//! small (`Threads::ALL.every_round()`, so that these small inputs reach
//! the parallel rounds). After each `reason()` the two must agree exactly:
//! the summary, the closure and the diagnostics, in the order the reads
//! give them. The one-thread run is the one compared with the reference.
//!
//! Deterministic: CHAOS_SEED and CHAOS_ITERS override the seed and the
//! count; CHAOS_ONLY=<i> runs iteration i alone; CHAOS_SWITCHES runs every
//! iteration in one configuration (`default`, `complete`, `none`, or the
//! switches that are on, comma-separated: `equality,checks`). A failure
//! prints the line that replays it, its configuration included, and saves
//! the input as N-Triples under `target/chaos/`. An iteration past its
//! budget (CHAOS_ITERATION_BUDGET_MS) is reported the same way by a
//! watchdog, which then exits the process.
//!
//! Each iteration also explains its final state (the full test's run, the
//! incremental test's last state): up to 8 inferred triples and 8
//! diagnostics (CHAOS_EXPLAINED sets another count), drawn from a stream of
//! their own (as the configuration is), each proved with one explainer per
//! state and checked by the reference rules
//! (`turbowl_reference::check_proof`), in the iteration's own
//! configuration. A missing or wrong proof fails the iteration with its
//! replay line, and the all-cores reasoner's own explainer, asked the same
//! goals in the same order, must give the same proofs as the one-thread
//! reasoner's (a proof may depend on what an explainer's cache holds).
//!
//! Each iteration also cancels (CHAOS_CANCEL=0 turns it off, to measure
//! it): one of its runs is first cancelled at a check drawn from a stream of
//! its own, on both reasoners alike, which must leave no closure and no
//! diagnostics; the run that follows is compared as any other. The first
//! goal explained is first cancelled halfway too: the one-thread
//! explainer's proofs must still equal the all-cores explainer's, which was
//! never cancelled.
//!
//! Inputs whose reference closure would pass `MAX_CLOSURE` triples are
//! skipped and counted: the reference has no representatives, so a large
//! `owl:sameAs` clique is beyond it. Every other input is compared, those
//! with `owl:sameAs` (on vocabulary terms too) included. Each shard prints
//! how many inputs it compared and how many were too large.

use oxrdf::Triple;
use std::collections::{BTreeSet, HashSet};
use std::io::Write;
use std::ops::Range;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use turbowl::{Error, Reasoner, Summary, Switch, Switches, Threads, profile};
use turbowl_reference::{Diagnostics, Rng, check_proof, evaluate_within, generate, to_ntriples};

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

/// Mixed into the seed for the configurations' stream, so that drawing a
/// configuration leaves the inputs and the splits as they were.
const CONFIGURATIONS: u64 = 0xC0F1_6A7E_5717_C4E5;

/// The configuration of iteration `i`: CHAOS_SWITCHES when it is set,
/// otherwise drawn from the seed: the default, the complete one, or each
/// switch on or off at random, a third each.
fn configuration_of(seed: u64, i: usize) -> Switches {
    if let Ok(names) = std::env::var("CHAOS_SWITCHES") {
        return parse_switches(&names).unwrap_or_else(|e| panic!("CHAOS_SWITCHES={names}: {e}"));
    }
    let mut rng = Rng::at(seed ^ CONFIGURATIONS, i);
    match rng.below(3) {
        0 => Switches::default(),
        1 => Switches::complete(),
        _ => Switch::ALL
            .into_iter()
            .fold(Switches::complete(), |on, s| if rng.one_in(2) { on.off(s) } else { on }),
    }
}

/// A configuration as CHAOS_SWITCHES writes it: `default`, `complete`,
/// `none`, or the switches that are on, comma-separated.
fn switches_name(switches: Switches) -> String {
    let on: Vec<&str> = Switch::ALL
        .into_iter()
        .filter(|&s| switches.is_on(s))
        .map(Switch::name)
        .collect();
    if switches == Switches::default() {
        "default".to_owned()
    } else if switches == Switches::complete() {
        "complete".to_owned()
    } else if on.is_empty() {
        "none".to_owned()
    } else {
        on.join(",")
    }
}

/// The configuration [`switches_name`] wrote.
fn parse_switches(names: &str) -> Result<Switches, String> {
    let none = Switch::ALL.into_iter().fold(Switches::complete(), Switches::off);
    match names.trim() {
        "default" => Ok(Switches::default()),
        "complete" => Ok(Switches::complete()),
        "none" => Ok(none),
        list => list.split(',').try_fold(none, |on, name| {
            Switch::from_name(name.trim())
                .map(|s| on.on(s))
                .ok_or_else(|| format!("no switch is called {name:?}"))
        }),
    }
}

/// Mixed into the seed for the explanations' stream, so that drawing what to
/// explain leaves the inputs, the splits and the configurations as they were.
const EXPLANATIONS: u64 = 0xE8A1_A1E5_0F5E_ED05;

/// The cancels leave the inputs, the splits, the configurations and the
/// explained goals as they were.
const CANCELS: u64 = 0xCA9C_E15E_0F5E_ED05;

/// The cancels of one iteration: the run cancelled first (its index among
/// the iteration's runs) and the checks at which its run and its first
/// search stop. `None` with CHAOS_CANCEL=0.
#[derive(Debug, Clone, Copy)]
struct Cancels {
    run: usize,
    run_check: usize,
    search_check: usize,
}

impl Cancels {
    fn of(seed: u64, i: usize, runs: usize) -> Option<Cancels> {
        if env_or("CHAOS_CANCEL", 1u8) == 0 {
            return None;
        }
        let mut rng = Rng::at(seed ^ CANCELS, i);
        // Early checks (most runs take a few rounds) and late ones: below
        // `small`, or below 8 times it, one time in two each.
        fn check(rng: &mut Rng, small: usize) -> usize {
            let below = if rng.one_in(2) { small } else { small * 8 };
            rng.below(below)
        }
        let run = rng.below(runs);
        Some(Cancels {
            run,
            run_check: check(&mut rng, 8),
            search_check: check(&mut rng, 512),
        })
    }
}

/// The most inferred triples, and the most diagnostics, a state explains
/// (CHAOS_EXPLAINED overrides it).
const EXPLAINED: usize = 8;

/// Up to `n` of `0..len`, drawn from `rng`, in increasing order.
fn sample(rng: &mut Rng, len: usize, n: usize) -> Vec<usize> {
    let mut all: Vec<usize> = (0..len).collect();
    for k in 0..n.min(len) {
        let j = k + rng.below(len - k);
        all.swap(k, j);
    }
    all.truncate(n);
    all.sort_unstable();
    all
}

/// Explains up to [`EXPLAINED`] inferred triples and diagnostics of the
/// final state of `twins`, whose base is `input`, and checks each proof;
/// the all-cores reasoner's own explainer, asked in the same order, must
/// give the same proofs. Returns how many proofs were checked.
fn explain(
    input: &[Triple],
    twins: &Twins,
    switches: Switches,
    rng: &mut Rng,
    cancels: Option<Cancels>,
) -> Result<usize, String> {
    let (one, all) = (&twins.one, &twins.all);
    let base: HashSet<Triple> = input.iter().cloned().collect();
    let one_explainer = one.explainer().map_err(|e| e.to_string())?;
    let all_explainer = all.explainer().map_err(|e| e.to_string())?;
    let inferred: Vec<Triple> = one.inferred().collect();
    let diagnostics = one.diagnostics();
    let n = env_or("CHAOS_EXPLAINED", EXPLAINED);
    let triples = sample(rng, inferred.len(), n);
    let found = sample(rng, diagnostics.len(), n);
    let mut checked = 0;
    // The first goal (a triple's, or with none a diagnostic's), cancelled
    // halfway: the search graph is dropped, back to the empty one the first
    // goal starts from.
    if let Some(c) = cancels {
        let cancel = profile::cancel_after(c.search_check);
        let searched = match (triples.first(), found.first()) {
            (Some(&i), _) => one_explainer.explain_cancellable(inferred[i].as_ref(), &cancel),
            (None, Some(&i)) => one_explainer.explain_diagnostic_cancellable(&diagnostics[i], &cancel),
            (None, None) => Ok(None),
        };
        match searched {
            Ok(_) | Err(Error::Cancelled) => {}
            Err(e) => return Err(format!("a cancelled search: {e}")),
        }
    }
    for i in triples {
        let t = &inferred[i];
        let Some(proof) = one_explainer.explain(t.as_ref()) else {
            return Err(format!("no proof of the inferred {t}"));
        };
        check_proof(&proof, &base, switches).map_err(|e| format!("the proof of {t}: {e}"))?;
        if all_explainer.explain(t.as_ref()).as_ref() != Some(&proof) {
            return Err(format!("the proof of {t} differs between one thread and all cores"));
        }
        checked += 1;
    }
    for i in found {
        let d = &diagnostics[i];
        let Some(proof) = one_explainer.explain_diagnostic(d) else {
            return Err(format!("no proof of the diagnostic {} {:?}", d.code, d.terms));
        };
        check_proof(&proof, &base, switches)
            .map_err(|e| format!("the proof of the diagnostic {} {:?}: {e}", d.code, d.terms))?;
        if all_explainer.explain_diagnostic(d).as_ref() != Some(&proof) {
            return Err(format!(
                "the proof of {} {:?} differs between one thread and all cores",
                d.code, d.terms
            ));
        }
        checked += 1;
    }
    Ok(checked)
}

/// What one iteration did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Done {
    /// Compared.
    Compared { proofs: usize, cancelled: usize },
    /// The reference closure would pass `MAX_CLOSURE`.
    TooLarge,
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

/// What the reference says in the configuration, or `None` when the input
/// is too large for it.
fn reference(base: &HashSet<Triple>, switches: Switches) -> Option<(HashSet<Triple>, Diagnostics)> {
    evaluate_within(base, switches, MAX_CLOSURE).map(|e| (e.closure, e.diagnostics))
}

/// Runs the reasoner, and checks that its summary counts what its reads
/// give (with `owl:sameAs`, the expanded closure: ADR 0003).
fn reason(reasoner: &mut Reasoner) -> Result<Summary, String> {
    let s = reasoner.reason();
    let (triples, inferred) = (reasoner.triples().count(), reasoner.inferred().count());
    if (s.base + s.inferred, s.inferred, s.diagnostics) != (triples, inferred, reasoner.diagnostics().len()) {
        return Err(format!(
            "the summary {s:?} does not count the reads: {triples} triples, {inferred} inferred, {} diagnostics",
            reasoner.diagnostics().len()
        ));
    }
    Ok(s)
}

/// The core at one thread and at all cores, every round in parallel,
/// driven alike.
struct Twins {
    one: Reasoner,
    all: Reasoner,
}

impl Twins {
    fn new(switches: Switches) -> Twins {
        let mut one = Reasoner::with_switches(switches);
        one.set_threads(Threads::ONE);
        let mut all = Reasoner::with_switches(switches);
        all.set_threads(Threads::ALL.every_round());
        Twins { one, all }
    }

    fn insert(&mut self, triples: &[&Triple]) {
        for r in [&mut self.one, &mut self.all] {
            r.insert(triples.iter().map(|t| t.as_ref()));
        }
    }

    fn remove(&mut self, triples: &[&Triple]) {
        for r in [&mut self.one, &mut self.all] {
            r.remove(triples.iter().map(|t| t.as_ref()));
        }
    }

    /// Runs both with a handle that stops them at check `k`: a cancelled run
    /// leaves no closure and no diagnostics, and both must stop alike.
    /// Returns 1 when they stopped, 0 when they ran to the end.
    fn cancel_at(&mut self, k: usize) -> Result<usize, String> {
        let mut stopped = Vec::new();
        for r in [&mut self.one, &mut self.all] {
            match r.reason_cancellable(&profile::cancel_after(k)) {
                Ok(_) => stopped.push(false),
                Err(Error::Cancelled) => {
                    if r.triples().next().is_some() || !r.diagnostics().is_empty() {
                        return Err(format!("a run cancelled at check {k} left a partial result"));
                    }
                    stopped.push(true);
                }
                Err(e) => return Err(format!("a run cancelled at check {k}: {e}")),
            }
        }
        if stopped[0] != stopped[1] {
            return Err(format!(
                "at check {k}, one thread stopped: {}, all cores: {}",
                stopped[0], stopped[1]
            ));
        }
        Ok(usize::from(stopped[0]))
    }

    /// Runs both, and checks that they agree exactly: the summary, then the
    /// closure and the diagnostics in the order the reads give them.
    fn reason(&mut self) -> Result<(), String> {
        let (one, all) = (reason(&mut self.one)?, reason(&mut self.all)?);
        if one != all {
            return Err(format!("one thread gives {one:?}, all cores {all:?}"));
        }
        let (a, b): (Vec<Triple>, Vec<Triple>) = (self.one.triples().collect(), self.all.triples().collect());
        if a != b {
            let at = a
                .iter()
                .zip(&b)
                .position(|(x, y)| x != y)
                .unwrap_or(a.len().min(b.len()));
            return Err(format!(
                "the closure differs between one thread and all cores from triple {at}: {:?} against {:?}",
                a.get(at).map(ToString::to_string),
                b.get(at).map(ToString::to_string)
            ));
        }
        if self.one.diagnostics() != self.all.diagnostics() {
            return Err(format!(
                "the diagnostics differ between one thread and all cores: {:?} against {:?}",
                self.one.diagnostics(),
                self.all.diagnostics()
            ));
        }
        Ok(())
    }
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

/// The iteration a report is about.
#[derive(Debug, Clone, Copy)]
struct Iteration {
    seed: u64,
    iters: usize,
    i: usize,
    switches: Switches,
}

/// Saves the input and returns the report: what failed, the replay line
/// (with the configuration), and where the input is.
fn failure_report(kind: &str, at: Iteration, input: &[Triple], why: &str) -> String {
    let Iteration {
        seed,
        iters,
        i,
        switches,
    } = at;
    let dir = chaos_dir();
    let path = dir.join(format!("{kind}-seed{seed}-iteration{i}.nt"));
    let saved = std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&path, to_ntriples(input)))
        .map_or_else(
            |e| format!("not saved ({e})"),
            |()| format!("saved to {}", path.display()),
        );
    format!(
        "{kind} iteration {i} (seed {seed}, switches {switches:?}; replay with CHAOS_SEED={seed} \
         CHAOS_ITERS={iters} CHAOS_ONLY={i} CHAOS_SWITCHES={}) failed: {why}\n  input: {} triples, {saved}",
        switches_name(switches),
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

/// The iteration a shard is running: which, its start and its input.
type Running = Option<(Iteration, Instant, Arc<Vec<Triple>>)>;

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
    fn new(kind: &'static str) -> Watchdog {
        let budget = Duration::from_millis(env_or("CHAOS_ITERATION_BUDGET_MS", ITERATION_BUDGET.as_millis() as u64));
        let slot: Arc<Mutex<Running>> = Arc::default();
        let weak = Arc::downgrade(&slot);
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(100));
                let Some(slot) = weak.upgrade() else { return };
                let running = slot.lock().unwrap_or_else(|e| e.into_inner());
                let Some((at, started, input)) = &*running else {
                    continue;
                };
                if started.elapsed() > budget {
                    let why = format!("still running after {:.2?} (budget {budget:?})", started.elapsed());
                    let report = failure_report(kind, *at, input, &why);
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

    /// Iteration `at` starts on `input`.
    fn start(&mut self, at: Iteration, input: &[Triple]) {
        let next = Some((at, Instant::now(), Arc::new(input.to_vec())));
        let last = std::mem::replace(&mut *self.slot.lock().unwrap_or_else(|e| e.into_inner()), next);
        self.time(last);
    }

    fn time(&mut self, last: Running) {
        if let Some((at, started, _)) = last {
            self.slowest.push((started.elapsed(), at.i));
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

/// Runs `check` on every iteration of shard `k`, in the iteration's
/// configuration: a returned error or a panic fails the test with the
/// replay line.
fn run(
    kind: &'static str,
    default_seed: u64,
    k: usize,
    check: impl Fn(&[Triple], Switches, &mut Rng, &mut Rng, &dyn Fn(usize) -> Option<Cancels>) -> Result<Done, String>,
) {
    let seed = env_or("CHAOS_SEED", default_seed);
    let iters = env_or("CHAOS_ITERS", 400usize);
    let mut watchdog = Watchdog::new(kind);
    // Compared iterations by configuration: default, complete or random.
    let (mut too_large, mut proofs, mut cancelled) = (0, 0, 0);
    let mut compared = std::collections::BTreeMap::<String, usize>::new();
    for i in iterations(iters, k) {
        let (input, mut rng) = input_of(seed, i);
        let switches = configuration_of(seed, i);
        let at = Iteration {
            seed,
            iters,
            i,
            switches,
        };
        watchdog.start(at, &input);
        let mut explanations = Rng::at(seed ^ EXPLANATIONS, i);
        let cancels = |runs: usize| Cancels::of(seed, i, runs);
        match catch_unwind(AssertUnwindSafe(|| {
            check(&input, switches, &mut rng, &mut explanations, &cancels)
        })) {
            Ok(Ok(Done::Compared {
                proofs: checked,
                cancelled: stopped,
            })) => {
                proofs += checked;
                cancelled += stopped;
                let name = switches_name(switches);
                let label = match name.as_str() {
                    "default" | "complete" => name,
                    _ => "random".to_owned(),
                };
                *compared.entry(label).or_default() += 1;
            }
            Ok(Ok(Done::TooLarge)) => too_large += 1,
            Ok(Err(why)) => panic!("{}", failure_report(kind, at, &input, &why)),
            Err(payload) => panic!(
                "{}",
                failure_report(kind, at, &input, &format!("panicked: {}", panic_message(&*payload)))
            ),
        }
    }
    let total: usize = compared.values().sum();
    say(&format!(
        "{kind} shard {k}: {total} compared ({compared:?}), {proofs} proofs checked, {cancelled} runs cancelled; not compared: {too_large} too large for the reference"
    ));
}

/// One full run equals the reference, and its proofs are valid.
fn full(
    input: &[Triple],
    switches: Switches,
    _rng: &mut Rng,
    explanations: &mut Rng,
    cancels: &dyn Fn(usize) -> Option<Cancels>,
) -> Result<Done, String> {
    let base: HashSet<Triple> = input.iter().cloned().collect();
    let Some(expected) = reference(&base, switches) else {
        return Ok(Done::TooLarge);
    };
    let cancels = cancels(1);
    let mut twins = Twins::new(switches);
    twins.insert(&input.iter().collect::<Vec<_>>());
    let stopped = match cancels {
        Some(c) => twins.cancel_at(c.run_check)?,
        None => 0,
    };
    twins.reason()?;
    if let Some(diff) = difference(&expected, &core_state(&twins.one)) {
        return Err(format!("one full run:\n{diff}"));
    }
    let proofs = explain(input, &twins, switches, explanations, cancels)?;
    Ok(Done::Compared {
        proofs,
        cancelled: stopped,
    })
}

/// Batches of insertions, then a removal, then the removed triples put
/// back, each followed by `reason()`, equal a full evaluation of the base;
/// the last state's proofs are valid.
fn incremental(
    input: &[Triple],
    switches: Switches,
    rng: &mut Rng,
    explanations: &mut Rng,
    cancels: &dyn Fn(usize) -> Option<Cancels>,
) -> Result<Done, String> {
    let base: HashSet<Triple> = input.iter().cloned().collect();
    let batches = 1 + rng.below(4);
    let mut split: Vec<Vec<&Triple>> = vec![Vec::new(); batches];
    for triple in input {
        split[rng.below(batches)].push(triple);
    }
    let removed: Vec<&Triple> = input.iter().filter(|_| rng.one_in(4)).collect();
    let kept: HashSet<Triple> = base.iter().filter(|t| !removed.contains(t)).cloned().collect();

    let (Some(whole), Some(after_removal)) = (reference(&base, switches), reference(&kept, switches)) else {
        return Ok(Done::TooLarge);
    };
    // The runs: one per batch, after the removal, after putting it back.
    let cancels = cancels(batches + 2);
    let cancel = |twins: &mut Twins, run: usize| match cancels {
        Some(c) if c.run == run => twins.cancel_at(c.run_check),
        _ => Ok(0),
    };
    let mut stopped = 0;
    let mut twins = Twins::new(switches);
    for (run, batch) in split.iter().enumerate() {
        twins.insert(batch);
        stopped += cancel(&mut twins, run)?;
        twins.reason()?;
    }
    if let Some(diff) = difference(&whole, &core_state(&twins.one)) {
        let sizes: Vec<usize> = split.iter().map(Vec::len).collect();
        return Err(format!("inserted in batches of {sizes:?}:\n{diff}"));
    }
    twins.remove(&removed);
    stopped += cancel(&mut twins, batches)?;
    twins.reason()?;
    if let Some(diff) = difference(&after_removal, &core_state(&twins.one)) {
        let lines: Vec<String> = removed.iter().map(|t| format!("    {t} .")).collect();
        return Err(format!("after removing:\n{}\n{diff}", lines.join("\n")));
    }
    twins.insert(&removed);
    stopped += cancel(&mut twins, batches + 1)?;
    twins.reason()?;
    if let Some(diff) = difference(&whole, &core_state(&twins.one)) {
        return Err(format!("after putting the removed triples back:\n{diff}"));
    }
    let proofs = explain(input, &twins, switches, explanations, cancels)?;
    Ok(Done::Compared {
        proofs,
        cancelled: stopped,
    })
}

const FULL_SEED: u64 = 0x5EED_0005;
const INCREMENTAL_SEED: u64 = 0x5EED_0105;
const REMOVAL_SEED: u64 = 0x5EED_0205;

fn removals(
    input: &[Triple],
    switches: Switches,
    rng: &mut Rng,
    explanations: &mut Rng,
    cancels: &dyn Fn(usize) -> Option<Cancels>,
) -> Result<Done, String> {
    let mut base: HashSet<Triple> = input.iter().cloned().collect();
    let Some(whole) = reference(&base, switches) else {
        return Ok(Done::TooLarge);
    };
    let mut twins = Twins::new(switches);
    twins.insert(&input.iter().collect::<Vec<_>>());
    twins.reason()?;
    if let Some(diff) = difference(&whole, &core_state(&twins.one)) {
        return Err(diff);
    }
    let cancel = cancels(9);
    let mut stopped = 0;
    for step in 0..8 {
        let mut ordered: Vec<_> = base.iter().cloned().collect();
        ordered.sort_by_cached_key(ToString::to_string);
        let mut gone = Vec::new();
        for _ in 0..1 + rng.below(4) {
            if ordered.is_empty() {
                break;
            }
            gone.push(ordered.remove(rng.below(ordered.len())));
        }
        twins.remove(&gone.iter().collect::<Vec<_>>());
        for t in &gone {
            base.remove(t);
        }
        if let Some(c) = cancel.filter(|c| c.run == step + 1) {
            stopped += twins.cancel_at(c.run_check)?;
        }
        twins.reason()?;
        let Some(expected) = reference(&base, switches) else {
            return Ok(Done::TooLarge);
        };
        if let Some(diff) = difference(&expected, &core_state(&twins.one)) {
            return Err(format!("removal step {step}, removed {gone:?}:\n{diff}"));
        }
    }
    let mut final_input: Vec<_> = base.into_iter().collect();
    final_input.sort_by_cached_key(ToString::to_string);
    let proofs = explain(&final_input, &twins, switches, explanations, cancel)?;
    Ok(Done::Compared {
        proofs,
        cancelled: stopped,
    })
}

#[test]
fn removal_runs_match_a_full_run_0() {
    run("removal", REMOVAL_SEED, 0, removals);
}

#[test]
fn removal_runs_match_a_full_run_1() {
    run("removal", REMOVAL_SEED, 1, removals);
}

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

/// The configurations: drawn from the seed, the default, the complete one
/// and random ones each about a third of the time over 300 iterations;
/// every name [`switches_name`] writes reads back as the same switches.
#[test]
fn configurations_are_drawn_from_the_seed_and_replay() {
    if std::env::var_os("CHAOS_SWITCHES").is_some() {
        return;
    }
    let (mut default, mut complete, mut other) = (0, 0, 0);
    for i in 0..300 {
        let switches = configuration_of(FULL_SEED, i);
        assert_eq!(
            switches,
            configuration_of(FULL_SEED, i),
            "the same seed gives the same configuration"
        );
        match switches_name(switches).as_str() {
            "default" => default += 1,
            "complete" => complete += 1,
            _ => other += 1,
        }
        assert_eq!(parse_switches(&switches_name(switches)), Ok(switches));
    }
    for count in [default, complete, other] {
        assert!(
            (70..=130).contains(&count),
            "{default} default, {complete} complete, {other} other"
        );
    }
    let none = Switch::ALL.into_iter().fold(Switches::complete(), Switches::off);
    assert_eq!(parse_switches("none"), Ok(none));
    assert_eq!(
        parse_switches("equality, checks"),
        Ok(none.on(Switch::Equality).on(Switch::Checks))
    );
    assert!(parse_switches("equality,unknown").is_err());
}

/// An iteration past its budget: the child (this test binary, run again on
/// this test alone) starts an iteration that sleeps past a 100 ms budget;
/// the watchdog must name the iteration and the replay line with its
/// configuration, save the input and exit 101.
#[test]
fn watchdog_reports_a_stuck_iteration() {
    if std::env::var_os("CHAOS_WATCHDOG_CHILD").is_some() {
        let mut watchdog = Watchdog::new("watchdog");
        let (input, _) = input_of(7, 5);
        let switches = Switches::default().off(Switch::Equality);
        watchdog.start(
            Iteration {
                seed: 7,
                iters: 2000,
                i: 5,
                switches,
            },
            &input,
        );
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
        stderr.contains("CHAOS_SEED=7 CHAOS_ITERS=2000 CHAOS_ONLY=5 CHAOS_SWITCHES=property-chains,checks"),
        "{stderr}"
    );
    assert!(stderr.contains("budget 100ms"), "{stderr}");
    let saved = chaos_dir().join("watchdog-seed7-iteration5.nt");
    let text = std::fs::read_to_string(&saved).unwrap();
    assert_eq!(text, to_ntriples(&input_of(7, 5).0));
    std::fs::remove_file(saved).unwrap();
}
