//! IRI validation and resolution for the Turtle and N-Triples parsers.
//!
//! Copyright (c) 2018 Oxigraph developers; used under the MIT license (see
//! `THIRD-PARTY-NOTICES.md` at the repository root).
//!
//! This follows the parser of oxiri 0.2.11 (MIT/Apache-2.0) in its checked
//! mode, because oxiri is not a direct dependency of this crate. oxttl
//! validates and resolves IRIs with oxiri, so parity with oxiri is the
//! contract: the same strings accepted, the same resolutions, the same error
//! messages.
//!
//! [`validate`] first tries a scan over bytes for the common shape (ASCII, a
//! scheme, an optional plain host and port, a path, a query, a fragment). The
//! scan only accepts; anything it does not accept goes through the ported
//! parser, which decides and gives oxiri's message.

use std::fmt;
use std::net::{AddrParseError, Ipv6Addr};
use std::str::{Chars, FromStr};

/// Validates an absolute IRI exactly as `oxiri::Iri::parse` 0.2.11 does.
/// Err holds the message of oxiri's `IriParseError` (its Display).
pub(crate) fn validate(iri: &str) -> Result<(), String> {
    if is_plain_absolute(iri.as_bytes()) {
        return Ok(());
    }
    parse_absolute(iri).map(|_| ()).map_err(|e| e.to_string())
}

/// True when the path of `iri`, a valid absolute IRI, is empty, as
/// `oxiri::Iri::parse_unchecked(iri).path().is_empty()` (oxttl uses it to
/// decide whether a prefixed name must be validated again). The answer for an
/// invalid IRI is unspecified.
pub(crate) fn has_empty_path(iri: &str) -> bool {
    // In a valid absolute IRI the scheme ends at the first ':', and an
    // authority holds no '/', '?' or '#' (an IP literal included), so the path
    // is empty exactly when what follows the scheme and the authority is
    // empty or starts a query or a fragment.
    let bytes = iri.as_bytes();
    let Some(colon) = bytes.iter().position(|&b| b == b':') else {
        return false;
    };
    let mut rest = &bytes[colon + 1..];
    if let Some(authority) = rest.strip_prefix(b"//") {
        let end = authority
            .iter()
            .position(|&b| matches!(b, b'/' | b'?' | b'#'))
            .unwrap_or(authority.len());
        rest = &authority[end..];
    }
    rest.first().is_none_or(|&b| matches!(b, b'?' | b'#'))
}

/// A base IRI: an absolute, valid IRI, with what resolution needs (the ends of
/// its scheme, authority, path and query), parsed once.
#[derive(Debug, Clone)]
pub(crate) struct Base {
    iri: String,
    positions: Positions,
}

impl Base {
    /// As `oxiri::Iri::parse`: Err holds oxiri's message.
    pub(crate) fn new(iri: &str) -> Result<Base, String> {
        let positions = parse_absolute(iri).map_err(|e| e.to_string())?;
        Ok(Base {
            iri: iri.to_owned(),
            positions,
        })
    }

    /// Appends `reference` resolved against this base to `out`, exactly as
    /// `oxiri::Iri::resolve` 0.2.11 (checked mode) does: the same output for
    /// every reference it accepts, and an error (oxiri's message) for every
    /// reference it rejects. On Err, `out` may hold a partial result; the
    /// caller truncates it.
    pub(crate) fn resolve_into(&self, reference: &str, out: &mut String) -> Result<(), String> {
        // A reference with a scheme never reads the base, and oxiri copies it
        // unchanged (no dot segments are removed), so it only needs validating.
        if scheme_end(reference.as_bytes()).is_some() {
            validate(reference)?;
            out.push_str(reference);
            return Ok(());
        }
        let base = BaseRef {
            iri: &self.iri,
            positions: self.positions,
        };
        let start = out.len();
        let mut output = Appended { buf: out, start };
        Parser::parse(reference, Some(base), &mut output)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

// The scan over bytes.

const SCHEME: u8 = 1; // ALPHA / DIGIT / '+' / '-' / '.'
const HOST: u8 = 2; // unreserved / sub-delims
const PATH: u8 = 4; // HOST / ':' / '@' / '/'
const QUERY: u8 = 8; // PATH / '?': the ASCII of a query and of a fragment

static CLASS: [u8; 256] = classes();

const fn classes() -> [u8; 256] {
    let mut table = [0; 256];
    let mut i = 0;
    while i < 128 {
        let c = i as u8;
        let mut class = 0;
        if c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.') {
            class |= SCHEME;
        }
        if c.is_ascii_alphanumeric()
            || matches!(
                c,
                b'-' | b'.' | b'_' | b'~' | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
            )
        {
            class |= HOST | PATH | QUERY;
        }
        if matches!(c, b':' | b'@' | b'/') {
            class |= PATH | QUERY;
        }
        if c == b'?' {
            class |= QUERY;
        }
        table[i] = class;
        i += 1;
    }
    table
}

/// True when `iri` is an ASCII absolute IRI of the common shape that oxiri
/// accepts: `scheme ":" ["//" host [":" port]] path ["?" query] ["#"
/// fragment]`, with a host of unreserved, sub-delims and percent encodings
/// (no userinfo, no IP literal). False means only "not decided here".
fn is_plain_absolute(bytes: &[u8]) -> bool {
    plain_absolute(bytes).is_some()
}

/// [`is_plain_absolute`], giving where the path starts and ends.
fn plain_absolute(bytes: &[u8]) -> Option<(usize, usize)> {
    let mut i = scheme_end(bytes)?;
    if bytes[i..].starts_with(b"//") {
        i = scan(bytes, i + 2, HOST)?;
        if bytes.get(i) == Some(&b':') {
            i += 1;
            while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
        }
        if bytes.get(i).is_some_and(|b| !matches!(b, b'/' | b'?' | b'#')) {
            return None;
        }
    }
    Some((i, plain_tail(bytes, i)?))
}

/// The rest of [`plain_absolute`] from `i`, a point of the path between two
/// characters: the path, the query, the fragment, the end. Gives where the
/// path ends.
#[inline]
fn plain_tail(bytes: &[u8], i: usize) -> Option<usize> {
    let path_end = scan(bytes, i, PATH)?;
    let mut i = path_end;
    if bytes.get(i) == Some(&b'?') {
        i = scan(bytes, i + 1, QUERY)?;
    }
    if bytes.get(i) == Some(&b'#') {
        i = scan(bytes, i + 1, QUERY)?;
    }
    (i == bytes.len()).then_some(path_end)
}

/// The index just after the ':' ending the scheme, if `bytes` starts with one.
fn scheme_end(bytes: &[u8]) -> Option<usize> {
    if !bytes.first()?.is_ascii_alphabetic() {
        return None;
    }
    let mut i = 1;
    while bytes.get(i).is_some_and(|&b| CLASS[b as usize] & SCHEME != 0) {
        i += 1;
    }
    (bytes.get(i) == Some(&b':')).then_some(i + 1)
}

/// Skips the bytes of `class` and valid percent encodings from `i`; None on a
/// '%' not followed by two hex digits.
#[inline]
fn scan(bytes: &[u8], mut i: usize, class: u8) -> Option<usize> {
    if class & (PATH | QUERY) != 0 {
        i = skip_plain(bytes, i);
    }
    while let Some(&b) = bytes.get(i) {
        if CLASS[b as usize] & class != 0 {
            i += 1;
        } else if b == b'%' {
            if bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
                && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit)
            {
                i += 3;
            } else {
                return None;
            }
        } else {
            break;
        }
    }
    Some(i)
}

