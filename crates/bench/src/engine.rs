//! The engines, and one measured run of one engine (the child process's
//! work: the `--one` mode): this reasoner in process, each competitor as a
//! subprocess on the converted input (`convert.rs`), run by [`run_tool`].

use oxrdf::Triple;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use turbowl::{Reasoner, Switches, Threads};

/// The engines a scenario can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngineId {
    /// This repository's reasoner, in process.
    Turbowl,
    /// Nemo's `nmo`, a subprocess.
    Nemo,
    /// Soufflé, the program compiled to a binary, a subprocess.
    Souffle,
    /// VLog through Rulewerk's command-line client on the JRE, a subprocess.
    Vlog,
}

impl EngineId {
    pub(crate) const ALL: [EngineId; 4] = [EngineId::Turbowl, EngineId::Nemo, EngineId::Souffle, EngineId::Vlog];

    pub(crate) fn name(self) -> &'static str {
        match self {
            EngineId::Turbowl => "turbowl",
            EngineId::Nemo => "nemo",
            EngineId::Souffle => "souffle",
            EngineId::Vlog => "vlog",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.name() == name)
    }

    /// The pinned tool a competitor runs; `None` for this reasoner.
    pub(crate) fn tool(self) -> Option<&'static Tool> {
        match self {
            EngineId::Turbowl => None,
            EngineId::Nemo => Some(&crate::nemo::TOOL),
            EngineId::Souffle => Some(&crate::souffle::TOOL),
            EngineId::Vlog => Some(&crate::vlog::TOOL),
        }
    }

    /// Why the engine cannot run a scenario of this mode, if it cannot.
    pub(crate) fn unsupported(self, mode: Mode) -> Option<&'static str> {
        (self != EngineId::Turbowl && mode != Mode::Full)
            .then_some("a Datalog engine runs a program once over its input; it has no incremental changes")
    }
}

/// A competitor's pinned tool: `tools/<dir>/`, fetched by
/// `scripts/fetch-tools.sh`, which writes the version into `VERSION`.
#[derive(Debug)]
pub(crate) struct Tool {
    pub(crate) dir: &'static str,
    pub(crate) version: &'static str,
    /// The file that runs, under the directory.
    pub(crate) file: &'static str,
}

impl Tool {
    /// The version in `tools/<dir>/VERSION`, when the tool is fetched.
    pub(crate) fn installed_version(&self) -> Option<String> {
        std::fs::read_to_string(crate::repo_root().join("tools").join(self.dir).join("VERSION"))
            .ok()
            .map(|v| v.trim().to_string())
    }

    /// The tool's file, or why the engine is skipped.
    pub(crate) fn path(&self) -> Result<PathBuf, String> {
        let path = crate::repo_root().join("tools").join(self.dir).join(self.file);
        match self.installed_version() {
            Some(v) if v == self.version && path.is_file() => Ok(path),
            Some(v) => Err(format!(
                "tools/{} is {v}, not {}: run scripts/fetch-tools.sh",
                self.dir, self.version
            )),
            None => Err(format!(
                "tools/{}/{} is missing: run scripts/fetch-tools.sh",
                self.dir, self.file
            )),
        }
    }
}

/// The number of distinct values among 64-bit hashes: sorted in place, so
/// it holds 8 bytes per hash and nothing more.
pub(crate) fn distinct(mut hashes: Vec<u64>) -> usize {
    hashes.sort_unstable();
    hashes.dedup();
    hashes.len()
}

/// The number of distinct triples, counted by their 64-bit hashes (a fixed
/// hasher, so a collision would show the same way on every run).
fn distinct_triples(triples: impl Iterator<Item = Triple>, expected: usize) -> usize {
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
    let hasher = BuildHasherDefault::<DefaultHasher>::default();
    let mut hashes = Vec::with_capacity(expected);
    hashes.extend(triples.map(|t| hasher.hash_one(&t)));
    distinct(hashes)
}

