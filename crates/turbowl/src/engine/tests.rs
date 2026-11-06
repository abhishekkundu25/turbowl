//! The engine against naive nested loops, on seeded generated relations.

use super::*;
use std::collections::BTreeSet;
use std::time::Instant;

/// A tiny xorshift64* generator: seeded, reproducible, no dependency.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u32) -> u32 {
        (self.next() % u64::from(n)) as u32
    }
    /// `count` random pairs over `0..domain`.
    fn pairs(&mut self, count: usize, domain: u32) -> Vec<(u32, u32)> {
        (0..count).map(|_| (self.below(domain), self.below(domain))).collect()
    }
    /// Fewer than `most` random pairs over `0..domain`.
    fn some_pairs(&mut self, most: u32, domain: u32) -> Vec<(u32, u32)> {
        let count = self.below(most) as usize;
        self.pairs(count, domain)
    }
}

const SEEDS: u64 = 200;

fn set<T: Ord + Copy>(items: impl IntoIterator<Item = T>) -> BTreeSet<T> {
    items.into_iter().collect()
}

fn naive_closure(edges: &BTreeSet<(u32, u32)>) -> BTreeSet<(u32, u32)> {
    let mut closure = edges.clone();
    loop {
        let mut next = closure.clone();
        for &(x, y) in &closure {
            for &(y2, z) in edges {
                if y == y2 {
                    next.insert((x, z));
                }
            }
        }
        if next.len() == closure.len() {
            return closure;
        }
        closure = next;
    }
}

#[test]
fn gallop_matches_partition_point() {
    let mut rng = Rng::new(1);
    for _ in 0..2000 {
        let mut v: Vec<u32> = (0..rng.below(100)).map(|_| rng.below(200)).collect();
        v.sort_unstable();
        let x = rng.below(220);
        let rest = gallop(&v, |y| *y < x);
        assert_eq!(v.len() - rest.len(), v.partition_point(|y| *y < x));
    }
}

#[test]
fn relation_merge_is_union() {
    let mut rng = Rng::new(2);
    for _ in 0..2000 {
        let a: Vec<u32> = (0..rng.below(60)).map(|_| rng.below(80)).collect();
        let b: Vec<u32> = (0..rng.below(60)).map(|_| rng.below(80)).collect();
        let merged = Relation::from_vec(a.clone()).merge(Relation::from_vec(b.clone()));
        let expected: Vec<u32> = set(a.into_iter().chain(b)).into_iter().collect();
        assert_eq!(&merged[..], &expected[..]);
    }
}

#[test]
fn variable_recent_holds_only_new_tuples() {
    let mut rng = Rng::new(3);
    for _ in 0..300 {
        let mut var = Variable::new();
        let mut seen = BTreeSet::new();
        for _ in 0..rng.below(12) + 1 {
            let mut batch = BTreeSet::new();
            for _ in 0..rng.below(4) {
                let tuples: Vec<u32> = (0..rng.below(40)).map(|_| rng.below(300)).collect();
                batch.extend(tuples.iter().copied());
                var.extend(tuples);
            }
            let changed = var.changed();
            let new: BTreeSet<u32> = batch.difference(&seen).copied().collect();
            assert_eq!(set(var.recent().iter().copied()), new);
            assert_eq!(changed, !new.is_empty());
            seen.extend(new);
            assert_eq!(var.len(), seen.len());
            assert_eq!(set(var.all().iter()), seen);
            let stable_total: usize = var.stable().iter().map(|b| b.len()).sum();
            assert_eq!(stable_total + var.recent().len(), seen.len(), "parts are disjoint");
        }
        while var.changed() {}
        assert_eq!(set(var.complete().iter().copied()), seen);
    }
}

#[test]
fn variable_remap_moves_rewritten_tuples_to_the_delta() {
    let mut rng = Rng::new(4);
    for _ in 0..300 {
        let mut var = Variable::new();
        let mut seen = BTreeSet::new();
        for _ in 0..rng.below(6) + 1 {
            let pairs = rng.some_pairs(30, 20);
            seen.extend(pairs.iter().copied());
            var.extend(pairs);
            var.changed();
        }
        let (from, to) = (rng.below(20), rng.below(20));
        let f = |x: u32| if x == from { to } else { x };
        let g = |(a, b): (u32, u32)| (f(a), f(b));
        var.remap(g);
        while var.changed() {}
        let expected = set(seen.iter().map(|&t| g(t)));
        assert_eq!(set(var.all().iter()), expected);
    }
}

/// Semi-naive linear transitive closure with variables: path(x, z) :- edge(x, y), path(y, z).
fn closure_with_variables(edges: &[(u32, u32)]) -> (Variable<(u32, u32)>, usize) {
    let edges = Relation::from_vec(edges.to_vec());
    // path keyed by its first node, so it joins edge's second node: (y, z).
    let mut path: Variable<(u32, u32)> = Variable::new();
    path.insert(edges.clone());
    let by_target = Relation::from_vec(edges.iter().map(|&(x, y)| (y, x)).collect());
    let by_target = View::from(&by_target);
    let mut rounds = 0;
    let mut out = Vec::new();
    while path.changed() {
        rounds += 1;
        join_static(&path, &by_target, |_y, z, x| out.push((x, z)));
        path.insert(Relation::from_vec(std::mem::take(&mut out)));
    }
    (path, rounds)
}

#[test]
fn transitive_closure_matches_naive() {
    for seed in 0..SEEDS {
        let mut rng = Rng::new(seed);
        let domain = rng.below(30) + 2;
        let edges = rng.some_pairs(60, domain);
        let expected = naive_closure(&set(edges.iter().copied()));
        let (path, _) = closure_with_variables(&edges);
        assert_eq!(set(path.complete().iter().copied()), expected, "seed {seed}");
    }
}

