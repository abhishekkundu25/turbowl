//! This reasoner's counters in the results file and the text report: what a
//! run did, round by round and step by step, and the bytes it held after
//! (`turbowl::profile`, experiment P0 of `docs/specs/experiments.md`).
//!
//! Each run of `turbowl` records its profile (`run.profile`): the counters
//! of its full run (the first `reason()`), and the memory held at its end,
//! read just after the peak. A row's summary adds the medians of the step
//! times and the bytes per closure triple by part (`summary.profile`), and
//! [`print`] prints them, one line per scenario.

use serde_json::{Value, json};
use turbowl::profile::{Memory, Parts, Profile};

/// Nanoseconds as the results file's milliseconds (to the microsecond).
fn ms(ns: u64) -> f64 {
    (ns as f64 / 1e3).round() / 1e3
}

fn parts(p: &Parts) -> Value {
    json!({
        "tuples": p.tuples,
        "stable": p.stable,
        "recent": p.recent,
        "object_index": p.object_index,
    })
}

fn memory(m: &Memory) -> Value {
    json!({
        "dictionary_text": m.dictionary_text,
        "dictionary_entries": m.dictionary_entries,
        "dictionary_slots": m.dictionary_slots,
        "base": m.base,
        "store_type": parts(&m.store_type),
        "store_other": parts(&m.store_other),
        "store_rest": m.store_rest,
        "largest": m.largest.iter().map(|(p, t)| {
            let mut v = parts(t);
            v["predicate"] = json!(p);
            v
        }).collect::<Vec<_>>(),
        "canon": m.canon,
        "findings": m.findings,
        "diagnostics": m.diagnostics,
        "other": m.other,
        "total": m.total(),
    })
}

