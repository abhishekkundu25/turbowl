//! Tokens of N-Triples and Turtle, and where an error is.
//!
//! Copyright (c) 2018 Oxigraph developers; used under the MIT license (see
//! `THIRD-PARTY-NOTICES.md` at the repository root).
//!
//! A port of `oxttl` 0.2.5's lexer (`N3Lexer` in its N-Triples and Turtle
//! modes, and the toolkit lexer's rules for whitespace, comments and
//! positions), because the contract is parity: TurbOWL's parsers cut a
//! document into the same tokens as Oxigraph's, accept and reject the same
//! tokens, and report an error at the same line and column. Oxigraph's own
//! quirks are kept on purpose (each is marked "as oxttl"); the cross-check
//! tests (`io/cross_check.rs`) compare the two on every document the suites
//! read.
//!
//! What differs is the cost. A token borrows its text from the document; only
//! text that must be built (decoded escapes, an IRI resolved against a base,
//! a local name with escapes) is written into the parse's arena, a `String`
//! the grammar clears between statements. IRIs are validated by
//! [`super::iri`], language tags by `oxrdf` (which is `oxilangtag`, as in
//! oxttl), each distinct tag once.
//!
//! Positions are worked out only for an error ([`Failure::position`]): the
//! lexer keeps no line count while it runs.

use super::iri::{self, Base};
use rustc_hash::FxHashSet;

/// Which grammar's tokens: N-Triples has no long strings and no `'` strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    NTriples,
    Turtle,
}

/// The text of a token or a term: borrowed from the document, or a range of
/// the parse's arena.
#[derive(Debug, Clone, Copy)]
pub(super) enum Text<'a> {
    Doc(&'a str),
    Arena(usize, usize),
}

impl<'a> Text<'a> {
    /// The text, reading the arena for an arena range.
    #[inline]
    pub(super) fn get<'s>(self, arena: &'s str) -> &'s str
    where
        'a: 's,
    {
        match self {
            Text::Doc(s) => s,
            Text::Arena(start, end) => &arena[start..end],
        }
    }
}

