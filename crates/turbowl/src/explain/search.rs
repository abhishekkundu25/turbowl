//! The backward search: the facts relevant to a goal, their ranks from the
//! base, and the proof made of minimum-rank rule instances.
//!
//! # Relevant facts
//!
//! The rule program is evaluated top down, with tables (as QSQ or magic
//! sets would): a work list of *items*, each re-evaluated whenever what it
//! read grew, to a fixpoint.
//!
//! - A **fact item** is a closure triple (or the goal finding) to explain.
//!   Each rule whose head matches it is joined over the closure with the
//!   head's terms bound, and every match is a rule instance of that fact.
//!   The closure triples a match reads are its premises: a base triple is a
//!   leaf, any other may become a fact item in turn (below).
//! - A **call** is a helper relation (`listCell`, `chainFrom`, ...) with
//!   some arguments bound. Its answers are the tuples its rules derive over
//!   the closure, with the calls their bodies make; each answer keeps the
//!   instances that derive it. A call is always evaluated to its fixpoint.
//!
//! A body is joined one element at a time: a `!=` or a test on terms as
//! soon as its terms are bound, then the atom with the most bound
//! arguments (the head's terms first: a join in written order touches
//! facts that cannot match).
//!
//! Which facts are explained is bounded by iterative deepening
//! ([`Search::prove`]): those within a distance of the goal, the distance
//! doubling until the goal's rank fits it. What a search finds stays in the
//! explainer's [`Graph`], so a fact is explained once for all the goals of
//! an explainer.
//!
//! # Ranks
//!
//! Over the instances found, a base triple has rank 0, and an instance of a
//! closure triple or a finding has rank one more than its highest premise;
//! an instance of a helper relation has its highest premise's rank, so a
//! rank counts proof steps, which helpers are not. Knuth's generalization
//! of Dijkstra's algorithm gives each fact its least rank and an instance
//! that has it, whose premises were all ranked before it: the instances
//! kept form an acyclic graph grounded in the base. The rank of a closure
//! triple is the depth of its shallowest proof.
//!
//! # The proof
//!
//! From the goal, each closure triple's kept instance is a step. A helper
//! premise is folded into the step that reads it: replaced by the premises
//! of its own kept instance, down to closure triples, so a step's premises
//! are triples, list cells included. Each premise ranks below the step, so
//! the steps are acyclic. A triple used twice is one step.

use super::program::{Arg, Body, Builtin, Compiled, Kind, MAX_ARITY, NONE, RelId, T};
use crate::Reasoner;
use crate::rules::datatype::map::{Datatype, Reading, value_of};
use crate::term::{Dictionary, Term, TermId};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

/// A tuple of a relation, padded with zeros past its arity.
pub(crate) type Tuple = [u32; MAX_ARITY];

/// A call's arguments: bound or not.
type Pattern = [Option<u32>; MAX_ARITY];

/// The closure, sorted three ways, so that every pattern is a range.
#[derive(Debug, Default)]
pub(crate) struct Facts {
    spo: Vec<[u32; 3]>,
    /// `[p, o, s]`.
    pos: Vec<[u32; 3]>,
    /// `[o, s, p]`.
    osp: Vec<[u32; 3]>,
}

/// The triples of `sorted` that start with `prefix`.
fn range<'a>(sorted: &'a [[u32; 3]], prefix: &[u32]) -> &'a [[u32; 3]] {
    let n = prefix.len();
    let from = sorted.partition_point(|t| t[..n] < *prefix);
    let to = from + sorted[from..].partition_point(|t| t[..n] == *prefix);
    &sorted[from..to]
}

impl Facts {
    pub(crate) fn new(mut triples: Vec<[u32; 3]>) -> Self {
        triples.sort_unstable();
        triples.dedup();
        let mut pos: Vec<[u32; 3]> = triples.iter().map(|&[s, p, o]| [p, o, s]).collect();
        pos.sort_unstable();
        let mut osp: Vec<[u32; 3]> = triples.iter().map(|&[s, p, o]| [o, s, p]).collect();
        osp.sort_unstable();
        Self { spo: triples, pos, osp }
    }

    pub(crate) fn contains(&self, t: [u32; 3]) -> bool {
        self.spo.binary_search(&t).is_ok()
    }