/// Transitive closure as prp-trp writes it over the store: (x p y), (y p z) ⇒ (x p z),
/// a self-join through the object index the rule asks for.
fn store_transitive(round: &mut Round<'_>, p: u32) {
    let (Some(so), Some(os)) = (round.by_subject(p), round.by_object(p)) else {
        return;
    };
    join(os, so, |_y, x, z| round.emit(x, p, z));
}

#[test]
fn store_transitive_closure_matches_naive_incrementally() {
    const P: u32 = 7;
    const Q: u32 = 8;
    for seed in 0..SEEDS {
        let mut rng = Rng::new(seed);
        let domain = rng.below(25) + 2;
        let mut store = Store::new();
        let mut all = BTreeSet::new();
        let mut other = BTreeSet::new();
        // Several runs: each adds edges and continues from the last fixpoint.
        for _ in 0..rng.below(4) + 1 {
            let edges = rng.some_pairs(30, domain);
            all.extend(edges.iter().copied());
            for &(s, o) in &edges {
                store.insert(s, P, o);
            }
            // An unrelated predicate rides along untouched.
            let extra = rng.some_pairs(5, domain);
            other.extend(extra.iter().copied());
            store.extend(extra.iter().map(|&(s, o)| (s, Q, o)));
            store.run(|round| store_transitive(round, P));
            let got = set(store.triples().filter(|t| t.1 == P).map(|(s, _, o)| (s, o)));
            assert_eq!(got, naive_closure(&all), "seed {seed}");
            let got_other = set(store.triples().filter(|t| t.1 == Q).map(|(s, _, o)| (s, o)));
            assert_eq!(got_other, other, "seed {seed}");
            assert_eq!(store.len(), got.len() + got_other.len());
            if let Some(&(s, o)) = all.iter().next() {
                assert!(store.contains(s, P, o));
            }
        }
    }
}

/// The round buffer grows to the largest round's derivations, duplicates
/// included: on OpenCyc, 3.5 GB of the 5.2 GB a reasoned store held
/// (ticket 25). A run gives it back when it ends; the next run grows it
/// again.
#[test]
fn a_run_keeps_no_round_buffer() {
    const P: u32 = 7;
    let mut store = Store::new();
    // A chain of 200: its closure is derived many times over.
    store.extend((0..200).map(|i| (i, P, i + 1)));
    store.run(|round| store_transitive(round, P));
    assert_eq!(store.len(), 200 * 201 / 2);
    assert_eq!(store.buffered(), 0, "the buffer is released when the run ends");
    // Queued triples still wait in it for the next run.
    store.insert(500, P, 0);
    assert!(store.buffered() > 0);
    store.run(|round| store_transitive(round, P));
    assert_eq!(store.len(), 200 * 201 / 2 + 201);
    assert_eq!(store.buffered(), 0);
}