/// Skips, eight at a time, bytes that every part of a path, a query and a
/// fragment accepts and that end none of them: ASCII letters and digits and
/// `&'()*+,-./:;@_`. A word is tested per byte with additions that cannot
/// carry between bytes (each byte is below 0x80 first): `b + (0x80 - lo)` has
/// its high bit set when `b >= lo`, `b + (0x7f - hi)` when `b > hi`.
#[inline]
fn skip_plain(bytes: &[u8], mut i: usize) -> usize {
    const ONES: u64 = u64::from_le_bytes([1; 8]);
    const HIGH: u64 = u64::from_le_bytes([0x80; 8]);
    const fn within(w: u64, lo: u8, hi: u8) -> u64 {
        let at_least = w + ONES * (0x80 - lo as u64);
        let above = w + ONES * (0x7f - hi as u64);
        at_least & !above & HIGH
    }
    while let Some(chunk) = bytes.get(i..i + 8) {
        let w = u64::from_le_bytes(chunk.try_into().unwrap_or_default());
        if w & HIGH != 0 {
            break;
        }
        let plain = within(w, b'&', b';') | within(w, b'@', b'Z') | within(w, b'a', b'z') | within(w, b'_', b'_');
        if plain != HIGH {
            break;
        }
        i += 8;
    }
    i
}

// The parser, ported from oxiri 0.2.11's `IriParser` in checked mode.

/// Parses `iri` as `oxiri::Iri::parse` does: an IRI reference, then a check
/// that it has a scheme.
fn parse_absolute(iri: &str) -> Result<Positions, IriError> {
    let positions = Parser::parse(iri, None, &mut Void::default())?;
    if positions.scheme_end == 0 {
        return Err(IriError::NoScheme);
    }
    Ok(positions)
}

/// The ends of the parts of a parsed IRI, as byte offsets into its output.
#[derive(Debug, Clone, Copy, Default)]
struct Positions {
    scheme_end: usize,
    authority_end: usize,
    path_end: usize,
    query_end: usize,
}

#[derive(Debug, Clone, Copy)]
struct BaseRef<'a> {
    iri: &'a str,
    positions: Positions,
}

/// oxiri's `IriParseErrorKind`; Display gives its messages.
#[derive(Debug)]
enum IriError {
    NoScheme,
    InvalidHostCharacter(char),
    InvalidHostIp(AddrParseError),
    InvalidPortCharacter(char),
    InvalidIriCodePoint(char),
    InvalidPercentEncoding([Option<char>; 3]),
    PathStartingWithTwoSlashes,
}

impl fmt::Display for IriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IriError::NoScheme => write!(f, "No scheme found in an absolute IRI"),
            IriError::InvalidHostCharacter(c) => write!(f, "Invalid character '{c}' in host"),
            IriError::InvalidHostIp(e) => write!(f, "Invalid host IP ({e})"),
            IriError::InvalidPortCharacter(c) => write!(f, "Invalid character '{c}'"),
            IriError::InvalidIriCodePoint(c) => write!(f, "Invalid IRI code point '{c}'"),
            IriError::InvalidPercentEncoding(cs) => {
                let cs: String = cs.iter().flatten().collect();
                write!(f, "Invalid IRI percent encoding '{cs}'")
            }
            IriError::PathStartingWithTwoSlashes => {
                write!(f, "An IRI path is not allowed to start with //")
            }
        }
    }
}

