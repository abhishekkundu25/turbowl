//! Cancellation through the public interface: a cancelled run keeps no
//! partial result and the next run equals one that was never cancelled; a
//! cancelled proof search keeps the indexes and the next question gets the
//! proof it would have got. `profile::cancel_after` cancels at a given
//! check, so these tests need no clock (but the ones that time a cancel
//! from another thread or a deadline).

use oxrdf::{NamedNode, Triple};
use std::time::{Duration, Instant};
use turbowl::{Cancel, Diagnostic, Error, Format, Reasoner, Switches, profile};

const PREFIXES: &str = "@prefix ex: <http://example.org/> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .\n";

/// Several rounds, merges, findings and an ill-typed literal.
const TEXT: &str = "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C . ex:C rdfs:subClassOf ex:G .
    ex:A owl:disjointWith ex:D .
    ex:p rdfs:domain ex:A ; rdfs:range ex:B .
    ex:t a owl:TransitiveProperty .
    ex:n1 ex:t ex:n2 . ex:n2 ex:t ex:n3 . ex:n3 ex:t ex:n4 . ex:n4 ex:t ex:n5 .
    ex:x ex:p ex:y . ex:y ex:p ex:z . ex:x a ex:D .
    ex:x owl:sameAs ex:x2 . ex:x2 owl:sameAs ex:x3 .
    ex:q owl:sameAs ex:p .
    ex:z ex:age \"twelve\"^^<http://www.w3.org/2001/XMLSchema#integer> .";

/// More of it, with a merge of a vocabulary term, which restarts the run.
const MORE: &str = "ex:w ex:p ex:x . ex:z owl:sameAs ex:w . ex:C owl:sameAs ex:E .
    ex:sub owl:sameAs rdfs:subClassOf . ex:E ex:sub ex:F .";

/// What the reads give, in read order.
type State = (Vec<String>, Vec<Diagnostic>);

fn state(r: &Reasoner) -> State {
    (r.triples().map(|t| t.to_string()).collect(), r.diagnostics().to_vec())
}

/// What the reads give, in an order that does not depend on the history of
/// the reasoner's dictionary.
fn sorted(r: &Reasoner) -> (Vec<String>, Vec<String>) {
    let (mut triples, diagnostics) = state(r);
    triples.sort();
    let mut diagnostics: Vec<String> = diagnostics.iter().map(|d| format!("{d:?}")).collect();
    diagnostics.sort();
    (triples, diagnostics)
}

/// The checks to cancel at: every one of the first 64, then a growing
/// step, so that a long search is cancelled early, halfway and late
/// without a search per check.
fn checks() -> impl Iterator<Item = usize> {
    (0..64).chain(std::iter::successors(Some(64usize), |k| Some(k + k / 2)))
}

fn load(r: &mut Reasoner, text: &str) {
    r.load_str(&format!("{PREFIXES}{text}"), Format::Turtle, None)
        .unwrap_or_else(|e| panic!("{e}"));
}

fn ex(local: &str) -> NamedNode {
    NamedNode::new_unchecked(format!("http://example.org/{local}"))
}

/// Cancels the run `prepare` leaves to do at each of its checks in turn,
/// until one run is not cancelled: each cancelled run leaves the reasoner as
/// loaded and not yet reasoned, and the next run, a full one, equals a full
/// run of `base` in a fresh reasoner. The run that is not cancelled equals
/// the run `prepare` leaves. Returns the runs cancelled.
fn cancel_at_every_check(switches: Switches, base: &[&str], prepare: impl Fn() -> Reasoner) -> usize {
    let mut full = Reasoner::with_switches(switches);
    for text in base {
        load(&mut full, text);
    }
    let full = (full.reason(), sorted(&full));
    let mut r = prepare();
    let uncancelled = (r.reason(), state(&r));
    for k in checks() {
        let mut r = prepare();
        match r.reason_cancellable(&profile::cancel_after(k)) {
            Err(Error::Cancelled) => {
                assert_eq!(r.triples().count(), 0, "check {k}: no partial closure");
                assert!(r.diagnostics().is_empty(), "check {k}: no partial diagnostics");
                assert!(matches!(r.explainer(), Err(Error::NotReasoned)), "check {k}");
                let summary = r.reason();
                assert_eq!(summary, full.0, "check {k}: the run after the cancel");
                assert!(sorted(&r) == full.1, "check {k}: the closure after the cancel");
            }
            Ok(summary) => {
                assert_eq!(summary, uncancelled.0, "check {k}: not cancelled");
                assert!(state(&r) == uncancelled.1, "check {k}: the closure, not cancelled");
                return k;
            }
            Err(e) => panic!("check {k}: {e}"),
        }
    }
    unreachable!()
}