/// A token, as oxttl's `N3Token` (the N3-only tokens never appear).
#[derive(Debug, Clone, Copy)]
pub(super) enum Token<'a> {
    /// An IRI, validated, and resolved when there is a base.
    IriRef(Text<'a>),
    PrefixedName {
        prefix: &'a str,
        local: Text<'a>,
        might_be_invalid_iri: bool,
    },
    Variable(Text<'a>),
    BlankNodeLabel(&'a str),
    String(Text<'a>),
    LongString(Text<'a>),
    Integer(&'a str),
    Decimal(&'a str),
    Double(&'a str),
    /// A language tag, validated, as written (not lowercased).
    LangTag(&'a str),
    Punctuation(&'static str),
    PlainKeyword(&'a str),
}

impl Token<'_> {
    /// The token as oxttl's `{:?}` prints it (for one of its messages).
    pub(super) fn describe(&self, arena: &str) -> String {
        match *self {
            Token::IriRef(t) => format!("IriRef({:?})", t.get(arena)),
            Token::PrefixedName {
                prefix,
                local,
                might_be_invalid_iri,
            } => format!(
                "PrefixedName {{ prefix: {prefix:?}, local: {:?}, might_be_invalid_iri: {might_be_invalid_iri} }}",
                local.get(arena)
            ),
            Token::Variable(t) => format!("Variable({:?})", t.get(arena)),
            Token::BlankNodeLabel(s) => format!("BlankNodeLabel({s:?})"),
            Token::String(t) => format!("String({:?})", t.get(arena)),
            Token::LongString(t) => format!("LongString({:?})", t.get(arena)),
            Token::Integer(s) => format!("Integer({s:?})"),
            Token::Decimal(s) => format!("Decimal({s:?})"),
            Token::Double(s) => format!("Double({s:?})"),
            Token::LangTag(s) => format!("LangTag {{ language: {s:?} }}"),
            Token::Punctuation(s) => format!("Punctuation({s:?})"),
            Token::PlainKeyword(s) => format!("PlainKeyword({s:?})"),
        }
    }

    #[inline]
    pub(super) fn is_punctuation(&self, p: &str) -> bool {
        matches!(self, Token::Punctuation(q) if *q == p)
    }
}

/// What the lexer gives next.
#[derive(Debug, Clone, Copy)]
pub(super) enum Event<'a> {
    Token(Token<'a>),
    /// A line jump (or a comment, which ends at one). Only N-Triples reads it.
    LineJump,
    /// The end of the document.
    End,
}

/// A parse error before it has a line and column: the start of the token it
/// belongs to, the byte where it is, and the message.
#[derive(Debug)]
pub(super) struct Failure {
    pub(super) token_start: usize,
    pub(super) offset: usize,
    pub(super) message: String,
}

impl Failure {
    /// An error of the token (or line jump) that starts at `start`.
    pub(super) fn at(start: usize, message: String) -> Self {
        Failure {
            token_start: start,
            offset: start,
            message,
        }
    }

    /// The 0-based line and column (in characters) oxttl gives this error.
    ///
    /// Before the token, lines are counted as the lexer counts them: `\n`,
    /// `\r\n` and a lone `\r` each end a line. Inside the token, as oxttl
    /// (`find_number_of_line_jumps_and_start_of_last_line`, which does not
    /// count a `\n` at the token's second byte). A column counts the
    /// characters of the line's longest valid UTF-8 prefix.
    pub(super) fn position(&self, text: &[u8]) -> (u64, u64) {
        let start = self.token_start.min(text.len());
        let offset = self.offset.clamp(start, text.len());
        let before = &text[..start];
        let mut line = 0u64;
        let mut line_start = 0;
        let mut i = 0;
        while let Some(j) = before[i..].iter().position(|&b| b == b'\n' || b == b'\r') {
            let at = i + j;
            if before[at] == b'\r' && text.get(at + 1) == Some(&b'\n') {
                i = at + 2;
            } else {
                i = at + 1;
            }
            line += 1;
            line_start = i;
            if i >= before.len() {
                break;
            }
        }
        // `\r` as the token's last byte before it with `\n` as the token's
        // first cannot happen: a token never starts with `\n`.
        let (extra, last) = line_jumps(&text[start..offset]);
        let line_start = if extra > 0 { start + last } else { line_start };
        (line + extra, column(&text[line_start.min(offset)..offset]))
    }
}

/// oxttl's `find_number_of_line_jumps_and_start_of_last_line`, quirks kept.
fn line_jumps(bytes: &[u8]) -> (u64, usize) {
    let mut jumps = 0;
    let mut last = 0;
    let mut previous_cr = 0usize;
    for (pos, &b) in bytes.iter().enumerate() {
        if b == b'\r' {
            previous_cr = pos;
            jumps += 1;
            last = pos + 1;
        } else if b == b'\n' {
            if previous_cr < pos.wrapping_sub(1) {
                jumps += 1;
            }
            last = pos + 1;
        }
    }
    (jumps, last)
}

/// oxttl's `column_from_bytes`: the characters of the longest valid UTF-8 prefix.
fn column(bytes: &[u8]) -> u64 {
    let valid = match std::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => std::str::from_utf8(&bytes[..e.valid_up_to()]).unwrap_or_default(),
    };
    valid.chars().count() as u64
}

/// An error inside a token: the byte offset from the token's start, and the message.
type TokenError = (usize, String);

type Recognized<'a> = Option<(usize, Result<Token<'a>, TokenError>)>;

/// A local name, and whether the IRI it makes must be validated again.
type LocalName<'a> = (Text<'a>, bool);

/// The lexer over one document.
pub(super) struct Lexer<'a> {
    text: &'a [u8],
    pos: usize,
    /// Where the last token (or the line after the last line jump) starts.
    token_start: usize,
    mode: Mode,
    /// The base IRI relative IRIs resolve against (Turtle's `@base` sets it).
    pub(super) base: Option<Base>,
    /// The document as text, when all of it is UTF-8 (a token's text is then
    /// sliced from it, not checked again).
    utf8: Option<&'a str>,
    /// Language tags already validated.
    tags: FxHashSet<String>,
    /// Bytes of an IRI with escapes, decoded before validation.
    scratch: Vec<u8>,
}

impl<'a> Lexer<'a> {
    pub(super) fn new(text: &'a [u8], mode: Mode, base: Option<Base>) -> Self {
        Lexer {
            text,
            pos: 0,
            token_start: 0,
            mode,
            base,
            utf8: std::str::from_utf8(text).ok(),
            tags: FxHashSet::default(),
            scratch: Vec::new(),
        }
    }

    /// Where the last event starts: grammar errors are reported there.
    #[inline]
    pub(super) fn token_start(&self) -> usize {
        self.token_start
    }

    /// The last token's source text, as oxttl puts it into messages for `TOKEN`.
    pub(super) fn token_source(&self) -> String {
        String::from_utf8_lossy(&self.text[self.token_start..self.pos]).into_owned()
    }

    /// The next token, line jump or the end. Text a token builds goes into `arena`.
    pub(super) fn next(&mut self, arena: &mut String) -> Result<Event<'a>, Failure> {
        if self.skip_whitespace_and_comments() {
            self.token_start = self.pos;
            return Ok(Event::LineJump);
        }
        self.token_start = self.pos;
        let text = self.text;
        if self.pos >= text.len() {
            return Ok(Event::End);
        }
        match self.recognize(&text[self.pos..], arena) {
            None => {
                self.pos = text.len();
                Err(Failure::at(self.token_start, "Unexpected end of file".into()))
            }
            Some((consumed, result)) => {
                self.pos += consumed;
                result.map(Event::Token).map_err(|(at, message)| Failure {
                    token_start: self.token_start,
                    offset: self.token_start + at,
                    message,
                })
            }
        }
    }

    /// Skips spaces, tabs and comments; true when a line jump (or the line
    /// jump ending a comment) was consumed, as oxttl.
    #[inline]
    fn skip_whitespace_and_comments(&mut self) -> bool {
        let t = self.text;
        while let Some(&c) = t.get(self.pos) {
            match c {
                b' ' | b'\t' => self.pos += 1,
                b'\r' => {
                    self.pos += if t.get(self.pos + 1) == Some(&b'\n') { 2 } else { 1 };
                    return true;
                }
                b'\n' => {
                    self.pos += 1;
                    return true;
                }
                _ => break,
            }
        }
        if t.get(self.pos) == Some(&b'#') {
            let rest = &t[self.pos + 1..];
            return match rest.iter().position(|&b| b == b'\r' || b == b'\n') {
                Some(end) => {
                    let mut end = self.pos + 1 + end;
                    if t[end] == b'\r' && t.get(end + 1) == Some(&b'\n') {
                        end += 1;
                    }
                    self.pos = end + 1;
                    true
                }
                None => {
                    self.pos = t.len();
                    false
                }
            };
        }
        false
    }

    /// oxttl's `recognize_next_token` with `is_ending` set: `None` is an
    /// unexpected end of file.
    fn recognize(&mut self, data: &'a [u8], arena: &mut String) -> Recognized<'a> {
        let punctuation = |n: usize, p: &'static str| Some((n, Ok(Token::Punctuation(p))));
        match data[0] {
            b'<' => match *data.get(1)? {
                b'<' => {
                    if *data.get(2)? == b'(' {
                        punctuation(3, "<<(")
                    } else {
                        punctuation(2, "<<")
                    }
                }
                _ => self.recognize_iri(data, arena),
            },
            b'>' => {
                if *data.get(1)? == b'>' {
                    punctuation(2, ">>")
                } else {
                    punctuation(1, ">")
                }
            }
            b'_' => match *data.get(1)? {
                b':' => recognize_blank_node_label(data),
                c => Some((1, Err((0, format!("Unexpected character '{}'", char::from(c)))))),
            },
            b'"' => {
                if self.mode != Mode::NTriples && *data.get(1)? == b'"' && *data.get(2)? == b'"' {
                    recognize_long_string(data, b'"', arena)
                } else {
                    recognize_string(data, b'"', arena)
                }
            }
            b'\'' if self.mode != Mode::NTriples => {
                if *data.get(1)? == b'\'' && *data.get(2)? == b'\'' {
                    recognize_long_string(data, b'\'', arena)
                } else {
                    recognize_string(data, b'\'', arena)
                }
            }
            b'@' => self.recognize_lang_tag(data),
            b'.' => match data.get(1) {
                Some(b'0'..=b'9') => recognize_number(data),
                _ => punctuation(1, "."),
            },
            b'^' => {
                if *data.get(1)? == b'^' {
                    punctuation(2, "^^")
                } else {
                    punctuation(1, "^")
                }
            }
            b'(' => punctuation(1, "("),
            b')' => {
                if *data.get(1)? == b'>' && *data.get(2)? == b'>' {
                    punctuation(3, ")>>")
                } else {
                    punctuation(1, ")")
                }
            }
            b'[' => punctuation(1, "["),
            b']' => punctuation(1, "]"),
            b'{' => {
                if *data.get(1)? == b'|' {
                    punctuation(2, "{|")
                } else {
                    punctuation(1, "{")
                }
            }
            b'}' => punctuation(1, "}"),
            b',' => punctuation(1, ","),
            b';' => punctuation(1, ";"),
            b'!' => punctuation(1, "!"),
            b'|' => {
                if *data.get(1)? == b'}' {
                    punctuation(2, "|}")
                } else {
                    punctuation(1, "|")
                }
            }
            b'=' => {
                if *data.get(1)? == b'>' {
                    punctuation(2, "=>")
                } else {
                    punctuation(1, "=")
                }
            }
            b'~' => punctuation(1, "~"),
            b'0'..=b'9' | b'+' | b'-' => recognize_number(data),
            b'?' => recognize_variable(data, arena),
            _ => recognize_pname_or_keyword(data, arena),
        }
    }

    /// `IRIREF ::= '<' ([^#x00-#x20<>"{}|^`\] | UCHAR)* '>'`, as oxttl: the
    /// text up to `>` with escapes decoded, then validated (or resolved).
    fn recognize_iri(&mut self, data: &'a [u8], arena: &mut String) -> Recognized<'a> {
        let end = find2(&data[1..], b'>', b'\\')? + 1;
        if data[end] == b'>' {
            // No escape: the text is the document's.
            let consumed = end + 1;
            let iri = match self.utf8.and_then(|text| text.get(self.pos + 1..self.pos + end)) {
                Some(iri) => Ok(iri),
                None => {
                    std::str::from_utf8(&data[1..end]).map_err(|e| (e.valid_up_to(), format!("Invalid UTF-8: {e}")))
                }
            };
            return Some((consumed, iri.and_then(|iri| self.iri_token(iri, Some(iri), arena))));
        }
        let mut scratch = std::mem::take(&mut self.scratch);
        scratch.clear();
        let result = recognize_escaped_iri(data, &mut scratch).map(|(consumed, result)| {
            let token = result.and_then(|()| match std::str::from_utf8(&scratch) {
                Ok(iri) => self.iri_token(iri, None, arena),
                // As oxttl: the offset in the decoded text, taken as one in the token.
                Err(e) => Err((e.valid_up_to(), format!("Invalid UTF-8: {e}"))),
            });
            (consumed, token)
        });
        self.scratch = scratch;
        result
    }

    /// oxttl's `parse_iri`: the IRI resolved against the base when there is
    /// one, else validated as absolute. `doc` is the IRI when the document
    /// holds it as is (no escape).
    fn iri_token(&self, iri: &str, doc: Option<&'a str>, arena: &mut String) -> Result<Token<'a>, TokenError> {
        let start = arena.len();
        match &self.base {
            Some(base) => base.resolve_into(iri, arena).map_err(|message| (0, message))?,
            None => {
                iri::validate(iri).map_err(|message| (0, message))?;
                match doc {
                    Some(doc) => return Ok(Token::IriRef(Text::Doc(doc))),
                    None => arena.push_str(iri),
                }
            }
        }
        Ok(Token::IriRef(Text::Arena(start, arena.len())))
    }

    /// `LANG_DIR ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)* ('--' [a-zA-Z]+)?`, as oxttl.
    fn recognize_lang_tag(&mut self, data: &'a [u8]) -> Recognized<'a> {
        let mut is_last_block_empty = true;
        let mut are_digits_allowed = false;
        for (i, c) in data[1..].iter().enumerate() {
            if (are_digits_allowed && c.is_ascii_alphanumeric()) || (!are_digits_allowed && c.is_ascii_alphabetic()) {
                is_last_block_empty = false;
            } else if i == 0 {
                return Some((1, Err((1, "A language code should always start with a letter".into()))));
            } else if is_last_block_empty {
                if *c != b'-' {
                    return Some((i, self.lang_tag(&data[1..i], None, 1, i - 1)));
                }
                // `--`: a base direction follows.
                let after_dir = i + 2 + data[i + 2..].iter().take_while(|c| c.is_ascii_alphabetic()).count();
                if after_dir == data.len() {
                    return None;
                }
                return Some((
                    after_dir,
                    self.lang_tag(&data[1..i], Some(&data[i + 2..after_dir]), 1, after_dir),
                ));
            } else if *c == b'-' {
                is_last_block_empty = true;
                are_digits_allowed = true;
            } else {
                return Some((i + 1, self.lang_tag(&data[1..=i], None, 1, i)));
            }
        }
        None
    }

    /// oxttl's `parse_lang_tag` without RDF 1.2: a base direction is an error;
    /// the tag must be a valid BCP 47 tag. `start..end` is its range in the token.
    fn lang_tag(
        &mut self,
        tag: &'a [u8],
        direction: Option<&[u8]>,
        start: usize,
        end: usize,
    ) -> Result<Token<'a>, TokenError> {
        if let Some(direction) = direction {
            return Err((
                end - direction.len(),
                "Literal base direction are only allowed in RDF 1.2".into(),
            ));
        }
        let tag = str_from_utf8(tag, start)?;
        if !self.tags.contains(tag) {
            oxrdf::Literal::new_language_tagged_literal("", tag).map_err(|e| (start, e.to_string()))?;
            self.tags.insert(tag.to_owned());
        }
        Ok(Token::LangTag(tag))
    }
}

/// The first `a` or `b` in `bytes`, eight bytes at a time: a byte equal to
/// `a` is a zero byte of `word ^ a…a`, which `(x - 0x01…01) & !x & 0x80…80`
/// marks (its lowest mark is the first zero byte exactly).
#[inline]
fn find2(bytes: &[u8], a: u8, b: u8) -> Option<usize> {
    const ONES: u64 = u64::from_le_bytes([1; 8]);
    const HIGH: u64 = u64::from_le_bytes([0x80; 8]);
    let zero = |x: u64| x.wrapping_sub(ONES) & !x & HIGH;
    let (fa, fb) = (ONES * u64::from(a), ONES * u64::from(b));
    let (chunks, rest) = bytes.as_chunks::<8>();
    for (k, chunk) in chunks.iter().enumerate() {
        let word = u64::from_le_bytes(*chunk);
        let marks = zero(word ^ fa) | zero(word ^ fb);
        if marks != 0 {
            return Some(8 * k + (marks.trailing_zeros() / 8) as usize);
        }
    }
    let at = 8 * chunks.len();
    rest.iter().position(|&c| c == a || c == b).map(|i| at + i)
}

#[inline]
fn find1(bytes: &[u8], a: u8) -> Option<usize> {
    find2(bytes, a, a)
}

/// oxttl's `str_from_utf8`: an error at the first invalid byte, `start` being
/// where `data` sits in the token.
#[inline]
fn str_from_utf8(data: &[u8], start: usize) -> Result<&str, TokenError> {
    std::str::from_utf8(data).map_err(|e| (start + e.valid_up_to(), format!("Invalid UTF-8: {e}")))
}

/// The slow path of `recognize_iri`: an IRI with escapes, decoded into `out`.
/// `Ok(())` once the closing `>` is reached.
fn recognize_escaped_iri(data: &[u8], out: &mut Vec<u8>) -> Option<(usize, Result<(), TokenError>)> {
    let mut i = 1;
    loop {
        let end = find2(&data[i..], b'>', b'\\')?;
        out.extend_from_slice(&data[i..i + end]);
        i += end;
        if data[i] == b'>' {
            return Some((i + 1, Ok(())));
        }
        let (additional, c) = recognize_escape(&data[i..], i, false)?;
        i += additional + 1;
        match c {
            Ok(c) => {
                let mut buf = [0; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
            Err(e) => return Some((i, Err(e))),
        }
    }
}

/// `UCHAR` and, in strings, `ECHAR`, as oxttl: `data` starts at the
/// backslash, which is at `position` in the token. Gives how many bytes
/// follow the backslash.
fn recognize_escape(data: &[u8], position: usize, with_echar: bool) -> Option<(usize, Result<char, TokenError>)> {
    match *data.get(1)? {
        b'u' => match recognize_hex_char(&data[2..], 4, 'u', position) {
            Ok(c) => Some((5, Ok(c?))),
            Err(e) => Some((5, Err(e))),
        },
        b'U' => match recognize_hex_char(&data[2..], 8, 'U', position) {
            Ok(c) => Some((9, Ok(c?))),
            Err(e) => Some((9, Err(e))),
        },
        b't' if with_echar => Some((1, Ok('\t'))),
        b'b' if with_echar => Some((1, Ok('\x08'))),
        b'n' if with_echar => Some((1, Ok('\n'))),
        b'r' if with_echar => Some((1, Ok('\r'))),
        b'f' if with_echar => Some((1, Ok('\x0C'))),
        b'"' if with_echar => Some((1, Ok('"'))),
        b'\'' if with_echar => Some((1, Ok('\''))),
        b'\\' if with_echar => Some((1, Ok('\\'))),
        c => Some((
            1,
            Err((position, format!("Unexpected escape character '\\{}'", char::from(c)))),
        )),
    }
}

fn recognize_hex_char(data: &[u8], len: usize, escape_char: char, position: usize) -> Result<Option<char>, TokenError> {
    if data.len() < len {
        return Ok(None);
    }
    let val = || std::str::from_utf8(&data[..len]).unwrap_or_default();
    let mut codepoint: u32 = 0;
    for (i, &c) in data[..len].iter().enumerate() {
        let digit = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => {
                return Err((
                    position + i + 2,
                    format!(
                        "The escape sequence '\\{escape_char}{}' is not a valid hexadecimal string",
                        val()
                    ),
                ));
            }
        };
        codepoint = codepoint * 16 + u32::from(digit);
    }
    char::from_u32(codepoint).map(Some).ok_or_else(|| {
        (
            position,
            format!(
                "The escape sequence '\\{escape_char}{}' is encoding {codepoint:X} that is not a valid unicode character",
                val()
            ),
        )
    })
}

/// `STRING_LITERAL_QUOTE` or `STRING_LITERAL_SINGLE_QUOTE`, as oxttl
/// (`recognize_string`, not lenient).
fn recognize_string<'a>(data: &'a [u8], delimiter: u8, arena: &mut String) -> Recognized<'a> {
    // Fast path: no escape and no line jump before the closing delimiter.
    let end = find2(&data[1..], delimiter, b'\\')? + 1;
    let body = &data[1..end];
    if data[end] == delimiter && !body.iter().any(|&b| b == b'\n' || b == b'\r') {
        return Some((end + 1, str_from_utf8(body, 1).map(|s| Token::String(Text::Doc(s)))));
    }
    let start = arena.len();
    let (consumed, result) = recognize_string_slow(data, delimiter, arena)?;
    Some((
        consumed,
        result.map(|()| Token::String(Text::Arena(start, arena.len()))),
    ))
}

fn recognize_string_slow(data: &[u8], delimiter: u8, out: &mut String) -> Option<(usize, Result<(), TokenError>)> {
    let mut i = 1;
    loop {
        let mut end = find2(&data[i..], delimiter, b'\\')?;
        if let Some(line_jump) = find2(&data[i..i + end], b'\n', b'\r') {
            end = line_jump;
        }
        match str_from_utf8(&data[i..i + end], i) {
            Ok(s) => out.push_str(s),
            Err(e) => return Some((end, Err(e))),
        }
        i += end;
        match data[i] {
            c if c == delimiter => return Some((i + 1, Ok(()))),
            b'\\' => {
                let (additional, c) = recognize_escape(&data[i..], i, true)?;
                i += additional + 1;
                match c {
                    Ok(c) => out.push(c),
                    Err(e) => {
                        // As oxttl: read to the end of the string first.
                        let end = find1(&data[i..], delimiter)?;
                        return Some((i + end + 1, Err(e)));
                    }
                }
            }
            _ => {
                // A line jump.
                let end = find1(&data[i..], delimiter)?;
                return Some((
                    i + end + 1,
                    Err((i, "Line jumps are not allowed in string literals, use \\n".into())),
                ));
            }
        }
    }
}

/// `STRING_LITERAL_LONG_QUOTE` or `STRING_LITERAL_LONG_SINGLE_QUOTE`, as oxttl.
fn recognize_long_string<'a>(data: &'a [u8], delimiter: u8, arena: &mut String) -> Recognized<'a> {
    let start = arena.len();
    let mut i = 3;
    loop {
        let end = find2(&data[i..], delimiter, b'\\')?;
        match str_from_utf8(&data[i..i + end], i) {
            Ok(s) => arena.push_str(s),
            Err(e) => return Some((end, Err(e))),
        }
        i += end;
        if data[i] == delimiter {
            if *data.get(i + 1)? == delimiter && *data.get(i + 2)? == delimiter {
                return Some((i + 3, Ok(Token::LongString(Text::Arena(start, arena.len())))));
            }
            i += 1;
            arena.push(char::from(delimiter));
        } else {
            let (additional, c) = recognize_escape(&data[i..], i, true)?;
            i += additional + 1;
            match c {
                Ok(c) => arena.push(c),
                Err(e) => return Some((i, Err(e))),
            }
        }
    }
}

/// `INTEGER`, `DECIMAL` and `DOUBLE`, as oxttl's `recognize_number` at the end of input.
fn recognize_number<'a>(data: &'a [u8]) -> Recognized<'a> {
    let mut i = 0;
    if matches!(data[0], b'+' | b'-') {
        i += 1;
    }
    let count_before = digits(&data[i..]);
    i += count_before;
    let count_after = if data.get(i) == Some(&b'.') {
        i += 1;
        let count_after = digits(&data[i..]);
        i += count_after;
        Some(count_after)
    } else {
        None
    };
    let number = |n: usize, make: fn(&'a str) -> Token<'a>| str_from_utf8(&data[..n], 0).map(make);
    if matches!(data.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(data.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let count_exp = digits(&data[i..]);
        i += count_exp;
        Some((
            i,
            if count_exp == 0 {
                Err((0, "A double exponent cannot be empty".into()))
            } else if count_before == 0 && count_after.unwrap_or(0) == 0 {
                Err((0, "A double should not be empty".into()))
            } else {
                number(i, Token::Double)
            },
        ))
    } else if let Some(count_after) = count_after {
        if count_after == 0 {
            // The '.' is not part of the number after all.
            i -= 1;
            Some((
                i,
                if count_before == 0 {
                    Err((0, "An integer should not be empty".into()))
                } else {
                    number(i, Token::Integer)
                },
            ))
        } else {
            Some((i, number(i, Token::Decimal)))
        }
    } else {
        Some((
            i,
            if count_before == 0 {
                Err((0, "An integer should not be empty".into()))
            } else {
                number(i, Token::Integer)
            },
        ))
    }
}

#[inline]
fn digits(data: &[u8]) -> usize {
    data.iter().take_while(|c| c.is_ascii_digit()).count()
}

/// `QUICK_VAR_NAME ::= "?" PN_LOCAL`, as oxttl (Turtle rejects it in the grammar).
fn recognize_variable<'a>(data: &'a [u8], arena: &mut String) -> Recognized<'a> {
    let (consumed, result) = recognize_optional_pn_local(&data[1..], arena)?;
    Some((
        consumed + 1,
        result.and_then(|(name, _)| {
            let empty = match name {
                Text::Doc(s) => s.is_empty(),
                Text::Arena(a, b) => a == b,
            };
            if empty {
                Err((0, "A variable name is not allowed to be empty".into()))
            } else {
                Ok(Token::Variable(name))
            }
        }),
    ))
}

/// `PNAME_NS`, `PNAME_LN` or a keyword (`a`, `true`, `PREFIX`...), as oxttl.
fn recognize_pname_or_keyword<'a>(data: &'a [u8], arena: &mut String) -> Recognized<'a> {
    let mut i = 0;
    loop {
        match recognize_unicode_char(&data[i..], i) {
            Some(Ok((c, consumed))) => {
                if c == ':' {
                    i += consumed;
                    break;
                } else if i == 0 {
                    if !is_possible_pn_chars_base(c) {
                        return Some((
                            consumed,
                            Err((0, format!("'{c}' is not allowed at the beginning of a prefix name"))),
                        ));
                    }
                    i += consumed;
                } else if is_possible_pn_chars(c) || c == '.' {
                    i += consumed;
                } else {
                    while data[..i].ends_with(b".") {
                        i -= 1;
                    }
                    return Some((i, str_from_utf8(&data[..i], 0).map(Token::PlainKeyword)));
                }
            }
            Some(Err(e)) => return Some((data.len(), Err(e))),
            None => {
                while data[..i].ends_with(b".") {
                    i -= 1;
                }
                return Some(if i == 0 {
                    (1, Err((0, format!("Unexpected byte {}", data[0]))))
                } else {
                    (i, str_from_utf8(&data[..i], 0).map(Token::PlainKeyword))
                });
            }
        }
    }
    let prefix = match str_from_utf8(&data[..i - 1], 0) {
        Ok(prefix) => prefix,
        Err(e) => return Some((i, Err(e))),
    };
    if prefix.ends_with('.') {
        return Some((
            i,
            Err((
                0,
                format!("'{prefix}' is not a valid prefix: prefixes are not allowed to end with '.'"),
            )),
        ));
    }
    // As oxttl, an error in the local name is placed as if the local name
    // started the token.
    let (consumed, local) = recognize_optional_pn_local(&data[i..], arena)?;
    Some((
        consumed + i,
        local.map(|(local, might_be_invalid_iri)| Token::PrefixedName {
            prefix,
            local,
            might_be_invalid_iri,
        }),
    ))
}