    /// The bytes the three orders hold.
    pub(crate) fn bytes(&self) -> usize {
        3 * self.spo.capacity() * std::mem::size_of::<[u32; 3]>()
    }

    /// Each triple that matches the pattern (`None`: any term), as `[s, p, o]`.
    fn each(&self, s: Option<u32>, p: Option<u32>, o: Option<u32>, mut f: impl FnMut([u32; 3])) {
        match (s, p, o) {
            (Some(s), Some(p), Some(o)) => {
                if self.contains([s, p, o]) {
                    f([s, p, o]);
                }
            }
            (Some(s), Some(p), None) => range(&self.spo, &[s, p]).iter().for_each(|&t| f(t)),
            (Some(s), None, None) => range(&self.spo, &[s]).iter().for_each(|&t| f(t)),
            (Some(s), None, Some(o)) => range(&self.osp, &[o, s]).iter().for_each(|t| f([s, t[2], o])),
            (None, Some(p), Some(o)) => range(&self.pos, &[p, o]).iter().for_each(|t| f([t[2], p, o])),
            (None, Some(p), None) => range(&self.pos, &[p]).iter().for_each(|t| f([t[2], p, t[1]])),
            (None, None, Some(o)) => range(&self.osp, &[o]).iter().for_each(|t| f([t[1], t[2], o])),
            (None, None, None) => self.spo.iter().for_each(|&t| f(t)),
        }
    }
}

/// What the search reads: the program of the configuration, the closure,
/// the dictionary, and the base.
#[derive(Clone, Copy)]
pub(crate) struct World<'a> {
    pub(crate) program: &'a Compiled,
    pub(crate) facts: &'a Facts,
    pub(crate) reasoner: &'a Reasoner,
    /// The literals the rules name (`datatype::RULE_LITERALS`).
    pub(crate) rule_literals: &'a [u32],
}

impl World<'_> {
    fn terms(&self) -> &Dictionary {
        self.reasoner.terms()
    }

    /// The term of `id`, or `None` for an id the dictionary does not hold
    /// (a constant it lacks).
    fn term(&self, id: u32) -> Option<Term<'_>> {
        ((id as usize) < self.terms().len()).then(|| self.terms().term(TermId(id)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Item {
    Fact(u32),
    Call(u32),
}

#[derive(Debug)]
struct Node {
    rel: RelId,
    tuple: Tuple,
    /// A base triple: a leaf.
    base: bool,
    /// A step of a proof: a closure triple or a finding (not a helper).
    step: bool,
    /// Explained: every rule instance of it is in the graph (a helper's are
    /// once its call is).
    expanded: bool,
}

#[derive(Debug)]
struct Call {
    rel: RelId,
    pattern: Pattern,
    answers: Vec<u32>,
    answered: FxHashSet<u32>,
    readers: Vec<Item>,
    read_by: FxHashSet<Item>,
}

#[derive(Debug)]
struct Instance {
    rule: usize,
    head: u32,
    premises: Vec<u32>,
}

/// What the searches of one explainer found: the facts, the calls, and
/// the rule instances. The instances of a fact do not depend on the goal
/// that led to it, so each fact is explained once per explainer, and a
/// later goal reads what an earlier one explained (kept in the
/// [`ExplainCache`](super::ExplainCache)).
#[derive(Debug, Default)]
pub(crate) struct Graph {
    nodes: Vec<Node>,
    node_ids: FxHashMap<(RelId, Tuple), u32>,
    calls: Vec<Call>,
    call_ids: FxHashMap<(RelId, Pattern), u32>,
    instances: Vec<Instance>,
    seen: FxHashSet<(usize, u32, Vec<u32>)>,
    /// The instances of each node.
    of: Vec<Vec<usize>>,
    /// The ranks of the graph as it was, by (nodes, instances) then.
    ranks: Option<((usize, usize), Ranks)>,
}

impl Graph {
    /// The nodes and instances held.
    pub(crate) fn len(&self) -> (usize, usize) {
        (self.nodes.len(), self.instances.len())
    }
}

/// Each node's least rank, and the instance that gives it (`None` for a
/// base triple).
#[derive(Debug)]
struct Ranks {
    rank: Vec<Option<u32>>,
    kept: Vec<Option<usize>>,
}

/// One search, from one goal, over an explainer's graph.
pub(crate) struct Search<'a> {
    world: World<'a>,
    g: &'a mut Graph,
    queue: VecDeque<Item>,
    queued: FxHashSet<Item>,
}

/// A step of a proof in term ids: a closure triple, its rule (`None`: a
/// base triple), and its premises, earlier steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawStep {
    pub(crate) triple: [u32; 3],
    pub(crate) rule: Option<&'static str>,
    pub(crate) premises: Vec<usize>,
}

