//! The Nemo oracle: the W3C forms of the rules, as Datalog, composed from
//! `oracles/rules/` for a configuration ([`crate::programs::nemo`]) and run
//! by Nemo's `nmo` (0.10.1, pinned in `scripts/fetch-tools.sh`). It is the
//! same rule set on a different engine, so it judges the closure and the
//! findings exactly (dt-not-type aside: literals reach Nemo as opaque IRIs).

use crate::input::Codec;
use crate::process::{run as run_process, scratch_dir, timeout};
use crate::{Closure, Finding, Input, Outcome, fetched_version, tools_dir};
use oxrdf::{NamedNode, Term};
use oxttl::NTriplesParser;
use std::path::{Path, PathBuf};
use std::process::Command;
use turbowl::Switches;

/// The pinned version.
pub const VERSION: &str = "0.10.1";

/// The finding codes the program of a configuration exports
/// ([`crate::programs::codes`]): those of [`crate::codes`] but dt-not-type
/// (literals reach Nemo as opaque IRIs), none with `checks` off.
///
/// # Panics
///
/// When the program exports a code that [`crate::CODES`] does not list: a
/// new code goes there too, with its terms' order on [`Finding`].
pub fn codes(switches: Switches) -> Vec<&'static str> {
    let exported = crate::programs::codes(switches);
    let known = crate::codes(switches);
    for code in &exported {
        assert!(
            known.contains(&code.as_str()),
            "oracles/rules/ exports {code}, which crate::CODES does not list in {switches:?}"
        );
    }
    known
        .iter()
        .copied()
        .filter(|c| exported.iter().any(|e| e == c))
        .collect()
}

/// `tools/nemo/nmo`.
pub fn binary() -> PathBuf {
    tools_dir().join("nemo/nmo")
}

/// Runs the program of the default configuration on the input.
pub fn run(input: &Input) -> Outcome {
    run_with(input, Switches::default())
}

/// Runs the program of the configuration on the input.
pub fn run_with(input: &Input, switches: Switches) -> Outcome {
    let nmo = binary();
    match fetched_version(&tools_dir().join("nemo")) {
        Some(v) if v == VERSION && nmo.exists() => {}
        Some(v) => {
            return Outcome::skipped(
                "Nemo",
                format!("tools/nemo is {v}, not {VERSION}: run scripts/fetch-tools.sh"),
            );
        }
        None => return Outcome::skipped("Nemo", "tools/nemo/nmo is missing: run scripts/fetch-tools.sh"),
    }
    let dir = match scratch_dir("nemo") {
        Ok(d) => d,
        Err(e) => return Outcome::Failed(e),
    };
    let program = crate::programs::nemo(switches);
    let mut codec = Codec::new(true);
    let encoded: Vec<_> = input.triples().iter().map(|t| codec.encode_triple(t)).collect();
    let written = std::fs::write(dir.join("program.rls"), program)
        .and_then(|()| std::fs::write(dir.join("input.nt"), crate::input::ntriples(encoded.iter())));
    if let Err(e) = written {
        return Outcome::Failed(e.to_string());
    }
    let out = dir.join("out");
    let mut cmd = Command::new(&nmo);
    cmd.arg("-o")
        .arg("-I")
        .arg(&dir)
        .arg("-D")
        .arg(&out)
        .arg(dir.join("program.rls"));
    if let Err(e) = run_process(&mut cmd, &dir, timeout()) {
        return Outcome::Failed(format!("Nemo: {e}"));
    }
    match read(&codec, &out) {
        Ok(c) => {
            let _ = std::fs::remove_dir_all(&dir);
            Outcome::Ran(c)
        }
        Err(e) => Outcome::Failed(format!("Nemo output: {e}")),
    }
}

fn read(codec: &Codec, out: &Path) -> Result<Closure, String> {
    let text = std::fs::read(out.join("closure.nt")).map_err(|e| format!("closure.nt: {e}"))?;
    let mut triples = Vec::new();
    for t in NTriplesParser::new().for_slice(&text) {
        let t = t.map_err(|e| e.to_string())?;
        triples.extend(codec.decode_triple(t.subject.into(), t.predicate.into(), t.object));
    }
    // One CSV per rule that concludes `false` in the configuration, named
    // after the rule.
    let mut findings = Vec::new();
    let entries = std::fs::read_dir(out).map_err(|e| format!("{}: {e}", out.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        let Some(code) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".csv"))
        else {
            continue;
        };
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for line in text.lines().filter(|l| !l.is_empty()) {
            let terms = csv_fields(line)
                .into_iter()
                .map(|f| {
                    let iri = f.trim_start_matches('<').trim_end_matches('>');
                    NamedNode::new(iri)
                        .map(|n| codec.decode(Term::from(n)))
                        .map_err(|e| format!("{code}: {f}: {e}"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            findings.push(Finding::new(code, terms));
        }
    }
    Ok(Closure::new(triples, findings))
}

/// The fields of one CSV line (RFC 4180 quoting).
fn csv_fields(line: &str) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                fields.last_mut().expect("one field").push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => fields.push(String::new()),
            c => fields.last_mut().expect("one field").push(c),
        }
    }
    fields
}

#[cfg(test)]
mod tests {
    #[test]
    fn splits_csv() {
        assert_eq!(super::csv_fields("a,\"b,\"\"c\"\"\",d"), ["a", "b,\"c\"", "d"]);
    }
}