/// `PN_LOCAL`, possibly empty, as oxttl's `recognize_optional_pn_local` at
/// the end of input (not lenient). Escapes make the text an arena range.
fn recognize_optional_pn_local<'a>(
    data: &'a [u8],
    arena: &mut String,
) -> Option<(usize, Result<LocalName<'a>, TokenError>)> {
    let mut i = 0;
    let mut buffer: Option<usize> = None; // where the decoded text starts in the arena
    let mut in_buffer = 0; // data before this is in the buffer
    let mut might_be_invalid_iri = false;
    let mut ends_with_unescaped_dot = 0;
    loop {
        match recognize_unicode_char(&data[i..], i) {
            Some(Ok((c, consumed))) => {
                if c == '%' {
                    i += 1;
                    let &a = data.get(i)?;
                    i += 1;
                    let &b = data.get(i)?;
                    let (a, b) = (char::from(a), char::from(b));
                    if !a.is_ascii_hexdigit() || !b.is_ascii_hexdigit() {
                        return Some((
                            i + 1,
                            Err((
                                i - 2,
                                format!(
                                    "escapes in IRIs should be % followed by two hexadecimal characters, found '%{a}{b}'"
                                ),
                            )),
                        ));
                    }
                    i += 1;
                    ends_with_unescaped_dot = 0;
                } else if c == '\\' {
                    i += 1;
                    let &a = data.get(i)?;
                    let a = char::from(a);
                    if matches!(
                        a,
                        '_' | '~' | '.' | '-' | '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '='
                    ) {
                        // Allowed.
                    } else if matches!(a, '/' | '?' | '#' | '@' | '%') {
                        // Allowed, but the IRI must be checked.
                        might_be_invalid_iri = true;
                    } else {
                        return Some((
                            i + 1,
                            Err((
                                i,
                                format!(
                                    "The character that are allowed to be escaped in IRIs are _~.-!$&'()*+,;=/?#@%, found '{a}'"
                                ),
                            )),
                        ));
                    }
                    buffer.get_or_insert(arena.len());
                    if i - in_buffer > 1 {
                        match str_from_utf8(&data[in_buffer..i - 1], in_buffer) {
                            Ok(s) => arena.push_str(s),
                            Err(e) => return Some((i, Err(e))),
                        }
                    }
                    arena.push(a);
                    i += 1;
                    in_buffer = i;
                    ends_with_unescaped_dot = 0;
                } else if i == 0 {
                    if !(is_possible_pn_chars_u(c) || c == ':' || c.is_ascii_digit()) {
                        return Some((0, Ok((Text::Doc(""), false))));
                    }
                    might_be_invalid_iri |= is_possible_pn_chars_base_but_not_valid_iri(c) || c == ':';
                    i += consumed;
                } else if is_possible_pn_chars(c) || c == ':' {
                    might_be_invalid_iri |= is_possible_pn_chars_base_but_not_valid_iri(c) || c == ':';
                    i += consumed;
                    ends_with_unescaped_dot = 0;
                } else if c == '.' {
                    i += consumed;
                    ends_with_unescaped_dot += 1;
                } else {
                    let text = if let Some(start) = buffer {
                        match str_from_utf8(&data[in_buffer..i], in_buffer) {
                            Ok(s) => arena.push_str(s),
                            Err(e) => return Some((i, Err(e))),
                        }
                        // The last dots are not part of the name.
                        for _ in 0..ends_with_unescaped_dot {
                            arena.pop();
                        }
                        i -= ends_with_unescaped_dot;
                        Text::Arena(start, arena.len())
                    } else {
                        let s = match str_from_utf8(&data[..i], 0) {
                            Ok(s) => s,
                            Err(e) => return Some((i, Err(e))),
                        };
                        i -= ends_with_unescaped_dot;
                        Text::Doc(&s[..s.len() - ends_with_unescaped_dot])
                    };
                    return Some((i, Ok((text, might_be_invalid_iri))));
                }
            }
            Some(Err(e)) => return Some((data.len(), Err(e))),
            None => {
                // The end of the input.
                let text = if let Some(start) = buffer {
                    // As oxttl: the text after the last escape is not added,
                    // and every trailing dot of the buffer goes.
                    while arena.len() > start && arena.ends_with('.') {
                        arena.pop();
                        i -= 1;
                    }
                    Text::Arena(start, arena.len())
                } else {
                    let mut s = match str_from_utf8(&data[..i], 0) {
                        Ok(s) => s,
                        Err(e) => return Some((i, Err(e))),
                    };
                    while let Some(d) = s.strip_suffix('.') {
                        s = d;
                        i -= 1;
                    }
                    Text::Doc(s)
                };
                return Some((i, Ok((text, might_be_invalid_iri))));
            }
        }
    }
}

