//! The `turbowl` binary, run as a user runs it: every flag, every exit
//! code, the report in each form, and the closure it writes compared with
//! the library's (`turbowl::Reasoner` called directly) as sets.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use turbowl::{Format, Reasoner, Switch, Switches};

/// A schema: a subclass, a range, a symmetric property, a language tag, a
/// blank node.
const SCHEMA: &str = r#"
@prefix ex: <http://example.org/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
ex:Dog rdfs:subClassOf ex:Animal ; rdfs:label "chien"@fr .
ex:hasOwner rdfs:range ex:Person .
ex:knows a owl:SymmetricProperty .
_:b ex:p ex:o .
"#;

/// Data for the schema, in another format and another document: its `_:b`
/// is not the schema's.
const DATA: &str = "\
<http://example.org/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Dog> .
<http://example.org/rex> <http://example.org/hasOwner> <http://example.org/ann> .
<http://example.org/ann> <http://example.org/knows> <http://example.org/bob> .
_:b <http://example.org/q> <http://example.org/o> .
";

/// More data, as RDF/XML.
const MORE: &str = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="http://example.org/fido">
    <rdf:type rdf:resource="http://example.org/Dog"/>
  </rdf:Description>
</rdf:RDF>
"#;

/// Two errors: an individual in two disjoint classes (cax-dw) and an
/// ill-typed literal (dt-not-type).
const INCONSISTENT: &str = r#"
@prefix ex: <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:Dog owl:disjointWith ex:Cat .
ex:tom a ex:Dog, ex:Cat ; ex:age "old"^^xsd:integer .
"#;

/// Equality and a property chain, for the switches: `ex:b ex:p ex:c` takes
/// `equality`, `ex:x ex:grandparent ex:z` takes `property-chains`. The list
/// nodes are named, so two parses give the same triples.
const SWITCHED: &str = r#"
@prefix ex: <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
ex:a owl:sameAs ex:b ; ex:p ex:c .
ex:grandparent owl:propertyChainAxiom ex:chain .
ex:chain rdf:first ex:parent ; rdf:rest ex:rest .
ex:rest rdf:first ex:parent ; rdf:rest rdf:nil .
ex:x ex:parent ex:y . ex:y ex:parent ex:z .
"#;

/// A finished run of the binary.
#[derive(Debug)]
struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn turbowl<S: AsRef<std::ffi::OsStr>>(args: &[S]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_turbowl"))
        .args(args)
        .output()
        .expect("the binary runs");
    Run {
        code: out.status.code().expect("an exit code, not a signal"),
        stdout: String::from_utf8(out.stdout).expect("utf-8 stdout"),
        stderr: String::from_utf8(out.stderr).expect("utf-8 stderr"),
    }
}

/// A fresh directory for one test, with the fixtures written in it.
fn fixtures(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli").join(test);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, text) in [
        ("schema.ttl", SCHEMA),
        ("data.nt", DATA),
        ("more.rdf", MORE),
        ("bad.ttl", INCONSISTENT),
        ("data.n3", DATA),
        ("switched.ttl", SWITCHED),
    ] {
        std::fs::write(dir.join(name), text).unwrap();
    }
    dir
}

fn arg(path: &Path) -> String {
    path.to_str().unwrap().to_owned()
}

/// The library's closure (or inferred triples) of `inputs`, as N-Triples lines.
fn library(inputs: &[PathBuf], inferred_only: bool) -> BTreeSet<String> {
    library_with(Switches::default(), inputs, inferred_only)
}

/// As [`library`], with these switches.
fn library_with(switches: Switches, inputs: &[PathBuf], inferred_only: bool) -> BTreeSet<String> {
    let mut r = Reasoner::with_switches(switches);
    for input in inputs {
        r.load_file(input).unwrap();
    }
    r.reason();
    if inferred_only {
        r.inferred().map(|t| t.to_string()).collect()
    } else {
        r.triples().map(|t| t.to_string()).collect()
    }
}

/// The triples of a document the binary wrote, as N-Triples lines.
fn parsed(text: &str, format: Format) -> BTreeSet<String> {
    let bytes = text.as_bytes();
    match format {
        Format::Turtle => oxttl::TurtleParser::new()
            .for_slice(bytes)
            .map(|t| t.unwrap().to_string())
            .collect(),
        Format::NTriples => oxttl::NTriplesParser::new()
            .for_slice(bytes)
            .map(|t| t.unwrap().to_string())
            .collect(),
        Format::N3 => oxttl::N3Parser::new()
            .for_slice(bytes)
            .map(|q| q.unwrap().to_string())
            .collect(),
        Format::RdfXml => oxrdfxml::RdfXmlParser::new()
            .for_slice(bytes)
            .map(|t| t.unwrap().to_string())
            .collect(),
    }
}