/// out(a, d) :- r(a, b), s(b, c), t(c, d), with r, s and t all growing over
/// rounds, as two semi-naive binary joins through an intermediate variable.
#[test]
fn three_relation_join_matches_naive() {
    for seed in 0..SEEDS {
        let mut rng = Rng::new(seed);
        let domain = rng.below(15) + 2;
        let mut r_by_b: Variable<(u32, u32)> = Variable::new(); // (b, a)
        let mut s_by_b: Variable<(u32, u32)> = Variable::new(); // (b, c)
        let mut t_by_c: Variable<(u32, u32)> = Variable::new(); // (c, d)
        let mut rs_by_c: Variable<(u32, u32)> = Variable::new(); // (c, a)
        let mut out: Variable<(u32, u32)> = Variable::new(); // (a, d)
        let (mut r, mut s, mut t) = (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
        let batches = rng.below(5) + 1;
        let mut round = 0;
        let mut buffer = Vec::new();
        loop {
            if round < batches {
                let (nr, ns, nt) = (rng.below(12), rng.below(12), rng.below(12));
                let rb = rng.pairs(nr as usize, domain);
                let sb = rng.pairs(ns as usize, domain);
                let tb = rng.pairs(nt as usize, domain);
                r.extend(rb.iter().copied());
                s.extend(sb.iter().copied());
                t.extend(tb.iter().copied());
                r_by_b.extend(rb.into_iter().map(|(a, b)| (b, a)));
                s_by_b.extend(sb);
                t_by_c.extend(tb);
            }
            round += 1;
            let changed = r_by_b.changed() | s_by_b.changed() | t_by_c.changed() | rs_by_c.changed() | out.changed();
            if !changed && round > batches {
                break;
            }
            join(&r_by_b, &s_by_b, |_b, a, c| buffer.push((c, a)));
            rs_by_c.insert(Relation::from_vec(std::mem::take(&mut buffer)));
            join(&rs_by_c, &t_by_c, |_c, a, d| buffer.push((a, d)));
            out.insert(Relation::from_vec(std::mem::take(&mut buffer)));
        }
        let mut expected = BTreeSet::new();
        for &(a, b) in &r {
            for &(b2, c) in &s {
                for &(c2, d) in &t {
                    if b == b2 && c == c2 {
                        expected.insert((a, d));
                    }
                }
            }
        }
        assert_eq!(set(out.complete().iter().copied()), expected, "seed {seed}");
    }
}

#[test]
fn antijoin_matches_naive() {
    for seed in 0..SEEDS {
        let mut rng = Rng::new(seed);
        let domain = rng.below(20) + 1;
        let mut input: Variable<(u32, u32)> = Variable::new();
        input.extend(rng.some_pairs(50, domain));
        input.changed();
        // Keys in several parts, as a variable's batches are.
        let mut keys: Variable<u32> = Variable::new();
        for _ in 0..rng.below(4) {
            keys.extend((0..rng.below(8)).map(|_| rng.below(domain)));
            keys.changed();
        }
        let key_set = set(keys.all().iter());
        let mut got = Vec::new();
        antijoin(&input, &keys.all(), |k, v| got.push((k, v)));
        let expected = set(input.recent().iter().copied().filter(|t| !key_set.contains(&t.0)));
        assert_eq!(got.len(), expected.len(), "seed {seed}: each tuple once");
        assert_eq!(set(got), expected, "seed {seed}");
    }
}

/// Builds a variable whose tuples sit in several stable batches and a recent part.
type Pairs = BTreeSet<(u32, u32)>;

fn variable_in_parts(rng: &mut Rng, most: u32, domain: u32) -> (Variable<(u32, u32)>, Pairs) {
    let mut var = Variable::new();
    let mut all = BTreeSet::new();
    for _ in 0..rng.below(4) + 1 {
        let pairs = rng.some_pairs(most, domain);
        all.extend(pairs.iter().copied());
        var.extend(pairs);
        var.changed();
    }
    (var, all)
}

/// q(a, b, c) :- r(a, b), s(b, c), t(a, c), not u(b, c), c != a, and v(a)
/// holds; triangles, through leapjoin with every leaper kind.
#[test]
fn leapjoin_matches_naive() {
    for seed in 0..SEEDS {
        let mut rng = Rng::new(seed);
        let domain = rng.below(12) + 2;
        let r = set(rng.some_pairs(40, domain));
        let (s, s_set) = variable_in_parts(&mut rng, 25, domain);
        let (t, t_set) = variable_in_parts(&mut rng, 25, domain);
        let (u, u_set) = variable_in_parts(&mut rng, 10, domain);
        let mut v: Variable<u32> = Variable::new();
        v.extend((0..rng.below(domain + 1)).map(|_| rng.below(domain)));
        v.changed();
        let v_set = set(v.all().iter());
        let mut w: Variable<u32> = Variable::new();
        w.extend((0..rng.below(4)).map(|_| rng.below(domain)));
        w.changed();
        let w_set = set(w.all().iter());

        let source: Vec<(u32, u32)> = r.iter().copied().collect();
        let (s_all, t_all, u_all, v_all, w_all) = (s.all(), t.all(), u.all(), v.all(), w.all());
        let mut got = Vec::new();
        leapjoin(
            &source,
            &mut (
                extend_with(&s_all, |&(_a, b): &(u32, u32)| b),
                extend_with(&t_all, |&(a, _b): &(u32, u32)| a),
                extend_anti(&u_all, |&(_a, b): &(u32, u32)| b),
                value_filter(|&(a, _b): &(u32, u32), &c: &u32| c != a),
                filter_with(&v_all, |&(a, _b): &(u32, u32)| a),
            ),
            |&(a, b), c| got.push((a, b, c)),
        );
        // FilterAnti in its own join, to cover it too.
        let mut got_anti = Vec::new();
        leapjoin(
            &source,
            &mut (
                extend_with(&s_all, |&(_a, b): &(u32, u32)| b),
                filter_anti(&w_all, |&(a, _b): &(u32, u32)| a),
            ),
            |&(a, b), c| got_anti.push((a, b, c)),
        );

        let mut expected = BTreeSet::new();
        let mut expected_anti = BTreeSet::new();
        for &(a, b) in &r {
            for &(b2, c) in &s_set {
                if b2 != b {
                    continue;
                }
                if !w_set.contains(&a) {
                    expected_anti.insert((a, b, c));
                }
                if t_set.contains(&(a, c)) && !u_set.contains(&(b, c)) && c != a && v_set.contains(&a) {
                    expected.insert((a, b, c));
                }
            }
        }
        assert_eq!(got.len(), expected.len(), "seed {seed}: each result once");
        assert_eq!(set(got), expected, "seed {seed}");
        assert_eq!(got_anti.len(), expected_anti.len(), "seed {seed}");
        assert_eq!(set(got_anti), expected_anti, "seed {seed}");
    }
}

/// path(x, z) :- path(x, y), edge(y, z), over the store: `path` joined on its
/// object with `edge` on its subject, as a rule family would write it.
fn store_linear_closure(round: &mut Round<'_>, path: u32, edge: u32) {
    let (Some(path_os), Some(edge_so)) = (round.by_object(path), round.by_subject(edge)) else {
        return;
    };
    join(path_os, edge_so, |_y, x, z| round.emit(x, path, z));
}

/// The ticket's benchmark: transitive closure over 100,000 chain edges,
/// through the store. The full closure of one 100,000-edge chain has
/// 5,000,050,000 pairs (40 GB), so it is measured in two feasible shapes:
/// the closure row of the chain's head (100,000 rounds, one new pair each:
/// the per-round overhead), and the full closure of 1,000 disjoint chains of
/// 100 edges (5,050,000 pairs in 100 rounds: the join and merge throughput).
#[test]
fn timing_transitive_closure_100k_edge_chain() {
    // Not beside a scaling test: neither would time alone.
    let _lock = crate::scaling::lock();
    const EDGES: u32 = 100_000;
    const PATH: u32 = 1;
    const EDGE: u32 = 2;

    let mut store = Store::new();
    store.extend((0..EDGES).map(|i| (i, EDGE, i + 1)));
    store.insert(0, PATH, 0);
    let started = Instant::now();
    let rounds = store.run(|round| store_linear_closure(round, PATH, EDGE));
    let elapsed = started.elapsed();
    let reached = store.triples().filter(|t| t.1 == PATH).count();
    assert_eq!(reached, EDGES as usize + 1);
    println!(
        "chain of {EDGES} edges, closure from its head: {reached} pairs, {rounds} rounds, {:.3} s",
        elapsed.as_secs_f64()
    );
    let total = elapsed;

    const CHAINS: u32 = 1_000;
    const LENGTH: u32 = EDGES / CHAINS;
    let edges: Vec<(u32, u32)> = (0..CHAINS)
        .flat_map(|c| {
            let base = c * (LENGTH + 1);
            (base..base + LENGTH).map(|i| (i, i + 1))
        })
        .collect();
    let mut store = Store::new();
    store.extend(edges.iter().map(|&(s, o)| (s, EDGE, o)));
    store.extend(edges.iter().map(|&(s, o)| (s, PATH, o)));
    let started = Instant::now();
    let rounds = store.run(|round| store_linear_closure(round, PATH, EDGE));
    let elapsed = started.elapsed();
    let pairs = store.triples().filter(|t| t.1 == PATH).count();
    let per_chain = (LENGTH as usize) * (LENGTH as usize + 1) / 2;
    assert_eq!(pairs, CHAINS as usize * per_chain);
    println!(
        "{CHAINS} chains of {LENGTH} edges, full closure: {pairs} pairs, {rounds} rounds, {:.3} s, {:.1} M pairs/s",
        elapsed.as_secs_f64(),
        pairs as f64 / elapsed.as_secs_f64() / 1e6
    );
    let total = total + elapsed;
    println!(
        "timing_transitive_closure_100k_edge_chain: {:.3} s",
        total.as_secs_f64()
    );
    assert!(total.as_secs() < 30, "far over budget");
}

/// Wall-clock: linear transitive closure of a 2,000-node chain (1,999,000
/// tuples), and reachability from 100 sources in a random 100,000-edge
/// graph. Plus the store's self-join form (as prp-trp) on a shorter chain.
#[test]
fn timing_engine_transitive_chain() {
    // Not beside a scaling test: neither would time alone.
    let _lock = crate::scaling::lock();
    const N: u32 = 2_000;
    let chain: Vec<(u32, u32)> = (0..N - 1).map(|i| (i, i + 1)).collect();
    let started = Instant::now();
    let (path, rounds) = closure_with_variables(&chain);
    let closure = path.len();
    let elapsed = started.elapsed();
    assert_eq!(closure, (N as usize) * (N as usize - 1) / 2);
    println!(
        "chain {N}: {closure} tuples, {rounds} rounds, {:.3} s, {:.1} M tuples/s",
        elapsed.as_secs_f64(),
        closure as f64 / elapsed.as_secs_f64() / 1e6
    );

    // Reachability: reach(s, z) :- reach(s, y), edge(y, z), from 100 sources.
    const NODES: u32 = 50_000;
    const EDGES: usize = 100_000;
    const SOURCES: u32 = 100;
    let mut rng = Rng::new(42);
    let edges = Relation::from_vec(rng.pairs(EDGES, NODES));
    let edges_view = View::from(&edges);
    let started = Instant::now();
    let mut reach: Variable<(u32, u32)> = Variable::new(); // (node, source)
    reach.extend((0..SOURCES).map(|s| (s, s)));
    let mut out = Vec::new();
    let mut rounds = 0;
    while reach.changed() {
        rounds += 1;
        join_static(&reach, &edges_view, |_y, s, z| out.push((z, s)));
        reach.insert(Relation::from_vec(std::mem::take(&mut out)));
    }
    let elapsed = started.elapsed();
    let reached = reach.len();
    println!(
        "random graph {NODES} nodes, {EDGES} edges, {SOURCES} sources: {reached} tuples, {rounds} rounds, \
         {:.3} s, {:.1} M tuples/s",
        elapsed.as_secs_f64(),
        reached as f64 / elapsed.as_secs_f64() / 1e6
    );

    // The store, as prp-trp does it: a self-join, cubic on a chain.
    const M: u32 = 400;
    const P: u32 = 1;
    let mut store = Store::new();
    store.extend((0..M - 1).map(|i| (i, P, i + 1)));
    let started = Instant::now();
    let rounds = store.run(|round| store_transitive(round, P));
    let elapsed = started.elapsed();
    assert_eq!(store.len(), (M as usize) * (M as usize - 1) / 2);
    println!(
        "store self-join chain {M}: {} tuples, {rounds} rounds, {:.3} s",
        store.len(),
        elapsed.as_secs_f64()
    );
    assert!(started.elapsed().as_secs() < 30, "far over budget");
}

/// Transitive closure of `p` as [`store_transitive`], through
/// [`Round::join`], and a copy of `q` into `r` read in runs
/// ([`Round::split_runs`]): both families as tasks ([`Round::tasks`]).
fn store_parallel_rules(round: &mut Round<'_>, p: u32, q: u32, r: u32) {
    let mut families = [0, 1];
    round.tasks(&mut families, |round, &mut family| {
        if family == 0 {
            let (Some(so), Some(os)) = (round.by_subject(p), round.by_object(p)) else {
                return;
            };
            round.join(os, so, |round, _y, x, z| round.emit(x, p, z));
        } else if let Some(table) = round.by_subject(q) {
            round.split_runs(table.recent(), |round, chunk, _: &mut ()| {
                let mut last = None;
                for &(x, y) in chunk {
                    // One triple per subject run, as prp-dom writes.
                    if last != Some(x) {
                        round.emit(x, r, y);
                        last = Some(x);
                    }
                }
            });
        }
    });
}

/// A store on one thread and on four, with every round parallel and chunks
/// of a few tuples, give the same triples in the same order, in the same
/// rounds, run after run.
#[test]
fn parallel_rounds_match_one_thread() {
    const P: u32 = 7;
    const Q: u32 = 8;
    const R: u32 = 9;
    for seed in 0..SEEDS {
        let mut rng = Rng::new(seed);
        let domain = rng.below(25) + 2;
        let mut one = Store::new();
        let mut four = Store::new();
        four.set_workers(Workers::exactly(4).every_round());
        for _ in 0..rng.below(4) + 1 {
            let edges = rng.some_pairs(40, domain);
            let extra = rng.some_pairs(20, domain);
            for store in [&mut one, &mut four] {
                store.extend(edges.iter().map(|&(s, o)| (s, P, o)));
                store.extend(extra.iter().map(|&(s, o)| (s, Q, o)));
            }
            let rounds = one.run(|round| store_parallel_rules(round, P, Q, R));
            assert_eq!(
                four.run(|round| store_parallel_rules(round, P, Q, R)),
                rounds,
                "seed {seed}"
            );
            assert_eq!(
                four.triples().collect::<Vec<_>>(),
                one.triples().collect::<Vec<_>>(),
                "seed {seed}"
            );
        }
        assert_eq!(one.engaged(), 0, "one thread never engages");
        assert!(four.rounds() == 0 || four.engaged() > 0, "seed {seed}");
    }
}

/// A store filled with sorted triples ([`Store::fill`]) starts its first
/// round with each table holding exactly its triples, in one sorted part
/// with no spare room, the tables made in ascending predicate order. It then
/// runs as a store given the same triples through the round buffer: the
/// same triples in the same order, the same rounds and the same counts in
/// every step. On one thread, and on four with every round parallel (the
/// triples cut into chunks).
#[test]
fn a_filled_store_starts_with_exact_tables() {
    const P: u32 = 7;
    const Q: u32 = 8;
    const R: u32 = 9;
    let predicates = [3, P, Q, R, 20];
    for seed in 0..SEEDS {
        let mut rng = Rng::new(seed);
        let domain = rng.below(25) + 2;
        let triple = |rng: &mut Rng| {
            let p = predicates[rng.below(predicates.len() as u32) as usize];
            [rng.below(domain), p, rng.below(domain)]
        };
        let mut triples: Vec<[u32; 3]> = (0..rng.below(120)).map(|_| triple(&mut rng)).collect();
        triples.sort_unstable();
        triples.dedup();
        // The facts: a few of the triples again, and a few new ones.
        let mut more: Vec<[u32; 3]> = triples.iter().copied().filter(|_| rng.below(8) == 0).collect();
        more.extend((0..rng.below(6)).map(|_| triple(&mut rng)));
        more.sort_unstable();
        more.dedup();
        let mut expected: BTreeSet<(u32, u32, u32)> = BTreeSet::new();
        expected.extend(triples.iter().chain(&more).map(|&[s, p, o]| (p, s, o)));

        for workers in [Workers::one(), Workers::exactly(4).every_round()] {
            let mut filled = Store::new();
            filled.set_workers(workers.clone());
            filled.fill(&triples, &more);
            assert_eq!(filled.buffered(), 0, "nothing goes through the round buffer");
            let mut queued = Store::new();
            queued.set_workers(workers);
            queued.extend(more.iter().map(|&[s, p, o]| (s, p, o)));
            queued.extend_from_slice(&triples);

            let mut checked = false;
            let rounds = filled.run(|round| {
                if round.number() == 1 {
                    let store = round.store();
                    let mut got: Vec<(u32, u32, u32)> = Vec::new();
                    for (p, table) in store.tables() {
                        let so = table.by_subject();
                        assert!(so.stable().is_empty(), "seed {seed}");
                        let recent = so.recent();
                        assert_eq!(recent.bytes(), recent.len() * size_of::<Pair>(), "seed {seed}");
                        assert!(table.by_object().is_none());
                        got.extend(recent.iter().map(|&(s, o)| (p, s, o)));
                    }
                    assert_eq!(got, expected.iter().copied().collect::<Vec<_>>(), "seed {seed}");
                    checked = true;
                }
                store_parallel_rules(round, P, Q, R);
            });
            assert_eq!(checked, !expected.is_empty(), "seed {seed}");
            assert_eq!(queued.run(|round| store_parallel_rules(round, P, Q, R)), rounds);
            assert_eq!(
                filled.triples().collect::<Vec<_>>(),
                queued.triples().collect::<Vec<_>>(),
                "seed {seed}"
            );
            let counts = |steps: Vec<crate::profile::Step>| -> Vec<(usize, usize)> {
                steps.iter().map(|s| (s.pairs, s.new)).collect()
            };
            assert_eq!(counts(filled.take_steps()), counts(queued.take_steps()), "seed {seed}");
            // Later runs continue from the filled store as from the other.
            let extra = rng.some_pairs(20, domain);
            for store in [&mut filled, &mut queued] {
                store.extend(extra.iter().map(|&(s, o)| (s, P, o)));
            }
            let rounds = filled.run(|round| store_parallel_rules(round, P, Q, R));
            assert_eq!(queued.run(|round| store_parallel_rules(round, P, Q, R)), rounds);
            assert_eq!(
                filled.triples().collect::<Vec<_>>(),
                queued.triples().collect::<Vec<_>>(),
                "seed {seed}"
            );
        }
    }
}

/// Past [`PARALLEL_FROM`] triples, the threads fill each table in chunks
/// side by side, and every table is what one thread gives: its pairs in
/// order, in a vector of exactly their length.
#[test]
fn filling_in_chunks_matches_one_thread() {
    let mut rng = Rng::new(5);
    let n = PARALLEL_FROM + 20_000;
    let mut triples: Vec<[u32; 3]> = (0..n)
        .map(|_| [rng.below(50_000), rng.below(40), rng.below(50_000)])
        .collect();
    triples.sort_unstable();
    triples.dedup();
    let len = triples.len();
    // The facts: triples of the base at and around every 64th of it (where
    // the chunks are cut), a predicate of their own, and a triple after
    // every other.
    let mut more = vec![[0, 99, 0], [60_000, 5, 1], triples[len - 1]];
    for k in 0..64 {
        let at = k * len / 64;
        more.extend_from_slice(&triples[at.saturating_sub(1)..(at + 2).min(len)]);
    }
    more.sort_unstable();
    more.dedup();
    let tables = |workers: Workers| -> Vec<(u32, Vec<Pair>, usize)> {
        let mut store = Store::new();
        store.set_workers(workers);
        store.fill(&triples, &more);
        let mut got = Vec::new();
        store.run(|round| {
            if round.number() == 1 {
                for (p, table) in round.store().tables() {
                    let recent = table.by_subject().recent();
                    got.push((p, recent.to_vec(), recent.bytes() / size_of::<Pair>()));
                }
            }
        });
        got
    };
    let one = tables(Workers::one());
    let mut expected: Vec<(u32, u32, u32)> = triples.iter().chain(&more).map(|&[s, p, o]| (p, s, o)).collect();
    expected.sort_unstable();
    expected.dedup();
    let flat: Vec<(u32, u32, u32)> = one
        .iter()
        .flat_map(|(p, pairs, _)| pairs.iter().map(move |&(s, o)| (*p, s, o)))
        .collect();
    assert_eq!(flat, expected);
    assert!(one.iter().all(|(_, pairs, room)| *room == pairs.len()));
    if Workers::exactly(4).threads() > 1 {
        assert!(
            Workers::exactly(4).engage(triples.len()),
            "the triples are cut into chunks"
        );
    }
    assert_eq!(tables(Workers::exactly(4)), one);
}

/// A forced split cuts a slice into several chunks, never inside a run of
/// one key ([`Round::split_runs`]); the chunks cover the slice in order.
#[test]
fn split_runs_keeps_each_key_whole() {
    let mut rng = Rng::new(3);
    let mut pairs = rng.pairs(500, 20);
    pairs.sort_unstable();
    pairs.dedup();
    let mut store = Store::new();
    store.set_workers(Workers::exactly(3).every_round());
    store.insert(0, 1, 2);
    let mut chunks: Vec<Vec<(u32, u32)>> = Vec::new();
    store.run(|round| {
        if round.number() == 1 {
            chunks = round.split_runs(&pairs, |_, chunk, seen: &mut Vec<(u32, u32)>| {
                seen.extend_from_slice(chunk)
            });
        }
    });
    assert!(chunks.len() > 1, "{} chunks", chunks.len());
    assert_eq!(chunks.concat(), pairs);
    for w in chunks.windows(2) {
        assert_ne!(w[0].last().map(|t| t.0), w[1].first().map(|t| t.0));
    }
}

/// On every core, a round runs in parallel only when its delta passes
/// [`PARALLEL_FROM`]; a small one never does (and so never waits on a
/// thread pool).
#[test]
fn small_rounds_stay_on_one_thread() {
    const P: u32 = 7;
    let mut store = Store::new();
    store.set_workers(Workers::all());
    store.extend((0..100).map(|i| (i, P, i + 1)));
    let mut parallel = Vec::new();
    store.run(|round| parallel.push(round.is_parallel()));
    assert!(parallel.iter().all(|p| !p), "{parallel:?}");
    assert_eq!(store.engaged(), 0);
    let big = PARALLEL_FROM as u32;
    store.extend((1_000..1_000 + big).map(|i| (i, P, i + 1)));
    parallel.clear();
    store.run(|round| parallel.push(round.is_parallel()));
    if Workers::all().threads() > 1 {
        assert_eq!(parallel, [true], "the one round has a delta of {big}");
    } else {
        assert_eq!(parallel, [false]);
    }
}

/// The parallel helpers give what one thread gives: sorting batches,
/// merging, and filtering out what is known, on inputs large enough to be
/// cut.
#[test]
fn parallel_helpers_match_one_thread() {
    let (one, four) = (Workers::one(), Workers::exactly(4));
    for seed in 0..4 {
        let mut rng = Rng::new(seed);
        let n = 20_000 + rng.below(60_000) as usize;
        let batches: Vec<Vec<(u32, u32)>> = (0..rng.below(6) + 1).map(|_| rng.pairs(n / 3, 5_000)).collect();
        let sorted = one.sort_dedup_batches(batches.clone());
        assert_eq!(four.sort_dedup_batches(batches), sorted, "seed {seed}");
        assert!(sorted.windows(2).all(|w| w[0] < w[1]));
        let mut other = rng.pairs(n, 5_000);
        other.sort_unstable();
        other.dedup();
        let union = one.merge(sorted.clone(), other.clone());
        assert_eq!(four.merge(sorted.clone(), other.clone()), union, "seed {seed}");
        assert_eq!(
            union,
            set(sorted.iter().chain(&other).copied())
                .into_iter()
                .collect::<Vec<_>>()
        );
        let absent = one.absent(sorted.clone(), &other);
        assert_eq!(four.absent(sorted.clone(), &other), absent, "seed {seed}");
        let known = set(other.iter().copied());
        assert_eq!(
            absent,
            sorted
                .iter()
                .copied()
                .filter(|t| !known.contains(t))
                .collect::<Vec<_>>()
        );
    }
}

/// Sorting and merging leave a relation at most an eighth of its length in
/// spare room, and a merge grows its allocation by exactly what it adds:
/// a store keeps its relations for the whole run, so their room counts
/// towards its peak.
#[test]
fn a_relation_holds_little_spare_room() {
    let within = |r: &Relation<(u32, u32)>| r.bytes() <= (r.len() + r.len() / 8) * size_of::<(u32, u32)>();
    // Many duplicates in a vector grown by pushes.
    let mut pushed = Vec::new();
    for i in 0..10_000u32 {
        pushed.push((i % 100, 0));
    }
    let deduped = Relation::from_vec(pushed);
    assert_eq!(deduped.len(), 100);
    assert!(within(&deduped), "{} bytes for {}", deduped.bytes(), deduped.len());
    // Exactly sized and disjoint: the union is exactly sized too.
    let a = Relation::from_sorted((0..1_000u32).map(|i| (2 * i, 0)).collect());
    let b = Relation::from_sorted((0..1_000u32).map(|i| (2 * i + 1, 0)).collect());
    let union = a.merge(b);
    assert_eq!(union.len(), 2_000);
    assert_eq!(
        union.bytes(),
        2_000 * size_of::<(u32, u32)>(),
        "no room for later pushes"
    );
    // Mostly the same tuples: the room of the duplicates is given back.
    let c = Relation::from_sorted((0..2_000u32).map(|i| (i, 0)).collect());
    let d = Relation::from_sorted((0..1_000u32).map(|i| (i, 0)).collect());
    let overlap = c.merge(d);
    assert_eq!(overlap.len(), 2_000);
    assert!(within(&overlap), "{} bytes for {}", overlap.bytes(), overlap.len());
    // Nothing left: no allocation.
    let mut emptied = Variable::new();
    emptied.insert(Relation::from_vec(vec![(1, 2), (3, 4)]));
    assert!(emptied.changed());
    emptied.insert(Relation::from_vec(vec![(1, 2), (3, 4)]));
    assert!(!emptied.changed());
    assert_eq!(emptied.recent().bytes(), 0);
}

/// After every step of a run, each part of each table (stable batches,
/// recent part, both indexes) holds at most an eighth of its length in
/// spare room, on one thread and on four with every round parallel, run
/// after run. The rounds derive each pair many times over, into buffers
/// that grow by doubling.
#[test]
fn a_store_holds_little_spare_room() {
    const P: u32 = 7;
    fn check(store: &Store) -> usize {
        let mut parts = 0;
        for (p, table) in store.tables() {
            let indexes = std::iter::once(table.by_subject()).chain(table.by_object());
            for variable in indexes {
                for part in variable.stable().iter().chain(Some(variable.recent())) {
                    let room = part.bytes() / size_of::<Pair>() - part.len();
                    assert!(
                        room <= part.len() / 8,
                        "predicate {p}: room for {room} more pairs in a part of {}",
                        part.len()
                    );
                    parts += 1;
                }
            }
        }
        parts
    }
    for workers in [Workers::one(), Workers::exactly(4).every_round()] {
        let mut store = Store::new();
        store.set_workers(workers);
        let mut checked = 0;
        // Two chains, the second joined to the first in a later run: the
        // closure grows to tens of thousands of pairs, large enough for
        // the parallel merges, sorts and filters.
        for (from, to) in [(0, 200), (200, 300)] {
            store.extend((from..to).map(|i| (i, P, i + 1)));
            store.run(|round| {
                checked += check(round.store());
                store_transitive(round, P);
            });
            checked += check(&store);
        }
        assert_eq!(store.len(), 300 * 301 / 2);
        assert!(checked > 20, "{checked} parts checked");
    }
}

/// `count` distinct random pairs over `0..domain`, in random order.
fn distinct_pairs(rng: &mut Rng, count: usize, domain: u32) -> Vec<(u32, u32)> {
    let mut seen = BTreeSet::new();
    let mut pairs = Vec::with_capacity(count);
    while pairs.len() < count {
        let pair = (rng.below(domain), rng.below(domain));
        if seen.insert(pair) {
            pairs.push(pair);
        }
    }
    pairs
}

/// The sizes around the two thresholds an index build meets: where a step
/// hands it the workers ([`PARALLEL_FROM`]) and where the workers cut the
/// work (`4 * GRAIN`).
fn sizes_around_the_thresholds() -> Vec<usize> {
    let inner = 4 * parallel::GRAIN;
    vec![
        0,
        1,
        2,
        inner - 1,
        inner,
        inner + 1,
        PARALLEL_FROM - 1,
        PARALLEL_FROM,
        PARALLEL_FROM + 1,
    ]
}

/// An object index built by flipping a variable's parts and sorting them in
/// place ([`store::flipped`]) is the one the step built before: the flip
/// sorted and deduplicated ([`Workers::sort_dedup`]), on one thread and on
/// four. It is strictly increasing, in a vector of exactly its length, on
/// any workers, at the thresholds and around them.
#[test]
fn an_index_sorted_in_place_matches_the_deduplicated_sort() {
    let (one, four) = (Workers::one(), Workers::exactly(4));
    let workers = [
        Workers::one(),
        Workers::exactly(4),
        Workers::all(),
        Workers::exactly(4).every_round(),
    ];
    for (seed, n) in sizes_around_the_thresholds().into_iter().enumerate() {
        let mut rng = Rng::new(seed as u64);
        // A table's pairs by subject, in a few stable batches and a recent
        // part, from a few rounds of random size.
        let pairs = distinct_pairs(&mut rng, n, 1_000);
        let mut so: Variable<Pair> = Variable::new();
        let mut rest = &pairs[..];
        while !rest.is_empty() {
            let take = (rng.below(rest.len() as u32) as usize + 1).min(rest.len());
            so.extend(rest[..take].iter().copied());
            so.changed();
            rest = &rest[take..];
        }
        assert_eq!(so.len(), n);
        let parts: Vec<&[Pair]> = so.parts().collect();
        let flip: Vec<Pair> = parts
            .iter()
            .flat_map(|part| part.iter().map(|&(s, o)| (o, s)))
            .collect();
        let before = one.sort_dedup(flip.clone());
        assert_eq!(four.sort_dedup(flip), before, "n {n}");
        assert_eq!(before.len(), n, "the flip of distinct pairs is distinct");
        for w in std::iter::once(None).chain(workers.iter().map(Some)) {
            let index = store::flipped(&parts, w);
            assert_eq!(&index[..], &before[..], "n {n}, {w:?}");
            assert!(parallel::strictly_increasing(&index), "n {n}, {w:?}");
            assert_eq!(index.bytes(), n * size_of::<Pair>(), "n {n}, {w:?}: capacity is length");
        }
        // A recent part alone, as a step's incremental index flips it.
        let recent: &[Pair] = so.recent();
        let expected = one.sort_dedup(recent.iter().map(|&(s, o)| (o, s)).collect());
        for w in workers.iter().map(Some) {
            let index = store::flipped(&[recent], w);
            assert_eq!(&index[..], &expected[..], "n {n}, {w:?}");
            assert_eq!(index.bytes(), size_of_val(recent), "n {n}, {w:?}");
        }
    }
}

/// Mapping parts into one vector gives the items in part order, in a vector
/// of exactly their length, on one thread and on four, at the threshold
/// where the workers cut the work and around it.
#[test]
fn mapping_parts_matches_one_thread() {
    let mut rng = Rng::new(11);
    for n in sizes_around_the_thresholds() {
        let items: Vec<u32> = (0..n).map(|_| rng.below(1 << 30)).collect();
        let cut = rng.below(n as u32 + 1) as usize;
        let parts: Vec<&[u32]> = vec![&items[..cut], &[], &items[cut..]];
        let expected: Vec<u64> = items.iter().map(|&x| u64::from(x) * 3).collect();
        for w in [Workers::one(), Workers::exactly(4), Workers::exactly(4).every_round()] {
            let mapped = w.map_parts(&parts, |&x| u64::from(x) * 3);
            assert_eq!(mapped, expected, "n {n}, {w:?}");
            assert_eq!(mapped.capacity(), n, "n {n}, {w:?}");
        }
    }
}

/// Sorting distinct items in place gives what sorting and deduplicating
/// gives, on any workers. Items that repeat trip the debug assertion in a
/// debug build (and only there; a release build does not check).
#[test]
fn sorting_distinct_items_checks_them_in_debug_builds() {
    let mut rng = Rng::new(12);
    for n in sizes_around_the_thresholds() {
        let items = distinct_pairs(&mut rng, n, 1_000);
        let expected = Workers::one().sort_dedup(items.clone());
        for w in [Workers::one(), Workers::exactly(4), Workers::exactly(4).every_round()] {
            let mut sorted = items.clone();
            w.sort_distinct(&mut sorted);
            assert_eq!(sorted, expected, "n {n}, {w:?}");
        }
    }
    assert!(parallel::strictly_increasing::<u32>(&[]));
    assert!(parallel::strictly_increasing(&[1, 2, 5]));
    assert!(!parallel::strictly_increasing(&[1, 2, 2]));
    assert!(!parallel::strictly_increasing(&[2, 1]));
    for w in [Workers::one(), Workers::exactly(4)] {
        let mut repeated: Vec<u32> = (0..5 * parallel::GRAIN as u32).map(|i| i / 2).collect();
        let tripped =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| w.sort_distinct(&mut repeated))).is_err();
        assert_eq!(tripped, cfg!(debug_assertions), "{w:?}");
    }
}