/// `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?`, as oxttl.
fn recognize_blank_node_label(data: &[u8]) -> Recognized<'_> {
    let mut i = 2;
    let label = |i: usize| {
        if i > 2 {
            str_from_utf8(&data[2..i], 2).map(Token::BlankNodeLabel)
        } else {
            Err((0, "A blank node ID cannot be empty".into()))
        }
    };
    loop {
        match recognize_unicode_char(&data[i..], i) {
            Some(Ok((c, consumed))) => {
                if (i == 2 && (is_possible_pn_chars_u(c) || c.is_ascii_digit()))
                    || (i > 2 && (is_possible_pn_chars(c) || c == '.'))
                {
                    i += consumed;
                } else {
                    while data[i - 1] == b'.' {
                        i -= 1;
                    }
                    return Some((i, label(i)));
                }
            }
            Some(Err(e)) => return Some((data.len(), Err(e))),
            None => {
                while data[i - 1] == b'.' {
                    i -= 1;
                }
                return Some((i, label(i)));
            }
        }
    }
}

/// One UTF-8 character at the start of `data` (at `position` in the token)
/// and its length, as oxttl; `None` when `data` ends first.
#[inline]
fn recognize_unicode_char(data: &[u8], position: usize) -> Option<Result<(char, usize), TokenError>> {
    let byte = *data.first()?;
    let (bytes_needed, mut code_point, mut lower, mut upper) = match byte {
        0x00..=0x7F => return Some(Ok((char::from(byte), 1))),
        0xC2..=0xDF => (1, u32::from(byte) & 0x1F, 0x80, 0xBF),
        0xE0..=0xEF => (
            2,
            u32::from(byte) & 0xF,
            if byte == 0xE0 { 0xA0 } else { 0x80 },
            if byte == 0xED { 0x9F } else { 0xBF },
        ),
        0xF0..=0xF4 => (
            3,
            u32::from(byte) & 0x7,
            if byte == 0xF0 { 0x90 } else { 0x80 },
            if byte == 0xF4 { 0x8F } else { 0xBF },
        ),
        _ => return Some(Err((position, "Invalid UTF-8 character encoding".into()))),
    };
    for i in 1..=bytes_needed {
        let byte = *data.get(i)?;
        if byte < lower || upper < byte {
            return Some(Err((position, "Invalid UTF-8 character encoding".into())));
        }
        lower = 0x80;
        upper = 0xBF;
        code_point = (code_point << 6) | (u32::from(byte) & 0x3F);
    }
    Some(
        char::from_u32(code_point)
            .map(|c| (c, bytes_needed + 1))
            .ok_or_else(|| {
                (
                    position,
                    format!("The codepoint {code_point:X} is not a valid unicode character"),
                )
            }),
    )
}

