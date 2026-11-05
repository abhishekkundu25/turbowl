//! The engines behind one small trait, and one measured run of one engine
//! (the child process's work: the `--one` mode).

use oxrdf::Triple;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::time::Instant;
use turtle_reasoner::Reasoner;

/// The `reasonable` release linked in (the exact pin in `Cargo.toml`).
pub(crate) const REASONABLE_VERSION: &str = "0.5.0-a1";

/// What a scenario needs from a reasoner.
trait Engine {
    fn load(&mut self, path: &Path) -> Result<(), String>;
    fn reason(&mut self);
    /// Called once after the first run, outside the timings, before a change stream.
    fn prepare_changes(&mut self) {}
    fn insert(&mut self, triple: &Triple);
    fn remove(&mut self, triple: &Triple);
    /// The closure size the engine reports, and the number of distinct
    /// closure triples counted by iterating it.
    fn closure(&self) -> (usize, usize);
}

/// The engines a scenario can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngineId {
    /// This repository's reasoner, in process.
    TurtleReasoner,
    /// `reasonable`, linked in, in process.
    Reasonable,
    /// Nemo's `nmo` running `oracles/nemo/owl2rl.rls`, a subprocess.
    Nemo,
}

impl EngineId {
    pub(crate) const ALL: [EngineId; 3] = [EngineId::TurtleReasoner, EngineId::Reasonable, EngineId::Nemo];

    pub(crate) fn name(self) -> &'static str {
        match self {
            EngineId::TurtleReasoner => "turtle-reasoner",
            EngineId::Reasonable => "reasonable",
            EngineId::Nemo => "nemo",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.name() == name)
    }

    /// Why the engine cannot run a scenario of this mode, if it cannot.
    pub(crate) fn unsupported(self, mode: Mode) -> Option<&'static str> {
        (self == EngineId::Nemo && mode != Mode::Full)
            .then_some("Nemo runs a program once over its imports; it has no incremental changes")
    }

    /// The in-process engines; Nemo runs as a subprocess instead.
    fn make(self) -> Option<Box<dyn Engine>> {
        match self {
            EngineId::TurtleReasoner => Some(Box::new(Ours {
                r: Reasoner::new(),
                reported: 0,
            })),
            EngineId::Reasonable => Some(Box::new(Theirs {
                r: reasonable::reasoner::Reasoner::new(),
                base: Vec::new(),
            })),
            EngineId::Nemo => None,
        }
    }
}

struct Ours {
    r: Reasoner,
    /// Base plus inferred, from the last run's `Summary`.
    reported: usize,
}

impl Engine for Ours {
    fn load(&mut self, path: &Path) -> Result<(), String> {
        self.r.load_file(path).map_err(|e| e.to_string())
    }
    fn reason(&mut self) {
        let s = self.r.reason();
        self.reported = s.base + s.inferred;
    }
    fn insert(&mut self, triple: &Triple) {
        self.r.insert([triple.as_ref()]);
    }
    fn remove(&mut self, triple: &Triple) {
        self.r.remove([triple.as_ref()]);
    }
    fn closure(&self) -> (usize, usize) {
        (self.reported, distinct(self.r.triples()))
    }
}

struct Theirs {
    r: reasonable::reasoner::Reasoner,
    /// The base triples, kept for removals: `reasonable` removes only by
    /// replacing the whole base (`set_base_triples`, as its Python `update_graph`).
    base: Vec<Triple>,
}

impl Engine for Theirs {
    fn load(&mut self, path: &Path) -> Result<(), String> {
        let path = path.to_str().ok_or("path is not UTF-8")?;
        self.r.load_file(path).map_err(|e| e.to_string())
    }
    fn reason(&mut self) {
        self.r.reason();
    }
    fn prepare_changes(&mut self) {
        self.base = self.r.get_input();
    }
    fn insert(&mut self, triple: &Triple) {
        self.r.load_triples(vec![triple.clone()]);
    }
    fn remove(&mut self, triple: &Triple) {
        self.base.retain(|t| t != triple);
        self.r.set_base_triples(self.base.clone());
    }
    fn closure(&self) -> (usize, usize) {
        let out = self.r.view_output();
        (out.len(), distinct(out.iter()))
    }
}

/// The number of distinct triples, counted by their 64-bit hashes (a fixed
/// hasher, so a collision would show the same way on every run).
pub(crate) fn distinct<T: std::borrow::Borrow<Triple>>(triples: impl Iterator<Item = T>) -> usize {
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
    let hasher = BuildHasherDefault::<DefaultHasher>::default();
    let hashes: std::collections::HashSet<u64> = triples.map(|t| hasher.hash_one(t.borrow())).collect();
    hashes.len()
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
/// Removals are base triples spread evenly through all the files in order.
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
            all.retain(|t| seen.insert(t.clone()));
            if all.len() < count {
                return Err(format!("only {} base triples, {count} removals asked", all.len()));
            }
            let step = all.len() / count;
            Ok((0..count).map(|k| all[k * step].clone()).collect())
        }
    }
}

/// One measured run, as the child process does it. The result is one JSON
/// object: `status` is `ok`, `unavailable` (our reasoner is still the stub),
/// `unsupported`, `skipped` (Nemo is not fetched), `timeout` (Nemo ran over
/// `timeout`) or `error`.
pub(crate) fn run_one(engine: EngineId, mode: Mode, files: &[PathBuf], count: usize, timeout: Duration) -> Value {
    if let Some(why) = engine.unsupported(mode) {
        return json!({ "status": "unsupported", "message": why });
    }
    if engine == EngineId::Nemo {
        return crate::nemo::measure(files, timeout)
            .unwrap_or_else(|message| json!({ "status": "error", "message": message }));
    }
    // The stub panics with `unimplemented!`; report it, not a backtrace.
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| measure(engine, mode, files, count)));
    match result {
        Ok(Ok(v)) => v,
        Ok(Err(message)) => json!({ "status": "error", "message": message }),
        Err(payload) => {
            let message = payload
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panic".into());
            let status = if message.contains("not implemented") {
                "unavailable"
            } else {
                "error"
            };
            json!({ "status": status, "message": message })
        }
    }
}

fn measure(id: EngineId, mode: Mode, files: &[PathBuf], count: usize) -> Result<Value, String> {
    let stream = changes(mode, files, count)?;
    let mut engine = id.make().ok_or("not an in-process engine")?;

    let start = Instant::now();
    for f in files {
        engine.load(f)?;
    }
    let load_ms = ms(start);

    let start = Instant::now();
    engine.reason();
    let reason_ms = ms(start);

    let mut change_ms = Vec::with_capacity(stream.len());
    if mode != Mode::Full {
        engine.prepare_changes();
        for t in &stream {
            let start = Instant::now();
            match mode {
                Mode::Add => engine.insert(t),
                _ => engine.remove(t),
            }
            engine.reason();
            change_ms.push(ms(start));
        }
    }
    // Read the peak before the checks below, which hold their own copies.
    let peak_rss_bytes = crate::machine::peak_rss_bytes(libc::RUSAGE_SELF);
    let (reported, counted) = engine.closure();
    Ok(json!({
        "status": "ok",
        "load_ms": load_ms,
        "reason_ms": reason_ms,
        "change_ms": change_ms,
        "peak_rss_bytes": peak_rss_bytes,
        "closure": reported,
        "closure_counted": counted,
    }))
}
