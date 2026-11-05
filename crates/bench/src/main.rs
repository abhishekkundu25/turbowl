//! The benchmark: runs each scenario for this crate's reasoner (both
//! profiles) and for `reasonable`, the same inputs in the same way, and
//! reports median, spread, peak memory and closure size. See `README.md`.
//!
//! Every run is a fresh child process (this binary with the hidden `--one`
//! mode), so that peak memory is one engine's on one input.

mod compare;
mod engine;
mod lubm;
mod machine;
mod scenario;

use engine::{EngineId, Mode};
use scenario::{SCENARIOS, Scenario};
use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

/// The results file's format version; `--compare` refuses others.
const SCHEMA: u64 = 1;

const USAGE: &str = "\
usage: turtle-reasoner-bench [options]
       turtle-reasoner-bench --compare OLD.json NEW.json

  --list              print the scenarios and stop
  --only A,B          run only these scenarios
  --engines A,B       run only these engines (turtle-owl-rl, turtle-reasonable, reasonable)
  --runs N            runs per scenario and engine (default 5)
  --changes N         changes in the incremental scenarios (default 100)
  --timeout SECS      stop a run after this long (default 600)
  --data-dir DIR      fetched models (default: data/ at the repository root)
  --json FILE         also write the results to FILE
  --compare OLD NEW   print per-scenario deltas between two results files

Run on a quiet machine: load average under 1, nothing building or testing.";

#[derive(Debug)]
struct Args {
    list: bool,
    only: Option<Vec<String>>,
    engines: Vec<EngineId>,
    runs: usize,
    changes: usize,
    timeout: Duration,
    data_dir: Option<PathBuf>,
    json: Option<PathBuf>,
    compare: Option<(PathBuf, PathBuf)>,
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut a = Args {
        list: false,
        only: None,
        engines: EngineId::ALL.to_vec(),
        runs: 5,
        changes: 100,
        timeout: Duration::from_secs(600),
        data_dir: None,
        json: None,
        compare: None,
    };
    let mut it = argv.iter();
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
            "--runs" => a.runs = number(value()?)?,
            "--changes" => a.changes = number(value()?)?,
            "--timeout" => a.timeout = Duration::from_secs(number(value()?)? as u64),
            "--data-dir" => a.data_dir = Some(value()?.into()),
            "--json" => a.json = Some(value()?.into()),
            "--compare" => a.compare = Some((value()?.into(), value()?.into())),
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if let Some(only) = &a.only {
        for name in only {
            if scenario::find(name).is_none() {
                return Err(format!("unknown scenario {name}; --list shows them"));
            }
        }
    }
    Ok(a)
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
        for s in SCENARIOS {
            println!("{:<20} {}", s.name, s.about);
        }
        return ExitCode::SUCCESS;
    }
    if let Some((old, new)) = &args.compare {
        return compare::run(old, new);
    }
    run(&args)
}

/// The hidden `--one ENGINE MODE CHANGES FILE...` mode: one measured run,
/// one JSON line on stdout.
fn child(argv: &[String]) -> ExitCode {
    let (Some(engine), Some(mode), Some(changes)) = (
        argv.first().and_then(|e| EngineId::from_name(e)),
        argv.get(1).and_then(|m| Mode::from_name(m)),
        argv.get(2).and_then(|c| c.parse().ok()),
    ) else {
        eprintln!("--one ENGINE MODE CHANGES FILE...");
        return ExitCode::from(64);
    };
    let files: Vec<PathBuf> = argv[3..].iter().map(PathBuf::from).collect();
    println!("{}", engine::run_one(engine, mode, &files, changes));
    ExitCode::SUCCESS
}

fn repo_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    root.canonicalize().unwrap_or(root)
}

