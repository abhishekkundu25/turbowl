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

Switches
    Every rule belongs to the always-on core (subclasses, domains, ranges,
    inverses, ...) or to one switch: a named group of rules, on or off.
    ``Reasoner(switches={"checks": False})`` changes the defaults,
    ``Reasoner(complete=True)`` starts from every switch on (the complete
    W3C OWL 2 RL rule set), and :attr:`Reasoner.switches` reads them. The
    names are the command line's (``turbowl --switch checks=off``). Each
    switch, its default, what it does and what it costs:

    ``equality`` (on)
        ``owl:sameAs`` makes terms equal (eq-sym, eq-trans, eq-rep-*). Cheap
        until ``owl:sameAs`` occurs; each merge then rewrites the merged
        terms' facts.
    ``property-chains`` (on)
        ``owl:propertyChainAxiom`` (prp-spo2). A join per link of each chain;
        free without chains.
    ``checks`` (on)
        Reports contradictions and ill-typed literals (cax-dw, dt-not-type,
        ...). Derives no triple: off saves a pass over the facts each check
        names, and loses the diagnostics.
    ``identity`` (off)
        ``owl:hasKey`` and max cardinality 1 make individuals the same
        (prp-key, cls-maxc2, ...). A join per key and restriction, then
        equality's cost per merge: 26% slower on uobm-010-u, whose closure it
        grows from 21 M to 39 M triples.
    ``schema`` (off)
        Subsumptions between schema axioms (scm-spo, scm-dom1, ...). Its cost
        grows with the data too: 35% slower on LUBM at every size, though it
        adds only 99 triples there.
    ``trivial`` (off)
        Filler: ``x owl:sameAs x`` for every term, four triples per class,
        each literal's datatypes (eq-ref, scm-cls, dt-type2, ...). Grows with
        every term.
    ``literal-values`` (off)
        Literals equal by value, as ``"1"`` and ``"01"`` of ``xsd:integer``
        (dt-eq, dt-diff). Reads every literal's value, and merges equal ones
        at equality's cost per merge; dt-diff only checks, it lists no
        ``owl:differentFrom`` triples.

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

Snapshots
    :meth:`Reasoner.save_snapshot` saves a reasoner's whole state to a file
    (the terms, the base, the closure, the diagnostics, the switches, and any
    change since the last run), and :meth:`Reasoner.from_snapshot` loads it
    back without parsing or reasoning::

        r.save_snapshot("state.snapshot")
        again = turbowl.Reasoner.from_snapshot("state.snapshot")

    A snapshot of another TurbOWL version, or a damaged one, raises
    :class:`SnapshotError`.

Explanations
    :meth:`Reasoner.explain` tells why a triple is in the closure, and
    :meth:`Reasoner.explain_diagnostic` why a diagnostic is reported, as a
    :class:`Proof`: each :class:`Step` a statement and the W3C rule that
    concludes it from earlier steps, down to base triples::

        proof = r.explain(EX.rex, RDF.type, EX.Animal)
        proof.steps[-1].rule      # "cax-sco": the goal is the last step
        proof.leaves()            # the base triples it rests on
        print(proof)              # an indented tree, the goal first

    A triple outside the closure, or a diagnostic not reported, gives
    ``None``. A proof is of the last run's closure: after a change, until
    :meth:`Reasoner.reason` runs again, asking raises
    :class:`NotReasonedError`. The reasoner keeps what the first proof
    builds (its indexes) for the next ones, until it changes.

Errors
    :class:`TurbowlError` is the base of every exception raised here:
    :class:`ParseError` (with the line and column), :class:`FileError` (an
    ``OSError``), :class:`UnknownFormatError`, :class:`TermError`,
    :class:`SnapshotError` and :class:`NotReasonedError` (a
    ``RuntimeError``). Data problems are not exceptions: they are
    diagnostics.

Threads
    Parsing, reasoning, writing, snapshots and explaining release the GIL. A
    reasoner is used by one thread at a time; a call while another thread's
    call runs raises ``RuntimeError``.
