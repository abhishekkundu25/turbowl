//! `turbowl-bench --explain`: explanation latency (`docs/specs/explanations.md`,
//! the budgets).
//!
//! Per scenario, in this process (reasoning is not measured here): load and
//! reason once, in the configuration and thread mode the arguments give,
//! then draw a seeded sample of 1,000 inferred triples and up to 1,000
//! diagnostics (reservoir sampling over the reads, so a closure of millions
//! is never collected) and explain each with one explainer.
//!
//! - **Cold**: the first explanation, which builds the cache (the rule
//!   program with its constants as term ids, and the lazy indexes it
//!   needs), timed on its own, on the first triple of the sample. It is
//!   then explained again, warm, with the others.
//! - **Warm**: each triple and diagnostic of the sample once, in sample
//!   order, on the cache the earlier ones left: median, 99th percentile and
//!   maximum.
//! - **Cache**: the bytes the cache holds after the sample
//!   (`turbowl::profile::explain_cache_bytes`), which the process peak
//!   cannot separate from reasoning.
//!
//! A triple or diagnostic with no proof is counted as missing: a bug.

use crate::engine::ThreadMode;
use crate::scenario::Scenario;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use turbowl::{Diagnostic, Reasoner, Switches};

/// The triples, and the most diagnostics, a scenario explains.
pub(crate) const SAMPLE: usize = 1000;

/// The sample's seed.
const SEED: u64 = 0x0E8A_1A1E_5EED_0001;

/// SplitMix64, seeded: the sample is the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// Up to `n` items of `items`, each equally likely (Algorithm R), in the
/// order they were drawn into the reservoir.
fn sample<T>(items: impl Iterator<Item = T>, n: usize, rng: &mut Rng) -> Vec<T> {
    let mut kept: Vec<T> = Vec::with_capacity(n);
    for (i, item) in items.enumerate() {
        if kept.len() < n {
            kept.push(item);
        } else {
            let j = rng.below(i as u64 + 1) as usize;
            if j < n {
                kept[j] = item;
            }
        }
    }
    kept
}

/// Median, 99th percentile and maximum, in milliseconds (nearest rank).
fn spread(times: &[Duration]) -> Option<(f64, f64, f64)> {
    if times.is_empty() {
        return None;
    }
    let mut ms: Vec<f64> = times.iter().map(|t| t.as_secs_f64() * 1000.0).collect();
    ms.sort_by(f64::total_cmp);
    let at = |q: f64| ms[((q * ms.len() as f64).ceil() as usize).clamp(1, ms.len()) - 1];
    Some((at(0.5), at(0.99), ms[ms.len() - 1]))
}

/// One scenario's measures.
pub(crate) fn measure(
    scenario: &Scenario,
    files: &[PathBuf],
    switches: Switches,
    threads: ThreadMode,
) -> Result<Value, String> {
    let mut r = Reasoner::with_switches(switches);
    r.set_threads(threads.threads());
    for f in files {
        r.load_file(f).map_err(|e| e.to_string())?;
    }
    let start = Instant::now();
    let summary = r.reason();
    let reason_ms = start.elapsed().as_secs_f64() * 1000.0;

    let mut rng = Rng(SEED);
    let triples = sample(r.inferred(), SAMPLE, &mut rng);
    let diagnostics: Vec<Diagnostic> = sample(r.diagnostics().iter(), SAMPLE, &mut rng)
        .into_iter()
        .cloned()
        .collect();

    let explainer = r.explainer().map_err(|e| e.to_string())?;
    let (mut missing, mut steps) = (0usize, 0usize);
    let cold = triples.first().map(|t| {
        let start = Instant::now();
        let proof = explainer.explain(t.as_ref());
        let took = start.elapsed();
        missing += usize::from(proof.is_none());
        took
    });
    let mut warm = Vec::with_capacity(triples.len());
    for t in &triples {
        let start = Instant::now();
        let proof = explainer.explain(t.as_ref());
        warm.push(start.elapsed());
        match proof {
            Some(p) => steps += p.steps().len(),
            None => missing += 1,
        }
    }
    let mut warm_diagnostics = Vec::with_capacity(diagnostics.len());
    for d in &diagnostics {
        let start = Instant::now();
        let proof = explainer.explain_diagnostic(d);
        warm_diagnostics.push(start.elapsed());
        missing += usize::from(proof.is_none());
    }
    let cache = explainer.into_cache();
    let bytes = turbowl::profile::explain_cache_bytes(&cache);

    let ms = |t: Option<(f64, f64, f64)>| {
        t.map(|(median, p99, max)| json!({ "median_ms": median, "p99_ms": p99, "max_ms": max }))
    };
    Ok(json!({
        "scenario": scenario.name,
        "closure": summary.base + summary.inferred,
        "reason_ms": reason_ms,
        "triples": triples.len(),
        "diagnostics": diagnostics.len(),
        "cold_ms": cold.map(|t| t.as_secs_f64() * 1000.0),
        "warm": ms(spread(&warm)),
        "warm_diagnostics": ms(spread(&warm_diagnostics)),
        "mean_steps": if triples.is_empty() { 0.0 } else { steps as f64 / triples.len() as f64 },
        "cache_bytes": bytes,
        "missing": missing,
    }))
}

/// The table of [`measure`]'s results.
pub(crate) fn print(rows: &[Value]) {
    println!(
        "\n{:<18} {:>10} {:>7} {:>9} {:>9} {:>9} {:>9} {:>6} {:>9} {:>9} {:>9} {:>9} {:>8}",
        "scenario",
        "closure",
        "triples",
        "median ms",
        "p99 ms",
        "max ms",
        "cold ms",
        "diags",
        "d median",
        "d p99",
        "d max",
        "cache MB",
        "missing"
    );
    let num = |v: &Value| v.as_f64().map_or("-".to_string(), |x| format!("{x:.3}"));
    for row in rows {
        if let Some(e) = row["error"].as_str() {
            println!("{:<18} error: {e}", row["scenario"].as_str().unwrap_or("?"));
            continue;
        }
        println!(
            "{:<18} {:>10} {:>7} {:>9} {:>9} {:>9} {:>9} {:>6} {:>9} {:>9} {:>9} {:>9.1} {:>8}",
            row["scenario"].as_str().unwrap_or("?"),
            row["closure"],
            row["triples"],
            num(&row["warm"]["median_ms"]),
            num(&row["warm"]["p99_ms"]),
            num(&row["warm"]["max_ms"]),
            num(&row["cold_ms"]),
            row["diagnostics"],
            num(&row["warm_diagnostics"]["median_ms"]),
            num(&row["warm_diagnostics"]["p99_ms"]),
            num(&row["warm_diagnostics"]["max_ms"]),
            row["cache_bytes"].as_f64().unwrap_or(0.0) / 1_048_576.0,
            row["missing"],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sample_is_seeded_and_bounded() {
        let a = sample(0..10_000, 1000, &mut Rng(SEED));
        let b = sample(0..10_000, 1000, &mut Rng(SEED));
        assert_eq!(a, b);
        assert_eq!(a.len(), 1000);
        assert_eq!(sample(0..10, 1000, &mut Rng(SEED)), (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn the_spread_is_by_nearest_rank() {
        let times: Vec<Duration> = (1..=100).map(Duration::from_millis).collect();
        assert_eq!(spread(&times), Some((50.0, 99.0, 100.0)));
        assert_eq!(spread(&[]), None);
    }
}