/// A reasoner of `TEXT`, reasoned, then `more` loaded.
fn prepared(more: &[&str]) -> Reasoner {
    let mut r = Reasoner::new();
    load(&mut r, TEXT);
    r.reason();
    for text in more {
        load(&mut r, text);
    }
    r
}

#[test]
fn a_tripped_handle_changes_nothing() {
    let mut r = Reasoner::new();
    load(&mut r, TEXT);
    let cancel = Cancel::new();
    cancel.cancel();
    assert!(matches!(r.reason_cancellable(&cancel), Err(Error::Cancelled)));
    let fresh = r.reason();
    let reasoned = state(&r);

    // With additions queued (an incremental run to do) or after a removal
    // (a full run to do), the reads still show the last run, and the run
    // after is the one there was to do.
    let removed = Triple::new(ex("x"), ex("p"), ex("y"));
    for prepare in [
        &(|| {
            let mut r = prepared(&[]);
            load(&mut r, MORE);
            r
        }) as &dyn Fn() -> Reasoner,
        &|| {
            let mut r = prepared(&[]);
            r.remove([removed.as_ref()]);
            r
        },
    ] {
        let mut expected = prepare();
        let shown = state(&expected);
        let expected = (expected.reason(), state(&expected));
        let mut r = prepare();
        assert!(matches!(r.reason_cancellable(&cancel), Err(Error::Cancelled)));
        assert!(state(&r) == shown, "the reads still show the last run");
        assert!(
            matches!(r.explainer(), Err(Error::NotReasoned)),
            "the base changed and was not reasoned"
        );
        assert_eq!(r.reason(), expected.0, "the run there was to do");
        assert!(state(&r) == expected.1);
    }

    // A reasoned closure stays reasoned and explained.
    let mut again = Reasoner::new();
    load(&mut again, TEXT);
    assert_eq!(again.reason(), fresh);
    assert_eq!(state(&again), reasoned);
    assert!(matches!(again.reason_cancellable(&cancel), Err(Error::Cancelled)));
    assert_eq!(state(&again), reasoned);
    assert!(again.explainer().is_ok());
}

#[test]
fn a_full_run_cancelled_at_any_check_keeps_nothing() {
    for switches in [Switches::default(), Switches::complete()] {
        let prepare = || {
            let mut r = Reasoner::with_switches(switches);
            load(&mut r, TEXT);
            r
        };
        let summary = prepare().reason();
        assert!(summary.rounds > 3 && summary.diagnostics > 0, "{summary:?}");
        let cancelled = cancel_at_every_check(switches, &[TEXT], prepare);
        assert!(cancelled >= summary.rounds, "a check per round: {cancelled}");
    }
}

#[test]
fn an_incremental_run_cancelled_at_any_check_keeps_nothing() {
    for switches in [Switches::default(), Switches::complete()] {
        // MORE merges a vocabulary term, so the run restarts from the base.
        for more in [MORE, "ex:w ex:p ex:x . ex:n5 ex:t ex:n6 ."] {
            let prepare = || {
                let mut r = Reasoner::with_switches(switches);
                load(&mut r, TEXT);
                r.reason();
                load(&mut r, more);
                r
            };
            assert!(cancel_at_every_check(switches, &[TEXT, more], prepare) > 0);
        }
    }
}

#[test]
fn a_run_after_a_removal_cancelled_at_any_check_keeps_nothing() {
    let removed = Triple::new(ex("x"), ex("p"), ex("y"));
    let prepare = || {
        let mut r = Reasoner::new();
        load(&mut r, TEXT);
        r.reason();
        r.remove([removed.as_ref()]);
        r
    };
    let without = TEXT.replace("ex:x ex:p ex:y . ", "");
    assert!(cancel_at_every_check(Switches::default(), &[&without], prepare) > 0);
}

