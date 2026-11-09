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
//! facts that cannot match). A fact (or a call that binds its whole head)
//! with an instance whose premises are all base triples has the least rank
//! a derived fact can have, so its other instances are not looked for.
//!
//! Which facts are explained is bounded by iterative deepening
//! ([`Search::prove`]): those within a distance of the goal, the distance
//! doubling until the goal's rank fits it. What a search finds stays in the
//! explainer's [`Graph`], so a fact is explained once for all the goals of
//! an explainer.
//!
//! # Equality
//!
//! The closure is read in the terms the user wrote (`closure.rs`): a fact
//! about terms `owl:sameAs` made equal holds of every member of their
//! classes. Run as written, eq-trans and eq-rep-* have an instance for
//! every member of a class, and a goal deep in a class of n members reaches
//! n³ of them. So the search writes the equality steps itself
//! ([`Search::equality`]), in the user's terms and with the eq-* rules'
//! names, and a join gives a free variable only what is stated
//! ([`Free::Stated`]): the representative of the class matched, and the
//! members base triples of the same image name there.
//!
//! - **Links**: a class's base `owl:sameAs` triples (either way round),
//!   dt-eq's pairs of equal literals, and the pairs a rule other than the
//!   eq-* ones concludes (prp-fp, prp-key, cls-maxc2, ...), found once per
//!   class ([`Search::links`]): by joining those rules with each member as
//!   the subject and any member as the object, within a budget of matches
//!   ([`LINKS_WORK`]); for a class past it, or of more than
//!   [`MANY_MEMBERS`] members, by joining them once for the class, both
//!   terms bound up to it and *pinned* to the members base triples state
//!   ([`Item::Class`], [`Search::pin`]). Each member reading every triple
//!   of its class costs members × triples; the class read once, its
//!   triples.
//! - **`x owl:sameAs y`** in a class: eq-sym from a link the other way
//!   round, and eq-trans from `x owl:sameAs m` and the link
//!   `m owl:sameAs y`, for each link `m` of `y` (in a class of more than
//!   [`FEW_LINKS`] links, only each `m` one link nearer to `x` than `y` is,
//!   by a breadth-first search over the links from `x`,
//!   [`Search::distance`]: the shortest chains, so that a goal reaches the
//!   members on them, not every member). The chains are left-linear, so
//!   the ranks keep the shortest whose links have proofs. A base link and
//!   a dt-eq pair need no chain; at the first level a `owl:sameAs` fact in
//!   a class gets no other instances (its links' come with the links).
//! - **Any other fact `s p o`**: eq-rep-s, eq-rep-p and eq-rep-o, in that
//!   order, from up to [`SEEDS`] *seeds* with the same representatives,
//!   those sharing most terms with the goal first: the base triples of its
//!   image, and the triples the rules derive as stated with that image
//!   ([`Search::realizations`]: the rules joined with each classed position
//!   bound only up to its class, [`Free::Classed`]). With none, from the
//!   image itself and from the literals of its classes (a rule that reads a
//!   literal, dt-type2 or dt-not-type, derives it of the literal alone).
//! - **dt-diff's `x unlisted y`**: eq-rep-s and eq-rep-o from the triple
//!   between a literal equal to `x` and one equal to `y` of different
//!   values, which the program's dt-diff derives.
//!
//! These steps are complete when every fact has a stated form that a rule
//! derives over stated premises; a list cell made `owl:sameAs rdf:nil`, or
//! a link whose premises need its own class, can need another member. So a
//! goal with no proof is searched again at the next **level**, in a graph
//! of its own kept for the next goals: level 1 gives a free variable of a
//! fact's own rules every member of its class (and every `owl:sameAs` fact
//! its rules), the helper relations still joined as stated; level 2 joins
//! the helpers with every member too and adds the eq-* rules of the
//! program, under which every proof the program has is found. The goals
//! that do not need them never walk their graphs.
//!
//! A fallback's graph that a goal left unchanged is ranked whole once
//! ([`Search::ranks_all`]), and a later goal it ranks at r is built from
//! those ranks when every fact within r - 1 of it is explained already.
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
//! triple is the depth of its shallowest proof among the instances found.
//!
//! # The proof
//!
//! From the goal, each closure triple's kept instance is a step. A helper
//! premise is folded into the step that reads it: replaced by the premises
//! of its own kept instance, down to closure triples, so a step's premises
//! are triples, list cells included. Each premise ranks below the step, so
//! the steps are acyclic. A triple used twice is one step.

use super::closure::{Closure, Flow, Free};
use super::program::{Arg, Body, Builtin, Compiled, EqRules, Kind, MAX_ARITY, NONE, RelId, T, UNLISTED};
use crate::Cancel;
use crate::engine::Members;
use crate::rules::datatype::map::{Datatype, Reading, Value, value_of};
use crate::term::{Term, TermId, vocab};
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::hash::{Hash, Hasher};

/// A call's relation, pattern, and how it reads members: expanded, the
/// positions bound up to their class and those pinned, and pinning.
type CallKey = (RelId, Pattern, bool, u8, u8, bool);

/// A tuple of a relation, padded with zeros past its arity.
pub(crate) type Tuple = [u32; MAX_ARITY];

/// A call's arguments: bound or not.
type Pattern = [Option<u32>; MAX_ARITY];

/// `owl:sameAs`, as written.
const SAME: u32 = vocab::OWL_SAME_AS.0;

/// The most base triples an eq-rep rewrite starts from.
pub(crate) const SEEDS: usize = 16;

/// The instances a fallback's graph holds before a goal that explains
/// nothing new has the whole graph ranked for the next ones
/// ([`Search::ranks_all`]).
const WHOLE: usize = 4096;

/// The matches the members' links of one class may read before the class
/// is linked as a whole ([`Search::links`]): about 0.1 s. Chaos's classes
/// read up to about 250,000.
const LINKS_WORK: usize = 1 << 20;

/// The members of a class linked as a whole at once ([`Search::links`]):
/// each member's links read every triple of the class again.
const MANY_MEMBERS: usize = 64;

/// The links a class may have for a chain to take every link into its last
/// member ([`Search::chain`]); a class of more takes the shortest chains
/// only.
const FEW_LINKS: usize = 4096;

/// The first value id ([`Builtin::ValueId`]): far from the term ids, which
/// they are never compared with.
const VALUE_IDS: u32 = 1 << 31;

/// What the search reads: the program of the configuration, the closure,
/// and the literals the rules name.
#[derive(Clone, Copy)]
pub(crate) struct World<'a> {
    pub(crate) program: &'a Compiled,
    pub(crate) closure: Closure<'a>,
    /// The literals the rules name (`datatype::RULE_LITERALS`).
    pub(crate) rule_literals: &'a [u32],
    /// Checked between the search's steps ([`Search::halted`]) and while a
    /// proof is built ([`Checks`]).
    pub(crate) cancel: Option<&'a Cancel>,
}

