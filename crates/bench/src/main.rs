//! The benchmark: runs each scenario for this repository's reasoner and for
//! its competitors Nemo, Soufflé and VLog (pinned tools, each a subprocess,
//! on the scenario's inputs converted once), in one configuration of the
//! rules and one thread mode for this reasoner (`--threads`), and reports
//! median, spread, peak memory, closure size and each engine's ratio to
//! this reasoner; each competitor's closure must equal this reasoner's. `--scoreboard` reads results files and prints each
//! engine's value and the leader per scenario and axis, with RDFox's
//! published results scaled through a bridge where they allow it
//! (`published.rs`, `report.rs`). See `README.md`.
//!
//! Every run is a fresh child process (this binary with the hidden `--one`
//! mode), so that peak memory is one engine's on one input. The parent
//! watches the memory each run holds and stops it past the memory cap.

mod compare;
mod convert;
mod datasets;
mod engine;
mod lubm;
mod machine;
mod nemo;
mod profile;
mod published;
mod report;
mod scaling;
mod scenario;
mod souffle;
mod vlog;

use engine::{EngineId, Mode, ThreadMode};
use scenario::Scenario;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::time::{Duration, Instant};
use turbowl::{Switch, Switches};

/// The results file's format version.
const SCHEMA: u64 = 9;
/// The versions `--compare` reads: 2 only adds to 1 (and renames engines),
/// 3 only adds the `scaling` table to 2, 4 only renames this reasoner
/// (`turtle-reasoner` became `turbowl`; [`read_results`] reads the old name
/// and skips the rows of engines this bench no longer runs), 5 only adds the
/// status `over-memory` and `meta.memory_cap_bytes`, 6 adds the engines
/// `souffle` and `vlog`, `meta.configuration` and `meta.peak_memory`, a run's
/// `total_ms` (an engine that does not split loading from reasoning), and
/// the peak memory becomes the peak physical footprint on macOS, 7 adds
/// `meta.switches`, `meta.threads` and `published_counts`, and
/// `meta.configuration` can name switches turned from the default, 8 only
/// adds this reasoner's counters and memory breakdown (`profile` in its runs
/// and summaries), 9 records the thread mode (`--threads`) in `meta.threads`
/// (0 for all cores) and in each of this reasoner's runs (`threads`).
/// Before 9, `meta.threads` is 1, and it is read as one thread.
const READS: [u64; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];
/// This reasoner's engine name in results files before version 4.
const OLD_NAME: &str = "turtle-reasoner";

/// Extra time the parent gives a child over `--timeout` before killing it:
/// the child stops a competitor itself at the timeout, and the competitor
/// must not outlive it.
const GRACE: Duration = Duration::from_secs(30);

/// A gigabyte as `--memory-cap` counts it: 2^30 bytes, as the bench's MB is 2^20.
const GB: f64 = (1u64 << 30) as f64;

const USAGE: &str = "\
usage: turbowl-bench [options]
       turbowl-bench --compare OLD.json NEW.json
       turbowl-bench --scoreboard RESULTS.json...

  --list              print the scenarios and the engines, and stop
  --only A,B          run only these scenarios (opt-in ones run only this way)
  --engines A,B       run only these engines (turbowl, nemo, souffle, vlog)
  --complete          start from every switch on, the complete W3C rule set
                      (default: the default switches)
  --switch NAME=on|off  turn one switch on or off, after --complete;
                      repeatable, a later one for the same name wins
  --threads one|all   this reasoner's threads: one, or every core (default
                      one); the competitors run as they always do
  --runs N            runs per scenario and engine (default 5)
  --changes N         changes in the incremental scenarios (default 100)
  --timeout SECS      stop a run after this long (default 600)
  --memory-cap GB     stop a run whose memory passes this (default 16)
  --data-dir DIR      fetched models (default: data/ at the repository root)
  --json FILE         also write the results to FILE
  --compare OLD NEW   print per-scenario deltas between two results files
  --scoreboard FILE...  print each engine's value and the leader per scenario
                      and axis, and the estimates from published results

Run on a quiet machine: load average under 1, nothing building or testing.";

