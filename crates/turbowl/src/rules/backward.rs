use super::program::{Arg, Body, Compiled, Rule, T};
use crate::Cancel;
use crate::engine::Store;
use crate::term::vocab;
use rustc_hash::FxHashSet;

const WORK: usize = 1024;
const DEPTH: usize = 64;

#[derive(Debug)]
pub(crate) struct Backward {
    rules: Vec<Rule>,
    proved: FxHashSet<[u32; 3]>,
    left: usize,
}

struct Search<'a> {
    store: &'a Store,
    rules: &'a [Rule],
    base: &'a dyn Fn(&[u32; 3]) -> bool,
    axioms: &'a FxHashSet<[u32; 3]>,
    proved: &'a mut FxHashSet<[u32; 3]>,
    checked: FxHashSet<[u32; 3]>,
    left: &'a mut usize,
    cancel: Option<&'a Cancel>,
}

impl Backward {
    pub(crate) fn new(program: &Compiled) -> Self {
        let rules = program
            .rules
            .iter()
            .filter(|r| {
                r.head.rel == T
                    && !r.name.is_some_and(|n| n.starts_with("eq-"))
                    && r.body.len() < 64
                    && r.body.iter().all(|b| match b {
                        Body::Atom(a) => a.rel == T,
                        Body::NotEqual(_, _) => true,
                    })
            })
            .cloned()
            .collect();
        Self {
            rules,
            proved: FxHashSet::default(),
            left: WORK,
        }
    }

    /// Only a grounded proof returns true. Exhaustion or unsupported helper
    /// relations leave the fact to DRed.
    pub(crate) fn proves(
        &mut self,
        store: &Store,
        base: &dyn Fn(&[u32; 3]) -> bool,
        axioms: &FxHashSet<[u32; 3]>,
        goal: [u32; 3],
        cancel: Option<&Cancel>,
    ) -> bool {
        if base(&goal) || axioms.contains(&goal) || self.proved.contains(&goal) {
            return true;
        }
        if self.left == 0 {
            return false;
        }
        Search {
            store,
            rules: &self.rules,
            base,
            axioms,
            proved: &mut self.proved,
            checked: FxHashSet::default(),
            left: &mut self.left,
            cancel,
        }
        .prove(goal, 0)
    }
}

fn value(arg: Arg, bindings: &[Option<u32>]) -> Option<u32> {
    match arg {
        Arg::Id(id) => Some(id),
        Arg::Var(v) => bindings[v as usize],
    }
}

fn bind(args: &[Arg], triple: [u32; 3], bindings: &mut [Option<u32>]) -> bool {
    for (&arg, id) in args.iter().zip(triple) {
        match arg {
            Arg::Id(want) if want != id => return false,
            Arg::Var(v) => match &mut bindings[v as usize] {
                Some(want) if *want != id => return false,
                slot => *slot = Some(id),
            },
            _ => (),
        }
    }
    true
}

impl Search<'_> {
    fn spend(&mut self) -> bool {
        if *self.left == 0 || ((*self.left).is_multiple_of(64) && self.cancel.is_some_and(Cancel::check)) {
            return false;
        }
        *self.left -= 1;
        true
    }

    fn prove(&mut self, goal: [u32; 3], depth: usize) -> bool {
        if (self.base)(&goal) || self.axioms.contains(&goal) || self.proved.contains(&goal) {
            return true;
        }
        if depth >= DEPTH
            || !self.spend()
            || goal[1] == vocab::OWL_SAME_AS.0
            || !self.store.contains(goal[0], goal[1], goal[2])
            || !self.checked.insert(goal)
        {
            return false;
        }
        let mut proved = false;
        for rule in self.rules {
            let mut bindings = vec![None; rule.vars];
            if bind(&rule.head.args, goal, &mut bindings) && self.join(rule, bindings, 0, depth + 1) {
                proved = true;
                break;
            }
        }
        self.checked.remove(&goal);
        if proved {
            self.proved.insert(goal);
        }
        proved
    }

    fn join(&mut self, rule: &Rule, bindings: Vec<Option<u32>>, done: u64, depth: usize) -> bool {
        if done.count_ones() as usize == rule.body.len() {
            return true;
        }
        if !self.spend() {
            return false;
        }
        let next = rule
            .body
            .iter()
            .enumerate()
            .filter(|(i, _)| done & (1 << i) == 0)
            .filter_map(|(i, body)| match body {
                Body::NotEqual(a, b) => {
                    (value(*a, &bindings).is_some() && value(*b, &bindings).is_some()).then_some((i, 4))
                }
                Body::Atom(a) => Some((i, a.args.iter().filter(|&&a| value(a, &bindings).is_some()).count())),
            })
            .max_by_key(|&(_, score)| score)
            .map(|(i, _)| i);
        let Some(i) = next else { return false };
        let done = done | (1 << i);
        match &rule.body[i] {
            Body::NotEqual(a, b) => {
                value(*a, &bindings) != value(*b, &bindings) && self.join(rule, bindings, done, depth)
            }
            Body::Atom(atom) => {
                let pattern = std::array::from_fn(|i| value(atom.args[i], &bindings));
                let mut matches = self.matches(pattern);
                matches.sort_by_key(|t| !((self.base)(t) || self.axioms.contains(t) || self.proved.contains(t)));
                for triple in matches {
                    let mut next = bindings.clone();
                    if !bind(&atom.args, triple, &mut next) {
                        continue;
                    }
                    if self.prove(triple, depth) && self.join(rule, next, done, depth) {
                        return true;
                    }
                }
                false
            }
        }
    }

    fn matches(&mut self, pattern: [Option<u32>; 3]) -> Vec<[u32; 3]> {
        if let [Some(s), Some(p), Some(o)] = pattern {
            return if self.store.contains(s, p, o) {
                vec![[s, p, o]]
            } else {
                Vec::new()
            };
        }
        let mut out = Vec::new();
        for (p, table) in self.store.tables() {
            if pattern[1].is_some_and(|want| want != p) {
                continue;
            }
            let reversed = pattern[0].is_none() && pattern[2].is_some() && table.by_object().is_some();
            let relation = if reversed {
                table.by_object().unwrap()
            } else {
                table.by_subject()
            };
            let key = if reversed { pattern[2] } else { pattern[0] };
            for part in relation.parts() {
                let from = key.map_or(0, |key| part.partition_point(|t| t.0 < key));
                for &(a, b) in &part[from..] {
                    if key.is_some_and(|key| key != a) {
                        break;
                    }
                    if !self.spend() {
                        return out;
                    }
                    let (s, o) = if reversed { (b, a) } else { (a, b) };
                    if pattern[0].is_none_or(|want| want == s) && pattern[2].is_none_or(|want| want == o) {
                        out.push([s, p, o]);
                        if out.len() == 16 {
                            return out;
                        }
                    }
                }
            }
        }
        out
    }
}