/// A run cancelled from another thread stops within its longest round and
/// step, plus 10 ms for the thread to be scheduled.
#[test]
fn a_cancel_from_another_thread_stops_the_run_within_a_round() {
    // A chain of existentials: one class more per round, each round short.
    let mut text = String::from("ex:x0 a ex:R0 .\n");
    for i in 0..2000 {
        let j = i + 1;
        text.push_str(&format!(
            "ex:x{j} ex:p ex:x{i} . ex:R{j} owl:someValuesFrom ex:R{i} ; owl:onProperty ex:p .\n"
        ));
    }
    let mut r = Reasoner::new();
    load(&mut r, &text);
    let start = Instant::now();
    r.reason();
    let whole = start.elapsed();
    // Between two checks: a round, then the step that applies its output.
    let profile = r.profile();
    let most = |ns: &mut dyn Iterator<Item = u64>| ns.max().unwrap_or(0);
    let longest = Duration::from_nanos(
        most(&mut profile.rounds.iter().map(|round| round.ns)) + most(&mut profile.steps.iter().map(|step| step.ns)),
    );
    assert!(r.profile().rounds.len() > 100, "{} rounds", r.profile().rounds.len());
    assert!(
        whole > longest * 4,
        "the run spans several rounds: {whole:?}, longest {longest:?}"
    );

    let mut r = Reasoner::new();
    load(&mut r, &text);
    let cancel = Cancel::new();
    let after = whole / 3;
    let (result, late) = std::thread::scope(|s| {
        let trip = cancel.clone();
        let tripped = s.spawn(move || {
            std::thread::sleep(after);
            trip.cancel();
            Instant::now()
        });
        let result = r.reason_cancellable(&cancel);
        let returned = Instant::now();
        (result, returned.saturating_duration_since(tripped.join().unwrap()))
    });
    assert!(matches!(result, Err(Error::Cancelled)), "{result:?}");
    assert!(
        late <= longest + Duration::from_millis(10),
        "returned {late:?} after the cancel; the longest round takes {longest:?}"
    );
    assert_eq!(r.triples().count(), 0);
}

#[test]
fn a_proof_search_cancelled_at_any_check_gives_the_proof_next_time() {
    // A deep proof: x is a C30 through 30 subclass steps.
    let mut text = String::from("ex:x a ex:C0 .\n");
    for i in 0..30 {
        text.push_str(&format!("ex:C{i} rdfs:subClassOf ex:C{} .\n", i + 1));
    }
    text.push_str("ex:C30 owl:disjointWith ex:D . ex:x a ex:D .");
    for switches in [Switches::default(), Switches::complete()] {
        let mut r = Reasoner::with_switches(switches);
        load(&mut r, &text);
        r.reason();
        let goal = Triple::new(ex("x"), oxrdf::vocab::rdf::TYPE, ex("C30"));
        let goal = goal.as_ref();
        let fresh = r.explain(goal).unwrap().expect("in the closure");
        assert!(fresh.depth() > 1);
        let diagnostic = r.diagnostics().first().expect("cax-dw").clone();
        let fresh_diagnostic = r.explain_diagnostic(&diagnostic).unwrap().expect("reported");

        // The bytes of the indexes alone: an explainer that built them (a
        // triple looked up, outside the closure) and searched nothing.
        // Known terms, so that the lookup builds the indexes.
        let outside = Triple::new(ex("C0"), oxrdf::vocab::rdf::TYPE, ex("D"));
        let indexes = {
            let explainer = r.explainer().unwrap();
            assert!(explainer.explain(outside.as_ref()).is_none());
            profile::explain_cache_bytes(&explainer.into_cache())
        };
        assert!(indexes > 0);
        let mut cancelled = 0;
        for k in checks() {
            let explainer = r.explainer().unwrap();
            match explainer.explain_cancellable(goal, &profile::cancel_after(k)) {
                Err(Error::Cancelled) => {
                    cancelled += 1;
                    // The indexes are kept: the cache holds at least what one that
                    // searched nothing holds (a search builds more of them).
                    let cache = explainer.into_cache();
                    let bytes = profile::explain_cache_bytes(&cache);
                    assert!(
                        bytes >= indexes,
                        "check {k}: {bytes} bytes, the indexes alone {indexes}"
                    );
                    let again = r.explainer_with(cache).unwrap();
                    assert_eq!(again.explain(goal).as_ref(), Some(&fresh), "check {k}");
                    assert_eq!(again.explain_diagnostic(&diagnostic).as_ref(), Some(&fresh_diagnostic));
                }
                Ok(proof) => {
                    assert_eq!(proof.as_ref(), Some(&fresh), "check {k}: not cancelled");
                    break;
                }
                Err(e) => panic!("check {k}: {e}"),
            }
        }
        assert!(cancelled > 5, "a check per search step: {cancelled}");

        for k in checks() {
            let explainer = r.explainer().unwrap();
            match explainer.explain_diagnostic_cancellable(&diagnostic, &profile::cancel_after(k)) {
                Err(Error::Cancelled) => {
                    assert_eq!(
                        explainer.explain_diagnostic(&diagnostic).as_ref(),
                        Some(&fresh_diagnostic)
                    );
                    assert_eq!(explainer.explain(goal).as_ref(), Some(&fresh), "check {k}");
                }
                Ok(proof) => {
                    assert_eq!(proof.as_ref(), Some(&fresh_diagnostic), "check {k}: not cancelled");
                    break;
                }
                Err(e) => panic!("check {k}: {e}"),
            }
        }

        // A tripped handle stops before any search; a triple outside the
        // closure is still `None`.
        let tripped = Cancel::new();
        tripped.cancel();
        let explainer = r.explainer().unwrap();
        assert!(matches!(
            explainer.explain_cancellable(goal, &tripped),
            Err(Error::Cancelled)
        ));
        assert!(matches!(
            explainer.explain_cancellable(outside.as_ref(), &Cancel::new()),
            Ok(None)
        ));
    }
}

