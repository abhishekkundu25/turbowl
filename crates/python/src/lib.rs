//! The native half of the `turbowl` Python package: the module
//! `turbowl._native`, a thin wrapper over [`turbowl::Reasoner`].
//!
//! The Python half (`python/turbowl/__init__.py`) is the public API. It
//! takes graphs as well as iterables of triples, maps format names, and
//! wraps the plain tuples this module returns in `Summary` and
//! `Diagnostic`. This module converts terms, because it is the cheaper side
//! to do it on, and raises the exceptions of `turbowl._errors`.
//!
//! Terms, rdflib to `oxrdf`:
//!
//! | rdflib | `oxrdf` | Rejected with `TermError` |
//! |---|---|---|
//! | `URIRef` | `NamedNode` | an IRI that is not absolute or not valid |
//! | `BNode` | `BlankNode`, its label kept | a label that is not a valid N-Triples label |
//! | `Literal` | `Literal`: language-tagged, typed or simple | an invalid language tag; a literal subject; a predicate that is not a `URIRef` |
//!
//! Any other object is a `TypeError`. A batch is converted whole before the
//! reasoner sees it, so a bad term leaves the reasoner unchanged.
//!
//! Back to rdflib, a literal is built with `normalize=False`, so its lexical
//! form is the one the reasoner holds (rdflib would turn `"01"^^xsd:integer`
//! into `"1"`). A literal of `xsd:string` comes back with no datatype, as
//! rdflib's parsers give `"x"`. Language tags are lowercase (the parsers and
//! `oxrdf` lowercase them; rdflib compares them ignoring case). Each
//! distinct term of one result is built once and shared.
//!
//! Switches are given and read by their names ([`Switch::name`]), parsed
//! here, once, with [`Switch::from_name`].
//!
//! Snapshots are saved and loaded with files only
//! ([`turbowl::Reasoner::save_snapshot`], [`turbowl::Reasoner::from_snapshot`]);
//! a refused one raises `SnapshotError`.
//!
//! Explanations ([`turbowl::Explainer`]) take their terms converted as
//! above, once, and return a proof as plain tuples, its terms built as the
//! reads build them; the Python half wraps them in `Proof` and `Step`. The
//! explainer's indexes ([`turbowl::ExplainCache`]) are kept between calls
//! and dropped by every method that changes the reasoner. A change since
//! the last run raises `NotReasonedError`.
//!
//! Parsing, reasoning, writing, snapshots and explaining release the GIL.
//!
//! Reasoning and explaining can be cancelled ([`turbowl::Cancel`], which
//! `NativeCancel` wraps): they run on a worker thread while the calling
//! thread looks at Python's signals every 20 ms ([`interruptible`]), so
//! Ctrl+C stops them too and raises `KeyboardInterrupt`. A cancelled call
//! raises `CancelledError`.

use oxrdf::{BlankNode, Literal, NamedNode, NamedOrBlankNode, Term, Triple};
use pyo3::call::PyCallArgs;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyBool, PyDict, PyList, PyMapping, PySequence, PyString, PyTuple, PyType};
use rustc_hash::FxHashMap;
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use turbowl::{Cancel, Diagnostic, Error, ExplainCache, Format, Goal, Proof, Reason, Severity, Switch, Switches};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// The native module.
#[pymodule]
#[pyo3(name = "_native")]
fn native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<NativeReasoner>()?;
    m.add_class::<NativeCancel>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

/// The cancel handle behind `turbowl.Cancel`.
#[pyclass(module = "turbowl._native", frozen)]
#[derive(Debug, Default)]
struct NativeCancel(Cancel);

#[pymethods]
impl NativeCancel {
    /// A handle nobody has tripped.
    #[new]
    fn new() -> Self {
        Self::default()
    }

    /// Trips the handle: the calls it was given stop at their next check.
    fn cancel(&self) {
        self.0.cancel();
    }

    /// True once tripped.
    fn cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}

/// The handle a call runs with: the one given, or one of its own for
/// Ctrl+C.
fn handle(cancel: Option<&Bound<'_, NativeCancel>>) -> Cancel {
    cancel.map(|c| c.get().0.clone()).unwrap_or_default()
}

/// How long the calling thread waits between two looks at Python's signals.
const SIGNALS_EVERY: Duration = Duration::from_millis(20);

