//! Bounded maintenance queries over one immutable closure. Helper calls
//! reach a fixpoint before their answers are cached for later queries.

use super::program::{Arg, Body, Compiled, RelId, Rule, T};
use crate::Cancel;
use crate::engine::Store;
use crate::term::vocab;
use rustc_hash::{FxHashMap, FxHashSet};

type Fact = (RelId, Vec<u32>);
type Call = (RelId, Vec<Option<u32>>);

#[derive(Default)]
struct Answers {
    rows: Vec<Vec<u32>>,
    complete: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Projection {
    Input(usize),
    Id(u32),
    Any,
}

pub(crate) struct Query<'a> {
    program: &'a Compiled,
    store: &'a Store,
    cancel: Option<&'a Cancel>,
    cache: FxHashMap<Call, Answers>,
    pending: Vec<Call>,
    building: bool,
    deleted_helpers: FxHashSet<Fact>,
    left: usize,
    live: Vec<bool>,
    objects: FxHashMap<u32, Vec<(u32, u32)>>,
    exits: Vec<Vec<[Projection; 3]>>,
    covered: FxHashSet<(u32, u32)>,
}

impl<'a> Query<'a> {
    pub(crate) fn new(program: &'a Compiled, store: &'a Store, cancel: Option<&'a Cancel>) -> Self {
        let mut live = vec![false; program.relations.len()];
        live[T as usize] = true;
        loop {
            let mut changed = false;
            for rule in &program.rules {
                if !live[rule.head.rel as usize] {
                    continue;
                }
                for body in &rule.body {
                    if let Body::Atom(atom) = body
                        && !live[atom.rel as usize]
                    {
                        live[atom.rel as usize] = true;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        Self {
            program,
            store,
            cancel,
            cache: FxHashMap::default(),
            pending: Vec::new(),
            building: false,
            deleted_helpers: FxHashSet::default(),
            left: 100_000,
            live,
            objects: FxHashMap::default(),
            exits: Vec::new(),
            covered: FxHashSet::default(),
        }
    }

    fn spend(&mut self) -> Result<(), &'static str> {
        if self.left == 0 || (self.left.is_multiple_of(64) && self.cancel.is_some_and(Cancel::check)) {
            return Err("query_limit");
        }
        self.left -= 1;
        Ok(())
    }

    pub(crate) fn dependents(
        &mut self,
        delta: &[[u32; 3]],
        covered: &dyn Fn(&[u32; 3]) -> bool,
    ) -> Result<Vec<[u32; 3]>, &'static str> {
        if self.exits.is_empty() {
            self.exits = projections(self.program);
        }
        let mut agenda: Vec<Fact> = delta.iter().map(|t| (T, t.to_vec())).collect();
        let mut triples = Vec::new();
        while let Some((rel, tuple)) = agenda.pop() {
            self.spend()?;
            for rule in &self.program.rules {
                if !self.live[rule.head.rel as usize] {
                    continue;
                }
                for (i, body) in rule.body.iter().enumerate() {
                    let Body::Atom(atom) = body else { continue };
                    if atom.rel != rel {
                        continue;
                    }
                    let mut bindings = vec![None; rule.vars];
                    if !bind(&atom.args, &tuple, &mut bindings) {
                        continue;
                    }
                    if self.rule_covered(rule, &bindings, covered)? {
                        continue;
                    }
                    for head in self.join(rule, bindings, 1 << i)? {
                        if rule.head.rel == T {
                            triples.push([head[0], head[1], head[2]]);
                        } else if self.deleted_helpers.insert((rule.head.rel, head.clone())) {
                            agenda.push((rule.head.rel, head));
                        }
                    }
                }
            }
        }
        triples.sort_unstable();
        triples.dedup();
        Ok(triples)
    }

    pub(crate) fn supports(&mut self, goal: [u32; 3]) -> Result<bool, &'static str> {
        for &i in &self.program.by_head[T as usize] {
            let rule = &self.program.rules[i];
            let mut bindings = vec![None; rule.vars];
            if bind(&rule.head.args, &goal, &mut bindings) && !self.join(rule, bindings, 0)?.is_empty() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn answers(&mut self, rel: RelId, pattern: Vec<Option<u32>>) -> Result<Vec<Vec<u32>>, &'static str> {
        self.spend()?;
        if rel == T {
            return self.triples(&pattern);
        }
        let call = (rel, pattern);
        if let Some(answers) = self.cache.get(&call) {
            return Ok(answers.rows.clone());
        }
        if matches!(
            self.program.relations[rel as usize].kind,
            super::program::Kind::Builtin(_)
        ) {
            return Err("builtin_query");
        }
        self.cache.insert(call.clone(), Answers::default());
        self.pending.push(call.clone());
        if self.building {
            return Ok(Vec::new());
        }
        self.building = true;
        loop {
            self.spend()?;
            let count = self.pending.len();
            let mut changed = false;
            for index in 0..count {
                let key = self.pending[index].clone();
                let mut answers = Vec::new();
                for &i in &self.program.by_head[key.0 as usize] {
                    let rule = &self.program.rules[i];
                    let mut bindings = vec![None; rule.vars];
                    if rule
                        .head
                        .args
                        .iter()
                        .zip(&key.1)
                        .all(|(&arg, &id)| id.is_none_or(|id| bind(&[arg], &[id], &mut bindings)))
                    {
                        answers.extend(self.join(rule, bindings, 0)?);
                    }
                }
                let current = &mut self.cache.get_mut(&key).unwrap().rows;
                let before = current.len();
                current.extend(answers);
                current.sort_unstable();
                current.dedup();
                changed |= before != current.len();
            }
            if !changed && self.pending.len() == count {
                break;
            }
        }
        for key in std::mem::take(&mut self.pending) {
            self.cache.get_mut(&key).unwrap().complete = true;
        }
        self.building = false;
        Ok(self.cache[&call].rows.clone())
    }

    fn join(&mut self, rule: &Rule, bindings: Vec<Option<u32>>, done: u64) -> Result<Vec<Vec<u32>>, &'static str> {
        self.spend()?;
        if done.count_ones() as usize == rule.body.len() {
            let head = rule
                .head
                .args
                .iter()
                .map(|&a| value(a, &bindings).ok_or("unbound_query"))
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(vec![head]);
        }
        let next = rule
            .body
            .iter()
            .enumerate()
            .filter(|(i, _)| done & (1 << i) == 0)
            .filter_map(|(i, body)| match body {
                Body::NotEqual(a, b) => {
                    (value(*a, &bindings).is_some() && value(*b, &bindings).is_some()).then_some((i, 0))
                }
                Body::Atom(a) => {
                    let pattern: Vec<_> = a.args.iter().map(|&a| value(a, &bindings)).collect();
                    let cost = if a.rel == T {
                        self.estimate(&pattern)
                    } else {
                        self.cache
                            .get(&(a.rel, pattern.clone()))
                            .filter(|answers| answers.complete || !answers.rows.is_empty())
                            .map_or(256 / (1 + pattern.iter().filter(|a| a.is_some()).count()), |a| {
                                a.rows.len()
                            })
                    };
                    Some((i, cost))
                }
            })
            .min_by_key(|&(_, cost)| cost)
            .map(|(i, _)| i)
            .ok_or("unbound_query")?;
        let done = done | (1 << next);
        match &rule.body[next] {
            Body::NotEqual(a, b) => {
                if value(*a, &bindings) != value(*b, &bindings) {
                    self.join(rule, bindings, done)
                } else {
                    Ok(Vec::new())
                }
            }
            Body::Atom(atom) => {
                let pattern = atom.args.iter().map(|&a| value(a, &bindings)).collect();
                let mut out = Vec::new();
                for tuple in self.answers(atom.rel, pattern)? {
                    let mut next = bindings.clone();
                    if bind(&atom.args, &tuple, &mut next) {
                        out.extend(self.join(rule, next, done)?);
                    }
                }
                Ok(out)
            }
        }
    }

    fn triples(&mut self, pattern: &[Option<u32>]) -> Result<Vec<Vec<u32>>, &'static str> {
        if let [Some(s), Some(p), Some(o)] = *pattern {
            return Ok(if self.store.contains(s, p, o) {
                vec![vec![s, p, o]]
            } else {
                Vec::new()
            });
        }
        if let [None, Some(p), Some(_)] = *pattern {
            self.object_index(p);
        }
        let mut out = Vec::new();
        for (p, table) in self.store.tables() {
            if pattern[1].is_some_and(|want| want != p) {
                continue;
            }
            if pattern[0].is_none()
                && let Some(o) = pattern[2]
                && let Some(index) = self.objects.get(&p)
            {
                let from = index.partition_point(|t| t.0 < o);
                let len = index[from..].partition_point(|t| t.0 == o);
                if len > self.left {
                    return Err("query_limit");
                }
                self.left -= len;
                out.extend(index[from..from + len].iter().map(|&(o, s)| vec![s, p, o]));
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
                    self.spend()?;
                    let (s, o) = if reversed { (b, a) } else { (a, b) };
                    if pattern[0].is_none_or(|want| want == s) && pattern[2].is_none_or(|want| want == o) {
                        out.push(vec![s, p, o]);
                    }
                }
            }
        }
        Ok(out)
    }

    fn estimate(&mut self, pattern: &[Option<u32>]) -> usize {
        let Some(p) = pattern[1] else {
            return self.store.len();
        };
        let Some(table) = self.store.table(p) else {
            return 0;
        };
        if let (Some(s), Some(o)) = (pattern[0], pattern[2]) {
            return usize::from(table.by_subject().contains(&(s, o)));
        }
        if pattern[0].is_none()
            && let Some(o) = pattern[2]
            && table.by_object().is_none()
        {
            self.object_index(p);
            if let Some(index) = self.objects.get(&p) {
                let from = index.partition_point(|t| t.0 < o);
                return index[from..].partition_point(|t| t.0 == o);
            }
        }
        let (relation, key) = if let Some(s) = pattern[0] {
            (table.by_subject(), Some(s))
        } else if let Some(o) = pattern[2]
            && let Some(os) = table.by_object()
        {
            (os, Some(o))
        } else {
            (table.by_subject(), None)
        };
        key.map_or_else(
            || relation.len(),
            |key| {
                relation
                    .parts()
                    .map(|part| {
                        let from = part.partition_point(|t| t.0 < key);
                        part[from..].partition_point(|t| t.0 == key)
                    })
                    .sum()
            },
        )
    }

    fn object_index(&mut self, p: u32) {
        if self.objects.contains_key(&p) {
            return;
        }
        let Some(table) = self.store.table(p) else {
            return;
        };
        if table.by_object().is_some() || table.by_subject().len() > self.left {
            return;
        }
        let mut index: Vec<_> = table.by_subject().all().iter().map(|(s, o)| (o, s)).collect();
        self.left -= index.len();
        index.sort_unstable();
        self.objects.insert(p, index);
    }

    fn rule_covered(
        &mut self,
        rule: &Rule,
        bindings: &[Option<u32>],
        covered: &dyn Fn(&[u32; 3]) -> bool,
    ) -> Result<bool, &'static str> {
        let patterns = self.exits[rule.head.rel as usize].clone();
        if patterns.is_empty() || !self.store.canon().is_empty() {
            return Ok(false);
        }
        for pattern in patterns {
            let bound = pattern.map(|p| match p {
                Projection::Input(i) => value(rule.head.args[i], bindings),
                Projection::Id(id) => Some(id),
                Projection::Any => None,
            });
            let [Some(s), Some(p), _] = bound else { return Ok(false) };
            if p == vocab::OWL_SAME_AS.0 {
                return Ok(false);
            }
            if self.covered.contains(&(s, p)) {
                continue;
            }
            if let Some(table) = self.store.table(p) {
                for part in table.by_subject().parts() {
                    let from = part.partition_point(|t| t.0 < s);
                    for &(_, o) in part[from..].iter().take_while(|t| t.0 == s) {
                        self.spend()?;
                        if !covered(&[s, p, o]) {
                            return Ok(false);
                        }
                    }
                }
            }
            self.covered.insert((s, p));
        }
        Ok(true)
    }
}

