//! Parsing a large document on every core (ticket 18), and a document read
//! in windows.
//!
//! The document is cut at line starts into pieces. Each piece is parsed on a
//! thread of its own into a term table of its own (a [`Dictionary`]), then the
//! pieces are merged in order on the caller's thread: each piece's terms are
//! interned into the dictionary in the order the piece first met them, and its
//! triples, renumbered, go to the sink. A term's first piece is its first
//! occurrence in the document, so the dictionary gets the same terms at the
//! same ids as the parse on one thread, and the sink the same triples in the
//! same order.
//!
//! A piece's parse is speculative: it starts at its cut as if the document
//! were between statements there (between lines for N-Triples), with the
//! prefixes and base the first piece ended with. The merge keeps it only when
//! that was so:
//!
//! - every parse reads past its piece's end to the first line start where its
//!   grammar is between statements ([`Stop::At`]); the next piece is kept only
//!   when it starts there;
//! - a Turtle piece is kept only when the prefixes it read without declaring,
//!   and the base if it read one, are the same in the true state as in the
//!   one it started with.
//!
//! A piece that is not kept is parsed again on the caller's thread, from where
//! the previous one stopped and in the true state. So the result never
//! depends on the cuts: N-Triples cuts are always right; Turtle cuts are
//! placed where a statement seems to start, and most are right.
//!
//! An error is a piece's: the merge hands over the triples before it and
//! returns it, with the line and column the parse on one thread gives, since a
//! kept piece reads exactly as that parse would. When the dictionary could
//! fill up, the merge parses the rest on the caller's thread into it, which
//! fails where the parse on one thread would.
//!
//! # Windows
//!
//! A text can be a window of the document: it starts at a line start between
//! statements, in a [`State`] (prefixes and base) the caller carries over,
//! and, when it is open, the document goes on past its end. An open window's
//! pieces cover it up to a line start about [`SLACK`] bytes from its end, and
//! their lexers stop short of the end ([`Fail::Short`]) rather than read it
//! as the document's end. Its pieces are always speculative, even on one
//! thread, since only a piece's own table can be dropped: the merge takes
//! them up to the first that ran short, and gives where it stopped
//! ([`Outcome::More`]), a line start between statements, with the state
//! there. The caller reads on from there; nothing past it reached the sink
//! or the dictionary. A closed window (the document's last) is parsed as a
//! whole document is.
//!
//! The parse on one thread is `ntriples::parse` and `turtle::parse` from the
//! start; `io/cross_check.rs` compares the two on every test document,
//! valid and malformed, cut at every few lines and read in windows of a few
//! bytes.

use super::iri::Base;
use super::lexer::{Lexer, Mode};
use super::terms::{Fail, Out, Stop, error};
use super::turtle::Prefixes;
use super::{ntriples, turtle};
use crate::engine::Workers;
use crate::term::{Dictionary, DocId, MAX_TERMS, Term, TermId, vocab};
use crate::{Error, Format};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

/// The smallest document parsed in parallel, in bytes (384 KiB). Below it
/// the pieces' merge and the thread pool cost more than they save. Measured
/// by `scaling::io::timing_parallel_parse_threshold` on 2026-11-06, three
/// times, on the 12-core machine the tests run on, shared with other jobs
/// (load 4 to 5): at 100 KB all cores were slower (0.63 to 0.76 times as
/// fast), at 200 KB even (0.97 to 1.15), and from 406 KB faster at every
/// size (1.45 to 1.65 times at 406 KB, 2.1 to 2.8 times at 3 to 7 MB), for
/// N-Triples and Turtle alike.
pub(crate) const PARALLEL_FROM: usize = 384 << 10;

/// The most bytes the first piece gets: it is parsed before the others, which
/// start with its prefixes and base (a document's directives come first).
const HEAD: usize = 1 << 20;

/// The fewest bytes a piece gets in a parallel parse.
const MIN_PIECE: usize = 1 << 16;

/// How far past its place a Turtle cut looks for a line that seems to start
/// a statement, before it takes any line start.
const LOOK: usize = 1 << 16;

