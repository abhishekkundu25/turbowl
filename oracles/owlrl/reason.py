"""The OWL-RL oracle: RDFLib's OWL 2 RL closure of N-Triples input.

Usage: reason.py INPUT.nt CLOSURE.nt FINDINGS.tsv

Writes the closure as N-Triples (without literal subjects, which RDF 1.1
cannot write, and without OWL-RL's error-message triples) and one line per
inconsistency OWL-RL reports: the W3C rule name `crates/oracle` uses as the
code, then the terms involved in N-Triples syntax, tab-separated, in the
order documented on `Finding` (crates/oracle/src/closure.rs). A message this
script does not recognise (a rule outside TurbOWL's rule set) is written with
the code `other` and the message itself.

Some messages name fewer terms than the finding, or are shared by two rules;
the rest is read from OWL-RL's own closure, the graph the message was made
from (see `complete`): which rule a cax-dw or cax-adc message, and which
axiom an eq-diff2 or eq-diff3 message, comes from, and the individual of a
cardinality message.
"""

import logging
import re
import sys

from owlrl import DatatypeHandling
from owlrl.Namespaces import ERRNS
from owlrl.OWLRL import OWLRL_Semantics
import rdflib
from rdflib import Graph, Literal, URIRef

# OWL-RL formats each term with `%s`, so a message holds bare strings.
# Each pattern names the code and the order of the captured terms in the
# order `Finding` documents for that code (crates/oracle/src/closure.rs).
# A pattern whose code is None is completed from the graph (`complete`).
T = r"(.+?)"
PATTERNS = [
    # cax-dw and cax-adc share this message (OWLRL.py:836-857).
    (re.compile(rf"^Disjoint classes {T} and {T} have a common individual {T}$"), "disjoint-classes", (0, 1, 2)),
    (re.compile(rf"^Erroneous usage of disjoint properties {T} and {T} on {T} and {T}$"), "prp-pdw", (0, 1, 2, 3)),
    (re.compile(rf"^{T} is defined of type 'Nothing'$"), "cls-nothing2", (0,)),
    (re.compile(rf"^Erroneous usage of asymmetric property {T} on {T} and {T}$"), "prp-asyp", (0, 1, 2)),
    (re.compile(rf"^Irreflexive property used on {T} with {T}$"), "prp-irp", (1, 0)),
    # owlrl 7.6.2 prints the second class where the element belongs
    # (OWLRL.py:684-691 formats `x`, which is c2 there), so the third term is
    # c2: oracle fault F2.
    (re.compile(rf"^Violation of complementarity for classes {T} and {T} on element {T}$"), "cls-com", (0, 1, 2)),
    (
        re.compile(rf"^Lexical value of the literal '{T}' does not match its datatype \({T}\)$", re.S),
        "dt-not-type",
        None,
    ),
    (re.compile(rf"^'sameAs' and 'differentFrom' cannot be used on the same subject-object pair: \({T}, {T}\)$"), "eq-diff1", (0, 1)),
    # eq-diff2 and eq-diff3 share this message (OWLRL.py:416-436).
    (re.compile(rf"^'sameAs' and 'AllDifferent' cannot be used on the same subject-object pair: \({T}, {T}\)$"), "all-different", (0, 1)),
    (
        re.compile(rf"^Disjoint properties in an 'AllDisjointProperties' are not really disjoint: \({T}, {T},{T}\) and \({T},{T},{T}\)$"),
        "prp-adp",
        (1, 4, 0, 2),
    ),
    (re.compile(rf"^Negative \(object\) property assertion violated for: \({T}, {T}, {T}\)$"), "prp-npa1", (0, 1, 2)),
    (re.compile(rf"^Negative \(datatype\) property assertion violated for: \({T}, {T}, {T}\)$"), "prp-npa2", (0, 1, 2)),
    # The restriction and the value: the individual is read from the graph.
    (re.compile(rf"^Erroneous usage of maximum cardinality with {T} and {T}$"), "max-cardinality", (0, 1)),
    # cls-maxqc1 and cls-maxqc2 share this message: restriction, class, value.
    (re.compile(rf"^Erroneous usage of maximum qualified cardinality with {T}, {T} and {T}$"), "max-qualified", (0, 1, 2)),
]

OWL = "http://www.w3.org/2002/07/owl#"
TYPE = rdflib.RDF.type
REST, NIL = rdflib.RDF.rest, rdflib.RDF.nil


