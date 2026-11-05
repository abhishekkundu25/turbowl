//! The OWL-RL oracle: RDFLib's implementation of the W3C OWL 2 RL rule tables
//! (`owlrl` 7.6.2, `rdflib` 7.6.0, pinned by `oracles/owlrl/uv.lock`), run as
//! a subprocess through `uv` in that project-local environment.
//!
//! OWL-RL implements the full rule set (and adds vocabulary triples such as
//! `x owl:sameAs x` for every term and datatype declarations), so it judges
//! by containment: TurbOWL's closure and findings must be a subset of
//! OWL-RL's, after the filters of its known faults (`differences.rs`).

use crate::input::Codec;
use crate::process::{run as run_process, scratch_dir, timeout};
use crate::{Closure, Finding, Input, Outcome, workspace_root};
use oxrdf::{Literal, Term};
use oxttl::NTriplesParser;
use std::process::Command;
use std::str::FromStr;

/// The pinned versions (oracles/owlrl/pyproject.toml).
pub const VERSION: &str = "owlrl 7.6.2, rdflib 7.6.0";

/// The finding codes of the rule set OWL-RL reports (`reason.py` maps its
/// messages onto them; any other message becomes an `other` finding).
pub const CODES: &[&str] = crate::closure::CODES;

/// Runs OWL-RL on the input.
pub fn run(input: &Input) -> Outcome {
    let project = workspace_root().join("oracles/owlrl");
    if Command::new("uv").arg("--version").output().is_err() {
        return Outcome::skipped("OWL-RL", "`uv` is not on PATH (https://docs.astral.sh/uv/)");
    }
    let dir = match scratch_dir("owlrl") {
        Ok(d) => d,
        Err(e) => return Outcome::Failed(e),
    };
    let mut codec = Codec::new(false);
    let encoded: Vec<_> = input.triples().iter().map(|t| codec.encode_triple(t)).collect();
    let (in_path, closure_path, findings_path) =
        (dir.join("input.nt"), dir.join("closure.nt"), dir.join("findings.tsv"));
    if let Err(e) = std::fs::write(&in_path, crate::input::ntriples(encoded.iter())) {
        return Outcome::Failed(e.to_string());
    }
    let mut cmd = Command::new("uv");
    cmd.args(["run", "--quiet", "--frozen", "--project"])
        .arg(&project)
        .args(["python", "-I"])
        .arg(project.join("reason.py"))
        .args([&in_path, &closure_path, &findings_path])
        .env("UV_PYTHON_DOWNLOADS", "never");
    if let Err(e) = run_process(&mut cmd, &dir, timeout()) {
        return Outcome::Failed(format!("OWL-RL: {e}"));
    }
    match read(&codec, &closure_path, &findings_path) {
        Ok(c) => {
            let _ = std::fs::remove_dir_all(&dir);
            Outcome::Ran(c)
        }
        Err(e) => Outcome::Failed(format!("OWL-RL output: {e}")),
    }
}

fn read(codec: &Codec, closure: &std::path::Path, findings: &std::path::Path) -> Result<Closure, String> {
    let text = std::fs::read(closure).map_err(|e| e.to_string())?;
    let mut triples = Vec::new();
    for t in NTriplesParser::new().for_slice(&text) {
        let t = t.map_err(|e| e.to_string())?;
        triples.extend(codec.decode_triple(t.subject.into(), t.predicate.into(), t.object));
    }
    let text = std::fs::read_to_string(findings).map_err(|e| e.to_string())?;
    let mut found = Vec::new();
    for line in text.lines().filter(|l| !l.is_empty()) {
        let mut fields = line.split('\t');
        let code = fields.next().unwrap_or_default();
        let rest: Vec<&str> = fields.collect();
        if code == "other" {
            found.push(Finding::new(
                code,
                vec![Literal::new_simple_literal(rest.join(" ")).into()],
            ));
            continue;
        }
        let terms = rest
            .iter()
            .map(|s| {
                Term::from_str(s)
                    .map(|t| codec.decode(t))
                    .map_err(|e| format!("{s}: {e}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        found.push(Finding::new(code, terms));
    }
    Ok(Closure::new(triples, found))
}