/// Where the parser writes. Lengths and truncations are relative to the start
/// of this IRI's output.
trait Output {
    fn push(&mut self, c: char);
    fn push_str(&mut self, s: &str);
    fn clear(&mut self);
    fn truncate(&mut self, new_len: usize);
    fn len(&self) -> usize;
    fn as_str(&self) -> &str;
}

/// Counts the output without keeping it: validation without a base never
/// reads the output back.
#[derive(Debug, Default)]
struct Void {
    len: usize,
}

impl Output for Void {
    fn push(&mut self, c: char) {
        self.len += c.len_utf8();
    }
    fn push_str(&mut self, s: &str) {
        self.len += s.len();
    }
    fn clear(&mut self) {
        self.len = 0;
    }
    fn truncate(&mut self, new_len: usize) {
        self.len = new_len;
    }
    fn len(&self) -> usize {
        self.len
    }
    fn as_str(&self) -> &str {
        ""
    }
}

/// Appends to a caller's buffer after its first `start` bytes.
#[derive(Debug)]
struct Appended<'a> {
    buf: &'a mut String,
    start: usize,
}

impl Output for Appended<'_> {
    fn push(&mut self, c: char) {
        self.buf.push(c);
    }
    fn push_str(&mut self, s: &str) {
        self.buf.push_str(s);
    }
    fn clear(&mut self) {
        self.buf.truncate(self.start);
    }
    fn truncate(&mut self, new_len: usize) {
        self.buf.truncate(self.start + new_len);
    }
    fn len(&self) -> usize {
        self.buf.len() - self.start
    }
    fn as_str(&self) -> &str {
        &self.buf[self.start..]
    }
}

#[derive(Debug)]
struct Input<'a> {
    value: Chars<'a>,
    position: usize,
}

impl<'a> Input<'a> {
    fn at(iri: &'a str, position: usize) -> Self {
        Input {
            value: iri[position..].chars(),
            position,
        }
    }

    fn next(&mut self) -> Option<char> {
        let c = self.value.next()?;
        self.position += c.len_utf8();
        Some(c)
    }

    fn front(&self) -> Option<char> {
        self.value.clone().next()
    }

    fn starts_with(&self, c: char) -> bool {
        self.value.as_str().starts_with(c)
    }
}

#[derive(Debug)]
struct Parser<'a, O: Output> {
    iri: &'a str,
    base: Option<BaseRef<'a>>,
    input: Input<'a>,
    output: &'a mut O,
    positions: Positions,
    input_scheme_end: usize,
}

impl<'a, O: Output> Parser<'a, O> {
    fn parse(iri: &'a str, base: Option<BaseRef<'a>>, output: &'a mut O) -> Result<Positions, IriError> {
        let mut parser = Parser {
            iri,
            base,
            input: Input::at(iri, 0),
            output,
            positions: Positions::default(),
            input_scheme_end: 0,
        };
        parser.parse_scheme_start()?;
        Ok(parser.positions)
    }

    fn parse_scheme_start(&mut self) -> Result<(), IriError> {
        match self.input.front() {
            Some(':') => Err(IriError::NoScheme),
            Some(c) if c.is_ascii_alphabetic() => self.parse_scheme(),
            _ => self.parse_relative(),
        }
    }

    fn parse_scheme(&mut self) -> Result<(), IriError> {
        loop {
            match self.input.next() {
                Some(c) if c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.') => self.output.push(c),
                Some(':') => {
                    self.output.push(':');
                    self.positions.scheme_end = self.output.len();
                    self.input_scheme_end = self.input.position;
                    return if self.input.starts_with('/') {
                        self.input.next();
                        self.output.push('/');
                        self.parse_path_or_authority()
                    } else {
                        self.positions.authority_end = self.output.len();
                        self.parse_path::<false>()
                    };
                }
                _ => {
                    // Not a scheme after all: start again as a relative reference.
                    self.input = Input::at(self.iri, 0);
                    self.output.clear();
                    return self.parse_relative();
                }
            }
        }
    }

    fn parse_path_or_authority(&mut self) -> Result<(), IriError> {
        if self.input.starts_with('/') {
            self.input.next();
            self.output.push('/');
            self.parse_authority()
        } else {
            self.positions.authority_end = self.output.len() - 1;
            self.parse_path::<false>()
        }
    }

    fn parse_relative(&mut self) -> Result<(), IriError> {
        let Some(base) = self.base else {
            self.positions.scheme_end = 0;
            self.input_scheme_end = 0;
            return if self.input.starts_with('/') {
                self.input.next();
                self.output.push('/');
                self.parse_path_or_authority()
            } else {
                self.positions.authority_end = 0;
                self.parse_relative_path::<false>()
            };
        };
        let bp = base.positions;
        match self.input.front() {
            None => {
                self.output.push_str(&base.iri[..bp.query_end]);
                self.positions = bp;
                Ok(())
            }
            Some('/') => {
                self.input.next();
                self.parse_relative_slash(base)
            }
            Some('?') => {
                self.input.next();
                self.output.push_str(&base.iri[..bp.path_end]);
                self.output.push('?');
                self.positions.scheme_end = bp.scheme_end;
                self.positions.authority_end = bp.authority_end;
                self.positions.path_end = bp.path_end;
                self.parse_query()
            }
            Some('#') => {
                self.input.next();
                self.output.push_str(&base.iri[..bp.query_end]);
                self.positions = bp;
                self.output.push('#');
                self.parse_fragment()
            }
            _ => {
                self.output.push_str(&base.iri[..bp.path_end]);
                self.positions.scheme_end = bp.scheme_end;
                self.positions.authority_end = bp.authority_end;
                self.positions.path_end = bp.path_end;
                self.remove_last_segment();
                self.parse_relative_path::<true>()
            }
        }
    }

