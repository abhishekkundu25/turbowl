use super::*;

impl Store {
    pub(crate) fn clear_delta(&mut self) {
        for t in std::mem::take(&mut self.active) {
            self.tables[t].so.changed();
            if let Some(os) = &mut self.tables[t].os {
                os.changed();
            }
            self.is_active[t] = false;
        }
        self.deleting = false;
    }

    pub(crate) fn deletion_delta(&mut self, triples: &[[u32; 3]]) {
        self.clear_delta();
        self.deleting = true;
        let mut by_predicate: FxHashMap<u32, Vec<Pair>> = FxHashMap::default();
        for &[s, p, o] in triples {
            by_predicate.entry(p).or_default().push((s, o));
        }
        for (p, pairs) in by_predicate {
            let t = self.index[&p];
            if let Some(os) = &mut self.tables[t].os {
                os.select_recent(pairs.iter().map(|&(s, o)| (o, s)).collect());
            }
            self.tables[t].so.select_recent(pairs);
            self.activate(t);
        }
    }

    pub(crate) fn remove_triples(&mut self, triples: &[[u32; 3]]) {
        self.clear_delta();
        let mut by_predicate: FxHashMap<u32, Vec<Pair>> = FxHashMap::default();
        for &[s, p, o] in triples {
            by_predicate.entry(p).or_default().push((s, o));
        }
        for (p, pairs) in by_predicate {
            let pairs = Relation::from_vec(pairs);
            let t = self.index[&p];
            self.tables[t].so.remove(&pairs);
            if let Some(os) = &mut self.tables[t].os {
                let flipped = Relation::from_iter(pairs.iter().map(|&(s, o)| (o, s)));
                os.remove(&flipped);
            }
        }
    }

    pub(crate) fn all_recent(&mut self) {
        self.clear_delta();
        for t in 0..self.tables.len() {
            if self.tables[t].so.is_empty() {
                continue;
            }
            self.tables[t].so.all_recent();
            if let Some(os) = &mut self.tables[t].os {
                os.all_recent();
            }
            self.activate(t);
        }
    }

    /// Evaluates rules without inserting their heads or applying merges.
    /// A missing object index is built and the round repeated before returning.
    pub(crate) fn evaluate(&mut self, mut rules: impl FnMut(&mut Round<'_>)) -> Option<Vec<[u32; 3]>> {
        let mut heads = Vec::new();
        loop {
            let mut pending = Pending::default();
            self.rounds += 1;
            rules(&mut Round {
                store: self,
                pending: &mut pending,
                parallel: false,
            });
            if pending.aborted
                || pending
                    .merges
                    .iter()
                    .any(|&(a, b)| self.canon.find(a) != self.canon.find(b))
            {
                return None;
            }
            heads.extend(
                pending
                    .triples()
                    .map(|(s, p, o)| [self.canon.find(s), self.canon.find(p), self.canon.find(o)]),
            );
            let mut built = false;
            for p in pending.by_object {
                let Some(&t) = self.index.get(&p) else { continue };
                if self.tables[t].os.is_none() && !self.tables[t].so.is_empty() {
                    let pairs = self.tables[t].so.parts().collect::<Vec<_>>();
                    let mut os = Variable::new();
                    os.insert(flipped(&pairs, None));
                    os.changed();
                    self.tables[t].os = Some(os);
                    self.activate(t);
                    built = true;
                }
            }
            if !built {
                return Some(heads);
            }
        }
    }
}
