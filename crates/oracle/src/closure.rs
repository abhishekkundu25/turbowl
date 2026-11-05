//! What an oracle returns: the closure as a set of triples and the
//! diagnostics as a set of [`Finding`]s.

use oxrdf::{Term, Triple};
use oxttl::NTriplesParser;
use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

/// A closure (base, seed and inferred triples) and its findings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Closure {
    pub triples: HashSet<Triple>,
    pub findings: HashSet<Finding>,
}

impl Closure {
    pub fn new(triples: impl IntoIterator<Item = Triple>, findings: impl IntoIterator<Item = Finding>) -> Self {
        Self {
            triples: triples.into_iter().collect(),
            findings: findings.into_iter().collect(),
        }
    }

    /// True when the closure holds the triple written in N-Triples (without the final dot).
    pub fn has(&self, ntriple: &str) -> bool {
        self.triples.contains(&triple(ntriple))
    }

    /// The triples as sorted N-Triples lines.
    pub fn lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self.triples.iter().map(|t| format!("{t} .")).collect();
        lines.sort();
        lines
    }

    /// The finding codes, sorted, with their counts (`cax-dw×2`).
    pub fn finding_summary(&self) -> String {
        let mut codes: Vec<&str> = self.findings.iter().map(|f| f.code.as_str()).collect();
        codes.sort_unstable();
        let mut out: Vec<String> = Vec::new();
        for code in codes.iter().copied().collect::<std::collections::BTreeSet<_>>() {
            let n = codes.iter().filter(|c| **c == code).count();
            out.push(if n == 1 {
                code.to_owned()
            } else {
                format!("{code}×{n}")
            });
        }
        if out.is_empty() { "-".to_owned() } else { out.join(" ") }
    }
}

/// One triple written in N-Triples, with or without the final dot (panics when invalid).
pub fn triple(ntriple: &str) -> Triple {
    let line = ntriple.trim().trim_end_matches('.').trim();
    NTriplesParser::new()
        .for_slice(format!("{line} .\n").as_bytes())
        .next()
        .unwrap_or_else(|| panic!("no triple in {ntriple:?}"))
        .unwrap_or_else(|e| panic!("{ntriple:?}: {e}"))
}

/// The diagnostic codes of the rule set: the W3C rules that conclude
/// `false`, and dt-not-type. Every oracle reports its findings with these
/// codes and the term orders on [`Finding`].
pub const CODES: &[&str] = &[
    "cax-dw",
    "prp-pdw",
    "cls-nothing2",
    "prp-asyp",
    "prp-irp",
    "cls-com",
    "dt-not-type",
];

/// A diagnostic, reduced to what every oracle can say: its code (the W3C rule
/// name) and the terms involved, in one order per code.
///
/// | Code | Terms |
/// |---|---|
/// | `cax-dw` | individual, class 1, class 2 (as asserted in `class1 owl:disjointWith class2`) |
/// | `prp-pdw` | property 1, property 2, subject, object |
/// | `cls-nothing2` | individual |
/// | `prp-asyp` | property, subject, object |
/// | `prp-irp` | property, individual |
/// | `cls-com` | class 1, class 2, individual |
/// | `dt-not-type` | literal, datatype (the literal's own, or a type such as one from prp-rng) |
/// | `other` | the message, as a literal (OWL-RL findings outside the rule set) |
///
/// turtle-reasoner's `Diagnostic` maps onto it by `code` and `terms` (ticket
/// 09 emits the terms in this order); its `severity` and `message` are not
/// compared. Nemo writes the same columns. OWL-RL's messages name the same
/// terms for every code, but see the oracle faults in `differences.rs`: its
/// `cls-com` names the second class where the individual belongs (F2), and
/// its `dt-not-type` checks only a literal's own datatype (F5), so it always
/// names the literal and the literal's datatype.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Finding {
    pub code: String,
    pub terms: Vec<Term>,
}

impl Finding {
    pub fn new(code: impl Into<String>, terms: Vec<Term>) -> Self {
        Self {
            code: code.into(),
            terms,
        }
    }

    /// A turtle-reasoner diagnostic.
    pub fn from_diagnostic(d: &turtle_reasoner::Diagnostic) -> Self {
        Self::new(d.code, d.terms.clone())
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.code)?;
        for t in &self.terms {
            write!(f, " {t}")?;
        }
        Ok(())
    }
}

/// The terms written in N-Triples syntax inside a text, in order.
pub(crate) fn terms_in(text: &str) -> Vec<Term> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        let len = if rest.starts_with('<') {
            rest.find('>').map(|e| e + 1)
        } else if rest.starts_with("_:") {
            Some(
                rest.find(|c: char| c.is_whitespace() || c == ')' || c == ',')
                    .unwrap_or(rest.len()),
            )
        } else if rest.starts_with('"') {
            literal_len(rest)
        } else {
            None
        };
        match len.and_then(|n| Term::from_str(&rest[..n]).ok().map(|t| (n, t))) {
            Some((n, t)) => {
                out.push(t);
                i += n;
            }
            None => i += rest.chars().next().map_or(1, char::len_utf8),
        }
    }
    out
}

/// The length of the N-Triples literal at the start of `s`.
fn literal_len(s: &str) -> Option<usize> {
    let mut escaped = false;
    let mut end = None;
    for (i, c) in s.char_indices().skip(1) {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '"' => {
                end = Some(i + 1);
                break;
            }
            _ => {}
        }
    }
    let end = end?;
    let tail = &s[end..];
    if let Some(dt) = tail.strip_prefix("^^<") {
        return dt.find('>').map(|e| end + 3 + e + 1);
    }
    if let Some(lang) = tail.strip_prefix('@') {
        let n = lang
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(lang.len());
        return Some(end + 1 + n);
    }
    Some(end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_terms_from_text() {
        let terms = terms_in(
            "<http://example.org/p> and _:b1, \"a \\\"b\\\"\"@en then \"5\"^^<http://www.w3.org/2001/XMLSchema#int>",
        );
        let shown: Vec<String> = terms.iter().map(ToString::to_string).collect();
        assert_eq!(
            shown,
            [
                "<http://example.org/p>",
                "_:b1",
                "\"a \\\"b\\\"\"@en",
                "\"5\"^^<http://www.w3.org/2001/XMLSchema#int>"
            ]
        );
    }
}
