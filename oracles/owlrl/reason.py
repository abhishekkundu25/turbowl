"""The OWL-RL oracle: RDFLib's OWL 2 RL closure of N-Triples input.

Usage: reason.py INPUT.nt CLOSURE.nt FINDINGS.tsv

Writes the closure as N-Triples (without literal subjects, which RDF 1.1
cannot write, and without OWL-RL's error-message triples) and one line per
inconsistency OWL-RL reports: the W3C rule name `crates/oracle` uses as the
code, then the terms involved in N-Triples syntax, tab-separated, in the
order documented on `Finding` (crates/oracle/src/closure.rs). A message this
script does not recognise (a rule outside TurbOWL's rule set, such
as eq-diff1 or cls-maxc1) is written with the code `other` and the message
itself.
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
# OWL-RL formats cax-adc like cax-dw, so it is read as cax-dw.
T = r"(.+?)"
PATTERNS = [
    (re.compile(rf"^Disjoint classes {T} and {T} have a common individual {T}$"), "cax-dw", (2, 0, 1)),
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
]


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
            findings.write("\t".join(finding(message, by_string, input_literals)) + "\n")


def finding(message, by_string, input_literals):
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
            return [code, literal.n3(), URIRef(datatype).n3()]
        groups = match.groups()
        terms = [by_string.get(groups[i]) for i in order]
        if all(t is not None for t in terms):
            return [code] + [t.n3() for t in terms]
    return ["other", message.replace("\t", " ").replace("\n", " ")]


if __name__ == "__main__":
    main(sys.argv)