impl World<'_> {
    /// The term of `id`, or `None` for an id the dictionary does not hold
    /// (a constant it lacks, dt-diff's unlisted predicate, a value id).
    fn term(&self, id: u32) -> Option<Term<'_>> {
        let terms = self.closure.reasoner.terms();
        ((id as usize) < terms.len()).then(|| terms.term(TermId(id)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Item {
    Fact(u32),
    Call(u32),
    /// The links a member of a class has by a rule other than the eq-*
    /// ones: `m owl:sameAs ?y`, `?y` any member ([`Search::links`]).
    Links(u32),
    /// The links of the class of this representative by such a rule, with
    /// both terms bound up to the class: `x owl:sameAs y` for the members
    /// the premises state ([`Search::links`]).
    Class(u32),
    /// The triples a rule derives as stated whose image is this node's, a
    /// triple in representatives ([`Search::realized`]).
    Realize(u32),
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
    /// once its call is), the eq-* rules of the program aside.
    expanded: bool,
    /// Explained with the eq-* rules of the program too.
    generic: bool,
    /// It has an instance over base triples alone: none other is needed.
    grounded: bool,
}

#[derive(Debug)]
struct Call {
    rel: RelId,
    pattern: Pattern,
    /// Evaluated with every member of a class for a free variable (the
    /// fallback), not its representative alone.
    expand: bool,
    /// The positions bound only up to their class ([`Free::Classed`]), as
    /// bits: an answer holds there the member a premise states.
    classed: u8,
    /// The positions bound to a member that the premises must state, as
    /// bits, and whether a member a premise states for a classed position
    /// is then pinned so (a class's links, [`Item::Class`]).
    pinned: u8,
    pin: bool,
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

/// The links of one class (see the module doc): the base `owl:sameAs`
/// triples between its members, either way round, and its literals
/// grouped by value (dt-eq links every two in a group).
#[derive(Debug, Default)]
struct Links {
    /// Base triples, either way round, and the pairs a rule other than the
    /// eq-* ones concludes (prp-fp, prp-key, cls-maxc2, ...), by member.
    base: FxHashMap<u32, Vec<u32>>,
    /// The pairs `(x, y)` whose `x owl:sameAs y` such a rule concludes.
    derived: FxHashSet<(u32, u32)>,
    /// The base links as stated, `(x, y)` for `x q y`, with the first `q`
    /// met: `owl:sameAs`, or a term made equal to it.
    stated: FxHashMap<(u32, u32), u32>,
    group_of: FxHashMap<u32, u32>,
    groups: Vec<Vec<u32>>,
    /// Every link is found: a link found later makes the searches over the
    /// links stale ([`Graph::reach`]).
    built: bool,
    /// Linked as a whole ([`Item::Class`]): a large class, whose members
    /// share partners (a star), so that a chain takes the shortest links.
    whole: bool,
}

impl Links {
    /// The members linked to `x` (some more than once), its group of equal
    /// values included unless `groups` says it was walked already.
    fn neighbours(&self, x: u32, groups: Option<&mut FxHashSet<u32>>) -> Vec<u32> {
        let mut out: Vec<u32> = self.base.get(&x).cloned().unwrap_or_default();
        if let Some(&g) = self.group_of.get(&x)
            && groups.is_none_or(|done| done.insert(g))
        {
            out.extend(&self.groups[g as usize]);
        }
        out.retain(|&m| m != x);
        out
    }

    /// The links held, each once each way, and the pairs of equal values.
    fn len(&self) -> usize {
        self.base.values().map(Vec::len).sum::<usize>() + self.groups.iter().map(|g| g.len() * g.len()).sum::<usize>()
    }

    fn bytes(&self) -> usize {
        use crate::profile::{hash_bytes, vec_bytes};
        hash_bytes(self.base.capacity(), 32)
            + self.base.values().map(vec_bytes).sum::<usize>()
            + hash_bytes(self.group_of.capacity(), 8)
            + hash_bytes(self.derived.capacity(), 8)
            + hash_bytes(self.stated.capacity(), 12)
            + self.groups.iter().map(vec_bytes).sum::<usize>()
    }
}

/// A breadth-first search over a class's links from one member, run as far
/// as the members asked for need ([`Search::distance`]).
#[derive(Debug, Default)]
struct Reach {
    /// The number of links from the source, of each member met.
    dist: FxHashMap<u32, u32>,
    /// The members met and not walked yet, nearest first.
    queue: VecDeque<u32>,
    /// The groups of equal values walked.
    groups: FxHashSet<u32>,
}

impl Reach {
    fn bytes(&self) -> usize {
        use crate::profile::hash_bytes;
        hash_bytes(self.dist.capacity(), 8) + self.queue.capacity() * 4 + hash_bytes(self.groups.capacity(), 4)
    }
}

/// The value ids [`Builtin::ValueId`] gives: one per value, by the first
/// literal met with it.
#[derive(Debug, Default)]
struct Values {
    by_hash: FxHashMap<u64, Vec<(u32, u32)>>,
    next: u32,
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
    call_ids: FxHashMap<CallKey, u32>,
    instances: Vec<Instance>,
    seen: FxHashSet<(usize, u32, Vec<u32>)>,
    /// The instances of each node.
    of: Vec<Vec<usize>>,
    /// The links of each class met, by representative.
    links: FxHashMap<u32, Links>,
    /// The searches over the links, by source member.
    reach: FxHashMap<u32, Reach>,
    values: Values,
    /// The triples [`Search::realizations`] found, by image.
    realized: FxHashMap<[u32; 3], Vec<[u32; 3]>>,
    /// The graph of the searches that fell back (see the module doc), kept
    /// apart so that the goals that do not need it do not walk it.
    fallback: Option<Box<Graph>>,
    /// Scratch for [`Search::ranks`]: each node's slot in the ranking, when
    /// its stamp is the ranking's epoch.
    slot: Vec<u32>,
    stamp: Vec<u32>,
    /// By instance: the epoch of the ranking that took it last.
    taken: Vec<u32>,
    /// The ranks of every node, by the graph's size then
    /// ([`Search::ranks_all`]).
    whole: Option<((usize, usize), Ranks)>,
    epoch: u32,
}

impl Graph {
    /// The instance kept for node `n` by `ranks`, the last ranking made.
    fn kept(&self, ranks: &Ranks, n: u32) -> Option<usize> {
        if ranks.whole {
            return ranks.kept[n as usize];
        }
        debug_assert_eq!(self.stamp[n as usize], ranks.epoch, "a node of the ranking");
        ranks.kept[self.slot[n as usize] as usize]
    }

    /// The nodes and instances held.
    pub(crate) fn len(&self) -> (usize, usize) {
        (self.nodes.len(), self.instances.len())
    }

    /// The bytes held, about: each vector's room and each map's table.
    pub(crate) fn bytes(&self) -> usize {
        use crate::profile::{hash_bytes, vec_bytes};
        use std::mem::size_of;
        vec_bytes(&self.nodes)
            + hash_bytes(self.node_ids.capacity(), size_of::<((RelId, Tuple), u32)>())
            + vec_bytes(&self.calls)
            + self
                .calls
                .iter()
                .map(|c| {
                    vec_bytes(&c.answers)
                        + vec_bytes(&c.readers)
                        + hash_bytes(c.answered.capacity(), 4)
                        + hash_bytes(c.read_by.capacity(), size_of::<Item>())
                })
                .sum::<usize>()
            + hash_bytes(self.call_ids.capacity(), size_of::<(CallKey, u32)>())
            + vec_bytes(&self.instances)
            + self.instances.iter().map(|i| 2 * vec_bytes(&i.premises)).sum::<usize>()
            + hash_bytes(self.seen.capacity(), size_of::<(usize, u32, Vec<u32>)>())
            + vec_bytes(&self.of)
            + self.of.iter().map(vec_bytes).sum::<usize>()
            + self.links.values().map(Links::bytes).sum::<usize>()
            + hash_bytes(self.reach.capacity(), size_of::<(u32, Reach)>())
            + self.reach.values().map(Reach::bytes).sum::<usize>()
            + self.values.by_hash.values().map(vec_bytes).sum::<usize>()
            + vec_bytes(&self.slot)
            + vec_bytes(&self.stamp)
            + vec_bytes(&self.taken)
            + self
                .whole
                .as_ref()
                .map_or(0, |(_, r)| vec_bytes(&r.rank) + vec_bytes(&r.kept))
            + self.fallback.as_ref().map_or(0, |g| g.bytes())
    }
}

/// The least rank of each node a goal reaches, and the instance that gives
/// it (`None` for a base triple), by the node's place in the graph's
/// scratch slots of `epoch` (valid until the next ranking).
#[derive(Debug)]
struct Ranks {
    epoch: u32,
    /// Of every node, by id ([`Search::ranks_all`]).
    whole: bool,
    rank: Vec<Option<u32>>,
    kept: Vec<Option<usize>>,
}

/// One search, from one goal, over an explainer's graph.
pub(crate) struct Search<'a> {
    world: World<'a>,
    g: &'a mut Graph,
    queue: VecDeque<Item>,
    /// The calls queued, evaluated before [`Search::queue`]'s facts.
    calls: VecDeque<Item>,
    queued: FxHashSet<Item>,
    /// A free variable takes every member of its class (see the module
    /// doc).
    generic: bool,
    /// The eq-* rules of the program are run too.
    eq_rules: bool,
    /// The item being evaluated has an instance over base triples alone.
    stop: bool,
    /// The triples an [`Item::Realize`] found.
    realized: Vec<[u32; 3]>,
    /// The variables of the rule being joined bound only up to their class
    /// ([`Free::Classed`]), as bits.
    classed: u64,
    /// The variables bound to a member that the premises must state, as
    /// bits (see [`Search::pin`]).
    pinned: u64,
    /// A variable bound up to its class takes the member a premise states
    /// pinned: later premises must state that member too, not any other
    /// spelling of the class (a class's links, [`Item::Class`]: each
    /// premise is then read for the members it states, never once per
    /// member).
    pin: bool,
    /// The matches left to the members' links of the class being linked
    /// ([`Search::links`]).
    work: usize,
    /// The links [`Item::Class`] concluded (see [`Search::solve`]).
    linked: usize,
    /// The cancel handle tripped: the search stops, and its graph is only
    /// fit to be dropped.
    halted: bool,
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
            calls: VecDeque::new(),
            queued: FxHashSet::default(),
            generic: false,
            eq_rules: false,
            stop: false,
            realized: Vec::new(),
            classed: 0,
            pinned: 0,
            pin: false,
            work: 0,
            linked: 0,
            halted: false,
        }
    }

    /// Checks the cancel handle (one counted check, until it trips) and
    /// records in `halted` that it tripped. True once it has.
    fn check_cancel(&mut self) -> bool {
        if !self.halted {
            self.halted = self.world.cancel.is_some_and(Cancel::check);
        }
        self.halted
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
    /// A goal with no proof then is searched again at the next level (see
    /// the module doc), in the graph kept for that level.
    pub(crate) fn prove(mut self, rel: RelId, tuple: Tuple) -> Option<Built> {
        let goal = self.node(rel, tuple);
        // A goal the kept ranks of the whole graph rank at r: when every
        // fact within r - 1 of it is already explained, no proof is
        // shallower, and it is built at once.
        let at = self.g.len();
        if let Some(r) = self
            .g
            .whole
            .as_ref()
            .filter(|(size, _)| *size == at)
            .and_then(|(_, ranks)| ranks.rank.get(goal as usize).copied().flatten())
        {
            let (expanded, _) = self.frontier(goal, r.saturating_sub(1));
            if self.halted {
                return None;
            }
            if !expanded && self.g.len() == at {
                let (_, ranks) = self.g.whole.take().expect("kept");
                let built = self.build(goal, &ranks);
                self.g.whole = Some((at, ranks));
                return built;
            }
        }
        let start = self.g.len();
        let built = self.deepen(rel, tuple, goal);
        // A fallback's search halted too, with its own flag.
        if self.halted || self.world.cancel.is_some_and(Cancel::is_cancelled) {
            return None;
        }
        // A goal that explained nothing new in a fallback's graph: the next
        // ones likely will not either, and ranking the whole graph once
        // serves them all. (A fallback joins every member of a class, so
        // its goals reach most of its graph. At the first level a goal
        // reaches a small part of a graph that most goals grow, and ranking
        // it all again would cost each goal the whole graph.)
        if self.generic && self.g.len() == start && self.g.instances.len() >= WHOLE {
            self.ranks_all();
        }
        built
    }

    /// [`Search::prove`]'s iterative deepening.
    fn deepen(&mut self, rel: RelId, tuple: Tuple, goal: u32) -> Option<Built> {
        let mut bound = 0u32;
        loop {
            if self.check_cancel() {
                return None;
            }
            let cut = loop {
                let (expanded, cut) = self.frontier(goal, bound);
                if self.halted {
                    return None;
                }
                if !expanded {
                    break cut;
                }
            };
            let ranks = self.ranks(goal, bound);
            let rank = ranks.rank[0];
            if rank.is_some_and(|r| r <= bound + 1) {
                return self.build(goal, &ranks);
            }
            if !cut {
                let ranks = self.ranks(goal, u32::MAX);
                if ranks.rank[0].is_some() {
                    return self.build(goal, &ranks);
                }
                if self.eq_rules || self.world.program.eq.is_none() || self.world.closure.is_flat() {
                    return None;
                }
                let deeper = self.g.fallback.get_or_insert_with(Box::default);
                let mut again = Search::new(self.world, deeper);
                again.generic = true;
                again.eq_rules = self.generic;
                return again.prove(rel, tuple);
            }
            // A proof of rank r is in hand: every fact within r - 1 of the
            // goal explained, the least rank is known.
            bound = match rank {
                Some(r) => r - 1,
                None => bound.saturating_mul(2).saturating_add(1),
            };
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
        let base = rel == T && self.world.closure.reasoner.is_base(&[tuple[0], tuple[1], tuple[2]]);
        self.g.nodes.push(Node {
            rel,
            tuple,
            base,
            step,
            expanded: base,
            generic: base,
            grounded: false,
        });
        self.g.of.push(Vec::new());
        self.g.node_ids.insert((rel, tuple), n);
        n
    }

    /// The node of a closure triple.
    fn triple(&mut self, t: [u32; 3]) -> u32 {
        self.node(T, [t[0], t[1], t[2], 0])
    }

    /// Explains the steps within `bound` of the goal not explained yet, as
    /// they are met. A premise that is a step is one further than the step
    /// that reads it; a helper is as far, and the premises of its instances
    /// are as its reader's (0-1 breadth first). True when it explained one
    /// (which may have given instances to steps met before: another pass
    /// finds them), and whether the bound left out a step that has
    /// instances (when not, every fact the goal reaches is explained).
    fn frontier(&mut self, goal: u32, bound: u32) -> (bool, bool) {
        let fit = |g: &mut Graph| {
            let total = g.nodes.len();
            if g.slot.len() < total {
                g.slot.resize(total, 0);
                g.stamp.resize(total, 0);
            }
        };
        fit(self.g);
        self.g.epoch = self.g.epoch.wrapping_add(1);
        if self.g.epoch == 0 {
            self.g.stamp.fill(0);
            self.g.taken.fill(0);
            self.g.epoch = 1;
        }
        let epoch = self.g.epoch;
        let mut dist: Vec<u32> = vec![0];
        self.g.stamp[goal as usize] = epoch;
        self.g.slot[goal as usize] = 0;
        let mut queue = VecDeque::from([(goal, 0u32)]);
        let (mut expanded, mut cut) = (false, false);
        while let Some((n, k)) = queue.pop_front() {
            if dist[self.g.slot[n as usize] as usize] < k {
                continue;
            }
            let node = &self.g.nodes[n as usize];
            if node.step && !(node.expanded && (node.generic || !self.generic)) {
                let node = &mut self.g.nodes[n as usize];
                node.expanded = true;
                node.generic |= self.generic;
                self.enqueue(Item::Fact(n));
                self.run();
                if self.halted {
                    return (false, false);
                }
                fit(self.g);
                expanded = true;
            }
            let g = &mut *self.g;
            if g.nodes[n as usize].step && k >= bound {
                cut |= !g.of[n as usize].is_empty();
                continue;
            }
            for &i in &g.of[n as usize] {
                for &p in &g.instances[i].premises {
                    let w = u32::from(g.nodes[p as usize].step);
                    let d = k + w;
                    let fresh = g.stamp[p as usize] != epoch;
                    if fresh {
                        g.stamp[p as usize] = epoch;
                        g.slot[p as usize] = dist.len() as u32;
                        dist.push(d);
                    }
                    let at = g.slot[p as usize] as usize;
                    if fresh || d < dist[at] {
                        dist[at] = d;
                        if w == 0 {
                            queue.push_front((p, d));
                        } else {
                            queue.push_back((p, d));
                        }
                    }
                }
            }
        }
        (expanded, cut)
    }

    /// Queues an item: a call before any fact, so that a call reaches its
    /// fixpoint before the facts that read it are joined again.
    fn enqueue(&mut self, item: Item) {
        if self.queued.insert(item) {
            match item {
                Item::Call(_) => self.calls.push_back(item),
                _ => self.queue.push_back(item),
            }
        }
    }

    /// Runs what an item evaluated inside another's equality steps asked
    /// for (its calls), to their fixpoint: a fact queued meanwhile is left
    /// for the outer run, so that nesting stops there.
    fn run_nested(&mut self) {
        let mut facts = Vec::new();
        while !self.check_cancel()
            && let Some(item) = self.calls.pop_front().or_else(|| self.queue.pop_front())
        {
            if let Item::Fact(_) = item {
                facts.push(item);
                continue;
            }
            self.queued.remove(&item);
            self.evaluate(item);
        }
        self.queue.extend(facts);
    }

    fn run(&mut self) {
        while !self.check_cancel()
            && let Some(item) = self.calls.pop_front().or_else(|| self.queue.pop_front())
        {
            self.queued.remove(&item);
            self.evaluate(item);
        }
    }

    /// True when the call binds every argument of its relation, none only
    /// up to its class.
    fn closed(&self, c: u32) -> bool {
        let call = &self.g.calls[c as usize];
        let arity = self.world.program.relations[call.rel as usize].arity;
        call.classed == 0 && call.pattern[..arity].iter().all(Option::is_some)
    }

    /// Joins the body of every rule whose head matches the item, and for a
    /// closure triple, writes its equality steps.
    fn evaluate(&mut self, item: Item) {
        let (rel, bound): (RelId, Pattern) = match item {
            Item::Fact(n) => {
                let node = &self.g.nodes[n as usize];
                if node.grounded {
                    return;
                }
                (node.rel, node.tuple.map(Some))
            }
            Item::Call(c) => {
                let call = &self.g.calls[c as usize];
                if self.closed(c) && call.answers.first().is_some_and(|&a| self.g.nodes[a as usize].grounded) {
                    return;
                }
                (call.rel, call.pattern)
            }
            Item::Links(m) => (T, [Some(m), Some(SAME), None, None]),
            Item::Class(r) => (T, [Some(r), Some(SAME), Some(r), None]),
            Item::Realize(n) => (T, self.g.nodes[n as usize].tuple.map(Some)),
        };
        let program = self.world.program;
        self.stop = false;
        let pin = std::mem::replace(
            &mut self.pin,
            match item {
                Item::Class(_) => true,
                Item::Call(c) => self.g.calls[c as usize].pin,
                _ => false,
            },
        );
        // `x owl:sameAs y` in a class: its links' own instances are found
        // with the class's links (`Search::links`), and the rest is chains.
        let chained = match item {
            Item::Fact(n) => !self.generic && self.in_class(n),
            _ => false,
        };
        for &r in &program.by_head[rel as usize] {
            if self.stop || chained {
                break;
            }
            if program.is_eq[r] && (!self.eq_rules || matches!(item, Item::Links(_) | Item::Class(_))) {
                continue;
            }
            let rule = &program.rules[r];
            // A class's links: a rule whose head has one variable for both
            // terms (eq-ref) links a member to itself only.
            if matches!(item, Item::Class(_)) && rule.head.args[0] == rule.head.args[2] {
                continue;
            }
            let mut b = vec![None; rule.vars];
            let arity = program.relations[rel as usize].arity;
            let closure = self.world.closure;
            // Realizing: a position whose class has two or more members
            // binds its variable up to the class; finding a class's links,
            // both terms; a call, the positions its reader bound so.
            let (call_classed, call_pinned) = match item {
                Item::Call(c) => (self.g.calls[c as usize].classed, self.g.calls[c as usize].pinned),
                _ => (0, 0),
            };
            let classed = |k: usize, x: u32| match item {
                Item::Realize(_) => k < 3 && closure.members(x).len() > 1,
                Item::Class(_) => k == 0 || k == 2,
                Item::Call(_) => call_classed & (1 << k) != 0,
                Item::Fact(_) | Item::Links(_) => false,
            };
            let mut bits = 0u64;
            let mut pins = 0u64;
            let unified = rule
                .head
                .args
                .iter()
                .zip(&bound[..arity])
                .enumerate()
                .all(|(k, (arg, x))| match (arg, x) {
                    (_, None) => true,
                    (Arg::Id(c), Some(x)) => c == x || (classed(k, *x) && closure.find(*c) == *x),
                    (Arg::Var(v), Some(x)) => match b[*v as usize] {
                        Some(y) => y == *x,
                        None => {
                            b[*v as usize] = Some(*x);
                            if classed(k, *x) {
                                bits |= 1 << v;
                            }
                            if call_pinned & (1 << k) != 0 {
                                pins |= 1 << v;
                            }
                            true
                        }
                    },
                });
            if unified {
                (self.classed, self.pinned) = (bits, pins);
                self.solve(r, item, &mut b, 0, &mut Vec::new());
                (self.classed, self.pinned) = (0, 0);
            }
        }
        self.pin = pin;
        if let Item::Fact(n) = item
            && rel == T
            && !self.stop
        {
            self.equality(n);
        }
        self.stop = false;
    }

    /// The next body element of rule `r` to join, given what is bound and
    /// done: a filter as soon as it can run, then the atom with the most
    /// bound arguments (a triple before a helper). A variable bound only up
    /// to its class is bound for an atom, which gives it the member stated,
    /// and not yet for a filter, which would read the representative: a
    /// filter reads it so only when no atom is left.
    fn choose(&self, r: usize, b: &[Option<u32>], done: u64) -> Option<usize> {
        let program = self.world.program;
        let rule = &program.rules[r];
        let bound = |a: &Arg| match a {
            Arg::Id(_) => true,
            Arg::Var(v) => b[*v as usize].is_some(),
        };
        let settled = |a: &Arg| bound(a) && !matches!(a, Arg::Var(v) if self.classed & (1 << v) != 0);
        let ready = |element: &Body, bound: &dyn Fn(&Arg) -> bool| match element {
            Body::NotEqual(x, y) => bound(x) && bound(y),
            Body::Atom(a) => match program.relations[a.rel as usize].kind {
                Kind::Builtin(f) => match f {
                    Builtin::RuleLiteral => true,
                    Builtin::TypesOf | Builtin::IllTyped | Builtin::ValueId => bound(&a.args[0]),
                    Builtin::OutsideValueSpace => bound(&a.args[0]) && bound(&a.args[1]),
                },
                _ => false,
            },
        };
        let mut best: Option<(usize, usize)> = None;
        let mut left = None;
        for (k, element) in rule.body.iter().enumerate() {
            if done & (1 << k) != 0 {
                continue;
            }
            left.get_or_insert(k);
            match element {
                Body::Atom(a) if !matches!(program.relations[a.rel as usize].kind, Kind::Builtin(_)) => {
                    let kind = program.relations[a.rel as usize].kind;
                    let score = 2 * a.args.iter().filter(|a| bound(a)).count() + usize::from(kind == Kind::Triple);
                    if best.is_none_or(|(s, _)| score > s) {
                        best = Some((score, k));
                    }
                }
                filter => {
                    if ready(filter, &settled) {
                        return Some(k);
                    }
                }
            }
        }
        if let Some((_, k)) = best {
            return Some(k);
        }
        let left = left?;
        let filter = (left..rule.body.len()).find(|&k| done & (1 << k) == 0 && ready(&rule.body[k], &bound));
        Some(filter.unwrap_or_else(|| {
            panic!(
                "the rule {:?} has a filter whose terms no atom binds",
                rule.name.unwrap_or("(a helper)")
            )
        }))
    }

    /// The variables of `args` bound only up to their class, as bits.
    fn classed_in(&self, args: &[Arg]) -> u64 {
        args.iter()
            .filter_map(|a| match *a {
                Arg::Var(v) if self.classed & (1 << v) != 0 => Some(1 << v),
                _ => None,
            })
            .fold(0, |x, y| x | y)
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
                    // A variable bound up to its class is the
                    // representative from here on.
                    let classed = self.classed;
                    self.classed &= !self.classed_in(&[*x, *y]);
                    self.solve(r, item, b, done, premises);
                    self.classed = classed;
                }
            }
            Body::Atom(atom) => {
                let pattern: Vec<Option<u32>> = atom.args.iter().map(|a| value(b, a)).collect();
                match program.relations[atom.rel as usize].kind {
                    Kind::Triple => {
                        let closure = world.closure;
                        let (classed, pinned) = (self.classed, self.pinned);
                        let var_in = |bits: u64, k: usize| matches!(&atom.args[k], Arg::Var(v) if bits & (1 << v) != 0);
                        let mut expand = match item {
                            // Discovering a member's links: every partner it
                            // has, and for the rest what is stated.
                            Item::Links(_) if !self.generic => [0, 1, 2].map(|k| match &atom.args[k] {
                                Arg::Var(v) if rule.head.args.contains(&Arg::Var(*v)) => Free::Members,
                                _ => Free::Stated,
                            }),
                            _ if self.expands(item) => [Free::Members; 3],
                            _ => [Free::Stated; 3],
                        };
                        // A pinned position is read as one bound up to its
                        // class, and only its member is kept. Pinning, a
                        // position bound up to its class is matched by base
                        // triples only.
                        let unlisted = pattern[1] == Some(UNLISTED);
                        let pinning = self.pin;
                        let pins = [0, 1, 2].map(|k| var_in(pinned, k) && !unlisted);
                        let classes = [0, 1, 2].map(|k| var_in(classed, k) || pins[k]);
                        let mut query = [pattern[0], pattern[1], pattern[2]];
                        for (k, free) in expand.iter_mut().enumerate() {
                            if classes[k] {
                                *free = if pinning { Free::Base } else { Free::Classed };
                            }
                            if pins[k] {
                                query[k] = query[k].map(|x| closure.find(x));
                            }
                        }
                        // A class's links: the atom that gives the second
                        // member of a pair, the first pinned, gives one
                        // partner, not every one (a member of many partners,
                        // as a functional property's subject of many
                        // values, links them as a star, not pairwise).
                        let partner =
                            matches!(item, Item::Class(_)) && pinned != 0 && (0..3).any(|k| var_in(classed, k));
                        let visit = &mut |t: [u32; 3]| {
                            if (0..3).any(|k| pins[k] && Some(t[k]) != pattern[k]) {
                                return Flow::Continue(());
                            }
                            let linked = self.linked;
                            // A member's links within their budget.
                            if let Item::Links(_) = item {
                                if self.work == 0 {
                                    self.stop = true;
                                    return Flow::Break(());
                                }
                                self.work -= 1;
                            }
                            // A variable bound up to its class takes the
                            // member matched.
                            let mut was = [(0u8, None); 3];
                            let mut w = 0;
                            if self.classed != 0 {
                                for (arg, &x) in atom.args.iter().zip(&t) {
                                    if let Arg::Var(v) = *arg
                                        && self.classed & (1 << v) != 0
                                    {
                                        was[w] = (v, b[v as usize]);
                                        w += 1;
                                        b[v as usize] = Some(x);
                                        self.classed &= !(1 << v);
                                        if self.pin {
                                            self.pinned |= 1 << v;
                                        }
                                    }
                                }
                            }
                            if let Some(new) = bind(&atom.args, &t, b) {
                                let n = self.triple(t);
                                premises.push(n);
                                self.solve(r, item, b, done, premises);
                                premises.pop();
                                unbind(new, b);
                            }
                            for &(v, old) in &was[..w] {
                                b[v as usize] = old;
                            }
                            (self.classed, self.pinned) = (classed, pinned);
                            if self.stop || (partner && self.linked > linked) {
                                Flow::Break(())
                            } else {
                                Flow::Continue(())
                            }
                        };
                        // Base triples only: read by the member pinned when it
                        // is not its class's representative, or with no
                        // subject bound, by each member of the class, not
                        // through the representatives' tables (whose
                        // subjects of an object need an index of their own,
                        // one for each predicate when it is free).
                        let by_member = (0..3)
                            .find(|&k| pins[k] && query[k] != pattern[k])
                            .or_else(|| (0..3).find(|&k| classes[k] && pattern[0].is_none()))
                            .filter(|_| pinning && !unlisted && !expand.contains(&Free::Members));
                        match by_member {
                            Some(k) => {
                                let x = pattern[k].expect("bound up to its class");
                                let members = if pins[k] { Members::One(x) } else { closure.members(x) };
                                'members: for m in members.iter() {
                                    for &st in closure.stating(m) {
                                        let mut u = st;
                                        let fits = st[k] == m
                                            && (0..3).all(|j| match pattern[j] {
                                                None => true,
                                                Some(x) if pins[j] => st[j] == x,
                                                Some(x) if classes[j] => closure.find(st[j]) == x,
                                                Some(x) => {
                                                    u[j] = x;
                                                    closure.find(st[j]) == closure.find(x)
                                                }
                                            });
                                        if fits && visit(u).is_break() {
                                            break 'members;
                                        }
                                    }
                                }
                            }
                            None => {
                                let _ = closure.each(query[0], query[1], query[2], expand, visit);
                            }
                        }
                    }
                    Kind::Builtin(f) => {
                        let classed = self.classed;
                        self.classed &= !self.classed_in(&atom.args);
                        for t in self.test(f, &pattern) {
                            if self.stop {
                                break;
                            }
                            if let Some(new) = bind(&atom.args, &t, b) {
                                self.solve(r, item, b, done, premises);
                                unbind(new, b);
                            }
                        }
                        self.classed = classed;
                    }
                    Kind::Derived => {
                        let mut call_pattern: Pattern = [None; MAX_ARITY];
                        call_pattern[..pattern.len()].copy_from_slice(&pattern);
                        // A variable bound up to its class is so in the call
                        // too, and takes the member an answer holds.
                        let (classed, pinned) = (self.classed, self.pinned);
                        let (mut mask, mut pins) = (0u8, 0u8);
                        for (k, arg) in atom.args.iter().enumerate() {
                            if let Arg::Var(v) = *arg {
                                if classed & (1 << v) != 0 {
                                    mask |= 1 << k;
                                }
                                if pinned & (1 << v) != 0 {
                                    pins |= 1 << k;
                                }
                            }
                        }
                        let expand = self.expands(item) && self.eq_rules;
                        let pin = self.pin && (mask | pins) != 0;
                        let c = self.call(atom.rel, call_pattern, (expand, mask, pins, pin), item);
                        let partner = matches!(item, Item::Class(_)) && pinned != 0 && mask != 0;
                        let linked = self.linked;
                        let mut j = 0;
                        while j < self.g.calls[c as usize].answers.len()
                            && !self.stop
                            && !(partner && self.linked > linked)
                        {
                            let n = self.g.calls[c as usize].answers[j];
                            let t = self.g.nodes[n as usize].tuple;
                            let mut was = [(0u8, None); MAX_ARITY];
                            let mut w = 0;
                            if mask != 0 {
                                for (arg, &x) in atom.args.iter().zip(&t) {
                                    if let Arg::Var(v) = *arg
                                        && self.classed & (1 << v) != 0
                                    {
                                        was[w] = (v, b[v as usize]);
                                        w += 1;
                                        b[v as usize] = Some(x);
                                        self.classed &= !(1 << v);
                                        if self.pin {
                                            self.pinned |= 1 << v;
                                        }
                                    }
                                }
                            }
                            if let Some(new) = bind(&atom.args, &t, b) {
                                premises.push(n);
                                self.solve(r, item, b, done, premises);
                                premises.pop();
                                unbind(new, b);
                            }
                            for &(v, old) in &was[..w] {
                                b[v as usize] = old;
                            }
                            (self.classed, self.pinned) = (classed, pinned);
                            j += 1;
                        }
                    }
                }
            }
        }
    }

    /// The tuples a test on terms holds of, given the bound arguments.
    fn test(&mut self, f: Builtin, pattern: &[Option<u32>]) -> Vec<Tuple> {
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
            Builtin::ValueId => {
                let lt = pattern[0].expect("bound");
                if let Some(v) = world.closure.value(lt) {
                    let id = self.value_id(lt, &v);
                    if pattern[1].is_none_or(|x| x == id) {
                        out.push([lt, id, 0, 0]);
                    }
                }
            }
        }
        out
    }

    /// The id of the value `v` of the literal `lt`.
    fn value_id(&mut self, lt: u32, v: &Value<'_>) -> u32 {
        let closure = self.world.closure;
        let mut h = FxHasher::default();
        v.hash(&mut h);
        let values = &mut self.g.values;
        let bucket = values.by_hash.entry(h.finish()).or_default();
        if let Some(&(_, id)) = bucket
            .iter()
            .find(|&&(first, _)| closure.value(first).as_ref() == Some(v))
        {
            return id;
        }
        let id = VALUE_IDS + values.next;
        values.next += 1;
        bucket.push((lt, id));
        id
    }

    /// True when the item's joins give a free variable every member of its
    /// class.
    fn expands(&self, item: Item) -> bool {
        match item {
            Item::Fact(_) => self.generic,
            Item::Call(c) => self.g.calls[c as usize].expand,
            Item::Links(_) | Item::Class(_) | Item::Realize(_) => false,
        }
    }

    /// The call of `rel` with `pattern`, read as `how` says (expanded, the
    /// positions bound up to their class, those pinned, and pinning), read
    /// by `reader`, which is evaluated again when the call gains an answer.
    fn call(&mut self, rel: RelId, pattern: Pattern, how: (bool, u8, u8, bool), reader: Item) -> u32 {
        let (expand, classed, pinned, pin) = how;
        let key = (rel, pattern, expand, classed, pinned, pin);
        let c = match self.g.call_ids.get(&key).copied() {
            Some(c) => c,
            None => {
                let c = self.g.calls.len() as u32;
                self.g.calls.push(Call {
                    rel,
                    pattern,
                    expand,
                    classed,
                    pinned,
                    pin,
                    answers: Vec::new(),
                    answered: FxHashSet::default(),
                    readers: Vec::new(),
                    read_by: FxHashSet::default(),
                });
                self.g.call_ids.insert(key, c);
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
            Item::Realize(n) => {
                let h = self.node(T, tuple);
                if h != n {
                    self.realized.push([tuple[0], tuple[1], tuple[2]]);
                }
                h
            }
            Item::Links(m) => {
                let n = self.node(T, tuple);
                let closure = self.world.closure;
                let c = closure.find(m);
                if tuple[2] != m && closure.find(tuple[2]) == c {
                    self.link(c, m, tuple[2]);
                }
                n
            }
            Item::Class(c) => {
                let n = self.node(T, tuple);
                let closure = self.world.closure;
                let (x, y) = (tuple[0], tuple[2]);
                if x != y && closure.find(x) == c && closure.find(y) == c {
                    self.link(c, x, y);
                    self.linked += 1;
                }
                // A link over base triples alone needs no other instance.
                if self.g.nodes[n as usize].grounded {
                    return;
                }
                n
            }
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
        let grounded = self.instance(r, head, premises.to_vec());
        let closed = match item {
            Item::Fact(_) => true,
            Item::Links(_) | Item::Class(_) | Item::Realize(_) => false,
            Item::Call(c) => self.closed(c),
        };
        if grounded && closed {
            self.stop = true;
        }
    }

    /// Adds an instance of rule `r` (unless the graph has it). True when its
    /// premises are all base triples: its head then needs no other.
    fn instance(&mut self, r: usize, head: u32, premises: Vec<u32>) -> bool {
        let grounded = premises.iter().all(|&p| self.g.nodes[p as usize].base);
        if self.g.seen.insert((r, head, premises.clone())) {
            self.g.of[head as usize].push(self.g.instances.len());
            self.g.instances.push(Instance {
                rule: r,
                head,
                premises,
            });
        }
        if grounded {
            self.g.nodes[head as usize].grounded = true;
        }
        grounded
    }

    /// True when the node is `x owl:sameAs y` with `x` in a class of two or
    /// more.
    fn in_class(&self, n: u32) -> bool {
        let node = &self.g.nodes[n as usize];
        let closure = self.world.closure;
        node.rel == T
            && node.tuple[1] == SAME
            && self.world.program.eq.is_some()
            && !closure.is_flat()
            && closure.members(closure.find(node.tuple[0])).len() > 1
    }

    /// The equality steps of a closure triple (see the module doc), when
    /// some term is equal to another.
    fn equality(&mut self, n: u32) {
        let Some(eq) = self.world.program.eq else {
            return;
        };
        if self.world.closure.is_flat() {
            return;
        }
        let node = &self.g.nodes[n as usize];
        if node.base {
            return;
        }
        let t = [node.tuple[0], node.tuple[1], node.tuple[2]];
        match t[1] {
            UNLISTED => self.unlisted(eq, t),
            SAME => self.chain(eq, n, t),
            _ => self.rewrite(eq, t),
        }
    }

    /// The links of the class whose representative is `r`, built once.
    fn links(&mut self, r: u32) -> &Links {
        if !self.g.links.contains_key(&r) {
            let closure = self.world.closure;
            let mut links = Links::default();
            let members: Vec<u32> = closure.members(r).iter().collect();
            // Under `owl:sameAs` or a term made equal to it.
            let spellings: Vec<u32> = closure.members(closure.find(SAME)).iter().collect();
            for &m in &members {
                for &q in &spellings {
                    let _ = closure.base_objects(m, q, &mut |o| {
                        if o != m {
                            links.base.entry(m).or_default().push(o);
                            links.base.entry(o).or_default().push(m);
                            links.stated.entry((m, o)).or_insert(q);
                        }
                        Flow::Continue(())
                    });
                }
            }
            if closure.reasoner.switches().is_on(crate::Switch::LiteralValues) {
                let mut by_value: FxHashMap<Value<'_>, u32> = FxHashMap::default();
                for &m in &members {
                    if let Some(v) = closure.value(m) {
                        let next = links.groups.len() as u32;
                        let g = *by_value.entry(v).or_insert(next);
                        if g == next {
                            links.groups.push(Vec::new());
                        }
                        links.groups[g as usize].push(m);
                        links.group_of.insert(m, g);
                    }
                }
            }
            self.g.links.insert(r, links);
            // The derived links, each added as it is concluded
            // ([`Search::link`]): each member's `m owl:sameAs ?y` by every
            // rule but the eq-* ones, `?y` any member, within a budget of
            // matches for the class. Past it, or for a class of many
            // members (whose members each read every triple of the class
            // again), the rules are joined once for the class instead, with
            // both terms bound up to it, so that each takes the members the
            // premises state.
            let budget = if members.len() > MANY_MEMBERS { 0 } else { LINKS_WORK };
            let outer = std::mem::replace(&mut self.work, budget);
            for &m in members.iter().filter(|_| budget > 0) {
                self.evaluate(Item::Links(m));
                self.run_nested();
                if self.work == 0 {
                    break;
                }
            }
            if self.work == 0 {
                self.g.links.get_mut(&r).expect("inserted").whole = true;
                self.evaluate(Item::Class(r));
                self.run_nested();
            }
            self.work = outer;
            self.g.links.get_mut(&r).expect("inserted").built = true;
        }
        &self.g.links[&r]
    }

    /// Adds the link `x owl:sameAs y` a rule concludes in the class of `c`.
    fn link(&mut self, c: u32, x: u32, y: u32) {
        let links = self
            .g
            .links
            .get_mut(&c)
            .expect("a class's links are built from its links item");
        if links.derived.insert((x, y)) {
            links.base.entry(x).or_default().push(y);
            links.base.entry(y).or_default().push(x);
            if links.built {
                self.g.reach.clear();
            }
        }
    }

    /// The number of links from `x` to `y` in the class of `r`, by a
    /// breadth-first search from `x` kept for the next members asked for,
    /// and run only as far as `y`. When `y` is met, every member one link
    /// nearer to `x` is met too.
    fn distance(&mut self, r: u32, x: u32, y: u32) -> Option<u32> {
        let g = &mut *self.g;
        let links = &g.links[&r];
        let reach = g.reach.entry(x).or_insert_with(|| Reach {
            dist: FxHashMap::from_iter([(x, 0)]),
            queue: VecDeque::from([x]),
            groups: FxHashSet::default(),
        });
        while !reach.dist.contains_key(&y) {
            let Some(m) = reach.queue.pop_front() else {
                break;
            };
            let d = reach.dist[&m] + 1;
            for w in links.neighbours(m, Some(&mut reach.groups)) {
                if let std::collections::hash_map::Entry::Vacant(e) = reach.dist.entry(w) {
                    e.insert(d);
                    reach.queue.push_back(w);
                }
            }
        }
        reach.dist.get(&y).copied()
    }

    /// `x owl:sameAs y` in a class: eq-sym from a link the other way round,
    /// and eq-trans from `x owl:sameAs m` and the link `m owl:sameAs y`, for
    /// each link `m` of `y`. The chains are left-linear, so the ranks keep
    /// the shortest one whose links have proofs (see the module doc).
    fn chain(&mut self, eq: EqRules, n: u32, [x, _, y]: [u32; 3]) {
        let closure = self.world.closure;
        let r = closure.find(x);
        if closure.members(r).len() < 2 {
            return;
        }
        let links = self.links(r);
        // A base link spelled with a term made equal to `owl:sameAs`:
        // eq-rep-p from it.
        let spelled = links.stated.get(&(x, y)).copied().filter(|&q| q != SAME);
        let based = x != y && links.stated.contains_key(&(y, x));
        let valued = x != y
            && links
                .group_of
                .get(&x)
                .is_some_and(|g| links.group_of.get(&y) == Some(g));
        let back = based || (x != y && links.derived.contains(&(y, x)));
        if back {
            let back = self.triple([y, SAME, x]);
            self.instance(eq.sym, n, vec![back]);
        }
        if let Some(q) = spelled {
            let (same, stated) = (self.triple([q, SAME, SAME]), self.triple([x, q, y]));
            self.instance(eq.rep[1], n, vec![same, stated]);
        }
        // A base link either way round, or dt-eq's (the program's), needs
        // no chain; a derived one may, when its premises need the class.
        if based || valued || spelled.is_some() {
            return;
        }
        // In a class of few links, every link into `y`: the ranks keep the
        // shortest chain whose links have proofs. In a larger one (or one
        // linked as a whole, whose members share partners), the links
        // into `y` from a member one link nearer to `x`, on the shortest
        // chains, so that a chain reaches only the members on them (for
        // `x owl:sameAs x`, every link of `x`).
        let links = &self.g.links[&r];
        let mut before = links.neighbours(y, None);
        if (links.whole || links.len() > FEW_LINKS) && x != y {
            match self.distance(r, x, y) {
                Some(d) => {
                    let reach = &self.g.reach[&x];
                    before.retain(|&m| reach.dist.get(&m).is_some_and(|&e| e + 1 == d));
                }
                None => before.clear(),
            }
        }
        before.retain(|&m| m != x);
        before.sort_unstable();
        before.dedup();
        for m in before {
            let (to, link) = (self.triple([x, SAME, m]), self.triple([m, SAME, y]));
            self.instance(eq.trans, n, vec![to, link]);
        }
    }

    /// Any other closure triple: eq-rep-s, eq-rep-p and eq-rep-o from base
    /// triples with the same representatives, or from the triple in
    /// representatives.
    fn rewrite(&mut self, eq: EqRules, t: [u32; 3]) {
        let closure = self.world.closure;
        let reps = t.map(|x| closure.find(x));
        if (0..3).all(|k| reps[k] == t[k] && closure.members(reps[k]).len() == 1) {
            return;
        }
        // The base triples of its image, and those the rules derive as
        // stated: the ones that share most terms with the goal need fewest
        // rewrites.
        let mut seeds: Vec<[u32; 3]> = closure.images().get(&reps).cloned().unwrap_or_default();
        seeds.extend(self.realizations(reps));
        seeds.retain(|&seed| seed != t);
        seeds.sort_by_key(|seed| (0..3).filter(|&k| seed[k] != t[k]).count());
        seeds.dedup();
        seeds.truncate(SEEDS);
        if seeds.is_empty() {
            // From the image itself, and from the literals of its classes
            // (a rule that reads a literal, dt-type2 or dt-not-type, derives
            // it of the literal alone).
            let options = |k: usize| -> Vec<u32> {
                let mut out = vec![t[k], reps[k]];
                out.extend(closure.members(reps[k]).iter().filter(|&m| closure.is_literal(m)));
                out.dedup();
                out
            };
            let (os, ps, oo) = (options(0), options(1), options(2));
            for &s in &os {
                for &p in &ps {
                    for &o in &oo {
                        let seed = [s, p, o];
                        if seed != t
                            && !seeds.contains(&seed)
                            && (seed == reps || [s, o].iter().any(|&x| closure.is_literal(x)))
                        {
                            seeds.push(seed);
                        }
                    }
                }
            }
        }
        for seed in seeds {
            self.rewritten(eq, seed, t);
        }
    }

    /// The triples the rules derive as stated whose image is `reps` (found
    /// once): the rules of `T` joined with each position whose class has
    /// two or more members bound up to the class ([`Free::Classed`]), so
    /// that it takes the member a premise states.
    fn realizations(&mut self, reps: [u32; 3]) -> Vec<[u32; 3]> {
        if let Some(found) = self.g.realized.get(&reps) {
            return found.clone();
        }
        let n = self.triple(reps);
        // The run below may realize other images first (a fact queued
        // before): each keeps its own finds.
        let outer = std::mem::take(&mut self.realized);
        self.evaluate(Item::Realize(n));
        self.run_nested();
        let mut found = std::mem::replace(&mut self.realized, outer);
        found.sort_unstable();
        found.dedup();
        self.g.realized.insert(reps, found.clone());
        found
    }

    /// The eq-rep steps from `from` to `to`, a position at a time.
    fn rewritten(&mut self, eq: EqRules, from: [u32; 3], to: [u32; 3]) {
        let mut at = from;
        for k in 0..3 {
            if at[k] == to[k] {
                continue;
            }
            let mut next = at;
            next[k] = to[k];
            let same = self.triple([at[k], SAME, to[k]]);
            let (before, after) = (self.triple(at), self.triple(next));
            self.instance(eq.rep[k], after, vec![same, before]);
            at = next;
        }
    }

    /// dt-diff's `x unlisted y`: from the triple between a literal equal to
    /// `x` and one equal to `y` of different values (dt-diff's own, which
    /// the program derives), by eq-rep-s and eq-rep-o.
    fn unlisted(&mut self, eq: EqRules, [x, _, y]: [u32; 3]) {
        let closure = self.world.closure;
        let (xs, ys) = (closure.valued(x), closure.valued(y));
        for (a, va) in &xs {
            if let Some((b, _)) = ys.iter().find(|(_, vb)| vb != va) {
                if (*a, *b) != (x, y) {
                    self.rewritten(eq, [*a, UNLISTED, *b], [x, UNLISTED, y]);
                }
                return;
            }
        }
    }

    /// The least rank of each node within `bound` of the goal (as
    /// [`Search::frontier`] counts) and the instance that gives it, by
    /// Knuth's algorithm over those nodes and the instances of the steps
    /// among them within the bound. Leaving instances out can only raise a
    /// rank, never lower it, and a proof of depth `bound + 1` or less has
    /// all its instances there.
    fn ranks(&mut self, goal: u32, bound: u32) -> Ranks {
        let g = &mut *self.g;
        let total = g.nodes.len();
        if g.slot.len() < total {
            g.slot.resize(total, 0);
            g.stamp.resize(total, 0);
        }
        g.taken.resize(g.instances.len(), 0);
        g.epoch = g.epoch.wrapping_add(1);
        if g.epoch == 0 {
            g.stamp.fill(0);
            g.taken.fill(0);
            g.epoch = 1;
        }
        let epoch = g.epoch;
        let step = |g: &Graph, n: u32| g.nodes[n as usize].step;
        // The nodes and instances within the bound (0-1 breadth first).
        let mut nodes = vec![goal];
        let mut dist = vec![0u32];
        let mut followed = vec![false];
        g.stamp[goal as usize] = epoch;
        g.slot[goal as usize] = 0;
        let mut queue = VecDeque::from([goal]);
        let mut instances: Vec<usize> = Vec::new();
        while let Some(n) = queue.pop_front() {
            let at = g.slot[n as usize] as usize;
            let k = dist[at];
            if followed[at] || (step(g, n) && k > bound) {
                continue;
            }
            followed[at] = true;
            for &i in &g.of[n as usize] {
                // An instance read twice (a node followed again, closer) is
                // one.
                if g.taken[i] != epoch {
                    g.taken[i] = epoch;
                    instances.push(i);
                }
                for &p in &g.instances[i].premises {
                    let d = k + u32::from(step(g, p));
                    if g.stamp[p as usize] != epoch {
                        g.stamp[p as usize] = epoch;
                        g.slot[p as usize] = nodes.len() as u32;
                        nodes.push(p);
                        dist.push(d);
                        followed.push(false);
                        if d == k {
                            queue.push_front(p);
                        } else {
                            queue.push_back(p);
                        }
                    } else {
                        let at = g.slot[p as usize] as usize;
                        if d < dist[at] {
                            dist[at] = d;
                            followed[at] = false;
                            queue.push_front(p);
                        }
                    }
                }
            }
        }
        Self::knuth(g, epoch, nodes, instances, false)
    }

    /// The least rank of every node of the graph, and the instance that
    /// gives it, kept by the graph until it grows ([`Graph::whole`]).
    fn ranks_all(&mut self) {
        let g = &mut *self.g;
        if g.whole.as_ref().is_some_and(|(at, _)| *at == g.len()) {
            return;
        }
        let total = g.nodes.len();
        g.slot.resize(total.max(g.slot.len()), 0);
        g.stamp.resize(total.max(g.stamp.len()), 0);
        g.epoch = g.epoch.wrapping_add(1);
        if g.epoch == 0 {
            g.stamp.fill(0);
            g.taken.fill(0);
            g.epoch = 1;
        }
        let epoch = g.epoch;
        for n in 0..total {
            g.stamp[n] = epoch;
            g.slot[n] = n as u32;
        }
        let nodes: Vec<u32> = (0..total as u32).collect();
        let instances: Vec<usize> = (0..g.instances.len()).collect();
        let ranks = Self::knuth(g, epoch, nodes, instances, true);
        g.whole = Some((g.len(), ranks));
    }

    /// Knuth's algorithm over `nodes` (each at its slot of `epoch`) and
    /// `instances`.
    fn knuth(g: &Graph, epoch: u32, nodes: Vec<u32>, instances: Vec<usize>, whole: bool) -> Ranks {
        let step = |g: &Graph, n: u32| g.nodes[n as usize].step;
        let local = |g: &Graph, n: u32| g.slot[n as usize] as usize;
        let mut rank: Vec<Option<u32>> = vec![None; nodes.len()];
        let mut kept: Vec<Option<usize>> = vec![None; nodes.len()];
        let mut uses: Vec<Vec<u32>> = vec![Vec::new(); nodes.len()];
        let mut waiting: Vec<u32> = Vec::with_capacity(instances.len());
        let mut heap: BinaryHeap<Reverse<(u32, u32, usize)>> = BinaryHeap::new();
        const LEAF: usize = usize::MAX;
        let mut distinct: Vec<usize> = Vec::new();
        for (j, &i) in instances.iter().enumerate() {
            let inst = &g.instances[i];
            distinct.clear();
            distinct.extend(inst.premises.iter().map(|&p| local(g, p)));
            distinct.sort_unstable();
            distinct.dedup();
            for &p in &distinct {
                uses[p].push(j as u32);
            }
            waiting.push(distinct.len() as u32);
            if distinct.is_empty() {
                heap.push(Reverse((u32::from(step(g, inst.head)), inst.head, i)));
            }
        }
        for &n in &nodes {
            if g.nodes[n as usize].base {
                heap.push(Reverse((0, n, LEAF)));
            }
        }
        while let Some(Reverse((r, node, inst))) = heap.pop() {
            let at = local(g, node);
            if rank[at].is_some() {
                continue;
            }
            rank[at] = Some(r);
            kept[at] = (inst != LEAF).then_some(inst);
            for &j in &uses[at] {
                let j = j as usize;
                waiting[j] -= 1;
                if waiting[j] == 0 {
                    let i = instances[j];
                    let inst = &g.instances[i];
                    let highest = inst.premises.iter().map(|&p| rank[local(g, p)].expect("ranked")).max();
                    let r = highest.unwrap_or(0) + u32::from(step(g, inst.head));
                    heap.push(Reverse((r, inst.head, i)));
                }
            }
        }
        Ranks {
            epoch,
            whole,
            rank,
            kept,
        }
    }

    /// The closure triples instance `i`'s premises stand for: its triples,
    /// and for each helper, the triples of its kept instance, recursively,
    /// each once, in order. Depth first, a helper met again adding nothing
    /// (the kept instances are acyclic, so its triples are all out already):
    /// a long list's cells, a long chain of helpers, are walked once, not
    /// copied from each cell's tail. `None` when the cancel handle tripped.
    fn folded(&self, i: usize, kept: &Ranks, checks: &mut Checks<'_>) -> Option<Vec<u32>> {
        let mut out = Vec::new();
        // The triples and the helpers met.
        let mut seen = FxHashSet::default();
        let mut stack = vec![self.g.instances[i].premises.iter()];
        while let Some(top) = stack.last_mut() {
            let Some(&p) = top.next() else {
                stack.pop();
                continue;
            };
            if !seen.insert(p) {
                continue;
            }
            if self.g.nodes[p as usize].rel == T {
                out.push(p);
                continue;
            }
            if checks.tick() {
                return None;
            }
            let h = self.g.kept(kept, p).expect("a premise is ranked");
            stack.push(self.g.instances[h].premises.iter());
        }
        Some(out)
    }

    /// The proof of `goal` from the kept instances ([`Search::built`]), or
    /// `None` when the cancel handle tripped, which halts the search.
    fn build(&mut self, goal: u32, kept: &Ranks) -> Option<Built> {
        let built = self.built(goal, kept);
        self.halted |= built.is_none();
        built
    }

    /// The proof of `goal` from the kept instances: the closure triples in
    /// post-order (each premise before the steps that read it), the goal
    /// last, or the finding's root apart. `None` when the cancel handle
    /// tripped: a proof through a long list has many steps.
    fn built(&self, goal: u32, kept: &Ranks) -> Option<Built> {
        let mut checks = Checks::new(self.world.cancel);
        let mut index: FxHashMap<u32, usize> = FxHashMap::default();
        let mut steps: Vec<RawStep> = Vec::new();
        let mut children: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
        let finding = self.g.nodes[goal as usize].rel != T;
        let roots: Vec<u32> = if finding {
            self.folded(self.g.kept(kept, goal).expect("ranked"), kept, &mut checks)?
        } else {
            vec![goal]
        };
        for &root in &roots {
            let mut stack = vec![(root, false)];
            while let Some((x, expanded)) = stack.pop() {
                if index.contains_key(&x) {
                    continue;
                }
                if checks.tick() {
                    return None;
                }
                let premises = match children.get(&x) {
                    Some(premises) => premises.clone(),
                    None => {
                        let premises = match self.g.kept(kept, x) {
                            Some(i) => self.folded(i, kept, &mut checks)?,
                            None => Vec::new(),
                        };
                        children.insert(x, premises.clone());
                        premises
                    }
                };
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
                let rule = self.g.kept(kept, x).map(|i| {
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
            let i = self.g.kept(kept, goal).expect("ranked");
            let rule = self.world.program.rules[self.g.instances[i].rule]
                .name
                .expect("a finding's rule has a name");
            (rule, roots.iter().map(|p| index[p]).collect())
        });
        Some(Built { steps, root })
    }
}

/// The work a proof's building does between two checks of the cancel
/// handle ([`Checks`]): a helper unfolded or a step visited.
const BUILD_CHECK: u32 = 1024;

/// Checks the cancel handle while a proof is built, once per
/// [`BUILD_CHECK`] units of work.
struct Checks<'a> {
    cancel: Option<&'a Cancel>,
    work: u32,
}

impl<'a> Checks<'a> {
    fn new(cancel: Option<&'a Cancel>) -> Self {
        Self { cancel, work: 0 }
    }

    /// Counts a unit of work: true when the handle tripped.
    fn tick(&mut self) -> bool {
        self.work += 1;
        if self.work < BUILD_CHECK {
            return false;
        }
        self.work = 0;
        self.cancel.is_some_and(Cancel::check)
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
