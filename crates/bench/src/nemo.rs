//! Nemo as a baseline: the pinned `nmo` (`tools/nemo/`, fetched by
//! `scripts/fetch-tools.sh`) runs `oracles/nemo/owl2rl.rls`, the W3C forms of
//! the version 1 rule set, as a subprocess of the `--one` child.
//!
//! The program's one `@import input` line is replaced by one import per
//! input file, read by Nemo's own parser (Turtle for `.ttl` and `.n3`,
//! N-Triples for `.nt`). Nemo has no incremental changes: it runs a program
//! once, so only full runs are measured.

use oxttl::NTriplesParser;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The pinned release (`scripts/fetch-tools.sh`).
pub(crate) const VERSION: &str = "0.10.1";

fn tool_dir() -> PathBuf {
    crate::repo_root().join("tools/nemo")
}

/// The version in `tools/nemo/VERSION`, when Nemo is fetched.
pub(crate) fn installed_version() -> Option<String> {
    std::fs::read_to_string(tool_dir().join("VERSION"))
        .ok()
        .map(|v| v.trim().to_string())
}

/// Why Nemo cannot run here, or `None` when the pinned `nmo` is in place.
fn not_installed() -> Option<String> {
    match installed_version() {
        Some(v) if v == VERSION && tool_dir().join("nmo").is_file() => None,
        Some(v) => Some(format!("tools/nemo is {v}, not {VERSION}: run scripts/fetch-tools.sh")),
        None => Some("tools/nemo/nmo is missing: run scripts/fetch-tools.sh".into()),
    }
}

