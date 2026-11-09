use super::*;
use crate::rules::backward::Backward;
use crate::rules::program::{Compiled, program};
use crate::rules::query::Query;

impl Reasoner {
    pub(super) fn queue_removals(&mut self, gone: Vec<[u32; 3]>) {
        #[cfg(feature = "io")]
        self.loaded.clear();
        if matches!(self.state, State::Current | State::Removed) {
            self.removed.extend(gone);
            self.state = State::Removed;
        } else {
            self.restart();
        }
    }

    fn deletion_is_safe(&self, triple: &[u32; 3]) -> bool {
        triple[1] != vocab::OWL_SAME_AS.0
            && triple.iter().all(|&t| {
                let canon = self.closure.canon();
                canon.members(canon.find(t)).len() == 1
            })
    }

    pub(super) fn maintain_removals(&mut self, families: &[&'static Family], cancel: Option<&Cancel>) -> Option<bool> {
        self.counters.deletion.removed = self.removed.len();
        if self.switches.is_on(Switch::Trivial) || self.switches.is_on(Switch::LiteralValues) {
            return fallback(&mut self.counters, "configuration");
        }
        let limit = (self.base.len() / 8).clamp(16, 4096);
        self.removed.sort_unstable();
        self.removed.dedup();
        if self.removed.len() > limit || self.removed.iter().any(|t| !self.deletion_is_safe(t)) {
            return fallback(&mut self.counters, "batch_or_equality");
        }
        let axioms: FxHashSet<[u32; 3]> = families
            .iter()
            .flat_map(|f| f.facts)
            .map(|&[s, p, o]| [s.0, p.0, o.0])
            .collect();
        let compiled = std::ptr::eq(self.families, FAMILIES).then(|| program().compile(self.switches, &self.terms));
        let mut backward = compiled.as_ref().map(Backward::new);
        let mut delta: Vec<_> = self
            .removed
            .iter()
            .copied()
            .filter(|t| !axioms.contains(t))
            .filter(|&t| {
                !backward
                    .as_mut()
                    .is_some_and(|b| b.proves(&self.closure, &|t| self.base.contains(t), &axioms, t, cancel))
            })
            .collect();
        if delta.is_empty() {
            return Some(true);
        }
        if self.closure.canon().is_empty()
            && let Some(program) = &compiled
        {
            return self.maintain_program(families, program, &axioms, backward.as_mut().unwrap(), delta, cancel);
        }
        let mut affected: FxHashSet<[u32; 3]> = delta.iter().copied().collect();
        let work_limit = (self.closure.len() / 4).max(64);
        while !delta.is_empty() {
            if cancel.is_some_and(Cancel::check) {
                return None;
            }
            self.closure.deletion_delta(&delta);
            let Some(heads) = self.removal_round(families, &mut Vec::new(), false) else {
                return fallback(&mut self.counters, "native_evaluation");
            };
            delta.clear();
            for t in heads {
                if !self.closure.contains(t[0], t[1], t[2]) {
                    continue;
                }
                if !self.deletion_is_safe(&t) {
                    return fallback(&mut self.counters, "equality");
                }
                if affected.contains(&t) || self.base.contains(&t) || axioms.contains(&t) {
                    continue;
                }
                if backward
                    .as_mut()
                    .is_some_and(|b| b.proves(&self.closure, &|t| self.base.contains(t), &axioms, t, cancel))
                {
                    continue;
                }
                affected.insert(t);
                delta.push(t);
            }
            if affected.len() > work_limit {
                return fallback(&mut self.counters, "affected_set");
            }
        }
        if cancel.is_some_and(Cancel::check) {
            return None;
        }
        let mut affected: Vec<_> = affected.into_iter().collect();
        affected.sort_unstable();
        self.counters.deletion.overdeleted = affected.len();
        self.closure.remove_triples(&affected);
        let lost = affected.iter().filter(|t| self.is_rdf(t[0], t[1])).count();
        let Some(rdf) = self.rdf.checked_sub(lost) else {
            return fallback(&mut self.counters, "invalid_counts");
        };
        self.rdf = rdf;
        self.closure.all_recent();
        let mut findings = Vec::new();
        let Some(heads) = self.removal_round(families, &mut findings, false) else {
            return fallback(&mut self.counters, "native_evaluation");
        };
        self.closure.clear_delta();
        if self.switches.is_on(Switch::Checks) {
            datatype::recheck_class_literals(&self.closure, &self.terms, &mut findings);
        }
        self.findings = findings;
        let restored: Vec<_> = heads
            .into_iter()
            .filter(|t| affected.binary_search(t).is_ok())
            .collect();
        self.counters.deletion.restored = restored.len();
        self.closure.extend(restored.into_iter().map(|[s, p, o]| (s, p, o)));
        Some(true)
    }

    fn removal_round(
        &mut self,
        families: &[&'static Family],
        findings: &mut Vec<Finding>,
        findings_only: bool,
    ) -> Option<Vec<[u32; 3]>> {
        let Self {
            closure,
            counters,
            terms,
            switches,
            ..
        } = self;
        closure.evaluate(|round| {
            let start = Clock::now();
            counters
                .delta
                .push(round.store().tables().map(|(_, t)| t.by_subject().recent().len()).sum());
            for family in families {
                let before = round.emitted();
                let begin = Clock::now();
                if !findings_only || matches!(family.name, "checks" | "datatype") {
                    (family.rules)(
                        round,
                        &mut Context {
                            terms,
                            switches: *switches,
                            findings,
                        },
                    );
                }
                counters.emitted.push(round.emitted() - before);
                counters.family_ns.push(begin.elapsed_ns());
            }
            counters.round_ns.push(start.elapsed_ns());
        })
    }

    fn maintain_program(
        &mut self,
        families: &[&'static Family],
        program: &Compiled,
        axioms: &FxHashSet<[u32; 3]>,
        backward: &mut Backward,
        mut delta: Vec<[u32; 3]>,
        cancel: Option<&Cancel>,
    ) -> Option<bool> {
        let mut affected: FxHashSet<_> = delta.iter().copied().collect();
        let limit = (self.closure.len() / 4).max(64);
        for t in delta.clone() {
            if t[1] == vocab::RDF_TYPE.0 && !self.enclose_types(t[0], axioms, &mut affected, &mut delta, limit) {
                return fallback(&mut self.counters, "affected_set");
            }
        }
        if affected.len() > limit {
            return fallback(&mut self.counters, "affected_set");
        }
        {
            let mut query = Query::new(program, &self.closure, cancel);
            while !delta.is_empty() {
                if cancel.is_some_and(Cancel::check) {
                    return None;
                }
                let start = Clock::now();
                let heads = match query.dependents(&delta, &|t| {
                    affected.contains(t) || self.base.contains(t) || axioms.contains(t)
                }) {
                    Ok(heads) => heads,
                    Err(reason) => return fallback(&mut self.counters, reason),
                };
                record_query(&mut self.counters, delta.len(), start.elapsed_ns());
                delta.clear();
                for t in heads {
                    if !self.closure.contains(t[0], t[1], t[2]) {
                        continue;
                    }
                    if !self.deletion_is_safe(&t) {
                        return fallback(&mut self.counters, "equality");
                    }
                    if affected.contains(&t) || self.base.contains(&t) || axioms.contains(&t) {
                        continue;
                    }
                    if backward.proves(&self.closure, &|t| self.base.contains(t), axioms, t, cancel) {
                        continue;
                    }
                    affected.insert(t);
                    delta.push(t);
                    if t[1] == vocab::RDF_TYPE.0 && !self.enclose_types(t[0], axioms, &mut affected, &mut delta, limit)
                    {
                        return fallback(&mut self.counters, "affected_set");
                    }
                }
                if affected.len() > limit {
                    return fallback(&mut self.counters, "affected_set");
                }
            }
        }
        let mut affected: Vec<_> = affected.into_iter().collect();
        affected.sort_unstable();
        self.counters.deletion.overdeleted = affected.len();
        self.closure.remove_triples(&affected);
        let lost = affected.iter().filter(|t| self.is_rdf(t[0], t[1])).count();
        let Some(rdf) = self.rdf.checked_sub(lost) else {
            return fallback(&mut self.counters, "invalid_counts");
        };
        self.rdf = rdf;
        let start = Clock::now();
        let mut restored = Vec::new();
        {
            let mut query = Query::new(program, &self.closure, cancel);
            for &t in &affected {
                match query.supports(t) {
                    Ok(true) => restored.push(t),
                    Ok(false) => (),
                    Err(reason) => return fallback(&mut self.counters, reason),
                }
            }
        }
        record_query(&mut self.counters, affected.len(), start.elapsed_ns());
        // Deletion cannot create a finding in this positive rule program.
        if !self.findings.is_empty() {
            self.closure.all_recent();
            let mut findings = Vec::new();
            if self.removal_round(families, &mut findings, true).is_none() {
                return fallback(&mut self.counters, "native_evaluation");
            }
            self.closure.clear_delta();
            self.findings = findings;
        }
        self.counters.deletion.restored = restored.len();
        self.closure.extend(restored.into_iter().map(|[s, p, o]| (s, p, o)));
        Some(true)
    }

    fn enclose_types(
        &self,
        subject: u32,
        axioms: &FxHashSet<[u32; 3]>,
        affected: &mut FxHashSet<[u32; 3]>,
        delta: &mut Vec<[u32; 3]>,
        limit: usize,
    ) -> bool {
        let p = vocab::RDF_TYPE.0;
        let Some(table) = self.closure.table(p) else {
            return true;
        };
        for part in table.by_subject().parts() {
            let from = part.partition_point(|t| t.0 < subject);
            for &(s, o) in part[from..].iter().take_while(|t| t.0 == subject) {
                let t = [s, p, o];
                if !self.base.contains(&t) && !axioms.contains(&t) && affected.insert(t) {
                    delta.push(t);
                    if affected.len() > limit {
                        return false;
                    }
                }
            }
        }
        true
    }
}

fn fallback(counters: &mut Counters, reason: &'static str) -> Option<bool> {
    counters.deletion.fallback = Some(reason);
    Some(false)
}

fn record_query(counters: &mut Counters, delta: usize, ns: u64) {
    counters.delta.push(delta);
    counters.round_ns.push(ns);
    counters.emitted.extend(std::iter::repeat_n(0, counters.families.len()));
    counters
        .family_ns
        .extend(std::iter::repeat_n(0, counters.families.len()));
}
