"""The bindings against the Rust library, on the minimal inputs of the oracle
suite (``turbowl_oracle::cases::minimal()``: one per rule and per diagnostic).

``examples/minimal_cases.rs`` runs the Rust library on each input and writes
the input, the closure, the inferred triples, the diagnostics and the
summary as text (see its doc). Each case is then run through the bindings,
from rdflib terms, and must give the same sets. The files are read with the
small N-Triples reader below, not rdflib's: rdflib's parser renames blank
nodes and normalizes lexical forms, and the comparison is exact.
"""

from __future__ import annotations

import atexit
import functools
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

from rdflib import BNode, Graph, Literal, URIRef
from rdflib.term import Node

import turbowl

CRATE = Path(__file__).resolve().parents[1]
XSD_STRING = "http://www.w3.org/2001/XMLSchema#string"


@functools.cache
def generated() -> Path:
    """The directory the Rust example wrote into (once per session)."""
    out = Path(tempfile.mkdtemp(prefix="turbowl-minimal-"))
    atexit.register(shutil.rmtree, out, ignore_errors=True)
    command = ["cargo", "run", "--release", "--locked", "--quiet", "-p", "turbowl-python"]
    command += ["--example", "minimal_cases", "--", str(out)]
    subprocess.run(command, cwd=CRATE, check=True, timeout=110)
    return out


def case_names() -> list[str]:
    return (generated() / "cases.txt").read_text().split()


def pytest_generate_tests(metafunc):
    if "case" in metafunc.fixturenames:
        metafunc.parametrize("case", case_names())


# N-Triples terms, as oxrdf writes them.
_TERM = re.compile(r'<([^>]*)>|_:(\S+)|"((?:[^"\\]|\\.)*)"(?:@([A-Za-z0-9-]+)|\^\^<([^>]*)>)?')
_ESCAPE = re.compile(r"\\(?:u([0-9A-Fa-f]{4})|U([0-9A-Fa-f]{8})|(.))")
_ECHAR = {"t": "\t", "b": "\b", "n": "\n", "r": "\r", "f": "\f", '"': '"', "'": "'", "\\": "\\"}


def _unescape(text: str) -> str:
    def one(m: re.Match[str]) -> str:
        code = m[1] or m[2]
        return chr(int(code, 16)) if code else _ECHAR[m[3]]

    return _ESCAPE.sub(one, text)


def _term(m: re.Match[str]) -> Node:
    if m[1] is not None:
        return URIRef(m[1])
    if m[2] is not None:
        return BNode(m[2])
    value = _unescape(m[3])
    if m[4] is not None:
        return Literal(value, lang=m[4])
    if m[5] is not None and m[5] != XSD_STRING:
        return Literal(value, datatype=URIRef(m[5]), normalize=False)
    return Literal(value)


def parse_terms(text: str) -> list[Node]:
    """The N-Triples terms of ``text``, separated by spaces or tabs."""
    terms, pos = [], 0
    while True:
        while pos < len(text) and text[pos] in " \t":
            pos += 1
        if pos == len(text):
            return terms
        m = _TERM.match(text, pos)
        assert m, f"not an N-Triples term at {pos}: {text!r}"
        terms.append(_term(m))
        pos = m.end()


def read_triples(path: Path) -> list[tuple[Node, Node, Node]]:
    triples = []
    for line in path.read_text().splitlines():
        assert line.endswith(" ."), line
        s, p, o = parse_terms(line[:-2])
        triples.append((s, p, o))
    return triples


def read_diagnostics(path: Path) -> set[tuple[str, str, str, tuple[Node, ...]]]:
    out = set()
    for line in path.read_text().splitlines():
        code, severity, message, *terms = line.split("\t")
        out.add((code, severity, message, tuple(t for text in terms for t in parse_terms(text))))
    return out


def diagnostics(r: turbowl.Reasoner) -> set[tuple[str, str, str, tuple[Node, ...]]]:
    return {(d.code, d.severity.value, d.message, d.terms) for d in r.diagnostics()}


def check(r: turbowl.Reasoner, summary: turbowl.Summary, case: str) -> None:
    """The reasoner's results equal the Rust library's on ``case``."""
    d = generated()
    closure = r.triples()
    assert len(closure) == len(set(closure)), "a triple is returned twice"
    assert set(closure) == set(read_triples(d / f"{case}.closure.nt"))
    assert set(r.inferred()) == set(read_triples(d / f"{case}.inferred.nt"))
    assert diagnostics(r) == read_diagnostics(d / f"{case}.diagnostics.tsv")
    base, inferred, found = map(int, (d / f"{case}.summary").read_text().split())
    assert (summary.base, summary.inferred, summary.diagnostics) == (base, inferred, found)


def test_every_minimal_case_was_written():
    names = case_names()
    assert len(names) >= 40
    assert len(set(names)) == len(names)
    assert {"prp-dom", "cax-dw", "dt-not-type", "language-tag"} <= set(names)


def test_inserting_the_triples_gives_the_rust_closure(case):
    r = turbowl.Reasoner()
    r.insert(read_triples(generated() / f"{case}.input.nt"))
    check(r, r.reason(), case)


def test_inserting_a_graph_gives_the_rust_closure_as_a_graph(case):
    g = Graph()
    for t in read_triples(generated() / f"{case}.input.nt"):
        g.add(t)
    r = turbowl.Reasoner()
    r.insert(g)
    check(r, r.reason(), case)
    assert set(r.graph()) == set(read_triples(generated() / f"{case}.closure.nt"))


def test_two_incremental_batches_give_the_rust_closure(case):
    triples = read_triples(generated() / f"{case}.input.nt")
    half = len(triples) // 2
    r = turbowl.Reasoner()
    r.insert(triples[:half])
    r.reason()
    r.insert(triples[half:])
    summary = r.reason()
    check(r, summary, case)
    assert r.set_base(triples) is False
    assert r.reason().rounds == 0