/// A proof in term ids: its steps in topological order, and for a finding,
/// the rule and premises of the root (the goal is otherwise the last step).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Built {
    pub(crate) steps: Vec<RawStep>,
    pub(crate) root: Option<(&'static str, Vec<usize>)>,
}

impl<'a> Search<'a> {
    pub(crate) fn new(world: World<'a>, g: &'a mut Graph) -> Self {
        Self {
            world,
            g,
            queue: VecDeque::new(),
            queued: FxHashSet::default(),
        }
    }

    /// Explains the fact `tuple` of `rel` (a closure triple the closure
    /// holds, or a reported finding): its proof, or `None` when the rules
    /// give it none.
    ///
    /// By iterative deepening: every fact within distance `bound` of the
    /// goal is explained (all its instances found), which holds every proof
    /// of depth `bound + 1` or less, so a goal whose rank is at most that
    /// has its shallowest proof. Otherwise the bound grows (1, 3, 7, ...),
    /// until the goal ranks within it or every fact it reaches is explained.
    pub(crate) fn prove(mut self, rel: RelId, tuple: Tuple) -> Option<Built> {
        let goal = self.node(rel, tuple);
        let mut bound = 0;
        loop {
            loop {
                let todo = self.frontier(goal, bound);
                if todo.is_empty() {
                    break;
                }
                for n in todo {
                    self.g.nodes[n as usize].expanded = true;
                    self.enqueue(Item::Fact(n));
                }
                self.run();
            }
            let ranks = self.ranks();
            let rank = ranks.rank[goal as usize];
            if rank.is_some_and(|r| r <= bound + 1) || self.frontier(goal, u32::MAX).is_empty() {
                rank?;
                return Some(self.build(goal));
            }
            bound = 2 * bound + 1;
        }
    }

    /// The node of a fact, new or not.
    fn node(&mut self, rel: RelId, tuple: Tuple) -> u32 {
        if let Some(&n) = self.g.node_ids.get(&(rel, tuple)) {
            return n;
        }
        let n = self.g.nodes.len() as u32;
        let program = self.world.program;
        let step = rel == T || program.findings.iter().any(|&(_, r)| r == rel);
        let base = rel == T && self.world.reasoner.is_base(&[tuple[0], tuple[1], tuple[2]]);
        self.g.nodes.push(Node {
            rel,
            tuple,
            base,
            step,
            expanded: base,
        });
        self.g.of.push(Vec::new());
        self.g.node_ids.insert((rel, tuple), n);
        n
    }

    /// The steps within `bound` of the goal not explained yet. A premise
    /// that is a step is one further than the step that reads it; a helper
    /// is as far, and the premises of its instances are as its reader's
    /// (0-1 breadth first).
    fn frontier(&self, goal: u32, bound: u32) -> Vec<u32> {
        let mut dist: FxHashMap<u32, u32> = FxHashMap::default();
        let mut queue = VecDeque::from([(goal, 0u32)]);
        dist.insert(goal, 0);
        let mut todo = Vec::new();
        while let Some((n, k)) = queue.pop_front() {
            if dist[&n] < k {
                continue;
            }
            let node = &self.g.nodes[n as usize];
            if node.step && !node.expanded {
                todo.push(n);
                continue;
            }
            if node.step && k >= bound {
                continue;
            }
            for &i in &self.g.of[n as usize] {
                for &p in &self.g.instances[i].premises {
                    let w = u32::from(self.g.nodes[p as usize].step);
                    let d = k + w;
                    if dist.get(&p).is_none_or(|&old| d < old) {
                        dist.insert(p, d);
                        if w == 0 {
                            queue.push_front((p, d));
                        } else {
                            queue.push_back((p, d));
                        }
                    }
                }
            }
        }
        todo.sort_unstable();
        todo.dedup();
        todo
    }