#[derive(Debug)]
struct Args {
    list: bool,
    only: Option<Vec<String>>,
    engines: Vec<EngineId>,
    /// The configuration: the default switches, or every switch on
    /// (`--complete`), then each `--switch` in order.
    switches: Switches,
    /// This reasoner's threads (`--threads`).
    threads: ThreadMode,
    runs: usize,
    changes: usize,
    timeout: Duration,
    /// In bytes.
    memory_cap: u64,
    data_dir: Option<PathBuf>,
    json: Option<PathBuf>,
    compare: Option<(PathBuf, PathBuf)>,
    scoreboard: Option<Vec<PathBuf>>,
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut a = Args {
        list: false,
        only: None,
        engines: EngineId::ALL.to_vec(),
        switches: Switches::default(),
        threads: ThreadMode::One,
        runs: 5,
        changes: 100,
        timeout: Duration::from_secs(600),
        memory_cap: 16 << 30,
        data_dir: None,
        json: None,
        compare: None,
        scoreboard: None,
    };
    let (mut complete, mut settings) = (false, Vec::new());
    let mut it = argv.iter().peekable();
    while let Some(arg) = it.next() {
        let mut value = || it.next().cloned().ok_or_else(|| format!("{arg} needs a value"));
        let number = |v: String| {
            v.parse::<usize>()
                .ok()
                .filter(|&n| n > 0)
                .ok_or(format!("{arg}: not a positive number: {v}"))
        };
        match arg.as_str() {
            "--list" => a.list = true,
            "--only" => a.only = Some(value()?.split(',').map(str::to_string).collect()),
            "--engines" => {
                a.engines = value()?
                    .split(',')
                    .map(|e| EngineId::from_name(e).ok_or(format!("unknown engine {e}")))
                    .collect::<Result<_, _>>()?
            }
            "--complete" => complete = true,
            "--switch" => settings.push(switch_setting(&value()?)?),
            "--threads" => {
                let v = value()?;
                a.threads = ThreadMode::from_name(&v).ok_or(format!("{arg} takes one or all, not {v}"))?;
            }
            "--runs" => a.runs = number(value()?)?,
            "--changes" => a.changes = number(value()?)?,
            "--timeout" => a.timeout = Duration::from_secs(number(value()?)? as u64),
            "--memory-cap" => {
                let v = value()?;
                let gb = v
                    .parse::<f64>()
                    .ok()
                    .filter(|g| g.is_finite() && *g > 0.0)
                    .ok_or(format!("{arg}: not a positive number of GB: {v}"))?;
                a.memory_cap = (gb * GB) as u64;
            }
            "--data-dir" => a.data_dir = Some(value()?.into()),
            "--json" => a.json = Some(value()?.into()),
            "--compare" => a.compare = Some((value()?.into(), value()?.into())),
            "--scoreboard" => {
                let mut files = Vec::new();
                while let Some(file) = it.next_if(|f| !f.starts_with("--")) {
                    files.push(file.into());
                }
                if files.is_empty() {
                    return Err(format!("{arg} needs a results file"));
                }
                a.scoreboard = Some(files);
            }
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    // --complete is a starting point wherever it comes, as in the CLI.
    a.switches = settings.into_iter().fold(
        if complete {
            Switches::complete()
        } else {
            Switches::default()
        },
        |s, (switch, on)| {
            if on { s.on(switch) } else { s.off(switch) }
        },
    );
    if let Some(only) = &a.only {
        for name in only {
            if scenario::find(name).is_none() {
                return Err(format!("unknown scenario {name}; --list shows them"));
            }
        }
    }
    Ok(a)
}

/// A `--switch` value: `NAME=on` or `NAME=off`, as the CLI takes it.
fn switch_setting(value: &str) -> Result<(Switch, bool), String> {
    let (name, on) = match value.split_once('=') {
        Some((name, "on")) => (name, true),
        Some((name, "off")) => (name, false),
        _ => return Err(format!("--switch takes NAME=on or NAME=off, not {value}")),
    };
    let switch = Switch::from_name(name).ok_or_else(|| {
        let names: Vec<&str> = Switch::ALL.into_iter().map(Switch::name).collect();
        format!("unknown switch {name} (the switches: {})", names.join(", "))
    })?;
    Ok((switch, on))
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.first().is_some_and(|a| a == "--one") {
        return child(&argv[1..]);
    }
    let args = match parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}\n");
            }
            eprintln!("{USAGE}");
            return ExitCode::from(if e.is_empty() { 0 } else { 64 });
        }
    };
    if args.list {
        for s in scenario::all() {
            let opt_in = if s.opt_in { " [opt-in: --only]" } else { "" };
            println!("{:<20} {}{opt_in}", s.name, s.about);
            if let Some(published) = s.published() {
                println!("{:<20} papers: {published}", "");
            }
        }
        println!("\nengines:");
        for e in EngineId::ALL {
            let Some(tool) = e.tool() else {
                println!("  {:<10} this build, in process", e.name());
                continue;
            };
            let fetched = match tool.path() {
                Ok(path) => format!("fetched: {}", path.display()),
                Err(why) => format!("not fetched ({why})"),
            };
            println!("  {:<10} {:<8} {fetched}", e.name(), tool.version);
        }
        println!("  graphdb    not run: GraphDB Free needs a licence by e-mail, which the owner has not obtained");
        return ExitCode::SUCCESS;
    }
    if let Some((old, new)) = &args.compare {
        return compare::run(old, new);
    }
    if let Some(files) = &args.scoreboard {
        return report::run(files);
    }
    run(&args)
}

/// The hidden `--one` mode's arguments, after `--one`.
const CHILD_USAGE: &str = "--one ENGINE MODE SWITCHES THREADS CHANGES TIMEOUT_S FILE...";

/// One run, as the parent hands it to a child.
#[derive(Debug, PartialEq)]
struct OneRun {
    engine: EngineId,
    mode: Mode,
    switches: Switches,
    threads: ThreadMode,
    changes: usize,
    timeout: Duration,
    files: Vec<PathBuf>,
}

