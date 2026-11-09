use oxrdf::{NamedNode, Triple};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;
use turbowl::{Reasoner, Summary, Threads};

const RUNS: usize = 11;
const NO_CHANGE_CALLS: usize = 1000;

fn loaded(root: &Path, fact: Option<&Triple>) -> Reasoner {
    let mut r = Reasoner::new();
    r.set_threads(Threads::ONE);
    for file in [
        "data/reasonable/example_models/ontologies/Brick.n3",
        "data/reasonable/example_models/small1.n3",
    ] {
        r.load_file(&root.join(file)).expect("load benchmark data");
    }
    if let Some(fact) = fact {
        r.insert([fact.as_ref()]);
    }
    r
}

fn closure(r: &Reasoner) -> HashSet<Triple> {
    r.triples().collect()
}

fn measure(root: &Path, fact: &Triple, case: &str, expected: &HashSet<Triple>) -> Value {
    let with_fact = matches!(case, "unchanged" | "delete_one");
    let mut r = loaded(root, with_fact.then_some(fact));
    let mut summary = if case == "full_run" {
        Summary::default()
    } else {
        r.reason()
    };
    let before = summary.base;
    let calls = if case == "unchanged" { NO_CHANGE_CALLS } else { 1 };
    let start = Instant::now();
    match case {
        "insert_one" => r.insert([fact.as_ref()]),
        "delete_one" => r.remove([fact.as_ref()]),
        "unchanged" | "full_run" => (),
        _ => unreachable!(),
    }
    for _ in 0..calls {
        summary = black_box(black_box(&mut r).reason());
    }
    let ns = start.elapsed().as_secs_f64() * 1e9 / calls as f64;
    assert_eq!(closure(&r), *expected, "{case}: complete closure");
    assert_eq!(summary.base + summary.inferred, expected.len());
    assert!(r.diagnostics().is_empty(), "{case}: unexpected diagnostics");
    match case {
        "insert_one" => assert_eq!(summary.base, before + 1),
        "delete_one" => assert_eq!(summary.base, before - 1),
        "unchanged" => assert_eq!(summary.rounds, 0),
        "full_run" => assert!(summary.rounds > 0),
        _ => unreachable!(),
    }
    let profile = r.profile();
    let families: Vec<_> = profile
        .families
        .iter()
        .enumerate()
        .map(|(i, name)| {
            json!({
                "name": name,
                "emitted": profile.rounds.iter().map(|r| r.emitted[i]).sum::<usize>(),
                "ns": profile.rounds.iter().map(|r| r.family_ns[i]).sum::<u64>(),
            })
        })
        .collect();
    json!({
        "case": case, "ns_per_call": ns, "calls": calls,
        "base": summary.base, "closure": expected.len(), "rounds": summary.rounds,
        "families": families,
        "deltas": profile.rounds.iter().map(|r| r.delta).collect::<Vec<_>>(),
        "round_ns": profile.rounds.iter().map(|r| r.ns).collect::<Vec<_>>(),
        "deletion": { "removed": profile.deletion.removed, "overdeleted": profile.deletion.overdeleted,
            "restored": profile.deletion.restored, "fallback": profile.deletion.fallback },
        "step_ns": profile.steps.iter().map(|s| s.ns).sum::<u64>(),
        "closure_checked": true, "diagnostics": summary.diagnostics,
    })
}

fn main() {
    if cfg!(debug_assertions) {
        eprintln!("build in release mode");
        std::process::exit(2);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fact = Triple::new(
        NamedNode::new("http://example.org/deletion-bench/sensor").unwrap(),
        NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
        NamedNode::new("https://brickschema.org/schema/1.1/Brick#Air_Temperature_Sensor").unwrap(),
    );
    let mut base = loaded(&root, None);
    base.reason();
    let without = closure(&base);
    base.insert([fact.as_ref()]);
    base.reason();
    let with = closure(&base);
    assert!(with.len() > without.len() + 1, "probe must cause inferences");
    let cases = ["unchanged", "insert_one", "delete_one", "full_run"];
    let mut samples = Vec::new();
    let mut warmups = Vec::new();
    for run in 0..=RUNS {
        for offset in 0..cases.len() {
            let case = cases[(run + offset) % cases.len()];
            let expected = if matches!(case, "unchanged" | "insert_one") {
                &with
            } else {
                &without
            };
            let mut sample = measure(&root, &fact, case, expected);
            if run > 0 {
                sample["run"] = json!(run);
                samples.push(sample);
            } else {
                sample["run"] = json!(0);
                warmups.push(sample);
            }
        }
    }
    println!(
        "{}",
        json!({
            "scenario": "Brick + small1", "threads": 1, "configuration": "default",
            "runs_per_case": RUNS, "warmups_per_case": 1,
            "timing": "mutation + reason; loading, initial reasoning and checks excluded",
            "probe_fact": fact.to_string(),
            "closure_without_probe": without.len(), "closure_with_probe": with.len(),
            "samples": samples,
            "warmups": warmups,
        })
    );
}