/// What a competitor's process left: what it printed, its wall clock and its
/// peak memory.
#[derive(Debug)]
pub(crate) struct Ran {
    pub(crate) stdout: String,
    pub(crate) wall_ms: f64,
    pub(crate) peak_bytes: u64,
}

/// Runs a competitor's command in `dir`, its standard output and error into
/// files there and its standard input from `stdin` (or nothing), for at most
/// `timeout`. `Ok(None)` when it ran over `timeout` and was stopped; `Err`
/// when it did not start or exited with a failure, with the end of its
/// standard error.
pub(crate) fn run_tool(
    cmd: &mut Command,
    dir: &Path,
    stdin: Option<&Path>,
    timeout: Duration,
) -> Result<Option<Ran>, String> {
    let io = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    let (out, err) = (dir.join("stdout.txt"), dir.join("stderr.txt"));
    let stdout = std::fs::File::create(&out).map_err(|e| io(&out, e))?;
    let stderr = std::fs::File::create(&err).map_err(|e| io(&err, e))?;
    let stdin = match stdin {
        Some(path) => Stdio::from(std::fs::File::open(path).map_err(|e| io(path, e))?),
        None => Stdio::null(),
    };
    let program = cmd.get_program().to_string_lossy().into_owned();
    let start = Instant::now();
    let mut child = cmd
        .current_dir(dir)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .map_err(|e| format!("{program}: {e}"))?;
    let Some((status, peak_bytes)) =
        crate::machine::wait_with_peak(&mut child, timeout).map_err(|e| format!("{program}: {e}"))?
    else {
        return Ok(None);
    };
    let wall_ms = ms(start);
    if !status.success() {
        let text = std::fs::read_to_string(&err).unwrap_or_default();
        let tail: Vec<&str> = text.lines().rev().take(5).collect();
        let tail: Vec<&str> = tail.into_iter().rev().collect();
        return Err(format!("{program} exited with {status}: {}", tail.join(" / ")));
    }
    Ok(Some(Ran {
        stdout: std::fs::read_to_string(&out).unwrap_or_default(),
        wall_ms,
        peak_bytes,
    }))
}

/// A run stopped at `timeout`.
pub(crate) fn timed_out(timeout: Duration) -> Value {
    json!({ "status": "timeout", "message": format!("over {} s", timeout.as_secs()) })
}

/// A fresh scratch directory for one run of a competitor:
/// `target/bench-<engine>/<pid>/`, emptied first.
pub(crate) fn scratch(engine: EngineId) -> Result<PathBuf, String> {
    let dir = crate::repo_root().join(format!("target/bench-{}/{}", engine.name(), std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(dir)
}

/// What a run does after loading and the first `reason()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Nothing: load and reason.
    Full,
    /// Add one triple, reason, `changes` times.
    Add,
    /// Remove one base triple, reason, `changes` times.
    Remove,
}

impl Mode {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Mode::Full => "full",
            Mode::Add => "add",
            Mode::Remove => "remove",
        }
    }
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        [Mode::Full, Mode::Add, Mode::Remove]
            .into_iter()
            .find(|m| m.name() == name)
    }
}

/// The threads this reasoner's runs use (`--threads`). The competitors run
/// as they always do, whatever the mode (README).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThreadMode {
    /// [`Threads::ONE`]: every round on the run's own thread.
    One,
    /// [`Threads::ALL`]: every core, for the rounds large enough.
    All,
}

impl ThreadMode {
    pub(crate) fn name(self) -> &'static str {
        match self {
            ThreadMode::One => "one",
            ThreadMode::All => "all",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        [ThreadMode::One, ThreadMode::All]
            .into_iter()
            .find(|t| t.name() == name)
    }

    /// The setting this reasoner's runs take.
    pub(crate) fn threads(self) -> Threads {
        match self {
            ThreadMode::One => Threads::ONE,
            ThreadMode::All => Threads::ALL,
        }
    }

