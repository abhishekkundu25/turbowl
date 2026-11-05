//! The "Scaling" table: when a run includes two or more sizes of one
//! scenario family (`lubm-1`, `lubm-10`, `lubm-50`), each engine's fitted
//! growth exponent of total time (load plus reasoning) and of peak memory
//! against the input triple count. 1 is linear; the spec's budget for this
//! reasoner is a time exponent of at most 1.2 over LUBM 1, 10 and 50.

use crate::engine::EngineId;
use crate::scenario::Scenario;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// One scenario and engine, as the scaling fit sees it.
#[derive(Debug)]
pub(crate) struct Measured<'a> {
    pub(crate) scenario: &'static Scenario,
    pub(crate) engine: EngineId,
    pub(crate) status: &'a str,
    /// The row's summary (`null` fields when no run finished).
    pub(crate) summary: Option<Value>,
}

/// The families that have two or more scenarios among `scenarios`.
pub(crate) fn families<'a>(scenarios: impl IntoIterator<Item = &'a Scenario>) -> Vec<&'static str> {
    let mut seen: Vec<(&'static str, usize)> = Vec::new();
    for s in scenarios {
        if let Some(f) = s.family() {
            match seen.iter_mut().find(|(g, _)| *g == f) {
                Some((_, n)) => *n += 1,
                None => seen.push((f, 1)),
            }
        }
    }
    seen.into_iter().filter(|&(_, n)| n >= 2).map(|(f, _)| f).collect()
}

/// The triples in the input files, by each file's own syntax (outside every timing).
pub(crate) fn count_triples(files: &[PathBuf]) -> Result<u64, String> {
    files.iter().map(|f| count_file(f)).sum()
}

fn count_file(path: &Path) -> Result<u64, String> {
    let err = |e: &dyn std::fmt::Display| format!("{}: {e}", path.display());
    let file = std::fs::File::open(path).map_err(|e| err(&e))?;
    let reader = std::io::BufReader::with_capacity(1 << 20, file);
    let mut count = 0;
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "nt" => {
            for t in oxttl::NTriplesParser::new().for_reader(reader) {
                t.map_err(|e| err(&e))?;
                count += 1;
            }
        }
        "n3" => {
            for q in oxttl::N3Parser::new().for_reader(reader) {
                q.map_err(|e| err(&e))?;
                count += 1;
            }
        }
        _ => {
            for t in oxttl::TurtleParser::new().for_reader(reader) {
                t.map_err(|e| err(&e))?;
                count += 1;
            }
        }
    }
    Ok(count)
}

/// The least-squares slope of ln(y) against ln(x); `None` with fewer than
/// two points, or when a value is not positive.
pub(crate) fn exponent(points: &[(f64, f64)]) -> Option<f64> {
    if points.len() < 2 || points.iter().any(|&(x, y)| !(x > 0.0 && y > 0.0)) {
        return None;
    }
    let n = points.len() as f64;
    let logs: Vec<(f64, f64)> = points.iter().map(|&(x, y)| (x.ln(), y.ln())).collect();
    let mx = logs.iter().map(|p| p.0).sum::<f64>() / n;
    let my = logs.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx: f64 = logs.iter().map(|&(x, _)| (x - mx) * (x - mx)).sum();
    let sxy: f64 = logs.iter().map(|&(x, y)| (x - mx) * (y - my)).sum();
    (sxx > 0.0).then(|| (sxy / sxx * 1000.0).round() / 1000.0)
}