    fn parse_relative_path<const REMOVE_DOT_SEGMENTS: bool>(&mut self) -> Result<(), IriError> {
        while let Some(c) = self.input.front() {
            if matches!(c, '/' | '?' | '#') {
                break;
            }
            self.input.next();
            self.read_url_codepoint_or_echar(c, |c| is_iunreserved_or_sub_delims(c) || c == '@')?;
        }
        self.parse_path::<REMOVE_DOT_SEGMENTS>()
    }

    fn parse_relative_slash(&mut self, base: BaseRef<'_>) -> Result<(), IriError> {
        let bp = base.positions;
        if self.input.starts_with('/') {
            self.input.next();
            self.output.push_str(&base.iri[..bp.scheme_end]);
            self.positions.scheme_end = bp.scheme_end;
            self.output.push('/');
            self.output.push('/');
            self.parse_authority()
        } else {
            self.output.push_str(&base.iri[..bp.authority_end]);
            self.output.push('/');
            self.positions.scheme_end = bp.scheme_end;
            self.positions.authority_end = bp.authority_end;
            self.parse_path::<true>()
        }
    }

    fn parse_authority(&mut self) -> Result<(), IriError> {
        // '@' is not allowed in an IRI host, so a first '@' ends the userinfo.
        loop {
            match self.input.next() {
                Some('@') => {
                    self.output.push('@');
                    return self.parse_host();
                }
                None | Some('[' | '/' | '?' | '#') => {
                    // No userinfo: read the authority again as a host.
                    self.input = Input::at(self.iri, self.input_scheme_end + 2);
                    self.output.truncate(self.positions.scheme_end + 2);
                    return self.parse_host();
                }
                Some(c) => {
                    self.read_url_codepoint_or_echar(c, |c| is_iunreserved_or_sub_delims(c) || c == ':')?;
                }
            }
        }
    }

    fn parse_host(&mut self) -> Result<(), IriError> {
        if self.input.starts_with('[') {
            // An IP literal.
            let start_position = self.input.position;
            while let Some(c) = self.input.next() {
                self.output.push(c);
                if c == ']' {
                    let ip = &self.iri[start_position + 1..self.input.position - 1];
                    if ip.starts_with(['v', 'V']) {
                        validate_ip_v_future(ip)?;
                    } else if let Err(error) = Ipv6Addr::from_str(ip) {
                        return Err(IriError::InvalidHostIp(error));
                    }
                    let c = self.input.next();
                    return match c {
                        Some(':') => {
                            self.output.push(':');
                            self.parse_port()
                        }
                        None | Some('/' | '?' | '#') => {
                            self.positions.authority_end = self.output.len();
                            self.parse_path_start(c)
                        }
                        Some(c) => Err(IriError::InvalidHostCharacter(c)),
                    };
                }
            }
            Err(IriError::InvalidHostCharacter('['))
        } else {
            loop {
                let c = self.input.next();
                match c {
                    Some(':') => {
                        self.output.push(':');
                        return self.parse_port();
                    }
                    None | Some('/' | '?' | '#') => {
                        self.positions.authority_end = self.output.len();
                        return self.parse_path_start(c);
                    }
                    Some(c) => self.read_url_codepoint_or_echar(c, is_iunreserved_or_sub_delims)?,
                }
            }
        }
    }

    fn parse_port(&mut self) -> Result<(), IriError> {
        loop {
            let c = self.input.next();
            match c {
                None | Some('/' | '?' | '#') => {
                    self.positions.authority_end = self.output.len();
                    return self.parse_path_start(c);
                }
                Some(c) if c.is_ascii_digit() => self.output.push(c),
                Some(c) => return Err(IriError::InvalidPortCharacter(c)),
            }
        }
    }

    fn parse_path_start(&mut self, c: Option<char>) -> Result<(), IriError> {
        match c {
            None => {
                self.positions.path_end = self.output.len();
                self.positions.query_end = self.output.len();
                Ok(())
            }
            Some('?') => {
                self.positions.path_end = self.output.len();
                self.output.push('?');
                self.parse_query()
            }
            Some('#') => {
                self.positions.path_end = self.output.len();
                self.positions.query_end = self.output.len();
                self.output.push('#');
                self.parse_fragment()
            }
            Some('/') => {
                self.output.push('/');
                self.parse_path::<false>()
            }
            Some(c) => {
                self.read_url_codepoint_or_echar(c, |c| is_iunreserved_or_sub_delims(c) || matches!(c, ':' | '@'))?;
                self.parse_path::<false>()
            }
        }
    }