/// The worker thread's stack: a main thread's on macOS and Linux.
const STACK: usize = 8 << 20;

/// Runs `work` on a worker thread without the GIL, while this thread looks
/// at Python's signals every [`SIGNALS_EVERY`]. A signal whose handler
/// raises (Ctrl+C: `KeyboardInterrupt`) trips `cancel`, and its exception
/// comes back beside what `work` returned once it stopped. A panic in
/// `work` goes on in this thread.
fn interruptible<T: Send>(
    py: Python<'_>,
    cancel: &Cancel,
    work: impl FnOnce() -> T + Send,
) -> PyResult<(T, Option<PyErr>)> {
    std::thread::scope(|s| {
        let (done, mut finished) = mpsc::channel::<()>();
        let worker = std::thread::Builder::new()
            .name("turbowl".to_owned())
            .stack_size(STACK)
            .spawn_scoped(s, move || {
                let out = work();
                let _ = done.send(());
                out
            })
            .map_err(|e| raise(py, "TurbowlError", (format!("cannot start a thread: {e}"),)))?;
        let mut signal = None;
        loop {
            // The receiver moves in and out: it is not `Sync`, so the
            // closure cannot borrow it.
            let (back, waited) = py.detach(move || {
                let waited = finished.recv_timeout(SIGNALS_EVERY);
                (finished, waited)
            });
            finished = back;
            if !matches!(waited, Err(mpsc::RecvTimeoutError::Timeout)) {
                break;
            }
            if signal.is_none()
                && let Err(e) = py.check_signals()
            {
                cancel.cancel();
                signal = Some(e);
            }
        }
        match worker.join() {
            Ok(out) => Ok((out, signal)),
            Err(panic) => std::panic::resume_unwind(panic),
        }
    })
}

/// The reasoner behind `turbowl.Reasoner`. Its methods take and return
/// rdflib terms; see the module doc of the crate for the conversions.
#[pyclass(module = "turbowl._native")]
#[derive(Debug)]
struct NativeReasoner {
    inner: turbowl::Reasoner,
    /// The explainer's indexes, kept while the reasoner does not change:
    /// every `&mut self` method drops them.
    cache: Option<ExplainCache>,
}

#[pymethods]
impl NativeReasoner {
    /// An empty reasoner. Its switches start from the defaults, or from
    /// every switch on when `complete`; `switches` then maps names to
    /// `True` (on) or `False` (off).
    #[new]
    #[pyo3(signature = (switches=None, complete=false))]
    fn new(switches: Option<&Bound<'_, PyAny>>, complete: bool) -> PyResult<Self> {
        let start = if complete {
            Switches::complete()
        } else {
            Switches::default()
        };
        let switches = match switches {
            Some(given) => switches_from(start, given)?,
            None => start,
        };
        Ok(Self {
            inner: turbowl::Reasoner::with_switches(switches),
            cache: None,
        })
    }

    /// The reasoner a snapshot file holds.
    #[staticmethod]
    fn from_snapshot(py: Python<'_>, path: PathBuf) -> PyResult<Self> {
        py.detach(|| turbowl::Reasoner::from_snapshot(&path))
            .map(|inner| Self { inner, cache: None })
            .map_err(|e| error(py, e))
    }

    /// Saves a snapshot of the reasoner to a file, atomically.
    fn save_snapshot(&self, py: Python<'_>, path: PathBuf) -> PyResult<()> {
        let inner = &self.inner;
        py.detach(|| inner.save_snapshot(&path)).map_err(|e| error(py, e))
    }