/// One entry per family and engine (in the order they ran): the points
/// with a finished run, the scenarios without one, and the two exponents.
pub(crate) fn entries(
    families: &[&str],
    measured: &[Measured<'_>],
    triples: &dyn Fn(&str) -> Option<u64>,
) -> Vec<Value> {
    let mut out = Vec::new();
    for &family in families {
        let mut engines: Vec<EngineId> = Vec::new();
        for m in measured.iter().filter(|m| m.scenario.family() == Some(family)) {
            if !engines.contains(&m.engine) {
                engines.push(m.engine);
            }
        }
        for engine in engines {
            let mut points = Vec::new();
            let mut missing = Vec::new();
            for m in measured
                .iter()
                .filter(|m| m.scenario.family() == Some(family) && m.engine == engine)
            {
                let total = m.summary.as_ref().and_then(|s| s["total_median_ms"].as_f64());
                let peak = m.summary.as_ref().and_then(|s| s["peak_rss_bytes"].as_f64());
                match (triples(m.scenario.name), total, peak) {
                    (Some(n), Some(total), Some(peak)) => points.push(json!({
                        "scenario": m.scenario.name,
                        "input_triples": n,
                        "total_median_ms": total,
                        "peak_rss_bytes": peak as u64,
                    })),
                    _ => missing.push(json!({ "scenario": m.scenario.name, "status": m.status })),
                }
            }
            points.sort_by_key(|p| p["input_triples"].as_u64());
            let fit = |key: &str| {
                let xy: Vec<(f64, f64)> = points
                    .iter()
                    .filter_map(|p| Some((p["input_triples"].as_f64()?, p[key].as_f64()?)))
                    .collect();
                exponent(&xy)
            };
            out.push(json!({
                "family": family,
                "engine": engine.name(),
                "time_exponent": fit("total_median_ms"),
                "memory_exponent": fit("peak_rss_bytes"),
                "points": points,
                "missing": missing,
            }));
        }
    }
    out
}

/// Prints the "Scaling" table.
pub(crate) fn print(entries: &[Value]) {
    if entries.is_empty() {
        return;
    }
    println!("Scaling: the fitted exponent of total time (load + reasoning, median) and of peak memory");
    println!("against the input triples, least squares on a log-log scale. 1 is linear.");
    println!();
    println!("| family | engine | input triples | total ms | peak MB | time exponent | memory exponent | not fitted |");
    println!("|---|---|---|---|---|---:|---:|---|");
    let empty = Vec::new();
    for e in entries {
        let points = e["points"].as_array().unwrap_or(&empty);
        let list = |f: &dyn Fn(&Value) -> String| points.iter().map(f).collect::<Vec<_>>().join(", ");
        let exp = |key: &str| e[key].as_f64().map_or("-".into(), |x| format!("{x:.2}"));
        let missing = e["missing"]
            .as_array()
            .unwrap_or(&empty)
            .iter()
            .map(|m| {
                format!(
                    "{} ({})",
                    m["scenario"].as_str().unwrap_or("?"),
                    m["status"].as_str().unwrap_or("?")
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            e["family"].as_str().unwrap_or("?"),
            e["engine"].as_str().unwrap_or("?"),
            list(&|p| format!(
                "{} {}",
                p["scenario"].as_str().unwrap_or("?"),
                p["input_triples"].as_u64().unwrap_or(0)
            )),
            list(&|p| format!("{:.0}", p["total_median_ms"].as_f64().unwrap_or(f64::NAN))),
            list(&|p| crate::mb(p["peak_rss_bytes"].as_f64().unwrap_or(f64::NAN))),
            exp("time_exponent"),
            exp("memory_exponent"),
            missing,
        );
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario;

    #[test]
    fn exponent_of_powers() {
        let linear = [(1e5, 2.0), (1e6, 20.0), (5e6, 100.0)];
        assert_eq!(exponent(&linear), Some(1.0));
        let square = [(10.0, 100.0), (100.0, 10_000.0)];
        assert_eq!(exponent(&square), Some(2.0));
        assert_eq!(exponent(&[(10.0, 1.0)]), None, "one point fits nothing");
        assert_eq!(exponent(&[(10.0, 1.0), (20.0, 0.0)]), None);
    }

    #[test]
    fn families_need_two_sizes() {
        let find = |n| scenario::find(n).unwrap();
        assert_eq!(families([find("lubm-1"), find("brick-small1")]), Vec::<&str>::new());
        assert_eq!(
            families([find("lubm-1"), find("brick-small1"), find("lubm-10")]),
            ["lubm"]
        );
    }

    #[test]
    fn entries_fit_finished_runs_only() {
        let find = |n| scenario::find(n).unwrap();
        let summary = |total: f64, peak: f64| Some(json!({ "total_median_ms": total, "peak_rss_bytes": peak }));
        let measured = [
            Measured {
                scenario: find("lubm-1"),
                engine: EngineId::Turbowl,
                status: "ok",
                summary: summary(10.0, 1e8),
            },
            Measured {
                scenario: find("lubm-1"),
                engine: EngineId::Nemo,
                status: "timeout",
                summary: None,
            },
            Measured {
                scenario: find("lubm-10"),
                engine: EngineId::Turbowl,
                status: "ok",
                summary: summary(100.0, 4e8),
            },
            Measured {
                scenario: find("lubm-10"),
                engine: EngineId::Nemo,
                status: "ok",
                summary: summary(50.0, 1e8),
            },
        ];
        let triples = |name: &str| match name {
            "lubm-1" => Some(1_000),
            "lubm-10" => Some(10_000),
            _ => None,
        };
        let e = entries(&["lubm"], &measured, &triples);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0]["engine"], "turbowl");
        assert_eq!(e[0]["time_exponent"], 1.0);
        assert_eq!(e[0]["memory_exponent"], 0.602);
        assert_eq!(e[1]["engine"], "nemo");
        assert_eq!(e[1]["time_exponent"], Value::Null);
        assert_eq!(e[1]["missing"][0]["status"], "timeout");
    }
}