    fn enqueue(&mut self, item: Item) {
        if self.queued.insert(item) {
            self.queue.push_back(item);
        }
    }

    fn run(&mut self) {
        while let Some(item) = self.queue.pop_front() {
            self.queued.remove(&item);
            self.evaluate(item);
        }
    }

    /// Joins the body of every rule whose head matches the item.
    fn evaluate(&mut self, item: Item) {
        let (rel, bound): (RelId, Pattern) = match item {
            Item::Fact(n) => {
                let node = &self.g.nodes[n as usize];
                (node.rel, node.tuple.map(Some))
            }
            Item::Call(c) => (self.g.calls[c as usize].rel, self.g.calls[c as usize].pattern),
        };
        let program = self.world.program;
        for &r in &program.by_head[rel as usize] {
            let rule = &program.rules[r];
            let mut b = vec![None; rule.vars];
            let arity = program.relations[rel as usize].arity;
            let unified = rule
                .head
                .args
                .iter()
                .zip(&bound[..arity])
                .all(|(arg, x)| match (arg, x) {
                    (_, None) => true,
                    (Arg::Id(c), Some(x)) => c == x,
                    (Arg::Var(v), Some(x)) => match b[*v as usize] {
                        Some(y) => y == *x,
                        None => {
                            b[*v as usize] = Some(*x);
                            true
                        }
                    },
                });
            if unified {
                self.solve(r, item, &mut b, 0, &mut Vec::new());
            }
        }
    }

    /// The next body element of rule `r` to join, given what is bound and
    /// done: a filter as soon as it can run, then the atom with the most
    /// bound arguments (a triple before a helper).
    fn choose(&self, r: usize, b: &[Option<u32>], done: u64) -> Option<usize> {
        let program = self.world.program;
        let rule = &program.rules[r];
        let bound = |a: &Arg| match a {
            Arg::Id(_) => true,
            Arg::Var(v) => b[*v as usize].is_some(),
        };
        let mut best: Option<(usize, usize)> = None;
        let mut left = false;
        for (k, element) in rule.body.iter().enumerate() {
            if done & (1 << k) != 0 {
                continue;
            }
            left = true;
            match element {
                Body::NotEqual(x, y) => {
                    if bound(x) && bound(y) {
                        return Some(k);
                    }
                }
                Body::Atom(a) => match program.relations[a.rel as usize].kind {
                    Kind::Builtin(f) => {
                        let ready = match f {
                            Builtin::RuleLiteral => true,
                            Builtin::TypesOf | Builtin::IllTyped => bound(&a.args[0]),
                            Builtin::OutsideValueSpace => bound(&a.args[0]) && bound(&a.args[1]),
                        };
                        if ready {
                            return Some(k);
                        }
                    }
                    kind => {
                        let score = 2 * a.args.iter().filter(|a| bound(a)).count() + usize::from(kind == Kind::Triple);
                        if best.is_none_or(|(s, _)| score > s) {
                            best = Some((score, k));
                        }
                    }
                },
            }
        }
        match best {
            Some((_, k)) => Some(k),
            None if left => panic!(
                "the rule {:?} has a filter whose terms no atom binds",
                rule.name.unwrap_or("(a helper)")
            ),
            None => None,
        }
    }

