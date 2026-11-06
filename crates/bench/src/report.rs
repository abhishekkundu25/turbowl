//! `--scoreboard FILE...`: per scenario and axis, each engine's value and the
//! leader, for each configuration and thread mode the results files hold,
//! then the estimates RDFox's published results allow (`published.rs`).
//!
//! The leader is the engine with the lowest median. It leads only when its
//! range (min to max over the runs) lies below every other engine's; the
//! engines whose ranges overlap the lowest one's tie with it, as
//! `--compare` flags a change only when the ranges do not overlap. An
//! engine that did not finish, or does not report the axis, has no value
//! and cannot lead.

use crate::engine::EngineId;
use crate::published::{self, Counts, Here, RDFOX, Ran};
use serde_json::Value;
use std::fmt::Write;
use std::path::PathBuf;
use std::process::ExitCode;

/// What the scoreboard ranks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    /// The first `reason()` of a full run.
    Reasoning,
    /// Load plus reasoning.
    Total,
    /// The median time of one incremental change.
    Change,
    /// The largest peak memory.
    Memory,
}

impl Axis {
    fn name(self) -> &'static str {
        match self {
            Axis::Reasoning => "reasoning",
            Axis::Total => "total (load + reasoning)",
            Axis::Change => "per change",
            Axis::Memory => "peak memory",
        }
    }
}

/// Median, minimum and maximum over the runs.
type Spread = (f64, f64, f64);

/// One engine on one scenario and axis.
#[derive(Debug, Clone, PartialEq)]
enum Cell {
    Value(Spread),
    /// It finished, but does not report this number (Soufflé's and VLog's
    /// reasoning: they do not split it from loading).
    NotSplit,
    /// It did not finish: `timeout`, `over-memory`, `unsupported`, ...
    Status(String),
    /// It is not in the results.
    NotRun,
}

/// Who leads a scenario on an axis.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Leader {
    Lead(EngineId),
    /// The engines whose ranges overlap the lowest median's.
    Tie(Vec<EngineId>),
    /// No engine has a value.
    Nobody,
}

fn leader(cells: &[(EngineId, Cell)]) -> Leader {
    let values: Vec<(EngineId, Spread)> = cells
        .iter()
        .filter_map(|(e, c)| match c {
            Cell::Value(s) => Some((*e, *s)),
            _ => None,
        })
        .collect();
    let Some(&(_, best)) = values.iter().min_by(|a, b| a.1.0.total_cmp(&b.1.0)) else {
        return Leader::Nobody;
    };
    // A range overlaps the lowest one's when it starts at or below its top.
    let tied: Vec<EngineId> = values.iter().filter(|(_, s)| s.1 <= best.2).map(|(e, _)| *e).collect();
    match tied.as_slice() {
        [one] => Leader::Lead(*one),
        _ => Leader::Tie(tied),
    }
}

/// The spread of one axis over a results row's runs.
fn cell(row: Option<&Value>, axis: Axis) -> Cell {
    let Some(row) = row else { return Cell::NotRun };
    let status = row["status"].as_str().unwrap_or("?");
    if status != "ok" {
        return Cell::Status(status.to_string());
    }
    let runs = row["runs"].as_array().map(Vec::as_slice).unwrap_or_default();
    let per_run = |run: &Value| -> Option<f64> {
        match axis {
            Axis::Reasoning => run["reason_ms"].as_f64(),
            Axis::Total => run["total_ms"]
                .as_f64()
                .or_else(|| Some(run["load_ms"].as_f64()? + run["reason_ms"].as_f64()?)),
            Axis::Change => {
                let changes: Vec<f64> = run["change_ms"].as_array()?.iter().filter_map(Value::as_f64).collect();
                crate::spread(&changes).map(|s| s.0)
            }
            Axis::Memory => run["peak_rss_bytes"].as_f64(),
        }
    };
    let xs: Option<Vec<f64>> = runs.iter().map(per_run).collect();
    match xs.as_deref().and_then(crate::spread) {
        Some(s) => Cell::Value(s),
        None => Cell::NotSplit,
    }
}