// [157s] PN_CHARS_BASE
fn is_possible_pn_chars_base(c: char) -> bool {
    matches!(c,
        'A'..='Z'
        | 'a'..='z'
        | '\u{00C0}'..='\u{00D6}'
        | '\u{00D8}'..='\u{00F6}'
        | '\u{00F8}'..='\u{02FF}'
        | '\u{0370}'..='\u{037D}'
        | '\u{037F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}'
        | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}'
        | '\u{FDF0}'..='\u{FFFD}'
        | '\u{10000}'..='\u{EFFFF}')
}

// [158s] PN_CHARS_U
fn is_possible_pn_chars_u(c: char) -> bool {
    is_possible_pn_chars_base(c) || c == '_'
}

// [160s] PN_CHARS
fn is_possible_pn_chars(c: char) -> bool {
    is_possible_pn_chars_u(c)
        || matches!(c, '-' | '0'..='9' | '\u{00B7}' | '\u{0300}'..='\u{036F}' | '\u{203F}'..='\u{2040}')
}

fn is_possible_pn_chars_base_but_not_valid_iri(c: char) -> bool {
    matches!(c, '\u{FFF0}'..='\u{FFFD}')
        || u32::from(c) % u32::from('\u{FFFE}') == 0
        || u32::from(c) % u32::from('\u{FFFF}') == 0
}

