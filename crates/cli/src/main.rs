//! The `turbowl` command: reason over files from the command line.
//!
//! A thin adapter over [`turbowl::Reasoner`]: it loads every input file
//! into one reasoner (each format from its extension, blank nodes kept
//! apart per file), runs it once, writes the closure, and prints a report
//! (the diagnostics, then the summary). The reasoning is the library's.
//!
//! # Switches
//!
//! The library's switches ([`turbowl::Switches`]), by the names
//! [`Switch::name`] gives. The run starts from the default switches, or
//! from every switch on with `--complete` (the complete W3C rule set);
//! each `--switch NAME=on|off` then turns one on or off, in the order
//! given, so a later `--switch` for the same name wins over an earlier
//! one, and `--complete` is a starting point wherever it comes.
//! `--switches` lists each switch with its default and what it costs
//! (the listing `--help` also carries), and exits 0.
//!
//! # Snapshots
//!
//! `--save-snapshot PATH` saves the reasoner's state after the run
//! ([`turbowl::Reasoner::save_snapshot`]: written to a temporary file and
//! renamed, so a crash never leaves half of one). `--load-snapshot PATH`
//! starts from a saved state instead of an empty reasoner
//! ([`turbowl::Reasoner::from_snapshot`]); the FILEs are then added to it,
//! and may be none. A loaded state keeps its switches: `--complete` and
//! `--switch` start from them, and a change makes the run a full one. A
//! snapshot that cannot be read or is refused (not a snapshot, another
//! format or TurbOWL version, truncated, damaged) exits 1. The two can be
//! given together, and name the same file.
//!
//! # Where things go
//!
//! - The closure goes to `-o PATH`, or to stdout (no `-o`, or `-o -`).
//! - The report goes to stdout when the closure does not, and to stderr
//!   when the closure takes stdout, so the two never mix.
//! - Errors (exit 1 and 64) go to stderr as one line; no report follows.
//! - A reader that closes stdout early (`turbowl model.ttl | head`) ends
//!   the output quietly: not an error. The report and the exit status
//!   stay as they would be.
//!
//! # The report
//!
//! `--diagnostics` picks its form:
//!
//! - `text` (the default): one line per diagnostic,
//!   `error[cax-dw]: <message>`, then one summary line,
//!   `<n> base, <n> inferred, <n> rounds, <n> diagnostics (loaded in <s> s, reasoned in <s> s)`.
//!   The wording may change; use `json` or `ndjson` to read it from a script.
//! - `json`: one object, `{"summary": SUMMARY, "diagnostics": [DIAGNOSTIC, ...]}`.
//! - `ndjson`: one object per line: each diagnostic with `"type": "diagnostic"`,
//!   then the summary with `"type": "summary"`, always last.
//!
//! These field names are stable:
//!
//! - DIAGNOSTIC: `code` (the W3C rule, as `cax-dw`), `severity` (`"error"`
//!   or `"warning"`), `message` (one sentence; its wording may change) and
//!   `terms` (the terms involved, each an N-Triples string such as
//!   `<http://example.org/a>`, `_:b0` or `"1"^^<http://www.w3.org/2001/XMLSchema#integer>`,
//!   in the order `turbowl::Diagnostic` documents for the code).
//! - SUMMARY: `base`, `inferred`, `rounds` and `diagnostics` (the counts of
//!   `turbowl::Summary`; `diagnostics` counts them all, listed or not),
//!   `load_seconds` and `reason_seconds` (wall-clock time, in seconds),
//!   and `switches` (the names of the switches that were on, in the order
//!   `--switches` lists them).
//!
//! `--max-diagnostics N` lists at most N diagnostics. `--summary-only`
//! writes no closure and lists no diagnostic: the report is the summary
//! alone (and any `--explain` proof), on stdout.
//!
//! # Explanations
//!
//! `--explain '<s> <p> <o>'` asks why a triple is in the closure: its value
//! is one N-Triples triple (the final ` .` may be left out), and the flag
//! can be repeated. A blank node is named as the closure writes it
//! (`_:b_1`). `--explain-diagnostics` asks the same of every listed
//! diagnostic. The answer is a proof ([`turbowl::Proof`]): each step a
//! triple and the W3C rule that concludes it from earlier steps, down to
//! base triples. A triple outside the closure has none: the report says
//! so, and the exit status does not change. A value that is not one
//! N-Triples triple is a usage error (64).
//!
//! In `text`, the proofs follow the report, each after a blank line: the
//! `--explain` triples in the order given, then the listed diagnostics, each
//! as the indented tree [`turbowl::Proof`]'s `Display` writes (the goal
//! first, each premise indented under the step that reads it). A triple
//! without a proof is one line, `not in the closure: <s> <p> <o> .`.
//!
//! In `json` and `ndjson`:
//!
//! - PROOF: `{"depth": N, "steps": [STEP, ...]}`, the steps in order, each
//!   premise before the steps that read it, the goal last. `depth` is the
//!   most rule steps from the goal to a base triple.
//! - STEP: `statement` (the triple, three N-Triples terms as in `terms`;
//!   a generalized triple, whose subject may be a literal), `rule` (the W3C
//!   rule, as `cax-sco`, or `null` for a base triple) and `premises` (the
//!   indexes in `steps` of the steps it reads; a rule such as cls-thing has
//!   none). The goal of a diagnostic's proof has `diagnostic` (its code) in
//!   place of `statement`.
//! - EXPLANATION: `triple` (the `--explain` triple, three N-Triples terms)
//!   and `proof` (a PROOF, or `null` when the triple is not in the closure).
//! - In `json`, `--explain` adds `"explanations": [EXPLANATION, ...]`, in
//!   the order given. In `ndjson`, each EXPLANATION is a line with
//!   `"type": "explanation"`, after the diagnostics, before the summary.
//! - `--explain-diagnostics` adds `proof` (a PROOF, or `null`) to each
//!   listed DIAGNOSTIC.
//!
//! # Exit status
//!
//! | Code | Meaning |
//! |---|---|
//! | 0 | Success. |
//! | 1 | An input or output error, a parse error, or a snapshot refused. |
//! | 2 | A diagnostic at or above `--fail-on` (default `error`); the closure and the report are still written. |
//! | 64 | A usage error (an unknown option, a bad value, an unknown switch, an `--explain` value that is not one N-Triples triple, no input and no `--load-snapshot`, an input whose extension names no format). |