fn show(c: &Cell, axis: Axis) -> String {
    let one = |x: f64| match axis {
        Axis::Memory => format!("{} MB", crate::mb(x)),
        _ if x >= 10_000.0 => format!("{:.1} s", x / 1000.0),
        _ => format!("{x:.1} ms"),
    };
    match c {
        Cell::Value((median, min, max)) if min == max => one(*median),
        Cell::Value((median, min, max)) => format!("{} ({}..{})", one(*median), one(*min), one(*max)),
        Cell::NotSplit => "n/a (not split)".into(),
        Cell::Status(s) => s.clone(),
        Cell::NotRun => "not run".into(),
    }
}

/// One configuration and thread mode: the rows of the files that ran it.
#[derive(Debug, Default)]
struct Group {
    files: Vec<String>,
    /// Whether the configuration has `equality` on.
    equality: bool,
    /// (scenario, engine, row), in the files' order; a later file's row
    /// replaces an earlier one's.
    rows: Vec<(String, String, Value)>,
    counts: Vec<(String, Counts)>,
}

impl Group {
    fn row(&self, scenario: &str, engine: EngineId) -> Option<&Value> {
        self.rows
            .iter()
            .find(|(s, e, _)| s == scenario && e == engine.name())
            .map(|(_, _, r)| r)
    }

    fn scenarios(&self) -> Vec<&str> {
        let mut seen: Vec<&str> = Vec::new();
        for (s, _, _) in &self.rows {
            if !seen.contains(&s.as_str()) {
                seen.push(s);
            }
        }
        seen
    }

    fn ran(&self, scenario: &str, engine: EngineId) -> Option<Ran> {
        let row = self.row(scenario, engine)?;
        let s = &row["summary"];
        Some(Ran {
            status: row["status"].as_str().unwrap_or("?").to_string(),
            reasoning_ms: s["reason_median_ms"].as_f64(),
            total_ms: s["total_median_ms"].as_f64(),
            closure: s["closure"].as_u64(),
        })
    }
}

/// The groups of the files, keyed by configuration and threads, in the order
/// the scoreboard prints them: default and complete, each with one thread
/// and all cores (empty when no file ran them), then any other
/// configuration.
fn groups(files: &[(String, Value)]) -> Vec<((String, u64), Group)> {
    let mut out: Vec<((String, u64), Group)> = Vec::new();
    for configuration in ["default", "complete"] {
        for threads in [1, 0] {
            out.push(((configuration.into(), threads), Group::default()));
        }
    }
    for (name, v) in files {
        let meta = &v["meta"];
        // Before schema 7 every file ran the default or the complete
        // configuration, both with equality on, on one thread; before
        // schema 9 `meta.threads` is 1 (README).
        let configuration = meta["configuration"].as_str().unwrap_or("default").to_string();
        let threads = match meta["threads"].as_u64().unwrap_or(1) {
            1 => 1,
            _ => 0,
        };
        let equality = meta["switches"]
            .as_array()
            .is_none_or(|on| on.iter().any(|s| s == "equality"));
        let key = (configuration, threads);
        let at = match out.iter().position(|(k, _)| *k == key) {
            Some(at) => at,
            None => {
                out.push((key, Group::default()));
                out.len() - 1
            }
        };
        let g = &mut out[at].1;
        g.files.push(name.clone());
        g.equality = equality;
        for row in v["results"].as_array().into_iter().flatten() {
            let (s, e) = (
                row["scenario"].as_str().unwrap_or("?").to_string(),
                row["engine"].as_str().unwrap_or("?").to_string(),
            );
            g.rows.retain(|(s2, e2, _)| (s2, e2) != (&s, &e));
            g.rows.push((s, e, row.clone()));
        }
        for c in v["published_counts"].as_array().into_iter().flatten() {
            if let (Some(s), Some(counts)) = (c["scenario"].as_str(), Counts::from_json(c)) {
                g.counts.retain(|(s2, _)| s2 != s);
                g.counts.push((s.to_string(), counts));
            }
        }
    }
    out
}

fn axes(scenario: &str, g: &Group) -> &'static [Axis] {
    let incremental = g
        .rows
        .iter()
        .filter(|(s, _, _)| s == scenario)
        .any(|(_, _, r)| r["summary"]["measure"] == "change_median_ms")
        || crate::scenario::find(scenario).is_some_and(|s| s.mode != crate::engine::Mode::Full);
    if incremental {
        &[Axis::Change, Axis::Memory]
    } else {
        &[Axis::Reasoning, Axis::Total, Axis::Memory]
    }
}