/// The rule program with its `@import input` line replaced by one import of
/// each file, in order.
pub(crate) fn program(rules: &str, files: &[PathBuf]) -> Result<String, String> {
    let is_import = |l: &str| {
        l.trim_start()
            .strip_prefix("@import")
            .is_some_and(|r| r.trim_start().starts_with("input ") || r.trim_start().starts_with("input:"))
    };
    let count = rules.lines().filter(|l| is_import(l)).count();
    if count != 1 {
        return Err(format!(
            "owl2rl.rls has {count} `@import input` lines; the bench replaces exactly one"
        ));
    }
    let mut imports = String::new();
    for f in files {
        let format = match f.extension().and_then(|e| e.to_str()) {
            Some("nt") => "ntriples",
            Some("ttl" | "n3") => "turtle",
            _ => return Err(format!("{}: Nemo reads only .nt, .ttl and .n3 here", f.display())),
        };
        let path = f
            .to_str()
            .ok_or_else(|| format!("{}: path is not UTF-8", f.display()))?;
        let path = path.replace('\\', "\\\\").replace('"', "\\\"");
        imports.push_str(&format!("@import input :- {format}{{resource=\"{path}\"}} .\n"));
    }
    let mut out = String::with_capacity(rules.len() + imports.len());
    for line in rules.lines() {
        if is_import(line) {
            out.push_str(&imports);
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    Ok(out)
}

/// Nemo's own split of its run (`--report short`): data import, reasoning
/// and data export, in milliseconds.
pub(crate) fn report(text: &str) -> Option<(f64, f64, f64)> {
    let field = |name: &str| {
        text.lines().find_map(|l| {
            l.trim()
                .strip_prefix(name)?
                .trim()
                .strip_suffix("ms")?
                .trim()
                .parse::<f64>()
                .ok()
        })
    };
    Some((field("Data import:")?, field("Reasoning:")?, field("Data export:")?))
}

/// One run of `nmo` on the files: a run result like the in-process engines'
/// (`status` `ok`, `skipped` or `timeout`), or `Err` when it failed.
pub(crate) fn measure(files: &[PathBuf], timeout: Duration) -> Result<Value, String> {
    if let Some(why) = not_installed() {
        return Ok(json!({ "status": "skipped", "message": why }));
    }
    let root = crate::repo_root();
    let rules_path = root.join("oracles/nemo/owl2rl.rls");
    let rules = std::fs::read_to_string(&rules_path).map_err(|e| format!("{}: {e}", rules_path.display()))?;
    let files: Vec<PathBuf> = files
        .iter()
        .map(|f| f.canonicalize().map_err(|e| format!("{}: {e}", f.display())))
        .collect::<Result<_, _>>()?;
    let program = program(&rules, &files)?;

    let dir = root.join(format!("target/bench-nemo/{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let result = run_in(&dir, &program, timeout);
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn run_in(dir: &Path, program: &str, timeout: Duration) -> Result<Value, String> {
    let io = |e: std::io::Error| format!("{}: {e}", dir.display());
    std::fs::write(dir.join("program.rls"), program).map_err(io)?;
    let stdout = std::fs::File::create(dir.join("stdout.txt")).map_err(io)?;
    let stderr = std::fs::File::create(dir.join("stderr.txt")).map_err(io)?;
    let out = dir.join("out");

    let start = Instant::now();
    let mut child = Command::new(tool_dir().join("nmo"))
        .args(["--overwrite-results", "--report", "short", "--export-dir"])
        .arg(&out)
        .arg(dir.join("program.rls"))
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .map_err(|e| format!("nmo: {e}"))?;
    let status = loop {
        match child.try_wait().map_err(|e| format!("nmo: {e}"))? {
            Some(status) => break status,
            None if start.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(json!({ "status": "timeout", "message": format!("over {} s", timeout.as_secs()) }));
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };
    let wall_ms = start.elapsed().as_secs_f64() * 1000.0;
    // `nmo` is the only child this process waited for, so the largest peak
    // of its children is Nemo's.
    let peak_rss_bytes = crate::machine::peak_rss_bytes(libc::RUSAGE_CHILDREN);

    let report_text = std::fs::read_to_string(dir.join("stdout.txt")).unwrap_or_default();
    if !status.success() {
        let err = std::fs::read_to_string(dir.join("stderr.txt")).unwrap_or_default();
        let tail: Vec<&str> = err.lines().rev().take(5).collect();
        let tail: Vec<&str> = tail.into_iter().rev().collect();
        return Err(format!("nmo exited with {status}: {}", tail.join(" / ")));
    }
    let (load_ms, reason_ms, export_ms) =
        report(&report_text).ok_or_else(|| format!("no timing report in nmo's output: {report_text:?}"))?;
    let (reported, counted) = closure(&out.join("closure.nt"))?;
    Ok(json!({
        "status": "ok",
        "load_ms": load_ms,
        "reason_ms": reason_ms,
        "export_ms": export_ms,
        "wall_ms": wall_ms,
        "change_ms": [],
        "peak_rss_bytes": peak_rss_bytes,
        "closure": reported,
        "closure_counted": counted,
    }))
}

/// The lines Nemo wrote to the closure file, and the distinct triples parsed
/// from it.
fn closure(path: &Path) -> Result<(usize, usize), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let lines = bytes
        .split(|&b| b == b'\n')
        .filter(|l| l.iter().any(|b| !b.is_ascii_whitespace()))
        .count();
    let triples: Vec<oxrdf::Triple> = NTriplesParser::new()
        .for_slice(&bytes)
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok((lines, crate::engine::distinct(triples.iter())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_the_one_import() {
        let rules = "% x\n@import input :- ntriples{resource=\"input.nt\"} .\nT(?s, ?p, ?o) :- input(?s, ?p, ?o) .\n";
        let files = [PathBuf::from("/d/Brick.n3"), PathBuf::from("/d/a\"b.nt")];
        let p = program(rules, &files).unwrap();
        assert_eq!(
            p,
            "% x\n@import input :- turtle{resource=\"/d/Brick.n3\"} .\n\
             @import input :- ntriples{resource=\"/d/a\\\"b.nt\"} .\n\
             T(?s, ?p, ?o) :- input(?s, ?p, ?o) .\n"
        );
    }

    #[test]
    fn refuses_a_program_without_one_import() {
        assert!(program("T(?s, ?p, ?o) :- input(?s, ?p, ?o) .\n", &[]).is_err());
        assert!(program("@import input :- a .\n@import input :- b .\n", &[]).is_err());
        assert!(program("@import input :- a .\n", &[PathBuf::from("x.rdf")]).is_err());
    }

    #[test]
    fn reads_the_short_report() {
        let text = "Reasoning completed in 1691ms. Derived 33010 facts.\n   Data import:     68ms\n   Reasoning:     1611ms\n   Data export:     11ms\n";
        assert_eq!(report(text), Some((68.0, 1611.0, 11.0)));
        assert_eq!(report("nothing"), None);
    }
}
