//! Comparing two closures as sets, with a readable diff.

use crate::Closure;
use std::fmt;

/// What two closures are expected to satisfy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// The same triples and the same findings.
    Equal,
    /// Every triple and finding of the left is in the right (the left is sound
    /// with respect to the right).
    Subset,
}

/// The difference between two closures.
#[derive(Debug, Clone)]
pub struct Diff {
    pub left: String,
    pub right: String,
    pub relation: Relation,
    /// N-Triples lines only the left has, sorted.
    pub only_left: Vec<String>,
    /// N-Triples lines only the right has, sorted.
    pub only_right: Vec<String>,
    pub findings_only_left: Vec<String>,
    pub findings_only_right: Vec<String>,
}

impl Diff {
    /// True when the relation holds.
    pub fn holds(&self) -> bool {
        let left_ok = self.only_left.is_empty() && self.findings_only_left.is_empty();
        match self.relation {
            Relation::Subset => left_ok,
            Relation::Equal => left_ok && self.only_right.is_empty() && self.findings_only_right.is_empty(),
        }
    }

    /// A one-line summary: `=`, `⊆`, or the counts on each side.
    pub fn summary(&self) -> String {
        if self.holds() {
            return match self.relation {
                Relation::Equal => "=".to_owned(),
                Relation::Subset => format!("⊆ (+{} beyond)", self.only_right.len()),
            };
        }
        format!(
            "differ: -{} +{} triples, -{} +{} findings",
            self.only_left.len(),
            self.only_right.len(),
            self.findings_only_left.len(),
            self.findings_only_right.len()
        )
    }
}

/// How many lines of each side a diff prints.
const SHOWN: usize = 20;

impl fmt::Display for Diff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rel = match self.relation {
            Relation::Equal => "=",
            Relation::Subset => "⊆",
        };
        writeln!(f, "{} {rel} {}: {}", self.left, self.right, self.summary())?;
        let side = |f: &mut fmt::Formatter<'_>, mark: &str, who: &str, lines: &[String]| -> fmt::Result {
            if lines.is_empty() {
                return Ok(());
            }
            writeln!(f, "  only in {who} ({}):", lines.len())?;
            for l in lines.iter().take(SHOWN) {
                writeln!(f, "  {mark} {l}")?;
            }
            if lines.len() > SHOWN {
                writeln!(f, "  {mark} ... {} more", lines.len() - SHOWN)?;
            }
            Ok(())
        };
        side(f, "-", &self.left, &self.only_left)?;
        side(f, "-", &format!("{} (findings)", self.left), &self.findings_only_left)?;
        // For a subset, what only the right has is expected; show it only on failure.
        if self.relation == Relation::Equal || !self.holds() {
            side(f, "+", &self.right, &self.only_right)?;
            side(f, "+", &format!("{} (findings)", self.right), &self.findings_only_right)?;
        }
        Ok(())
    }
}

/// Compares two closures.
pub fn compare(left: (&str, &Closure), right: (&str, &Closure), relation: Relation) -> Diff {
    let (ln, l) = left;
    let (rn, r) = right;
    let sorted = |mut v: Vec<String>| {
        v.sort();
        v
    };
    Diff {
        left: ln.to_owned(),
        right: rn.to_owned(),
        relation,
        only_left: sorted(l.triples.difference(&r.triples).map(|t| format!("{t} .")).collect()),
        only_right: sorted(r.triples.difference(&l.triples).map(|t| format!("{t} .")).collect()),
        findings_only_left: sorted(l.findings.difference(&r.findings).map(|f| f.to_string()).collect()),
        findings_only_right: sorted(r.findings.difference(&l.findings).map(|f| f.to_string()).collect()),
    }
}
