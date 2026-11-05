//! What an oracle returns: the closure as a set of triples and the
//! diagnostics as a set of [`Finding`]s.

use oxrdf::{Literal, Term, Triple};
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

    /// The finding codes, sorted, with their counts (`OWLRL.CAX_DW×2`).
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

/// A diagnostic, reduced to what every oracle can say: its code and the terms
/// involved, in one order per code.
///
/// | Code | Terms |
/// |---|---|
/// | `OWLRL.CAX_DW` | individual, class 1, class 2 (as asserted) |
/// | `OWLRL.PRP_PDW` | property 1, property 2, subject, object |
/// | `OWLRL.CLS_NOTHING` | individual |
/// | `OWLRL.PRP_ASYP` | property, subject, object |
/// | `OWLRL.PRP_IRP` | property, individual |
/// | `OWLRL.CLS_COM` | class 1, class 2, individual |
/// | `RDFS.DATATYPE` | the ill-formed literal |
/// | `RDFS.DATATYPE_RANGE` | object, datatype, property |
/// | `OWLRL.OTHER` | the message, as a literal (OWL-RL findings with no code here) |
///
/// turtle-reasoner's `Diagnostic::terms` must use this order (ticket 09);
/// `RDFS.DATATYPE` keeps only the literal, since OWL-RL names nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Finding {
    pub code: String,
    pub terms: Vec<Term>,
}

impl Finding {
    pub fn new(code: impl Into<String>, terms: Vec<Term>) -> Self {
        let code = code.into();
        let terms = if code == "RDFS.DATATYPE" {
            terms
                .into_iter()
                .filter(|t| matches!(t, Term::Literal(_)))
                .take(1)
                .collect()
        } else {
            terms
        };
        Self { code, terms }
    }

    /// A turtle-reasoner diagnostic.
    pub fn from_diagnostic(d: &turtle_reasoner::Diagnostic) -> Self {
        Self::new(d.code, d.terms.clone())
    }

    /// A `reasonable` diagnostic, from its code and message (it has no term
    /// fields; see the message formats in docs/research/reasonable-parity.md §3.2).
    pub fn from_reasonable(code: &str, message: &str) -> Self {
        let terms = terms_in(message);
        let pick: &[usize] = match code {
            "OWLRL.CAX_DW" | "OWLRL.PRP_ASYP" | "RDFS.DATATYPE_RANGE" => &[0, 1, 2],
            // property {p1} is disjoint with {p2} for pair ({x} {p1} {y})
            "OWLRL.PRP_PDW" => &[0, 1, 2, 4],
            "OWLRL.CLS_NOTHING" => &[0],
            "OWLRL.PRP_IRP" => &[0, 1],
            // ill-formed literal {lit} used in triple {s} {p} {o}
            "RDFS.DATATYPE" => &[0],
            _ => return Self::new(code, vec![Literal::new_simple_literal(message).into()]),
        };
        Self::new(code, pick.iter().filter_map(|&i| terms.get(i).cloned()).collect())
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

/// The terms written in N-Triples syntax inside a message, in order.
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
    fn reads_terms_from_reasonable_messages() {
        let f = Finding::from_reasonable(
            "OWLRL.PRP_PDW",
            "property <http://example.org/p> is disjoint with <http://example.org/q> for pair (_:b1 <http://example.org/p> \"a \\\"b\\\"\"@en)",
        );
        assert_eq!(
            f.to_string(),
            "OWLRL.PRP_PDW <http://example.org/p> <http://example.org/q> _:b1 \"a \\\"b\\\"\"@en"
        );
        let f = Finding::from_reasonable(
            "RDFS.DATATYPE",
            "ill-formed literal \"abc\"^^<http://www.w3.org/2001/XMLSchema#integer> used in triple <http://example.org/s> <http://example.org/p> \"abc\"^^<http://www.w3.org/2001/XMLSchema#integer>",
        );
        assert_eq!(
            f.to_string(),
            "RDFS.DATATYPE \"abc\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
        let f = Finding::from_reasonable(
            "OWLRL.CLS_NOTHING",
            "instance <http://example.org/x> is owl:Nothing (unsatisfiable)",
        );
        assert_eq!(f.terms.len(), 1);
    }
}