/// The scoreboard of the results files, as text.
fn scoreboard(files: &[(String, Value)]) -> String {
    let mut out = String::new();
    let o = &mut out;
    let _ = writeln!(o, "# Scoreboard\n");
    for (name, v) in files {
        let m = &v["meta"];
        let _ = writeln!(
            o,
            "- {name}: schema {}, {} at {}, configuration {}, {}, load average {} before",
            v["schema"],
            m["date"].as_str().unwrap_or("?"),
            m["commit"].as_str().map_or("?", |c| &c[..c.len().min(10)]),
            m["configuration"].as_str().unwrap_or("default"),
            if m["threads"].as_u64().unwrap_or(1) == 1 {
                "one thread"
            } else {
                "all cores"
            },
            m["load_before"][0].as_f64().map_or("?".into(), |l| format!("{l:.2}")),
        );
        if m["load_before"][0].as_f64().is_some_and(|l| l > 1.0) {
            let _ = writeln!(
                o,
                "  warning: not a quiet machine; its numbers show a trend, not a claim"
            );
        }
    }
    let _ = writeln!(
        o,
        "\nEach cell is the median over the runs (min..max). The leader has the lowest median and \
         leads only when its range lies below every other engine's; overlapping ranges tie. \
         Soufflé and VLog report load and reasoning as one time: their reasoning is n/a, and \
         Soufflé's total also holds writing its output (README)."
    );
    let groups = groups(files);
    for ((configuration, threads), g) in &groups {
        let mode = if *threads == 1 { "one thread" } else { "all cores" };
        let _ = writeln!(o, "\n## {configuration} configuration, {mode}\n");
        if g.rows.is_empty() {
            let _ = writeln!(
                o,
                "{}",
                if *threads == 1 {
                    "No results file ran it."
                } else {
                    "No results file ran it (`--threads all`)."
                }
            );
            continue;
        }
        let _ = writeln!(o, "From {}.\n", g.files.join(", "));
        let names: Vec<&str> = EngineId::ALL.iter().map(|e| e.name()).collect();
        let _ = writeln!(o, "| scenario | axis | {} | leader |", names.join(" | "));
        let _ = writeln!(o, "|---|---|{}---|", "---:|".repeat(names.len()));
        let (mut leads, mut ties, mut ranked) = (0, 0, 0);
        let mut behind = Vec::new();
        for scenario in g.scenarios() {
            for &axis in axes(scenario, g) {
                let cells: Vec<(EngineId, Cell)> = EngineId::ALL
                    .iter()
                    .map(|&e| (e, cell(g.row(scenario, e), axis)))
                    .collect();
                let lead = leader(&cells);
                let shown = match &lead {
                    Leader::Lead(e) => e.name().to_string(),
                    Leader::Tie(es) => format!("tie: {}", es.iter().map(|e| e.name()).collect::<Vec<_>>().join(", ")),
                    Leader::Nobody => "none finished".into(),
                };
                let values: Vec<String> = cells.iter().map(|(_, c)| show(c, axis)).collect();
                let _ = writeln!(o, "| {scenario} | {} | {} | {shown} |", axis.name(), values.join(" | "));
                match lead {
                    Leader::Nobody => {}
                    Leader::Lead(EngineId::Turbowl) => {
                        ranked += 1;
                        leads += 1;
                    }
                    Leader::Tie(es) if es.contains(&EngineId::Turbowl) => {
                        ranked += 1;
                        ties += 1;
                    }
                    other => {
                        ranked += 1;
                        let by = match other {
                            Leader::Lead(e) => e.name().to_string(),
                            Leader::Tie(es) => es.iter().map(|e| e.name()).collect::<Vec<_>>().join(" and "),
                            Leader::Nobody => unreachable!(),
                        };
                        behind.push(format!("{scenario} {} (led by {by})", axis.name()));
                    }
                }
            }
        }
        let _ = writeln!(
            o,
            "\nturbowl leads {leads} and ties {ties} of the {ranked} where an engine finished."
        );
        if !behind.is_empty() {
            let _ = writeln!(o, "It does not lead: {}.", behind.join("; "));
        }
    }
    estimates(o, &groups);
    out
}