fn projections(program: &Compiled) -> Vec<Vec<[Projection; 3]>> {
    // Stop at T: its affected tuples are queued separately for propagation.
    use Projection::{Any, Id, Input};
    let mut exits = vec![Vec::new(); program.relations.len()];
    exits[T as usize].push([Input(0), Input(1), Input(2)]);
    loop {
        let mut changed = false;
        for rule in &program.rules {
            for body in &rule.body {
                let Body::Atom(atom) = body else { continue };
                if atom.rel == T || exits[atom.rel as usize].contains(&[Any; 3]) {
                    continue;
                }
                for pattern in exits[rule.head.rel as usize].clone() {
                    let mapped = pattern.map(|p| match p {
                        Input(i) => match rule.head.args[i] {
                            Arg::Id(id) => Id(id),
                            Arg::Var(v) => atom.args.iter().position(|a| *a == Arg::Var(v)).map_or(Any, Input),
                        },
                        other => other,
                    });
                    let out = &mut exits[atom.rel as usize];
                    if !out.contains(&mapped) {
                        if out.len() >= 64 || mapped == [Any; 3] {
                            *out = vec![[Any; 3]];
                        } else {
                            out.push(mapped);
                        }
                        changed = true;
                    }
                    if out.contains(&[Any; 3]) {
                        break;
                    }
                }
            }
        }
        if !changed {
            return exits;
        }
    }
}

fn value(arg: Arg, bindings: &[Option<u32>]) -> Option<u32> {
    match arg {
        Arg::Id(id) => Some(id),
        Arg::Var(v) => bindings[v as usize],
    }
}

fn bind(args: &[Arg], tuple: &[u32], bindings: &mut [Option<u32>]) -> bool {
    args.iter().zip(tuple).all(|(&arg, &id)| match arg {
        Arg::Id(want) => want == id,
        Arg::Var(v) => match &mut bindings[v as usize] {
            Some(want) => *want == id,
            slot => {
                *slot = Some(id);
                true
            }
        },
    })
}
