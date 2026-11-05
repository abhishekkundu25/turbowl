"""Round trips of rdflib terms: language tags, datatypes and blank nodes come
back as they went in (with the exceptions the module doc lists)."""

from __future__ import annotations

import pytest
from rdflib import BNode, Graph, Literal, Namespace, URIRef

from turbowl import Reasoner

EX = Namespace("http://example.org/")
XSD = "http://www.w3.org/2001/XMLSchema#"
LABEL = URIRef("http://www.w3.org/2000/01/rdf-schema#label")
SUB_PROPERTY_OF = URIRef("http://www.w3.org/2000/01/rdf-schema#subPropertyOf")


def back(term) -> object:
    """``term`` as the reasoner gives it back, as an object."""
    r = Reasoner()
    r.insert([(EX.s, EX.p, term)])
    r.reason()
    [o] = [o for s, _, o in r.triples() if s == EX.s]
    return o


# language tags


@pytest.mark.parametrize("tag", ["fr", "en", "de-ch", "zh-hant-tw"])
def test_a_language_tag_comes_back(tag: str):
    o = back(Literal("chat", lang=tag))
    assert o == Literal("chat", lang=tag)
    assert isinstance(o, Literal)
    assert o.language == tag
    assert o.datatype is None


def test_a_language_tag_comes_back_lowercase_and_equal():
    o = back(Literal("colour", lang="en-GB"))
    assert o.language == "en-gb"
    assert o == Literal("colour", lang="en-GB")


def test_a_language_tag_survives_inference():
    r = Reasoner()
    r.insert([(EX.name, SUB_PROPERTY_OF, LABEL), (EX.cat, EX.name, Literal("chat", lang="fr"))])
    r.reason()
    assert (EX.cat, LABEL, Literal("chat", lang="fr")) in r.inferred()


def test_a_language_tag_of_a_document_comes_back():
    r = Reasoner()
    r.load_str('@prefix ex: <http://example.org/> . ex:cat ex:name "chat"@fr .')
    r.reason()
    assert (EX.cat, EX.name, Literal("chat", lang="fr")) in r.triples()


# datatypes


@pytest.mark.parametrize(
    ("lexical", "datatype"),
    [
        ("42", "integer"),
        ("01", "integer"),
        ("abc", "integer"),
        ("1.50", "decimal"),
        ("1.5E0", "double"),
        ("true", "boolean"),
        ("1", "boolean"),
        ("2026-11-05T10:00:00Z", "dateTime"),
        ("2026-11-05", "date"),
        ("PT1H", "duration"),
        ("2147483648", "int"),
    ],
)
def test_a_typed_literal_comes_back_with_its_lexical_form(lexical: str, datatype: str):
    literal = Literal(lexical, datatype=URIRef(XSD + datatype), normalize=False)
    o = back(literal)
    assert o == literal
    assert str(o) == lexical
    assert o.datatype == URIRef(XSD + datatype)


def test_a_literal_of_a_datatype_of_our_own_comes_back():
    literal = Literal("3 apples", datatype=EX.amount)
    o = back(literal)
    assert o == literal
    assert o.datatype == EX.amount


def test_literals_made_from_python_values_come_back():
    for value in [5, -7, True, 1.5, "plain"]:
        assert back(Literal(value)) == Literal(value)


def test_a_simple_literal_comes_back_without_a_datatype():
    o = back(Literal("x"))
    assert o == Literal("x")
    assert o.datatype is None
    assert o.language is None


def test_a_literal_of_xsd_string_comes_back_as_a_simple_literal():
    o = back(Literal("x", datatype=URIRef(XSD + "string")))
    assert o == Literal("x")
    assert o.datatype is None


@pytest.mark.parametrize("text", ['say "hi"', "back\\slash", "line\nbreak\ttab\r", "été 日本 🦉", "", "\x00\x7f"])
def test_any_lexical_form_comes_back(text: str):
    assert back(Literal(text)) == Literal(text)
    assert back(Literal(text, lang="fr")) == Literal(text, lang="fr")


@pytest.mark.parametrize(
    "iri",
    ["http://example.org/a#b", "urn:isbn:0451450523", "http://example.org/é?q=1&r=%20", "tag:example.org,2026:x"],
)
def test_an_iri_comes_back(iri: str):
    assert back(URIRef(iri)) == URIRef(iri)


# blank nodes


@pytest.mark.parametrize("label", ["x", "b0", "node-1", "N0123abcdef", "a.b"])
def test_an_inserted_blank_node_keeps_its_label(label: str):
    r = Reasoner()
    r.insert([(BNode(label), EX.p, BNode(label + "o"))])
    r.reason()
    assert (BNode(label), EX.p, BNode(label + "o")) in r.triples()


def test_a_generated_blank_node_comes_back():
    b = BNode()
    assert back(b) == b
    r = Reasoner()
    r.insert([(b, EX.p, EX.o)])
    r.reason()
    assert (b, EX.p, EX.o) in r.triples()


def test_a_label_like_a_document_nodes_gets_a_suffix():
    # Document blank nodes are written `<label>_<n>`; an inserted label of
    # that shape gets `_0`, so the two never meet.
    r = Reasoner()
    r.load_str("@prefix ex: <http://example.org/> . _:b ex:p ex:o .")
    r.insert([(BNode("b_1"), EX.p, EX.o)])
    r.reason()
    subjects = {s for s, _, _ in r.triples() if isinstance(s, BNode)}
    assert subjects == {BNode("b_1"), BNode("b_1_0")}


def test_the_same_label_is_the_same_node_in_every_call():
    r = Reasoner()
    r.insert([(BNode("x"), EX.p, EX.a)])
    r.insert([(BNode("x"), EX.p, EX.b)])
    r.reason()
    assert {o for s, _, o in r.triples() if s == BNode("x")} == {EX.a, EX.b}
    r.remove([(BNode("x"), EX.p, EX.a)])
    r.reason()
    assert {o for s, _, o in r.triples() if s == BNode("x")} == {EX.b}


def test_blank_nodes_survive_inference():
    r = Reasoner()
    r.insert([(EX.p, SUB_PROPERTY_OF, EX.q), (BNode("s"), EX.p, BNode("o"))])
    r.reason()
    assert (BNode("s"), EX.q, BNode("o")) in r.inferred()


# whole graphs


def test_a_graph_rdflib_parsed_comes_back_whole():
    g = Graph()
    g.parse(
        data="""@prefix ex: <http://example.org/> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:a ex:name "chat"@fr, "cat"@en-GB, "plain", "typed"^^xsd:string ;
             ex:count 7, "007"^^xsd:integer, 1.5, true, "x"^^ex:dt ;
             ex:link [ ex:p ex:b ; ex:q ( 1 2 ) ] .
        """,
        format="turtle",
    )
    r = Reasoner()
    r.insert(g)
    r.reason()
    closure = Graph()
    closure += r.triples()
    # Every triple comes back equal, xsd:string as a simple literal.
    expected = {
        (s, p, Literal(str(o)) if isinstance(o, Literal) and o.datatype == URIRef(XSD + "string") else o)
        for s, p, o in g
    }
    assert expected <= set(closure)
    assert len(r.triples()) - len(r.inferred()) == len(g)
