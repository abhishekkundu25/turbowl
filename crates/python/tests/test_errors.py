"""Errors are Python exceptions under ``turbowl.TurbowlError``."""

from __future__ import annotations

import errno
import pickle
from pathlib import Path

import pytest
from rdflib import BNode, Literal, Namespace, URIRef, Variable

import turbowl
from turbowl import FileError, ParseError, Reasoner, SnapshotError, TermError, TurbowlError, UnknownFormatError

EX = Namespace("http://example.org/")
BAD_TURTLE = "@prefix ex: <http://example.org/> .\nex:a ex:p ex:b .\nex:a ex:p .\n"


def test_every_error_is_a_turbowl_error_and_a_builtin_one():
    assert issubclass(ParseError, TurbowlError) and issubclass(ParseError, ValueError)
    assert issubclass(UnknownFormatError, TurbowlError) and issubclass(UnknownFormatError, ValueError)
    assert issubclass(TermError, TurbowlError) and issubclass(TermError, ValueError)
    assert issubclass(SnapshotError, TurbowlError) and issubclass(SnapshotError, ValueError)
    assert issubclass(FileError, TurbowlError) and issubclass(FileError, OSError)
    assert issubclass(TurbowlError, Exception)


# ParseError


def test_a_parse_error_in_text_carries_its_line_and_column():
    with pytest.raises(ParseError) as caught:
        Reasoner().load_str(BAD_TURTLE)
    e = caught.value
    assert (e.line, e.path) == (3, None)
    assert e.column > 0
    assert e.message
    assert str(e) == f"3:{e.column}: {e.message}"


def test_a_parse_error_in_a_file_carries_its_path(tmp_path: Path):
    path = tmp_path / "bad.ttl"
    path.write_text(BAD_TURTLE)
    with pytest.raises(ParseError) as caught:
        Reasoner().load_file(path)
    e = caught.value
    assert e.path == str(path)
    assert e.line == 3
    assert str(e).startswith(f"{path}: 3:")


@pytest.mark.parametrize(
    ("format", "text", "line"),
    [
        ("nt", "<http://example.org/a> <http://example.org/p> <http://example.org/b> .\n<a> <b> .\n", 2),
        # The tag is still open at the end of the input: line 4.
        ("xml", '<?xml version="1.0"?>\n<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">\n<oops\n', 4),
    ],
)
def test_a_parse_error_in_every_format_has_a_line(format: str, text: str, line: int):
    with pytest.raises(ParseError) as caught:
        Reasoner().load_str(text, format)
    assert caught.value.line == line


def test_an_invalid_base_iri_is_a_parse_error_at_no_line():
    with pytest.raises(ParseError) as caught:
        Reasoner().load_str("<a> <b> <c> .", base_iri="not an iri")
    e = caught.value
    assert (e.line, e.column) == (0, 0)
    assert str(e) == e.message


def test_a_document_with_an_error_adds_nothing():
    r = Reasoner()
    with pytest.raises(ParseError):
        r.load_str(BAD_TURTLE)
    assert r.reason().base == 0


def test_a_parse_error_pickles_with_its_fields():
    with pytest.raises(ParseError) as caught:
        Reasoner().load_str(BAD_TURTLE)
    e = pickle.loads(pickle.dumps(caught.value))
    assert isinstance(e, ParseError)
    assert (e.message, e.path, e.line, e.column) == (
        caught.value.message,
        None,
        3,
        caught.value.column,
    )


# FileError


def test_a_missing_file_is_a_file_error_and_an_os_error(tmp_path: Path):
    path = tmp_path / "missing.ttl"
    with pytest.raises(OSError) as caught:
        Reasoner().load_file(path)
    e = caught.value
    assert isinstance(e, FileError)
    assert e.errno == errno.ENOENT
    assert e.filename == str(path)
    assert e.strerror == "No such file or directory"
    assert str(e) == f"{path}: No such file or directory"


def test_an_unwritable_file_is_a_file_error(tmp_path: Path):
    r = Reasoner()
    r.reason()
    with pytest.raises(FileError) as caught:
        r.write(tmp_path / "no" / "such" / "dir.ttl")
    assert caught.value.errno == errno.ENOENT


# UnknownFormatError


def test_an_unknown_extension_is_an_unknown_format_error(tmp_path: Path):
    path = tmp_path / "data.json"
    path.write_text("{}")
    with pytest.raises(UnknownFormatError) as caught:
        Reasoner().load_file(path)
    assert caught.value.path == str(path)
    with pytest.raises(UnknownFormatError):
        Reasoner().write(tmp_path / "closure.json")


@pytest.mark.parametrize("name", ["json-ld", "trig", "", "turtle2"])
def test_an_unknown_format_name_is_an_unknown_format_error(name: str):
    r = Reasoner()
    with pytest.raises(UnknownFormatError) as caught:
        r.load_str("", name)
    assert caught.value.path is None
    with pytest.raises(UnknownFormatError):
        r.serialize(name)
    with pytest.raises(UnknownFormatError):
        r.write("closure.ttl", format=name)


# TermError and TypeError


@pytest.mark.parametrize(
    "triple",
    [
        (URIRef("relative"), EX.p, EX.o),
        (EX.s, URIRef("not an iri"), EX.o),
        (EX.s, EX.p, Literal("x", datatype=URIRef("relative"))),
        (BNode("has space"), EX.p, EX.o),
        (Literal("x"), EX.p, EX.o),
        (EX.s, BNode("p"), EX.o),
        (EX.s, Literal("p"), EX.o),
    ],
)
def test_a_term_rdf_does_not_allow_there_is_a_term_error(triple):
    r = Reasoner()
    for method in (r.insert, r.remove, r.set_base):
        with pytest.raises(TermError):
            method([triple])


@pytest.mark.parametrize(
    "item",
    [
        ("http://example.org/s", EX.p, EX.o),
        (EX.s, EX.p, Variable("o")),
        (EX.s, EX.p, None),
        (EX.s, EX.p),
        (EX.s, EX.p, EX.o, EX.g),
        None,
        "spo",
    ],
)
def test_an_item_that_is_not_a_triple_of_rdflib_terms_is_a_type_error(item):
    r = Reasoner()
    for method in (r.insert, r.remove, r.set_base):
        with pytest.raises(TypeError):
            method([item])


def test_triples_must_be_iterable():
    with pytest.raises(TypeError):
        Reasoner().insert(5)  # type: ignore[arg-type]


def test_a_batch_with_a_bad_triple_changes_nothing():
    r = Reasoner()
    with pytest.raises(TermError):
        r.insert([(EX.s, EX.p, EX.o), (Literal("x"), EX.p, EX.o)])
    assert r.reason().base == 0


def test_load_str_takes_text_only():
    with pytest.raises(TypeError):
        Reasoner().load_str(b"<a> <b> <c> .")  # type: ignore[arg-type]


def test_the_errors_are_exported():
    for name in ("TurbowlError", "ParseError", "FileError", "UnknownFormatError", "TermError", "SnapshotError"):
        assert name in turbowl.__all__