/// A store builds a table's object index whole when a rule first asks for
/// it, and then flips each step's new pairs into it: on one thread, on
/// four and on every core, at [`PARALLEL_FROM`] and around it, the index
/// holds the table's pairs flipped, in the same parts, and the first build
/// is a part of exactly its length.
#[test]
fn a_store_builds_its_object_index_in_place() {
    const P: u32 = 7;
    fn index(store: &Store) -> Vec<Vec<Pair>> {
        let os = store.table(P).and_then(Table::by_object).expect("an index");
        os.parts().map(<[Pair]>::to_vec).collect()
    }
    for n in [PARALLEL_FROM - 1, PARALLEL_FROM, PARALLEL_FROM + 1] {
        let mut rng = Rng::new(n as u64);
        let first = distinct_pairs(&mut rng, n, 1_000);
        // The second run: new pairs, and some known ones again.
        let mut second = distinct_pairs(&mut rng, n, 1_000);
        second.extend_from_slice(&first[..n / 8]);
        let mut expected: Option<Vec<Vec<Vec<Pair>>>> = None;
        for workers in [Workers::one(), Workers::exactly(4), Workers::all()] {
            let mut store = Store::new();
            store.set_workers(workers.clone());
            let mut got = Vec::new();
            for pairs in [&first, &second] {
                store.extend(pairs.iter().map(|&(s, o)| (s, P, o)));
                let mut built = None;
                store.run(|round| {
                    if let Some(os) = round.by_object(P)
                        && built.is_none()
                    {
                        built = Some((os.recent().len(), os.recent().bytes()));
                    }
                });
                if let Some((len, bytes)) = built
                    && got.is_empty()
                {
                    assert_eq!(len, n, "the first build holds every pair");
                    assert_eq!(bytes, n * size_of::<Pair>(), "{workers:?}: capacity is length");
                }
                let so = store.table(P).expect("a table").by_subject();
                let mut flip: Vec<Pair> = so.all().iter().map(|(s, o)| (o, s)).collect();
                flip.sort_unstable();
                let mut all: Vec<Pair> = index(&store).concat();
                all.sort_unstable();
                assert_eq!(all, flip, "n {n}, {workers:?}");
                got.push(index(&store));
            }
            match &expected {
                None => expected = Some(got),
                Some(e) => assert_eq!(&got, e, "n {n}, {workers:?}"),
            }
        }
    }
}