    /// Every switch's name, mapped to whether it is on, in the order of
    /// [`Switch::ALL`].
    fn switches<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let on = self.inner.switches();
        let out = PyDict::new(py);
        for switch in Switch::ALL {
            out.set_item(switch.name(), on.is_on(switch))?;
        }
        Ok(out)
    }

    /// Adds a file's triples to the base; the format comes from its extension.
    fn load_file(&mut self, py: Python<'_>, path: PathBuf) -> PyResult<()> {
        self.cache = None;
        let inner = &mut self.inner;
        py.detach(|| inner.load_file(&path)).map_err(|e| error(py, e))
    }

    /// Adds a document's triples to the base. `format` is a canonical name
    /// (`turtle`, `ntriples`, `n3`, `rdfxml`).
    #[pyo3(signature = (text, format, base_iri=None))]
    fn load_str(&mut self, py: Python<'_>, text: &str, format: &str, base_iri: Option<&str>) -> PyResult<()> {
        self.cache = None;
        let format = format_named(py, format)?;
        let inner = &mut self.inner;
        py.detach(|| inner.load_str(text, format, base_iri))
            .map_err(|e| error(py, e))
    }

    /// Adds an iterable of `(subject, predicate, object)` rdflib triples.
    fn insert(&mut self, py: Python<'_>, triples: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cache = None;
        let triples = from_python(triples)?;
        let inner = &mut self.inner;
        py.detach(|| inner.insert(triples.iter().map(Triple::as_ref)));
        Ok(())
    }

    /// Removes an iterable of rdflib triples from the base.
    fn remove(&mut self, py: Python<'_>, triples: &Bound<'_, PyAny>) -> PyResult<()> {
        self.cache = None;
        let triples = from_python(triples)?;
        let inner = &mut self.inner;
        py.detach(|| inner.remove(triples.iter().map(Triple::as_ref)));
        Ok(())
    }

    /// Makes the base exactly these rdflib triples; true when a base triple
    /// was removed.
    fn set_base(&mut self, py: Python<'_>, triples: &Bound<'_, PyAny>) -> PyResult<bool> {
        self.cache = None;
        let triples = from_python(triples)?;
        let inner = &mut self.inner;
        Ok(py.detach(|| inner.set_base(triples.iter().map(Triple::as_ref))))
    }

    /// Empties the reasoner.
    fn clear(&mut self) {
        self.cache = None;
        self.inner.clear();
    }

    /// Runs to the fixpoint: `(base, inferred, rounds, diagnostics)`. A
    /// cancelled run keeps no partial result.
    #[pyo3(signature = (cancel=None))]
    fn reason(
        &mut self,
        py: Python<'_>,
        cancel: Option<&Bound<'_, NativeCancel>>,
    ) -> PyResult<(usize, usize, usize, usize)> {
        self.cache = None;
        let cancel = handle(cancel);
        let inner = &mut self.inner;
        let (done, signal) = interruptible(py, &cancel, || inner.reason_cancellable(&cancel))?;
        if let Some(e) = signal {
            return Err(e);
        }
        let s = done.map_err(|e| error(py, e))?;
        Ok((s.base, s.inferred, s.rounds, s.diagnostics))
    }

    /// The closure as a list of rdflib triples.
    fn triples<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        ToPython::new(py)?.triples(self.inner.triples())
    }

    /// The inferred triples as a list of rdflib triples.
    fn inferred<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        ToPython::new(py)?.triples(self.inner.inferred())
    }

    /// The diagnostics as `(code, severity, message, terms)` tuples:
    /// severity is `"error"` or `"warning"`, terms a tuple of rdflib terms.
    fn diagnostics<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let mut to = ToPython::new(py)?;
        let out = self
            .inner
            .diagnostics()
            .iter()
            .map(|d| diagnostic(&mut to, d))
            .collect::<PyResult<Vec<_>>>()?;
        PyList::new(py, out)
    }

    /// The proof of a triple of the last run's closure, as [`proof`] gives
    /// it, or `None` when the triple is not in the closure.
    #[pyo3(signature = (s, p, o, cancel=None))]
    fn explain<'py>(
        &mut self,
        py: Python<'py>,
        s: &Bound<'py, PyAny>,
        p: &Bound<'py, PyAny>,
        o: &Bound<'py, PyAny>,
        cancel: Option<&Bound<'py, NativeCancel>>,
    ) -> PyResult<Option<Bound<'py, PyTuple>>> {
        let rdflib = Rdflib::new(py)?;
        let triple = Triple::new(subject(&rdflib, s)?, predicate(&rdflib, p)?, object(&rdflib, o)?);
        let found = self.explaining(py, &handle(cancel), |_, explainer, cancel| {
            explainer.explain_cancellable(triple.as_ref(), cancel)
        })?;
        found.map(|proof| self::proof(py, &proof)).transpose()
    }

    /// The proof of a diagnostic of the last run, given by its code and its
    /// terms, as [`proof`] gives it, or `None` when it is not reported.
    #[pyo3(signature = (code, terms, cancel=None))]
    fn explain_diagnostic<'py>(
        &mut self,
        py: Python<'py>,
        code: &str,
        terms: &Bound<'py, PyAny>,
        cancel: Option<&Bound<'py, NativeCancel>>,
    ) -> PyResult<Option<Bound<'py, PyTuple>>> {
        let rdflib = Rdflib::new(py)?;
        let terms = terms
            .try_iter()?
            .map(|t| object(&rdflib, &t?))
            .collect::<PyResult<Vec<Term>>>()?;
        let found = self.explaining(py, &handle(cancel), |reasoner, explainer, cancel| {
            match reasoner
                .diagnostics()
                .iter()
                .find(|d| d.code == code && d.terms == terms)
            {
                Some(d) => explainer.explain_diagnostic_cancellable(d, cancel),
                None => Ok(None),
            }
        })?;
        found.map(|proof| self::proof(py, &proof)).transpose()
    }

    /// True while the explainer's indexes are kept (for the tests).
    fn explain_cache_held(&self) -> bool {
        self.cache.is_some()
    }

    /// The closure (or only the inferred triples) as text in a format with a
    /// canonical name.
    fn serialize(&self, py: Python<'_>, format: &str, inferred_only: bool) -> PyResult<String> {
        let format = format_named(py, format)?;
        let inner = &self.inner;
        let mut out = Vec::new();
        py.detach(|| inner.write(&mut out, format, inferred_only))
            .map_err(|e| error(py, e))?;
        String::from_utf8(out).map_err(|e| raise(py, "TurbowlError", (e.to_string(),)))
    }

    /// Writes the closure (or only the inferred triples) to a file, in a
    /// format with a canonical name or, with none, the one its extension names.
    #[pyo3(signature = (path, format, inferred_only))]
    fn write(&self, py: Python<'_>, path: PathBuf, format: Option<&str>, inferred_only: bool) -> PyResult<()> {
        let format = match format {
            Some(name) => format_named(py, name)?,
            None => path
                .extension()
                .and_then(|e| e.to_str())
                .and_then(Format::from_extension)
                .ok_or_else(|| error(py, Error::UnknownFormat { path: path.clone() }))?,
        };
        let inner = &self.inner;
        py.detach(|| {
            let io = |source| Error::Io {
                path: Some(path.clone()),
                source,
            };
            let file = std::fs::File::create(&path).map_err(io)?;
            let mut out = std::io::BufWriter::new(file);
            inner.write(&mut out, format, inferred_only).map_err(|e| match e {
                Error::Io { path: None, source } => io(source),
                e => e,
            })?;
            out.flush().map_err(io)
        })
        .map_err(|e| error(py, e))
    }
}