    fn parse_path<const REMOVE_DOT_SEGMENTS: bool>(&mut self) -> Result<(), IriError> {
        loop {
            let c = self.input.next();
            match c {
                None | Some('/' | '?' | '#') => {
                    if REMOVE_DOT_SEGMENTS {
                        let path = &self.output.as_str()[self.positions.authority_end..];
                        if path.ends_with("/..") {
                            self.output.truncate(self.output.len() - 3);
                            self.remove_last_segment();
                        } else if path.ends_with("/.") || path == "." {
                            self.output.truncate(self.output.len() - 1);
                        } else if path == ".." {
                            self.output.truncate(self.output.len() - 2);
                        } else if c == Some('/') {
                            self.output.push('/');
                            continue;
                        }
                        // Removing dot segments can leave a path starting with
                        // "//", which would read as an authority.
                        if self.output.as_str()[self.positions.authority_end..].starts_with("//")
                            && self.positions.authority_end == self.positions.scheme_end
                        {
                            return Err(IriError::PathStartingWithTwoSlashes);
                        }
                    } else if c == Some('/') {
                        self.output.push('/');
                        continue;
                    }

                    match c {
                        Some('?') => {
                            self.positions.path_end = self.output.len();
                            self.output.push('?');
                            return self.parse_query();
                        }
                        Some('#') => {
                            self.positions.path_end = self.output.len();
                            self.positions.query_end = self.output.len();
                            self.output.push('#');
                            return self.parse_fragment();
                        }
                        None => {
                            self.positions.path_end = self.output.len();
                            self.positions.query_end = self.output.len();
                            return Ok(());
                        }
                        // A '/' after a dot segment was dropped: the segment
                        // already ends with one.
                        Some(_) => {}
                    }
                }
                Some(c) => {
                    self.read_url_codepoint_or_echar(c, |c| is_iunreserved_or_sub_delims(c) || matches!(c, ':' | '@'))?
                }
            }
        }
    }

    fn parse_query(&mut self) -> Result<(), IriError> {
        while let Some(c) = self.input.next() {
            if c == '#' {
                self.positions.query_end = self.output.len();
                self.output.push('#');
                return self.parse_fragment();
            }
            self.read_url_codepoint_or_echar(c, |c| {
                is_iunreserved_or_sub_delims(c)
                    || matches!(c, ':' | '@' | '/' | '?' | '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}')
            })?;
        }
        self.positions.query_end = self.output.len();
        Ok(())
    }

    fn parse_fragment(&mut self) -> Result<(), IriError> {
        while let Some(c) = self.input.next() {
            self.read_url_codepoint_or_echar(c, |c| {
                is_iunreserved_or_sub_delims(c) || matches!(c, ':' | '@' | '/' | '?')
            })?;
        }
        Ok(())
    }

    fn remove_last_segment(&mut self) {
        let authority_end = self.positions.authority_end;
        if let Some(last_slash) = self.output.as_str()[authority_end..].rfind('/') {
            self.output.truncate(authority_end + last_slash);
            self.output.push('/');
        } else {
            self.output.truncate(authority_end);
            if authority_end > self.positions.scheme_end {
                self.output.push('/');
            }
        }
    }

    fn read_url_codepoint_or_echar(&mut self, c: char, valid: impl Fn(char) -> bool) -> Result<(), IriError> {
        if valid(c) {
            self.output.push(c);
            Ok(())
        } else if c == '%' {
            self.read_echar()
        } else {
            Err(IriError::InvalidIriCodePoint(c))
        }
    }

    fn read_echar(&mut self) -> Result<(), IriError> {
        let c1 = self.input.next();
        let c2 = self.input.next();
        match (c1, c2) {
            (Some(c1), Some(c2)) if c1.is_ascii_hexdigit() && c2.is_ascii_hexdigit() => {
                self.output.push('%');
                self.output.push(c1);
                self.output.push(c2);
                Ok(())
            }
            _ => Err(IriError::InvalidPercentEncoding([Some('%'), c1, c2])),
        }
    }
}

/// `IPvFuture = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )`, with
/// oxiri's choice of the offending character.
fn validate_ip_v_future(ip: &str) -> Result<(), IriError> {
    let mut chars = ip.chars();
    let c = chars.next().ok_or(IriError::InvalidHostCharacter(']'))?;
    if !matches!(c, 'v' | 'V') {
        return Err(IriError::InvalidHostCharacter(c));
    }
    let mut with_a_version = false;
    for c in &mut chars {
        if c == '.' {
            break;
        } else if c.is_ascii_hexdigit() {
            with_a_version = true;
        } else {
            return Err(IriError::InvalidHostCharacter(c));
        }
    }
    if !with_a_version {
        return Err(IriError::InvalidHostCharacter(chars.next().unwrap_or(']')));
    }
    if chars.as_str().is_empty() {
        return Err(IriError::InvalidHostCharacter(']'));
    }
    for c in chars {
        if !is_unreserved_or_sub_delims(c) && c != ':' {
            return Err(IriError::InvalidHostCharacter(c));
        }
    }
    Ok(())
}

fn is_iunreserved_or_sub_delims(c: char) -> bool {
    matches!(c,
        'a'..='z'
        | 'A'..='Z'
        | '0'..='9'
        | '!'
        | '$'
        | '&'
        | '\''
        | '('
        | ')'
        | '*'
        | '+'
        | ','
        | '-'
        | '.'
        | ';'
        | '='
        | '_'
        | '~'
        | '\u{A0}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}'
        | '\u{FDF0}'..='\u{FFEF}'
        | '\u{10000}'..='\u{1FFFD}'
        | '\u{20000}'..='\u{2FFFD}'
        | '\u{30000}'..='\u{3FFFD}'
        | '\u{40000}'..='\u{4FFFD}'
        | '\u{50000}'..='\u{5FFFD}'
        | '\u{60000}'..='\u{6FFFD}'
        | '\u{70000}'..='\u{7FFFD}'
        | '\u{80000}'..='\u{8FFFD}'
        | '\u{90000}'..='\u{9FFFD}'
        | '\u{A0000}'..='\u{AFFFD}'
        | '\u{B0000}'..='\u{BFFFD}'
        | '\u{C0000}'..='\u{CFFFD}'
        | '\u{D0000}'..='\u{DFFFD}'
        | '\u{E1000}'..='\u{EFFFD}'
    )
}

