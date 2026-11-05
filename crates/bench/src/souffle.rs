//! Soufflé as a competitor: the pinned Homebrew bottle of Soufflé 2.5
//! (`tools/souffle/`, fetched by `scripts/fetch-tools.sh`) compiles the
//! W3C forms of the rules of a configuration, emitted for Soufflé by
//! `turbowl_oracle::programs::souffle`, to a binary, which runs as a
//! subprocess of the `--one` child.
//!
//! - The binary is compiled once per program (with `/usr/bin/c++`, Xcode's
//!   command-line tools) and kept under `target/bench-souffle/bin/`; the
//!   compile is not timed, as building this reasoner is not.
//! - It reads `input.tsv` from the converted input's directory (`-F`) and
//!   writes `closure.tsv` and the findings into the run's directory (`-D`).
//! - Soufflé reports no split of its run, so its time is the binary's wall
//!   clock: reading the input, reasoning and writing the outputs, one
//!   number, recorded as the total.
//! - The bottle is built without OpenMP, so it runs on one thread.

use crate::convert::Encoded;
use crate::engine::{EngineId, Tool};
use serde_json::{Value, json};
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use turbowl::Switches;

/// The pinned bottle (`scripts/fetch-tools.sh`).
pub(crate) const TOOL: Tool = Tool {
    dir: "souffle",
    version: "2.5",
    file: "bin/souffle",
};

/// One run of the compiled program on the converted input: a run result
/// like this reasoner's (`status` `ok`, `skipped` or `timeout`), or `Err`
/// when it failed.
pub(crate) fn measure(input: &Encoded, switches: Switches, timeout: Duration) -> Result<Value, String> {
    let souffle = match TOOL.path() {
        Ok(p) => p,
        Err(why) => return Ok(json!({ "status": "skipped", "message": why })),
    };
    let binary = compiled(&souffle, &turbowl_oracle::programs::souffle(switches), timeout)?;
    let dir = crate::engine::scratch(EngineId::Souffle)?;
    let out = dir.join("out");
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut cmd = Command::new(binary);
    cmd.arg("-F").arg(input.dir()).arg("-D").arg(&out);
    let result = run(&mut cmd, &dir, input, timeout);
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// The program compiled to a binary: from `target/bench-souffle/bin/` when
/// it is there, else compiled now (under a temporary name, renamed whole).
fn compiled(souffle: &Path, program: &str, timeout: Duration) -> Result<PathBuf, String> {
    let key = BuildHasherDefault::<DefaultHasher>::default().hash_one((TOOL.version, program));
    let root = crate::repo_root().join("target/bench-souffle/bin");
    let dir = root.join(format!("{key:016x}"));
    let binary = dir.join("program");
    if binary.is_file() {
        return Ok(binary);
    }
    let partial = root.join(format!("{key:016x}.partial"));
    let io = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    let _ = std::fs::remove_dir_all(&partial);
    std::fs::create_dir_all(&partial).map_err(|e| io(&partial, e))?;
    std::fs::write(partial.join("program.dl"), program).map_err(|e| io(&partial, e))?;
    let mut cmd = Command::new(souffle);
    cmd.args(["--no-preprocessor", "-o", "program", "program.dl"]);
    let start = std::time::Instant::now();
    let ran = crate::engine::run_tool(&mut cmd, &partial, None, timeout)
        .and_then(|ran| ran.ok_or_else(|| format!("compiling the Soufflé program took over {} s", timeout.as_secs())));
    if let Err(e) = ran {
        let _ = std::fs::remove_dir_all(&partial);
        return Err(e);
    }
    // macOS checks a new binary at its first launch (0.4 s here), so the
    // first launch is not a measured one.
    let _ = Command::new(partial.join("program"))
        .arg("--help")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&partial, &dir).map_err(|e| io(&dir, e))?;
    eprintln!(
        "compiled the Soufflé program into {} in {:.1} s",
        binary.display(),
        start.elapsed().as_secs_f64()
    );
    Ok(binary)
}

/// Runs the command in `dir`, then reads `out/closure.tsv`.
fn run(cmd: &mut Command, dir: &Path, input: &Encoded, timeout: Duration) -> Result<Value, String> {
    let Some(ran) = crate::engine::run_tool(cmd, dir, None, timeout)? else {
        return Ok(crate::engine::timed_out(timeout));
    };
    let (closure, counted) = input.closure(&dir.join("out/closure.tsv"), crate::convert::tsv_terms)?;
    Ok(json!({
        "status": "ok",
        "load_ms": null,
        "reason_ms": null,
        "total_ms": ran.wall_ms,
        "wall_ms": ran.wall_ms,
        "change_ms": [],
        "peak_rss_bytes": ran.peak_bytes,
        "closure": closure,
        "closure_counted": counted,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::tests::{CLOSURE, fixture, remove, sh};

    /// A stand-in for the compiled program that writes a closure is read,
    /// its wall clock as the total; one that fails is an error with its
    /// message; one that runs over the timeout is a timeout.
    #[test]
    fn reads_a_run_and_its_failures() {
        let (input, dir) = fixture("souffle");
        let lines: String = CLOSURE.iter().map(|t| t.join("\\t") + "\\n").collect();
        let ok = run(
            &mut sh(&format!("mkdir -p out && printf '{lines}' > out/closure.tsv")),
            &dir,
            &input,
            Duration::from_secs(30),
        )
        .unwrap();
        assert_eq!(ok["status"], "ok", "{ok}");
        assert!(
            ok["reason_ms"].is_null() && ok["total_ms"].as_f64().is_some_and(|t| t > 0.0),
            "{ok}"
        );
        assert_eq!(
            (ok["closure"].as_u64(), ok["closure_counted"].as_u64()),
            (Some(2), Some(2)),
            "{ok}"
        );

        let failed = run(
            &mut sh("echo 'Error loading input.tsv' >&2; exit 1"),
            &dir,
            &input,
            Duration::from_secs(30),
        );
        assert!(failed.unwrap_err().contains("Error loading input.tsv"));
        std::fs::remove_dir_all(dir.join("out")).unwrap();
        let no_output = run(&mut sh("exit 0"), &dir, &input, Duration::from_secs(30));
        assert!(no_output.unwrap_err().contains("closure.tsv"));
        let slow = run(&mut sh("sleep 10"), &dir, &input, Duration::from_millis(200)).unwrap();
        assert_eq!(slow["status"], "timeout");
        remove(&input, &dir);
    }

    /// A compiler that fails leaves no binary behind and says why.
    #[test]
    fn a_failed_compile_is_an_error() {
        let program = format!("// a program no compiler accepts ({})\n", std::process::id());
        let err = compiled(Path::new("/usr/bin/false"), &program, Duration::from_secs(30)).unwrap_err();
        assert!(err.contains("exited with"), "{err}");
    }
}
