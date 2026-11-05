"""The switches of ``turbowl.Reasoner``: given by name, read back, and
turning their rules on and off."""

from __future__ import annotations

from types import MappingProxyType

import pytest
from rdflib import Namespace, URIRef

import turbowl
from turbowl import Reasoner

EX = Namespace("http://example.org/")
TYPE = URIRef("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
SAME_AS = URIRef("http://www.w3.org/2002/07/owl#sameAs")
DISJOINT_WITH = URIRef("http://www.w3.org/2002/07/owl#disjointWith")
CHAIN = URIRef("http://www.w3.org/2002/07/owl#propertyChainAxiom")
FIRST = URIRef("http://www.w3.org/1999/02/22-rdf-syntax-ns#first")
REST = URIRef("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest")
NIL = URIRef("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil")

NAMES = ["equality", "property-chains", "checks", "identity", "schema", "trivial", "literal-values"]
DEFAULTS = {
    "equality": True,
    "property-chains": True,
    "checks": True,
    "identity": True,
    "schema": True,
    "trivial": False,
    "literal-values": False,
}

# `ex:b ex:p ex:c` takes equality; `ex:x ex:grandparent ex:z` property
# chains; the diagnostic, checks.
DATA = [
    (EX.a, SAME_AS, EX.b),
    (EX.a, EX.p, EX.c),
    (EX.grandparent, CHAIN, EX.chain),
    (EX.chain, FIRST, EX.parent),
    (EX.chain, REST, EX.rest),
    (EX.rest, FIRST, EX.parent),
    (EX.rest, REST, NIL),
    (EX.x, EX.parent, EX.y),
    (EX.y, EX.parent, EX.z),
    (EX.Dog, DISJOINT_WITH, EX.Cat),
    (EX.tom, TYPE, EX.Dog),
    (EX.tom, TYPE, EX.Cat),
]
BY_EQUALITY = (EX.b, EX.p, EX.c)
BY_CHAIN = (EX.x, EX.grandparent, EX.z)


def reasoned(r: Reasoner) -> Reasoner:
    r.insert(DATA)
    r.reason()
    return r


def test_a_new_reasoner_has_the_default_switches_in_order():
    switches = Reasoner().switches
    assert switches == DEFAULTS
    assert list(switches) == NAMES


def test_complete_turns_every_switch_on():
    assert Reasoner(complete=True).switches == dict.fromkeys(NAMES, True)


def test_switches_change_only_the_switches_they_name():
    r = Reasoner(switches={"identity": False, "trivial": True})
    assert r.switches == {**DEFAULTS, "identity": False, "trivial": True}


def test_switches_apply_after_complete():
    r = Reasoner(complete=True, switches={"literal-values": False})
    assert r.switches == {**dict.fromkeys(NAMES, True), "literal-values": False}


def test_switches_takes_any_mapping():
    r = Reasoner(switches=MappingProxyType({"checks": False}))
    assert r.switches["checks"] is False


def test_the_switches_read_are_a_copy():
    r = Reasoner()
    r.switches["equality"] = False
    assert r.switches["equality"] is True


def test_clear_keeps_the_switches():
    r = reasoned(Reasoner(switches={"equality": False}))
    r.clear()
    assert r.switches == {**DEFAULTS, "equality": False}


@pytest.mark.parametrize("name", ["sameas", "Equality", "property_chains", "literal_values", ""])
def test_an_unknown_name_raises_value_error(name: str):
    with pytest.raises(ValueError, match=f"unknown switch '{name}' \\(the switches: equality, property-chains"):
        Reasoner(switches={name: False})


@pytest.mark.parametrize("value", [0, 1, "off", None])
def test_a_value_that_is_not_a_bool_raises_type_error(value):
    with pytest.raises(TypeError, match="switch 'checks' takes True or False"):
        Reasoner(switches={"checks": value})


@pytest.mark.parametrize("switches", [["checks"], [("checks", False)], "checks"])
def test_switches_that_are_not_a_mapping_raise_type_error(switches):
    with pytest.raises(TypeError, match="switches must map switch names to True or False"):
        Reasoner(switches=switches)


def test_a_name_that_is_not_a_str_raises_type_error():
    with pytest.raises(TypeError, match="a switch name is a str"):
        Reasoner(switches={1: False})


def test_switches_and_complete_are_keyword_only():
    with pytest.raises(TypeError):
        Reasoner({"checks": False})  # type: ignore[misc]


def test_complete_must_be_a_bool():
    with pytest.raises(TypeError):
        Reasoner(complete=1)  # type: ignore[arg-type]


def test_the_defaults_reason_with_equality_chains_and_checks():
    r = reasoned(Reasoner())
    assert BY_EQUALITY in r.triples()
    assert BY_CHAIN in r.triples()
    assert [d.code for d in r.diagnostics()] == ["cax-dw"]


@pytest.mark.parametrize(
    ("name", "gone"),
    [("equality", BY_EQUALITY), ("property-chains", BY_CHAIN)],
)
def test_a_switch_off_takes_its_inferences_away(name: str, gone):
    r = reasoned(Reasoner(switches={name: False}))
    assert gone not in r.triples()
    others = {BY_EQUALITY, BY_CHAIN} - {gone}
    assert others <= set(r.triples())


def test_checks_off_reports_no_diagnostic_and_keeps_the_closure():
    on = reasoned(Reasoner())
    off = reasoned(Reasoner(switches={"checks": False}))
    assert off.diagnostics() == []
    assert set(off.triples()) == set(on.triples())


def test_the_module_doc_lists_every_switch_with_its_default():
    doc = turbowl.__doc__
    for name, on in DEFAULTS.items():
        assert f"``{name}`` ({'on' if on else 'off'})" in doc
