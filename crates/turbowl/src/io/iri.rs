//! IRI validation and resolution for the Turtle and N-Triples parsers.
//!
//! oxttl validates and resolves IRIs with oxiri 0.2.11, so parity with oxiri
//! is the contract: the same strings accepted, the same resolutions, the same
//! error messages. This module calls oxiri itself ([`oxiri::Iri::parse`] and
//! [`oxiri::Iri::resolve_into`]), with a scan over bytes in front for the
//! common shape (ASCII, a scheme, an optional plain host and port, a path, a
//! query, a fragment). The scan only accepts; anything it does not accept goes
//! to oxiri, which decides and gives the message.

use oxiri::Iri;

/// Validates an absolute IRI exactly as `oxiri::Iri::parse` 0.2.11 does.
/// Err holds the message of oxiri's `IriParseError` (its Display).
pub(crate) fn validate(iri: &str) -> Result<(), String> {
    if is_plain_absolute(iri.as_bytes()) {
        return Ok(());
    }
    parse(iri)
}

/// oxiri's verdict, out of the scan's hot path.
#[cold]
#[inline(never)]
fn parse(iri: &str) -> Result<(), String> {
    Iri::parse(iri).map(|_| ()).map_err(|e| e.to_string())
}

/// True when the path of `iri`, a valid absolute IRI, is empty, as oxttl
/// decides whether a prefixed name must be validated again.
pub(crate) fn has_empty_path(iri: &str) -> bool {
    Iri::parse_unchecked(iri).path().is_empty()
}

/// A base IRI: an absolute, valid IRI, parsed once.
#[derive(Debug, Clone)]
pub(crate) struct Base(Iri<String>);

impl Base {
    /// As `oxiri::Iri::parse`: Err holds oxiri's message.
    pub(crate) fn new(iri: &str) -> Result<Base, String> {
        Iri::parse(iri.to_owned()).map(Base).map_err(|e| e.to_string())
    }

    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Appends `reference` resolved against this base to `out`, as
    /// `oxiri::Iri::resolve` 0.2.11 does; Err holds oxiri's message. oxiri
    /// writes into `scratch` (it clears its buffer first), which is then
    /// appended; on Err, `out` is unchanged.
    pub(crate) fn resolve_into(&self, reference: &str, out: &mut String, scratch: &mut String) -> Result<(), String> {
        // A reference with a scheme never reads the base, and oxiri copies it
        // unchanged (no dot segments are removed): the scan's accepting it is
        // enough.
        if is_plain_absolute(reference.as_bytes()) {
            out.push_str(reference);
            return Ok(());
        }
        scratch.clear();
        self.0.resolve_into(reference, scratch).map_err(|e| e.to_string())?;
        out.push_str(scratch);
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    use oxrdf::{NamedNodeRef, NamedOrBlankNode};
    use oxttl::TurtleParser;

    fn resolve(base: &str, reference: &str) -> Result<String, String> {
        let mut out = String::new();
        Base::new(base)
            .unwrap()
            .resolve_into(reference, &mut out, &mut String::new())
            .map(|()| out)
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

    /// The scan's shortcut for references with a scheme: copied as they are.
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

    /// The scan rejects what it must leave to oxiri, which gives the message.
    #[test]
    fn hosts_ports_and_percent_encodings() {
        for iri in [
            "http://[::1]",
            "http://[::1]:80/p",
            "http://[v1.x]/",
            "http://u:p@example.org:8080/",
            "http://example.org:/",
            "http://%41.example.org/",
            "http:///p",
            "a://",
            "http://example.org/%2Fa%c3%A9",
            "http://example.org/é?\u{E000}#ü",
        ] {
            assert_eq!(validate(iri), Ok(()), "{iri}");
        }
        for (iri, message) in [
            ("http://[::1", "Invalid character '[' in host"),
            ("http://[:::1]/", "Invalid host IP (invalid IPv6 address syntax)"),
            ("http://example.org:8a/", "Invalid character 'a'"),
            ("http://a@b@c/", "Invalid IRI code point '@'"),
            ("http://exa mple.org/", "Invalid IRI code point ' '"),
            ("http://example.org/%zz", "Invalid IRI percent encoding '%zz'"),
            ("http://example.org/%2", "Invalid IRI percent encoding '%2'"),
            ("http://example.org/a#b#c", "Invalid IRI code point '#'"),
            ("http://example.org/<", "Invalid IRI code point '<'"),
            ("example", "No scheme found in an absolute IRI"),
            ("1a:b", "Invalid IRI code point ':'"),
        ] {
            assert_eq!(validate(iri).unwrap_err(), message, "{iri}");
        }
    }

    #[test]
    fn base_errors_and_appending() {
        assert_eq!(Base::new("example").unwrap_err(), "No scheme found in an absolute IRI");
        let base = Base::new("http://example.org/a/b").unwrap();
        let mut scratch = String::from("left over");
        let mut out = String::from("<");
        base.resolve_into("../c?d", &mut out, &mut scratch).unwrap();
        assert_eq!(out, "<http://example.org/c?d");
        // A reference that is not a scheme after all is relative.
        let mut out = String::from("x");
        base.resolve_into("c+d/e:f", &mut out, &mut scratch).unwrap();
        assert_eq!(out, "xhttp://example.org/a/c+d/e:f");
        assert_eq!(
            base.resolve_into("c d:e", &mut out, &mut scratch).unwrap_err(),
            "Invalid IRI code point ' '"
        );
        assert_eq!(out, "xhttp://example.org/a/c+d/e:f", "unchanged on an error");
        assert!(has_empty_path("http://example.org?q"));
        assert!(!has_empty_path("http://example.org/"));
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
            accepted += usize::from(ours.is_ok());
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
        let (mut out, mut scratch) = (String::new(), String::new());
        for _ in 0..250_000 {
            let i = rng.below(BASES.len());
            let reference = generate(&mut rng, REF_STARTS, REF_PIECES, 6);
            out.clear();
            let ours = bases[i]
                .resolve_into(&reference, &mut out, &mut scratch)
                .map(|()| out.clone());
            let theirs = oxttl_resolve(BASES[i], &reference);
            assert_eq!(ours, theirs, "base {:?}, reference {reference:?}", BASES[i]);
            accepted += usize::from(ours.is_ok());
        }
        assert!(accepted > 60_000, "only {accepted} accepted");
    }
}