def main(argv):
    if len(argv) != 4:
        sys.exit(__doc__)
    input_path, closure_path, findings_path = argv[1:]

    # rdflib logs every ill-formed literal it cannot convert; OWL-RL reports them.
    logging.getLogger("rdflib.term").setLevel(logging.CRITICAL)

    # Keep literals as written: rdflib would rewrite " 42"^^xsd:int as "42".
    rdflib.NORMALIZE_LITERALS = False

    graph = Graph()
    graph.parse(input_path, format="nt")
    input_literals = {o for o in graph.objects() if isinstance(o, Literal)}

    # As DeductiveClosure(OWLRL_Semantics).expand does, with improved datatypes,
    # no axiomatic triples and no datatype axioms; the closure object is kept so
    # its error messages can be read without parsing the graph.
    DatatypeHandling.use_Alt_lexical_conversions()
    closure = OWLRL_Semantics(graph, False, False, rdfs=False)
    closure.closure()
    DatatypeHandling.use_RDFLib_lexical_conversions()

    message_nodes = set(graph.subjects(None, ERRNS.ErrorMessage))
    out = Graph()
    for s, p, o in graph:
        if s in message_nodes or isinstance(s, Literal) or not isinstance(p, URIRef):
            continue
        out.add((s, p, o))
    out.serialize(destination=closure_path, format="nt", encoding="utf-8")

    by_string = {}
    for s, p, o in graph:
        for term in (o, p, s):
            if s in message_nodes:
                continue
            # Prefer an IRI or blank node when a literal has the same string.
            if str(term) not in by_string or isinstance(by_string[str(term)], Literal):
                by_string[str(term)] = term

    with open(findings_path, "w", encoding="utf-8") as findings:
        for message in closure.error_messages:
            for found in message_findings(message, graph, by_string, input_literals):
                findings.write("\t".join(found) + "\n")


def message_findings(message, graph, by_string, input_literals):
    """The findings of one OWL-RL message: a list of [code, term, ...]."""
    for pattern, code, order in PATTERNS:
        match = pattern.match(message)
        if not match:
            continue
        if order is None:
            # dt-not-type: OWL-RL checks only a literal against its own
            # datatype (oracle fault F5), so the datatype is the literal's.
            lexical, datatype = match.groups()
            literal = next(
                (lit for lit in input_literals if str(lit) == lexical and str(lit.datatype) == datatype),
                Literal(lexical, datatype=URIRef(datatype)),
            )
            return [[code, literal.n3(), URIRef(datatype).n3()]]
        groups = match.groups()
        terms = [by_string.get(groups[i]) for i in order]
        if all(t is not None for t in terms):
            return [[c] + [t.n3() for t in ts] for c, ts in complete(code, terms, graph)]
    return [["other", message.replace("\t", " ").replace("\n", " ")]]


def ordered_in_list(graph, head, a, b):
    """Whether OWL-RL's own loops (OWLRL.py:416-436, 517-531) pair `a` then
    `b` in the list at `head`: `graph.items` reads one rdf:first per cell,
    and the second index stops before the last item."""
    items = list(graph.items(head))
    return any(items[i] == a and b in items[i + 1 : len(items) - 1] for i in range(len(items) - 1))


def complete(code, terms, graph):
    """The (code, terms) a message stands for, in `Finding`'s orders: the
    messages that name fewer terms, or that two rules share, are read
    against the graph OWL-RL made them from."""
    owl = lambda local: URIRef(OWL + local)
    if code == "disjoint-classes":
        c1, c2, x = terms
        found = []
        if (c1, owl("disjointWith"), c2) in graph:
            found.append(("cax-dw", [x, c1, c2]))
        for axiom in graph.subjects(TYPE, owl("AllDisjointClasses")):
            for head in graph.objects(axiom, owl("members")):
                items = list(graph.items(head))
                if any(items[i] == c1 and c2 in items[i + 1 :] for i in range(len(items))):
                    found.append(("cax-adc", [x, c1, c2]))
        return found
    if code == "all-different":
        zi, zj = terms
        found = []
        for axiom in graph.subjects(TYPE, owl("AllDifferent")):
            for rule, predicate in (("eq-diff2", "members"), ("eq-diff3", "distinctMembers")):
                for head in graph.objects(axiom, owl(predicate)):
                    if ordered_in_list(graph, head, zi, zj):
                        found.append((rule, [axiom, zi, zj]))
        return found
    if code == "eq-diff1":
        # The message names the owl:sameAs pair; the finding, the
        # owl:differentFrom pair as stated, either way round.
        s, o = terms
        return [("eq-diff1", [x, y]) for x, y in ((s, o), (o, s)) if (x, owl("differentFrom"), y) in graph]
    if code in ("max-cardinality", "max-qualified"):
        if code == "max-cardinality":
            (restriction, value), rules = terms, ["cls-maxc1"]
        else:
            restriction, cls, value = terms
            rules = [r for r, holds in (("cls-maxqc1", (value, TYPE, cls) in graph), ("cls-maxqc2", cls == owl("Thing"))) if holds]
        individuals = {
            u
            for p in graph.objects(restriction, owl("onProperty"))
            for u in graph.subjects(p, value)
            if (u, TYPE, restriction) in graph
        }
        return [(r, [restriction, u, value]) for r in rules for u in individuals]
    return [(code, terms)]


if __name__ == "__main__":
    main(sys.argv)