/// A proof search cancelled from another thread stops within 10 ms.
#[test]
fn a_cancel_from_another_thread_stops_a_search_within_10_ms() {
    // A chain of existentials: the proof of the last class is as deep as
    // the chain, and its search long.
    let n = 12000;
    let mut text = String::from("ex:x0 a ex:R0 .\n");
    for i in 0..n {
        let j = i + 1;
        text.push_str(&format!(
            "ex:x{j} ex:p ex:x{i} . ex:R{j} owl:someValuesFrom ex:R{i} ; owl:onProperty ex:p .\n"
        ));
    }
    let mut r = Reasoner::new();
    load(&mut r, &text);
    r.reason();
    let goal = Triple::new(ex(&format!("x{n}")), oxrdf::vocab::rdf::TYPE, ex(&format!("R{n}")));
    let outside = Triple::new(ex("x0"), oxrdf::vocab::rdf::TYPE, ex("R1"));
    // Each explainer builds its indexes first, so that the times are the
    // search's alone.
    let explainer = r.explainer().unwrap();
    assert!(explainer.explain(outside.as_ref()).is_none());
    let start = Instant::now();
    assert!(explainer.explain(goal.as_ref()).is_some());
    let whole = start.elapsed();
    assert!(whole > Duration::from_millis(30), "a long search: {whole:?}");

    let explainer = r.explainer().unwrap();
    assert!(explainer.explain(outside.as_ref()).is_none());
    let cancel = Cancel::new();
    let (result, late) = std::thread::scope(|s| {
        let trip = cancel.clone();
        let tripped = s.spawn(move || {
            std::thread::sleep(whole / 3);
            trip.cancel();
            Instant::now()
        });
        let result = explainer.explain_cancellable(goal.as_ref(), &cancel);
        let returned = Instant::now();
        (result, returned.saturating_duration_since(tripped.join().unwrap()))
    });
    assert!(matches!(result, Err(Error::Cancelled)), "{result:?}");
    assert!(late <= Duration::from_millis(10), "returned {late:?} after the cancel");
}

