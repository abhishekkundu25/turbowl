"""``Reasoner.explain`` and ``Reasoner.explain_diagnostic``: proofs of the
last run's closure, as ``Proof`` objects of rdflib terms."""

from __future__ import annotations

import dataclasses
from pathlib import Path

import pytest
from rdflib import BNode, Literal, Namespace, URIRef

import turbowl
from turbowl import Diagnostic, NotReasonedError, Proof, Reasoner, Step, TermError, TurbowlError

EX = Namespace("http://example.org/")
TYPE = URIRef("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
SUB_CLASS_OF = URIRef("http://www.w3.org/2000/01/rdf-schema#subClassOf")
DISJOINT_WITH = URIRef("http://www.w3.org/2002/07/owl#disjointWith")

CHAIN = [
    (EX.Dog, SUB_CLASS_OF, EX.Animal),
    (EX.Animal, SUB_CLASS_OF, EX.Thing),
    (EX.rex, TYPE, EX.Dog),
]
REX_IS_A_THING = (EX.rex, TYPE, EX.Thing)
DISJOINT = [(EX.Dog, DISJOINT_WITH, EX.Cat), (EX.tom, TYPE, EX.Dog), (EX.tom, TYPE, EX.Cat)]


def reasoned(triples) -> Reasoner:
    r = Reasoner()
    r.insert(triples)
    r.reason()
    return r


def check_order(proof: Proof) -> None:
    """Every premise comes before the step that reads it."""
    for i, step in enumerate(proof.steps):
        assert all(p < i for p in step.premises), proof
        assert step.base == (step.rule is None)
        if step.base:
            assert step.premises == ()


# A triple's proof


def test_a_proof_has_its_steps_the_goal_last_and_its_leaves():
    r = reasoned(CHAIN)
    proof = r.explain(*REX_IS_A_THING)
    assert isinstance(proof, Proof)
    check_order(proof)
    assert all(isinstance(step, Step) for step in proof.steps)
    goal = proof.steps[-1]
    assert goal.statement == REX_IS_A_THING
    assert proof.goal == REX_IS_A_THING
    assert goal.rule == proof.rule == "cax-sco"
    assert goal.premises == proof.premises
    premises = {proof.steps[i].statement for i in goal.premises}
    assert premises == {(EX.Dog, SUB_CLASS_OF, EX.Thing), (EX.rex, TYPE, EX.Dog)}
    by_statement = {step.statement: step for step in proof.steps}
    assert by_statement[(EX.Dog, SUB_CLASS_OF, EX.Thing)].rule == "scm-sco"
    assert sorted(proof.leaves()) == sorted(CHAIN)
    assert proof.depth == 2


def test_a_proof_reads_as_an_indented_tree_the_goal_first():
    proof = reasoned(CHAIN).explain(*REX_IS_A_THING)
    lines = str(proof).splitlines()
    assert lines[0] == "<http://example.org/rex> rdf:type <http://example.org/Thing>  [cax-sco]"
    assert "  <http://example.org/rex> rdf:type <http://example.org/Dog>  [base]" in lines
    assert "    <http://example.org/Dog> rdfs:subClassOf <http://example.org/Animal>  [base]" in lines
    assert len(lines) == len(proof.steps)


def test_a_base_triple_is_its_own_proof():
    proof = reasoned(CHAIN).explain(EX.rex, TYPE, EX.Dog)
    assert proof is not None
    assert [(s.statement, s.rule, s.premises) for s in proof.steps] == [((EX.rex, TYPE, EX.Dog), None, ())]
    assert proof.rule is None and proof.depth == 0
    assert proof.leaves() == [(EX.rex, TYPE, EX.Dog)]


def test_proofs_are_values():
    r = reasoned(CHAIN)
    first, again = r.explain(*REX_IS_A_THING), r.explain(*REX_IS_A_THING)
    assert first == again
    with pytest.raises(dataclasses.FrozenInstanceError):
        first.depth = 0  # type: ignore[misc]


def test_a_triple_outside_the_closure_has_no_proof():
    r = reasoned(CHAIN)
    assert r.explain(EX.rex, TYPE, EX.Cat) is None
    assert r.explain(EX.unknown, TYPE, EX.Thing) is None
    assert r.explain(EX.rex, EX.unknownProperty, EX.Thing) is None
    assert r.explain(EX.rex, TYPE, Literal("x")) is None


def test_a_loaded_documents_blank_node_is_named_as_the_reads_return_it():
    r = Reasoner()
    r.load_str(
        "@prefix ex: <http://example.org/> .\n"
        "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n"
        "ex:Dog rdfs:subClassOf ex:Animal .\n_:b a ex:Dog .\n"
    )
    r.reason()
    (blank,) = [s for s, p, o in r.inferred() if o == EX.Animal and isinstance(s, BNode)]
    proof = r.explain(blank, TYPE, EX.Animal)
    assert proof is not None
    assert (blank, TYPE, EX.Dog) in proof.leaves()


def test_an_inserted_blank_node_keeps_its_label():
    r = reasoned([(EX.Dog, SUB_CLASS_OF, EX.Animal), (BNode("x"), TYPE, EX.Dog)])
    proof = r.explain(BNode("x"), TYPE, EX.Animal)
    assert proof is not None
    assert (BNode("x"), TYPE, EX.Dog) in proof.leaves()


def test_terms_are_checked_as_insert_checks_them():
    r = reasoned(CHAIN)
    with pytest.raises(TermError):
        r.explain(Literal("x"), TYPE, EX.Thing)
    with pytest.raises(TermError):
        r.explain(EX.rex, BNode("p"), EX.Thing)
    with pytest.raises(TermError):
        r.explain(URIRef("not an iri"), TYPE, EX.Thing)
    with pytest.raises(TypeError):
        r.explain("http://example.org/rex", TYPE, EX.Thing)  # type: ignore[arg-type]


def test_a_reasoner_from_a_snapshot_explains(tmp_path: Path):
    path = tmp_path / "state.snapshot"
    reasoned(CHAIN).save_snapshot(path)
    proof = Reasoner.from_snapshot(path).explain(*REX_IS_A_THING)
    assert proof is not None and proof.depth == 2


# A diagnostic's proof


def test_a_diagnostic_has_a_proof_whose_goal_is_the_diagnostic():
    r = reasoned(DISJOINT)
    (d,) = r.diagnostics()
    proof = r.explain_diagnostic(d)
    assert proof is not None
    check_order(proof)
    assert proof.goal == d
    assert isinstance(proof.goal, Diagnostic)
    assert proof.rule == "cax-dw"
    assert sorted(proof.steps[i].statement for i in proof.premises) == sorted(DISJOINT)
    assert sorted(proof.leaves()) == sorted(DISJOINT)
    assert proof.depth == 1
    assert str(proof).splitlines()[0] == f"{d.message}  [cax-dw]"


def test_a_diagnostic_from_inferred_triples():
    r = reasoned([(EX.Puppy, SUB_CLASS_OF, EX.Dog), DISJOINT[0], (EX.tom, TYPE, EX.Puppy), (EX.tom, TYPE, EX.Cat)])
    (d,) = r.diagnostics()
    proof = r.explain_diagnostic(d)
    assert proof is not None
    assert proof.depth == 2
    assert (EX.tom, TYPE, EX.Dog) in {proof.steps[i].statement for i in proof.premises}


def test_a_diagnostic_not_reported_has_no_proof():
    r = reasoned(DISJOINT)
    (d,) = r.diagnostics()
    assert r.explain_diagnostic(dataclasses.replace(d, terms=(EX.jerry, EX.Dog, EX.Cat))) is None
    assert r.explain_diagnostic(dataclasses.replace(d, code="prp-irp")) is None
    assert reasoned(CHAIN).explain_diagnostic(d) is None


# The last run's closure, and the kept indexes


def test_a_change_since_the_last_run_raises_not_reasoned():
    r = Reasoner()
    r.insert(CHAIN)
    with pytest.raises(NotReasonedError):
        r.explain(*REX_IS_A_THING)
    r.reason()
    (d,) = reasoned(DISJOINT).diagnostics()
    r.insert(DISJOINT)
    with pytest.raises(NotReasonedError) as caught:
        r.explain_diagnostic(d)
    assert isinstance(caught.value, TurbowlError) and isinstance(caught.value, RuntimeError)
    r.reason()
    assert r.explain_diagnostic(d) is not None


def test_the_cache_is_kept_between_proofs_and_dropped_after_insert():
    r = reasoned(CHAIN)
    assert not r._native.explain_cache_held()
    first = r.explain(*REX_IS_A_THING)
    assert r._native.explain_cache_held()
    assert r.explain(*REX_IS_A_THING) == first
    assert r._native.explain_cache_held()

    # A shorter way to the same triple: the next proof is of the new closure.
    r.insert([(EX.Dog, SUB_CLASS_OF, EX.Thing)])
    assert not r._native.explain_cache_held()
    r.reason()
    proof = r.explain(*REX_IS_A_THING)
    assert proof is not None and proof.depth == 1
    assert sorted(proof.leaves()) == sorted([(EX.Dog, SUB_CLASS_OF, EX.Thing), (EX.rex, TYPE, EX.Dog)])


@pytest.mark.parametrize(
    "change",
    [
        lambda r, tmp: r.load_str("<http://example.org/a> <http://example.org/p> <http://example.org/b> .", "nt"),
        lambda r, tmp: r.load_file(tmp),
        lambda r, tmp: r.insert([(EX.a, EX.p, EX.b)]),
        lambda r, tmp: r.remove([(EX.rex, TYPE, EX.Dog)]),
        lambda r, tmp: r.set_base(CHAIN),
        lambda r, tmp: r.clear(),
        lambda r, tmp: r.reason(),
    ],
    ids=["load_str", "load_file", "insert", "remove", "set_base", "clear", "reason"],
)
def test_every_change_drops_the_cache(change, tmp_path: Path):
    document = tmp_path / "more.nt"
    document.write_text("<http://example.org/c> <http://example.org/p> <http://example.org/d> .\n")
    r = reasoned(CHAIN)
    r.explain(*REX_IS_A_THING)
    assert r._native.explain_cache_held()
    change(r, document)
    assert not r._native.explain_cache_held()


def test_the_proof_types_are_exported():
    for name in ("Proof", "Step", "Statement", "NotReasonedError"):
        assert name in turbowl.__all__
