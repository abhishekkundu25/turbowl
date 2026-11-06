"""Snapshots: a reasoner saved to a file and loaded back."""

from __future__ import annotations

from pathlib import Path

import pytest
from rdflib import RDF, BNode, Namespace

import turbowl
from turbowl import FileError, Reasoner, SnapshotError, TurbowlError

EX = Namespace("http://example.org/")

TURTLE = """@prefix ex: <http://example.org/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:Dog rdfs:subClassOf ex:Animal ; owl:disjointWith ex:Cat .
ex:rex a ex:Dog , ex:Cat ; ex:age "old"^^xsd:integer .
ex:rex owl:sameAs ex:rexy .
_:b ex:owns ex:rex .
"""


def reasoned() -> Reasoner:
    r = Reasoner(switches={"literal-values": True})
    r.load_str(TURTLE)
    r.insert([(BNode("x"), EX.owns, EX.fido)])
    r.reason()
    return r


def test_a_snapshot_loads_back_the_same_reasoner(tmp_path: Path):
    r = reasoned()
    path = tmp_path / "state.snapshot"
    r.save_snapshot(path)
    again = Reasoner.from_snapshot(path)
    assert isinstance(again, Reasoner)
    assert again.triples() == r.triples()
    assert again.diagnostics() == r.diagnostics()
    # cax-dw for ex:rex and for ex:rexy (the same individual), and dt-not-type.
    assert len(again.diagnostics()) == 3
    assert again.switches == r.switches
    assert again.switches["literal-values"]
    summary = r.reason()
    assert again.reason() == summary
    assert summary.rounds == 0


def test_a_loaded_reasoner_goes_on_as_the_saved_one(tmp_path: Path):
    r = reasoned()
    path = tmp_path / "state.snapshot"
    r.save_snapshot(str(path))
    again = Reasoner.from_snapshot(str(path))
    for reasoner in (r, again):
        reasoner.insert([(EX.tom, RDF.type, EX.Dog)])
        reasoner.remove([(BNode("x"), EX.owns, EX.fido)])
        # The document again changes nothing: the snapshot knows it.
        reasoner.load_str(TURTLE)
    assert again.reason() == r.reason()
    assert again.triples() == r.triples()
    assert again.diagnostics() == r.diagnostics()


def test_saving_again_replaces_the_file_whole(tmp_path: Path):
    path = tmp_path / "state.snapshot"
    Reasoner().save_snapshot(path)
    small = path.stat().st_size
    reasoned().save_snapshot(path)
    assert path.stat().st_size > small
    assert [p.name for p in tmp_path.iterdir()] == ["state.snapshot"]
    assert Reasoner.from_snapshot(path).reason().base > 0


def test_a_file_that_is_not_a_snapshot_is_a_snapshot_error(tmp_path: Path):
    path = tmp_path / "data.ttl"
    path.write_text(TURTLE)
    with pytest.raises(SnapshotError) as caught:
        Reasoner.from_snapshot(path)
    e = caught.value
    assert isinstance(e, TurbowlError) and isinstance(e, ValueError)
    assert e.path == str(path)
    assert e.message == "not a TurbOWL snapshot"
    assert str(e) == f"{path}: not a TurbOWL snapshot"


def test_a_damaged_or_truncated_snapshot_is_refused(tmp_path: Path):
    path = tmp_path / "state.snapshot"
    reasoned().save_snapshot(path)
    data = path.read_bytes()
    damaged = bytearray(data)
    damaged[len(data) // 2] ^= 1
    path.write_bytes(bytes(damaged))
    with pytest.raises(SnapshotError, match="checksum"):
        Reasoner.from_snapshot(path)
    path.write_bytes(data[: len(data) // 2])
    with pytest.raises(SnapshotError, match="truncated"):
        Reasoner.from_snapshot(path)


def test_a_missing_or_unwritable_snapshot_is_a_file_error(tmp_path: Path):
    with pytest.raises(FileError):
        Reasoner.from_snapshot(tmp_path / "missing.snapshot")
    with pytest.raises(FileError):
        Reasoner().save_snapshot(tmp_path / "no" / "such" / "state.snapshot")


def test_snapshot_error_is_exported():
    assert turbowl.SnapshotError is SnapshotError