fn json(line: &str) -> serde_json::Value {
    serde_json::from_str(line).unwrap_or_else(|e| panic!("not JSON ({e}): {line}"))
}

// The closure.

#[test]
fn the_closure_on_stdout_is_the_librarys_as_turtle() {
    let dir = fixtures("stdout");
    let inputs = [dir.join("schema.ttl"), dir.join("data.nt")];
    let run = turbowl(&[arg(&inputs[0]), arg(&inputs[1])]);
    assert_eq!(run.code, 0, "{run:?}");
    let expected = library(&inputs, false);
    assert!(expected.contains(
        "<http://example.org/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Animal>"
    ));
    assert_eq!(parsed(&run.stdout, Format::Turtle), expected);
}

#[test]
fn every_input_format_loads_together() {
    let dir = fixtures("together");
    let inputs = [dir.join("schema.ttl"), dir.join("data.n3"), dir.join("more.rdf")];
    let out = dir.join("closure.nt");
    let run = turbowl(&[
        arg(&inputs[0]),
        arg(&inputs[1]),
        arg(&inputs[2]),
        "-o".into(),
        arg(&out),
    ]);
    assert_eq!(run.code, 0, "{run:?}");
    let closure = parsed(&std::fs::read_to_string(&out).unwrap(), Format::NTriples);
    assert_eq!(closure, library(&inputs, false));
    // Inferences cross files: the schema types the other files' individuals.
    for t in [
        "<http://example.org/fido> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Animal>",
        "<http://example.org/ann> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person>",
        "<http://example.org/bob> <http://example.org/knows> <http://example.org/ann>",
    ] {
        assert!(closure.contains(t), "{t}");
    }
    // The two files' `_:b` stay two blank nodes; the language tag survives.
    let blanks: BTreeSet<&str> = closure
        .iter()
        .filter(|t| t.ends_with("<http://example.org/o>"))
        .map(|t| t.split(' ').next().unwrap())
        .collect();
    assert_eq!(blanks.len(), 2, "{blanks:?}");
    assert!(closure.iter().any(|t| t.ends_with("\"chien\"@fr")));
}

#[test]
fn the_output_format_comes_from_the_extension_or_format() {
    let dir = fixtures("formats");
    let inputs = [dir.join("schema.ttl"), dir.join("data.nt")];
    let expected = library(&inputs, false);
    let cases = [
        ("out.ttl", None, Format::Turtle),
        ("out.nt", None, Format::NTriples),
        ("out.n3", None, Format::N3),
        ("out.rdf", None, Format::RdfXml),
        ("out.owl", None, Format::RdfXml),
        ("out.txt", Some("ntriples"), Format::NTriples),
        ("out.ttl", Some("rdfxml"), Format::RdfXml),
        ("out.data", Some("n3"), Format::N3),
        ("out.x", Some("turtle"), Format::Turtle),
        ("out.y", Some("nt"), Format::NTriples),
    ];
    for (name, format, expected_format) in cases {
        let out = dir.join(name);
        let mut args = vec![arg(&inputs[0]), arg(&inputs[1]), "-o".into(), arg(&out)];
        if let Some(format) = format {
            args.extend(["--format".into(), format.into()]);
        }
        let run = turbowl(&args);
        assert_eq!(run.code, 0, "{name} {format:?}: {run:?}");
        let text = std::fs::read_to_string(&out).unwrap();
        assert_eq!(parsed(&text, expected_format), expected, "{name} {format:?}");
    }
}

#[test]
fn format_on_stdout_and_dash_output() {
    let dir = fixtures("dash");
    let inputs = [dir.join("schema.ttl"), dir.join("data.nt")];
    let expected = library(&inputs, false);
    for args in [
        vec![arg(&inputs[0]), arg(&inputs[1]), "--format".into(), "ntriples".into()],
        vec![
            arg(&inputs[0]),
            arg(&inputs[1]),
            "--format=ntriples".into(),
            "-o".into(),
            "-".into(),
        ],
        vec![
            "--output=-".into(),
            "--format=nt".into(),
            arg(&inputs[0]),
            arg(&inputs[1]),
        ],
    ] {
        let run = turbowl(&args);
        assert_eq!(run.code, 0, "{args:?}: {run:?}");
        assert_eq!(parsed(&run.stdout, Format::NTriples), expected, "{args:?}");
        assert!(run.stderr.contains(" base, "), "the report goes to stderr: {run:?}");
    }
}