impl NativeReasoner {
    /// Runs `ask` on the reasoner and an explainer over its last run's
    /// closure, without the GIL ([`interruptible`]), with the kept indexes,
    /// and keeps them for the next call, a cancelled one's too.
    fn explaining<T: Send>(
        &mut self,
        py: Python<'_>,
        cancel: &Cancel,
        ask: impl FnOnce(&turbowl::Reasoner, &turbowl::Explainer<'_>, &Cancel) -> Result<Option<T>, Error> + Send,
    ) -> PyResult<Option<T>> {
        let cache = self.cache.take().unwrap_or_default();
        let inner = &self.inner;
        let (done, signal) = interruptible(py, cancel, || {
            let explainer = inner.explainer_with(cache)?;
            let found = ask(inner, &explainer, cancel);
            Ok::<_, Error>((found, explainer.into_cache()))
        })?;
        let (found, cache) = done.map_err(|e| error(py, e))?;
        self.cache = Some(cache);
        if let Some(e) = signal {
            return Err(e);
        }
        found.map_err(|e| error(py, e))
    }
}

/// A proof as the Python half's `Proof` reads it:
/// `(steps, goal, rule, premises, depth, text)`. Each step is
/// `((s, p, o), rule, premises)`, `rule` `None` for a base triple and
/// `premises` a tuple of indexes of earlier steps. `goal` is `None` when
/// the proof is of the last step's statement, or a diagnostic as
/// `diagnostics` gives one; `rule` and `premises` are the goal's. `text` is
/// the tree.
fn proof<'py>(py: Python<'py>, proof: &Proof) -> PyResult<Bound<'py, PyTuple>> {
    let mut to = ToPython::new(py)?;
    let reason = |reason: &Reason| -> PyResult<(Option<&'static str>, Bound<'py, PyTuple>)> {
        Ok(match reason {
            Reason::Base => (None, PyTuple::empty(py)),
            Reason::Rule { rule, premises } => (Some(*rule), PyTuple::new(py, premises)?),
        })
    };
    let mut steps = Vec::with_capacity(proof.steps().len());
    for step in proof.steps() {
        let s = &step.statement;
        let statement = PyTuple::new(
            py,
            [
                to.term(s.subject.clone())?,
                to.term(s.predicate.clone())?,
                to.term(s.object.clone())?,
            ],
        )?;
        let (rule, premises) = reason(&step.reason)?;
        steps.push((statement, rule, premises).into_pyobject(py)?);
    }
    let goal = match proof.goal() {
        Goal::Statement(_) => py.None().into_bound(py),
        Goal::Diagnostic(d) => diagnostic(&mut to, d)?.into_any(),
    };
    let (rule, premises) = reason(proof.reason())?;
    (
        PyList::new(py, steps)?,
        goal,
        rule,
        premises,
        proof.depth(),
        proof.to_string(),
    )
        .into_pyobject(py)
}

