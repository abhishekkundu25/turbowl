"""TurbOWL: a fast OWL 2 RL reasoner, with rdflib graphs and terms in and out.

Load documents or insert triples, :meth:`Reasoner.reason` to the fixpoint,
then read the closure, the inferred triples and the diagnostics::

    from rdflib import Graph, Namespace, RDF, RDFS
    import turbowl

    EX = Namespace("http://example.org/")
    g = Graph()
    g.add((EX.Dog, RDFS.subClassOf, EX.Animal))
    g.add((EX.rex, RDF.type, EX.Dog))

    r = turbowl.Reasoner()
    r.insert(g)
    summary = r.reason()                  # Summary(base=2, inferred=..., ...)
    assert (EX.rex, RDF.type, EX.Animal) in r.inferred()
    closure = r.graph()                   # an rdflib Graph of the closure

The rules are the W3C OWL 2 RL rules TurbOWL implements; a run after a
change reuses the previous closure.

Terms
    Triples in are ``(subject, predicate, object)`` tuples of rdflib terms
    (``URIRef``, ``BNode``, ``Literal``), or an rdflib ``Graph``. Triples out
    are tuples of the same classes. A literal comes back with the lexical
    form the reasoner holds (``"01"^^xsd:integer`` stays ``"01"``), a literal
    of ``xsd:string`` comes back with no datatype (as rdflib's parsers read
    ``"x"``), and language tags come back lowercase (rdflib compares them
    ignoring case, so the literal is equal to the one inserted).

Blank nodes
    A ``BNode`` you insert keeps its label: ``BNode("x")`` is the same node in
    every call to :meth:`Reasoner.insert`, :meth:`Reasoner.remove` and
    :meth:`Reasoner.set_base`, and comes back as ``BNode("x")``. Blank nodes
    of a loaded document are that document's own: ``_:b`` in two documents
    is two nodes, which come back labelled ``b_1`` and ``b_2`` (the label,
    then the document's number), and :meth:`Reasoner.remove` cannot reach
    them. So that the two kinds never meet, an inserted label of that shape
    (``_`` and digits at the end) comes back with ``_0`` appended:
    ``BNode("b_1")`` comes back as ``BNode("b_1_0")``.

Errors
    :class:`TurbowlError` is the base of every exception raised here:
    :class:`ParseError` (with the line and column), :class:`FileError` (an
    ``OSError``), :class:`UnknownFormatError` and :class:`TermError`. Data
    problems are not exceptions: they are diagnostics.

Threads
    Parsing, reasoning and writing release the GIL. A reasoner is used by
    one thread at a time; a call while another thread's call runs raises
    ``RuntimeError``.
"""

from __future__ import annotations

import enum
import os
from collections.abc import Iterable
from dataclasses import dataclass
from typing import Union

from rdflib import Graph
from rdflib.term import BNode, Literal, Node, URIRef

from turbowl import _native
from turbowl._errors import FileError, ParseError, TermError, TurbowlError, UnknownFormatError

Triple = tuple[Union[URIRef, BNode], URIRef, Union[URIRef, BNode, Literal]]
"""A triple as TurbOWL returns it."""

Triples = Union[Graph, Iterable[tuple[Node, Node, Node]]]
"""What the methods that take triples accept: a graph, or an iterable of triples."""

__all__ = [
    "Diagnostic",
    "FileError",
    "ParseError",
    "Reasoner",
    "Severity",
    "Summary",
    "TermError",
    "Triple",
    "Triples",
    "TurbowlError",
    "UnknownFormatError",
    "__version__",
]

__version__: str = _native.__version__

# Format names, as rdflib names them and more, to the canonical name the
# native module takes. Matched ignoring case.
_FORMATS = {
    "turtle": "turtle",
    "ttl": "turtle",
    "text/turtle": "turtle",
    "nt": "ntriples",
    "nt11": "ntriples",
    "ntriples": "ntriples",
    "n-triples": "ntriples",
    "application/n-triples": "ntriples",
    "n3": "n3",
    "text/n3": "n3",
    "xml": "rdfxml",
    "rdfxml": "rdfxml",
    "rdf/xml": "rdfxml",
    "application/rdf+xml": "rdfxml",
}