    /// As the results file records it (`meta.threads`, a run's `threads`):
    /// 1 for one thread, 0 for all cores.
    pub(crate) fn count(self) -> u64 {
        match self {
            ThreadMode::One => 1,
            ThreadMode::All => 0,
        }
    }

    /// As the text report and the scoreboard say it.
    pub(crate) fn label(self) -> &'static str {
        match self {
            ThreadMode::One => "one thread",
            ThreadMode::All => "all cores",
        }
    }
}

fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

/// The triples of an input file, in file order (for the change streams).
fn read_triples(path: &Path) -> Result<Vec<Triple>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let reader = std::io::BufReader::new(file);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let triples: Result<Vec<Triple>, _> = if ext == "ttl" {
        oxttl::TurtleParser::new().for_reader(reader).collect()
    } else {
        oxttl::NTriplesParser::new().for_reader(reader).collect()
    };
    triples.map_err(|e| format!("{}: {e}", path.display()))
}

/// The change stream: `count` triples to add or remove, chosen deterministically
/// from the inputs. Additions copy the last file's triples (the building model)
/// onto new subjects, so each one is a new entity of a real class or relation.
/// Removals are base triples spread evenly through all the files in order,
/// among those without a blank node: a blank node read here is a new node,
/// not the engine's, so removing it would remove nothing.
fn changes(mode: Mode, files: &[PathBuf], count: usize) -> Result<Vec<Triple>, String> {
    match mode {
        Mode::Full => Ok(Vec::new()),
        Mode::Add => {
            let last = files.last().ok_or("no input files")?;
            let templates: Vec<Triple> = read_triples(last)?
                .into_iter()
                .filter(|t| matches!(t.subject, oxrdf::NamedOrBlankNode::NamedNode(_)))
                .collect();
            if templates.is_empty() {
                return Err(format!("{}: no triple with an IRI subject to copy", last.display()));
            }
            Ok((0..count)
                .map(|k| {
                    let t = &templates[k % templates.len()];
                    let subject = oxrdf::NamedNode::new_unchecked(format!("http://example.org/bench/added/{k}"));
                    Triple::new(subject, t.predicate.clone(), t.object.clone())
                })
                .collect())
        }
        Mode::Remove => {
            let mut all = Vec::new();
            for f in files {
                all.extend(read_triples(f)?);
            }
            let mut seen = std::collections::HashSet::new();
            all.retain(|t| {
                let blank = t.subject.is_blank_node() || t.object.is_blank_node();
                !blank && seen.insert(t.clone())
            });
            if all.len() < count {
                return Err(format!("only {} base triples, {count} removals asked", all.len()));
            }
            let step = all.len() / count;
            Ok((0..count).map(|k| all[k * step].clone()).collect())
        }
    }
}

/// One measured run, as the child process does it, in the configuration
/// `switches`; this reasoner runs on `threads`, a competitor as it always
/// does. The result is one JSON object: `status` is `ok`,
/// `unsupported`, `skipped` (a competitor is not fetched), `timeout` (a
/// competitor ran over `timeout`) or `error` (a failed load or run, or a
/// panic in this reasoner).
pub(crate) fn run_one(
    engine: EngineId,
    mode: Mode,
    switches: Switches,
    threads: ThreadMode,
    files: &[PathBuf],
    count: usize,
    timeout: Duration,
) -> Value {
    if let Some(why) = engine.unsupported(mode) {
        return json!({ "status": "unsupported", "message": why });
    }
    if engine != EngineId::Turbowl {
        let competitor = || -> Result<Value, String> {
            let input = crate::convert::encoded(files)?;
            match engine {
                EngineId::Nemo => crate::nemo::measure(&input, switches, timeout),
                EngineId::Souffle => crate::souffle::measure(&input, switches, timeout),
                EngineId::Vlog => crate::vlog::measure(&input, switches, timeout),
                EngineId::Turbowl => unreachable!("this reasoner runs in process"),
            }
        };
        return competitor().unwrap_or_else(|message| json!({ "status": "error", "message": message }));
    }
    // A panic is reported as an error result, not a backtrace.
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        measure(mode, switches, threads, files, count)
    }));
    match result {
        Ok(Ok(v)) => v,
        Ok(Err(message)) => json!({ "status": "error", "message": message }),
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panic".into());
            json!({ "status": "error", "message": format!("panicked: {message}") })
        }
    }
}