/// What an open window leaves to the next, about: its pieces end at a line
/// start this far from its end (or half the window, if that is less), so
/// that the statement there most likely ends inside the window. The rest is
/// read again with the next window.
pub(super) const SLACK: usize = 1 << 16;

/// The size of the pieces a document of `len` bytes is parsed in, or `None`
/// to parse it on one thread: `from` is the threshold, [`PARALLEL_FROM`]
/// but in the test that measures it.
pub(super) fn piece_size(workers: &Workers, len: usize, from: usize) -> Option<usize> {
    // `engage(0)` holds only when every round is to run in parallel (the
    // tests' setting): then even a small document is cut, in about 32
    // pieces. `engage(usize::MAX)` holds when there is more than one thread;
    // it is asked only past the threshold, so a small document never starts
    // the thread pool.
    if workers.engage(0) {
        return Some(len.div_ceil(32).max(1));
    }
    if len >= from && workers.engage(usize::MAX) {
        return Some(len.div_ceil(4 * workers.threads()).max(MIN_PIECE));
    }
    None
}

/// Where a parse stands at a line start between statements: the prefixes
/// declared before it, and the base.
#[derive(Debug, Clone, Default)]
pub(super) struct State {
    pub(super) prefixes: Prefixes<'static>,
    pub(super) base: Option<Base>,
}

/// How the parse of a text ended.
#[derive(Debug)]
pub(super) enum Outcome {
    /// At the end of the document; the triples handed to the sink.
    Done(usize),
    /// An open window, parsed up to `at` (0 when nothing could be parsed):
    /// a line start between statements, where the parse stands in `state`.
    /// The triples before `at` went to the sink. When `rest`, the dictionary
    /// may fill up: the rest of the document must be parsed as one text, so
    /// that it fails where the parse on one thread would.
    More {
        at: usize,
        triples: usize,
        state: State,
        rest: bool,
    },
}

/// One piece of the document: from a line start to another (or the end).
#[derive(Debug, Clone, Copy)]
struct Piece {
    from: usize,
    until: usize,
}

/// A piece's parse, into a term table of its own.
struct Parsed<'a> {
    terms: Dictionary,
    triples: Vec<[TermId; 3]>,
    result: Result<Stop, Fail>,
    /// The prefixes as the parse left them, with what it read.
    prefixes: Prefixes<'a>,
    /// The base as the parse left it, and whether it read the base.
    base: Option<Base>,
    base_read: bool,
}

/// Where the parse of a piece starts: its prefixes, its base, and a term
/// table holding the vocabulary (at the dictionary's ids).
struct Start<'s, 'a> {
    text: &'a [u8],
    /// The document goes on past `text`.
    open: bool,
    format: Format,
    document: DocId,
    vocabulary: &'s Dictionary,
}

impl<'a> Start<'_, 'a> {
    fn parse(&self, from: usize, until: usize, prefixes: Prefixes<'a>, base: Option<Base>) -> Parsed<'a> {
        let text = self.text;
        // Cuts are line starts, so a UTF-8 document's pieces are UTF-8.
        let utf8 = std::str::from_utf8(&text[from..until]).ok();
        let mode = match self.format {
            Format::NTriples => Mode::NTriples,
            _ => Mode::Turtle,
        };
        let mut lexer = Lexer::at(text, from, utf8, mode, base, self.open);
        let mut terms = self.vocabulary.clone();
        let mut triples = Vec::new();
        let mut prefixes = prefixes;
        let mut out = Out::new(&mut terms, self.document, |t| triples.push(t), None);
        let result = match self.format {
            Format::NTriples => ntriples::parse(&mut lexer, until, &mut out),
            _ => turtle::parse(&mut lexer, until, &mut prefixes, &mut out),
        };
        Parsed {
            terms,
            triples,
            result,
            prefixes,
            base: lexer.base,
            base_read: lexer.base_read,
        }
    }
}

