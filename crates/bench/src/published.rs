//! Published results: RDFox's numbers from its papers, the numbers of the
//! engines that ran next to it in the same table (`published.json`, from
//! `docs/research/competitors-2026-11.md` §1.2), and the estimates they allow
//! (ADR 0006: RDFox counts through its papers only).
//!
//! **The same job.** A paper counts the facts its program derives over the
//! data: the ontology is compiled into rules, not loaded as triples. This
//! reasoner loads the ontology as triples, so its comparable counts are:
//! before, the data's distinct triples; after, its closure over the data and
//! the ontology less the triples of its closure over the ontology alone that
//! are not data ([`Counts`]). The program must match the configuration:
//! one that axiomatises `owl:sameAs` needs `equality` on, one without
//! `owl:sameAs` rules needs it off. That is a property of the program, not
//! of its variant's name: Oxford's Claros L and UOBM L axiomatise equality,
//! its LUBM L has no `owl:sameAs` rule, CIKM 2019's L programs leave it out
//! ([`PublishedResult::equality`]). U and REW are never the same job
//! ([`Variant::same_job`]).
//!
//! **An estimate** ([`estimate`]) scales RDFox's published time to this
//! machine through a **bridge**, an engine that ran next to it in the paper
//! and runs here: RDFox here = the bridge's time here x RDFox's time / the
//! bridge's time in the paper. It is made only when both ran one thread in
//! the paper (the bridge runs one here), the variant matches, the comparable
//! counts match the paper's (to the precision printed, after a recorded
//! explained difference), the bridge ran here on the same scenario with
//! this reasoner's closure, and the time measures the same work in both
//! places (reasoning, or load plus reasoning). Otherwise it is refused, with
//! the reason.
//!
//! # The data file
//!
//! `papers`: `id`, `cite`, `url`, `where` (tables and pages), `machine`,
//! `note`. `results`: one per table row, with `paper`, `dataset`, `variant`
//! (`L`, `LE`, `U`, `L-eq AX`, `L-eq REW`, `U-eq AX`, `U-eq REW`, or another
//! rule set's name), `program`, `scenario` (the bench scenario on the same
//! data and variant, if any), `before` and `after` (fact counts as printed:
//! `18.8 M`, `1.5 G`, `66,751,196`), `memory` (as printed, RDFox's, when the
//! table gives one number per row), `note`, `explained` (`{"difference":
//! N, "why": "..."}`: this reasoner's after count is the paper's plus N, for
//! the reason given), `equality` (whether the program axiomatises
//! `owl:sameAs`; absent: not known), and `engines`: `engine` (`RDFox`, or the other
//! engine's name in the paper), `build`, `threads` (absent: not stated),
//! the seconds (`load_s`, `reasoning_s`, `total_s` for load plus reasoning
//! as one number, `unstated_s` when the paper does not say what its time
//! covers), `memory`, `outcome` (why there is no time: `out of memory`,
//! `timeout`), and `runs_here_as` (the bench engine it is, for a bridge).

use crate::engine::EngineId;
use serde_json::Value;
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
use std::path::PathBuf;
use std::sync::OnceLock;
use turbowl::{Reasoner, Switches};

const DATA: &str = include_str!("../published.json");

/// The engine name the papers give RDFox.
pub(crate) const RDFOX: &str = "RDFox";

#[derive(Debug)]
pub(crate) struct Paper {
    pub(crate) id: String,
    pub(crate) cite: String,
    pub(crate) url: String,
    pub(crate) place: String,
    pub(crate) machine: String,
}

/// One row of a paper's table.
#[derive(Debug)]
pub(crate) struct PublishedResult {
    pub(crate) paper: String,
    pub(crate) variant: Variant,
    /// Whether the paper's program axiomatises `owl:sameAs`, when known.
    pub(crate) equality: Option<bool>,
    pub(crate) program: Option<String>,
    pub(crate) scenario: Option<String>,
    pub(crate) before: Option<Count>,
    pub(crate) after: Option<Count>,
    pub(crate) explained: Option<Explained>,
    pub(crate) engines: Vec<EngineResult>,
}