impl OneRun {
    /// The child's arguments, `--one` first: SWITCHES names the switches
    /// that are on, separated by commas (`none` for none); THREADS is `one`
    /// or `all` (`--threads`).
    fn to_args(&self) -> Vec<OsString> {
        let switches = match switch_names(self.switches).join(",") {
            names if names.is_empty() => "none".into(),
            names => names,
        };
        let mut out: Vec<OsString> = [
            "--one",
            self.engine.name(),
            self.mode.name(),
            &switches,
            self.threads.name(),
            &self.changes.to_string(),
            &self.timeout.as_secs().to_string(),
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        out.extend(self.files.iter().map(|f| f.clone().into_os_string()));
        out
    }

    /// The arguments after `--one`, as [`OneRun::to_args`] writes them.
    fn from_args(argv: &[String]) -> Option<OneRun> {
        Some(OneRun {
            engine: EngineId::from_name(argv.first()?)?,
            mode: Mode::from_name(argv.get(1)?)?,
            switches: switches_from_names(argv.get(2)?)?,
            threads: ThreadMode::from_name(argv.get(3)?)?,
            changes: argv.get(4)?.parse().ok()?,
            timeout: Duration::from_secs(argv.get(5)?.parse().ok()?),
            files: argv[6..].iter().map(PathBuf::from).collect(),
        })
    }
}

/// The hidden `--one` mode ([`CHILD_USAGE`]): one measured run, one JSON
/// line on stdout.
fn child(argv: &[String]) -> ExitCode {
    let Some(r) = OneRun::from_args(argv) else {
        eprintln!("{CHILD_USAGE}");
        return ExitCode::from(64);
    };
    println!(
        "{}",
        engine::run_one(r.engine, r.mode, r.switches, r.threads, &r.files, r.changes, r.timeout)
    );
    ExitCode::SUCCESS
}

/// The names of the switches that are on, as the results file lists them.
fn switch_names(switches: Switches) -> Vec<&'static str> {
    Switch::ALL
        .into_iter()
        .filter(|&s| switches.is_on(s))
        .map(Switch::name)
        .collect()
}

/// The switches named, comma-separated (`none` for none).
fn switches_from_names(names: &str) -> Option<Switches> {
    let none = Switch::ALL.into_iter().fold(Switches::complete(), Switches::off);
    if names == "none" {
        return Some(none);
    }
    names
        .split(',')
        .try_fold(none, |s, name| Switch::from_name(name).map(|switch| s.on(switch)))
}

/// The configuration's name, as the results file spells it: `default`,
/// `complete`, or `default` with the switches turned from it
/// (`default, equality off`).
pub(crate) fn configuration(switches: Switches) -> String {
    if switches == Switches::complete() {
        return "complete".into();
    }
    let changed: Vec<String> = Switch::ALL
        .into_iter()
        .filter(|&s| switches.is_on(s) != Switches::default().is_on(s))
        .map(|s| format!("{} {}", s.name(), if switches.is_on(s) { "on" } else { "off" }))
        .collect();
    if changed.is_empty() {
        "default".into()
    } else {
        format!("default, {}", changed.join(", "))
    }
}

pub(crate) fn repo_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    root.canonicalize().unwrap_or(root)
}

/// Spawns one run and waits for it, at most `timeout` (plus a grace period:
/// a child running a competitor stops it at `timeout` and reports the
/// timeout) and while the memory it holds stays within `cap` bytes.
fn spawn_run(engine: EngineId, mode: Mode, args: &Args, files: &[PathBuf]) -> Value {
    let (timeout, cap) = (args.timeout, args.memory_cap);
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => return json!({ "status": "error", "message": format!("current_exe: {e}") }),
    };
    let one = OneRun {
        engine,
        mode,
        switches: args.switches,
        threads: args.threads,
        changes: args.changes,
        timeout,
        files: files.to_vec(),
    };
    let mut child = match Command::new(exe)
        .args(one.to_args())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return json!({ "status": "error", "message": format!("spawn: {e}") }),
    };
    let status = match watch(&mut child, timeout, cap) {
        Ok(status) => status,
        Err(stopped) => return stopped,
    };
    let mut out = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read_to_string(&mut out);
    }
    let line = out.lines().last().unwrap_or("");
    match serde_json::from_str::<Value>(line) {
        Ok(v) if v.get("status").is_some() => v,
        _ => json!({
            "status": "error",
            "message": format!("the run exited with {status} and wrote no result (out of memory?)"),
        }),
    }
}

