//! Snapshots on the oracles' inputs (ticket 19): a reasoner saved and
//! loaded back gives the same closure, diagnostics and switches on every
//! minimal input (`cases::minimal`) and on Brick with small1, in the default
//! and the complete configuration; and changes made after a load give what
//! the same changes give without the snapshot.

mod support;

use oxrdf::Triple;
use std::collections::{BTreeSet, HashSet};
use std::time::Instant;
use support::data::{BRICK, SMALL1};
use turbowl::{Diagnostic, Reasoner, Summary, Switches};
use turbowl_oracle::cases;

fn configurations() -> [(&'static str, Switches); 2] {
    [("default", Switches::default()), ("complete", Switches::complete())]
}

fn reasoned(triples: &[Triple], switches: Switches) -> Reasoner {
    let mut r = Reasoner::with_switches(switches);
    r.insert(triples.iter().map(Triple::as_ref));
    r.reason();
    r
}

fn snapshot(r: &Reasoner) -> Vec<u8> {
    let mut bytes = Vec::new();
    r.write_snapshot(&mut bytes).unwrap();
    bytes
}

/// The reasoner `r`'s snapshot holds. Saving it again gives the same bytes.
fn reloaded(r: &Reasoner, what: &str) -> Reasoner {
    let bytes = snapshot(r);
    let loaded = Reasoner::from_snapshot_bytes(&bytes).unwrap_or_else(|e| panic!("{what}: {e}"));
    assert!(snapshot(&loaded) == bytes, "{what}: saved again, the same bytes");
    loaded
}

/// The closure (in the order the reads give it), the inferred triples, the
/// diagnostics and the switches.
fn shown(r: &Reasoner) -> (Vec<Triple>, Vec<Triple>, Vec<Diagnostic>, Switches) {
    (
        r.triples().collect(),
        r.inferred().collect(),
        r.diagnostics().to_vec(),
        r.switches(),
    )
}

/// Asserts that `a` and `b` show the same, and that their next runs do the
/// same; returns what the run did.
fn same(a: &mut Reasoner, b: &mut Reasoner, what: &str) -> Summary {
    let (sa, sb) = (a.reason(), b.reason());
    assert_eq!(sa, sb, "{what}: the summaries");
    let (shown_a, shown_b) = (shown(a), shown(b));
    assert!(shown_a.0 == shown_b.0, "{what}: the closures");
    assert!(shown_a.1 == shown_b.1, "{what}: the inferred triples");
    assert_eq!(shown_a.2, shown_b.2, "{what}: the diagnostics");
    assert_eq!(shown_a.3, shown_b.3, "{what}: the switches");
    sa
}

/// Asserts that `a` and `b` hold the same closure and diagnostics, read as
/// sets: for two reasoners that reached them by different runs or interned
/// the terms in another order, whose reads may come in another order.
fn same_sets(a: &mut Reasoner, b: &mut Reasoner, what: &str) {
    let (sa, sb) = (a.reason(), b.reason());
    assert_eq!(
        (sa.base, sa.inferred, sa.diagnostics),
        (sb.base, sb.inferred, sb.diagnostics),
        "{what}: the summaries"
    );
    let set = |r: &Reasoner| r.triples().collect::<HashSet<Triple>>();
    assert!(set(a) == set(b), "{what}: the closures");
    // In the order of the terms' ids, which the order of interning sets.
    let diagnostics = |r: &Reasoner| r.diagnostics().iter().cloned().collect::<HashSet<Diagnostic>>();
    assert_eq!(diagnostics(a), diagnostics(b), "{what}: the diagnostics");
    assert_eq!(a.switches(), b.switches(), "{what}: the switches");
}

/// Every code a diagnostic can have (the table of `turbowl::Diagnostic`).
const CODES: [&str; 17] = [
    "cax-adc",
    "cax-dw",
    "cls-com",
    "cls-maxc1",
    "cls-maxqc1",
    "cls-maxqc2",
    "cls-nothing2",
    "dt-not-type",
    "eq-diff1",
    "eq-diff2",
    "eq-diff3",
    "prp-adp",
    "prp-asyp",
    "prp-irp",
    "prp-npa1",
    "prp-npa2",
    "prp-pdw",
];

#[test]
fn a_round_trip_keeps_every_minimal_input() {
    let mut codes = BTreeSet::new();
    let mut runs = 0;
    for case in cases::minimal() {
        let input = case.input();
        for (configuration, switches) in configurations() {
            let what = format!("{} ({configuration})", case.name);
            let mut r = reasoned(input.triples(), switches);
            let mut loaded = reloaded(&r, &what);
            assert_eq!(loaded.reason().rounds, 0, "{what}: nothing to run again");
            same(&mut r, &mut loaded, &what);
            codes.extend(r.diagnostics().iter().map(|d| d.code));
            runs += 1;
        }
    }
    println!("{runs} round trips; codes seen: {codes:?}");
    assert!(runs >= 200, "{runs} round trips");
    // Every code a finding can have was saved and loaded back.
    let missing: Vec<&str> = CODES.into_iter().filter(|c| !codes.contains(c)).collect();
    assert!(missing.is_empty(), "codes no minimal input reports: {missing:?}");
}

/// Half of each minimal input reasoned and saved; the other half, a
/// removal, and another save with additions queued, each on the loaded
/// reasoner and on the one that was saved.
#[test]
fn changes_after_a_load_equal_the_changes_without_it() {
    for case in cases::minimal() {
        let input = case.input();
        let triples = input.triples();
        let first: Vec<Triple> = triples.iter().step_by(2).cloned().collect();
        let second: Vec<Triple> = triples.iter().skip(1).step_by(2).cloned().collect();
        for (configuration, switches) in configurations() {
            let what = format!("{} ({configuration})", case.name);
            let mut kept = reasoned(&first, switches);
            let mut loaded = reloaded(&kept, &what);

            for r in [&mut kept, &mut loaded] {
                r.insert(second.iter().map(Triple::as_ref));
            }
            same(&mut kept, &mut loaded, &format!("{what}, the other half added"));

            let gone = &triples[..triples.len().min(1)];
            for r in [&mut kept, &mut loaded] {
                r.remove(gone.iter().map(Triple::as_ref));
            }
            same(&mut kept, &mut loaded, &format!("{what}, a triple removed"));

            // Put back, and saved with the incremental run due.
            for r in [&mut kept, &mut loaded] {
                r.insert(gone.iter().map(Triple::as_ref));
            }
            let mut again = reloaded(&loaded, &what);
            let s = again.reason();
            let put_back = same(&mut kept, &mut loaded, &format!("{what}, put back"));
            assert_eq!(s, put_back, "{what}, saved with additions queued: the summaries");
            assert!(shown(&again) == shown(&loaded), "{what}, saved with additions queued");
            assert!(
                snapshot(&again) == snapshot(&loaded),
                "{what}: the same state after the run"
            );
            // The whole input in one run: the same sets.
            let mut whole = reasoned(triples, switches);
            same_sets(&mut whole, &mut again, &format!("{what}, against one run"));
        }
    }
}

/// Brick with small1, saved to a file and loaded back; and Brick alone
/// saved, loaded, then small1 added, against both added without a snapshot.
#[test]
fn a_round_trip_keeps_brick() {
    let Some(input) = support::data::input("brick-small1", &[BRICK, SMALL1]) else {
        return;
    };
    let Some(brick) = support::data::input("brick", &[BRICK]) else {
        return;
    };
    let dir = turbowl_oracle::workspace_root().join("target/oracle");
    std::fs::create_dir_all(&dir).unwrap();
    let triples = input.triples();
    for (configuration, switches) in configurations() {
        let what = format!("Brick + small1 ({configuration})");
        let start = Instant::now();
        let mut r = reasoned(triples, switches);
        let reasoned_in = start.elapsed();
        let path = dir.join(format!("snapshot-brick-small1-{configuration}.snapshot"));
        let start = Instant::now();
        r.save_snapshot(&path).unwrap();
        let saved_in = start.elapsed();
        let start = Instant::now();
        let mut loaded = Reasoner::from_snapshot(&path).unwrap_or_else(|e| panic!("{what}: {e}"));
        let loaded_in = start.elapsed();
        println!(
            "{what}: {} triples in the closure, a snapshot of {} bytes; reasoned in {:.3} s, saved in {:.3} s, loaded in {:.3} s",
            r.triples().count(),
            std::fs::metadata(&path).unwrap().len(),
            reasoned_in.as_secs_f64(),
            saved_in.as_secs_f64(),
            loaded_in.as_secs_f64()
        );
        assert!(std::fs::read(&path).unwrap() == snapshot(&loaded));
        assert_eq!(loaded.reason().rounds, 0);
        same(&mut r, &mut loaded, &what);

        // Brick alone, saved; then small1 added, after the load and without it.
        let split = brick.triples().len();
        let mut kept = reasoned(&triples[..split], switches);
        let mut loaded = reloaded(&kept, &what);
        for r in [&mut kept, &mut loaded] {
            r.insert(triples[split..].iter().map(Triple::as_ref));
        }
        same(&mut kept, &mut loaded, &format!("{what}, small1 added after a load"));
        same_sets(&mut kept, &mut r, &format!("{what}, small1 added, against one run"));
        std::fs::remove_file(&path).unwrap();
    }
}