/// A handle with a deadline stops a long search on the searching thread,
/// with no other thread to trip it.
#[test]
fn a_deadline_stops_a_long_search() {
    // x is a C400 through a 400-link subclass chain: the search takes
    // seconds (about 6 s unbounded on the development machine).
    let n = 400;
    let mut text = String::from("ex:x a ex:C0 .\n");
    for i in 0..n {
        text.push_str(&format!("ex:C{i} rdfs:subClassOf ex:C{} .\n", i + 1));
    }
    let mut r = Reasoner::new();
    load(&mut r, &text);
    r.reason();
    let goal = Triple::new(ex("x"), oxrdf::vocab::rdf::TYPE, ex(&format!("C{n}")));
    let explainer = r.explainer().unwrap();
    let limit = Duration::from_millis(200);
    let start = Instant::now();
    let deadline = start + limit;
    let cancel = Cancel::with_check(move || Instant::now() >= deadline);
    let result = explainer.explain_cancellable(goal.as_ref(), &cancel);
    let elapsed = start.elapsed();
    assert!(matches!(result, Err(Error::Cancelled)), "{result:?}");
    assert!(cancel.is_cancelled());
    // Generous: a step between two checks, and the drop of the search's
    // graph after the cancel, on a loaded machine.
    assert!(
        elapsed <= limit + Duration::from_secs(2),
        "returned after {elapsed:?}, the limit {limit:?}"
    );
}

/// `ex:K0 … ex:K{n-1}`, the members of a list.
fn members(n: usize) -> String {
    (0..n).map(|i| format!("ex:K{i} ")).collect()
}

/// Runs `explain` under a handle with a deadline `limit` from now: it is
/// cancelled, and returns soon after the deadline.
fn stopped_at(limit: Duration, explain: impl FnOnce(&Cancel) -> Result<Option<turbowl::Proof>, Error>) {
    let start = Instant::now();
    let deadline = start + limit;
    let cancel = Cancel::with_check(move || Instant::now() >= deadline);
    let result = explain(&cancel);
    let elapsed = start.elapsed();
    assert!(
        matches!(result, Err(Error::Cancelled)),
        "{:?} after {elapsed:?}",
        result.map(|p| p.map(|p| p.steps().len()))
    );
    assert!(cancel.is_cancelled());
    // Generous: the work between two checks, and the drop of the search's
    // graph after the cancel, on a loaded machine.
    assert!(
        elapsed <= limit + Duration::from_secs(1),
        "returned after {elapsed:?}, the limit {limit:?}"
    );
}

/// A proof through a long list stops at the deadline, while it is searched
/// and while it is built: its steps fold the list's cells into the step
/// that reads them. Built once, without a deadline, the proof of a list of
/// 32,000 members takes about 0.15 s (release, an M-series Mac); it took
/// 17 s and 8 GB, unchecked, when each cell copied its tail.
#[test]
fn a_deadline_stops_a_proof_through_a_long_list() {
    let n = 32_000;
    let limit = Duration::from_millis(10);
    // cax-adc over the members of an AllDisjointClasses, and cls-uni over
    // the last member of a union.
    let disjoint = format!(
        "[] a owl:AllDisjointClasses ; owl:members ( {} ) . ex:x a ex:K0 , ex:K1 .",
        members(n)
    );
    let union = format!("ex:U owl:unionOf ( {} ) . ex:x a ex:K{} .", members(n), n - 1);
    let goal = Triple::new(ex("x"), oxrdf::vocab::rdf::TYPE, ex("U"));
    for (text, finding) in [(disjoint, true), (union, false)] {
        let mut r = Reasoner::new();
        load(&mut r, &text);
        r.reason();
        let diagnostic = r.diagnostics().first().cloned();
        assert_eq!(diagnostic.is_some(), finding, "{:?}", r.diagnostics());
        let explain = |explainer: &turbowl::Explainer<'_>, cancel: &Cancel| match &diagnostic {
            Some(d) => explainer.explain_diagnostic_cancellable(d, cancel),
            None => explainer.explain_cancellable(goal.as_ref(), cancel),
        };
        // Searched: the deadline passes while the list is read.
        let explainer = r.explainer().unwrap();
        stopped_at(limit, |cancel| explain(&explainer, cancel));
        // Built: an explainer that found the proof already searches
        // nothing (about 25 ms, all building, on the development machine),
        // and a deadline a fifth of that passes while the proof is built.
        let explainer = r.explainer().unwrap();
        let proof = explain(&explainer, &Cancel::new()).unwrap().expect("a proof");
        assert!(proof.steps().len() > 2 * n, "{} steps", proof.steps().len());
        let start = Instant::now();
        assert_eq!(explain(&explainer, &Cancel::new()).unwrap(), Some(proof));
        let built = start.elapsed();
        stopped_at(built / 5, |cancel| explain(&explainer, cancel));
    }
}
