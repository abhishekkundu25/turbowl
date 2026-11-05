//! Nemo as a competitor: the pinned `nmo` (`tools/nemo/`, fetched by
//! `scripts/fetch-tools.sh`) runs the W3C forms of the rules of a
//! configuration, composed from `oracles/rules/` by
//! `turbowl_oracle::programs`, as a subprocess of the `--one` child.
//!
//! The program imports `input.nt`, which `nmo -I` finds in the converted
//! input's directory (`convert.rs`). Nemo has no incremental changes: it
//! runs a program once, so only full runs are measured.

use crate::convert::Encoded;
use crate::engine::{EngineId, Tool};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use turbowl::Switches;

/// The pinned release (`scripts/fetch-tools.sh`).
pub(crate) const TOOL: Tool = Tool {
    dir: "nemo",
    version: "0.10.1",
    file: "nmo",
};

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

/// One run of `nmo` on the converted input: a run result like this
/// reasoner's (`status` `ok`, `skipped` or `timeout`), or `Err` when it
/// failed.
pub(crate) fn measure(input: &Encoded, switches: Switches, timeout: Duration) -> Result<Value, String> {
    let nmo = match TOOL.path() {
        Ok(p) => p,
        Err(why) => return Ok(json!({ "status": "skipped", "message": why })),
    };
    let dir = crate::engine::scratch(EngineId::Nemo)?;
    let program = dir.join("program.rls");
    std::fs::write(&program, turbowl_oracle::programs::nemo(switches))
        .map_err(|e| format!("{}: {e}", program.display()))?;
    let mut cmd = Command::new(nmo);
    cmd.args(["--overwrite-results", "--report", "short", "-I"])
        .arg(input.dir())
        .arg("-D")
        .arg(dir.join("out"))
        .arg(&program);
    let result = run(&mut cmd, &dir, input, timeout);
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// Runs the command in `dir`, then reads its report and `out/closure.nt`.
fn run(cmd: &mut Command, dir: &Path, input: &Encoded, timeout: Duration) -> Result<Value, String> {
    let Some(ran) = crate::engine::run_tool(cmd, dir, None, timeout)? else {
        return Ok(crate::engine::timed_out(timeout));
    };
    let (load_ms, reason_ms, export_ms) =
        report(&ran.stdout).ok_or_else(|| format!("no timing report in nmo's output: {:?}", ran.stdout))?;
    let (closure, counted) = input.closure(&dir.join("out/closure.nt"), crate::convert::ntriples_terms)?;
    Ok(json!({
        "status": "ok",
        "load_ms": load_ms,
        "reason_ms": reason_ms,
        "export_ms": export_ms,
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

    #[test]
    fn reads_the_short_report() {
        let text = "Reasoning completed in 1691ms. Derived 33010 facts.\n   Data import:     68ms\n   Reasoning:     1611ms\n   Data export:     11ms\n";
        assert_eq!(report(text), Some((68.0, 1611.0, 11.0)));
        assert_eq!(report("nothing"), None);
    }

    /// A stand-in for `nmo` that prints a report and writes a closure is
    /// read; one that fails is an error with its message; one without a
    /// report is an error; one that runs over the timeout is a timeout.
    #[test]
    fn reads_a_run_and_its_failures() {
        let (input, dir) = fixture("nemo");
        let lines: String = CLOSURE.iter().map(|[s, p, o]| format!("{s} {p} {o} .\\n")).collect();
        let script = format!(
            "mkdir -p out && printf '{lines}' > out/closure.nt && echo '  Data import: 2ms' && echo '  Reasoning: 3ms' && echo '  Data export: 1ms'"
        );
        let ok = run(&mut sh(&script), &dir, &input, Duration::from_secs(30)).unwrap();
        assert_eq!(ok["status"], "ok", "{ok}");
        assert_eq!(
            (ok["load_ms"].as_f64(), ok["reason_ms"].as_f64()),
            (Some(2.0), Some(3.0))
        );
        assert_eq!(
            (ok["closure"].as_u64(), ok["closure_counted"].as_u64()),
            (Some(2), Some(2)),
            "{ok}"
        );

        let failed = run(
            &mut sh("echo 'Error: no such file' >&2; exit 1"),
            &dir,
            &input,
            Duration::from_secs(30),
        );
        assert!(failed.unwrap_err().contains("no such file"));
        let silent = run(&mut sh("exit 0"), &dir, &input, Duration::from_secs(30));
        assert!(silent.unwrap_err().contains("no timing report"));
        let slow = run(&mut sh("sleep 10"), &dir, &input, Duration::from_millis(200)).unwrap();
        assert_eq!(slow["status"], "timeout");
        remove(&input, &dir);
    }
}