"""

from __future__ import annotations

import dataclasses
import enum
import os
from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from typing import Union

from rdflib import Graph
from rdflib.term import BNode, Literal, Node, URIRef

from turbowl import _native
from turbowl._errors import (
    FileError,
    NotReasonedError,
    ParseError,
    SnapshotError,
    TermError,
    TurbowlError,
    UnknownFormatError,
)

Triple = tuple[Union[URIRef, BNode], URIRef, Union[URIRef, BNode, Literal]]
"""A triple as TurbOWL returns it."""

Triples = Union[Graph, Iterable[tuple[Node, Node, Node]]]
"""What the methods that take triples accept: a graph, or an iterable of triples."""

__all__ = [
    "Diagnostic",
    "FileError",
    "NotReasonedError",
    "ParseError",
    "Proof",
    "Reasoner",
    "Severity",
    "SnapshotError",
    "Statement",
    "Step",
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
        code: The W3C rule that found it: ``cax-dw``, ``cax-adc``,
            ``prp-pdw``, ``prp-adp``, ``cls-nothing2``, ``prp-asyp``,
            ``prp-irp``, ``prp-npa1``, ``prp-npa2``, ``cls-com``,
            ``cls-maxc1``, ``cls-maxqc1``, ``cls-maxqc2``, ``eq-diff1``,
            ``eq-diff2``, ``eq-diff3`` or ``dt-not-type``.
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
            - ``eq-diff1``: x, y (as in ``x owl:differentFrom y``, where
              ``x owl:sameAs y`` holds).
            - ``eq-diff2``, ``eq-diff3``: the ``owl:AllDifferent`` node, then
              two members made ``owl:sameAs``, the earlier first.
            - ``cax-adc``: individual, then two classes of an
              ``owl:AllDisjointClasses`` list, the earlier first.
            - ``prp-adp``: two properties of an ``owl:AllDisjointProperties``
              list, the earlier first, then subject, object.
            - ``prp-npa1``, ``prp-npa2``: source individual, property,
              target (the triple a negative property assertion denies).
            - ``cls-maxc1``, ``cls-maxqc1``, ``cls-maxqc2``: restriction,
              individual, value.

            The rules run over generalized triples, so a term may be a
            literal where RDF allows only an IRI or a blank node.
    """

    code: str
    severity: Severity
    message: str
    terms: tuple[Node, ...]


def _diagnostic(native: tuple[str, str, str, tuple[Node, ...]]) -> Diagnostic:
    code, severity, message, terms = native
    return Diagnostic(code=code, severity=Severity(severity), message=message, terms=terms)


Statement = tuple[Node, Node, Node]
"""A statement of a proof: a generalized triple, whose subject may be a
literal (as ``rdfs:range`` types a literal object)."""


@dataclass(frozen=True)
class Step:
    """One step of a :class:`Proof`: a statement and why it holds.

    Attributes:
        statement: The ``(subject, predicate, object)`` it holds, rdflib
            terms as :meth:`Reasoner.triples` returns them.
        rule: The W3C rule that concludes it (``"cax-sco"``,
            ``"prp-trp"``, ...), or ``None`` for a base triple (loaded or
            inserted).
        premises: The indexes in :attr:`Proof.steps` of the steps the rule
            reads, each before this one. Empty for a base triple, and for a
            rule with no premises (cls-thing, prp-ap, ...).
    """

    statement: Statement
    rule: str | None
    premises: tuple[int, ...]

    @property
    def base(self) -> bool:
        """True for a base triple."""
        return self.rule is None


@dataclass(frozen=True)
class Proof:
    """Why a triple is in the closure, or why a diagnostic is reported:
    steps down to base triples (:meth:`Reasoner.explain`,
    :meth:`Reasoner.explain_diagnostic`).

    The steps are in order, each premise before the steps that read it, and
    each statement once, so a subproof read twice is not repeated. ``str()``
    gives the proof as an indented tree, the goal first.

    Attributes:
        steps: The steps. For a triple's proof, the goal is the last.
        goal: The triple proved (the last step's statement), or the
            :class:`Diagnostic` proved, which is not a step.
        rule: The rule that concludes the goal, or ``None`` when the goal
            is a base triple.
        premises: The goal's premises, as indexes in :attr:`steps`.
        depth: The most rule steps on a path from the goal to a base
            triple: 0 for a base triple, 1 for a rule over base triples.
    """

    steps: tuple[Step, ...]
    goal: Statement | Diagnostic
    rule: str | None
    premises: tuple[int, ...]
    depth: int
    _text: str = dataclasses.field(repr=False, compare=False)

    def leaves(self) -> list[Statement]:
        """The base triples the proof rests on, each once, in step order."""
        return [step.statement for step in self.steps if step.base]

    def __str__(self) -> str:
        return self._text


