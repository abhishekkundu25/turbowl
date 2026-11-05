//! The scenarios and where their inputs come from: fetched models under
//! `data/` (found by file name, wherever `scripts/fetch-data.sh` puts them)
//! or data generated here into `target/bench-data/`.

use crate::engine::Mode;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Where a scenario's input comes from.
#[derive(Debug, Clone, Copy)]
enum Input {
    /// Brick and one named building model from `data/`.
    Brick(&'static str),
    /// Brick and every building model in a `buildings/` folder under `data/`.
    BrickBuildings,
    /// The univ-bench ontology and generated data for this many universities.
    Lubm(u64),
    /// One `owl:sameAs` clique of this many aliases, each with 10 triples.
    SameAs(u64),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Scenario {
    pub(crate) name: &'static str,
    pub(crate) about: &'static str,
    input: Input,
    pub(crate) mode: Mode,
}

pub(crate) const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "brick-small1",
        about: "Brick plus small1.n3 (reasonable's README example), one full run",
        input: Input::Brick("small1.n3"),
        mode: Mode::Full,
    },
    Scenario {
        name: "brick-soda-hall",
        about: "Brick plus soda_hall.n3 (a building model), one full run",
        input: Input::Brick("soda_hall.n3"),
        mode: Mode::Full,
    },
    Scenario {
        name: "brick-buildings",
        about: "Brick plus every building model of reasonable's Python benchmark, loaded together, one full run",
        input: Input::BrickBuildings,
        mode: Mode::Full,
    },
    Scenario {
        name: "lubm-1",
        about: "LUBM-style data, 1 university (about 0.1 M triples), with the univ-bench axioms",
        input: Input::Lubm(1),
        mode: Mode::Full,
    },
    Scenario {
        name: "lubm-10",
        about: "LUBM-style data, 10 universities (about 1.3 M triples)",
        input: Input::Lubm(10),
        mode: Mode::Full,
    },
    Scenario {
        name: "lubm-50",
        about: "LUBM-style data, 50 universities (about 6.5 M triples)",
        input: Input::Lubm(50),
        mode: Mode::Full,
    },
    Scenario {
        name: "incremental-add",
        about: "Brick plus small1, then single-triple additions, reason() after each; per-change median",
        input: Input::Brick("small1.n3"),
        mode: Mode::Add,
    },
    Scenario {
        name: "incremental-remove",
        about: "Brick plus small1, then single-triple removals, reason() after each; per-change median",
        input: Input::Brick("small1.n3"),
        mode: Mode::Remove,
    },
    Scenario {
        name: "sameas-clique",
        about: "one owl:sameAs clique of 1,000 aliases with 10 triples each (about 11 M closure triples expanded)",
        input: Input::SameAs(1000),
        mode: Mode::Full,
    },
];

pub(crate) fn find(name: &str) -> Option<&'static Scenario> {
    SCENARIOS.iter().find(|s| s.name == name)
}

/// The first file named `name` under `dir`, searching in sorted order.
fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    find_entry(dir, &|p| p.is_file() && p.file_name().is_some_and(|n| n == name))
}

fn find_entry(dir: &Path, want: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).collect();
    entries.sort();
    if let Some(hit) = entries.iter().find(|p| want(p)) {
        return Some(hit.clone());
    }
    entries.iter().filter(|p| p.is_dir()).find_map(|p| find_entry(p, want))
}

fn brick(data: &Path) -> Result<PathBuf, String> {
    find_file(data, "Brick.n3").ok_or_else(|| missing(data, "Brick.n3"))
}

fn missing(data: &Path, what: &str) -> String {
    format!(
        "{what} not found under {}; run scripts/fetch-data.sh (or pass --data-dir)",
        data.display()
    )
}

impl Scenario {
    /// The files to load, in order, generating them first when needed.
    /// `Err` is the reason the scenario is skipped.
    pub(crate) fn inputs(&self, data: &Path, generated: &Path) -> Result<Vec<PathBuf>, String> {
        match self.input {
            Input::Brick(model) => {
                let brick = brick(data)?;
                let model = find_file(data, model).ok_or_else(|| missing(data, model))?;
                Ok(vec![brick, model])
            }
            Input::BrickBuildings => {
                let brick = brick(data)?;
                let dir = find_entry(data, &|p| p.is_dir() && p.file_name().is_some_and(|n| n == "buildings"))
                    .ok_or_else(|| missing(data, "a buildings/ folder"))?;
                let mut models: Vec<PathBuf> = std::fs::read_dir(&dir)
                    .map_err(|e| format!("{}: {e}", dir.display()))?
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|e| e == "ttl"))
                    .collect();
                if models.is_empty() {
                    return Err(format!("no .ttl building models in {}", dir.display()));
                }
                models.sort();
                Ok(std::iter::once(brick).chain(models).collect())
            }
            Input::Lubm(n) => {
                let ontology = generated.join("univ-bench.ttl");
                write_if_changed(&ontology, crate::lubm::ONTOLOGY.as_bytes())?;
                // The generator's version is in the name: a change to it makes new files.
                let path = generated.join(format!("lubm-v1-{n}.ttl"));
                generate_once(&path, |w| crate::lubm::generate(n, w))?;
                Ok(vec![ontology, path])
            }
            Input::SameAs(n) => {
                let path = generated.join(format!("sameas-v1-{n}.ttl"));
                generate_once(&path, |w| sameas_clique(n, w))?;
                Ok(vec![path])
            }
        }
    }
}

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if std::fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes a generated file unless it is already there (written whole: a
/// partial file is only ever under a temporary name).
fn generate_once(
    path: &Path,
    make: impl FnOnce(&mut BufWriter<std::fs::File>) -> std::io::Result<u64>,
) -> Result<(), String> {
    if path.is_file() {
        return Ok(());
    }
    let err = |e: std::io::Error| format!("{}: {e}", path.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    let partial = path.with_extension("partial");
    let start = std::time::Instant::now();
    let mut w = BufWriter::with_capacity(1 << 20, std::fs::File::create(&partial).map_err(err)?);
    let triples = make(&mut w).map_err(err)?;
    w.flush().map_err(err)?;
    drop(w);
    std::fs::rename(&partial, path).map_err(err)?;
    eprintln!(
        "generated {} ({triples} triples) in {:.1} s",
        path.display(),
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

/// One clique: `a0 owl:sameAs a1`, ..., a chain of `n` aliases (the closure
/// makes it a clique), each alias with 10 triples of its own.
fn sameas_clique(n: u64, w: &mut impl Write) -> std::io::Result<u64> {
    const SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";
    const EX: &str = "http://example.org/sameas/";
    let mut triples = 0;
    for i in 0..n {
        if i + 1 < n {
            writeln!(w, "<{EX}a{i}> <{SAME_AS}> <{EX}a{}> .", i + 1)?;
            triples += 1;
        }
        for j in 0..10 {
            writeln!(w, "<{EX}a{i}> <{EX}p{j}> <{EX}v{i}-{j}> .")?;
            triples += 1;
        }
    }
    Ok(triples)
}
