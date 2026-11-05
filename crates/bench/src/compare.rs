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
    for field in ["cpu", "os", "build", "changes"] {
        if om[field] != nm[field] {
            println!(
                "warning: {field} differs ({} vs {}): the numbers are not comparable",
                om[field], nm[field]
            );
        }
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
    use super::verdict;

    #[test]
    fn flags_only_ranges_apart() {
        assert_eq!(verdict((10.0, 12.0), (11.0, 13.0), "up", "down"), "-");
        assert_eq!(verdict((10.0, 12.0), (12.5, 13.0), "up", "down"), "up");
        assert_eq!(verdict((10.0, 12.0), (8.0, 9.9), "up", "down"), "down");
    }
}