/// oxttl's `resolve_local_name`: the IRI of `prefix:local`, written into
/// `arena`, given the prefix's namespace and whether its path is empty. The
/// IRI is validated when the local name may make it invalid, or when the
/// namespace has no path (the local name may then reach the authority).
pub(super) fn resolve_local_name<'a>(
    prefix: &str,
    local: Text<'a>,
    might_be_invalid_iri: bool,
    namespace: Option<&(String, bool)>,
    arena: &mut String,
) -> Result<Text<'a>, String> {
    let Some((namespace, empty_path)) = namespace else {
        return Err(format!("The prefix {prefix}: has not been declared"));
    };
    let start = arena.len();
    arena.push_str(namespace);
    match local {
        Text::Doc(local) => arena.push_str(local),
        Text::Arena(a, b) => arena.extend_from_within(a..b),
    }
    if (might_be_invalid_iri || *empty_path)
        && let Err(e) = iri::validate(&arena[start..])
    {
        return Err(format!(
            "The prefixed name {prefix}:{} builds IRI {} that is invalid: {e}",
            local.get(arena),
            &arena[start..]
        ));
    }
    Ok(Text::Arena(start, arena.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str, mode: Mode) -> Result<Vec<String>, (u64, u64, String)> {
        let mut lexer = Lexer::new(text.as_bytes(), mode, None);
        let mut arena = String::new();
        let mut out = Vec::new();
        loop {
            match lexer.next(&mut arena) {
                Ok(Event::Token(t)) => out.push(t.describe(&arena)),
                Ok(Event::LineJump) => out.push("\\n".into()),
                Ok(Event::End) => return Ok(out),
                Err(f) => {
                    let (line, column) = f.position(text.as_bytes());
                    return Err((line + 1, column + 1, f.message));
                }
            }
        }
    }

    #[test]
    fn tokens_borrow_or_decode() {
        let t = tokens(
            r#"<http://example.org/a> ex:b\-c _:x "s\n" """l"l""" 'q' @en-GB ^^ 1 -2.5 3e4 a ."#,
            Mode::Turtle,
        )
        .unwrap();
        assert_eq!(
            t,
            [
                r#"IriRef("http://example.org/a")"#,
                r#"PrefixedName { prefix: "ex", local: "b-c", might_be_invalid_iri: false }"#,
                r#"BlankNodeLabel("x")"#,
                r#"String("s\n")"#,
                r#"LongString("l\"l")"#,
                r#"String("q")"#,
                r#"LangTag { language: "en-GB" }"#,
                r#"Punctuation("^^")"#,
                r#"Integer("1")"#,
                r#"Decimal("-2.5")"#,
                r#"Double("3e4")"#,
                r#"PlainKeyword("a")"#,
                r#"Punctuation(".")"#,
            ]
        );
    }

    #[test]
    fn line_jumps_and_comments() {
        let t = tokens("<http://e/a> # c\r\n\r<http://e/b>\n", Mode::NTriples).unwrap();
        assert_eq!(t.len(), 5, "{t:?}");
        // A comment at the end without a line jump is not a line jump.
        assert_eq!(tokens("# c", Mode::NTriples).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn errors_are_placed_as_oxttl_places_them() {
        // An escape error at the backslash.
        assert_eq!(tokens("\n  \"ab\\q\"", Mode::Turtle).unwrap_err().0, 2);
        assert_eq!(tokens("\n  \"ab\\q\"", Mode::Turtle).unwrap_err().1, 6);
        // An unterminated IRI: the end of file, at the token.
        assert_eq!(tokens("<a> <http://e", Mode::NTriples).unwrap_err().1, 1);
        // A relative IRI without a base.
        let (l, c, m) = tokens("x\n <a>", Mode::Turtle).unwrap_err();
        assert_eq!((l, c), (2, 2), "{m}");
        // Columns count characters, not bytes.
        assert_eq!(tokens("é \"\\q\"", Mode::Turtle).unwrap_err().1, 4);
    }

    #[test]
    fn positions_inside_a_token_keep_oxttl_quirk() {
        // A `\n` as the token's second byte is not counted inside the token.
        let f = Failure {
            token_start: 0,
            offset: 4,
            message: String::new(),
        };
        assert_eq!(f.position(b"<\nab\\q>"), (0, 4));
        let f = Failure {
            token_start: 0,
            offset: 5,
            message: String::new(),
        };
        assert_eq!(f.position(b"<a\nbc\\q>"), (1, 2));
        assert_eq!(line_jumps(b"a\r\nb\rc\n"), (3, 7));
    }

    #[test]
    fn find2_finds_the_first_of_two_bytes() {
        let text = b"0123456789abcdef>ghij\\klmnopqrstuvwxyz>";
        for start in 0..text.len() {
            for end in start..=text.len() {
                let slice = &text[start..end];
                let expected = slice.iter().position(|&c| c == b'>' || c == b'\\');
                assert_eq!(find2(slice, b'>', b'\\'), expected, "{start}..{end}");
                assert_eq!(find1(slice, b'z'), slice.iter().position(|&c| c == b'z'));
            }
        }
        // Bytes around the marks' borrows: 0x00, 0x01, 0x80, 0xff.
        let tricky = [0u8, 1, 0x80, 0xff, 0x3e, 0x3d, 0x3f, 0];
        assert_eq!(find2(&tricky, b'>', b'\\'), Some(4));
        assert_eq!(find2(&tricky, 0, 0xff), Some(0));
        assert_eq!(find2(&tricky[1..], 0, 0xff), Some(2));
    }
}