/// Waits for a run to exit. It stops the run, with every process the run
/// started (a competitor), when it outlasts `timeout` plus the grace period
/// (status `timeout`) or when the memory they hold together passes `cap`
/// bytes (status `over-memory`): both are results, not problems, and neither
/// counts a time.
fn watch(child: &mut Child, timeout: Duration, cap: u64) -> Result<std::process::ExitStatus, Value> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(e) => return Err(json!({ "status": "error", "message": format!("wait: {e}") })),
        }
        let tree = machine::process_tree(child.id());
        let held: u64 = tree.iter().map(|&pid| machine::footprint_bytes(pid)).sum();
        let stopped = if held > cap {
            json!({
                "status": "over-memory",
                "message": format!("its memory, {} MB, passed the cap of {} MB", mb(held as f64), mb(cap as f64)),
            })
        } else if start.elapsed() > timeout + GRACE {
            json!({ "status": "timeout", "message": format!("over {} s", timeout.as_secs()) })
        } else {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        };
        // The run first, so that it starts nothing more; then what it started.
        let _ = child.kill();
        for &pid in &tree[1..] {
            // SAFETY: `kill` only sends a signal, to what was the run's
            // descendant a moment ago.
            #[allow(unsafe_code)]
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGKILL)
            };
        }
        let _ = child.wait();
        return Err(stopped);
    }
}

/// Median, minimum and maximum.
pub(crate) fn spread(xs: &[f64]) -> Option<(f64, f64, f64)> {
    if xs.is_empty() {
        return None;
    }
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    let median = if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    };
    Some((median, s[0], s[n - 1]))
}