    /// Joins what is left of rule `r`'s body; each match is an instance.
    fn solve(&mut self, r: usize, item: Item, b: &mut Vec<Option<u32>>, done: u64, premises: &mut Vec<u32>) {
        let world = self.world;
        let program = world.program;
        let rule = &program.rules[r];
        let Some(k) = self.choose(r, b, done) else {
            self.conclude(r, item, b, premises);
            return;
        };
        let done = done | (1 << k);
        let value = |b: &[Option<u32>], a: &Arg| match a {
            Arg::Id(c) => Some(*c),
            Arg::Var(v) => b[*v as usize],
        };
        match &rule.body[k] {
            Body::NotEqual(x, y) => {
                if value(b, x) != value(b, y) {
                    self.solve(r, item, b, done, premises);
                }
            }
            Body::Atom(atom) => {
                let pattern: Vec<Option<u32>> = atom.args.iter().map(|a| value(b, a)).collect();
                match program.relations[atom.rel as usize].kind {
                    Kind::Triple => {
                        world.facts.each(pattern[0], pattern[1], pattern[2], |t| {
                            if let Some(new) = bind(&atom.args, &t, b) {
                                let n = self.node(T, [t[0], t[1], t[2], 0]);
                                premises.push(n);
                                self.solve(r, item, b, done, premises);
                                premises.pop();
                                unbind(new, b);
                            }
                        });
                    }
                    Kind::Builtin(f) => {
                        for t in self.test(f, &pattern) {
                            if let Some(new) = bind(&atom.args, &t, b) {
                                self.solve(r, item, b, done, premises);
                                unbind(new, b);
                            }
                        }
                    }
                    Kind::Derived => {
                        let mut call_pattern: Pattern = [None; MAX_ARITY];
                        call_pattern[..pattern.len()].copy_from_slice(&pattern);
                        let c = self.call(atom.rel, call_pattern, item);
                        let mut j = 0;
                        while j < self.g.calls[c as usize].answers.len() {
                            let n = self.g.calls[c as usize].answers[j];
                            let t = self.g.nodes[n as usize].tuple;
                            if let Some(new) = bind(&atom.args, &t, b) {
                                premises.push(n);
                                self.solve(r, item, b, done, premises);
                                premises.pop();
                                unbind(new, b);
                            }
                            j += 1;
                        }
                    }
                }
            }
        }
    }

    /// The tuples a test on terms holds of, given the bound arguments.
    fn test(&self, f: Builtin, pattern: &[Option<u32>]) -> Vec<Tuple> {
        let world = self.world;
        let literal = |id: u32| match world.term(id) {
            Some(term @ Term::Literal { .. }) => Some(term),
            _ => None,
        };
        let mut out = Vec::new();
        match f {
            Builtin::RuleLiteral => {
                for &x in world.rule_literals {
                    if pattern[0].is_none_or(|y| y == x) {
                        out.push([x, 0, 0, 0]);
                    }
                }
            }
            Builtin::TypesOf => {
                let lt = pattern[0].expect("bound");
                if let Some(term) = literal(lt)
                    && let Reading::Value(v) = value_of(term)
                {
                    for dt in v.datatypes() {
                        if pattern[1].is_none_or(|d| d == dt.id().0) {
                            out.push([lt, dt.id().0, 0, 0]);
                        }
                    }
                }
            }
            Builtin::IllTyped => {
                let lt = pattern[0].expect("bound");
                if let Some(term @ Term::Literal { datatype, .. }) = literal(lt)
                    && matches!(value_of(term), Reading::IllTyped)
                    && pattern[1].is_none_or(|d| d == datatype.0)
                {
                    out.push([lt, datatype.0, 0, 0]);
                }
            }
            Builtin::OutsideValueSpace => {
                let (lt, dt) = (pattern[0].expect("bound"), pattern[1].expect("bound"));
                let checked = (dt != NONE)
                    .then(|| Datatype::of(TermId(dt)))
                    .flatten()
                    .filter(|d| Datatype::OWL_2_RL.contains(d));
                if let (Some(term), Some(d)) = (literal(lt), checked) {
                    let outside = match value_of(term) {
                        Reading::IllTyped => true,
                        Reading::Value(v) => !d.holds(&v),
                        Reading::Unknown => false,
                    };
                    if outside {
                        out.push([lt, dt, 0, 0]);
                    }
                }
            }
        }
        out
    }

    /// The call of `rel` with `pattern`, read by `reader`, which is
    /// evaluated again when the call gains an answer.
    fn call(&mut self, rel: RelId, pattern: Pattern, reader: Item) -> u32 {
        let c = match self.g.call_ids.get(&(rel, pattern)) {
            Some(&c) => c,
            None => {
                let c = self.g.calls.len() as u32;
                self.g.calls.push(Call {
                    rel,
                    pattern,
                    answers: Vec::new(),
                    answered: FxHashSet::default(),
                    readers: Vec::new(),
                    read_by: FxHashSet::default(),
                });
                self.g.call_ids.insert((rel, pattern), c);
                self.enqueue(Item::Call(c));
                c
            }
        };
        let call = &mut self.g.calls[c as usize];
        if call.read_by.insert(reader) {
            call.readers.push(reader);
        }
        c
    }