#[test]
fn inferred_only_writes_the_librarys_inferred_triples() {
    let dir = fixtures("inferred");
    let inputs = [dir.join("schema.ttl"), dir.join("data.nt")];
    let out = dir.join("inferred.nt");
    let run = turbowl(&[
        arg(&inputs[0]),
        arg(&inputs[1]),
        "--inferred-only".into(),
        "-o".into(),
        arg(&out),
    ]);
    assert_eq!(run.code, 0, "{run:?}");
    let inferred = parsed(&std::fs::read_to_string(&out).unwrap(), Format::NTriples);
    assert_eq!(inferred, library(&inputs, true));
    assert!(!inferred.is_empty());
    assert!(
        !inferred.iter().any(|t| t.contains("\"chien\"@fr")),
        "a base triple was written"
    );
}

#[test]
fn files_after_double_dash_are_inputs() {
    let dir = fixtures("dashdash");
    let odd = dir.join("-odd.ttl");
    std::fs::write(&odd, SCHEMA).unwrap();
    let run = turbowl(&["--format", "nt", "--", &arg(&odd)]);
    assert_eq!(run.code, 0, "{run:?}");
    assert_eq!(parsed(&run.stdout, Format::NTriples), library(&[odd], false));
}

// The report.

#[test]
fn the_report_goes_to_stdout_unless_the_closure_does() {
    let dir = fixtures("where");
    let bad = arg(&dir.join("bad.ttl"));
    let to_stdout = turbowl(&[&bad, "--fail-on", "never", "--format", "nt"]);
    assert!(to_stdout.stderr.contains("error[cax-dw]: "), "{to_stdout:?}");
    assert!(!to_stdout.stdout.contains("error["), "{to_stdout:?}");

    let out = arg(&dir.join("closure.ttl"));
    let to_file = turbowl(&[&bad, "--fail-on", "never", "-o", &out]);
    assert!(to_file.stdout.contains("error[cax-dw]: "), "{to_file:?}");
    assert_eq!(to_file.stderr, "");
}

#[test]
fn text_lists_each_diagnostic_then_the_summary() {
    let dir = fixtures("text");
    let out = arg(&dir.join("closure.ttl"));
    let run = turbowl(&[&arg(&dir.join("bad.ttl")), "-o", &out, "--diagnostics", "text"]);
    assert_eq!(run.code, 2, "{run:?}");
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 3, "{run:?}");
    assert!(lines[0].starts_with("error[cax-dw]: "), "{}", lines[0]);
    assert!(lines[1].starts_with("error[dt-not-type]: "), "{}", lines[1]);
    assert!(
        lines[2].contains(" base, ") && lines[2].contains("2 diagnostics"),
        "{}",
        lines[2]
    );
}

#[test]
fn json_is_one_object_with_the_summary_and_every_diagnostic() {
    let dir = fixtures("json");
    let input = dir.join("bad.ttl");
    let out = arg(&dir.join("closure.ttl"));
    let run = turbowl(&[&arg(&input), "-o", &out, "--diagnostics", "json"]);
    assert_eq!(run.code, 2, "{run:?}");
    let doc = json(&run.stdout);
    assert_eq!(run.stdout.lines().count(), 1);

    let mut r = Reasoner::new();
    r.load_file(&input).unwrap();
    let summary = r.reason();
    let s = &doc["summary"];
    assert_eq!(s["base"], summary.base);
    assert_eq!(s["inferred"], summary.inferred);
    assert_eq!(s["rounds"], summary.rounds);
    assert_eq!(s["diagnostics"], 2);
    assert!(s["load_seconds"].as_f64().unwrap() >= 0.0);
    assert!(s["reason_seconds"].as_f64().unwrap() >= 0.0);

    let listed = doc["diagnostics"].as_array().unwrap();
    assert_eq!(listed.len(), r.diagnostics().len());
    for (d, expected) in listed.iter().zip(r.diagnostics()) {
        assert_eq!(d["code"], expected.code);
        assert_eq!(d["severity"], "error");
        assert_eq!(d["message"], expected.message.as_str());
        let terms: Vec<String> = expected.terms.iter().map(|t| t.to_string()).collect();
        assert_eq!(d["terms"], serde_json::json!(terms));
    }
    assert_eq!(
        listed[0]["terms"],
        serde_json::json!([
            "<http://example.org/tom>",
            "<http://example.org/Dog>",
            "<http://example.org/Cat>"
        ])
    );
    assert_eq!(listed[1]["code"], "dt-not-type");
    assert_eq!(
        listed[1]["terms"][0], "\"old\"^^<http://www.w3.org/2001/XMLSchema#integer>",
        "terms are N-Triples strings"
    );
}

