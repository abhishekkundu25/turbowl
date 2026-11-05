//! The Nemo oracle: `oracles/nemo/owl2rl.rls` (the W3C forms of exactly the
//! rules version 1 implements, as Datalog) run by Nemo's `nmo` (0.10.1,
//! pinned in `scripts/fetch-tools.sh`). It is the same rule set on a
//! different engine, so it judges the closure and the findings exactly
//! (dt-not-type aside: literals reach Nemo as opaque IRIs).

use crate::input::Codec;
use crate::process::{run as run_process, scratch_dir, timeout};
use crate::{Closure, Finding, Input, Outcome, fetched_version, tools_dir, workspace_root};
use oxrdf::{NamedNode, Term};
use oxttl::NTriplesParser;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The pinned version.
pub const VERSION: &str = "0.10.1";

/// The finding codes the program derives (no datatype checks: literals reach
/// Nemo as opaque IRIs).
pub const CODES: &[&str] = &["cax-dw", "prp-pdw", "cls-nothing2", "prp-asyp", "prp-irp", "cls-com"];

/// `oracles/nemo/owl2rl.rls`.
pub fn program() -> PathBuf {
    workspace_root().join("oracles/nemo/owl2rl.rls")
}

/// `tools/nemo/nmo`.
pub fn binary() -> PathBuf {
    tools_dir().join("nemo/nmo")
}

/// Runs the program on the input.
pub fn run(input: &Input) -> Outcome {
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
    let program = match std::fs::read_to_string(program()) {
        Ok(p) => p,
        Err(e) => return Outcome::Failed(format!("oracles/nemo/owl2rl.rls: {e}")),
    };
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
    let mut findings = Vec::new();
    for code in CODES {
        let path = out.join(format!("{code}.csv"));
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
            findings.push(Finding::new(*code, terms));
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
