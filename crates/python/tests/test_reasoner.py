"""Every method of ``turbowl.Reasoner``."""

from __future__ import annotations

import dataclasses
import textwrap
from pathlib import Path

import pytest
from rdflib import BNode, Graph, Literal, Namespace, URIRef

import turbowl
from turbowl import Diagnostic, Reasoner, Severity, Summary

EX = Namespace("http://example.org/")
TYPE = URIRef("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
SUB_CLASS_OF = URIRef("http://www.w3.org/2000/01/rdf-schema#subClassOf")
DISJOINT_WITH = URIRef("http://www.w3.org/2002/07/owl#disjointWith")
XSD_INTEGER = URIRef("http://www.w3.org/2001/XMLSchema#integer")

TURTLE = """@prefix ex: <http://example.org/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
ex:Dog rdfs:subClassOf ex:Animal .
ex:rex a ex:Dog .
"""
NTRIPLES = """<http://example.org/Dog> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/Animal> .
<http://example.org/rex> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Dog> .
"""
RDFXML = """<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#">
  <rdf:Description rdf:about="http://example.org/Dog">
    <rdfs:subClassOf rdf:resource="http://example.org/Animal"/>
  </rdf:Description>
  <rdf:Description rdf:about="http://example.org/rex">
    <rdf:type rdf:resource="http://example.org/Dog"/>
  </rdf:Description>
</rdf:RDF>
"""
DOCUMENTS = {"turtle": TURTLE, "nt": NTRIPLES, "n3": TURTLE, "xml": RDFXML}
EXTENSIONS = {"ttl": TURTLE, "nt": NTRIPLES, "n3": TURTLE, "rdf": RDFXML, "owl": RDFXML, "xml": RDFXML}

DOG = [(EX.Dog, SUB_CLASS_OF, EX.Animal), (EX.rex, TYPE, EX.Dog)]
REX_IS_AN_ANIMAL = (EX.rex, TYPE, EX.Animal)


def reasoned(triples) -> Reasoner:
    r = Reasoner()
    r.insert(triples)
    r.reason()
    return r


# new


def test_a_new_reasoner_is_empty():
    r = Reasoner()
    assert r.triples() == []
    assert r.inferred() == []
    assert r.diagnostics() == []
    assert r.reason().base == 0


# load_file


@pytest.mark.parametrize("extension", sorted(EXTENSIONS))
def test_load_file_reads_the_format_its_extension_names(tmp_path: Path, extension: str):
    path = tmp_path / f"dog.{extension}"
    path.write_text(EXTENSIONS[extension])
    r = Reasoner()
    r.load_file(path)
    assert r.reason().base == 2
    assert REX_IS_AN_ANIMAL in r.inferred()


def test_load_file_takes_a_str_path(tmp_path: Path):
    path = tmp_path / "dog.ttl"
    path.write_text(TURTLE)
    r = Reasoner()
    r.load_file(str(path))
    r.reason()
    assert REX_IS_AN_ANIMAL in r.triples()


def test_loading_the_same_file_again_changes_nothing(tmp_path: Path):
    path = tmp_path / "list.ttl"
    path.write_text("@prefix ex: <http://example.org/> . ex:a ex:p [ ex:q ex:b ] .")
    r = Reasoner()
    r.load_file(path)
    r.load_file(path)
    assert r.reason().base == 2


# load_str


@pytest.mark.parametrize("format", sorted(DOCUMENTS))
def test_load_str_reads_each_format(format: str):
    r = Reasoner()
    r.load_str(DOCUMENTS[format], format)
    assert r.reason().base == 2
    assert REX_IS_AN_ANIMAL in r.inferred()


@pytest.mark.parametrize(
    ("alias", "text"),
    [
        ("ttl", TURTLE),
        ("TURTLE", TURTLE),
        ("text/turtle", TURTLE),
        ("ntriples", NTRIPLES),
        ("n-triples", NTRIPLES),
        ("nt11", NTRIPLES),
        ("application/n-triples", NTRIPLES),
        ("text/n3", TURTLE),
        ("rdfxml", RDFXML),
        ("rdf/xml", RDFXML),
        ("application/rdf+xml", RDFXML),
    ],
)
def test_load_str_takes_the_other_format_names(alias: str, text: str):
    r = Reasoner()
    r.load_str(text, alias)
    assert r.reason().base == 2


def test_load_str_reads_turtle_by_default():
    r = Reasoner()
    r.load_str(TURTLE)
    assert r.reason().base == 2


def test_load_str_resolves_relative_iris_against_the_base_iri():
    r = Reasoner()
    r.load_str("<a> <p> <b> .", "turtle", base_iri="http://example.org/")
    r.reason()
    assert (EX.a, EX.p, EX.b) in r.triples()


def test_documents_keep_their_blank_nodes_apart():
    r = Reasoner()
    r.load_str("@prefix ex: <http://example.org/> . _:b ex:p ex:o .")
    r.load_str("@prefix ex: <http://example.org/> . _:b ex:q ex:o .")
    r.reason()
    subjects = {s for s, _, _ in r.triples() if isinstance(s, BNode)}
    assert subjects == {BNode("b_1"), BNode("b_2")}


# insert


def test_insert_takes_a_graph():
    g = Graph()
    for t in DOG:
        g.add(t)
    assert REX_IS_AN_ANIMAL in reasoned(g).inferred()


def test_insert_takes_any_iterable_of_triples():
    assert REX_IS_AN_ANIMAL in reasoned(iter(DOG)).inferred()
    assert REX_IS_AN_ANIMAL in reasoned(t for t in DOG).inferred()
    assert REX_IS_AN_ANIMAL in reasoned([list(t) for t in DOG]).inferred()
    assert REX_IS_AN_ANIMAL in reasoned(set(DOG)).inferred()


def test_insert_then_reason_runs_incrementally():
    r = Reasoner()
    r.insert(DOG[:1])
    r.reason()
    r.insert(DOG[1:])
    summary = r.reason()
    assert summary.base == 2
    assert summary.rounds > 0
    assert REX_IS_AN_ANIMAL in r.inferred()


def test_inserting_a_base_triple_again_changes_nothing():
    r = reasoned(DOG)
    r.insert(DOG)
    summary = r.reason()
    assert (summary.base, summary.rounds) == (2, 0)


# remove


def test_remove_takes_back_what_the_triple_implied():
    r = reasoned(DOG)
    r.remove([(EX.rex, TYPE, EX.Dog)])
    assert r.reason().base == 1
    assert REX_IS_AN_ANIMAL not in r.triples()


def test_remove_takes_a_graph():
    g = Graph()
    g.add(DOG[0])
    r = reasoned(DOG)
    r.remove(g)
    r.reason()
    assert set(r.triples()) >= {DOG[1]}
    assert DOG[0] not in r.triples()


def test_remove_ignores_triples_that_are_not_base_triples():
    r = reasoned(DOG)
    r.remove([REX_IS_AN_ANIMAL, (EX.never, EX.seen, EX.before)])
    summary = r.reason()
    assert summary.rounds == 0
    assert REX_IS_AN_ANIMAL in r.inferred()


def test_remove_reaches_inserted_blank_nodes():
    r = reasoned([(BNode("x"), TYPE, EX.Dog)])
    r.remove([(BNode("x"), TYPE, EX.Dog)])
    assert r.reason().base == 0


# set_base


def test_set_base_says_whether_a_triple_was_removed():
    r = Reasoner()
    assert r.set_base(DOG[:1]) is False
    r.reason()
    assert r.set_base(DOG) is False
    assert r.reason().rounds > 0
    assert REX_IS_AN_ANIMAL in r.inferred()
    assert r.set_base(DOG[1:]) is True
    r.reason()
    assert REX_IS_AN_ANIMAL not in r.triples()


def test_setting_an_unchanged_base_costs_no_rounds():
    g = Graph()
    for t in DOG:
        g.add(t)
    r = Reasoner()
    r.set_base(g)
    r.reason()
    assert r.set_base(g) is False
    assert r.reason().rounds == 0


# clear


def test_clear_makes_the_reasoner_as_new():
    r = reasoned(DOG)
    r.clear()
    assert r.triples() == []
    assert r.reason().base == 0
    assert r.triples() != []  # the facts of the rule set, inferred again
    assert REX_IS_AN_ANIMAL not in r.triples()


# reason


def test_reason_returns_a_summary():
    r = Reasoner()
    r.insert(DOG)
    summary = r.reason()
    assert isinstance(summary, Summary)
    assert summary.base == 2
    assert summary.inferred == len(r.inferred())
    assert summary.rounds > 0
    assert summary.diagnostics == 0
    assert r.reason().rounds == 0
    with pytest.raises(dataclasses.FrozenInstanceError):
        summary.base = 3  # type: ignore[misc]


# triples, inferred, graph


def test_triples_are_the_closure_as_rdflib_tuples():
    r = reasoned(DOG)
    closure = r.triples()
    assert isinstance(closure, list)
    assert all(isinstance(t, tuple) and len(t) == 3 for t in closure)
    assert set(DOG) | {REX_IS_AN_ANIMAL} <= set(closure)
    assert all(isinstance(term, (URIRef, BNode, Literal)) for t in closure for term in t)


def test_inferred_leaves_out_the_base():
    r = reasoned(DOG)
    inferred = set(r.inferred())
    assert REX_IS_AN_ANIMAL in inferred
    assert not inferred & set(DOG)
    assert inferred == set(r.triples()) - set(DOG)


def test_the_reads_show_the_last_run_until_the_next():
    r = reasoned(DOG[:1])
    r.insert(DOG[1:])
    assert REX_IS_AN_ANIMAL not in r.triples()
    r.reason()
    assert REX_IS_AN_ANIMAL in r.triples()


def test_graph_is_the_closure_or_the_inferred_triples():
    r = reasoned(DOG)
    g = r.graph()
    assert isinstance(g, Graph)
    assert set(g) == set(r.triples())
    assert set(r.graph(inferred_only=True)) == set(r.inferred())


# diagnostics


def test_diagnostics_carry_code_severity_message_and_terms():
    r = reasoned([(EX.A, DISJOINT_WITH, EX.B), (EX.i, TYPE, EX.A), (EX.i, TYPE, EX.B)])
    assert r.reason().diagnostics == 1
    [d] = r.diagnostics()
    assert isinstance(d, Diagnostic)
    assert d.code == "cax-dw"
    assert d.severity is Severity.ERROR
    assert d.severity == "error"
    assert "disjoint" in d.message
    assert d.terms == (EX.i, EX.A, EX.B)


def test_diagnostic_terms_are_rdflib_blank_nodes_and_literals():
    bad = Literal("abc", datatype=XSD_INTEGER, normalize=False)
    r = reasoned(
        [(EX.A, DISJOINT_WITH, EX.B), (BNode("i"), TYPE, EX.A), (BNode("i"), TYPE, EX.B), (EX.s, EX.p, bad)]
    )
    found = {d.code: d.terms for d in r.diagnostics()}
    assert found == {"cax-dw": (BNode("i"), EX.A, EX.B), "dt-not-type": (bad, XSD_INTEGER)}
    assert str(found["dt-not-type"][0]) == "abc"


def test_diagnostics_go_when_their_cause_is_removed():
    r = reasoned([(EX.A, DISJOINT_WITH, EX.B), (EX.i, TYPE, EX.A), (EX.i, TYPE, EX.B)])
    r.remove([(EX.i, TYPE, EX.B)])
    assert r.reason().diagnostics == 0
    assert r.diagnostics() == []


# serialize


@pytest.mark.parametrize("format", ["turtle", "nt", "n3", "xml"])
def test_serialize_writes_the_closure_rdflib_reads_back(format: str):
    r = reasoned(DOG)
    text = r.serialize(format)
    assert isinstance(text, str)
    assert set(Graph().parse(data=text, format=format)) == set(r.triples())


def test_serialize_writes_turtle_by_default_and_the_inferred_triples_alone():
    r = reasoned(DOG)
    assert set(Graph().parse(data=r.serialize(), format="turtle")) == set(r.triples())
    inferred = r.serialize("nt", inferred_only=True)
    assert set(Graph().parse(data=inferred, format="nt")) == set(r.inferred())


# write


@pytest.mark.parametrize(("extension", "format"), [("ttl", "turtle"), ("nt", "nt"), ("rdf", "xml")])
def test_write_takes_the_format_from_the_extension(tmp_path: Path, extension: str, format: str):
    r = reasoned(DOG)
    path = tmp_path / f"closure.{extension}"
    r.write(path)
    assert set(Graph().parse(path, format=format)) == set(r.triples())


def test_write_takes_a_format_and_the_inferred_triples_alone(tmp_path: Path):
    r = reasoned(DOG)
    path = tmp_path / "inferred.txt"
    r.write(str(path), format="nt", inferred_only=True)
    assert set(Graph().parse(path, format="nt")) == set(r.inferred())


def test_written_files_load_back(tmp_path: Path):
    r = reasoned(DOG)
    path = tmp_path / "closure.ttl"
    r.write(path)
    again = Reasoner()
    again.load_file(path)
    again.reason()
    assert set(again.triples()) == set(r.triples())


# the module


def test_the_module_has_a_version():
    assert isinstance(turbowl.__version__, str)
    assert turbowl.__version__.count(".") == 2


def test_the_example_in_the_module_doc_runs():
    after = turbowl.__doc__.split("::\n", 1)[1].splitlines()
    block = []
    for line in after:
        if line and not line.startswith("    "):
            break
        block.append(line)
    code = textwrap.dedent("\n".join(block))
    assert "r.reason()" in code
    exec(code, {})