#[test]
fn ndjson_is_a_line_per_diagnostic_then_the_summary() {
    let dir = fixtures("ndjson");
    let out = arg(&dir.join("closure.ttl"));
    let run = turbowl(&[&arg(&dir.join("bad.ttl")), "-o", &out, "--diagnostics=ndjson"]);
    assert_eq!(run.code, 2, "{run:?}");
    let records: Vec<serde_json::Value> = run.stdout.lines().map(json).collect();
    assert_eq!(records.len(), 3, "{run:?}");
    assert_eq!(records[0]["type"], "diagnostic");
    assert_eq!(records[0]["code"], "cax-dw");
    assert_eq!(records[1]["type"], "diagnostic");
    assert_eq!(records[1]["code"], "dt-not-type");
    assert!(records[1]["terms"].is_array() && records[1]["message"].is_string());
    assert_eq!(records[2]["type"], "summary");
    assert_eq!(records[2]["diagnostics"], 2);
}

#[test]
fn json_on_stderr_parses_when_the_closure_takes_stdout() {
    let dir = fixtures("stderr-json");
    let input = dir.join("bad.ttl");
    let run = turbowl(&[&arg(&input), "--diagnostics", "json", "--format", "nt"]);
    assert_eq!(run.code, 2, "{run:?}");
    assert_eq!(json(&run.stderr)["summary"]["diagnostics"], 2);
    assert_eq!(parsed(&run.stdout, Format::NTriples), library(&[input], false));
}

#[test]
fn a_consistent_input_reports_no_diagnostic() {
    let dir = fixtures("clean");
    let run = turbowl(&[&arg(&dir.join("schema.ttl")), "--summary-only", "--diagnostics", "json"]);
    assert_eq!(run.code, 0, "{run:?}");
    let doc = json(&run.stdout);
    assert_eq!(doc["summary"]["diagnostics"], 0);
    assert_eq!(doc["diagnostics"], serde_json::json!([]));
}

#[test]
fn max_diagnostics_lists_fewer_and_the_summary_counts_all() {
    let dir = fixtures("max");
    let out = arg(&dir.join("closure.ttl"));
    let bad = arg(&dir.join("bad.ttl"));
    for (max, listed) in [("0", 0), ("1", 1), ("2", 2), ("50", 2)] {
        let run = turbowl(&[&bad, "-o", &out, "--diagnostics", "json", "--max-diagnostics", max]);
        assert_eq!(run.code, 2, "{run:?}");
        let doc = json(&run.stdout);
        assert_eq!(
            doc["diagnostics"].as_array().unwrap().len(),
            listed,
            "--max-diagnostics {max}"
        );
        assert_eq!(doc["summary"]["diagnostics"], 2);
    }
    let run = turbowl(&[&bad, "-o", &out, "--max-diagnostics=1"]);
    assert_eq!(run.stdout.lines().count(), 2, "one diagnostic and the summary: {run:?}");
}

#[test]
fn summary_only_writes_the_summary_and_no_closure() {
    let dir = fixtures("summary");
    let bad = arg(&dir.join("bad.ttl"));

    let text = turbowl(&[&bad, "--summary-only"]);
    assert_eq!(text.code, 2, "{text:?}");
    assert_eq!(text.stdout.lines().count(), 1, "{text:?}");
    assert!(text.stdout.contains(" base, ") && text.stdout.contains("2 diagnostics"));
    assert_eq!(text.stderr, "");

    let doc = json(&turbowl(&[&bad, "--summary-only", "--diagnostics", "json"]).stdout);
    assert_eq!(doc["diagnostics"], serde_json::json!([]));
    assert_eq!(doc["summary"]["diagnostics"], 2);

    let nd = turbowl(&[&bad, "--summary-only", "--diagnostics", "ndjson"]);
    assert_eq!(nd.stdout.lines().count(), 1);
    assert_eq!(json(&nd.stdout)["type"], "summary");
}

// The switches.

/// The names of the switches that are on, as the summary lists them.
fn names_on(switches: Switches) -> serde_json::Value {
    let names: Vec<&str> = Switch::ALL
        .into_iter()
        .filter(|&s| switches.is_on(s))
        .map(Switch::name)
        .collect();
    serde_json::json!(names)
}

/// Runs `switched.ttl` with `flags`: the closure must be the library's with
/// `expected`, and the summary must name `expected`.
fn check_switches(dir: &Path, flags: &[&str], expected: Switches) {
    let input = dir.join("switched.ttl");
    let mut args = vec![
        arg(&input),
        "--format".into(),
        "nt".into(),
        "--diagnostics".into(),
        "json".into(),
    ];
    args.extend(flags.iter().map(|f| f.to_string()));
    let run = turbowl(&args);
    assert_eq!(run.code, 0, "{flags:?}: {run:?}");
    let closure = parsed(&run.stdout, Format::NTriples);
    assert_eq!(closure, library_with(expected, &[input], false), "{flags:?}");
    assert_eq!(
        json(&run.stderr)["summary"]["switches"],
        names_on(expected),
        "{flags:?}"
    );
    let has = |t: &str| closure.contains(t);
    assert_eq!(
        has("<http://example.org/b> <http://example.org/p> <http://example.org/c>"),
        expected.is_on(Switch::Equality),
        "{flags:?}"
    );
    assert_eq!(
        has("<http://example.org/x> <http://example.org/grandparent> <http://example.org/z>"),
        expected.is_on(Switch::PropertyChains),
        "{flags:?}"
    );
}