use oxrdf::{Term, Triple};
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};
use turbowl::{Diagnostic, Format, Goal, Proof, Reason, Reasoner, Severity, Summary, Switch, Switches};

/// The help, up to the switches (see [`help`]).
const HELP_OPTIONS: &str = "\
turbowl: an OWL 2 RL reasoner

Usage: turbowl [OPTIONS] <FILE>...
       turbowl [OPTIONS] --load-snapshot <PATH> [FILE]...

Loads every FILE into one graph, reasons to the OWL 2 RL closure, writes
the closure, and reports the diagnostics and a summary. Each FILE's format
comes from its extension: .ttl, .nt, .n3, .rdf, .owl or .xml. Blank nodes
of different files stay apart.

Options:
  -o, --output <PATH>        Write the closure to PATH (default, or \"-\": stdout)
      --format <FORMAT>      The closure's format: turtle, ntriples, n3 or rdfxml
                             (default: from the -o extension, else turtle)
      --inferred-only        Write only the inferred triples, not the input ones
      --diagnostics <FORM>   The report's form: text (default), json or ndjson
      --fail-on <SEVERITY>   Exit 2 on a diagnostic at or above SEVERITY:
                             error (default), warning or never
      --max-diagnostics <N>  List at most N diagnostics (the summary counts all)
      --summary-only         Write no closure and list no diagnostic: print
                             only the summary (and --explain), to stdout
      --explain '<s> <p> <o>'
                             Print why the triple, one N-Triples triple (blank
                             nodes as the closure names them), is in the
                             closure: its proof, after the report. Repeatable
      --explain-diagnostics  Print the proof of every listed diagnostic
      --switch NAME=on|off   Turn the switch NAME on or off. Repeatable: the
                             flags apply in order, so a later one for the
                             same NAME wins
      --complete             Start from every switch on, the complete W3C
                             OWL 2 RL rule set; --switch still applies
      --switches             List the switches, their defaults and costs
      --save-snapshot <PATH> After the run, save the reasoner's state to PATH
                             (atomically: a crash never leaves half of it)
      --load-snapshot <PATH> Start from the state saved in PATH, then add the
                             FILEs (none needed). It keeps its switches, which
                             --complete and --switch then change
  -h, --help                 Print this help
  -V, --version              Print the version
";

/// The help, after the switches.
const HELP_REST: &str = "\
Where things go:
  The closure goes to -o, or to stdout. The report (the diagnostics, then
  the summary) goes to stdout when the closure does not, and to stderr when
  the closure takes stdout. Errors go to stderr. A reader that closes
  stdout early (| head) is not an error.

The report:
  text    One line per diagnostic, \"error[cax-dw]: <message>\", then one
          summary line. Meant for people: use json or ndjson in scripts.
  json    One object: {\"summary\": SUMMARY, \"diagnostics\": [DIAGNOSTIC, ...]}.
  ndjson  One object per line: each DIAGNOSTIC with \"type\": \"diagnostic\",
          then the SUMMARY with \"type\": \"summary\", always last.

  DIAGNOSTIC  {\"code\": \"cax-dw\", \"severity\": \"error\" or \"warning\",
               \"message\": \"...\", \"terms\": [\"<http://example.org/a>\", ...]}
               Each term is written as in N-Triples.
  SUMMARY     {\"base\": N, \"inferred\": N, \"rounds\": N, \"diagnostics\": N,
               \"load_seconds\": S, \"reason_seconds\": S,
               \"switches\": [\"equality\", ...]}
               \"diagnostics\" counts every diagnostic, listed or not;
               \"switches\" names the switches that were on.

Explanations:
  A proof gives each step's triple and the W3C rule that concludes it from
  earlier steps, down to base triples. In text, each proof follows the
  report as an indented tree, the goal first; a triple outside the closure
  is one line, \"not in the closure: ...\", and leaves the exit status as it
  is. In json, --explain adds \"explanations\": [EXPLANATION, ...], and
  --explain-diagnostics adds \"proof\" to each DIAGNOSTIC. In ndjson, each
  EXPLANATION is a line with \"type\": \"explanation\", before the summary.

  EXPLANATION {\"triple\": [\"<s>\", \"<p>\", \"<o>\"], \"proof\": PROOF or null}
  PROOF       {\"depth\": N, \"steps\": [STEP, ...]}, the goal last
  STEP        {\"statement\": [\"<s>\", \"<p>\", \"<o>\"],
               \"rule\": \"cax-sco\" or null (a base triple),
               \"premises\": [the indexes of the steps it reads]}
               A diagnostic's goal has \"diagnostic\": \"cax-dw\" in place
               of \"statement\".

Exit status:
  0   Success.
  1   An input or output error, a parse error, or a snapshot refused.
  2   A diagnostic at or above --fail-on. The closure and the report are
      still written.
  64  A usage error.
";

/// The help: the options, the switches, the rest.
fn help() -> String {
    format!(
        "{HELP_OPTIONS}\nSwitches (groups of rules; the other rules are always on):\n{}\n{HELP_REST}",
        listing()
    )
}

/// What a switch does, and what it costs.
fn about(switch: Switch) -> &'static str {
    match switch {
        Switch::Equality => {
            "owl:sameAs makes terms equal (eq-sym, eq-trans, eq-rep-*). Cheap \
             until owl:sameAs occurs; each merge then rewrites the merged \
             terms' facts."
        }
        Switch::PropertyChains => {
            "owl:propertyChainAxiom (prp-spo2). A join per link of each chain; free without chains."
        }
        Switch::Checks => {
            "Reports contradictions and ill-typed literals (cax-dw, \
             dt-not-type, ...). Derives no triple: off saves a pass over the \
             facts each check names, and loses the diagnostics."
        }
        Switch::Identity => {
            "owl:hasKey and max cardinality 1 make individuals the same \
             (prp-key, cls-maxc2, ...). A join per key and restriction, then \
             equality's cost per merge: 26% slower on uobm-010-u, whose \
             closure it grows from 21 M to 39 M triples."
        }
        Switch::Schema => {
            "Subsumptions between schema axioms (scm-spo, scm-dom1, ...). \
             Its cost grows with the data too: 35% slower on LUBM at every \
             size, though it adds only 99 triples there."
        }
        Switch::Trivial => {
            "Filler: x owl:sameAs x for every term, four triples per class, \
             each literal's datatypes (eq-ref, scm-cls, dt-type2, ...). Grows \
             with every term."
        }
        Switch::LiteralValues => {
            "Literals equal by value, as \"1\" and \"01\" of xsd:integer \
             (dt-eq, dt-diff). Reads every literal's value, and merges equal \
             ones at equality's cost per merge; dt-diff only checks, it lists \
             no owl:differentFrom triples."
        }
    }
}

/// The switches as `--switches` lists them (and `--help` shows them): a
/// header, then one block per switch, its name and default first.
fn listing() -> String {
    const INDENT: usize = 28;
    const WIDTH: usize = 79;
    let mut out = format!(
        "  {:<17}{:<9}{}\n",
        "Switch", "Default", "What it does, and what it costs"
    );
    for switch in Switch::ALL {
        let default = if Switches::default().is_on(switch) { "on" } else { "off" };
        let mut line = format!("  {:<17}{default:<9}", switch.name());
        for (i, word) in about(switch).split_whitespace().enumerate() {
            if i > 0 && line.len() + 1 + word.len() > WIDTH {
                out.push_str(&line);
                out.push('\n');
                line = " ".repeat(INDENT);
            } else if i > 0 {
                line.push(' ');
            }
            line.push_str(word);
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// A usage error: the message, printed before a pointer to `--help`.
#[derive(Debug)]
struct Usage(String);

/// What the command line asks for.
#[derive(Debug)]
enum Command {
    Help,
    Version,
    /// `--switches`: list them.
    Switches,
    Run(Options),
}

#[derive(Debug)]
struct Options {
    inputs: Vec<PathBuf>,
    /// `None`: stdout.
    output: Option<PathBuf>,
    format: Format,
    inferred_only: bool,
    report: Report,
    /// `None`: never fail on a diagnostic.
    fail_on: Option<Severity>,
    max_diagnostics: usize,
    summary_only: bool,
    /// `--complete`: start from every switch on.
    complete: bool,
    /// Each `--switch`, in order: applied after the starting point.
    settings: Vec<(Switch, bool)>,
    /// `--load-snapshot`: start from this state.
    load_snapshot: Option<PathBuf>,
    /// `--save-snapshot`: save the state here after the run.
    save_snapshot: Option<PathBuf>,
    /// Each `--explain` triple, in order.
    explain: Vec<Triple>,
    /// `--explain-diagnostics`.
    explain_diagnostics: bool,
}

impl Options {
    /// The switches of the run: from every switch on (`--complete`), from
    /// `loaded`'s (a snapshot's) or from the defaults, then each `--switch`.
    fn switches(&self, loaded: Option<Switches>) -> Switches {
        let start = if self.complete {
            Switches::complete()
        } else {
            loaded.unwrap_or_default()
        };
        self.settings
            .iter()
            .fold(start, |s, &(switch, on)| if on { s.on(switch) } else { s.off(switch) })
    }
}

/// The report's form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Report {
    Text,
    Json,
    Ndjson,
}

fn main() -> ExitCode {
    let command = match parse(std::env::args_os().skip(1)) {
        Ok(command) => command,
        Err(Usage(message)) => {
            eprintln!("turbowl: {message}\nTry 'turbowl --help'.");
            return ExitCode::from(64);
        }
    };
    match command {
        Command::Help => print(&help()),
        Command::Version => print(&format!("turbowl {}\n", env!("CARGO_PKG_VERSION"))),
        Command::Switches => print(&listing()),
        Command::Run(options) => match run(&options) {
            Ok(code) => code,
            Err(message) => {
                eprintln!("turbowl: error: {message}");
                ExitCode::from(1)
            }
        },
    }
}

/// Prints help or version text on stdout.
fn print(text: &str) -> ExitCode {
    match std::io::stdout().write_all(text.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
}

/// Loads, reasons, writes the closure and the report. `Err` is an input or
/// output error (exit 1), as one line.
fn run(options: &Options) -> Result<ExitCode, String> {
    let started = Instant::now();
    let mut reasoner = match &options.load_snapshot {
        Some(path) => {
            let mut r = Reasoner::from_snapshot(path).map_err(|e| e.to_string())?;
            r.set_switches(options.switches(Some(r.switches())));
            r
        }
        None => Reasoner::with_switches(options.switches(None)),
    };
    for input in &options.inputs {
        reasoner.load_file(input).map_err(|e| e.to_string())?;
    }
    let loaded = Instant::now();
    let summary = reasoner.reason();
    let timing = Timing {
        load: loaded - started,
        reason: loaded.elapsed(),
    };
    if let Some(path) = &options.save_snapshot {
        reasoner.save_snapshot(path).map_err(|e| e.to_string())?;
    }

    if !options.summary_only {
        match &options.output {
            Some(path) => {
                let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
                reasoner
                    .write(file, options.format, options.inferred_only)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
            }
            None => match reasoner.write(std::io::stdout().lock(), options.format, options.inferred_only) {
                // The reader stopped reading (`turbowl ... | head`): not a failure.
                Err(turbowl::Error::Io { source, .. }) if source.kind() == std::io::ErrorKind::BrokenPipe => {}
                result => result.map_err(|e| format!("writing the closure: {e}"))?,
            },
        }
    }

    let listed = if options.summary_only {
        &[][..]
    } else {
        let all = reasoner.diagnostics();
        &all[..all.len().min(options.max_diagnostics)]
    };
    let proofs = explain(&reasoner, options, listed)?;
    let report = report(options.report, listed, &summary, reasoner.switches(), &timing, &proofs);
    let closure_on_stdout = options.output.is_none() && !options.summary_only;
    if closure_on_stdout {
        // stderr may be closed; the report is not worth failing the run for.
        let _ = std::io::stderr().write_all(report.as_bytes());
    } else {
        let mut out = std::io::stdout().lock();
        match out.write_all(report.as_bytes()).and_then(|()| out.flush()) {
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
            result => result.map_err(|e| format!("writing the report: {e}"))?,
        }
    }

    let fails = options.fail_on.is_some_and(|threshold| {
        reasoner
            .diagnostics()
            .iter()
            .any(|d| rank(d.severity) >= rank(threshold))
    });
    Ok(if fails { ExitCode::from(2) } else { ExitCode::SUCCESS })
}

/// The proofs the run was asked for: `None` where there is none.
#[derive(Debug, Default)]
struct Proofs<'a> {
    /// Each `--explain` triple, with its proof.
    triples: Vec<(&'a Triple, Option<Proof>)>,
    /// With `--explain-diagnostics`, each listed diagnostic's proof, in
    /// the order listed.
    diagnostics: Option<Vec<Option<Proof>>>,
}

/// Explains what `options` asks, with one explainer over the closure.
fn explain<'a>(reasoner: &Reasoner, options: &'a Options, listed: &[Diagnostic]) -> Result<Proofs<'a>, String> {
    if options.explain.is_empty() && !options.explain_diagnostics {
        return Ok(Proofs::default());
    }
    let explainer = reasoner.explainer().map_err(|e| e.to_string())?;
    Ok(Proofs {
        triples: options
            .explain
            .iter()
            .map(|t| (t, explainer.explain(t.as_ref())))
            .collect(),
        diagnostics: options
            .explain_diagnostics
            .then(|| listed.iter().map(|d| explainer.explain_diagnostic(d)).collect()),
    })
}

/// How long the stages took.
#[derive(Debug)]
struct Timing {
    load: Duration,
    reason: Duration,
}

/// Severities in order: a higher rank is more serious.
fn rank(severity: Severity) -> u8 {
    match severity {
        Severity::Warning => 1,
        Severity::Error => 2,
    }
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

/// Seconds, to the microsecond.
fn seconds(d: Duration) -> f64 {
    (d.as_secs_f64() * 1e6).round() / 1e6
}

/// The report: the listed diagnostics, then the summary, then the proofs
/// asked for (in `json`, beside them; in `ndjson`, before the summary).
fn report(
    form: Report,
    listed: &[Diagnostic],
    summary: &Summary,
    switches: Switches,
    timing: &Timing,
    proofs: &Proofs<'_>,
) -> String {
    let mut out = String::new();
    // Each listed diagnostic as JSON, with its proof when asked.
    let diagnostics = || {
        listed.iter().enumerate().map(|(i, d)| {
            let mut record = diagnostic_json(d);
            if let Some(proofs) = &proofs.diagnostics {
                record["proof"] = proofs[i].as_ref().map_or(serde_json::Value::Null, proof_json);
            }
            record
        })
    };
    match form {
        Report::Text => {
            for d in listed {
                out.push_str(&format!("{}[{}]: {}\n", severity_name(d.severity), d.code, d.message));
            }
            out.push_str(&format!(
                "{} base, {} inferred, {} rounds, {} diagnostics (loaded in {:.3} s, reasoned in {:.3} s)\n",
                summary.base,
                summary.inferred,
                summary.rounds,
                summary.diagnostics,
                timing.load.as_secs_f64(),
                timing.reason.as_secs_f64(),
            ));
            for (triple, proof) in &proofs.triples {
                out.push('\n');
                match proof {
                    Some(proof) => out.push_str(&proof.to_string()),
                    None => out.push_str(&format!("not in the closure: {triple} .\n")),
                }
            }
            for (d, proof) in listed.iter().zip(proofs.diagnostics.iter().flatten()) {
                out.push('\n');
                match proof {
                    Some(proof) => out.push_str(&proof.to_string()),
                    None => out.push_str(&format!(
                        "no proof: {}[{}]: {}\n",
                        severity_name(d.severity),
                        d.code,
                        d.message
                    )),
                }
            }
        }
        Report::Json => {
            let mut document = serde_json::json!({
                "summary": summary_json(summary, switches, timing),
                "diagnostics": diagnostics().collect::<Vec<_>>(),
            });
            if !proofs.triples.is_empty() {
                document["explanations"] = proofs.triples.iter().map(explanation_json).collect();
            }
            out.push_str(&document.to_string());
            out.push('\n');
        }
        Report::Ndjson => {
            for mut record in diagnostics() {
                record["type"] = "diagnostic".into();
                out.push_str(&record.to_string());
                out.push('\n');
            }
            for explained in &proofs.triples {
                let mut record = explanation_json(explained);
                record["type"] = "explanation".into();
                out.push_str(&record.to_string());
                out.push('\n');
            }
            let mut record = summary_json(summary, switches, timing);
            record["type"] = "summary".into();
            out.push_str(&record.to_string());
            out.push('\n');
        }
    }
    out
}

fn diagnostic_json(d: &Diagnostic) -> serde_json::Value {
    serde_json::json!({
        "code": d.code,
        "severity": severity_name(d.severity),
        "message": d.message,
        "terms": d.terms.iter().map(|t| t.to_string()).collect::<Vec<_>>(),
    })
}

/// An `--explain` triple and its proof.
fn explanation_json((triple, proof): &(&Triple, Option<Proof>)) -> serde_json::Value {
    let subject = Term::from(triple.subject.clone());
    let predicate = Term::from(triple.predicate.clone());
    serde_json::json!({
        "triple": terms_json([&subject, &predicate, &triple.object]),
        "proof": proof.as_ref().map_or(serde_json::Value::Null, proof_json),
    })
}

/// A proof: its depth, and its steps, the goal last.
fn proof_json(proof: &Proof) -> serde_json::Value {
    let mut steps: Vec<serde_json::Value> = proof
        .steps()
        .iter()
        .map(|step| {
            let s = &step.statement;
            let mut record = reason_json(&step.reason);
            record["statement"] = terms_json([&s.subject, &s.predicate, &s.object]);
            record
        })
        .collect();
    if let Goal::Diagnostic(d) = proof.goal() {
        let mut record = reason_json(proof.reason());
        record["diagnostic"] = d.code.into();
        steps.push(record);
    }
    serde_json::json!({"depth": proof.depth(), "steps": steps})
}

/// A step's `rule` (`null` for a base triple) and `premises`.
fn reason_json(reason: &Reason) -> serde_json::Value {
    match reason {
        Reason::Base => serde_json::json!({"rule": null, "premises": []}),
        Reason::Rule { rule, premises } => serde_json::json!({"rule": rule, "premises": premises}),
    }
}

/// Three terms, each an N-Triples string.
fn terms_json(terms: [&Term; 3]) -> serde_json::Value {
    terms.iter().map(|t| t.to_string()).collect()
}

fn summary_json(summary: &Summary, switches: Switches, timing: &Timing) -> serde_json::Value {
    serde_json::json!({
        "base": summary.base,
        "inferred": summary.inferred,
        "rounds": summary.rounds,
        "diagnostics": summary.diagnostics,
        "load_seconds": seconds(timing.load),
        "reason_seconds": seconds(timing.reason),
        "switches": Switch::ALL.into_iter().filter(|&s| switches.is_on(s)).map(Switch::name).collect::<Vec<_>>(),
    })
}

/// Reads the arguments (without the program name).
fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, Usage> {
    let mut args = args.into_iter();
    let mut inputs = Vec::new();
    let mut output: Option<OsString> = None;
    let mut format: Option<Format> = None;
    let mut inferred_only = false;
    let mut report: Option<Report> = None;
    let mut fail_on: Option<Option<Severity>> = None;
    let mut max_diagnostics: Option<usize> = None;
    let mut summary_only = false;
    let mut complete = false;
    // Each `--switch`, in order: applied after the starting point.
    let mut settings: Vec<(Switch, bool)> = Vec::new();
    let mut load_snapshot: Option<OsString> = None;
    let mut save_snapshot: Option<OsString> = None;
    let mut explain = Vec::new();
    let mut explain_diagnostics = false;
    let mut options_ended = false;

    while let Some(arg) = args.next() {
        if options_ended || arg == "-" || !arg.as_encoded_bytes().starts_with(b"-") {
            inputs.push(PathBuf::from(arg));
            continue;
        }
        if arg == "--" {
            options_ended = true;
            continue;
        }
        let text = arg
            .to_str()
            .ok_or_else(|| Usage(format!("unknown option '{}'", arg.to_string_lossy())))?;
        let (name, inline) = match text.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(OsString::from(value))),
            _ => (text, None),
        };
        // The option's value: after '=', or the next argument.
        let mut value = || {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| Usage(format!("{name} needs a value")))
        };
        let once = |given: bool| {
            if given {
                Err(Usage(format!("{name} is given twice")))
            } else {
                Ok(())
            }
        };
        let flag = |given: bool| {
            once(given)?;
            match inline {
                Some(_) => Err(Usage(format!("{name} takes no value"))),
                None => Ok(true),
            }
        };
        match name {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "--switches" => return Ok(Command::Switches),
            "-o" | "--output" => {
                once(output.is_some())?;
                output = Some(value()?);
            }
            "--format" => {
                once(format.is_some())?;
                format = Some(output_format(&value()?)?);
            }
            "--inferred-only" => inferred_only = flag(inferred_only)?,
            "--diagnostics" => {
                once(report.is_some())?;
                report = Some(match value()?.to_str() {
                    Some("text") => Report::Text,
                    Some("json") => Report::Json,
                    Some("ndjson") => Report::Ndjson,
                    _ => return Err(Usage("--diagnostics takes text, json or ndjson".into())),
                });
            }
            "--fail-on" => {
                once(fail_on.is_some())?;
                fail_on = Some(match value()?.to_str() {
                    Some("error") => Some(Severity::Error),
                    Some("warning") => Some(Severity::Warning),
                    Some("never") => None,
                    _ => return Err(Usage("--fail-on takes error, warning or never".into())),
                });
            }
            "--max-diagnostics" => {
                once(max_diagnostics.is_some())?;
                let n = value()?;
                max_diagnostics = Some(n.to_str().and_then(|n| n.parse().ok()).ok_or_else(|| {
                    Usage(format!(
                        "--max-diagnostics takes a count (0 or more), not '{}'",
                        n.to_string_lossy()
                    ))
                })?);
            }
            "--summary-only" => summary_only = flag(summary_only)?,
            "--switch" => settings.push(switch_setting(&value()?)?),
            "--complete" => complete = flag(complete)?,
            "--load-snapshot" => {
                once(load_snapshot.is_some())?;
                load_snapshot = Some(value()?);
            }
            "--save-snapshot" => {
                once(save_snapshot.is_some())?;
                save_snapshot = Some(value()?);
            }
            "--explain" => explain.push(explain_triple(&value()?)?),
            "--explain-diagnostics" => explain_diagnostics = flag(explain_diagnostics)?,
            _ => return Err(Usage(format!("unknown option '{name}'"))),
        }
    }

    if inputs.is_empty() && load_snapshot.is_none() {
        return Err(Usage("no input file".into()));
    }
    for input in &inputs {
        if format_of(input).is_none() {
            return Err(Usage(format!(
                "{}: cannot tell the format from the extension (use .ttl, .nt, .n3, .rdf, .owl or .xml)",
                input.display()
            )));
        }
    }
    if summary_only && (output.is_some() || format.is_some() || inferred_only) {
        return Err(Usage(
            "--summary-only writes no closure: it takes no -o, --format or --inferred-only".into(),
        ));
    }
    if summary_only && explain_diagnostics {
        return Err(Usage(
            "--summary-only lists no diagnostic: it takes no --explain-diagnostics".into(),
        ));
    }
    let output = output.filter(|o| o != "-").map(PathBuf::from);
    let format = match (format, &output) {
        (Some(format), _) => format,
        (None, None) => Format::Turtle,
        (None, Some(path)) => format_of(path).ok_or_else(|| {
            Usage(format!(
                "{}: cannot tell the output format from the extension: give --format",
                path.display()
            ))
        })?,
    };
    Ok(Command::Run(Options {
        inputs,
        output,
        format,
        inferred_only,
        report: report.unwrap_or(Report::Text),
        fail_on: fail_on.unwrap_or(Some(Severity::Error)),
        max_diagnostics: max_diagnostics.unwrap_or(usize::MAX),
        summary_only,
        complete,
        settings,
        load_snapshot: load_snapshot.map(PathBuf::from),
        save_snapshot: save_snapshot.map(PathBuf::from),
        explain,
        explain_diagnostics,
    }))
}

/// An `--explain` value: one N-Triples triple, its final ` .` optional.
fn explain_triple(value: &OsStr) -> Result<Triple, Usage> {
    let shown = value.to_string_lossy();
    let bad = |why: &str| {
        Usage(format!(
            "--explain takes one N-Triples triple, as '<s> <p> <o>', not '{shown}' ({why})"
        ))
    };
    let text = value.to_str().ok_or_else(|| bad("not UTF-8"))?.trim();
    let line = if text.ends_with('.') {
        text.to_owned()
    } else {
        format!("{text} .")
    };
    let mut triples = oxttl::NTriplesParser::new().for_slice(line.as_bytes());
    let triple = triples
        .next()
        .ok_or_else(|| bad("no triple"))?
        .map_err(|e| bad(&e.to_string()))?;
    match triples.next() {
        None => Ok(triple),
        Some(_) => Err(bad("more than one triple")),
    }
}

/// A `--switch` value: `NAME=on` or `NAME=off`.
fn switch_setting(value: &OsStr) -> Result<(Switch, bool), Usage> {
    let shown = value.to_string_lossy();
    let (name, on) = match shown.split_once('=') {
        Some((name, "on")) => (name, true),
        Some((name, "off")) => (name, false),
        _ => return Err(Usage(format!("--switch takes NAME=on or NAME=off, not '{shown}'"))),
    };
    let switch = Switch::from_name(name).ok_or_else(|| {
        let names: Vec<&str> = Switch::ALL.into_iter().map(Switch::name).collect();
        Usage(format!("unknown switch '{name}' (the switches: {})", names.join(", ")))
    })?;
    Ok((switch, on))
}

/// A `--format` value: a format's name, or one of its extensions.
fn output_format(value: &OsStr) -> Result<Format, Usage> {
    let format = value.to_str().and_then(|v| match v.to_ascii_lowercase().as_str() {
        "turtle" => Some(Format::Turtle),
        "ntriples" | "n-triples" => Some(Format::NTriples),
        "n3" => Some(Format::N3),
        "rdfxml" | "rdf/xml" => Some(Format::RdfXml),
        other => Format::from_extension(other),
    });
    format.ok_or_else(|| {
        Usage(format!(
            "--format takes turtle, ntriples, n3 or rdfxml, not '{}'",
            value.to_string_lossy()
        ))
    })
}

/// The format a path's extension names.
fn format_of(path: &Path) -> Option<Format> {
    Format::from_extension(path.extension()?.to_str()?)
}