def _proof(native) -> Proof | None:
    """A proof from the native module's tuples."""
    if native is None:
        return None
    steps, goal, rule, premises, depth, text = native
    steps = tuple(Step(statement=statement, rule=r, premises=p) for statement, r, p in steps)
    return Proof(
        steps=steps,
        goal=steps[-1].statement if goal is None else _diagnostic(goal),
        rule=rule,
        premises=premises,
        depth=depth,
        _text=text,
    )


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

    def __init__(self, *, switches: Mapping[str, bool] | None = None, complete: bool = False) -> None:
        """An empty reasoner.

        Its switches start from the defaults, or from every switch on when
        ``complete`` is true; ``switches`` then turns each switch it names on
        (``True``) or off (``False``). The names and defaults are in the
        module doc::

            Reasoner(switches={"equality": False, "checks": False})
            Reasoner(complete=True, switches={"literal-values": False})

        Raises:
            ValueError: A name in ``switches`` is not a switch.
            TypeError: ``switches`` is not a mapping of names to ``True`` or
                ``False``.
        """
        self._native = _native.NativeReasoner(switches, complete)

    @classmethod
    def from_snapshot(cls, path: str | os.PathLike[str]) -> Reasoner:
        """The reasoner a snapshot file holds (:meth:`save_snapshot`).

        It is the saved reasoner: the same closure, diagnostics and switches,
        and the same run due next, so that :meth:`reason` and the changes
        then do what they would have done on it.

        Raises:
            FileError: The file cannot be read.
            SnapshotError: The file is not a snapshot this TurbOWL reads:
                another format or TurbOWL version, truncated, damaged or
                malformed.
        """
        reasoner = cls.__new__(cls)
        reasoner._native = _native.NativeReasoner.from_snapshot(os.fspath(path))
        return reasoner

    def save_snapshot(self, path: str | os.PathLike[str]) -> None:
        """Saves the reasoner's whole state to a file, which
        :meth:`from_snapshot` loads back without parsing or reasoning.

        The file is written beside ``path`` under a temporary name and
        renamed, so a crash never leaves half a snapshot.

        Raises:
            FileError: The file cannot be written.
        """
        self._native.save_snapshot(os.fspath(path))

    @property
    def switches(self) -> dict[str, bool]:
        """Every switch's name, mapped to whether it is on, in the order of
        the module doc. A new dict: changing it changes nothing."""
        return self._native.switches()

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
        """Empties the base, the closure and the diagnostics: as new, with
        the same switches."""
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
        return [_diagnostic(d) for d in self._native.diagnostics()]

    def explain(self, subject: Node, predicate: Node, object: Node) -> Proof | None:
        """Why a triple is in the closure of the last run: its :class:`Proof`
        from base triples, by the rules the switches leave on, or ``None``
        when the triple is not in the closure.

        The terms are rdflib terms, as for :meth:`insert`. A blank node is
        named as the reads return it: ``BNode("b_1")`` for a loaded
        document's ``_:b`` (see the module doc).

        The first proof builds indexes over the closure, which the reasoner
        keeps for the next proofs until it changes.

        Raises:
            NotReasonedError: The reasoner changed since its last run.
            TypeError: A term is not an rdflib term.
            TermError: A term RDF does not allow there (a literal subject, a
                predicate that is not a ``URIRef``, ...).
        """
        return _proof(self._native.explain(subject, predicate, object))

    def explain_diagnostic(self, diagnostic: Diagnostic) -> Proof | None:
        """Why a diagnostic of the last run is reported: its :class:`Proof`,
        whose goal is the diagnostic and whose steps prove the rule's
        premises, or ``None`` when no diagnostic of the last run has its
        code and terms.

        Raises:
            NotReasonedError: The reasoner changed since its last run.
            TypeError: A term is not an rdflib term.
        """
        return _proof(self._native.explain_diagnostic(diagnostic.code, diagnostic.terms))

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
