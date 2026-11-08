"""Cancellation: ``turbowl.Cancel`` stops ``reason``, ``explain`` and
``explain_diagnostic`` from another thread, and Ctrl+C stops them too. A
cancelled run keeps no partial result."""

from __future__ import annotations

import _thread
import threading
import time

import pytest
from rdflib import Namespace, URIRef

import turbowl
from turbowl import Cancel, CancelledError, NotReasonedError, Reasoner, TurbowlError

EX = Namespace("http://example.org/")
TYPE = URIRef("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
SUB_CLASS_OF = URIRef("http://www.w3.org/2000/01/rdf-schema#subClassOf")
SOME_VALUES_FROM = URIRef("http://www.w3.org/2002/07/owl#someValuesFrom")
ON_PROPERTY = URIRef("http://www.w3.org/2002/07/owl#onProperty")
DISJOINT_WITH = URIRef("http://www.w3.org/2002/07/owl#disjointWith")


def chain(n: int) -> list:
    """A chain of existentials: one class more per round, n rounds."""
    triples = [(EX.x0, TYPE, EX.R0)]
    for i in range(n):
        j = i + 1
        triples += [
            (EX[f"x{j}"], EX.p, EX[f"x{i}"]),
            (EX[f"R{j}"], SOME_VALUES_FROM, EX[f"R{i}"]),
            (EX[f"R{j}"], ON_PROPERTY, EX.p),
        ]
    return triples


@pytest.fixture(scope="module")
def long_run() -> tuple[list, float]:
    """A chain whose run takes at least 0.3 s here, and that time."""
    n = 1000
    while True:
        triples = chain(n)
        r = Reasoner()
        r.insert(triples)
        start = time.perf_counter()
        r.reason()
        took = time.perf_counter() - start
        if took >= 0.3 or n >= 64000:
            return triples, took
        n *= 2


def state(r: Reasoner) -> tuple[set, list]:
    return set(r.triples()), sorted(map(repr, r.diagnostics()))


def test_cancel_is_a_handle_that_stays_tripped():
    c = Cancel()
    assert not c.cancelled
    c.cancel()
    assert c.cancelled
    c.cancel()
    assert c.cancelled
    assert repr(c) == "Cancel(cancelled=True)"
    assert issubclass(CancelledError, TurbowlError)
    assert issubclass(CancelledError, RuntimeError)
    assert turbowl.CancelledError is CancelledError


def test_a_tripped_handle_changes_nothing():
    r = Reasoner()
    r.insert([(EX.Dog, SUB_CLASS_OF, EX.Animal), (EX.rex, TYPE, EX.Dog)])
    tripped = Cancel()
    tripped.cancel()
    with pytest.raises(CancelledError):
        r.reason(cancel=tripped)
    r.reason()
    before = state(r)
    with pytest.raises(CancelledError):
        r.reason(cancel=tripped)
    assert state(r) == before
    assert r.explain(EX.rex, TYPE, EX.Animal) is not None
    with pytest.raises(CancelledError):
        r.explain(EX.rex, TYPE, EX.Animal, cancel=tripped)


def test_a_cancel_from_another_thread_stops_a_run(long_run):
    triples, took = long_run
    r = Reasoner()
    r.insert(triples)
    cancel = Cancel()
    timer = threading.Timer(took / 4, cancel.cancel)
    timer.start()
    start = time.perf_counter()
    with pytest.raises(CancelledError):
        r.reason(cancel=cancel)
    assert time.perf_counter() - start < took * 0.9
    timer.join()
    # No partial result, and nothing to explain.
    assert r.triples() == []
    assert r.diagnostics() == []
    with pytest.raises(NotReasonedError):
        r.explain(EX.x1, TYPE, EX.R1)
    # The next run is the run that was never cancelled.
    summary = r.reason()
    fresh = Reasoner()
    fresh.insert(triples)
    assert summary == fresh.reason()
    assert state(r) == state(fresh)


def test_ctrl_c_stops_a_run_and_raises_keyboard_interrupt(long_run):
    triples, took = long_run
    r = Reasoner()
    r.insert(triples)
    timer = threading.Timer(took / 4, _thread.interrupt_main)
    timer.start()
    start = time.perf_counter()
    with pytest.raises(KeyboardInterrupt):
        r.reason()
    assert time.perf_counter() - start < took * 0.9
    timer.join()
    assert r.triples() == []
    r.reason()
    assert len(r.triples()) > len(triples)


def test_an_explanation_cancelled_or_interrupted_is_answered_next_time():
    triples = [(EX.x, TYPE, EX.C0), (EX.C30, DISJOINT_WITH, EX.D), (EX.x, TYPE, EX.D)]
    triples += [(EX[f"C{i}"], SUB_CLASS_OF, EX[f"C{i + 1}"]) for i in range(30)]
    r = Reasoner(complete=True)
    r.insert(triples)
    r.reason()
    fresh = r.explain(EX.x, TYPE, EX.C30)
    assert fresh is not None
    (diagnostic,) = r.diagnostics()
    fresh_diagnostic = r.explain_diagnostic(diagnostic)
    assert fresh_diagnostic is not None

    tripped = Cancel()
    tripped.cancel()
    with pytest.raises(CancelledError):
        r.explain(EX.x, TYPE, EX.C30, cancel=tripped)
    with pytest.raises(CancelledError):
        r.explain_diagnostic(diagnostic, cancel=tripped)
    assert r._native.explain_cache_held()
    assert r.explain(EX.x, TYPE, EX.C30) == fresh
    assert r.explain_diagnostic(diagnostic, cancel=Cancel()) == fresh_diagnostic

    # Ctrl+C while proofs are asked, inside a call or between two: the
    # indexes are kept and the next answer is the same.
    timer = threading.Timer(0.05, _thread.interrupt_main)
    timer.start()
    with pytest.raises(KeyboardInterrupt):
        while True:
            assert r.explain(EX.x, TYPE, EX.C30) == fresh
    timer.join()
    assert r._native.explain_cache_held()
    assert r.explain(EX.x, TYPE, EX.C30) == fresh


def test_cancel_must_be_a_cancel():
    r = Reasoner()
    with pytest.raises(TypeError):
        r.reason(cancel=True)