/// A diagnostic as `(code, severity, message, terms)`: severity is
/// `"error"` or `"warning"`, terms a tuple of rdflib terms.
fn diagnostic<'py>(to: &mut ToPython<'py>, d: &Diagnostic) -> PyResult<Bound<'py, PyTuple>> {
    let severity = match d.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    let terms = d
        .terms
        .iter()
        .map(|t| to.term(t.clone()))
        .collect::<PyResult<Vec<_>>>()?;
    (d.code, severity, d.message.as_str(), PyTuple::new(to.py, terms)?).into_pyobject(to.py)
}

/// `start`, with each switch a mapping names turned on (`True`) or off
/// (`False`).
fn switches_from(start: Switches, given: &Bound<'_, PyAny>) -> PyResult<Switches> {
    let given = given.cast::<PyMapping>().map_err(|_| {
        PyTypeError::new_err(format!(
            "switches must map switch names to True or False, got {}",
            shown(given)
        ))
    })?;
    let mut switches = start;
    for item in given.items()?.iter() {
        let (name, value): (Bound<'_, PyAny>, Bound<'_, PyAny>) = item.extract()?;
        let name = name
            .cast::<PyString>()
            .map_err(|_| PyTypeError::new_err(format!("a switch name is a str, got {}", shown(&name))))?
            .to_cow()?;
        let switch = Switch::from_name(&name).ok_or_else(|| {
            let names: Vec<&str> = Switch::ALL.into_iter().map(Switch::name).collect();
            PyValueError::new_err(format!("unknown switch '{name}' (the switches: {})", names.join(", ")))
        })?;
        let on = value
            .cast::<PyBool>()
            .map_err(|_| PyTypeError::new_err(format!("switch '{name}' takes True or False, got {}", shown(&value))))?;
        switches = if on.is_true() {
            switches.on(switch)
        } else {
            switches.off(switch)
        };
    }
    Ok(switches)
}

/// An object's `repr()`, for a message.
fn shown(object: &Bound<'_, PyAny>) -> String {
    object.repr().map(|r| r.to_string()).unwrap_or_default()
}

/// A format by its canonical name (the Python half maps the other names).
fn format_named(py: Python<'_>, name: &str) -> PyResult<Format> {
    match name {
        "turtle" => Ok(Format::Turtle),
        "ntriples" => Ok(Format::NTriples),
        "n3" => Ok(Format::N3),
        "rdfxml" => Ok(Format::RdfXml),
        _ => Err(raise(
            py,
            "UnknownFormatError",
            (format!("unknown format {name:?}"), None::<String>),
        )),
    }
}

/// An exception of `turbowl._errors`, built with `args`.
fn raise<'py>(py: Python<'py>, class: &str, args: impl PyCallArgs<'py>) -> PyErr {
    match py
        .import(intern!(py, "turbowl._errors"))
        .and_then(|m| m.getattr(class))
        .and_then(|c| c.call1(args))
    {
        Ok(e) => PyErr::from_value(e),
        Err(e) => e,
    }
}