#[test]
fn switch_turns_one_switch_on_or_off_and_a_later_one_wins() {
    let dir = fixtures("switch");
    let default = Switches::default();
    let cases: Vec<(Vec<&str>, Switches)> = vec![
        (vec![], default),
        (vec!["--switch", "equality=off"], default.off(Switch::Equality)),
        (
            vec!["--switch=property-chains=off"],
            default.off(Switch::PropertyChains),
        ),
        (
            vec!["--switch", "equality=off", "--switch", "property-chains=off"],
            default.off(Switch::Equality).off(Switch::PropertyChains),
        ),
        (vec!["--switch", "trivial=on"], default.on(Switch::Trivial)),
        (vec!["--switch", "equality=off", "--switch", "equality=on"], default),
        (
            vec!["--switch", "equality=on", "--switch", "equality=off"],
            default.off(Switch::Equality),
        ),
    ];
    for (flags, expected) in cases {
        check_switches(&dir, &flags, expected);
    }
}

#[test]
fn complete_starts_from_every_switch_on_wherever_it_comes() {
    let dir = fixtures("complete");
    let complete = Switches::complete();
    let cases: Vec<(Vec<&str>, Switches)> = vec![
        (vec!["--complete"], complete),
        (
            vec!["--complete", "--switch", "equality=off"],
            complete.off(Switch::Equality),
        ),
        (
            vec!["--switch", "equality=off", "--complete"],
            complete.off(Switch::Equality),
        ),
        (vec!["--complete", "--switch", "trivial=on"], complete),
    ];
    for (flags, expected) in cases {
        check_switches(&dir, &flags, expected);
    }
}

#[test]
fn checks_off_reports_no_diagnostic_and_exits_0() {
    let dir = fixtures("checks-off");
    let bad = arg(&dir.join("bad.ttl"));
    let run = turbowl(&[
        &bad,
        "--summary-only",
        "--diagnostics",
        "json",
        "--switch",
        "checks=off",
    ]);
    assert_eq!(run.code, 0, "{run:?}");
    let summary = &json(&run.stdout)["summary"];
    assert_eq!(summary["diagnostics"], 0);
    assert_eq!(summary["switches"], names_on(Switches::default().off(Switch::Checks)));

    let again = turbowl(&[
        &bad,
        "--summary-only",
        "--switch",
        "checks=off",
        "--switch",
        "checks=on",
    ]);
    assert_eq!(again.code, 2, "the later --switch wins: {again:?}");
}

#[test]
fn the_json_and_ndjson_summaries_name_the_switches_that_were_on() {
    let dir = fixtures("summary-switches");
    let schema = arg(&dir.join("schema.ttl"));
    let doc = json(&turbowl(&[&schema, "--summary-only", "--diagnostics", "json"]).stdout);
    assert_eq!(
        doc["summary"]["switches"],
        serde_json::json!(["equality", "property-chains", "checks"])
    );
    let nd = turbowl(&[&schema, "--summary-only", "--diagnostics", "ndjson", "--complete"]);
    let record = json(&nd.stdout);
    assert_eq!(record["type"], "summary");
    assert_eq!(
        record["switches"],
        serde_json::json!([
            "equality",
            "property-chains",
            "checks",
            "identity",
            "schema",
            "trivial",
            "literal-values"
        ])
    );
}

#[test]
fn switches_lists_each_switch_with_its_default_and_exits_0() {
    let dir = fixtures("switches");
    let alone = turbowl(&["--switches"]);
    assert_eq!(alone.code, 0, "{alone:?}");
    assert_eq!(alone.stderr, "");
    // A switch's block starts with its name and its default; its text
    // carries on in indented lines.
    let firsts: Vec<Vec<&str>> = alone
        .stdout
        .lines()
        .skip(1)
        .filter(|l| !l.starts_with("   "))
        .map(|l| l.split_whitespace().take(2).collect())
        .collect();
    let expected: Vec<Vec<&str>> = Switch::ALL
        .into_iter()
        .map(|s| vec![s.name(), if Switches::default().is_on(s) { "on" } else { "off" }])
        .collect();
    assert_eq!(firsts, expected, "{}", alone.stdout);
    assert!(
        alone.stdout.contains("dt-diff only checks"),
        "it says what a switch costs"
    );

    // With inputs, it lists and runs nothing.
    let with_inputs = turbowl(&[&arg(&dir.join("schema.ttl")), "--switches"]);
    assert_eq!(with_inputs.code, 0, "{with_inputs:?}");
    assert_eq!(with_inputs.stdout, alone.stdout);
    // --help shows the same listing.
    assert!(turbowl(&["--help"]).stdout.contains(&alone.stdout));
}