impl PublishedResult {
    /// Whether this reasoner's `equality` switch must be on to do the
    /// paper's job; `Err` when no configuration does it, or it is not known.
    pub(crate) fn equality(&self) -> Result<bool, String> {
        self.variant.same_job()?;
        self.equality
            .ok_or_else(|| "whether the paper's program axiomatises owl:sameAs is not known".into())
    }
}

/// A recorded reason why this reasoner's after count differs from the
/// paper's by `difference` facts.
#[derive(Debug)]
pub(crate) struct Explained {
    pub(crate) difference: i64,
    pub(crate) why: String,
}

/// One engine's numbers in one row.
#[derive(Debug)]
pub(crate) struct EngineResult {
    pub(crate) engine: String,
    pub(crate) build: Option<String>,
    pub(crate) threads: Option<u32>,
    pub(crate) seconds: Seconds,
    pub(crate) memory: Option<String>,
    pub(crate) outcome: Option<String>,
    pub(crate) runs_here_as: Option<EngineId>,
}

/// A paper's time for one engine, split as the paper splits it.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Seconds {
    pub(crate) load: Option<f64>,
    pub(crate) reasoning: Option<f64>,
    /// Load plus reasoning, given as one number.
    pub(crate) total: Option<f64>,
    /// A time whose paper does not say whether it includes loading.
    pub(crate) unstated: Option<f64>,
}

impl Seconds {
    /// Load plus reasoning: given, or the sum of the two.
    fn total(self) -> Option<f64> {
        self.total.or(self.load.zip(self.reasoning).map(|(l, r)| l + r))
    }

    fn get(self, axis: TimeAxis) -> Option<f64> {
        match axis {
            TimeAxis::Reasoning => self.reasoning,
            TimeAxis::Total => self.total(),
        }
    }

    fn describe(self) -> String {
        let mut parts = Vec::new();
        if let Some(r) = self.reasoning {
            parts.push(format!("reasoning {r} s"));
        }
        if let Some(t) = self.total() {
            parts.push(format!("load plus reasoning {t} s"));
        }
        if let Some(u) = self.unstated {
            parts.push(format!("{u} s, loading included or not (not stated)"));
        }
        if parts.is_empty() {
            "no time".into()
        } else {
            parts.join(", ")
        }
    }
}

/// The two times an estimate can scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimeAxis {
    Reasoning,
    Total,
}

impl TimeAxis {
    pub(crate) fn name(self) -> &'static str {
        match self {
            TimeAxis::Reasoning => "reasoning",
            TimeAxis::Total => "load + reasoning",
        }
    }
}

/// A fact count as a paper prints it, with the precision it is printed to.
#[derive(Debug, Clone)]
pub(crate) struct Count {
    pub(crate) printed: String,
    value: f64,
    /// Half the last printed digit: a count within this of `value` prints the same.
    half_step: f64,
}

impl Count {
    /// `18.8 M`, `1,529.7 M`, `1.5 G`, `500 K` or `66,751,196`.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let (number, scale) = match text.trim().rsplit_once(' ') {
            Some((n, "K")) => (n, 1e3),
            Some((n, "M")) => (n, 1e6),
            Some((n, "G")) => (n, 1e9),
            None => (text.trim(), 1.0),
            Some(_) => return Err(format!("count {text:?}: the unit is K, M or G")),
        };
        let digits = number.replace(',', "");
        let value: f64 = digits.parse().map_err(|_| format!("count {text:?}: not a number"))?;
        let decimals = digits.split_once('.').map_or(0, |(_, d)| d.len()) as i32;
        Ok(Count {
            printed: text.to_string(),
            value: value * scale,
            half_step: 0.5 * scale * 10f64.powi(-decimals),
        })
    }

    /// The count printed, in facts.
    pub(crate) fn value(&self) -> f64 {
        self.value
    }

    /// True when `n` prints as this count.
    pub(crate) fn matches(&self, n: i64) -> bool {
        (n as f64 - self.value).abs() <= self.half_step
    }
}

/// A paper's program, as the papers name it (AIJ 2019 §10.1, AAAI 2014 §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Variant {
    /// The OWL 2 RL part of the ontology, translated to rules; no equality.
    L,
    /// L plus hand-written chain rules.
    Le,
    /// AAAI 2015's programs with equality axiomatised.
    LEqAx,
    /// The same, with equality by rewriting.
    LEqRew,
    /// The upper bound: existentials as fresh constants.
    U,
    UEqAx,
    UEqRew,
    /// Another rule set (RDFS-Plus, ChaseBench's).
    Other(String),
}