/// A run's profile, for `run.profile`: the counters of `counters` (the full
/// run's) and the memory of `held` (the state at the end of the run).
/// Columns, one entry per round or step, keep it short.
pub(crate) fn to_json(counters: &Profile, held: &Memory) -> Value {
    let rounds = &counters.rounds;
    let steps = &counters.steps;
    let sum = |f: &dyn Fn(&turbowl::profile::Step) -> u64| ms(steps.iter().map(f).sum());
    json!({
        "families": counters.families,
        "rounds": {
            "delta": rounds.iter().map(|r| r.delta).collect::<Vec<_>>(),
            "emitted": rounds.iter().map(|r| &r.emitted).collect::<Vec<_>>(),
            "family_ms": rounds.iter().map(|r| r.family_ns.iter().map(|&ns| ms(ns)).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "ms": rounds.iter().map(|r| ms(r.ns)).collect::<Vec<_>>(),
        },
        "steps": {
            "ms": steps.iter().map(|s| ms(s.ns)).collect::<Vec<_>>(),
            "sort_ms": steps.iter().map(|s| ms(s.sort_ns)).collect::<Vec<_>>(),
            "changed_ms": steps.iter().map(|s| ms(s.changed_ns)).collect::<Vec<_>>(),
            "pairs": steps.iter().map(|s| s.pairs).collect::<Vec<_>>(),
            "new": steps.iter().map(|s| s.new).collect::<Vec<_>>(),
        },
        "round_count": rounds.len(),
        "rules_ms": ms(rounds.iter().map(|r| r.ns).sum()),
        "step_ms": sum(&|s| s.ns),
        "sort_ms": sum(&|s| s.sort_ns),
        "changed_ms": sum(&|s| s.changed_ns),
        "memory": memory(held),
    })
}

/// The summary of a row's profiles (`summary.profile`): the median over the
/// runs of each time, the last run's rounds and memory, and that memory
/// per closure triple by part beside the largest peak's. `None` when no run
/// has a profile (a competitor, or a file before schema 8).
pub(crate) fn summary(runs: &[Value], peak: f64, closure: Option<u64>) -> Option<Value> {
    let profiles: Vec<&Value> = runs.iter().map(|r| &r["profile"]).filter(|p| p.is_object()).collect();
    let last = profiles.last()?;
    let median = |key: &str| {
        let xs: Vec<f64> = profiles.iter().filter_map(|p| p[key].as_f64()).collect();
        crate::spread(&xs).map(|s| s.0)
    };
    let m = &last["memory"];
    let n = |v: &Value| v.as_f64().unwrap_or(0.0);
    let table = |t: &Value| n(&t["stable"]) + n(&t["recent"]) + n(&t["object_index"]);
    let dictionary = n(&m["dictionary_text"]) + n(&m["dictionary_entries"]) + n(&m["dictionary_slots"]);
    let base = n(&m["base"]);
    let store = table(&m["store_type"]) + table(&m["store_other"]) + n(&m["store_rest"]);
    let total = n(&m["total"]);
    let rest = total - dictionary - base - store;
    let per = |bytes: f64| closure.filter(|&c| c > 0).map(|c| round1(bytes / c as f64));
    Some(json!({
        "rounds": last["round_count"],
        "rules_median_ms": median("rules_ms"),
        "step_median_ms": median("step_ms"),
        "sort_median_ms": median("sort_ms"),
        "changed_median_ms": median("changed_ms"),
        "memory": {
            "dictionary": dictionary as u64,
            "base": base as u64,
            "store": store as u64,
            "rest": rest as u64,
            "total": total as u64,
        },
        "bytes_per_closure_triple": {
            "peak": per(peak),
            "dictionary": per(dictionary),
            "base": per(base),
            "store": per(store),
            "rest": per(rest),
            "not_counted": per(peak - total),
        },
    }))
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

/// One line per scenario with a profile: bytes per closure triple at the
/// peak and held after the run by part, and the time in the steps. `rows`
/// are each scenario's name and `summary.profile`.
pub(crate) fn print(rows: &[(&str, Value)]) {
    if rows.is_empty() {
        return;
    }
    println!("Breakdown (turbowl): bytes per closure triple, the peak and what the reasoner holds after the run");
    println!("(rest: canon, findings, diagnostics; not counted: peak - held). Times: medians of the full run's.");
    println!();
    println!(
        "| scenario | B/triple peak | dictionary | base | store | rest | not counted | rounds | rules ms | step ms | sort ms | changed ms |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    for (scenario, p) in rows {
        let b = &p["bytes_per_closure_triple"];
        let per = |k: &str| b[k].as_f64().map_or(String::new(), |v| format!("{v:.1}"));
        let t = |k: &str| p[k].as_f64().map_or(String::new(), |v| format!("{v:.1}"));
        println!(
            "| {scenario} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            per("peak"),
            per("dictionary"),
            per("base"),
            per("store"),
            per("rest"),
            per("not_counted"),
            p["rounds"],
            t("rules_median_ms"),
            t("step_median_ms"),
            t("sort_median_ms"),
            t("changed_median_ms"),
        );
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use turbowl::{Format, Reasoner};

    /// A run's profile goes into the results file and its summary: the
    /// rounds and steps line up, and the parts per triple add up to what
    /// the reasoner held.
    #[test]
    fn a_profile_is_recorded_and_summarised() {
        let mut r = Reasoner::new();
        r.load_str(
            "@prefix ex: <http://example.org/> .
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
             ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C . ex:x a ex:A .",
            Format::Turtle,
            None,
        )
        .unwrap();
        let s = r.reason();
        let p = r.profile();
        let run = json!({ "profile": to_json(&p, &p.memory) });
        let v = &run["profile"];
        assert_eq!(v["round_count"], s.rounds);
        assert_eq!(v["rounds"]["delta"].as_array().unwrap().len(), s.rounds);
        assert_eq!(v["steps"]["pairs"].as_array().unwrap().len(), s.rounds + 1);
        assert_eq!(v["rounds"]["emitted"][0].as_array().unwrap().len(), p.families.len());
        assert_eq!(v["memory"]["total"], p.memory.total());

        let closure = (s.base + s.inferred) as u64;
        let peak = 2.0 * p.memory.total() as f64;
        let sum = summary(&[run.clone(), json!({})], peak, Some(closure)).unwrap();
        let m = &sum["memory"];
        let parts = ["dictionary", "base", "store", "rest"];
        let total: u64 = parts.iter().map(|k| m[k].as_u64().unwrap()).sum();
        assert_eq!(total, m["total"].as_u64().unwrap());
        assert_eq!(m["total"], p.memory.total());
        let b = &sum["bytes_per_closure_triple"];
        let held: f64 = parts.iter().map(|k| b[k].as_f64().unwrap()).sum();
        let whole = b["peak"].as_f64().unwrap();
        assert!((held + b["not_counted"].as_f64().unwrap() - whole).abs() < 0.5, "{b}");
        assert_eq!(sum["rounds"], s.rounds);

        // No profile (a competitor, an older file): no summary.
        assert!(summary(&[json!({ "reason_ms": 1.0 })], peak, Some(closure)).is_none());
    }
}