#[test]
fn an_unknown_switch_or_a_bad_value_exits_64() {
    let dir = fixtures("bad-switch");
    let schema = arg(&dir.join("schema.ttl"));
    let unknown = turbowl(&[&schema, "--switch", "sameas=off"]);
    assert_eq!(unknown.code, 64, "{unknown:?}");
    assert!(
        unknown
            .stderr
            .starts_with("turbowl: unknown switch 'sameas' (the switches: equality, property-chains,"),
        "{unknown:?}"
    );
    let bad = turbowl(&[&schema, "--switch", "checks=no"]);
    assert_eq!(bad.code, 64, "{bad:?}");
    assert!(
        bad.stderr
            .starts_with("turbowl: --switch takes NAME=on or NAME=off, not 'checks=no'"),
        "{bad:?}"
    );
    for value in [
        "checks",
        "checks=",
        "=on",
        "checks=On",
        "checks=true",
        "Checks=on",
        "property_chains=off",
        "checks=on=off",
    ] {
        let run = turbowl(&[&schema, "--switch", value]);
        assert_eq!(run.code, 64, "--switch {value}: {run:?}");
        assert_eq!(run.stdout, "", "--switch {value}");
    }
}

// Exit codes.

#[test]
fn fail_on_sets_the_exit_code_and_the_closure_is_still_written() {
    let dir = fixtures("fail-on");
    let bad = dir.join("bad.ttl");
    let clean = arg(&dir.join("schema.ttl"));
    let out = dir.join("closure.nt");
    for (fail_on, code) in [(None, 2), (Some("error"), 2), (Some("warning"), 2), (Some("never"), 0)] {
        let _ = std::fs::remove_file(&out);
        let mut args = vec![arg(&bad), "-o".into(), arg(&out)];
        if let Some(fail_on) = fail_on {
            args.extend(["--fail-on".into(), fail_on.into()]);
        }
        let run = turbowl(&args);
        assert_eq!(run.code, code, "--fail-on {fail_on:?}: {run:?}");
        let closure = parsed(&std::fs::read_to_string(&out).unwrap(), Format::NTriples);
        assert_eq!(closure, library(std::slice::from_ref(&bad), false));
    }
    for fail_on in ["error", "warning"] {
        let run = turbowl(&[&clean, "--summary-only", "--fail-on", fail_on]);
        assert_eq!(run.code, 0, "--fail-on {fail_on} on a consistent input: {run:?}");
    }
}

#[test]
fn input_and_output_errors_exit_1() {
    let dir = fixtures("io");
    let schema = arg(&dir.join("schema.ttl"));

    let missing = turbowl(&[&schema, &arg(&dir.join("missing.ttl"))]);
    assert_eq!(missing.code, 1, "{missing:?}");
    assert!(missing.stderr.starts_with("turbowl: error: ") && missing.stderr.contains("missing.ttl"));
    assert_eq!(missing.stdout, "");

    let unwritable = arg(&dir.join("no-such-dir").join("closure.ttl"));
    let run = turbowl(&[&schema, "-o", &unwritable]);
    assert_eq!(run.code, 1, "{run:?}");
    assert!(run.stderr.contains("closure.ttl"), "{run:?}");
}

#[test]
fn a_parse_error_exits_1_with_its_line_and_writes_nothing() {
    let dir = fixtures("parse");
    let broken = dir.join("broken.ttl");
    std::fs::write(
        &broken,
        "@prefix ex: <http://example.org/> .\nex:a ex:b ex:c .\nex:a ex:b .\n",
    )
    .unwrap();
    let out = dir.join("closure.ttl");
    let run = turbowl(&[arg(&dir.join("schema.ttl")), arg(&broken), "-o".into(), arg(&out)]);
    assert_eq!(run.code, 1, "{run:?}");
    assert!(run.stderr.contains("broken.ttl: 3:"), "the line of the error: {run:?}");
    assert_eq!(run.stderr.lines().count(), 1, "{run:?}");
    assert!(!out.exists(), "no closure after a parse error");
}