fn is_unreserved_or_sub_delims(c: char) -> bool {
    matches!(c,
        'a'..='z'
        | 'A'..='Z'
        | '0'..='9'
        | '!'
        | '$'
        | '&'
        | '\''
        | '('
        | ')'
        | '*'
        | '+'
        | ','
        | '-'
        | '.'
        | ';'
        | '='
        | '_'
        | '~'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use oxrdf::{NamedNodeRef, NamedOrBlankNode};
    use oxttl::TurtleParser;

    fn resolve(base: &str, reference: &str) -> Result<String, String> {
        let mut out = String::new();
        Base::new(base).unwrap().resolve_into(reference, &mut out).map(|()| out)
    }

    /// The skip over plain bytes stops exactly where the byte classes say.
    #[test]
    fn skip_plain_skips_only_plain_words() {
        let plain = |b: u8| b.is_ascii_alphanumeric() || b"&'()*+,-./:;@_".contains(&b);
        for b in 0..=255u8 {
            for at in 0..8 {
                let mut word = *b"abcdefgh";
                word[at] = b;
                let text = [&word[..], b"/a/b/c/d", b"%zz"].concat();
                let skipped = skip_plain(&text, 0);
                let expected = if plain(b) { 16 } else { 0 };
                assert_eq!(skipped, expected, "byte {b:#x} at {at}");
                assert!(text[..skipped].iter().all(|&b| CLASS[b as usize] & PATH != 0));
            }
        }
    }

    #[test]
    fn rfc3986_normal_examples() {
        let base = "http://a/b/c/d;p?q";
        for (reference, expected) in [
            ("g:h", "g:h"),
            ("g", "http://a/b/c/g"),
            ("./g", "http://a/b/c/g"),
            ("g/", "http://a/b/c/g/"),
            ("/g", "http://a/g"),
            ("//g", "http://g"),
            ("?y", "http://a/b/c/d;p?y"),
            ("g?y", "http://a/b/c/g?y"),
            ("#s", "http://a/b/c/d;p?q#s"),
            ("g#s", "http://a/b/c/g#s"),
            ("g?y#s", "http://a/b/c/g?y#s"),
            (";x", "http://a/b/c/;x"),
            ("g;x", "http://a/b/c/g;x"),
            ("g;x?y#s", "http://a/b/c/g;x?y#s"),
            ("", "http://a/b/c/d;p?q"),
            (".", "http://a/b/c/"),
            ("./", "http://a/b/c/"),
            ("..", "http://a/b/"),
            ("../", "http://a/b/"),
            ("../g", "http://a/b/g"),
            ("../..", "http://a/"),
            ("../../", "http://a/"),
            ("../../g", "http://a/g"),
        ] {
            assert_eq!(resolve(base, reference).as_deref(), Ok(expected), "{reference}");
        }
    }

    #[test]
    fn rfc3986_abnormal_examples() {
        let base = "http://a/b/c/d;p?q";
        for (reference, expected) in [
            ("../../../g", "http://a/g"),
            ("../../../../g", "http://a/g"),
            ("/./g", "http://a/g"),
            ("/../g", "http://a/g"),
            ("g.", "http://a/b/c/g."),
            (".g", "http://a/b/c/.g"),
            ("g..", "http://a/b/c/g.."),
            ("..g", "http://a/b/c/..g"),
            ("./../g", "http://a/b/g"),
            ("./g/.", "http://a/b/c/g/"),
            ("g/./h", "http://a/b/c/g/h"),
            ("g/../h", "http://a/b/c/h"),
            ("g;x=1/./y", "http://a/b/c/g;x=1/y"),
            ("g;x=1/../y", "http://a/b/c/y"),
            ("g?y/./x", "http://a/b/c/g?y/./x"),
            ("g?y/../x", "http://a/b/c/g?y/../x"),
            ("g#s/./x", "http://a/b/c/g#s/./x"),
            ("g#s/../x", "http://a/b/c/g#s/../x"),
            ("http:g", "http:g"),
        ] {
            assert_eq!(resolve(base, reference).as_deref(), Ok(expected), "{reference}");
        }
    }

    #[test]
    fn query_fragment_and_empty_references() {
        assert_eq!(
            resolve("http://example.org/a?q#f", "").unwrap(),
            "http://example.org/a?q"
        );
        assert_eq!(
            resolve("http://example.org/a?q#f", "#").unwrap(),
            "http://example.org/a?q#"
        );
        assert_eq!(
            resolve("http://example.org/a?q#f", "?").unwrap(),
            "http://example.org/a?"
        );
        assert_eq!(resolve("http://example.org", "#x").unwrap(), "http://example.org#x");
        assert_eq!(resolve("http://example.org", "x").unwrap(), "http://example.org/x");
        assert_eq!(resolve("urn:a:b", "c").unwrap(), "urn:c");
        assert_eq!(
            resolve("http://example.org/a", "#a#b").unwrap_err(),
            "Invalid IRI code point '#'"
        );
    }

    #[test]
    fn absolute_references_keep_dot_segments() {
        let base = "http://example.org/a/b";
        assert_eq!(
            resolve(base, "http://example.org/x/../y/./z").unwrap(),
            "http://example.org/x/../y/./z"
        );
        assert_eq!(resolve(base, "../y/./z").unwrap(), "http://example.org/y/z");
        // A base keeps its dot segments too; a relative reference removes the
        // ones it reaches.
        assert_eq!(resolve("a:/x/./y", "z").unwrap(), "a:/x/./z");
    }

    #[test]
    fn network_path_references() {
        let base = "http://example.org/a/b";
        assert_eq!(resolve(base, "//example.com").unwrap(), "http://example.com");
        assert_eq!(resolve(base, "//u@h:8/p/../q").unwrap(), "http://u@h:8/p/../q");
        assert_eq!(resolve(base, "//[::1]/x").unwrap(), "http://[::1]/x");
        assert_eq!(resolve(base, "//a b").unwrap_err(), "Invalid IRI code point ' '");
    }

    #[test]
    fn path_starting_with_two_slashes() {
        // Removing "/." in a base without an authority leaves "//g".
        assert_eq!(
            resolve("a:b", "/.//g").unwrap_err(),
            "An IRI path is not allowed to start with //"
        );
        // With an authority the same path is fine.
        assert_eq!(
            resolve("http://example.org/b", "/.//g").unwrap(),
            "http://example.org//g"
        );
        // Parsing alone never reports it.
        assert_eq!(validate("a:.//g"), Ok(()));
    }

    #[test]
    fn hosts_and_ports() {
        for iri in [
            "http://[::1]",
            "http://[::1]:80/p",
            "http://[2001:db8::7]/",
            "http://[v1.x]/",
            "http://[V7.a:b]/",
            "http://u:p@example.org:8080/",
            "http://example.org:/",
            "http://%41.example.org/",
            "http:///p",
            "a://",
        ] {
            assert_eq!(validate(iri), Ok(()), "{iri}");
        }
        for (iri, message) in [
            ("http://[::1", "Invalid character '[' in host"),
            ("http://[::1]x", "Invalid character 'x' in host"),
            ("http://[:::1]/", "Invalid host IP (invalid IPv6 address syntax)"),
            ("http://[v.x]/", "Invalid character 'x' in host"),
            ("http://[v1]/", "Invalid character ']' in host"),
            ("http://[v1.]/", "Invalid character ']' in host"),
            ("http://[vg.x]/", "Invalid character 'g' in host"),
            ("http://[v1.x/]/", "Invalid character '/' in host"),
            ("http://example.org:8a/", "Invalid character 'a'"),
            ("http://a@b@c/", "Invalid IRI code point '@'"),
            ("http://exa mple.org/", "Invalid IRI code point ' '"),
        ] {
            assert_eq!(validate(iri).unwrap_err(), message, "{iri}");
        }
    }

    #[test]
    fn percent_encodings_and_code_points() {
        assert_eq!(validate("http://example.org/%2Fa%c3%A9"), Ok(()));
        assert_eq!(validate("http://example.org/é?\u{E000}#ü"), Ok(()));
        for (iri, message) in [
            ("http://example.org/%zz", "Invalid IRI percent encoding '%zz'"),
            ("http://example.org/%2", "Invalid IRI percent encoding '%2'"),
            ("http://example.org/%", "Invalid IRI percent encoding '%'"),
            ("http://example.org/a b", "Invalid IRI code point ' '"),
            ("http://example.org/\u{E000}", "Invalid IRI code point '\u{E000}'"),
            ("http://example.org/a#b#c", "Invalid IRI code point '#'"),
            ("http://example.org/a#\u{E000}", "Invalid IRI code point '\u{E000}'"),
            ("http://example.org/<", "Invalid IRI code point '<'"),
            ("example", "No scheme found in an absolute IRI"),
            ("", "No scheme found in an absolute IRI"),
            (":x", "No scheme found in an absolute IRI"),
            ("/a b", "Invalid IRI code point ' '"),
            ("1a:b", "Invalid IRI code point ':'"),
        ] {
            assert_eq!(validate(iri).unwrap_err(), message, "{iri}");
        }
    }

    #[test]
    fn base_errors_and_appending() {
        assert_eq!(Base::new("example").unwrap_err(), "No scheme found in an absolute IRI");
        assert_eq!(Base::new("http://example.org/a").unwrap().iri, "http://example.org/a");
        let base = Base::new("http://example.org/a/b").unwrap();
        let mut out = String::from("<");
        base.resolve_into("../c?d", &mut out).unwrap();
        assert_eq!(out, "<http://example.org/c?d");
        // A reference that is not a scheme after all is relative.
        let mut out = String::from("x");
        base.resolve_into("c+d/e:f", &mut out).unwrap();
        assert_eq!(out, "xhttp://example.org/a/c+d/e:f");
        assert_eq!(
            base.resolve_into("c d:e", &mut out).unwrap_err(),
            "Invalid IRI code point ' '"
        );
    }

    #[test]
    fn empty_paths() {
        assert!(has_empty_path("http://example.org"));
        assert!(!has_empty_path("http://example.org/"));
        assert!(!has_empty_path("urn:x"));
        assert!(has_empty_path("http://example.org?q"));
        assert!(has_empty_path("http://example.org#f"));
        assert!(has_empty_path("urn:"));
        assert!(has_empty_path("urn:?q"));
        assert!(!has_empty_path("a:/"));
        assert!(has_empty_path("http://[::1]:80"));
    }

    /// A deterministic xorshift generator.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }

        fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
            items[self.below(items.len())]
        }
    }

    /// Pieces of generated IRIs, weighted towards the characters each part
    /// of the grammar treats specially.
    #[rustfmt::skip]
    const PIECES: &[&str] = &[
        ":", "/", "?", "#", "[", "]", "@", "%", ".", "-", "_", "~", "!", "$", "&", "'", "(", ")", "*",
        "+", ",", ";", "=", ":", "/", "/", "?", "#", "%", ".", "a", "b", "v", "V", "x", "Z", "0", "1",
        "9", "f", "A", "e", " ", "<", ">", "\"", "{", "\u{A0}", "\u{E000}", "\u{FFFF}", "\u{10000}",
        "\u{F0000}", "\u{E1000}", "\u{FDD0}", "http://", "urn:", "a:", "//", "[::1]", "[v1.x]", "%41",
        "%2f", "..", "./", "../", ":80", "example.org", "u@",
    ];

    #[rustfmt::skip]
    const STARTS: &[&str] = &[
        "", "", "http://", "http://example.org", "http://example.org/", "urn:", "a:", "//", "/", "a:/",
        "http://[",
    ];

    fn generate(rng: &mut Rng, starts: &[&str], pieces: &[&str], max: usize) -> String {
        let mut s = String::from(rng.pick(starts));
        for _ in 0..rng.below(max + 1) {
            s.push_str(rng.pick(pieces));
        }
        s
    }

    #[test]
    fn validate_matches_oxiri() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let mut accepted = 0;
        for _ in 0..500_000 {
            let iri = generate(&mut rng, STARTS, PIECES, 8);
            let ours = validate(&iri);
            let theirs = NamedNodeRef::new(&iri).map(|_| ()).map_err(|e| e.to_string());
            assert_eq!(ours, theirs, "{iri:?}");
            if ours.is_ok() {
                accepted += 1;
                // The parsed positions are oxiri's: the scan must agree.
                let positions = parse_absolute(&iri).unwrap();
                assert_eq!(
                    has_empty_path(&iri),
                    positions.authority_end == positions.path_end,
                    "{iri:?}"
                );
            }
        }
        assert!(accepted > 50_000, "only {accepted} accepted");
    }

    #[rustfmt::skip]
    const REF_PIECES: &[&str] = &[
        ".", "..", "/", "/", "/", "./", "../", "?", "#", "g", "h", ":", "@", "%41", "%", "//", "[::1]",
        "[v1.x]", " ", "\u{E000}", "\u{A0}", "a:", ";", "=", "x", "<", "\\", "^", "|", "\u{10000}",
        "[:::1]", "[v.x]", "u@", ":8", "\u{FFFF}",
    ];

    #[rustfmt::skip]
    const REF_STARTS: &[&str] = &[
        "", "", "", "", "/", "//", "?", "#", ".", "..", "http://example.org/", "a:", "g:",
    ];

    #[rustfmt::skip]
    const BASES: &[&str] = &[
        "http://a/b/c/d;p?q", "http://example.org", "http://example.org/", "http://example.org/a/b?q#f",
        "http://u@[::1]:80/p/q", "urn:x:y", "a:b", "a:", "a:/", "a:/b/c", "a:b/c/d", "file:///tmp/x",
        "a://", "http://example.org?q", "tag:a/b/../c/./d",
    ];

    /// Writes `reference` as a Turtle IRIREF body that oxttl decodes back to
    /// `reference` before handing it to oxiri.
    fn turtle_iri(reference: &str) -> String {
        let mut out = String::new();
        for c in reference.chars() {
            if c <= ' ' || matches!(c, '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\') {
                out.push_str(&format!("\\u{:04X}", u32::from(c)));
            } else {
                out.push(c);
            }
        }
        out
    }

    fn oxttl_resolve(base: &str, reference: &str) -> Result<String, String> {
        let document = format!(
            "<{}> <http://example.org/p> <http://example.org/o> .",
            turtle_iri(reference)
        );
        let parser = TurtleParser::new().with_base_iri(base).unwrap();
        match parser.for_slice(document.as_bytes()).next().unwrap() {
            Ok(triple) => match triple.subject {
                NamedOrBlankNode::NamedNode(n) => Ok(n.into_string()),
                NamedOrBlankNode::BlankNode(b) => panic!("blank node {b}"),
            },
            Err(e) => Err(e.message().to_owned()),
        }
    }

    #[test]
    fn resolve_matches_oxttl() {
        let bases: Vec<Base> = BASES.iter().map(|b| Base::new(b).unwrap()).collect();
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        let mut accepted = 0;
        let mut out = String::new();
        for _ in 0..250_000 {
            let i = rng.below(BASES.len());
            let reference = generate(&mut rng, REF_STARTS, REF_PIECES, 6);
            out.clear();
            let ours = bases[i].resolve_into(&reference, &mut out).map(|()| out.clone());
            let theirs = oxttl_resolve(BASES[i], &reference);
            assert_eq!(ours, theirs, "base {:?}, reference {reference:?}", BASES[i]);
            accepted += usize::from(ours.is_ok());
        }
        assert!(accepted > 60_000, "only {accepted} accepted");
    }
}
