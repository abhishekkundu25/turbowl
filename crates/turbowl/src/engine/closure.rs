//! The transitive closure of a relation of pairs, in work proportional to
//! its input and output (ticket 25).
//!
//! A binary join closes a relation one middle term at a time: on a strongly
//! connected component of k nodes it derives each of the k² pairs once per
//! middle node, k³ derivations for a quadratic result (Hu, Motik and
//! Horrocks, AAAI 2019, §1: "O(n³) rule applications, whereas a custom
//! algorithm can achieve the same goal using only O(n²) steps"). Here, as
//! in Inferray (Subercaze et al., PVLDB 2016, §4.1) and Nuutila's
//! algorithm:
//!
//! 1. Tarjan's algorithm finds the strongly connected components. It
//!    finishes a component after every component it reaches, so their
//!    numbers are a reverse topological order.
//! 2. In that order, each component's reachable set (of components) is the
//!    union of its successors' sets and the successors themselves. A
//!    successor already in the set brings nothing: the successors are
//!    visited nearest first (highest number), so on a closed relation only
//!    the edges of its transitive reduction cost a union.
//! 3. Each node of a component reaches every node of the components in that
//!    set, and those of its own component when the component is a cycle
//!    (two nodes or more, or one with a pair to itself).
//!
//! Only pairs not in the input are reported, each once, merged against the
//! node's own successors.

use super::store::Pair;
use super::variable::View;

const NONE: u32 = u32::MAX;

/// Calls `f(x, z)` once for each pair of the transitive closure of `pairs`
/// that `pairs` does not hold.
pub(crate) fn transitive_closure(pairs: &View<'_, Pair>, mut f: impl FnMut(u32, u32)) {
    if pairs.is_empty() {
        return;
    }
    // Dense node numbers, in the order of the term ids.
    let mut ids: Vec<u32> = pairs.iter().flat_map(|(s, o)| [s, o]).collect();
    ids.sort_unstable();
    ids.dedup();
    let dense = |t: u32| ids.binary_search(&t).expect("a node of the relation") as u32;
    let (succ_start, succ) = {
        let mut edges: Vec<Pair> = pairs.iter().map(|(s, o)| (dense(s), dense(o))).collect();
        edges.sort_unstable();
        edges.dedup();
        let mut start = vec![0u32; ids.len() + 1];
        for &(s, _) in &edges {
            start[s as usize + 1] += 1;
        }
        for i in 1..start.len() {
            start[i] += start[i - 1];
        }
        (start, edges.into_iter().map(|e| e.1).collect::<Vec<u32>>())
    };
    let out = |v: u32| &succ[succ_start[v as usize] as usize..succ_start[v as usize + 1] as usize];

    let comp = components(ids.len(), &out);
    let count = comp.iter().map(|&c| c + 1).max().unwrap_or(0) as usize;
    // The members of each component, by component.
    let mut member_start = vec![0u32; count + 1];
    for &c in &comp {
        member_start[c as usize + 1] += 1;
    }
    for i in 1..member_start.len() {
        member_start[i] += member_start[i - 1];
    }
    let mut members = vec![0u32; ids.len()];
    let mut fill = member_start.clone();
    for (v, &c) in comp.iter().enumerate() {
        members[fill[c as usize] as usize] = v as u32;
        fill[c as usize] += 1;
    }
    let members_of = |c: u32| &members[member_start[c as usize] as usize..member_start[c as usize + 1] as usize];

    // Reachable components, by component, in reverse topological order.
    let mut reach_start = vec![0usize; count + 1];
    let mut reach: Vec<u32> = Vec::new();
    let mut mark = vec![NONE; count];
    let mut next: Vec<u32> = Vec::new();
    let mut targets: Vec<u32> = Vec::new();
    for c in 0..count as u32 {
        let start = reach.len();
        let mut cyclic = false;
        next.clear();
        for &v in members_of(c) {
            for &w in out(v) {
                let d = comp[w as usize];
                if d == c {
                    cyclic = true;
                } else {
                    next.push(d);
                }
            }
        }
        next.sort_unstable_by(|a, b| b.cmp(a));
        next.dedup();
        for &d in &next {
            if mark[d as usize] == c {
                continue;
            }
            mark[d as usize] = c;
            reach.push(d);
            for i in reach_start[d as usize]..reach_start[d as usize + 1] {
                let e = reach[i];
                if mark[e as usize] != c {
                    mark[e as usize] = c;
                    reach.push(e);
                }
            }
        }
        reach_start[c as usize + 1] = reach.len();

        targets.clear();
        for &d in &reach[start..] {
            targets.extend_from_slice(members_of(d));
        }
        if cyclic {
            targets.extend_from_slice(members_of(c));
        }
        targets.sort_unstable();
        for &x in members_of(c) {
            let mut known = out(x);
            for &z in &targets {
                while known.first().is_some_and(|&k| k < z) {
                    known = &known[1..];
                }
                if known.first() != Some(&z) {
                    f(ids[x as usize], ids[z as usize]);
                }
            }
        }
    }
}