/// One run of this reasoner, in process. The threads are set before
/// loading, which parses on them too.
fn measure(
    mode: Mode,
    switches: Switches,
    threads: ThreadMode,
    files: &[PathBuf],
    count: usize,
) -> Result<Value, String> {
    let stream = changes(mode, files, count)?;
    let mut r = Reasoner::with_switches(switches);
    r.set_threads(threads.threads());

    let start = Instant::now();
    for f in files {
        r.load_file(f).map_err(|e| e.to_string())?;
    }
    let load_ms = ms(start);

    let start = Instant::now();
    let mut summary = r.reason();
    let reason_ms = ms(start);
    // The counters of the full run, before the changes replace them (not
    // timed); the memory is read at the end.
    let full = (mode != Mode::Full).then(|| r.profile());

    let mut change_ms = Vec::with_capacity(stream.len());
    for t in &stream {
        let start = Instant::now();
        match mode {
            Mode::Add => r.insert([t.as_ref()]),
            _ => r.remove([t.as_ref()]),
        }
        summary = r.reason();
        change_ms.push(ms(start));
    }
    // Read the peak before the check below, which holds a hash per triple,
    // and before the profile, whose walk allocates a little.
    let peak_rss_bytes = crate::machine::own_peak_bytes();
    let held = r.profile();
    let profile = crate::profile::to_json(full.as_ref().unwrap_or(&held), &held.memory);
    let reported = summary.base + summary.inferred;
    let counted = distinct_triples(r.triples(), reported);
    // The setting the reasoner held at the end, as `meta.threads` counts it.
    let held_threads = [ThreadMode::One, ThreadMode::All]
        .into_iter()
        .find(|t| t.threads() == r.threads())
        .map(ThreadMode::count);
    Ok(json!({
        "status": "ok",
        "load_ms": load_ms,
        "reason_ms": reason_ms,
        "change_ms": change_ms,
        "peak_rss_bytes": peak_rss_bytes,
        "closure": reported,
        "closure_counted": counted,
        "profile": profile,
        "threads": held_threads,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A run of this reasoner holds the thread mode it was given, and the
    /// closure is the same in both.
    #[test]
    fn runs_this_reasoner_on_the_thread_mode() {
        let dir = std::env::temp_dir().join(format!("bench-threads-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("input.ttl");
        std::fs::write(
            &input,
            "@prefix ex: <http://example.org/> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             ex:A rdfs:subClassOf ex:B .\n\
             ex:x a ex:A .\n",
        )
        .unwrap();
        let files = [input];
        let mut closures = Vec::new();
        for (mode, count) in [(ThreadMode::One, 1), (ThreadMode::All, 0)] {
            assert_eq!(ThreadMode::from_name(mode.name()), Some(mode));
            assert_eq!(mode.count(), count);
            for run_mode in [Mode::Full, Mode::Add] {
                // `measure`, not `run_one`, which silences every panic message.
                let run = measure(run_mode, Switches::default(), mode, &files, 2).unwrap();
                assert_eq!(run["status"], "ok", "{run}");
                assert_eq!(run["threads"], count, "{} {}", mode.name(), run_mode.name());
                if run_mode == Mode::Full {
                    closures.push(run["closure"].clone());
                }
            }
        }
        assert_eq!(closures[0], closures[1]);
        assert!(closures[0].as_u64().is_some_and(|c| c > 3), "{closures:?}");
        assert_eq!(ThreadMode::from_name("2"), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