    /// A match of rule `r`'s whole body: an instance of the item's fact, or
    /// of an answer of the item's call.
    fn conclude(&mut self, r: usize, item: Item, b: &[Option<u32>], premises: &[u32]) {
        let program = self.world.program;
        let rule = &program.rules[r];
        let mut tuple: Tuple = [0; MAX_ARITY];
        for (i, a) in rule.head.args.iter().enumerate() {
            tuple[i] = match a {
                Arg::Id(c) => *c,
                Arg::Var(v) => b[*v as usize].expect("the head's variables are bound by the body"),
            };
        }
        let head = match item {
            Item::Fact(n) => n,
            Item::Call(c) => {
                let n = self.node(rule.head.rel, tuple);
                let call = &mut self.g.calls[c as usize];
                if call.answered.insert(n) {
                    call.answers.push(n);
                    let readers = call.readers.clone();
                    for reader in readers {
                        self.enqueue(reader);
                    }
                }
                n
            }
        };
        if self.g.seen.insert((r, head, premises.to_vec())) {
            self.g.of[head as usize].push(self.g.instances.len());
            self.g.instances.push(Instance {
                rule: r,
                head,
                premises: premises.to_vec(),
            });
        }
    }

    /// Each node's least rank and the instance that gives it, by Knuth's
    /// algorithm, computed again only when the graph grew.
    fn ranks(&mut self) -> &Ranks {
        let key = self.g.len();
        if self.g.ranks.as_ref().is_none_or(|(at, _)| *at != key) {
            let ranks = self.ranked();
            self.g.ranks = Some((key, ranks));
        }
        &self.g.ranks.as_ref().expect("ranked").1
    }

    /// The ranks of the graph, computed.
    fn ranked(&self) -> Ranks {
        let step = |n: u32| self.g.nodes[n as usize].step;
        let n = self.g.nodes.len();
        let mut rank: Vec<Option<u32>> = vec![None; n];
        let mut kept: Vec<Option<usize>> = vec![None; n];
        let mut uses: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut waiting: Vec<usize> = Vec::with_capacity(self.g.instances.len());
        let mut heap: BinaryHeap<Reverse<(u32, u32, usize)>> = BinaryHeap::new();
        const LEAF: usize = usize::MAX;
        for (i, inst) in self.g.instances.iter().enumerate() {
            let mut distinct = inst.premises.clone();
            distinct.sort_unstable();
            distinct.dedup();
            for &p in &distinct {
                uses[p as usize].push(i);
            }
            waiting.push(distinct.len());
            if distinct.is_empty() {
                heap.push(Reverse((u32::from(step(inst.head)), inst.head, i)));
            }
        }
        for (i, node) in self.g.nodes.iter().enumerate() {
            if node.base {
                heap.push(Reverse((0, i as u32, LEAF)));
            }
        }
        while let Some(Reverse((r, node, inst))) = heap.pop() {
            if rank[node as usize].is_some() {
                continue;
            }
            rank[node as usize] = Some(r);
            kept[node as usize] = (inst != LEAF).then_some(inst);
            for &i in &uses[node as usize] {
                waiting[i] -= 1;
                if waiting[i] == 0 {
                    let inst = &self.g.instances[i];
                    let highest = inst.premises.iter().map(|&p| rank[p as usize].expect("ranked")).max();
                    let r = highest.unwrap_or(0) + u32::from(step(inst.head));
                    heap.push(Reverse((r, inst.head, i)));
                }
            }
        }
        Ranks { rank, kept }
    }

    /// The closure triples instance `i`'s premises stand for: its triples,
    /// and for each helper, the triples of its kept instance, recursively,
    /// each once, in order.
    fn folded(&self, i: usize, kept: &[Option<usize>], memo: &mut FxHashMap<u32, Vec<u32>>) -> Vec<u32> {
        let mut out = Vec::new();
        let mut seen = FxHashSet::default();
        for &p in &self.g.instances[i].premises {
            if self.g.nodes[p as usize].rel == T {
                if seen.insert(p) {
                    out.push(p);
                }
                continue;
            }
            for t in self.helper(p, kept, memo) {
                if seen.insert(t) {
                    out.push(t);
                }
            }
        }
        out
    }

