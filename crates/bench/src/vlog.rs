//! VLog as a competitor: VLog 1.3.7, as Rulewerk 0.9.0's command-line
//! client ships it (`tools/vlog/`, fetched by `scripts/fetch-tools.sh`;
//! its VLog libraries are universal, x86_64 and arm64), run on the fetched
//! JRE (`tools/jre/`) as a subprocess of the `--one` child. It runs the W3C
//! forms of the rules of a configuration, emitted in Rulewerk's syntax by
//! `turbowl_oracle::programs::vlog`.
//!
//! - The client's `materialize` command cannot be reached (its `main` hands
//!   the subcommand's name to the subcommand's own parser, which rejects
//!   it), so the bench drives the client's shell through its standard
//!   input: `@load` the program, `@reason`, then one `@query ... EXPORTCSV`
//!   for each export the program lists in its closing comments.
//! - The program reads `input.nt`, the converted input, and the literal
//!   facts beside it (`literal-types.nt`, `literal-values.nt`) by their full
//!   paths: Rulewerk does not resolve a relative path against the working
//!   directory.
//! - VLog reports one time for loading and reasoning together ("Loading and
//!   materializing inferences ... finished in"), recorded as the total; each
//!   export reports its own, summed as the export time.
//! - The shell exits 0 after an error: an `Error:` line in its output is a
//!   failed run.
//! - Rulewerk runs VLog on one thread (it exposes none of VLog's thread
//!   options).

use crate::convert::Encoded;
use crate::engine::{EngineId, Tool};
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use turbowl::Switches;

/// The pinned client jar (`scripts/fetch-tools.sh`), versioned by the VLog
/// it bundles.
pub(crate) const TOOL: Tool = Tool {
    dir: "vlog",
    version: "1.3.7",
    file: "rulewerk-client-0.9.0.jar",
};

/// The JRE the client runs on.
const JRE: Tool = Tool {
    dir: "jre",
    version: "21.0.13+9",
    file: "bin/java",
};

/// The source line the program reads its input with, as the rule source
/// names it.
const SOURCE: &str = "load-rdf(\"input.nt\")";

/// The other files a program may read, beside the input (the literal facts
/// of `trivial.rls` and `literal-values.rls`).
const BESIDE: [&str; 2] = ["literal-types.nt", "literal-values.nt"];

