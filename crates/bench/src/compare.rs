//! `--compare OLD NEW`: per-scenario deltas between two results files.
//!
//! A change is flagged only when the two runs' ranges (min to max) do not
//! overlap: a median that moved inside the spread is noise until more runs
//! say otherwise. It reports and exits 0; problems reading the files exit 1.

use serde_json::Value;
use std::path::Path;
use std::process::ExitCode;

fn key(r: &Value) -> (String, String) {
    (
        r["scenario"].as_str().unwrap_or("").to_string(),
        r["engine"].as_str().unwrap_or("").to_string(),
    )
}

/// `slower`, `faster` or `-` (within the spread), from two (min, max) ranges.
fn verdict(old: (f64, f64), new: (f64, f64), up: &str, down: &str) -> String {
    if new.0 > old.1 {
        up.into()
    } else if new.1 < old.0 {
        down.into()
    } else {
        "-".into()
    }
}

/// The 1-minute load average of a `meta.load_*` field.
fn load(v: &Value) -> String {
    v[0].as_f64().map_or("?".into(), |l| format!("{l:.2}"))
}

fn pct(old: f64, new: f64) -> String {
    if old > 0.0 {
        format!("{:+.1}%", (new / old - 1.0) * 100.0)
    } else {
        "-".into()
    }
}

/// The fields of two files' `meta` that differ where they must not, each as
/// `NAME differs (OLD vs NEW)`.
fn differences(om: &Value, nm: &Value) -> Vec<String> {
    // A file from before schema 6 ran the default rules and read the peak
    // resident size; one from before schema 7 ran on one thread.
    let or = |v: &Value, old: Value| if v.is_null() { old } else { v.clone() };
    let mut out = Vec::new();
    for (field, before) in [
        ("cpu", None),
        ("os", None),
        ("build", None),
        ("changes", None),
        ("configuration", Some(Value::from("default"))),
        ("peak_memory", Some(Value::from("peak resident size"))),
        ("threads", Some(Value::from(1))),
    ] {
        let (o, n) = match before {
            Some(b) => (or(&om[field], b.clone()), or(&nm[field], b)),
            None => (om[field].clone(), nm[field].clone()),
        };
        if o != n {
            out.push(format!("{field} differs ({o} vs {n})"));
        }
    }
    out
}