/// Spawns one run and waits for it, at most `timeout`.
fn spawn_run(engine: EngineId, mode: Mode, changes: usize, files: &[PathBuf], timeout: Duration) -> Value {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => return json!({ "status": "error", "message": format!("current_exe: {e}") }),
    };
    let mut child = match Command::new(exe)
        .arg("--one")
        .arg(engine.name())
        .arg(mode.name())
        .arg(changes.to_string())
        .args(files)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return json!({ "status": "error", "message": format!("spawn: {e}") }),
    };
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return json!({ "status": "timeout", "message": format!("over {} s", timeout.as_secs()) });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return json!({ "status": "error", "message": format!("wait: {e}") }),
        }
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
        let total = of(&|r| field(r, "load_ms") + field(r, "reason_ms"))?;
        let peak = of(&|r| field(r, "peak_rss_bytes"))?;
        let closure = self.runs.last().and_then(|r| r["closure"].as_u64());
        Some(json!({
            "measure": if self.scenario.mode == Mode::Full { "reason_ms" } else { "change_median_ms" },
            "median_ms": round(median),
            "min_ms": round(min),
            "max_ms": round(max),
            "load_median_ms": round(load.0),
            "reason_median_ms": round(reason.0),
            "total_median_ms": round(total.0),
            "peak_rss_bytes": peak.2 as u64,
            "peak_rss_min_bytes": peak.1 as u64,
            "closure": closure,
            // The memory rubric's measure (docs/specs/experiments.md).
            "peak_bytes_per_closure_triple": closure.filter(|&c| c > 0).map(|c| round(peak.2 / c as f64)),
        }))
    }
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

    let chosen: Vec<&'static Scenario> = SCENARIOS
        .iter()
        .filter(|s| args.only.as_ref().is_none_or(|o| o.iter().any(|n| n == s.name)))
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

    let load_before = machine::load_average();
    eprintln!("load average before: {}", load_text(load_before));
    if load_before.is_some_and(|l| l[0] > 1.0) {
        eprintln!("warning: load average over 1; the machine is not quiet and the numbers are noisy");
    }

    for (s, files) in &inputs {
        let mut live: Vec<Row> = args
            .engines
            .iter()
            .map(|&engine| Row {
                scenario: s,
                engine,
                status: "ok".into(),
                message: None,
                runs: Vec::new(),
            })
            .collect();
        // Runs alternate between engines, so drift during a scenario falls on all of them.
        for i in 0..args.runs {
            for row in live.iter_mut().filter(|r| r.status == "ok") {
                eprint!("{} {} run {}/{} ... ", s.name, row.engine.name(), i + 1, args.runs);
                let result = spawn_run(row.engine, s.mode, args.changes, files, args.timeout);
                let status = result["status"].as_str().unwrap_or("error").to_string();
                if status == "ok" {
                    eprintln!("{:.1} ms", row.measure(&result));
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

    let problems = check(&rows);
    print_table(&rows, args, load_before, load_after);
    for p in &problems {
        println!("problem: {p}");
    }

    if let Some(path) = &args.json {
        let mut meta = machine::describe(&root);
        meta["date"] = json!(machine::utc_now());
        meta["load_before"] = json!(load_before);
        meta["load_after"] = json!(load_after);
        meta["runs"] = json!(args.runs);
        meta["changes"] = json!(args.changes);
        meta["timeout_s"] = json!(args.timeout.as_secs());
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
                    "runs": r.runs,
                })
            })
            .collect();
        let doc = json!({ "schema": SCHEMA, "meta": meta, "results": results, "problems": problems });
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

/// Failed runs, closures that differ from what the engine reports, and
/// closures that differ between runs of the same engine. A timeout, a
/// skipped scenario and our stub ("unavailable") are results, not problems.
fn check(rows: &[Row]) -> Vec<String> {
    let mut problems = Vec::new();
    for r in rows {
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

fn print_table(rows: &[Row], args: &Args, before: Option<[f64; 3]>, after: Option<[f64; 3]>) {
    println!();
    println!(
        "turtle-reasoner bench: {} runs each; load average {} before, {} after",
        args.runs,
        load_text(before),
        load_text(after)
    );
    println!("ms: reason() of a full run; for incremental-*, the median of one run's per-change times.");
    println!("x reasonable: over reasonable's median time and largest peak (under 1 is better).");
    println!();
    println!(
        "| scenario | engine | n | load ms | ms (median) | min | max | peak MB | closure | B/triple | x reasonable time | x reasonable mem |"
    );
    println!("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in rows {
        let Some(s) = r.summary() else {
            println!(
                "| {} | {} ({}) | 0 | | | | | | | | | |",
                r.scenario.name,
                r.engine.name(),
                r.status
            );
            continue;
        };
        let theirs = rows
            .iter()
            .find(|o| o.scenario.name == r.scenario.name && o.engine == EngineId::Reasonable)
            .and_then(Row::summary);
        let ratio = |key: &str| {
            theirs
                .as_ref()
                .and_then(|t| Some(s[key].as_f64()? / t[key].as_f64().filter(|&x| x > 0.0)?))
                .map_or("-".into(), |x| format!("{x:.2}"))
        };
        let status = if r.status == "ok" {
            String::new()
        } else {
            format!(" ({})", r.status)
        };
        let n = |key: &str| s[key].as_f64().unwrap_or(f64::NAN);
        println!(
            "| {} | {}{status} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {} | {} | {:.0} | {} | {} |",
            r.scenario.name,
            r.engine.name(),
            r.runs.len(),
            n("load_median_ms"),
            n("median_ms"),
            n("min_ms"),
            n("max_ms"),
            mb(n("peak_rss_bytes")),
            s["closure"],
            n("peak_bytes_per_closure_triple"),
            ratio("median_ms"),
            ratio("peak_rss_bytes"),
        );
    }
    println!();
}

/// Reads a results file.
pub(crate) fn read_results(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    if v["schema"].as_u64() != Some(SCHEMA) {
        return Err(format!(
            "{}: schema {} (this bench reads {SCHEMA})",
            path.display(),
            v["schema"]
        ));
    }
    Ok(v)
}