/// The Python exception for an error of the library.
fn error(py: Python<'_>, e: Error) -> PyErr {
    let shown = |p: Option<std::path::PathBuf>| p.map(|p| p.display().to_string());
    let message = e.to_string();
    match e {
        Error::Io { path, source } => {
            // As Python words it: without Rust's " (os error 2)".
            let errno = source.raw_os_error();
            let strerror = source.to_string();
            let strerror = match errno {
                Some(n) => strerror
                    .strip_suffix(&format!(" (os error {n})"))
                    .unwrap_or(&strerror)
                    .to_owned(),
                None => strerror,
            };
            raise(py, "FileError", (errno, strerror, shown(path)))
        }
        Error::Parse {
            path,
            line,
            column,
            message,
        } => raise(py, "ParseError", (message, shown(path), line, column)),
        Error::UnknownFormat { path } => raise(py, "UnknownFormatError", (message, shown(Some(path)))),
        Error::Snapshot { path, message } => raise(py, "SnapshotError", (message, shown(path))),
        Error::NotReasoned => raise(py, "NotReasonedError", (message,)),
        Error::Cancelled => raise(py, "CancelledError", (message,)),
    }
}

/// A `TermError` for a term rdflib holds that RDF does not allow here.
fn term_error(py: Python<'_>, message: String) -> PyErr {
    raise(py, "TermError", (message,))
}

/// The rdflib term classes.
struct Rdflib<'py> {
    uriref: Bound<'py, PyType>,
    bnode: Bound<'py, PyType>,
    literal: Bound<'py, PyType>,
}

impl<'py> Rdflib<'py> {
    fn new(py: Python<'py>) -> PyResult<Self> {
        static URIREF: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        static BNODE: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        static LITERAL: PyOnceLock<Py<PyType>> = PyOnceLock::new();
        Ok(Self {
            uriref: URIREF.import(py, "rdflib.term", "URIRef")?.clone(),
            bnode: BNODE.import(py, "rdflib.term", "BNode")?.clone(),
            literal: LITERAL.import(py, "rdflib.term", "Literal")?.clone(),
        })
    }
}

/// An iterable of rdflib triples as `oxrdf` triples, all converted before
/// any is used.
fn from_python(triples: &Bound<'_, PyAny>) -> PyResult<Vec<Triple>> {
    let py = triples.py();
    let rdflib = Rdflib::new(py)?;
    let mut out = Vec::with_capacity(triples.len().unwrap_or(0));
    for item in triples.try_iter()? {
        let item = item?;
        let [s, p, o] = parts(&item)?;
        out.push(Triple::new(
            subject(&rdflib, &s)?,
            predicate(&rdflib, &p)?,
            object(&rdflib, &o)?,
        ));
    }
    Ok(out)
}

/// The three items of a triple: a tuple, or any other sequence of three.
fn parts<'py>(item: &Bound<'py, PyAny>) -> PyResult<[Bound<'py, PyAny>; 3]> {
    if let Ok(t) = item.cast::<PyTuple>()
        && t.len() == 3
    {
        return Ok([t.get_item(0)?, t.get_item(1)?, t.get_item(2)?]);
    }
    if !item.is_instance_of::<PyString>()
        && let Ok(seq) = item.cast::<PySequence>()
        && seq.len()? == 3
    {
        return Ok([seq.get_item(0)?, seq.get_item(1)?, seq.get_item(2)?]);
    }
    Err(PyTypeError::new_err(format!(
        "expected a (subject, predicate, object) triple, got {}",
        item.repr()?
    )))
}

fn subject(rdflib: &Rdflib<'_>, term: &Bound<'_, PyAny>) -> PyResult<NamedOrBlankNode> {
    if term.is_instance(&rdflib.uriref)? {
        Ok(iri(term)?.into())
    } else if term.is_instance(&rdflib.bnode)? {
        Ok(blank(term)?.into())
    } else if term.is_instance(&rdflib.literal)? {
        Err(term_error(
            term.py(),
            format!("a subject cannot be a literal: {}", term.repr()?),
        ))
    } else {
        Err(not_a_term(term))
    }
}

fn predicate(rdflib: &Rdflib<'_>, term: &Bound<'_, PyAny>) -> PyResult<NamedNode> {
    if term.is_instance(&rdflib.uriref)? {
        iri(term)
    } else if term.is_instance(&rdflib.bnode)? || term.is_instance(&rdflib.literal)? {
        Err(term_error(
            term.py(),
            format!("a predicate must be a URIRef: {}", term.repr()?),
        ))
    } else {
        Err(not_a_term(term))
    }
}