#[test]
fn usage_errors_exit_64() {
    let dir = fixtures("usage");
    let schema = arg(&dir.join("schema.ttl"));
    let odd = arg(&dir.join("model.txt"));
    std::fs::write(&odd, SCHEMA).unwrap();
    let cases: Vec<Vec<&str>> = vec![
        vec![],
        vec!["--fail-on", "never"],
        vec![&schema, "--frobnicate"],
        vec![&schema, "-x"],
        vec![&schema, "--diagnostics", "xml"],
        vec![&schema, "--diagnostics"],
        vec![&schema, "--fail-on", "info"],
        vec![&schema, "--max-diagnostics", "-1"],
        vec![&schema, "--max-diagnostics", "many"],
        vec![&schema, "--format", "jsonld"],
        vec![&schema, "-o"],
        vec![&schema, "-o", "closure.txt"],
        vec![&schema, "-o", "a.ttl", "-o", "b.ttl"],
        vec![&schema, "--inferred-only", "--inferred-only"],
        vec![&schema, "--inferred-only=yes"],
        vec![&schema, "--summary-only", "-o", "closure.ttl"],
        vec![&schema, "--summary-only", "--format", "nt"],
        vec![&schema, "--summary-only", "--inferred-only"],
        vec![&schema, "--switch"],
        vec![&schema, "--switch", "bogus=on"],
        vec![&schema, "--switch", "checks=maybe"],
        vec![&schema, "--complete", "--complete"],
        vec![&schema, "--complete=yes"],
        vec![&schema, "--load-snapshot"],
        vec![&schema, "--save-snapshot"],
        vec![&schema, "--load-snapshot", "a", "--load-snapshot", "b"],
        vec![&schema, "--save-snapshot", "a", "--save-snapshot", "b"],
        vec![&odd],
        vec!["-"],
    ];
    for args in cases {
        let run = turbowl(&args);
        assert_eq!(run.code, 64, "{args:?}: {run:?}");
        assert!(run.stderr.starts_with("turbowl: "), "{args:?}: {run:?}");
        assert!(run.stderr.contains("--help"), "{args:?}: {run:?}");
        assert_eq!(run.stdout, "", "{args:?}");
    }
}

#[test]
fn help_and_version_exit_0() {
    for flag in ["--help", "-h"] {
        let run = turbowl(&[flag]);
        assert_eq!(run.code, 0, "{run:?}");
        assert!(run.stdout.starts_with("turbowl: "), "{run:?}");
        for option in [
            "Usage: turbowl",
            "--output",
            "--format",
            "--inferred-only",
            "--diagnostics",
            "--fail-on",
            "--max-diagnostics",
            "--summary-only",
            "--switch NAME=on|off",
            "--complete",
            "--switches",
            "--save-snapshot <PATH>",
            "--load-snapshot <PATH>",
            "--version",
            "Exit status",
            "64",
        ] {
            assert!(run.stdout.contains(option), "the help names {option}");
        }
        for switch in Switch::ALL {
            assert!(run.stdout.contains(switch.name()), "the help names {}", switch.name());
        }
    }
    for flag in ["--version", "-V"] {
        let run = turbowl(&[flag]);
        assert_eq!(run.code, 0, "{run:?}");
        assert_eq!(run.stdout, format!("turbowl {}\n", env!("CARGO_PKG_VERSION")));
    }
    // Help wins over a missing input.
    assert_eq!(turbowl(&["--diagnostics", "json", "--help"]).code, 0);
}

#[test]
fn a_reader_closing_stdout_early_is_not_an_error() {
    use std::io::Read;
    let dir = fixtures("pipe");
    // A closure far larger than a pipe's buffer.
    let big = dir.join("big.nt");
    let lines: String = (0..20_000)
        .map(|i| format!("<http://example.org/i{i}> <http://example.org/p> <http://example.org/o{i}> .\n"))
        .collect();
    std::fs::write(&big, lines).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_turbowl"))
        .args([arg(&big), "--format".into(), "nt".into()])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut first = [0u8; 100];
    child.stdout.take().unwrap().read_exact(&mut first).unwrap();
    // stdout is dropped: the reader is gone.
    let out = child.wait_with_output().unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(!stderr.contains("error"), "{stderr}");
    assert!(stderr.contains("20000 base"), "the report still comes: {stderr}");
}

// Snapshots.

/// The summary of a run with `--diagnostics json` and the closure on stdout.
fn summary(run: &Run) -> serde_json::Value {
    json(&run.stderr)["summary"].clone()
}

/// A run with `args`, writing the closure as N-Triples to stdout and the
/// report as JSON to stderr.
fn run_json(args: &[&str]) -> Run {
    let mut all = args.to_vec();
    all.extend(["--format", "nt", "--diagnostics", "json"]);
    turbowl(&all)
}