impl Variant {
    fn parse(name: &str) -> Self {
        match name {
            "L" => Variant::L,
            "LE" => Variant::Le,
            "L-eq AX" => Variant::LEqAx,
            "L-eq REW" => Variant::LEqRew,
            "U" => Variant::U,
            "U-eq AX" => Variant::UEqAx,
            "U-eq REW" => Variant::UEqRew,
            other => Variant::Other(other.to_string()),
        }
    }

    pub(crate) fn name(&self) -> &str {
        match self {
            Variant::L => "L",
            Variant::Le => "LE",
            Variant::LEqAx => "L-eq AX",
            Variant::LEqRew => "L-eq REW",
            Variant::U => "U",
            Variant::UEqAx => "U-eq AX",
            Variant::UEqRew => "U-eq REW",
            Variant::Other(name) => name,
        }
    }

    /// `Err` when no configuration of this reasoner does the job of a
    /// program of this variant.
    pub(crate) fn same_job(&self) -> Result<(), String> {
        match self {
            Variant::L | Variant::Le | Variant::LEqAx => Ok(()),
            Variant::LEqRew => Err(
                "REW counts one representative per owl:sameAs class; this reasoner's count expands them, as AX does"
                    .into(),
            ),
            Variant::U | Variant::UEqAx | Variant::UEqRew => {
                Err("U is an upper bound (existentials as fresh constants, unsound), not an OWL 2 RL job".into())
            }
            Variant::Other(name) => Err(format!("{name} is another rule set")),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Published {
    pub(crate) papers: Vec<Paper>,
    pub(crate) results: Vec<PublishedResult>,
}

impl Published {
    pub(crate) fn paper(&self, id: &str) -> Option<&Paper> {
        self.papers.iter().find(|p| p.id == id)
    }

    /// The rows measured on a scenario's data and variant.
    pub(crate) fn on(&self, scenario: &str) -> impl Iterator<Item = &PublishedResult> {
        self.results
            .iter()
            .filter(move |r| r.scenario.as_deref() == Some(scenario))
    }
}

/// The data file, read once; a test checks it reads.
pub(crate) fn all() -> &'static Published {
    static ALL: OnceLock<Published> = OnceLock::new();
    ALL.get_or_init(|| parse(DATA).expect("published.json reads (a test checks it)"))
}

fn parse(text: &str) -> Result<Published, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("published.json: {e}"))?;
    let string = |v: &Value, key: &str| v[key].as_str().map(str::to_string);
    let need = |v: &Value, key: &str| string(v, key).ok_or_else(|| format!("{key} missing in {v}"));
    let papers = v["papers"]
        .as_array()
        .ok_or("no papers")?
        .iter()
        .map(|p| {
            Ok(Paper {
                id: need(p, "id")?,
                cite: need(p, "cite")?,
                url: need(p, "url")?,
                place: need(p, "where")?,
                machine: need(p, "machine")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let count = |r: &Value, key: &str| string(r, key).map(|c| Count::parse(&c)).transpose();
    let mut results = Vec::new();
    for r in v["results"].as_array().ok_or("no results")? {
        let engines = r["engines"]
            .as_array()
            .ok_or_else(|| format!("no engines in {r}"))?
            .iter()
            .map(|e| {
                let seconds = |key: &str| e[key].as_f64();
                let runs_here_as = string(e, "runs_here_as")
                    .map(|n| EngineId::from_name(&n).ok_or(format!("runs_here_as: no engine {n}")))
                    .transpose()?;
                Ok(EngineResult {
                    engine: need(e, "engine")?,
                    build: string(e, "build"),
                    threads: e["threads"].as_u64().map(|t| t as u32),
                    seconds: Seconds {
                        load: seconds("load_s"),
                        reasoning: seconds("reasoning_s"),
                        total: seconds("total_s"),
                        unstated: seconds("unstated_s"),
                    },
                    memory: string(e, "memory"),
                    outcome: string(e, "outcome"),
                    runs_here_as,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let explained = match &r["explained"] {
            Value::Null => None,
            x => Some(Explained {
                difference: x["difference"].as_i64().ok_or("explained.difference")?,
                why: need(x, "why")?,
            }),
        };
        results.push(PublishedResult {
            paper: need(r, "paper")?,
            variant: Variant::parse(&need(r, "variant")?),
            equality: r["equality"].as_bool(),
            program: string(r, "program"),
            scenario: string(r, "scenario"),
            before: count(r, "before")?,
            after: count(r, "after")?,
            explained,
            engines,
        });
    }
    Ok(Published { papers, results })
}

/// This reasoner's counts on a published job's scenario, comparable with a
/// paper's: before, the data's distinct triples; after, the closure less the
/// triples of the ontology's own closure that are not data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Counts {
    pub(crate) before: u64,
    pub(crate) after: u64,
    /// The closure over the data and the ontology.
    pub(crate) closure: u64,
    /// The triples of the ontology's closure that are not data.
    pub(crate) ontology_only: u64,
}

impl Counts {
    pub(crate) fn to_json(self) -> Value {
        serde_json::json!({
            "before": self.before,
            "after": self.after,
            "closure": self.closure,
            "ontology_only": self.ontology_only,
        })
    }

    pub(crate) fn from_json(v: &Value) -> Option<Self> {
        Some(Counts {
            before: v["before"].as_u64()?,
            after: v["after"].as_u64()?,
            closure: v["closure"].as_u64()?,
            ontology_only: v["ontology_only"].as_u64()?,
        })
    }
}

/// Counts a published job's scenario: its first file is the program (the
/// ontology), the rest the data; `closure` is this reasoner's closure over
/// all of them, in the configuration `switches`. Not timed: it parses the
/// data again and reasons over the ontology alone.
pub(crate) fn count(files: &[PathBuf], switches: Switches, closure: u64) -> Result<Counts, String> {
    let (ontology, data) = match files {
        [ontology, data @ ..] if !data.is_empty() => (ontology, data),
        _ => return Err("the scenario does not load its program apart from its data".into()),
    };
    let hasher = BuildHasherDefault::<DefaultHasher>::default();
    // A data triple with a blank node is hashed with its file, as each file's
    // blank nodes are its own; no triple of the ontology's closure can equal
    // it, since that closure's blank nodes are the ontology's.
    let mut hashes = Vec::new();
    for (n, file) in data.iter().enumerate() {
        crate::convert::parse(file, &mut |t| {
            let blank = t.subject.is_blank_node() || t.object.is_blank_node();
            hashes.push(if blank {
                hasher.hash_one((n, &t))
            } else {
                hasher.hash_one(&t)
            });
            Ok(())
        })?;
    }
    hashes.sort_unstable();
    hashes.dedup();
    let mut r = Reasoner::with_switches(switches);
    r.load_file(ontology).map_err(|e| e.to_string())?;
    r.reason();
    let ontology_only = r
        .triples()
        .filter(|t| {
            t.subject.is_blank_node() || t.object.is_blank_node() || hashes.binary_search(&hasher.hash_one(t)).is_err()
        })
        .count() as u64;
    Ok(Counts {
        before: hashes.len() as u64,
        after: closure.saturating_sub(ontology_only),
        closure,
        ontology_only,
    })
}

/// One engine's run here, as the estimate reads it from a results file.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Ran {
    /// `ok`, or why there is no number (`timeout`, `over-memory`, ...).
    pub(crate) status: String,
    pub(crate) reasoning_ms: Option<f64>,
    pub(crate) total_ms: Option<f64>,
    pub(crate) closure: Option<u64>,
}

/// What a results file says about one scenario, for its estimates.
#[derive(Debug, Clone)]
pub(crate) struct Here {
    /// Whether the configuration has `equality` on.
    pub(crate) equality: bool,
    /// This reasoner's comparable counts, when recorded.
    pub(crate) counts: Option<Counts>,
    pub(crate) turbowl: Option<Ran>,
    /// The bridge engine's run, when it ran.
    pub(crate) bridge: Option<Ran>,
}

/// RDFox's time scaled to this machine through a bridge.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Estimate {
    pub(crate) axis: TimeAxis,
    pub(crate) rdfox_s: f64,
    pub(crate) bridge_s: f64,
    pub(crate) bridge_here_ms: f64,
    /// The estimate: `bridge_here_ms * rdfox_s / bridge_s`.
    pub(crate) rdfox_here_ms: f64,
    pub(crate) turbowl_here_ms: Option<f64>,
}

/// RDFox's time on `result` scaled through `bridge`, or why not.
pub(crate) fn estimate(
    result: &PublishedResult,
    rdfox: &EngineResult,
    bridge: &EngineResult,
    here: &Here,
) -> Result<Estimate, String> {
    let bridge_name = bridge.runs_here_as.map_or(bridge.engine.as_str(), |e| e.name());
    // Threads: the bridge runs one thread here.
    if rdfox.threads != Some(1) || bridge.threads != Some(1) {
        let t = |e: &EngineResult| {
            e.threads.map_or("threads not stated".into(), |n| {
                format!("{n} thread{}", if n == 1 { "" } else { "s" })
            })
        };
        return Err(format!(
            "threads: RDFox ran with {} and {} with {} in the paper; the bridge runs one thread here",
            t(rdfox),
            bridge.engine,
            t(bridge)
        ));
    }
    for e in [rdfox, bridge] {
        if let Some(outcome) = &e.outcome {
            return Err(format!("{} in the paper: {outcome}", e.engine));
        }
    }
    // The variant, and the program's equality.
    let needs = result.equality()?;
    if needs != here.equality {
        let on = |b: bool| if b { "on" } else { "off" };
        return Err(format!(
            "the paper's {} program {} owl:sameAs: it needs equality {}, this configuration has it {}",
            result.variant.name(),
            if needs { "axiomatises" } else { "has no rule for" },
            on(needs),
            on(here.equality)
        ));
    }
    // The counts.
    let counts = here.counts.ok_or(
        "this reasoner's comparable counts are not recorded (it did not finish, the scenario does not load its program apart from its data, or the file is older than schema 7)",
    )?;
    let (Some(before), Some(after)) = (&result.before, &result.after) else {
        return Err("the paper gives no count before or after".into());
    };
    if !before.matches(counts.before as i64) {
        return Err(format!(
            "counts before differ: {} data triples here, {} in the paper",
            counts.before, before.printed
        ));
    }
    let explained = result.explained.as_ref().map_or(0, |e| e.difference);
    if !after.matches(counts.after as i64 - explained) {
        let why = result
            .explained
            .as_ref()
            .map_or(String::new(), |e| format!(" ({explained:+} explained: {})", e.why));
        return Err(format!(
            "counts after differ: {} here (closure {} less {} of the ontology alone){why}, {} in the paper",
            counts.after, counts.closure, counts.ontology_only, after.printed
        ));
    }
    // The bridge ran here, with this reasoner's closure.
    let ran = here.bridge.as_ref().ok_or(format!("{bridge_name} did not run here"))?;
    if ran.status != "ok" {
        return Err(format!("{bridge_name} did not finish here: {}", ran.status));
    }
    let ours = here.turbowl.as_ref().and_then(|t| t.closure);
    if ran.closure.is_none() || ran.closure != ours {
        return Err(format!(
            "{bridge_name}'s closure here ({:?}) is not this reasoner's ({ours:?})",
            ran.closure
        ));
    }
    // The same time in the paper and here: reasoning, else load plus reasoning.
    let here_ms = |axis| match axis {
        TimeAxis::Reasoning => ran.reasoning_ms,
        TimeAxis::Total => ran.total_ms,
    };
    let axis = [TimeAxis::Reasoning, TimeAxis::Total]
        .into_iter()
        .find(|&a| rdfox.seconds.get(a).is_some() && bridge.seconds.get(a).is_some() && here_ms(a).is_some())
        .ok_or_else(|| {
            format!(
                "no time measures the same work: the paper gives RDFox {} and {} {}; {bridge_name} here reports {}",
                rdfox.seconds.describe(),
                bridge.engine,
                bridge.seconds.describe(),
                match (ran.reasoning_ms, ran.total_ms) {
                    (Some(_), Some(_)) => "reasoning and load plus reasoning",
                    (None, Some(_)) => "load plus reasoning only",
                    (Some(_), None) => "reasoning only",
                    (None, None) => "no time",
                }
            )
        })?;
    let (rdfox_s, bridge_s, bridge_here_ms) = (
        rdfox.seconds.get(axis).expect("checked"),
        bridge.seconds.get(axis).expect("checked"),
        here_ms(axis).expect("checked"),
    );
    let turbowl_here_ms = here.turbowl.as_ref().and_then(|t| match axis {
        TimeAxis::Reasoning => t.reasoning_ms,
        TimeAxis::Total => t.total_ms,
    });
    Ok(Estimate {
        axis,
        rdfox_s,
        bridge_s,
        bridge_here_ms,
        rdfox_here_ms: bridge_here_ms * rdfox_s / bridge_s,
        turbowl_here_ms,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn the_data_file_reads() {
        let p = parse(DATA).unwrap();
        assert!(p.results.len() > 50, "{}", p.results.len());
        for r in &p.results {
            assert!(p.paper(&r.paper).is_some(), "{} names no paper", r.paper);
            if let Some(name) = &r.scenario {
                // A published job's scenario loads its program first, then its data.
                let s = crate::scenario::find(name).unwrap_or_else(|| panic!("no scenario {name}"));
                assert!(s.published().is_some(), "{name}");
            }
        }
        let claros = p
            .on("claros-l")
            .find(|r| r.paper == "vlog2016")
            .expect("VLog 2016's Claros L");
        assert_eq!(claros.variant, Variant::L);
        assert_eq!(claros.equality(), Ok(true));
        assert_eq!(claros.after.as_ref().unwrap().printed, "95.5 M");
        assert_eq!(claros.engines[1].runs_here_as, Some(EngineId::Vlog));
        assert_eq!(claros.engines[0].seconds.reasoning, Some(2418.0));
    }

    #[test]
    fn counts_match_to_the_printed_precision() {
        let c = Count::parse("95.5 M").unwrap();
        assert!(c.matches(95_450_000) && c.matches(95_549_999));
        assert!(!c.matches(95_449_999) && !c.matches(95_550_001));
        assert!(Count::parse("1,529.7 M").unwrap().matches(1_529_700_000));
        assert!(Count::parse("1.5 G").unwrap().matches(1_460_000_000));
        let exact = Count::parse("66,751,196").unwrap();
        assert!(exact.matches(66_751_196) && !exact.matches(66_751_197));
        assert!(Count::parse("12 X").is_err());
    }

    /// A row like VLog 2016's Claros L, with a bridge that reports
    /// reasoning time, and a run here where every condition holds.
    pub(crate) fn row() -> PublishedResult {
        let engine = |name: &str, s: f64, here: Option<EngineId>| EngineResult {
            engine: name.into(),
            build: None,
            threads: Some(1),
            seconds: Seconds {
                reasoning: Some(s),
                ..Seconds::default()
            },
            memory: None,
            outcome: None,
            runs_here_as: here,
        };
        PublishedResult {
            paper: "vlog2016".into(),
            variant: Variant::L,
            equality: Some(false),
            program: None,
            scenario: Some("claros-l".into()),
            before: Some(Count::parse("18.8 M").unwrap()),
            after: Some(Count::parse("95.5 M").unwrap()),
            explained: None,
            engines: vec![engine(RDFOX, 2418.0, None), engine("VLog", 644.0, Some(EngineId::Vlog))],
        }
    }

    pub(crate) fn ran(reasoning: Option<f64>, total: f64, closure: u64) -> Ran {
        Ran {
            status: "ok".into(),
            reasoning_ms: reasoning,
            total_ms: Some(total),
            closure: Some(closure),
        }
    }

    pub(crate) fn here() -> Here {
        Here {
            equality: false,
            counts: Some(Counts {
                before: 18_793_277,
                after: 95_512_000,
                closure: 95_600_000,
                ontology_only: 88_000,
            }),
            turbowl: Some(ran(Some(15_000.0), 30_000.0, 95_600_000)),
            bridge: Some(ran(Some(200_000.0), 260_000.0, 95_600_000)),
        }
    }

    #[test]
    fn estimates_when_all_three_hold() {
        let r = row();
        let e = estimate(&r, &r.engines[0], &r.engines[1], &here()).unwrap();
        assert_eq!(e.axis, TimeAxis::Reasoning);
        // 200 s here x 2418 / 644.
        assert!((e.rdfox_here_ms - 200_000.0 * 2418.0 / 644.0).abs() < 1e-6, "{e:?}");
        assert_eq!(e.turbowl_here_ms, Some(15_000.0));
    }

    #[test]
    fn refuses_when_the_counts_differ() {
        let r = row();
        let mut h = here();
        h.counts.as_mut().unwrap().after = 91_006_354;
        let why = estimate(&r, &r.engines[0], &r.engines[1], &h).unwrap_err();
        assert!(why.starts_with("counts after differ: 91006354 here"), "{why}");
        h.counts = Some(Counts {
            before: 19_000_000,
            ..here().counts.unwrap()
        });
        let why = estimate(&r, &r.engines[0], &r.engines[1], &h).unwrap_err();
        assert!(why.starts_with("counts before differ"), "{why}");
        h.counts = None;
        assert!(estimate(&r, &r.engines[0], &r.engines[1], &h).is_err());
    }

    #[test]
    fn an_explained_difference_counts() {
        let mut r = row();
        let mut h = here();
        h.counts.as_mut().unwrap().after = 95_512_000 + 1_000_000;
        assert!(estimate(&r, &r.engines[0], &r.engines[1], &h).is_err());
        r.explained = Some(Explained {
            difference: 1_000_000,
            why: "a test".into(),
        });
        assert!(estimate(&r, &r.engines[0], &r.engines[1], &h).is_ok());
    }

    #[test]
    fn refuses_when_the_variant_does_not_match() {
        let mut r = row();
        let mut h = here();
        h.equality = true;
        let why = estimate(&r, &r.engines[0], &r.engines[1], &h).unwrap_err();
        assert_eq!(
            why,
            "the paper's L program has no rule for owl:sameAs: it needs equality off, this configuration has it on"
        );
        r.equality = None;
        let why = estimate(&r, &r.engines[0], &r.engines[1], &h).unwrap_err();
        assert!(why.ends_with("is not known"), "{why}");
        r.equality = Some(true);
        assert!(estimate(&r, &r.engines[0], &r.engines[1], &h).is_ok());
        for v in [Variant::U, Variant::LEqRew, Variant::Other("RDFS-Plus".into())] {
            r.variant = v;
            assert!(estimate(&r, &r.engines[0], &r.engines[1], &h).is_err());
        }
        r.variant = Variant::U;
        let why = estimate(&r, &r.engines[0], &r.engines[1], &h).unwrap_err();
        assert!(why.starts_with("U is an upper bound"), "{why}");
    }

    #[test]
    fn refuses_when_the_bridge_did_not_run() {
        let r = row();
        let mut h = here();
        h.bridge = None;
        let why = estimate(&r, &r.engines[0], &r.engines[1], &h).unwrap_err();
        assert_eq!(why, "vlog did not run here");
        h.bridge = Some(Ran {
            status: "timeout".into(),
            reasoning_ms: None,
            total_ms: None,
            closure: None,
        });
        let why = estimate(&r, &r.engines[0], &r.engines[1], &h).unwrap_err();
        assert_eq!(why, "vlog did not finish here: timeout");
        h.bridge = Some(ran(Some(1.0), 2.0, 1));
        let why = estimate(&r, &r.engines[0], &r.engines[1], &h).unwrap_err();
        assert!(why.contains("is not this reasoner's"), "{why}");
    }

    #[test]
    fn refuses_times_that_measure_different_work() {
        let r = row();
        let mut h = here();
        // VLog here reports load plus reasoning only; the paper, reasoning only.
        h.bridge.as_mut().unwrap().reasoning_ms = None;
        let why = estimate(&r, &r.engines[0], &r.engines[1], &h).unwrap_err();
        assert!(why.starts_with("no time measures the same work"), "{why}");
        assert!(why.ends_with("vlog here reports load plus reasoning only"), "{why}");
    }

    #[test]
    fn refuses_threads_not_stated_or_many() {
        let mut r = row();
        r.engines[0].threads = None;
        let why = estimate(&r, &r.engines[0], &r.engines[1], &here()).unwrap_err();
        assert!(why.starts_with("threads: RDFox ran with threads not stated"), "{why}");
        r.engines[0].threads = Some(16);
        assert!(estimate(&r, &r.engines[0], &r.engines[1], &here()).is_err());
    }
}