/// Parses N-Triples or Turtle: `text` runs from a line start between
/// statements, where the parse stands in `state` (the document's start when
/// `head`), to the end of the document, or, when `open`, to where the
/// document has been read so far. A document of [`piece_size`] is parsed
/// in pieces on `workers` (`from` is the threshold), others on the caller's
/// thread, but an open window is always parsed in pieces. The same result as
/// the parse on one thread, error included; an error's line and column count
/// from the start of `text`.
#[allow(clippy::too_many_arguments)]
pub(super) fn parse(
    dict: &mut Dictionary,
    text: &[u8],
    format: Format,
    state: &State,
    head: bool,
    open: bool,
    document: DocId,
    path: Option<&Path>,
    workers: &Workers,
    from: usize,
    sink: &mut impl FnMut([TermId; 3]),
) -> Result<Outcome, Error> {
    let end = if open {
        match span(text, format) {
            Some(end) => end,
            None => return Ok(nothing(state)),
        }
    } else {
        text.len()
    };
    let size = match piece_size(workers, end, from) {
        Some(size) => size,
        None if open => end,
        None => return whole(dict, text, format, state, document, path, sink).map(Outcome::Done),
    };
    let pieces = cuts(&text[..end], format, size);
    if pieces.len() == 1 && !open {
        return whole(dict, text, format, state, document, path, sink).map(Outcome::Done);
    }
    let vocabulary = Dictionary::new();
    let start = Start {
        text,
        open,
        format,
        document,
        vocabulary: &vocabulary,
    };
    // At the document's start, the first piece is parsed first: the others
    // start with the prefixes and base it ends with. Later in the document
    // they start with the state the window starts in.
    let (head, guessed) = if head {
        let head = start.parse(
            pieces[0].from,
            pieces[0].until,
            state.prefixes.start(),
            state.base.clone(),
        );
        let guessed = (head.prefixes.start(), head.base.clone());
        (Some(head), guessed)
    } else {
        (None, (state.prefixes.start(), state.base.clone()))
    };
    let first = usize::from(head.is_some());
    let mut merge = Merge {
        start: &start,
        guessed: &guessed,
        at: 0,
        prefixes: state.prefixes.start(),
        base: state.base.clone(),
        triples: 0,
        map: Vec::new(),
        dict,
        path,
        sink,
    };
    if let Some(head) = head
        && let Some(done) = merge.take(pieces[0], Some(head))
    {
        return done;
    }
    let (prefixes, guessed_base) = &guessed;
    let speculate = |piece: Piece| start.parse(piece.from, piece.until, prefixes.start(), guessed_base.clone());
    let threads = workers.threads();
    if threads == 1 {
        for &piece in &pieces[first..] {
            if let Some(done) = merge.take(piece, Some(speculate(piece))) {
                return done;
            }
        }
        return Ok(merge.end());
    }
    let next = AtomicUsize::new(first);
    let (sent, received) = mpsc::channel();
    std::thread::scope(|scope| {
        let (next, speculate, pieces) = (&next, &speculate, &pieces);
        scope.spawn(move || {
            // One task a thread, each taking the next piece until none is
            // left or the merge is done (it drops the receiver).
            let mut tasks = vec![(); threads];
            workers.for_each(&mut tasks, |()| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= pieces.len() || sent.send((i, speculate(pieces[i]))).is_err() {
                        break;
                    }
                }
            });
        });
        let mut ready: Vec<Option<Parsed<'_>>> = (0..pieces.len()).map(|_| None).collect();
        for (i, &piece) in pieces.iter().enumerate().skip(first) {
            while ready[i].is_none() {
                let (j, parsed) = received.recv().expect("every piece is parsed before the workers stop");
                ready[j] = Some(parsed);
            }
            if let Some(done) = merge.take(piece, ready[i].take()) {
                // Stops the workers after the pieces they are parsing.
                next.store(pieces.len(), Ordering::Relaxed);
                drop(received);
                return done;
            }
        }
        Ok(merge.end())
    })
}

/// An open window of which nothing could be parsed.
fn nothing(state: &State) -> Outcome {
    Outcome::More {
        at: 0,
        triples: 0,
        state: state.clone(),
        rest: false,
    }
}

/// Where an open window's pieces end: a line start [`SLACK`] bytes (or half
/// the window) or more before its end; Turtle's as [`cut`] places it. `None`
/// when there is none.
fn span(text: &[u8], format: Format) -> Option<usize> {
    let slack = SLACK.min(text.len() / 2);
    let end = cut(text, (text.len() - slack).max(1), format, slack / 2);
    (end < text.len()).then_some(end)
}