pub(crate) fn run(old_path: &Path, new_path: &Path) -> ExitCode {
    let ((old, old_skipped), (new, new_skipped)) = match (crate::read_results(old_path), crate::read_results(new_path))
    {
        (Ok(o), Ok(n)) => (o, n),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let (om, nm) = (&old["meta"], &new["meta"]);
    println!(
        "Old: {} at {} ({}), load {} before, {} after",
        om["date"].as_str().unwrap_or("?"),
        om["commit"].as_str().map_or("?", |c| &c[..c.len().min(10)]),
        if om["dirty"] == true { "dirty" } else { "clean" },
        load(&om["load_before"]),
        load(&om["load_after"])
    );
    println!(
        "New: {} at {} ({}), load {} before, {} after",
        nm["date"].as_str().unwrap_or("?"),
        nm["commit"].as_str().map_or("?", |c| &c[..c.len().min(10)]),
        if nm["dirty"] == true { "dirty" } else { "clean" },
        load(&nm["load_before"]),
        load(&nm["load_after"])
    );
    for field in differences(om, nm) {
        println!("warning: {field}: the numbers are not comparable");
    }
    for (name, meta) in [("old", om), ("new", nm)] {
        if meta["load_before"][0].as_f64().is_some_and(|l| l > 1.0) {
            println!("warning: the {name} run started with a load average over 1");
        }
    }
    for (path, skipped) in [(old_path, &old_skipped), (new_path, &new_skipped)] {
        if !skipped.is_empty() {
            let mut engines = skipped.clone();
            engines.sort();
            engines.dedup();
            println!(
                "note: skipped {} rows of {}: engines this bench no longer runs ({})",
                skipped.len(),
                path.display(),
                engines.join(", ")
            );
        }
    }
    println!();
    println!("Flagged only where the min..max ranges do not overlap. Time is the median of the scenario's measure.");
    println!();
    println!(
        "| scenario | engine | old ms | new ms | delta | old range | new range | time | old MB | new MB | memory |"
    );
    println!("|---|---|---:|---:|---:|---|---|---|---:|---:|---|");

    let empty = Vec::new();
    let old_rows = old["results"].as_array().unwrap_or(&empty);
    for n in new["results"].as_array().unwrap_or(&empty) {
        let k = key(n);
        let Some(o) = old_rows.iter().find(|o| key(o) == k) else {
            println!("| {} | {} | (absent) | | | | | | | | |", k.0, k.1);
            continue;
        };
        let (os, ns) = (&o["summary"], &n["summary"]);
        if os.is_null() || ns.is_null() {
            println!(
                "| {} | {} | {} | {} | | | | | | | |",
                k.0,
                k.1,
                o["status"].as_str().unwrap_or("?"),
                n["status"].as_str().unwrap_or("?")
            );
            continue;
        }
        let f = |v: &Value, key: &str| v[key].as_f64().unwrap_or(f64::NAN);
        let (o_ms, n_ms) = (f(os, "median_ms"), f(ns, "median_ms"));
        let (o_range, n_range) = ((f(os, "min_ms"), f(os, "max_ms")), (f(ns, "min_ms"), f(ns, "max_ms")));
        let (o_mem, n_mem) = (
            (f(os, "peak_rss_min_bytes"), f(os, "peak_rss_bytes")),
            (f(ns, "peak_rss_min_bytes"), f(ns, "peak_rss_bytes")),
        );
        println!(
            "| {} | {} | {:.1} | {:.1} | {} | {:.1}..{:.1} | {:.1}..{:.1} | {} | {} | {} | {} |",
            k.0,
            k.1,
            o_ms,
            n_ms,
            pct(o_ms, n_ms),
            o_range.0,
            o_range.1,
            n_range.0,
            n_range.1,
            verdict(o_range, n_range, "SLOWER", "faster"),
            crate::mb(o_mem.1),
            crate::mb(n_mem.1),
            verdict(o_mem, n_mem, "MORE", "less"),
        );
        if os["closure"] != ns["closure"] {
            println!(
                "|   | closure {} -> {} | | | | | | | | | |",
                os["closure"], ns["closure"]
            );
        }
    }
    for o in old_rows {
        let k = key(o);
        if !new["results"].as_array().unwrap_or(&empty).iter().any(|n| key(n) == k) {
            println!("| {} | {} | | (absent) | | | | | | | |", k.0, k.1);
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{differences, verdict};
    use serde_json::json;

    /// Every schema from 1 to 8 reads, and compares with any other: what an
    /// older file leaves out reads as what it ran (the default
    /// configuration, the peak resident size, one thread).
    #[test]
    fn compares_across_schemas() {
        let dir = std::env::temp_dir().join(format!("bench-compare-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let meta = |schema: u64| match schema {
            1..=5 => json!({ "cpu": "M", "date": "d" }),
            6 => json!({ "cpu": "M", "configuration": "default", "peak_memory": "lifetime peak physical footprint" }),
            _ => json!({ "cpu": "M", "configuration": "default, equality off", "switches": ["checks"],
                         "threads": 1, "peak_memory": "lifetime peak physical footprint" }),
        };
        let engine = |schema: u64| if schema < 4 { "turtle-reasoner" } else { "turbowl" };
        let mut read = Vec::new();
        for schema in 1..=9 {
            let path = dir.join(format!("v{schema}.json"));
            let row = json!({ "scenario": "lubm-1", "engine": engine(schema), "status": "ok",
                              "summary": { "median_ms": 1.0, "min_ms": 1.0, "max_ms": 1.0 } });
            std::fs::write(
                &path,
                json!({ "schema": schema, "meta": meta(schema), "results": [row] }).to_string(),
            )
            .unwrap();
            match crate::read_results(&path) {
                Ok((v, skipped)) => {
                    assert!(schema <= 8, "schema {schema} read");
                    assert!(skipped.is_empty());
                    assert_eq!(v["results"][0]["engine"], "turbowl", "schema {schema}");
                    read.push(v);
                }
                Err(e) => assert_eq!(schema, 9, "{e}"),
            }
        }
        assert_eq!(read.len(), 8);
        for old in 1..=8 {
            for new in 1..=8 {
                assert_eq!(
                    super::run(&dir.join(format!("v{old}.json")), &dir.join(format!("v{new}.json"))),
                    std::process::ExitCode::SUCCESS
                );
            }
        }
        let fields = |o: usize, n: usize| differences(&read[o - 1]["meta"], &read[n - 1]["meta"]);
        assert!(fields(1, 5).is_empty());
        assert_eq!(
            fields(5, 6),
            ["peak_memory differs (\"peak resident size\" vs \"lifetime peak physical footprint\")"]
        );
        assert_eq!(
            fields(6, 7),
            ["configuration differs (\"default\" vs \"default, equality off\")"]
        );
        assert!(fields(7, 8).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn flags_only_ranges_apart() {
        assert_eq!(verdict((10.0, 12.0), (11.0, 13.0), "up", "down"), "-");
        assert_eq!(verdict((10.0, 12.0), (12.5, 13.0), "up", "down"), "up");
        assert_eq!(verdict((10.0, 12.0), (8.0, 9.9), "up", "down"), "down");
    }
}