    /// The closure triples a helper fact's kept instance stands for, by
    /// post-order over the helpers (a long list's cells are a long chain).
    fn helper(&self, h: u32, kept: &[Option<usize>], memo: &mut FxHashMap<u32, Vec<u32>>) -> Vec<u32> {
        let mut stack = vec![(h, false)];
        while let Some((x, expanded)) = stack.pop() {
            if memo.contains_key(&x) {
                continue;
            }
            let i = kept[x as usize].expect("a premise is ranked");
            let premises = &self.g.instances[i].premises;
            if !expanded {
                stack.push((x, true));
                for &p in premises {
                    if self.g.nodes[p as usize].rel != T && !memo.contains_key(&p) {
                        stack.push((p, false));
                    }
                }
                continue;
            }
            let mut out = Vec::new();
            let mut seen = FxHashSet::default();
            for &p in premises {
                let part: Vec<u32> = if self.g.nodes[p as usize].rel == T {
                    vec![p]
                } else {
                    memo[&p].clone()
                };
                for t in part {
                    if seen.insert(t) {
                        out.push(t);
                    }
                }
            }
            memo.insert(x, out);
        }
        memo[&h].clone()
    }

    /// The proof of `goal` from the kept instances: the closure triples in
    /// post-order (each premise before the steps that read it), the goal
    /// last, or the finding's root apart.
    fn build(&self, goal: u32) -> Built {
        let kept = &self.g.ranks.as_ref().expect("ranked").1.kept[..];
        let mut memo = FxHashMap::default();
        let mut index: FxHashMap<u32, usize> = FxHashMap::default();
        let mut steps: Vec<RawStep> = Vec::new();
        let mut children: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
        let finding = self.g.nodes[goal as usize].rel != T;
        let roots: Vec<u32> = if finding {
            self.folded(kept[goal as usize].expect("ranked"), kept, &mut memo)
        } else {
            vec![goal]
        };
        for &root in &roots {
            let mut stack = vec![(root, false)];
            while let Some((x, expanded)) = stack.pop() {
                if index.contains_key(&x) {
                    continue;
                }
                let premises = children
                    .entry(x)
                    .or_insert_with(|| match kept[x as usize] {
                        Some(i) => self.folded(i, kept, &mut memo),
                        None => Vec::new(),
                    })
                    .clone();
                if !expanded {
                    stack.push((x, true));
                    stack.extend(
                        premises
                            .iter()
                            .rev()
                            .filter(|p| !index.contains_key(p))
                            .map(|&p| (p, false)),
                    );
                    continue;
                }
                let t = self.g.nodes[x as usize].tuple;
                let rule = kept[x as usize].map(|i| {
                    self.world.program.rules[self.g.instances[i].rule]
                        .name
                        .expect("a rule of a triple has a name")
                });
                index.insert(x, steps.len());
                steps.push(RawStep {
                    triple: [t[0], t[1], t[2]],
                    rule,
                    premises: premises.iter().map(|p| index[p]).collect(),
                });
            }
        }
        let root = finding.then(|| {
            let i = kept[goal as usize].expect("ranked");
            let rule = self.world.program.rules[self.g.instances[i].rule]
                .name
                .expect("a finding's rule has a name");
            (rule, roots.iter().map(|p| index[p]).collect())
        });
        Built { steps, root }
    }
}

/// Binds the variables of `args` to the terms of `t` where they agree with
/// `b`: the variables newly bound, as bits, or `None` on a mismatch.
fn bind(args: &[Arg], t: &[u32], b: &mut [Option<u32>]) -> Option<u64> {
    let mut new = 0u64;
    for (a, &x) in args.iter().zip(t) {
        let ok = match a {
            Arg::Id(c) => *c == x,
            Arg::Var(v) => match b[*v as usize] {
                Some(y) => y == x,
                None => {
                    b[*v as usize] = Some(x);
                    new |= 1 << v;
                    true
                }
            },
        };
        if !ok {
            unbind(new, b);
            return None;
        }
    }
    Some(new)
}

fn unbind(mut new: u64, b: &mut [Option<u32>]) {
    while new != 0 {
        b[new.trailing_zeros() as usize] = None;
        new &= new - 1;
    }
}