/// The merge of the pieces, in order, into the dictionary and the sink.
struct Merge<'m, 's, 'a, S> {
    start: &'m Start<'s, 'a>,
    /// The prefixes and base the pieces after the first start with.
    guessed: &'m (Prefixes<'a>, Option<Base>),
    /// The line start where the next piece must start, and the prefixes and
    /// base there.
    at: usize,
    prefixes: Prefixes<'a>,
    base: Option<Base>,
    /// The triples handed to the sink.
    triples: usize,
    /// A piece's term ids to the dictionary's.
    map: Vec<TermId>,
    dict: &'m mut Dictionary,
    path: Option<&'m Path>,
    sink: &'m mut S,
}

impl<'a, S: FnMut([TermId; 3])> Merge<'_, '_, 'a, S> {
    /// Merges the next piece, given its speculative parse; the result when
    /// the text is done.
    fn take(&mut self, piece: Piece, parsed: Option<Parsed<'a>>) -> Option<Result<Outcome, Error>> {
        let text = self.start.text;
        if piece.until <= self.at && piece.until < text.len() {
            return None; // read already, by the parse of an earlier piece
        }
        let kept = parsed.filter(|parsed| piece.from == self.at && (self.at == 0 || self.read_the_same(parsed)));
        let parsed = match kept {
            Some(parsed) => parsed,
            None => self
                .start
                .parse(self.at, piece.until, self.prefixes.start(), self.base.clone()),
        };
        // An open window's piece that ran short is left to the next window
        // (it cannot reach the end of the document: its lexer stops first).
        if matches!(parsed.result, Err(Fail::Short) | Ok(Stop::End)) && self.start.open {
            return Some(Ok(self.more(false)));
        }
        let fresh = parsed.terms.len() - vocab::IRIS.len();
        if self.dict.len() + fresh > MAX_TERMS - 3 {
            if self.start.open {
                return Some(Ok(self.more(true)));
            }
            // The dictionary may fill up: read the rest into it, as the
            // parse on one thread would, to fail where it would.
            let rest = sequential(
                self.dict,
                text,
                self.start.format,
                self.at,
                &self.prefixes,
                self.base.clone(),
                self.start.document,
                self.path,
                self.sink,
            );
            return Some(rest.map(|n| Outcome::Done(n + self.triples)));
        }
        let (dict, map) = (&mut *self.dict, &mut self.map);
        map.clear();
        map.extend((0..vocab::IRIS.len()).map(|i| TermId(i as u32)));
        for id in vocab::IRIS.len()..parsed.terms.len() {
            let id = match parsed.terms.term(TermId(id as u32)) {
                Term::Iri(iri) => dict.intern_iri(iri),
                Term::Blank { doc, label } => dict.intern_blank(doc, label),
                Term::Literal {
                    lexical,
                    language: Some(language),
                    ..
                } => dict.intern_lang_literal(lexical, language),
                Term::Literal { lexical, datatype, .. } => dict.intern_typed_literal(lexical, map[datatype.index()]),
                // Its blank nodes are already labelled with their document.
                #[cfg(feature = "rdf-12")]
                Term::Triple(triple) => dict.intern_written_triple(triple.clone()),
            };
            map.push(id);
        }
        for &[s, p, o] in &parsed.triples {
            (self.sink)([map[s.index()], map[p.index()], map[o.index()]]);
        }
        self.triples += parsed.triples.len();
        match parsed.result {
            Err(fail) => Some(Err(error(self.path, text, fail))),
            Ok(Stop::End) => Some(Ok(Outcome::Done(self.triples))),
            Ok(Stop::At(q)) => {
                self.at = q;
                self.prefixes.follow(&parsed.prefixes);
                if parsed.prefixes.base_set {
                    self.base = parsed.base;
                }
                None
            }
        }
    }