fn round(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// One scenario and engine: its runs and what they add up to.
struct Row {
    scenario: &'static Scenario,
    engine: EngineId,
    status: String,
    message: Option<String>,
    runs: Vec<Value>,
}

fn field(run: &Value, key: &str) -> f64 {
    run[key].as_f64().unwrap_or(f64::NAN)
}

impl Row {
    /// The headline number of one run: reasoning time, or the median change.
    fn measure(&self, run: &Value) -> f64 {
        match self.scenario.mode {
            Mode::Full => field(run, "reason_ms"),
            _ => {
                let changes: Vec<f64> = run["change_ms"]
                    .as_array()
                    .map(|a| a.iter().filter_map(Value::as_f64).collect())
                    .unwrap_or_default();
                spread(&changes).map_or(f64::NAN, |s| s.0)
            }
        }
    }

    fn summary(&self) -> Option<Value> {
        let of = |f: &dyn Fn(&Value) -> f64| spread(&self.runs.iter().map(f).collect::<Vec<_>>());
        let (median, min, max) = of(&|r| self.measure(r))?;
        let load = of(&|r| field(r, "load_ms"))?;
        let reason = of(&|r| field(r, "reason_ms"))?;
        let total = of(&|r| total_ms(r))?;
        let peak = of(&|r| field(r, "peak_rss_bytes"))?;
        // The competitors' runs only: the export, and the subprocess's wall clock.
        let competitor_only = |key: &str| {
            let xs: Vec<f64> = self.runs.iter().filter_map(|r| r[key].as_f64()).collect();
            spread(&xs).map(|s| round(s.0))
        };
        let closure = self.runs.last().and_then(|r| r["closure"].as_u64());
        let mut summary = json!({
            "measure": if self.scenario.mode == Mode::Full { "reason_ms" } else { "change_median_ms" },
            "median_ms": round(median),
            "min_ms": round(min),
            "max_ms": round(max),
            "load_median_ms": round(load.0),
            "reason_median_ms": round(reason.0),
            "total_median_ms": round(total.0),
            "export_median_ms": competitor_only("export_ms"),
            "wall_median_ms": competitor_only("wall_ms"),
            "peak_rss_bytes": peak.2 as u64,
            "peak_rss_min_bytes": peak.1 as u64,
            "closure": closure,
            // The memory rubric's measure (docs/specs/experiments.md).
            "peak_bytes_per_closure_triple": closure.filter(|&c| c > 0).map(|c| round(peak.2 / c as f64)),
        });
        // This reasoner's counters and memory breakdown (P0).
        if let Some(p) = profile::summary(&self.runs, peak.2, closure) {
            summary["profile"] = p;
        }
        Some(summary)
    }
}

/// A run's load plus first reasoning: its `total_ms` where the engine
/// reports only that (Soufflé, VLog), else the sum.
fn total_ms(run: &Value) -> f64 {
    run["total_ms"]
        .as_f64()
        .unwrap_or_else(|| field(run, "load_ms") + field(run, "reason_ms"))
}

pub(crate) fn mb(bytes: f64) -> String {
    format!("{:.0}", bytes / 1_048_576.0)
}

fn load_text(l: Option<[f64; 3]>) -> String {
    l.map_or("unknown".into(), |l| format!("{:.2} {:.2} {:.2}", l[0], l[1], l[2]))
}

fn run(args: &Args) -> ExitCode {
    let root = repo_root();
    let data = args.data_dir.clone().unwrap_or_else(|| root.join("data"));
    let generated = root.join("target/bench-data");
    if cfg!(debug_assertions) {
        eprintln!("warning: a debug build; the numbers mean nothing (use --release)");
    }

    let chosen: Vec<&'static Scenario> = scenario::all()
        .filter(|s| match &args.only {
            Some(only) => only.iter().any(|n| n == s.name),
            None => !s.opt_in,
        })
        .collect();

    // Inputs first (generating them is not measured), then the load average.
    let mut rows = Vec::new();
    let mut inputs = Vec::new();
    for &s in &chosen {
        match s.inputs(&data, &generated) {
            Ok(files) => inputs.push((s, files)),
            Err(why) => {
                eprintln!("{}: skipped: {why}", s.name);
                rows.extend(args.engines.iter().map(|&engine| Row {
                    scenario: s,
                    engine,
                    status: "skipped".into(),
                    message: Some(why.clone()),
                    runs: Vec::new(),
                }));
            }
        }
    }

    // The competitors' inputs, converted once (not measured): the children
    // read them from the cache.
    if args.engines.iter().any(|&e| e != EngineId::Turbowl) {
        for (s, files) in inputs.iter().filter(|(s, _)| s.mode == Mode::Full) {
            if let Err(e) = convert::encoded(files) {
                eprintln!("{}: converting the inputs: {e}", s.name);
            }
        }
    }

    let load_before = machine::load_average();
    eprintln!("load average before: {}", load_text(load_before));
    if load_before.is_some_and(|l| l[0] > 1.0) {
        eprintln!("warning: load average over 1; the machine is not quiet and the numbers are noisy");
    }

    for (s, files) in &inputs {
        let mut live: Vec<Row> = args
            .engines
            .iter()
            .map(|&engine| {
                let unsupported = engine.unsupported(s.mode);
                Row {
                    scenario: s,
                    engine,
                    status: if unsupported.is_some() { "unsupported" } else { "ok" }.into(),
                    message: unsupported.map(str::to_string),
                    runs: Vec::new(),
                }
            })
            .collect();
        // Runs alternate between engines, so drift during a scenario falls on all of them.
        for i in 0..args.runs {
            for row in live.iter_mut().filter(|r| r.status == "ok") {
                eprint!("{} {} run {}/{} ... ", s.name, row.engine.name(), i + 1, args.runs);
                let result = spawn_run(row.engine, s.mode, args, files);
                let status = result["status"].as_str().unwrap_or("error").to_string();
                if status == "ok" {
                    match row.measure(&result) {
                        ms if ms.is_nan() => eprintln!("{:.1} ms in all (no split)", total_ms(&result)),
                        ms => eprintln!("{ms:.1} ms"),
                    }
                    row.runs.push(result);
                } else {
                    let message = result["message"].as_str().unwrap_or("").to_string();
                    eprintln!("{status}: {message}");
                    row.status = status;
                    row.message = Some(message);
                }
            }
        }
        rows.extend(live);
    }

    let load_after = machine::load_average();
    eprintln!("load average after: {}", load_text(load_after));
    if load_after.is_some_and(|l| l[0] > 1.0) {
        eprintln!("warning: load average over 1 after the runs");
    }

    // The input sizes of the scenario families that ran at two sizes or
    // more, counted after every run: parsing them is not measured.
    let families = scaling::families(inputs.iter().map(|(s, _)| *s));
    let mut triples: Vec<(&str, u64)> = Vec::new();
    for (s, files) in inputs
        .iter()
        .filter(|(s, _)| s.family().is_some_and(|f| families.contains(&f)))
    {
        match scaling::count_triples(files) {
            Ok(n) => triples.push((s.name, n)),
            Err(e) => eprintln!("{}: counting the input triples: {e}", s.name),
        }
    }
    let measured: Vec<scaling::Measured<'_>> = rows
        .iter()
        .map(|r| scaling::Measured {
            scenario: r.scenario,
            engine: r.engine,
            status: &r.status,
            summary: r.summary(),
        })
        .collect();
    let scaling = scaling::entries(&families, &measured, &|name| {
        triples.iter().find(|(n, _)| *n == name).map(|&(_, t)| t)
    });

    // This reasoner's counts on the scenarios a paper published, comparable
    // with the paper's: counted after every run (not timed).
    let mut published_counts = Vec::new();
    for (s, files) in &inputs {
        let ours = rows
            .iter()
            .find(|r| r.scenario.name == s.name && r.engine == EngineId::Turbowl)
            .and_then(|r| r.runs.last())
            .and_then(|run| run["closure_counted"].as_u64());
        let (Some(closure), true) = (ours, published::all().on(s.name).next().is_some()) else {
            continue;
        };
        eprintln!(
            "{}: counting the data and the ontology's own closure (not timed) ...",
            s.name
        );
        match published::count(files, args.switches, closure) {
            Ok(counts) => published_counts.push((s.name, counts)),
            Err(e) => eprintln!("{}: counting for the published results: {e}", s.name),
        }
    }

    let problems = check(&rows);
    print_table(&rows, args, load_before, load_after);
    scaling::print(&scaling);
    report::print_counts(&published_counts);
    for p in &problems {
        println!("problem: {p}");
    }

    if let Some(path) = &args.json {
        let mut meta = machine::describe(&root);
        meta["date"] = json!(machine::utc_now());
        meta["load_before"] = json!(load_before);
        meta["load_after"] = json!(load_after);
        for (key, value) in settings_meta(args) {
            meta[key] = value;
        }
        meta["peak_memory"] = json!(if cfg!(target_os = "macos") {
            "lifetime peak physical footprint"
        } else {
            "peak resident size"
        });
        meta["data_dir"] = json!(data);
        let results: Vec<Value> = rows
            .iter()
            .map(|r| {
                json!({
                    "scenario": r.scenario.name,
                    "engine": r.engine.name(),
                    "status": r.status,
                    "message": r.message,
                    "inputs": inputs.iter().find(|(s, _)| s.name == r.scenario.name).map(|(_, f)| f),
                    "summary": r.summary(),
                    "x_turbowl": ratios(&rows, r),
                    "runs": r.runs,
                })
            })
            .collect();
        let doc = json!({
            "schema": SCHEMA,
            "meta": meta,
            "results": results,
            "scaling": scaling,
            "published_counts": published_counts
                .iter()
                .map(|(scenario, counts)| {
                    let mut v = counts.to_json();
                    v["scenario"] = json!(scenario);
                    v
                })
                .collect::<Vec<_>>(),
            "problems": problems,
        });
        let text = serde_json::to_string_pretty(&doc).expect("JSON values serialize") + "\n";
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(path, text) {
            eprintln!("{}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        println!("wrote {}", path.display());
    }
    if problems.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The `meta` fields the arguments set.
fn settings_meta(args: &Args) -> [(&'static str, Value); 7] {
    [
        ("runs", json!(args.runs)),
        ("changes", json!(args.changes)),
        ("timeout_s", json!(args.timeout.as_secs())),
        ("memory_cap_bytes", json!(args.memory_cap)),
        ("configuration", json!(configuration(args.switches))),
        ("switches", json!(switch_names(args.switches))),
        // This reasoner's thread mode: 1 for one thread, 0 for all cores, as
        // the scoreboard groups them. The competitors run as they always do
        // (README).
        ("threads", json!(args.threads.count())),
    ]
}

/// Failed runs, closures that differ from what the engine reports,
/// closures that differ between runs of the same engine, and a competitor's
/// closure that differs from this reasoner's on the same scenario. A
/// timeout, a run over the memory cap, a skipped scenario and an unsupported
/// one are results, not problems.
fn check(rows: &[Row]) -> Vec<String> {
    let mut problems = Vec::new();
    let closure = |r: &Row| r.runs.last().and_then(|run| run["closure_counted"].as_u64());
    for r in rows {
        let ours = rows
            .iter()
            .find(|o| o.scenario.name == r.scenario.name && o.engine == EngineId::Turbowl)
            .and_then(closure);
        if let (Some(ours), Some(theirs)) = (ours, closure(r).filter(|_| r.engine != EngineId::Turbowl))
            && ours != theirs
        {
            problems.push(format!(
                "{} {}: {theirs} closure triples, turbowl's {ours} ({:+})",
                r.scenario.name,
                r.engine.name(),
                theirs as i64 - ours as i64
            ));
        }
        let at = format!("{} {}", r.scenario.name, r.engine.name());
        if r.status == "error" {
            problems.push(format!("{at}: {}", r.message.as_deref().unwrap_or("error")));
        }
        for (i, run) in r.runs.iter().enumerate() {
            if run["closure"] != run["closure_counted"] {
                problems.push(format!(
                    "{at} run {}: the engine reports {} closure triples, {} distinct ones were counted",
                    i + 1,
                    run["closure"],
                    run["closure_counted"]
                ));
            }
        }
        let mut sizes: Vec<u64> = r.runs.iter().filter_map(|run| run["closure"].as_u64()).collect();
        sizes.dedup();
        if sizes.len() > 1 {
            problems.push(format!("{at}: the closure size differs between runs: {sizes:?}"));
        }
    }
    problems
}

/// One engine's ratios to this repository's reasoner on the same scenario:
/// the median of the measure (`time`), of load plus first reasoning
/// (`total`), and the largest peak memory (`peak_rss`). Over 1 means the
/// engine is slower or larger. `null` when either has no finished run.
fn ratios(rows: &[Row], r: &Row) -> Value {
    let ours = rows
        .iter()
        .find(|o| o.scenario.name == r.scenario.name && o.engine == EngineId::Turbowl)
        .and_then(Row::summary);
    let (Some(ours), Some(theirs)) = (ours, r.summary()) else {
        return Value::Null;
    };
    let ratio = |key: &str| {
        let (a, b) = (theirs[key].as_f64()?, ours[key].as_f64().filter(|&x| x > 0.0)?);
        Some(round(a / b))
    };
    json!({
        "time": ratio("median_ms"),
        "total": ratio("total_median_ms"),
        "peak_rss": ratio("peak_rss_bytes"),
    })
}

fn print_table(rows: &[Row], args: &Args, before: Option<[f64; 3]>, after: Option<[f64; 3]>) {
    println!();
    println!(
        "TurbOWL bench: {} configuration, turbowl on {}, {} runs each; load average {} before, {} after",
        configuration(args.switches),
        args.threads.label(),
        args.runs,
        load_text(before),
        load_text(after)
    );
    println!("ms: reasoning of a full run; for incremental-*, the median of one run's per-change times.");
    println!(
        "total: load + reasoning. A competitor's times are its own report (Souffle's, its wall clock); its process start is in neither."
    );
    println!("Souffle and VLog do not split loading from reasoning: their ms is blank, their total is the one number.");
    println!("x turbowl: over turbowl's median time, total and largest peak (over 1: slower or larger).");
    println!();
    println!(
        "| scenario | engine | n | load ms | ms (median) | min | max | total ms | peak MB | closure | B/triple | x turbowl time | x turbowl total | x turbowl mem |"
    );
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows {
        let status = if r.status == "ok" {
            String::new()
        } else {
            format!(" ({})", r.status)
        };
        let Some(s) = r.summary() else {
            println!(
                "| {} | {}{status} | 0 | | | | | | | | | | | |",
                r.scenario.name,
                r.engine.name()
            );
            continue;
        };
        let x = ratios(rows, r);
        let ratio = |key: &str| x[key].as_f64().map_or("-".into(), |v| format!("{v:.2}"));
        let n = |key: &str| s[key].as_f64().unwrap_or(f64::NAN);
        // Blank where the engine does not report the number.
        let t = |key: &str| s[key].as_f64().map_or(String::new(), |v| format!("{v:.1}"));
        println!(
            "| {} | {}{status} | {} | {} | {} | {} | {} | {} | {} | {} | {:.0} | {} | {} | {} |",
            r.scenario.name,
            r.engine.name(),
            r.runs.len(),
            t("load_median_ms"),
            t("median_ms"),
            t("min_ms"),
            t("max_ms"),
            t("total_median_ms"),
            mb(n("peak_rss_bytes")),
            s["closure"],
            n("peak_bytes_per_closure_triple"),
            ratio("time"),
            ratio("total"),
            ratio("peak_rss"),
        );
    }
    println!();
    let profiles: Vec<(&str, Value)> = rows
        .iter()
        .filter_map(|r| Some((r.scenario.name, r.summary()?.get("profile")?.clone())))
        .collect();
    profile::print(&profiles);
}

/// Reads a results file. The rows of an engine this bench no longer runs
/// are left out of `results` and `scaling`; the engine of each `results` row
/// left out is returned beside the file.
pub(crate) fn read_results(path: &Path) -> Result<(Value, Vec<String>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut v: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if !v["schema"].as_u64().is_some_and(|n| READS.contains(&n)) {
        return Err(format!(
            "{}: schema {} (this bench reads {READS:?})",
            path.display(),
            v["schema"]
        ));
    }
    if v["schema"].as_u64().is_some_and(|n| n < 4) {
        let new = EngineId::Turbowl.name();
        for list in ["results", "scaling"] {
            for row in v[list].as_array_mut().into_iter().flatten() {
                if row["engine"] == OLD_NAME {
                    row["engine"] = json!(new);
                }
            }
        }
    }
    let mut skipped = Vec::new();
    for list in ["results", "scaling"] {
        if let Some(rows) = v[list].as_array_mut() {
            rows.retain(|row| {
                let engine = row["engine"].as_str().unwrap_or("");
                let runs = EngineId::from_name(engine).is_some();
                if !runs && list == "results" {
                    skipped.push(engine.to_string());
                }
                runs
            });
        }
    }
    Ok((v, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A run whose child process holds more than the cap is stopped, its
    /// child too, and recorded `over-memory`; one under the cap is not.
    #[test]
    fn stops_a_run_over_the_memory_cap() {
        let cap = 64 << 20;
        let ok = Command::new("sh").args(["-c", "exit 0"]).spawn().unwrap();
        assert!(watch(&mut { ok }, Duration::from_secs(60), cap).is_ok());

        // The shell starts dd, which fills a 256 MB buffer from /dev/zero again
        // and again (for some seconds, if nothing stops it), and prints its pid.
        let mut run = Command::new("sh")
            .args([
                "-c",
                "dd if=/dev/zero of=/dev/null bs=268435456 count=2000 2>/dev/null & echo $!; wait",
            ])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(run.stdout.take().unwrap()), &mut line).unwrap();
        let dd: libc::pid_t = line.trim().parse().unwrap();
        let start = Instant::now();
        let stopped = watch(&mut run, Duration::from_secs(60), cap).unwrap_err();
        assert_eq!(stopped["status"], "over-memory", "{stopped}");
        assert!(start.elapsed() < Duration::from_secs(30), "{:?}", start.elapsed());
        // dd is gone once init has reaped it.
        // SAFETY: signal 0 sends nothing; it asks whether the pid exists.
        #[allow(unsafe_code)]
        let alive = || unsafe { libc::kill(dd, 0) } == 0;
        let deadline = Instant::now() + Duration::from_secs(5);
        while alive() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!alive(), "dd ({dd}) outlived the run");
    }

    #[test]
    fn reads_the_memory_cap() {
        let args = |a: &[&str]| parse_args(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(args(&[]).unwrap().memory_cap, 16 << 30);
        assert_eq!(args(&["--memory-cap", "0.5"]).unwrap().memory_cap, 512 << 20);
        for bad in ["0", "-1", "x", "inf"] {
            assert!(args(&["--memory-cap", bad]).is_err(), "{bad}");
        }
    }

    #[test]
    fn reads_the_thread_mode() {
        let args = |a: &[&str]| parse_args(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let threads = |a: &Args| settings_meta(a).into_iter().find(|(k, _)| *k == "threads").unwrap().1;
        let default = args(&[]).unwrap();
        assert_eq!(default.threads, ThreadMode::One);
        assert_eq!(threads(&default), 1);
        let one = args(&["--threads", "one"]).unwrap();
        assert_eq!(one.threads, ThreadMode::One);
        assert_eq!(threads(&one), 1);
        let all = args(&["--threads", "all"]).unwrap();
        assert_eq!(all.threads, ThreadMode::All);
        assert_eq!(threads(&all), 0);
        assert_eq!(
            args(&["--threads", "all", "--threads", "one"]).unwrap().threads,
            ThreadMode::One
        );
        for bad in ["", "1", "0", "ALL", "two", "every"] {
            let e = args(&["--threads", bad]).unwrap_err();
            assert!(e.contains("--threads takes one or all"), "{bad}: {e}");
        }
        assert_eq!(args(&["--threads"]).unwrap_err(), "--threads needs a value");
    }

    /// The parent's arguments for a run read back as the same run in the
    /// child, in both thread modes; a child without THREADS is refused.
    #[test]
    fn hands_the_run_to_the_child() {
        for (threads, switches) in [
            (ThreadMode::One, Switches::default()),
            (ThreadMode::All, Switches::complete()),
            (
                ThreadMode::All,
                Switch::ALL.into_iter().fold(Switches::complete(), Switches::off),
            ),
        ] {
            let run = OneRun {
                engine: EngineId::Turbowl,
                mode: Mode::Add,
                switches,
                threads,
                changes: 7,
                timeout: Duration::from_secs(42),
                files: vec!["a.ttl".into(), "dir/b.nt".into()],
            };
            let args: Vec<String> = run.to_args().into_iter().map(|a| a.into_string().unwrap()).collect();
            assert_eq!(args[0], "--one");
            assert_eq!(args[4], threads.name());
            assert_eq!(OneRun::from_args(&args[1..]), Some(run));
        }
        let old = ["turbowl", "full", "none", "100", "600", "a.ttl"].map(String::from);
        assert_eq!(OneRun::from_args(&old), None);
        let bad = ["turbowl", "full", "none", "two", "100", "600"].map(String::from);
        assert_eq!(OneRun::from_args(&bad), None);
    }

    #[test]
    fn reads_the_old_name_of_this_reasoner() {
        let dir = std::env::temp_dir().join(format!("bench-read-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let row = |engine: &str| json!({ "scenario": "lubm-1", "engine": engine });
        for (schema, engine, read) in [
            (3, OLD_NAME, Some("turbowl")),
            (4, "turbowl", Some("turbowl")),
            (4, OLD_NAME, None),
        ] {
            let path = dir.join(format!("v{schema}-{engine}.json"));
            let file = json!({ "schema": schema, "results": [row(engine), row("nemo")], "scaling": [row(engine)] });
            std::fs::write(&path, file.to_string()).unwrap();
            let (v, skipped) = read_results(&path).unwrap();
            let engines: Vec<&Value> = v["results"].as_array().unwrap().iter().map(|r| &r["engine"]).collect();
            match read {
                Some(name) => {
                    assert_eq!(engines, [name, "nemo"], "schema {schema}");
                    assert_eq!(v["scaling"][0]["engine"], name, "schema {schema}");
                    assert!(skipped.is_empty(), "schema {schema}");
                }
                // In version 4 the old name is an engine this bench does not run.
                None => {
                    assert_eq!(engines, ["nemo"], "schema {schema}");
                    assert_eq!(v["scaling"], json!([]), "schema {schema}");
                    assert_eq!(skipped, [OLD_NAME], "schema {schema}");
                }
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A results file from version 1 still compares with a newer one: the
    /// rows of the engine version 2 dropped are skipped, the rest match.
    #[test]
    fn skips_the_rows_of_dropped_engines() {
        let data = repo_root().join("docs/verification/data");
        let (old, skipped) = read_results(&data.join("2026-11-05-wave-3/bench-3252f68.json")).unwrap();
        let (new, none) = read_results(&data.join("2026-11-05-wave-4/bench-trend-7c31e93.json")).unwrap();
        assert!(none.is_empty());
        assert_eq!(skipped.len(), 7, "one row per scenario: {skipped:?}");
        assert!(
            skipped
                .iter()
                .all(|e| *e == skipped[0] && EngineId::from_name(e).is_none())
        );
        let rows = |v: &Value, list: &str| v[list].as_array().unwrap().clone();
        assert_eq!(rows(&old, "results").len(), 14, "turbowl and nemo on 7 scenarios");
        for list in ["results", "scaling"] {
            for r in rows(&old, list) {
                assert!(EngineId::from_name(r["engine"].as_str().unwrap()).is_some(), "{r}");
            }
        }
        let key = |r: &Value| (r["scenario"].clone(), r["engine"].clone());
        for n in rows(&new, "results") {
            assert!(rows(&old, "results").iter().any(|o| key(o) == key(&n)), "{n}");
        }
    }
}