/// Tarjan's strongly connected components of the graph on `n` nodes whose
/// successors `out` gives, iteratively: each node's component number, the
/// components numbered as they finish (a component after those it reaches).
fn components<'a>(n: usize, out: &impl Fn(u32) -> &'a [u32]) -> Vec<u32> {
    let mut index = vec![NONE; n];
    let mut low = vec![0u32; n];
    let mut comp = vec![NONE; n];
    let mut stack: Vec<u32> = Vec::new();
    // The depth-first path: a node and the position of its next successor.
    let mut path: Vec<(u32, usize)> = Vec::new();
    let (mut counter, mut finished) = (0u32, 0u32);
    for root in 0..n as u32 {
        if index[root as usize] != NONE {
            continue;
        }
        index[root as usize] = counter;
        low[root as usize] = counter;
        counter += 1;
        stack.push(root);
        path.push((root, 0));
        while let Some(&mut (v, ref mut next)) = path.last_mut() {
            let successors = out(v);
            if let Some(&w) = successors.get(*next) {
                *next += 1;
                if index[w as usize] == NONE {
                    index[w as usize] = counter;
                    low[w as usize] = counter;
                    counter += 1;
                    stack.push(w);
                    path.push((w, 0));
                } else if comp[w as usize] == NONE {
                    low[v as usize] = low[v as usize].min(index[w as usize]);
                }
                continue;
            }
            path.pop();
            if let Some(&(parent, _)) = path.last() {
                low[parent as usize] = low[parent as usize].min(low[v as usize]);
            }
            if low[v as usize] == index[v as usize] {
                while let Some(w) = stack.pop() {
                    comp[w as usize] = finished;
                    if w == v {
                        break;
                    }
                }
                finished += 1;
            }
        }
    }
    comp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Variable;
    use std::collections::BTreeSet;

    /// The closure by repeated joins, to the fixpoint: the definition.
    fn naive(pairs: &[Pair]) -> BTreeSet<Pair> {
        let mut all: BTreeSet<Pair> = pairs.iter().copied().collect();
        loop {
            let more: Vec<Pair> = all
                .iter()
                .flat_map(|&(x, y)| all.range((y, 0)..=(y, u32::MAX)).map(move |&(_, z)| (x, z)))
                .filter(|p| !all.contains(p))
                .collect();
            if more.is_empty() {
                return all;
            }
            all.extend(more);
        }
    }

    fn added(parts: &[&[Pair]]) -> Vec<Pair> {
        // A variable with one tier per part (the last one recent).
        let mut v = Variable::new();
        for p in parts {
            v.extend(p.iter().copied());
            v.changed();
        }
        let mut out = Vec::new();
        transitive_closure(&v.all(), |x, z| out.push((x, z)));
        out
    }

    fn check(parts: &[&[Pair]]) {
        let input: Vec<Pair> = parts.iter().flat_map(|p| p.iter().copied()).collect();
        let got = added(parts);
        let unique: BTreeSet<Pair> = got.iter().copied().collect();
        assert_eq!(unique.len(), got.len(), "each pair once: {got:?}");
        let expected: BTreeSet<Pair> = naive(&input).into_iter().filter(|p| !input.contains(p)).collect();
        assert_eq!(unique, expected, "input {input:?}");
    }

    #[test]
    fn closes_chains_cycles_and_dags() {
        check(&[]);
        check(&[&[(1, 2)]]);
        check(&[&[(1, 2), (2, 3), (3, 4)]]);
        // A cycle reaches itself, each node included.
        check(&[&[(1, 2), (2, 1)]]);
        check(&[&[(5, 5)], &[(5, 6)]]);
        check(&[&[(1, 2), (2, 3), (3, 1), (3, 4), (4, 5), (5, 4), (9, 1)]]);
        // A diamond, and a closed chain: only the reduction's edges count.
        check(&[&[(1, 2), (1, 3), (2, 4), (3, 4), (4, 5)]]);
        check(&[&[(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)], &[(4, 7)]]);
        // Ids far apart, and a pair split across parts.
        check(&[&[(4_000_000_000, 7)], &[(7, 4_000_000_000), (7, 12)]]);
    }

    #[test]
    fn closes_random_relations() {
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut next = |m: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % m) as u32
        };
        for case in 0..300u64 {
            let nodes = 2 + case % 12;
            let n = next(3 * nodes) as usize;
            let pairs: Vec<Pair> = (0..n).map(|_| (next(nodes), next(nodes))).collect();
            let half = pairs.len() / 2;
            check(&[&pairs[..half], &pairs[half..]]);
        }
    }
}