/// The estimates section: every published row on a scenario that ran, with
/// a bridge that runs here, estimated or refused.
fn estimates(o: &mut String, groups: &[((String, u64), Group)]) {
    let all = published::all();
    let _ = writeln!(
        o,
        "\n## Estimates: RDFox through its published results\n\n\
         Never measured here, and always an estimate: RDFox's time in the paper x the bridge's time \
         here / the bridge's time in the paper, on the same job (`src/published.rs` says when). A \
         published RDFox number is a floor: RDFox has improved since (ADR 0006).\n"
    );
    let mut made = Vec::new();
    let mut refused = Vec::new();
    let mut cited: Vec<String> = Vec::new();
    // The bridges run one thread here, so only the one-thread sections.
    for ((configuration, _), g) in groups.iter().filter(|((_, t), g)| *t == 1 && !g.rows.is_empty()) {
        for scenario in g.scenarios() {
            for result in all.on(scenario) {
                let source = all.paper(&result.paper).map_or(result.paper.clone(), |p| {
                    format!("{} ({}; {})", p.id, p.place, p.machine)
                });
                for bridge in result.engines.iter().filter(|e| e.runs_here_as.is_some()) {
                    if !cited.contains(&result.paper) {
                        cited.push(result.paper.clone());
                    }
                    let engine = bridge.runs_here_as.expect("filtered");
                    let here = Here {
                        equality: g.equality,
                        counts: g.counts.iter().find(|(s, _)| s == scenario).map(|(_, c)| *c),
                        turbowl: g.ran(scenario, EngineId::Turbowl),
                        bridge: g.ran(scenario, engine),
                    };
                    for rdfox in result.engines.iter().filter(|e| e.engine == RDFOX) {
                        let what = format!(
                            "{scenario} | {configuration} | {source}, {} | RDFox{} on {} | {}",
                            result.variant.name(),
                            rdfox.build.as_deref().map_or(String::new(), |b| format!(" ({b})")),
                            rdfox.threads.map_or("threads not stated".into(), |t| format!(
                                "{t} thread{}",
                                if t == 1 { "" } else { "s" }
                            )),
                            bridge.engine,
                        );
                        match published::estimate(result, rdfox, bridge, &here) {
                            Ok(e) => made.push(format!(
                                "| {what} | {} | {} s / {} s | {:.1} s | **{:.1} s** | {} | {} |",
                                e.axis.name(),
                                e.rdfox_s,
                                e.bridge_s,
                                e.bridge_here_ms / 1000.0,
                                e.rdfox_here_ms / 1000.0,
                                e.turbowl_here_ms.map_or("-".into(), |t| format!(
                                    "{:.1} s ({:.2}x the estimate)",
                                    t / 1000.0,
                                    t / e.rdfox_here_ms
                                )),
                                rdfox.memory.as_deref().unwrap_or("not given per engine"),
                            )),
                            Err(why) => refused.push(format!("| {what} | {why} |")),
                        }
                    }
                }
            }
        }
    }
    if made.is_empty() {
        let _ = writeln!(o, "No estimate: no published result meets every condition.");
    } else {
        let _ = writeln!(
            o,
            "| scenario | configuration | source, variant | RDFox | bridge | time | RDFox / bridge in the paper | bridge here | RDFox here (estimate) | turbowl here | RDFox's memory in the paper (not scaled) |"
        );
        let _ = writeln!(o, "|---|---|---|---|---|---|---:|---:|---:|---:|---:|");
        for m in &made {
            let _ = writeln!(o, "{m}");
        }
    }
    if !refused.is_empty() {
        let _ = writeln!(o, "\nRefused:\n");
        let _ = writeln!(
            o,
            "| scenario | configuration | source, variant | RDFox | bridge | why not |"
        );
        let _ = writeln!(o, "|---|---|---|---|---|---|");
        for r in &refused {
            let _ = writeln!(o, "{r}");
        }
    }
    if !cited.is_empty() {
        let _ = writeln!(o, "\nSources (`published.json`):\n");
        for p in cited.iter().filter_map(|id| all.paper(id)) {
            let _ = writeln!(o, "- {}: {}. {}", p.id, p.cite, p.url);
        }
    }
}

