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