def _format(name: str) -> str:
    """The canonical name of a format name, or :class:`UnknownFormatError`."""
    canonical = _FORMATS.get(name.lower()) if isinstance(name, str) else None
    if canonical is None:
        known = ", ".join(sorted(_FORMATS))
        raise UnknownFormatError(f"unknown format {name!r} (use one of: {known})")
    return canonical


def _triples(triples: Triples) -> Iterable[tuple[Node, Node, Node]]:
    """A graph's triples (any rdflib graph, a ``Dataset`` included), or the iterable itself."""
    if isinstance(triples, Graph):
        return triples.triples((None, None, None))
    return triples


class Severity(str, enum.Enum):
    """How serious a diagnostic is.

    Every inconsistency of the rule set is an ``ERROR``; ``WARNING`` is kept
    for later checks. A ``str``: ``Severity.ERROR == "error"``.
    """

    ERROR = "error"
    WARNING = "warning"


@dataclass(frozen=True)
class Summary:
    """What a run did.

    Attributes:
        base: Base triples (loaded or inserted).
        inferred: Inferred triples: the ones :meth:`Reasoner.inferred` returns.
        rounds: Evaluation rounds this run took; 0 when nothing changed since
            the last run.
        diagnostics: Diagnostics after the run.
    """

    base: int
    inferred: int
    rounds: int
    diagnostics: int


@dataclass(frozen=True)
class Diagnostic:
    """A finding about the data: an inconsistency, or an ill-typed literal.

    Attributes:
        code: The W3C rule that found it: ``cax-dw``, ``prp-pdw``,
            ``cls-nothing2``, ``prp-asyp``, ``prp-irp``, ``cls-com`` or
            ``dt-not-type``.
        severity: How serious it is.
        message: One sentence naming the terms. The wording may change; read
            ``code`` and ``terms``.
        terms: The rdflib terms involved, in an order fixed per code:

            - ``cax-dw``: individual, class 1, class 2 (as in
              ``class1 owl:disjointWith class2``).
            - ``prp-pdw``: property 1, property 2 (as in
              ``owl:propertyDisjointWith``), subject, object.
            - ``cls-nothing2``: individual.
            - ``prp-asyp``: property, subject, object.
            - ``prp-irp``: property, individual.
            - ``cls-com``: class 1, class 2 (as in
              ``class1 owl:complementOf class2``), individual.
            - ``dt-not-type``: literal, datatype (the literal's own when it
              is ill-typed, or a checked datatype the closure types it with,
              as by ``rdfs:range``).

            The rules run over generalized triples, so a term may be a
            literal where RDF allows only an IRI or a blank node.
    """

    code: str
    severity: Severity
    message: str
    terms: tuple[Node, ...]