/// One run of VLog on the converted input: a run result like this
/// reasoner's (`status` `ok`, `skipped` or `timeout`), or `Err` when it
/// failed.
pub(crate) fn measure(input: &Encoded, switches: Switches, timeout: Duration) -> Result<Value, String> {
    let (jar, java) = match (TOOL.path(), JRE.path()) {
        (Ok(jar), Ok(java)) => (jar, java),
        (Err(why), _) | (_, Err(why)) => return Ok(json!({ "status": "skipped", "message": why })),
    };
    let dir = crate::engine::scratch(EngineId::Vlog)?;
    let result = (|| {
        let (program, commands) = session(&turbowl_oracle::programs::vlog(switches), &input.ntriples(), &dir)?;
        let io = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
        std::fs::write(dir.join("program.rls"), program).map_err(|e| io(&dir, e))?;
        std::fs::write(dir.join("commands.txt"), commands).map_err(|e| io(&dir, e))?;
        let mut cmd = Command::new(java);
        cmd.arg("-jar").arg(jar).arg("shell");
        run(&mut cmd, &dir, input, timeout)
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// The program, reading its input from `input`, and the shell commands
/// that run it from `dir/program.rls` and export into `dir`.
fn session(program: &str, input: &Path, dir: &Path) -> Result<(String, String), String> {
    let quoted = |p: &Path| {
        p.to_str()
            .filter(|s| !s.contains('"'))
            .map(|s| format!("\"{s}\""))
            .ok_or_else(|| format!("{}: not a path Rulewerk can quote", p.display()))
    };
    if program.matches(SOURCE).count() != 1 {
        return Err(format!("the VLog program does not read its input once with {SOURCE}"));
    }
    let mut program = program.replace(SOURCE, &format!("load-rdf({})", quoted(input)?));
    let dir_of_input = input.parent().unwrap_or(Path::new(""));
    for file in BESIDE {
        let source = format!("load-rdf(\"{file}\")");
        program = program.replace(&source, &format!("load-rdf({})", quoted(&dir_of_input.join(file))?));
    }
    let mut commands = format!("@load {} .\n@reason .\n", quoted(&dir.join("program.rls"))?);
    for line in program.lines() {
        let Some(query) = line.strip_prefix("% @query ") else {
            continue;
        };
        let (query, file) = query
            .strip_suffix("\" .")
            .and_then(|q| q.split_once(" EXPORTCSV \""))
            .ok_or_else(|| format!("an export the bench cannot read: {line}"))?;
        commands.push_str(&format!("@query {query} EXPORTCSV {} .\n", quoted(&dir.join(file))?));
    }
    if !commands.contains("closure.csv") {
        return Err("the VLog program exports no closure.csv".into());
    }
    commands.push_str("@exit .\n");
    Ok((program, commands))
}

/// VLog's own times from the shell's output: loading and reasoning
/// together, and the exports summed, in milliseconds; `Err` with the first
/// error the shell printed.
fn report(text: &str) -> Result<(f64, f64), String> {
    if let Some(error) = text.lines().find(|l| l.contains("Error:")) {
        return Err(format!("VLog: {}", error.trim_start_matches("rulewerk> ").trim()));
    }
    let ms_after = |line: &str, marker: &str| -> Option<f64> {
        let rest = &line[line.find(marker)? + marker.len()..];
        rest[..rest.find("ms")?].trim().parse().ok()
    };
    let total = text
        .lines()
        .find_map(|l| ms_after(l, "... finished in "))
        .ok_or_else(|| format!("no reasoning time in VLog's output: {text:?}"))?;
    let export = text
        .lines()
        .filter_map(|l| ms_after(l, "Written query result file in "))
        .sum();
    Ok((total, export))
}

/// Runs the shell in `dir` on `commands.txt`, then reads `closure.csv`.
fn run(cmd: &mut Command, dir: &Path, input: &Encoded, timeout: Duration) -> Result<Value, String> {
    let Some(ran) = crate::engine::run_tool(cmd, dir, Some(&dir.join("commands.txt")), timeout)? else {
        return Ok(crate::engine::timed_out(timeout));
    };
    let (total_ms, export_ms) = report(&ran.stdout)?;
    let (closure, counted) = input.closure(&dir.join("closure.csv"), crate::convert::csv_terms)?;
    Ok(json!({
        "status": "ok",
        "load_ms": null,
        "reason_ms": null,
        "total_ms": total_ms,
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
    use std::path::PathBuf;

    const SHELL: &str = "rulewerk> Loaded 3 new fact(s), 72 new rule(s), and 1 new datasource declaration(s).\n\
        rulewerk> Loading and materializing inferences ...\n\
        ... finished in 959ms (949ms CPU time).\n\
        rulewerk> @query T(?x1, ?x2, ?x3)Written query result file in 12ms. This result is sound and complete.\n\
        rulewerk> @query caxDw(?x1, ?x2, ?x3)Written query result file in 3ms. This result is sound and complete.\n\
        rulewerk> Exiting Rulewerk shell ... bye.\n";

    #[test]
    fn reads_the_shell_output() {
        assert_eq!(report(SHELL), Ok((959.0, 15.0)));
        let failed = "rulewerk> Loading and materializing inferences ...\n\
            Error: While importing data for predicate \"input-3\": could not open file /x/input.nt\n";
        assert!(report(failed).unwrap_err().contains("could not open file /x/input.nt"));
        assert!(report("rulewerk> Exiting Rulewerk shell ... bye.\n").is_err());
    }

    /// The composed program reads its input by its full path and the
    /// session exports every relation the program lists into the run's
    /// directory, the closure first.
    #[test]
    fn the_session_runs_the_composed_program() {
        let (input, dir) = (PathBuf::from("/in/input.nt"), PathBuf::from("/run"));
        let composed = turbowl_oracle::programs::vlog(Switches::complete());
        let (program, commands) = session(&composed, &input, &dir).unwrap();
        assert!(program.contains("load-rdf(\"/in/input.nt\")") && !program.contains(SOURCE));
        for file in BESIDE {
            assert!(program.contains(&format!("load-rdf(\"/in/{file}\")")), "{file}");
        }
        let lines: Vec<&str> = commands.lines().collect();
        assert_eq!(lines[..2], ["@load \"/run/program.rls\" .", "@reason ."]);
        assert_eq!(lines[2], "@query T(?x1, ?x2, ?x3) EXPORTCSV \"/run/closure.csv\" .");
        assert_eq!(lines.last(), Some(&"@exit ."));
        assert_eq!(
            lines.len(),
            4 + turbowl_oracle::programs::codes(Switches::complete()).len()
        );
        assert!(session("% no input\n", &input, &dir).is_err());
        assert!(session(&composed, Path::new("/a\"b/input.nt"), &dir).is_err());
    }

    /// A stand-in for the shell that prints VLog's report and writes a
    /// closure is read; one that prints an error is an error though it exits
    /// 0; one that runs over the timeout is a timeout.
    #[test]
    fn reads_a_run_and_its_failures() {
        let (input, dir) = fixture("vlog");
        std::fs::write(dir.join("commands.txt"), "@exit .\n").unwrap();
        let lines: String = CLOSURE.iter().map(|t| t.join(",") + "\\n").collect();
        let script = format!("printf '{lines}' > closure.csv && cat <<'EOF'\n{SHELL}EOF");
        let ok = run(&mut sh(&script), &dir, &input, Duration::from_secs(30)).unwrap();
        assert_eq!(ok["status"], "ok", "{ok}");
        assert_eq!(
            (ok["total_ms"].as_f64(), ok["export_ms"].as_f64()),
            (Some(959.0), Some(15.0))
        );
        assert_eq!(
            (ok["closure"].as_u64(), ok["closure_counted"].as_u64()),
            (Some(2), Some(2)),
            "{ok}"
        );

        let failed = run(
            &mut sh("echo 'Error: knowledge base not loaded!'"),
            &dir,
            &input,
            Duration::from_secs(30),
        );
        assert!(failed.unwrap_err().contains("knowledge base not loaded"));
        let slow = run(&mut sh("sleep 10"), &dir, &input, Duration::from_millis(200)).unwrap();
        assert_eq!(slow["status"], "timeout");
        remove(&input, &dir);
    }
}