fn object(rdflib: &Rdflib<'_>, term: &Bound<'_, PyAny>) -> PyResult<Term> {
    if term.is_instance(&rdflib.uriref)? {
        Ok(iri(term)?.into())
    } else if term.is_instance(&rdflib.literal)? {
        Ok(literal(term)?.into())
    } else if term.is_instance(&rdflib.bnode)? {
        Ok(blank(term)?.into())
    } else {
        Err(not_a_term(term))
    }
}

fn not_a_term(term: &Bound<'_, PyAny>) -> PyErr {
    let shown = term
        .repr()
        .map(|r| r.to_string())
        .unwrap_or_else(|_| "an object".to_owned());
    PyTypeError::new_err(format!("expected an rdflib URIRef, BNode or Literal, got {shown}"))
}

/// The string an rdflib term is (they subclass `str`).
fn text<'a>(term: &'a Bound<'_, PyAny>) -> PyResult<std::borrow::Cow<'a, str>> {
    term.cast::<PyString>()?.to_cow()
}

fn iri(term: &Bound<'_, PyAny>) -> PyResult<NamedNode> {
    let s = text(term)?;
    NamedNode::new(s.as_ref()).map_err(|e| term_error(term.py(), format!("not an absolute IRI: {s:?} ({e})")))
}

fn blank(term: &Bound<'_, PyAny>) -> PyResult<BlankNode> {
    let s = text(term)?;
    BlankNode::new(s.as_ref()).map_err(|e| term_error(term.py(), format!("not a blank node label: {s:?} ({e})")))
}

fn literal(term: &Bound<'_, PyAny>) -> PyResult<Literal> {
    let py = term.py();
    let value = text(term)?;
    let language = term.getattr(intern!(py, "language"))?;
    if !language.is_none() {
        let tag = text(&language)?;
        return Literal::new_language_tagged_literal(value.as_ref(), tag.as_ref())
            .map_err(|e| term_error(py, format!("not a language tag: {tag:?} ({e})")));
    }
    let datatype = term.getattr(intern!(py, "datatype"))?;
    if datatype.is_none() {
        return Ok(Literal::new_simple_literal(value.as_ref()));
    }
    Ok(Literal::new_typed_literal(value.as_ref(), iri(&datatype)?))
}

/// Builds rdflib terms, each distinct term once.
struct ToPython<'py> {
    py: Python<'py>,
    rdflib: Rdflib<'py>,
    built: FxHashMap<Term, Bound<'py, PyAny>>,
}

impl<'py> ToPython<'py> {
    fn new(py: Python<'py>) -> PyResult<Self> {
        Ok(Self {
            py,
            rdflib: Rdflib::new(py)?,
            built: FxHashMap::default(),
        })
    }

    fn triples(&mut self, triples: impl Iterator<Item = Triple>) -> PyResult<Bound<'py, PyList>> {
        let out = triples
            .map(|t| {
                let s = self.term(t.subject.into())?;
                let p = self.term(t.predicate.into())?;
                let o = self.term(t.object)?;
                PyTuple::new(self.py, [s, p, o])
            })
            .collect::<PyResult<Vec<_>>>()?;
        PyList::new(self.py, out)
    }

    fn term(&mut self, term: Term) -> PyResult<Bound<'py, PyAny>> {
        if let Some(built) = self.built.get(&term) {
            return Ok(built.clone());
        }
        let built = match &term {
            Term::NamedNode(n) => self.rdflib.uriref.call1((n.as_str(),))?,
            Term::BlankNode(b) => self.rdflib.bnode.call1((b.as_str(),))?,
            Term::Literal(l) => {
                let none = self.py.None();
                if let Some(language) = l.language() {
                    self.rdflib.literal.call1((l.value(), language, none, false))?
                } else if l.datatype().as_str() == XSD_STRING {
                    self.rdflib.literal.call1((l.value(),))?
                } else {
                    let datatype = self.term(l.datatype().into_owned().into())?;
                    self.rdflib.literal.call1((l.value(), none, datatype, false))?
                }
            }
        };
        self.built.insert(term, built.clone());
        Ok(built)
    }
}