#[test]
fn a_snapshot_saves_the_state_and_loading_it_goes_on_from_there() {
    let dir = fixtures("snapshot");
    let (schema, data, more) = (dir.join("schema.ttl"), dir.join("data.nt"), dir.join("more.rdf"));
    let saved = arg(&dir.join("state.snapshot"));

    let first = run_json(&[&arg(&schema), &arg(&data), "--save-snapshot", &saved]);
    assert_eq!(first.code, 0, "{first:?}");
    assert!(std::fs::metadata(&saved).unwrap().len() > 0);

    // The saved state alone: the same closure, and no round to run.
    let loaded = run_json(&["--load-snapshot", &saved]);
    assert_eq!(loaded.code, 0, "{loaded:?}");
    assert_eq!(
        parsed(&loaded.stdout, Format::NTriples),
        library(&[schema.clone(), data.clone()], false)
    );
    assert_eq!(loaded.stdout, first.stdout, "the same closure, in the same order");
    let (s1, s2) = (summary(&first), summary(&loaded));
    assert_eq!((&s2["base"], &s2["inferred"]), (&s1["base"], &s1["inferred"]));
    assert_eq!(s2["rounds"], 0);

    // More files: added to the saved state, as to the reasoner that was saved.
    let grown = run_json(&["--load-snapshot", &saved, &arg(&more)]);
    assert_eq!(grown.code, 0, "{grown:?}");
    assert_eq!(
        parsed(&grown.stdout, Format::NTriples),
        library(&[schema, data, more.clone()], false)
    );

    // Loading and saving one file: the new state replaces the old.
    let again = turbowl(&[
        "--load-snapshot",
        &saved,
        &arg(&more),
        "--save-snapshot",
        &saved,
        "--summary-only",
    ]);
    assert_eq!(again.code, 0, "{again:?}");
    assert_eq!(run_json(&["--load-snapshot", &saved]).stdout, grown.stdout);
    let names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert!(
        names.iter().all(|n| !n.to_string_lossy().ends_with(".tmp")),
        "no temporary file is left: {names:?}"
    );
}

#[test]
fn a_loaded_snapshot_keeps_its_switches_unless_told() {
    let dir = fixtures("snapshot-switches");
    let input = dir.join("switched.ttl");
    let saved = arg(&dir.join("state.snapshot"));
    let off = Switches::default().off(Switch::Equality);
    let run = turbowl(&[
        &arg(&input),
        "--switch",
        "equality=off",
        "--save-snapshot",
        &saved,
        "--summary-only",
    ]);
    assert_eq!(run.code, 0, "{run:?}");
    let cases: Vec<(Vec<&str>, Switches)> = vec![
        (vec![], off),
        (vec!["--switch", "property-chains=off"], off.off(Switch::PropertyChains)),
        (vec!["--complete"], Switches::complete()),
    ];
    for (flags, expected) in cases {
        let mut args = vec!["--load-snapshot", saved.as_str()];
        args.extend(&flags);
        let run = run_json(&args);
        assert_eq!(run.code, 0, "{flags:?}: {run:?}");
        assert_eq!(summary(&run)["switches"], names_on(expected), "{flags:?}");
        assert_eq!(
            parsed(&run.stdout, Format::NTriples),
            library_with(expected, std::slice::from_ref(&input), false),
            "{flags:?}"
        );
    }
}

#[test]
fn a_refused_snapshot_exits_1() {
    let dir = fixtures("snapshot-refused");
    let schema = arg(&dir.join("schema.ttl"));
    let saved = dir.join("state.snapshot");
    assert_eq!(
        turbowl(&[&schema, "--save-snapshot", &arg(&saved), "--summary-only"]).code,
        0
    );
    let bytes = std::fs::read(&saved).unwrap();
    let truncated = dir.join("truncated.snapshot");
    std::fs::write(&truncated, &bytes[..bytes.len() / 2]).unwrap();
    let cases = [
        (dir.join("schema.ttl"), "not a TurbOWL snapshot"),
        (truncated, "the snapshot is truncated"),
        (dir.join("missing.snapshot"), "missing.snapshot"),
    ];
    for (path, says) in cases {
        let run = turbowl(&["--load-snapshot", &arg(&path)]);
        assert_eq!(run.code, 1, "{run:?}");
        assert!(run.stderr.starts_with("turbowl: error: "), "{run:?}");
        assert!(run.stderr.contains(says) && run.stderr.contains(&arg(&path)), "{run:?}");
        assert_eq!(run.stderr.lines().count(), 1, "{run:?}");
        assert_eq!(run.stdout, "");
    }
    // A snapshot that cannot be written: exit 1, and no closure.
    let run = turbowl(&[&schema, "--save-snapshot", &arg(&dir.join("no-such-dir").join("s"))]);
    assert_eq!(run.code, 1, "{run:?}");
    assert!(run.stderr.contains("no-such-dir"), "{run:?}");
    assert_eq!(run.stdout, "");
}