pub(crate) fn run(paths: &[PathBuf]) -> ExitCode {
    let mut files = Vec::new();
    for path in paths {
        match crate::read_results(path) {
            Ok((v, _)) => files.push((path.display().to_string(), v)),
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        }
    }
    print!("{}", scoreboard(&files));
    ExitCode::SUCCESS
}

/// After a run: this reasoner's comparable counts against each paper's
/// counts on the same scenario.
pub(crate) fn print_counts(counts: &[(&str, Counts)]) {
    if counts.is_empty() {
        return;
    }
    let all = published::all();
    println!("Published counts: turbowl's before is the data's distinct triples; its after, the closure less");
    println!(
        "the triples of the ontology's own closure that are not data. A paper's count is printed to its precision."
    );
    println!();
    println!(
        "| scenario | turbowl before | turbowl after (closure - ontology alone) | paper | program | variant | paper before -> after | before | after |"
    );
    println!("|---|---:|---:|---|---|---|---|---|---|");
    for (scenario, c) in counts {
        for r in all.on(scenario) {
            let verdict = |count: &Option<published::Count>, n: u64| match count {
                Some(p) if p.matches(n as i64) => "matches".to_string(),
                Some(p) => format!("differs ({:+.1}%)", (n as f64 / p.value() - 1.0) * 100.0),
                None => "not published".into(),
            };
            let printed =
                |count: &Option<published::Count>| count.as_ref().map_or("?".to_string(), |c| c.printed.clone());
            println!(
                "| {scenario} | {} | {} ({} - {}) | {} | {} | {} | {} -> {} | {} | {} |",
                c.before,
                c.after,
                c.closure,
                c.ontology_only,
                r.paper,
                r.program.as_deref().unwrap_or("?"),
                r.variant.name(),
                printed(&r.before),
                printed(&r.after),
                verdict(&r.before, c.before),
                verdict(&r.after, c.after),
            );
        }
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spread(m: f64, lo: f64, hi: f64) -> Cell {
        Cell::Value((m, lo, hi))
    }

    #[test]
    fn the_leader_is_the_lowest_range_apart() {
        use EngineId::*;
        let cells = [
            (Turbowl, spread(10.0, 9.0, 11.0)),
            (Nemo, spread(20.0, 19.0, 21.0)),
            (Souffle, Cell::NotSplit),
            (Vlog, Cell::NotRun),
        ];
        assert_eq!(leader(&cells), Leader::Lead(Turbowl));
        // Nemo's range reaches into turbowl's: a tie, whatever the medians.
        let cells = [(Turbowl, spread(10.0, 9.0, 11.0)), (Nemo, spread(12.0, 10.5, 13.0))];
        assert_eq!(leader(&cells), Leader::Tie(vec![Turbowl, Nemo]));
        // One run each, equal: a tie.
        let cells = [(Turbowl, spread(5.0, 5.0, 5.0)), (Vlog, spread(5.0, 5.0, 5.0))];
        assert_eq!(leader(&cells), Leader::Tie(vec![Turbowl, Vlog]));
        // turbowl did not finish: it cannot lead.
        let cells = [
            (Turbowl, Cell::Status("over-memory".into())),
            (Nemo, spread(3.0, 3.0, 3.0)),
        ];
        assert_eq!(leader(&cells), Leader::Lead(Nemo));
        assert_eq!(leader(&[(Turbowl, Cell::NotRun)]), Leader::Nobody);
    }

    fn row(scenario: &str, engine: &str, status: &str, runs: Value, summary: Value) -> Value {
        json!({ "scenario": scenario, "engine": engine, "status": status, "runs": runs, "summary": summary })
    }

    fn file(configuration: &str, switches: Option<Vec<&str>>, results: Vec<Value>, counts: Value) -> Value {
        json!({
            "schema": 7,
            "meta": { "configuration": configuration, "switches": switches, "threads": 1, "date": "d", "commit": "c", "load_before": [0.5, 0.5, 0.5] },
            "results": results,
            "published_counts": counts,
        })
    }

    #[test]
    fn prints_values_leaders_and_missing_engines() {
        let full = |reason: f64, load: f64, peak: f64| json!([{ "load_ms": load, "reason_ms": reason, "peak_rss_bytes": peak }]);
        let results = vec![
            row(
                "lubm-1",
                "turbowl",
                "ok",
                full(10.0, 50.0, 100.0 * 1048576.0),
                json!({ "measure": "reason_ms" }),
            ),
            // Nemo reasons faster but loads slower and holds more.
            row(
                "lubm-1",
                "nemo",
                "ok",
                full(5.0, 200.0, 300.0 * 1048576.0),
                json!({ "measure": "reason_ms" }),
            ),
            row(
                "lubm-1",
                "vlog",
                "ok",
                json!([{ "load_ms": null, "reason_ms": null, "total_ms": 60.0, "peak_rss_bytes": 100.0 * 1048576.0 }]),
                json!({ "measure": "reason_ms" }),
            ),
            row("lubm-1", "souffle", "timeout", json!([]), Value::Null),
        ];
        let text = scoreboard(&[(
            "a.json".into(),
            file("default", Some(vec!["equality"]), results, json!([])),
        )]);
        assert!(
            text.contains("| lubm-1 | reasoning | 10.0 ms | 5.0 ms | timeout | n/a (not split) | nemo |"),
            "{text}"
        );
        assert!(
            text.contains(
                "| lubm-1 | total (load + reasoning) | 60.0 ms | 205.0 ms | timeout | 60.0 ms | tie: turbowl, vlog |"
            ),
            "{text}"
        );
        assert!(
            text.contains("| lubm-1 | peak memory | 100 MB | 300 MB | timeout | 100 MB | tie: turbowl, vlog |"),
            "{text}"
        );
        assert!(
            text.contains("turbowl leads 0 and ties 2 of the 3 where an engine finished."),
            "{text}"
        );
        assert!(
            text.contains("It does not lead: lubm-1 reasoning (led by nemo)."),
            "{text}"
        );
        // The thread modes and configurations no file ran.
        assert!(text.contains("## default configuration, all cores\n\nNo results file ran it (`--threads all`)."));
        assert!(text.contains("## complete configuration, one thread\n\nNo results file ran it."));
        assert!(text.contains("No estimate: no published result meets every condition."));
    }

    /// A file run with `--threads all` (`meta.threads` 0) fills the
    /// all-cores section; one with 1, or without the field, the one-thread
    /// section.
    #[test]
    fn groups_by_thread_mode() {
        let ok = |median: f64| {
            row(
                "lubm-1",
                "turbowl",
                "ok",
                json!([{ "load_ms": 1.0, "reason_ms": median, "peak_rss_bytes": 1048576.0 }]),
                json!({ "measure": "reason_ms" }),
            )
        };
        let mut all = file("default", None, vec![ok(4.0)], json!([]));
        all["schema"] = json!(9);
        all["meta"]["threads"] = json!(0);
        let mut old = file("default", None, vec![ok(9.0)], json!([]));
        old["meta"].as_object_mut().unwrap().remove("threads");
        let one = file("default", None, vec![ok(8.0)], json!([]));
        let text = scoreboard(&[
            ("all.json".into(), all),
            ("old.json".into(), old),
            ("one.json".into(), one),
        ]);
        assert!(
            text.contains("- all.json: schema 9, d at c, configuration default, all cores,"),
            "{text}"
        );
        assert!(
            text.contains("- old.json: schema 7, d at c, configuration default, one thread,"),
            "{text}"
        );
        let section = |heading: &str| {
            let start = text.find(heading).unwrap_or_else(|| panic!("{heading}: {text}"));
            let rest = &text[start + heading.len()..];
            rest[..rest.find("\n## ").unwrap_or(rest.len())].to_string()
        };
        let one_thread = section("## default configuration, one thread\n");
        assert!(one_thread.contains("From old.json, one.json."), "{one_thread}");
        assert!(one_thread.contains("| lubm-1 | reasoning | 8.0 ms |"), "{one_thread}");
        let all_cores = section("## default configuration, all cores\n");
        assert!(all_cores.contains("From all.json."), "{all_cores}");
        assert!(all_cores.contains("| lubm-1 | reasoning | 4.0 ms |"), "{all_cores}");
    }

    #[test]
    fn incremental_scenarios_rank_the_change_time() {
        let results = vec![
            row(
                "incremental-add",
                "turbowl",
                "ok",
                json!([{ "load_ms": 1.0, "reason_ms": 2.0, "change_ms": [1.0, 3.0, 2.0], "peak_rss_bytes": 1048576.0 }]),
                json!({ "measure": "change_median_ms" }),
            ),
            row("incremental-add", "nemo", "unsupported", json!([]), Value::Null),
        ];
        let text = scoreboard(&[("a.json".into(), file("default", None, results, json!([])))]);
        assert!(
            text.contains("| incremental-add | per change | 2.0 ms | unsupported | not run | not run | turbowl |"),
            "{text}"
        );
        assert!(!text.contains("incremental-add | reasoning"), "{text}");
    }

    /// claros-l in the default configuration (Oxford's Claros L axiomatises
    /// equality), every condition of VLog 2016's estimate holding except the
    /// time: VLog here gives a total only, the paper reasoning only, so it
    /// is refused; with a reasoning time here it is made.
    #[test]
    fn the_estimate_section() {
        let closure = 95_500_000u64;
        let rows = |vlog_reason: Value| {
            vec![
                row(
                    "claros-l",
                    "turbowl",
                    "ok",
                    json!([{ "load_ms": 15000.0, "reason_ms": 15000.0, "peak_rss_bytes": 1.0 }]),
                    json!({ "measure": "reason_ms", "reason_median_ms": 15000.0, "total_median_ms": 30000.0, "closure": closure }),
                ),
                row(
                    "claros-l",
                    "vlog",
                    "ok",
                    json!([{ "load_ms": null, "reason_ms": vlog_reason, "total_ms": 400000.0, "peak_rss_bytes": 1.0 }]),
                    json!({ "measure": "reason_ms", "reason_median_ms": vlog_reason, "total_median_ms": 400000.0, "closure": closure }),
                ),
            ]
        };
        let counts = json!([{ "scenario": "claros-l", "before": 18_793_277u64, "after": 95_480_000u64, "closure": closure, "ontology_only": 20_000 }]);
        let text = scoreboard(&[(
            "a.json".into(),
            file("default", None, rows(Value::Null), counts.clone()),
        )]);
        assert!(
            text.contains("No estimate: no published result meets every condition."),
            "{text}"
        );
        assert!(
            text.contains("| claros-l | default | vlog2016 (Table 1 (data and rule counts); Table 2 (time and peak memory, p. 6); MacBook Pro, 2.2 GHz Core i7, 16 GB, OS X 10.10.5), L | RDFox (2014 sequential build, recompiled) on 1 thread | VLog | no time measures the same work"),
            "{text}"
        );
        // Other rows on claros-l are refused for their own reasons.
        // CIKM 2019's Claros L leaves owl:sameAs out.
        assert!(text.contains("cikm2019 (Table 3 (facts before and after, rules) and Table 4 (load and materialisation, p. 8, appendix B); Dell PowerEdge R720, 2x Xeon E5-2670 2.6 GHz, 256 GB, Fedora 27), L | RDFox on threads not stated | VLog (CSV) | threads: RDFox ran with threads not stated"), "{text}");
        assert!(text.contains("threads: RDFox ran with 8 threads"), "{text}");

        let text = scoreboard(&[(
            "a.json".into(),
            file("default", None, rows(json!(300000.0)), counts.clone()),
        )]);
        // 300 s x 2418 / 644 = 1126.4 s.
        assert!(
            text.contains("| reasoning | 2418 s / 644 s | 300.0 s | **1126.4 s** | 15.0 s (0.01x the estimate) |"),
            "{text}"
        );
        // With equality off, Oxford's Claros L is not the same job.
        let off = Some(vec!["property-chains", "checks", "identity", "schema"]);
        let text = scoreboard(&[(
            "a.json".into(),
            file("default, equality off", off, rows(json!(300000.0)), counts),
        )]);
        assert!(
            text.contains("## default, equality off configuration, one thread"),
            "{text}"
        );
        assert!(
            text.contains(
                "the paper's L program axiomatises owl:sameAs: it needs equality on, this configuration has it off"
            ),
            "{text}"
        );
        assert!(text.contains("No estimate"), "{text}");
    }
}