class Reasoner:
    """An OWL 2 RL reasoner over an in-memory graph.

    Change the base triples (:meth:`load_file`, :meth:`load_str`,
    :meth:`insert`, :meth:`remove`, :meth:`set_base`), then :meth:`reason`.
    The reads (:meth:`triples`, :meth:`inferred`, :meth:`graph`,
    :meth:`diagnostics`, :meth:`serialize`, :meth:`write`) show the closure
    of the last run, even after a change made since.

    A run after additions only joins the new triples against the stored
    closure; a run after a removal starts again from the base; a run with
    nothing changed takes no rounds.
    """

    __slots__ = ("_native",)

    def __init__(self) -> None:
        """An empty reasoner."""
        self._native = _native.NativeReasoner()

    def load_file(self, path: str | os.PathLike[str]) -> None:
        """Adds a file's triples to the base.

        The format comes from the extension: ``.ttl``, ``.nt``, ``.n3``,
        ``.rdf``, ``.owl`` or ``.xml``. The document's blank nodes are its
        own. All or nothing: on an error the base is unchanged. Loading the
        same document again changes nothing, unless a base triple was
        removed since.

        Raises:
            FileError: The file cannot be read.
            UnknownFormatError: The extension names no supported format.
            ParseError: The file is not valid in its format (with the line).
        """
        self._native.load_file(os.fspath(path))

    def load_str(self, data: str, format: str = "turtle", base_iri: str | None = None) -> None:
        """Adds a document given as text to the base.

        ``format`` is ``"turtle"`` (or ``"ttl"``), ``"nt"`` (or
        ``"ntriples"``), ``"n3"`` or ``"xml"`` (or ``"rdfxml"``), and their
        media types; case does not matter. ``base_iri`` resolves relative
        IRIs. Otherwise as :meth:`load_file`.

        Raises:
            UnknownFormatError: ``format`` names no supported format.
            ParseError: The text is not valid in its format (with the line).
        """
        self._native.load_str(data, _format(format), base_iri)

    def insert(self, triples: Triples) -> None:
        """Adds triples to the base: an rdflib ``Graph``, or any iterable of
        ``(subject, predicate, object)`` rdflib terms.

        A ``BNode`` keeps its label (see the module doc). All or nothing: a
        triple that cannot be converted leaves the base unchanged.

        Raises:
            TypeError: An item is not a triple of rdflib terms.
            TermError: A term RDF does not allow there (a literal subject, an
                IRI that is not absolute, ...).
        """
        self._native.insert(_triples(triples))

    def remove(self, triples: Triples) -> None:
        """Removes triples from the base: a ``Graph`` or an iterable, as for
        :meth:`insert`.

        Triples that are not base triples are ignored. After a removal the
        next run starts again from the base.

        Raises:
            TypeError: An item is not a triple of rdflib terms.
            TermError: A term RDF does not allow there.
        """
        self._native.remove(_triples(triples))

    def set_base(self, triples: Triples) -> bool:
        """Makes the base exactly these triples (a ``Graph`` or an iterable).

        Returns ``True`` when a base triple was removed, so the next run
        starts again from the base. A base that only grew runs
        incrementally; an unchanged base costs nothing (the next run takes
        no rounds), so setting the same graph again is free.

        Raises:
            TypeError: An item is not a triple of rdflib terms.
            TermError: A term RDF does not allow there.
        """
        return self._native.set_base(_triples(triples))

    def clear(self) -> None:
        """Empties the base, the closure and the diagnostics: as new."""
        self._native.clear()

    def reason(self) -> Summary:
        """Runs to the fixpoint, in full or over what changed since the last
        run, and returns what it did."""
        base, inferred, rounds, diagnostics = self._native.reason()
        return Summary(base=base, inferred=inferred, rounds=rounds, diagnostics=diagnostics)

    def triples(self) -> list[Triple]:
        """The closure: base and inferred triples, as a new list of rdflib
        triples (in no particular order).

        Generalized triples the rules derive internally (a literal subject,
        say) are left out. For a ``Graph``, use :meth:`graph`.
        """
        return self._native.triples()

    def inferred(self) -> list[Triple]:
        """Only the inferred triples (the closure's triples that are not base
        triples), as a new list of rdflib triples."""
        return self._native.inferred()

    def graph(self, inferred_only: bool = False) -> Graph:
        """The closure (or only the inferred triples) as a new rdflib ``Graph``."""
        g = Graph()
        g += self.inferred() if inferred_only else self.triples()
        return g

    def diagnostics(self) -> list[Diagnostic]:
        """The diagnostics of the last run, each reported once."""
        return [
            Diagnostic(code=code, severity=Severity(severity), message=message, terms=terms)
            for code, severity, message, terms in self._native.diagnostics()
        ]

    def serialize(self, format: str = "turtle", inferred_only: bool = False) -> str:
        """The closure (or only the inferred triples) as text.

        ``format`` as for :meth:`load_str`. Triples are sorted by subject, so
        Turtle groups each subject's triples.

        Raises:
            UnknownFormatError: ``format`` names no supported format.
        """
        return self._native.serialize(_format(format), inferred_only)

    def write(
        self, path: str | os.PathLike[str], format: str | None = None, inferred_only: bool = False
    ) -> None:
        """Writes the closure (or only the inferred triples) to a file.

        ``format`` as for :meth:`load_str`; ``None`` takes it from the
        extension, as :meth:`load_file` does.

        Raises:
            FileError: The file cannot be written.
            UnknownFormatError: The format, or the extension, names no
                supported format.
        """
        self._native.write(os.fspath(path), None if format is None else _format(format), inferred_only)