    /// True when what a speculative parse read of the prefixes and the base
    /// is the same here as where it started.
    fn read_the_same(&self, parsed: &Parsed<'a>) -> bool {
        let (prefixes, base) = self.guessed;
        parsed
            .prefixes
            .read
            .iter()
            .all(|name| self.prefixes.namespace(name) == prefixes.namespace(name))
            && (!parsed.base_read || self.base.as_ref().map(Base::as_str) == base.as_ref().map(Base::as_str))
    }

    /// Where an open window stopped, with the state there.
    fn more(&self, rest: bool) -> Outcome {
        Outcome::More {
            at: self.at,
            triples: self.triples,
            state: State {
                prefixes: self.prefixes.start(),
                base: self.base.clone(),
            },
            rest,
        }
    }

    /// The outcome once every piece is merged: only an open window's pieces
    /// stop short of the end.
    fn end(&self) -> Outcome {
        assert!(self.start.open, "the last piece reads to the end of the document");
        self.more(false)
    }
}

/// Parses N-Triples or Turtle on the caller's thread, from the start of
/// `text` in `state` to the end of the document; gives the triples handed to
/// the sink.
fn whole(
    dict: &mut Dictionary,
    text: &[u8],
    format: Format,
    state: &State,
    document: DocId,
    path: Option<&Path>,
    sink: &mut impl FnMut([TermId; 3]),
) -> Result<usize, Error> {
    sequential(
        dict,
        text,
        format,
        0,
        &state.prefixes,
        state.base.clone(),
        document,
        path,
        sink,
    )
}

/// The document from `at`, a line start between statements with these
/// prefixes and base, to its end, parsed on the caller's thread into the
/// dictionary.
#[allow(clippy::too_many_arguments)]
fn sequential(
    dict: &mut Dictionary,
    text: &[u8],
    format: Format,
    at: usize,
    prefixes: &Prefixes<'_>,
    base: Option<Base>,
    document: DocId,
    path: Option<&Path>,
    sink: &mut impl FnMut([TermId; 3]),
) -> Result<usize, Error> {
    let mode = match format {
        Format::NTriples => Mode::NTriples,
        _ => Mode::Turtle,
    };
    let mut lexer = Lexer::at(text, at, std::str::from_utf8(&text[at..]).ok(), mode, base, false);
    let mut prefixes = prefixes.start();
    let mut out = Out::new(dict, document, sink, path);
    let result = match format {
        Format::NTriples => ntriples::parse(&mut lexer, text.len(), &mut out),
        _ => turtle::parse(&mut lexer, text.len(), &mut prefixes, &mut out),
    };
    match result {
        Ok(_) => Ok(out.triples),
        Err(fail) => Err(out.error(text, fail)),
    }
}

/// The pieces of the text: cut at line starts about `size` bytes apart, the
/// first at most [`HEAD`] bytes long.
fn cuts(text: &[u8], format: Format, size: usize) -> Vec<Piece> {
    let mut starts = vec![0];
    let mut next = size.min(HEAD);
    while next < text.len() {
        let at = cut(text, next, format, size.min(LOOK));
        if at >= text.len() {
            break;
        }
        starts.push(at);
        next = at + size;
    }
    starts.push(text.len());
    starts
        .windows(2)
        .map(|w| Piece {
            from: w[0],
            until: w[1],
        })
        .collect()
}

/// A line start at or after `at`, `at` > 0 (the end of the text if none).
/// For Turtle, the first within `look` bytes that seems to start a statement:
/// it starts with neither a space nor a comment, and the text before it ends
/// in a dot. Else the first.
fn cut(text: &[u8], at: usize, format: Format, look: usize) -> usize {
    let line_start = |from: usize| {
        text[from - 1..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(text.len(), |i| from + i)
    };
    let first = line_start(at);
    if format == Format::NTriples {
        return first;
    }
    let mut c = first;
    while c < text.len() && c - first <= look {
        let starts = !matches!(text[c], b' ' | b'\t' | b'\r' | b'\n' | b'#');
        let after_dot = text[..c]
            .iter()
            .rev()
            .find(|b| !b.is_ascii_whitespace())
            .is_some_and(|&b| b == b'.');
        if starts && after_dot {
            return c;
        }
        c = line_start(c + 1);
    }
    first
}
