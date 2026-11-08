"""The exceptions TurbOWL raises (re-exported by :mod:`turbowl`).

Reasoning itself does not fail: problems in the data are diagnostics, not
exceptions. Exceptions come from reading and writing documents, from terms
that RDF does not allow, from asking for a proof of a closure that is out
of date, and from a cancelled call.
"""

from __future__ import annotations


class TurbowlError(Exception):
    """The base of every exception TurbOWL raises."""


class ParseError(TurbowlError, ValueError):
    """A document is not valid in its format.

    Attributes:
        message: The parser's message.
        path: The file, or ``None`` for text given to ``load_str``.
        line: The 1-based line of the error; 0 when the error is at no
            position in the text (an invalid base IRI).
        column: The 1-based column, in characters; 0 with line 0.
    """

    def __init__(self, message: str, path: str | None = None, line: int = 0, column: int = 0) -> None:
        super().__init__(message, path, line, column)
        self.message = message
        self.path = path
        self.line = line
        self.column = column

    def __str__(self) -> str:
        at = f"{self.path}: " if self.path is not None else ""
        if self.line == 0:
            return f"{at}{self.message}"
        return f"{at}{self.line}:{self.column}: {self.message}"


class UnknownFormatError(TurbowlError, ValueError):
    """A format name, or a file's extension, names no supported format.

    Attributes:
        path: The file whose extension was read, or ``None`` for a format name.
    """

    def __init__(self, message: str, path: str | None = None) -> None:
        super().__init__(message, path)
        self.path = path

    def __str__(self) -> str:
        return str(self.args[0])


class FileError(TurbowlError, OSError):
    """Reading or writing a file failed.

    An :class:`OSError`, with ``errno``, ``strerror`` and ``filename`` set as
    the operating system reported them (``errno`` may be ``None``).
    """

    def __init__(self, errno: int | None, strerror: str, filename: str | None = None) -> None:
        super().__init__(errno, strerror, filename)

    def __str__(self) -> str:
        return f"{self.filename}: {self.strerror}" if self.filename is not None else str(self.strerror)


class SnapshotError(TurbowlError, ValueError):
    """A snapshot is refused: it is not a snapshot, it is of another format
    version or another TurbOWL version, or it is truncated, damaged or
    malformed.

    Attributes:
        message: Why, as one sentence.
        path: The file.
    """

    def __init__(self, message: str, path: str | None = None) -> None:
        super().__init__(message, path)
        self.message = message
        self.path = path

    def __str__(self) -> str:
        return f"{self.path}: {self.message}" if self.path is not None else self.message


class TermError(TurbowlError, ValueError):
    """An rdflib term that RDF does not allow where it was given.

    For example a ``URIRef`` that is not an absolute IRI, a ``BNode`` label
    that N-Triples cannot write, an invalid language tag, a literal subject,
    or a predicate that is not a ``URIRef``.
    """


class NotReasonedError(TurbowlError, RuntimeError):
    """A proof was asked for while the reasoner has changed since its last
    run: there is no closure to explain until :meth:`Reasoner.reason` runs
    again.
    """


class CancelledError(TurbowlError, RuntimeError):
    """A call was stopped by its :class:`turbowl.Cancel`. A cancelled
    :meth:`Reasoner.reason` keeps no partial result: the reasoner is as
    loaded and not yet reasoned until the next run.
    """
