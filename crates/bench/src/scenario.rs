//! The scenarios and where their inputs come from: fetched models under
//! `data/` (found by file name, wherever `scripts/fetch-data.sh` puts them),
//! the public datasets at the paths that script gives them (`datasets.rs`),
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
    /// A public dataset's files.
    Fetched(Fetched),
}

/// A public dataset's files and what the papers report on them.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Fetched {
    /// The files to load, in order, relative to `data/`.
    pub(crate) files: &'static [&'static str],
    /// What `scripts/fetch-data.sh` needs to fetch them: "" for its default
    /// set, else its options.
    pub(crate) fetch: &'static str,
    /// The papers' fact counts, before and after reasoning.
    pub(crate) published: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Scenario {
    pub(crate) name: &'static str,
    pub(crate) about: &'static str,
    input: Input,
    pub(crate) mode: Mode,
    /// Left out of the default run; runs only when named with `--only`.
    pub(crate) opt_in: bool,
}

pub(crate) const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "brick-small1",
        about: "Brick plus small1.n3 (reasonable's README example), one full run",
        input: Input::Brick("small1.n3"),
        mode: Mode::Full,
        opt_in: false,
    },
    Scenario {
        name: "brick-soda-hall",
        about: "Brick plus soda_hall.n3 (a building model), one full run",
        input: Input::Brick("soda_hall.n3"),
        mode: Mode::Full,
        opt_in: false,
    },
    Scenario {
        name: "brick-buildings",
        about: "Brick plus every building model of reasonable's Python benchmark, loaded together, one full run",
        input: Input::BrickBuildings,
        mode: Mode::Full,
        opt_in: false,
    },
    Scenario {
        name: "lubm-1",
        about: "LUBM-style data, 1 university (about 0.1 M triples), with the univ-bench axioms",
        input: Input::Lubm(1),
        mode: Mode::Full,
        opt_in: false,
    },
    Scenario {
        name: "lubm-10",
        about: "LUBM-style data, 10 universities (about 1.3 M triples)",
        input: Input::Lubm(10),
        mode: Mode::Full,
        opt_in: false,
    },
    Scenario {
        name: "lubm-50",
        about: "LUBM-style data, 50 universities (about 6.5 M triples)",
        input: Input::Lubm(50),
        mode: Mode::Full,
        opt_in: false,
    },
    Scenario {
        name: "incremental-add",
        about: "Brick plus small1, then single-triple additions, reason() after each; per-change median",
        input: Input::Brick("small1.n3"),
        mode: Mode::Add,
        opt_in: false,
    },
    Scenario {
        name: "incremental-remove",
        about: "Brick plus small1, then single-triple removals, reason() after each; per-change median",
        input: Input::Brick("small1.n3"),
        mode: Mode::Remove,
        opt_in: false,
    },
    Scenario {
        name: "sameas-clique",
        about: "one owl:sameAs clique of 1,000 aliases with 10 triples each (about 11 M closure triples expanded)",
        input: Input::SameAs(1000),
        mode: Mode::Full,
        opt_in: true,
    },
];

/// Every scenario: the bench's own, then the public datasets.
pub(crate) fn all() -> impl Iterator<Item = &'static Scenario> {
    SCENARIOS.iter().chain(crate::datasets::DATASETS)
}

pub(crate) fn find(name: &str) -> Option<&'static Scenario> {
    all().find(|s| s.name == name)
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
    /// An opt-in full run on a public dataset.
    pub(crate) const fn fetched(name: &'static str, about: &'static str, files: Fetched) -> Scenario {
        Scenario {
            name,
            about,
            input: Input::Fetched(files),
            mode: Mode::Full,
            opt_in: true,
        }
    }

    /// The papers' fact counts, for a public dataset.
    pub(crate) fn published(&self) -> Option<&'static str> {
        match self.input {
            Input::Fetched(f) => Some(f.published),
            _ => None,
        }
    }

    /// The family of scenarios that differ only in input size, whose growth
    /// the "Scaling" table fits (`lubm-1`, `lubm-10`, `lubm-50`).
    pub(crate) fn family(&self) -> Option<&'static str> {
        match self.input {
            Input::Lubm(_) => Some("lubm"),
            _ => None,
        }
    }

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
            Input::Fetched(f) => f
                .files
                .iter()
                .map(|file| {
                    let path = data.join(file);
                    path.is_file().then_some(path).ok_or_else(|| {
                        let space = if f.fetch.is_empty() { "" } else { " " };
                        format!(
                            "not fetched: no {file} under {}; run scripts/fetch-data.sh{space}{}",
                            data.display(),
                            f.fetch
                        )
                    })
                })
                .collect(),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every name is unique and every public dataset is opt-in, with each
    /// file it loads written by the fetch script at that path.
    #[test]
    fn the_datasets_match_the_fetch_script() {
        let names: Vec<&str> = all().map(|s| s.name).collect();
        for (i, n) in names.iter().enumerate() {
            assert!(!names[..i].contains(n), "{n} twice");
        }
        let script = std::fs::read_to_string(crate::repo_root().join("scripts/fetch-data.sh")).unwrap();
        for s in crate::datasets::DATASETS {
            let Input::Fetched(f) = s.input else {
                panic!("{} is not a dataset", s.name)
            };
            assert!(s.opt_in && s.mode == Mode::Full, "{}", s.name);
            for file in f.files {
                let (dir, name) = file.rsplit_once('/').unwrap();
                let written = match name.strip_prefix("lubm-").and_then(|n| n.strip_suffix(".owl")) {
                    // UBA's data, generated with --lubm N.
                    Some(n) if dir == "uba" => {
                        f.fetch == format!("--lubm {n}") && script.contains("$data/uba/lubm-$n.owl")
                    }
                    _ => script.lines().any(|l| {
                        (l.contains(&format!(" {dir} ")) || l.contains(&format!("$data/{dir}/"))) && l.contains(name)
                    }),
                };
                assert!(written, "{}: the fetch script does not write {file}", s.name);
            }
        }
    }
}
