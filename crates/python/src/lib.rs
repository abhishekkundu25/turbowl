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
//! Parsing, reasoning and writing release the GIL.

use oxrdf::{BlankNode, Literal, NamedNode, NamedOrBlankNode, Term, Triple};
use pyo3::call::PyCallArgs;
use pyo3::exceptions::PyTypeError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyList, PySequence, PyString, PyTuple, PyType};
use rustc_hash::FxHashMap;
use std::io::Write as _;
use std::path::PathBuf;
use turbowl::{Error, Format, Severity};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// The native module.
#[pymodule]
#[pyo3(name = "_native")]
fn native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<NativeReasoner>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

/// The reasoner behind `turbowl.Reasoner`. Its methods take and return
/// rdflib terms; see the module doc of the crate for the conversions.
#[pyclass(module = "turbowl._native")]
#[derive(Debug, Default)]
struct NativeReasoner {
    inner: turbowl::Reasoner,
}

#[pymethods]
impl NativeReasoner {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    /// Adds a file's triples to the base; the format comes from its extension.
    fn load_file(&mut self, py: Python<'_>, path: PathBuf) -> PyResult<()> {
        let inner = &mut self.inner;
        py.detach(|| inner.load_file(&path)).map_err(|e| error(py, e))
    }

    /// Adds a document's triples to the base. `format` is a canonical name
    /// (`turtle`, `ntriples`, `n3`, `rdfxml`).
    #[pyo3(signature = (text, format, base_iri=None))]
    fn load_str(&mut self, py: Python<'_>, text: &str, format: &str, base_iri: Option<&str>) -> PyResult<()> {
        let format = format_named(py, format)?;
        let inner = &mut self.inner;
        py.detach(|| inner.load_str(text, format, base_iri))
            .map_err(|e| error(py, e))
    }

    /// Adds an iterable of `(subject, predicate, object)` rdflib triples.
    fn insert(&mut self, py: Python<'_>, triples: &Bound<'_, PyAny>) -> PyResult<()> {
        let triples = from_python(triples)?;
        let inner = &mut self.inner;
        py.detach(|| inner.insert(triples.iter().map(Triple::as_ref)));
        Ok(())
    }

    /// Removes an iterable of rdflib triples from the base.
    fn remove(&mut self, py: Python<'_>, triples: &Bound<'_, PyAny>) -> PyResult<()> {
        let triples = from_python(triples)?;
        let inner = &mut self.inner;
        py.detach(|| inner.remove(triples.iter().map(Triple::as_ref)));
        Ok(())
    }

    /// Makes the base exactly these rdflib triples; true when a base triple
    /// was removed.
    fn set_base(&mut self, py: Python<'_>, triples: &Bound<'_, PyAny>) -> PyResult<bool> {
        let triples = from_python(triples)?;
        let inner = &mut self.inner;
        Ok(py.detach(|| inner.set_base(triples.iter().map(Triple::as_ref))))
    }

    /// Empties the reasoner.
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Runs to the fixpoint: `(base, inferred, rounds, diagnostics)`.
    fn reason(&mut self, py: Python<'_>) -> (usize, usize, usize, usize) {
        let inner = &mut self.inner;
        let s = py.detach(|| inner.reason());
        (s.base, s.inferred, s.rounds, s.diagnostics)
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
        let mut out = Vec::with_capacity(self.inner.diagnostics().len());
        for d in self.inner.diagnostics() {
            let severity = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            let terms = d
                .terms
                .iter()
                .map(|t| to.term(t.clone()))
                .collect::<PyResult<Vec<_>>>()?;
            out.push((d.code, severity, d.message.as_str(), PyTuple::new(py, terms)?));
        }
        PyList::new(py, out)
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
