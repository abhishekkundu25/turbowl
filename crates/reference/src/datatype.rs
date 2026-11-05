//! The OWL 2 RL datatype map (OWL 2 Profiles §4.2, OWL 2 Structural
//! Specification §4), written from the specifications without the core's
//! code: every datatype's lexical space, its lexical-to-value mapping, and
//! its value space. The core has its own (`crates/turbowl/src/rules/datatype/`);
//! chaos holds the two to each other.
//!
//! The approach is deliberately plain, and differs from the core's:
//!
//! - every real number is a fraction in lowest terms, computed on decimal
//!   digit strings ([`Big`]): `"1.50"^^xsd:decimal` is 3/2, and so is
//!   `"6/4"^^owl:rational`;
//! - binary values are decoded to their octets; a base64 form is
//!   well-formed when re-encoding its octets gives it back;
//! - date-times are kept as fields, with `24:00:00` moved to the next day.
//!
//! dt-not-type fires for `lt rdf:type dt` when the literal `lt` is not in the
//! value space of `dt`. A literal is checked in two ways: against its own
//! datatype (an ill-typed literal, [`well_formed`]), and against any datatype
//! of the map the closure types it with, by prp-rng or any other rule
//! ([`in_value_space`]).
//!
//! Choices, as the core makes them: `rdfs:Literal` is the top data range, a
//! space with every value but no lexical forms read (its literals have an
//! unknown value); `owl:real` has no lexical form, so its literals are
//! ill-typed; any string is an `xsd:string` (version 1's reading);
//! `rdf:XMLLiteral` is checked for balanced tags only; `rdf:langString` is in
//! the map as version 1 had it.

use oxrdf::Literal;
use std::cmp::Ordering;

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const RDFS: &str = "http://www.w3.org/2000/01/rdf-schema#";
const OWL: &str = "http://www.w3.org/2002/07/owl#";

/// The primitive a datatype reads its lexical forms with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Base {
    /// `rdfs:Literal`: no lexical form is read.
    Top,
    /// `owl:real`: no lexical form.
    NoForms,
    Integer,
    Decimal,
    Rational,
    Double,
    Float,
    String,
    PlainLiteral,
    LangString,
    Xml,
    Boolean,
    Hex,
    Base64,
    AnyUri,
    DateTime,
}

/// What the value space holds beyond the primitive's kind of value.
#[derive(Debug, Clone, Copy)]
enum Facet {
    None,
    /// Integers from the first bound to the second, as decimal strings (a
    /// bound may be absent).
    Range(Option<&'static str>, Option<&'static str>),
    /// Strings of a pattern.
    Pattern(fn(&str) -> bool),
    /// Date-times with a time zone.
    Zoned,
}

/// The map: the namespace and local name, the primitive, the facet.
#[rustfmt::skip]
const MAP: &[(&str, &str, Base, Facet)] = &[
    (RDFS, "Literal", Base::Top, Facet::None),
    (RDF, "PlainLiteral", Base::PlainLiteral, Facet::None),
    (RDF, "langString", Base::LangString, Facet::None),
    (RDF, "XMLLiteral", Base::Xml, Facet::None),
    (OWL, "real", Base::NoForms, Facet::None),
    (OWL, "rational", Base::Rational, Facet::None),
    (XSD, "decimal", Base::Decimal, Facet::None),
    (XSD, "integer", Base::Integer, Facet::None),
    (XSD, "nonNegativeInteger", Base::Integer, Facet::Range(Some("0"), None)),
    (XSD, "nonPositiveInteger", Base::Integer, Facet::Range(None, Some("0"))),
    (XSD, "positiveInteger", Base::Integer, Facet::Range(Some("1"), None)),
    (XSD, "negativeInteger", Base::Integer, Facet::Range(None, Some("-1"))),
    (XSD, "long", Base::Integer, Facet::Range(Some("-9223372036854775808"), Some("9223372036854775807"))),
    (XSD, "int", Base::Integer, Facet::Range(Some("-2147483648"), Some("2147483647"))),
    (XSD, "short", Base::Integer, Facet::Range(Some("-32768"), Some("32767"))),
    (XSD, "byte", Base::Integer, Facet::Range(Some("-128"), Some("127"))),
    (XSD, "unsignedLong", Base::Integer, Facet::Range(Some("0"), Some("18446744073709551615"))),
    (XSD, "unsignedInt", Base::Integer, Facet::Range(Some("0"), Some("4294967295"))),
    (XSD, "unsignedShort", Base::Integer, Facet::Range(Some("0"), Some("65535"))),
    (XSD, "unsignedByte", Base::Integer, Facet::Range(Some("0"), Some("255"))),
    (XSD, "double", Base::Double, Facet::None),
    (XSD, "float", Base::Float, Facet::None),
    (XSD, "string", Base::String, Facet::None),
    (XSD, "normalizedString", Base::String, Facet::Pattern(is_normalized)),
    (XSD, "token", Base::String, Facet::Pattern(is_token)),
    (XSD, "language", Base::String, Facet::Pattern(is_language)),
    (XSD, "Name", Base::String, Facet::Pattern(is_name)),
    (XSD, "NCName", Base::String, Facet::Pattern(is_ncname)),
    (XSD, "NMTOKEN", Base::String, Facet::Pattern(is_nmtoken)),
    (XSD, "boolean", Base::Boolean, Facet::None),
    (XSD, "hexBinary", Base::Hex, Facet::None),
    (XSD, "base64Binary", Base::Base64, Facet::None),
    (XSD, "anyURI", Base::AnyUri, Facet::None),
    (XSD, "dateTime", Base::DateTime, Facet::None),
    (XSD, "dateTimeStamp", Base::DateTime, Facet::Zoned),
];

/// Every datatype IRI of the map.
pub(crate) fn datatype_iris() -> impl Iterator<Item = String> {
    MAP.iter().map(|(ns, local, _, _)| format!("{ns}{local}"))
}

fn lookup(iri: &str) -> Option<(Base, Facet)> {
    MAP.iter()
        .find(|(ns, local, _, _)| iri.strip_prefix(ns) == Some(local))
        .map(|&(_, _, base, facet)| (base, facet))
}

/// A data value. `==` is the identity of data values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Value {
    /// A real: sign, numerator and denominator in lowest terms.
    Real {
        negative: bool,
        num: Big,
        den: Big,
    },
    Double(u64),
    Float(u32),
    String(String),
    /// Text and the tag in lower case.
    LangString(String, String),
    Xml(String),
    Boolean(bool),
    Hex(Vec<u8>),
    Base64(Vec<u8>),
    AnyUri(String),
    DateTime(DateTime),
}

/// What the map says of a literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reading {
    /// Its datatype is not in the map, or is `rdfs:Literal`.
    Unknown,
    IllTyped,
    Value(Value),
}

/// The value of a literal.
pub(crate) fn read(literal: &Literal) -> Reading {
    let datatype = literal.datatype();
    let Some((base, facet)) = lookup(datatype.as_str()) else {
        return Reading::Unknown;
    };
    let s = literal.value();
    let value = match base {
        Base::Top => return Reading::Unknown,
        Base::NoForms => None,
        Base::Integer => parse_integer(s),
        Base::Decimal => parse_decimal(s),
        Base::Rational => parse_rational(s),
        Base::Double => parse_double(s).map(Value::Double),
        Base::Float => parse_float(s).map(Value::Float),
        Base::String => Some(Value::String(s.to_owned())),
        Base::PlainLiteral => parse_plain_literal(s),
        Base::LangString => literal
            .language()
            .map(|tag| Value::LangString(s.to_owned(), tag.to_ascii_lowercase())),
        Base::Xml => xml(s).then(|| Value::Xml(s.to_owned())),
        Base::Boolean => match s {
            "true" | "1" => Some(Value::Boolean(true)),
            "false" | "0" => Some(Value::Boolean(false)),
            _ => None,
        },
        Base::Hex => parse_hex(s).map(Value::Hex),
        Base::Base64 => parse_base64(s).map(Value::Base64),
        Base::AnyUri => Some(Value::AnyUri(s.to_owned())),
        Base::DateTime => parse_date_time(s).map(Value::DateTime),
    };
    match value {
        Some(v) if space_holds(base, facet, &v) => Reading::Value(v),
        _ => Reading::IllTyped,
    }
}

/// Whether the value space of the datatype `iri` holds `value`, or `None`
/// when the datatype is not in the map.
pub(crate) fn holds(iri: &str, value: &Value) -> Option<bool> {
    let (base, facet) = lookup(iri)?;
    Some(space_holds(base, facet, value))
}

/// Every datatype of the map whose value space holds `value`.
#[allow(dead_code)] // dt-type2 (ticket 10) reads it
pub(crate) fn datatypes(value: &Value) -> Vec<String> {
    datatype_iris().filter(|iri| holds(iri, value) == Some(true)).collect()
}

fn space_holds(base: Base, facet: Facet, value: &Value) -> bool {
    let kind = match (base, value) {
        (Base::Top, _) => true,
        (Base::NoForms | Base::Rational, Value::Real { .. }) => true,
        (Base::Decimal, Value::Real { den, .. }) => only_twos_and_fives(den),
        (Base::Integer, Value::Real { den, .. }) => den.is_one(),
        (Base::Double, Value::Double(_)) | (Base::Float, Value::Float(_)) => true,
        (Base::String, Value::String(_)) => true,
        (Base::PlainLiteral, Value::String(_) | Value::LangString(..)) => true,
        (Base::LangString, Value::LangString(..)) => true,
        (Base::Xml, Value::Xml(_)) | (Base::Boolean, Value::Boolean(_)) => true,
        (Base::Hex, Value::Hex(_)) | (Base::Base64, Value::Base64(_)) => true,
        (Base::AnyUri, Value::AnyUri(_)) | (Base::DateTime, Value::DateTime(_)) => true,
        _ => false,
    };
    kind && match (facet, value) {
        (Facet::None, _) => true,
        (Facet::Range(min, max), Value::Real { negative, num, .. }) => {
            let v = (*negative, num);
            min.is_none_or(|m| compare_signed(v, m) != Ordering::Less)
                && max.is_none_or(|m| compare_signed(v, m) != Ordering::Greater)
        }
        (Facet::Pattern(p), Value::String(s)) => p(s),
        (Facet::Zoned, Value::DateTime(d)) => d.zone.is_some(),
        _ => false,
    }
}

/// A signed integer against a bound written in decimal.
fn compare_signed((negative, magnitude): (bool, &Big), bound: &str) -> Ordering {
    let (bound_negative, digits) = match bound.strip_prefix('-') {
        Some(d) => (true, d),
        None => (false, bound),
    };
    let bound = Big::parse(digits);
    let negative = negative && !magnitude.is_zero();
    match (negative, bound_negative) {
        (false, true) => Ordering::Greater,
        (true, false) => Ordering::Less,
        (false, false) => magnitude.cmp(&bound),
        (true, true) => bound.cmp(magnitude),
    }
}

/// Whether `literal` is well-formed for its own datatype: `Some(false)`
/// when ill-typed, `None` when its datatype is not in the map.
pub(crate) fn well_formed(literal: &Literal) -> Option<bool> {
    match read(literal) {
        Reading::Unknown => None,
        Reading::IllTyped => Some(false),
        Reading::Value(_) => Some(true),
    }
}

/// Whether the value of `literal` is in the value space of `datatype`, or
/// `None` when the literal's value or the datatype's space is unknown here.
/// An ill-typed literal has no value, so it is in no space.
pub(crate) fn in_value_space(literal: &Literal, datatype: &str) -> Option<bool> {
    lookup(datatype)?;
    match read(literal) {
        Reading::Unknown => None,
        Reading::IllTyped => Some(false),
        Reading::Value(v) => holds(datatype, &v),
    }
}

// ---------------------------------------------------------------------------
// Numbers
// ---------------------------------------------------------------------------

/// A natural number as decimal digits, most significant first, without
/// leading zeros (zero is empty).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Big(Vec<u8>);

impl Big {
    fn parse(digits: &str) -> Big {
        let mut b = Big(digits.bytes().map(|c| c - b'0').collect());
        b.trim();
        b
    }

    fn trim(&mut self) {
        let zeros = self.0.iter().take_while(|&&d| d == 0).count();
        self.0.drain(..zeros);
    }

    fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    fn is_one(&self) -> bool {
        self.0 == [1]
    }

    /// `self - other`, for `other <= self`.
    fn sub(&self, other: &Big) -> Big {
        let mut out = self.0.clone();
        let mut borrow = 0i8;
        for i in 0..out.len() {
            let at = out.len() - 1 - i;
            let o = if i < other.0.len() {
                other.0[other.0.len() - 1 - i] as i8
            } else {
                0
            };
            let mut d = out[at] as i8 - o - borrow;
            borrow = i8::from(d < 0);
            if d < 0 {
                d += 10;
            }
            out[at] = d as u8;
        }
        let mut b = Big(out);
        b.trim();
        b
    }

    fn add_one(&self) -> Big {
        let mut out = self.0.clone();
        let mut i = out.len();
        loop {
            if i == 0 {
                out.insert(0, 1);
                break;
            }
            i -= 1;
            if out[i] == 9 {
                out[i] = 0;
            } else {
                out[i] += 1;
                break;
            }
        }
        Big(out)
    }

    /// Quotient and remainder, by long division in base 10.
    fn divmod(&self, d: &Big) -> (Big, Big) {
        let mut q = Vec::new();
        let mut r = Big(Vec::new());
        for &digit in &self.0 {
            r.0.push(digit);
            r.trim();
            let mut count = 0;
            while r >= *d {
                r = r.sub(d);
                count += 1;
            }
            q.push(count);
        }
        let mut q = Big(q);
        q.trim();
        (q, r)
    }

    fn gcd(a: &Big, b: &Big) -> Big {
        let (mut a, mut b) = (a.clone(), b.clone());
        while !b.is_zero() {
            let r = a.divmod(&b).1;
            a = b;
            b = r;
        }
        a
    }

    fn rem_small(&self, d: u32) -> u32 {
        self.0.iter().fold(0, |r, &digit| (r * 10 + u32::from(digit)) % d)
    }
}

impl PartialOrd for Big {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Big {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.len().cmp(&other.0.len()).then_with(|| self.0.cmp(&other.0))
    }
}

fn only_twos_and_fives(den: &Big) -> bool {
    let mut d = den.clone();
    for p in [2, 5] {
        let p_big = Big::parse(&p.to_string());
        while !d.is_zero() && d.rem_small(p) == 0 {
            d = d.divmod(&p_big).0;
        }
    }
    d.is_one()
}

/// The real `num / den` in lowest terms (den nonzero).
fn real(negative: bool, num: Big, den: Big) -> Value {
    if num.is_zero() {
        return Value::Real {
            negative: false,
            num,
            den: Big(vec![1]),
        };
    }
    let g = Big::gcd(&num, &den);
    Value::Real {
        negative,
        num: num.divmod(&g).0,
        den: den.divmod(&g).0,
    }
}

fn all_digits(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_digit())
}

fn split_sign(s: &str) -> (bool, &str) {
    if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else {
        (false, s.strip_prefix('+').unwrap_or(s))
    }
}

/// `(\+|-)?[0-9]+`.
fn parse_integer(s: &str) -> Option<Value> {
    let (negative, digits) = split_sign(s);
    (!digits.is_empty() && all_digits(digits)).then(|| real(negative, Big::parse(digits), Big(vec![1])))
}

/// `(\+|-)?([0-9]+(\.[0-9]*)?|\.[0-9]+)`: the digits over a power of ten.
fn parse_decimal(s: &str) -> Option<Value> {
    let (negative, body) = split_sign(s);
    let (int, frac) = match body.find('.') {
        Some(at) => (&body[..at], &body[at + 1..]),
        None => (body, ""),
    };
    if (int.is_empty() && frac.is_empty()) || !all_digits(int) || !all_digits(frac) {
        return None;
    }
    let den = Big::parse(&format!("1{}", "0".repeat(frac.len())));
    Some(real(negative, Big::parse(&format!("{int}{frac}")), den))
}

/// `numerator/denominator`: an integer over a nonzero unsigned integer, no
/// whitespace.
fn parse_rational(s: &str) -> Option<Value> {
    let (numerator, denominator) = s.split_once('/')?;
    let (negative, num) = split_sign(numerator);
    if num.is_empty() || !all_digits(num) || denominator.is_empty() || !all_digits(denominator) {
        return None;
    }
    let den = Big::parse(denominator);
    (!den.is_zero()).then(|| real(negative, Big::parse(num), den))
}

/// The lexical space of `xsd:double` and `xsd:float`, as a small state
/// machine: sign, digits, point, digits, exponent mark, sign, digits.
fn float_form(s: &str) -> bool {
    if matches!(s, "INF" | "+INF" | "-INF" | "NaN") {
        return true;
    }
    let mut chars = s.chars().peekable();
    if matches!(chars.peek(), Some('+' | '-')) {
        chars.next();
    }
    let mut mantissa_digits = 0;
    while chars.peek().is_some_and(char::is_ascii_digit) {
        chars.next();
        mantissa_digits += 1;
    }
    if chars.peek() == Some(&'.') {
        chars.next();
        while chars.peek().is_some_and(char::is_ascii_digit) {
            chars.next();
            mantissa_digits += 1;
        }
    }
    if mantissa_digits == 0 {
        return false;
    }
    if matches!(chars.peek(), Some('e' | 'E')) {
        chars.next();
        if matches!(chars.peek(), Some('+' | '-')) {
            chars.next();
        }
        let mut exponent_digits = 0;
        while chars.peek().is_some_and(char::is_ascii_digit) {
            chars.next();
            exponent_digits += 1;
        }
        if exponent_digits == 0 {
            return false;
        }
    }
    chars.next().is_none()
}

/// The special forms in Rust's spelling, which `parse` reads.
fn rust_float_form(s: &str) -> &str {
    match s {
        "INF" | "+INF" => "inf",
        "-INF" => "-inf",
        other => other,
    }
}

fn parse_double(s: &str) -> Option<u64> {
    let v: f64 = float_form(s).then(|| rust_float_form(s).parse().ok())??;
    Some(if v.is_nan() { u64::MAX } else { v.to_bits() })
}

fn parse_float(s: &str) -> Option<u32> {
    let v: f32 = float_form(s).then(|| rust_float_form(s).parse().ok())??;
    Some(if v.is_nan() { u32::MAX } else { v.to_bits() })
}

// ---------------------------------------------------------------------------
// Strings
// ---------------------------------------------------------------------------

fn is_normalized(s: &str) -> bool {
    s.chars().all(|c| !matches!(c, '\t' | '\n' | '\r'))
}

fn is_token(s: &str) -> bool {
    // A normalized string equal to itself with its spaces collapsed.
    is_normalized(s) && s.split(' ').filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ") == s
}

fn is_language(s: &str) -> bool {
    s.split('-').enumerate().all(|(i, p)| {
        (1..=8).contains(&p.len())
            && p.chars().all(|c| {
                if i == 0 {
                    c.is_ascii_alphabetic()
                } else {
                    c.is_ascii_alphanumeric()
                }
            })
    })
}

/// XML 1.0 fifth edition `NameStartChar`, as a table of ranges.
const NAME_START: &[(char, char)] = &[
    (':', ':'),
    ('A', 'Z'),
    ('_', '_'),
    ('a', 'z'),
    ('\u{C0}', '\u{D6}'),
    ('\u{D8}', '\u{F6}'),
    ('\u{F8}', '\u{2FF}'),
    ('\u{370}', '\u{37D}'),
    ('\u{37F}', '\u{1FFF}'),
    ('\u{200C}', '\u{200D}'),
    ('\u{2070}', '\u{218F}'),
    ('\u{2C00}', '\u{2FEF}'),
    ('\u{3001}', '\u{D7FF}'),
    ('\u{F900}', '\u{FDCF}'),
    ('\u{FDF0}', '\u{FFFD}'),
    ('\u{10000}', '\u{EFFFF}'),
];

/// The other `NameChar`s.
const NAME_MORE: &[(char, char)] = &[
    ('-', '-'),
    ('.', '.'),
    ('0', '9'),
    ('\u{B7}', '\u{B7}'),
    ('\u{300}', '\u{36F}'),
    ('\u{203F}', '\u{2040}'),
];

fn in_ranges(c: char, ranges: &[(char, char)]) -> bool {
    ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi)
}

fn is_nmtoken(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| in_ranges(c, NAME_START) || in_ranges(c, NAME_MORE))
}

fn is_name(s: &str) -> bool {
    s.chars().next().is_some_and(|c| in_ranges(c, NAME_START)) && is_nmtoken(s)
}

fn is_ncname(s: &str) -> bool {
    is_name(s) && !s.contains(':')
}

/// `text@tag`, the tag empty or a language tag.
fn parse_plain_literal(s: &str) -> Option<Value> {
    let at = s.rfind('@')?;
    let (text, tag) = (&s[..at], &s[at + 1..]);
    if tag.is_empty() {
        Some(Value::String(text.to_owned()))
    } else {
        is_language(tag).then(|| Value::LangString(text.to_owned(), tag.to_ascii_lowercase()))
    }
}

// ---------------------------------------------------------------------------
// Binary
// ---------------------------------------------------------------------------

fn parse_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode_base64(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(BASE64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Symbols separated by single spaces at most; decoded, then re-encoded:
/// a form whose padding bits are not zero does not come back.
fn parse_base64(s: &str) -> Option<Vec<u8>> {
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b' ' && (i == 0 || i + 1 == bytes.len() || bytes[i - 1] == b' ') {
            return None;
        }
    }
    let symbols: String = s.chars().filter(|&c| c != ' ').collect();
    if !symbols.len().is_multiple_of(4) {
        return None;
    }
    let data = symbols.trim_end_matches('=');
    if symbols.len() - data.len() > 2 {
        return None;
    }
    let mut bits = 0u32;
    let mut count = 0;
    let mut out = Vec::new();
    for c in data.bytes() {
        let v = BASE64.iter().position(|&x| x == c)? as u32;
        bits = (bits << 6) | v;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    (encode_base64(&out) == symbols).then_some(out)
}

// ---------------------------------------------------------------------------
// Date-times
// ---------------------------------------------------------------------------

/// A date-time's fields, `24:00:00` moved to the next day.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct DateTime {
    negative: bool,
    year: Big,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    /// The fraction's digits without trailing zeros.
    fraction: String,
    /// The offset in minutes.
    zone: Option<i32>,
}

fn two_digits(s: &str) -> Option<u32> {
    (s.len() == 2 && all_digits(s)).then(|| s.parse().ok())?
}

fn month_length(year: &Big, month: u32) -> u32 {
    let leap = year.rem_small(4) == 0 && (year.rem_small(100) != 0 || year.rem_small(400) == 0);
    match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn parse_date_time(s: &str) -> Option<DateTime> {
    let (date, time) = s.split_once('T')?;
    let (negative, date) = match date.strip_prefix('-') {
        Some(d) => (true, d),
        None => (false, date),
    };
    let mut parts = date.split('-');
    let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || year.len() < 4 || !all_digits(year) || (year.len() > 4 && year.starts_with('0')) {
        return None;
    }
    let (month, day) = (two_digits(month)?, two_digits(day)?);
    // The zone: Z, or a sign and hh:mm at the end.
    let tb = time.as_bytes();
    let (clock, zone) = if let Some(c) = time.strip_suffix('Z') {
        (c, Some(0))
    } else if tb.len() > 6 && matches!(tb[tb.len() - 6], b'+' | b'-') {
        let (c, z) = time.split_at(time.len() - 6);
        let (h, m) = z[1..].split_once(':')?;
        let (h, m) = (two_digits(h)? as i32, two_digits(m)? as i32);
        if m > 59 || h * 60 + m > 14 * 60 {
            return None;
        }
        (c, Some(if z.starts_with('-') { -(h * 60 + m) } else { h * 60 + m }))
    } else {
        (time, None)
    };
    let (whole, fraction) = match clock.split_once('.') {
        Some((w, f)) if !f.is_empty() && all_digits(f) => (w, f.trim_end_matches('0')),
        Some(_) => return None,
        None => (clock, ""),
    };
    let mut hms = whole.split(':');
    let (hour, minute, second) = (
        two_digits(hms.next()?)?,
        two_digits(hms.next()?)?,
        two_digits(hms.next()?)?,
    );
    if hms.next().is_some() {
        return None;
    }
    let year = Big::parse(year);
    let negative = negative && !year.is_zero();
    if !(1..=12).contains(&month) || day < 1 || day > month_length(&year, month) || minute > 59 || second > 59 {
        return None;
    }
    let end_of_day = hour == 24 && minute == 0 && second == 0 && fraction.is_empty();
    if hour > 23 && !end_of_day {
        return None;
    }
    let mut d = DateTime {
        negative,
        year,
        month,
        day,
        hour,
        minute,
        second,
        fraction: fraction.to_owned(),
        zone,
    };
    if end_of_day {
        d.hour = 0;
        d.day += 1;
        if d.day > month_length(&d.year, d.month) {
            d.day = 1;
            d.month += 1;
        }
        if d.month > 12 {
            d.month = 1;
            if d.negative {
                d.year = d.year.sub(&Big(vec![1]));
                d.negative = !d.year.is_zero();
            } else {
                d.year = d.year.add_one();
            }
        }
    }
    Some(d)
}

// ---------------------------------------------------------------------------
// XML literals
// ---------------------------------------------------------------------------

/// Balanced XML content, checked for its tags only: every end tag closes
/// the innermost open element, every markup construct is closed by its
/// `>`, and no element is left open. Plain text is content too. Entities
/// and attribute syntax are not checked.
fn xml(lexical: &str) -> bool {
    let mut open: Vec<&str> = Vec::new();
    let mut rest = lexical;
    while let Some(start) = rest.find('<') {
        rest = &rest[start..];
        let (end_marker, skip) = if rest.starts_with("<!--") {
            ("-->", 4)
        } else if rest.starts_with("<![CDATA[") {
            ("]]>", 9)
        } else if rest.starts_with("<?") {
            ("?>", 2)
        } else {
            (">", 1)
        };
        let Some(len) = rest[skip..].find(end_marker) else {
            return false;
        };
        let inner = &rest[1..skip + len];
        rest = &rest[skip + len + end_marker.len()..];
        if skip != 1 || inner.starts_with('!') {
            continue;
        }
        if let Some(name) = inner.strip_prefix('/') {
            if open.pop() != Some(name.trim_end()) {
                return false;
            }
        } else if !inner.ends_with('/') {
            let name = inner.split(|c: char| c.is_whitespace()).next().unwrap_or("");
            open.push(name);
        }
    }
    open.is_empty()
}

/// Lexical forms at the edges of each datatype of the map (bounds,
/// whitespace, signs, exponents, special float values, time zones), and
/// whether each is well-typed: the table the core's map passes too
/// (`crates/turbowl/src/rules/datatype/edge_cases.rs`). The generator draws
/// literals from it, so chaos compares the two maps on all of it.
#[rustfmt::skip]
pub(crate) const EDGE_CASES: &[(&str, &[(&str, bool)])] = &[
    ("http://www.w3.org/2002/07/owl#real", &[("1", false), ("1.5", false), ("1/3", false), ("", false)]),
    ("http://www.w3.org/2002/07/owl#rational", &[
        ("1/3", true), ("-1/3", true), ("+1/3", true), ("0/5", true), ("-0/5", true), ("4/2", true),
        ("007/010", true), ("123456789012345678901234567890123456789012/7", true),
        ("1/0", false), ("1/00", false), ("1/-3", false), ("1/+3", false), ("1 / 3", false), (" 1/3", false),
        ("1/3 ", false), ("1", false), ("1.5/2", false), ("/3", false), ("1/", false), ("1//3", false),
        ("1/3/5", false), ("", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#decimal", &[
        ("0", true), ("-0", true), ("+0.0", true), ("1.", true), (".5", true), ("-.5", true),
        ("007.500", true), ("123456789012345678901234567890.123456789", true),
        (".", false), ("", false), ("+", false), ("-.", false), ("1e3", false), ("1E3", false),
        (" 1", false), ("1 ", false), ("1,5", false), ("INF", false), ("NaN", false), ("1.2.3", false),
        ("--1", false), ("+-1", false), ("\u{663}", false), ("1_000", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#integer", &[
        ("+5", true), ("-7", true), ("42", true), ("007", true), ("0", true), ("-0", true),
        ("99999999999999999999999", true),
        (" 42", false), ("42 ", false), ("4 2", false), ("abc", false), ("", false), ("+", false),
        ("-", false), ("+-5", false), ("1.0", false), ("1.", false), ("1e3", false), ("\u{663}", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#nonNegativeInteger", &[
        ("0", true), ("-0", true), ("+0", true), ("1", true),
        ("99999999999999999999999999999999999999999", true), ("-1", false),
        ("-99999999999999999999999999999999999999999", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#nonPositiveInteger", &[
        ("0", true), ("-0", true), ("+0", true), ("-1", true),
        ("-99999999999999999999999999999999999999999", true), ("1", false),
        ("99999999999999999999999999999999999999999", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#positiveInteger", &[("1", true), ("+0001", true), ("0", false), ("-0", false), ("-1", false)]),
    ("http://www.w3.org/2001/XMLSchema#negativeInteger", &[("-1", true), ("-0001", true), ("0", false), ("-0", false), ("1", false)]),
    ("http://www.w3.org/2001/XMLSchema#long", &[
        ("9223372036854775807", true), ("-9223372036854775808", true),
        ("9223372036854775808", false), ("-9223372036854775809", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#int", &[
        ("-7", true), ("+7", true), ("2147483647", true), ("+2147483647", true), ("-2147483648", true),
        ("0002147483647", true), ("-0002147483648", true), ("-0", true),
        ("2147483648", false), ("-2147483649", false), ("4294967296", false), ("99999999999", false),
        ("99999999999999999999999", false), (" 42", false), (" 3 ", false), ("", false), ("abc", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#short", &[("32767", true), ("-32768", true), ("32768", false), ("-32769", false)]),
    ("http://www.w3.org/2001/XMLSchema#byte", &[("127", true), ("-128", true), ("128", false), ("-129", false), ("1.0", false)]),
    ("http://www.w3.org/2001/XMLSchema#unsignedLong", &[
        ("18446744073709551615", true), ("0", true), ("-0", true),
        ("18446744073709551616", false), ("-1", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#unsignedInt", &[("4294967295", true), ("4294967296", false), ("-1", false)]),
    ("http://www.w3.org/2001/XMLSchema#unsignedShort", &[("65535", true), ("65536", false)]),
    ("http://www.w3.org/2001/XMLSchema#unsignedByte", &[("255", true), ("+0", true), ("256", false), ("-1", false)]),
    ("http://www.w3.org/2001/XMLSchema#double", &[
        ("1", true), ("-1.5E-3", true), ("1e5", true), ("1.e5", true), (".5e-1", true), ("+1E+400", true),
        ("1E400", true), ("1e-400", true), ("-0", true), ("0.0e0", true), ("INF", true), ("+INF", true),
        ("-INF", true), ("NaN", true), ("9007199254740993", true), ("1e00000000000000000000000001", true),
        ("inf", false), ("Infinity", false), ("nan", false), ("-NaN", false), ("+NaN", false), ("1e", false),
        ("e5", false), (".e5", false), ("1e5.0", false), (" 1", false), ("1 ", false), ("", false),
        (".", false), ("1E+", false), ("0x10", false), ("1d", false), ("1f", false), ("- 1", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#float", &[
        ("1", true), ("1E40", true), ("3.4028235E38", true), ("-1e-50", true), ("INF", true), ("NaN", true),
        ("16777217", true), ("Inf", false), (" 1", false), ("1.5f", false), ("", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#string", &[("", true), (" x ", true), ("a\tb\nc", true), ("<a>", true)]),
    ("http://www.w3.org/2001/XMLSchema#normalizedString", &[
        ("", true), ("a b", true), (" a  b ", true), ("a\tb", false), ("a\nb", false), ("a\rb", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#token", &[
        ("", true), ("a b", true), ("a", true), (" a", false), ("a ", false), ("a  b", false), ("a\tb", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#language", &[
        ("en", true), ("en-GB", true), ("zh-Hant-TW", true), ("x-1", true), ("abcdefgh", true),
        ("abcdefghi", false), ("en-", false), ("-en", false), ("", false), ("en_GB", false),
        ("en-abcdefghi", false), ("1en", false), (" en", false), ("en--GB", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#Name", &[
        ("a", true), ("_a", true), (":a", true), ("a:b", true), ("a-1.b", true), ("\u{e9}t\u{e9}", true),
        ("a\u{b7}b", true), ("1a", false), ("-a", false), (".a", false), ("a b", false), ("", false),
        ("\u{b7}a", false), ("a,b", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#NCName", &[
        ("a", true), ("_a-1", true), ("a:b", false), (":a", false), ("1a", false), ("", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#NMTOKEN", &[
        ("1a", true), ("-.", true), ("a:b", true), ("\u{b7}", true), ("", false), ("a b", false),
        ("a,b", false), (" a", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#boolean", &[
        ("true", true), ("false", true), ("1", true), ("0", true), ("TRUE", false), ("True", false),
        ("yes", false), (" true", false), ("", false), ("01", false), ("-0", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#hexBinary", &[
        ("", true), ("0FB7", true), ("0fb7", true), ("0FB", false), ("0G", false), (" 0F", false),
        ("0F ", false), ("0x0F", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#base64Binary", &[
        ("", true), ("QQ==", true), ("QUI=", true), ("QUJD", true), ("QUJDRA==", true), ("Q Q = =", true),
        ("QUJD RA==", true), ("QQ= =", true), ("a+/9", true),
        ("QR==", false), ("QUJ=", false), ("QUI", false), ("Q===", false), ("=QQQ", false), (" QQ==", false),
        ("QQ== ", false), ("QU  I=", false), ("QQ=A", false), ("QUJD!A==", false), ("QQ", false),
        ("Q\tQ==", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#anyURI", &[
        ("http://example.org/", true), ("", true), ("relative/path", true), ("?q", true), (":x", true), ("a b", true),
    ]),
    ("http://www.w3.org/2001/XMLSchema#dateTime", &[
        ("2000-01-01T00:00:00", true), ("2000-01-01T00:00:00Z", true), ("2000-01-01T00:00:00.5+01:30", true),
        ("-0044-03-15T12:00:00", true), ("12345-01-01T00:00:00", true), ("0000-01-01T00:00:00", true),
        ("-0000-01-01T00:00:00", true), ("2000-02-29T00:00:00", true), ("2004-02-29T00:00:00", true),
        ("0000-02-29T00:00:00", true), ("-2000-02-29T00:00:00", true), ("2000-01-01T24:00:00", true),
        ("2000-01-01T24:00:00.000", true), ("2000-01-01T00:00:00+14:00", true),
        ("2000-01-01T00:00:00-14:00", true), ("2000-01-01T00:00:00-13:59", true),
        ("2000-01-01T23:59:59.999999999999", true),
        ("1900-02-29T00:00:00", false), ("2001-02-29T00:00:00", false), ("-0001-02-29T00:00:00", false),
        ("2000-04-31T00:00:00", false), ("2000-13-01T00:00:00", false), ("2000-00-01T00:00:00", false),
        ("2000-01-00T00:00:00", false), ("2000-01-32T00:00:00", false), ("2000-01-01T24:00:01", false),
        ("2000-01-01T24:00:00.1", false), ("2000-01-01T24:01:00", false), ("2000-01-01T25:00:00", false),
        ("2000-01-01T23:60:00", false), ("2000-01-01T23:59:60", false), ("2000-01-01T00:00:00+14:01", false),
        ("2000-01-01T00:00:00+15:00", false), ("2000-01-01T00:00:00+1:00", false),
        ("2000-01-01T00:00:00+01:60", false), ("2000-01-01T00:00:00+0100", false),
        ("2000-01-01T00:00:00z", false), ("2000-01-01T00:00:00.", false), ("2000-01-01T00:00:00Z ", false),
        (" 2000-01-01T00:00:00", false), ("2000-01-01", false), ("2000-01-01T00:00", false),
        ("200-01-01T00:00:00", false), ("02000-01-01T00:00:00", false), ("+2000-01-01T00:00:00", false),
        ("2000-1-01T00:00:00", false), ("2000-01-01t00:00:00", false), ("2000-01-01 00:00:00", false),
        ("--2000-01-01T00:00:00", false), ("", false),
    ]),
    ("http://www.w3.org/2001/XMLSchema#dateTimeStamp", &[
        ("2000-01-01T00:00:00Z", true), ("2000-01-01T00:00:00-05:00", true), ("2000-01-01T00:00:00", false),
        ("2000-02-30T00:00:00Z", false),
    ]),
    ("http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral", &[
        ("abc@", true), ("abc@en", true), ("abc@EN-gb", true), ("a@b@en", true), ("@", true), ("@en", true),
        ("abc", false), ("", false), ("abc@en-", false), ("abc@1en", false), ("abc@en gb", false),
        ("abc@en@", true),
    ]),
    ("http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral", &[
        ("<a>t</a>", true), ("plain text", true), ("", true), ("<a><b/></a>", true), ("<a x=\"1\">t</a>", true),
        ("<a >t</a >", true), ("<!-- c --><a/>", true), ("<![CDATA[<a>]]>", true), ("<?pi x?><a/>", true),
        ("<!DOCTYPE a><a/>", true), ("<a>", false), ("<a>t</b>", false), ("</a>", false), ("<a", false),
        ("<!-- c", false), ("<a><b></a></b>", false), ("<", false),
    ]),
];

#[cfg(test)]
mod tests {
    use super::*;
    use oxrdf::NamedNode;
    use std::collections::BTreeSet;

    fn typed(value: &str, datatype: &str) -> Literal {
        Literal::new_typed_literal(value, NamedNode::new_unchecked(datatype))
    }

    fn xsd(local: &str) -> String {
        format!("{XSD}{local}")
    }

    fn value(lexical: &str, datatype: &str) -> Value {
        match read(&typed(lexical, datatype)) {
            Reading::Value(v) => v,
            other => panic!("{lexical:?}^^{datatype}: {other:?}"),
        }
    }

    #[test]
    fn edge_cases_of_every_datatype() {
        let mut failures = Vec::new();
        for (datatype, cases) in EDGE_CASES {
            for &(lexical, ok) in *cases {
                if well_formed(&typed(lexical, datatype)) != Some(ok) {
                    failures.push(format!("{lexical:?}^^{datatype}"));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
        let covered: BTreeSet<&str> = EDGE_CASES.iter().map(|(d, _)| *d).collect();
        for iri in datatype_iris() {
            let exempt = iri.ends_with("#Literal") || iri.ends_with("#langString");
            assert!(exempt || covered.contains(iri.as_str()), "{iri}");
        }
    }

    #[test]
    fn literals_outside_the_map_and_language_strings() {
        assert_eq!(well_formed(&typed("x", "http://example.org/dt")), None);
        assert_eq!(well_formed(&typed("x", &format!("{RDFS}Literal"))), None);
        let tagged = Literal::new_language_tagged_literal_unchecked("x", "en");
        assert_eq!(well_formed(&tagged), Some(true));
        assert_eq!(in_value_space(&tagged, &xsd("string")), Some(false));
        assert_eq!(in_value_space(&tagged, &format!("{RDF}langString")), Some(true));
        assert_eq!(in_value_space(&tagged, &format!("{RDF}PlainLiteral")), Some(true));
        assert_eq!(in_value_space(&tagged, "http://example.org/C"), None);
        assert_eq!(
            in_value_space(&Literal::new_simple_literal("x"), &xsd("integer")),
            Some(false)
        );
    }

    #[test]
    fn value_spaces_hold_the_integer_subtypes_inside_the_decimals() {
        let five_int = typed("5", &xsd("int"));
        for (datatype, ok) in [
            ("integer", true),
            ("decimal", true),
            ("byte", true),
            ("unsignedByte", true),
            ("positiveInteger", true),
            ("negativeInteger", false),
            ("double", false),
            ("float", false),
            ("string", false),
        ] {
            assert_eq!(in_value_space(&five_int, &xsd(datatype)), Some(ok), "{datatype}");
        }
        assert_eq!(in_value_space(&five_int, &format!("{OWL}real")), Some(true));
        assert_eq!(in_value_space(&five_int, &format!("{RDFS}Literal")), Some(true));
        assert_eq!(
            in_value_space(&typed("1.0", &xsd("decimal")), &xsd("integer")),
            Some(true)
        );
        assert_eq!(
            in_value_space(&typed("1.5", &xsd("decimal")), &xsd("integer")),
            Some(false)
        );
        let rational = format!("{OWL}rational");
        assert_eq!(in_value_space(&typed("1/3", &rational), &xsd("decimal")), Some(false));
        assert_eq!(in_value_space(&typed("1/4", &rational), &xsd("decimal")), Some(true));
        assert_eq!(
            in_value_space(&typed("abc", &xsd("integer")), &format!("{RDFS}Literal")),
            Some(false)
        );
        let all = datatypes(&value("300", &xsd("integer")));
        assert!(all.contains(&xsd("short")) && !all.contains(&xsd("byte")), "{all:?}");
    }

    #[test]
    fn values_are_identical_by_value() {
        let rational = format!("{OWL}rational");
        let same = [
            (value("1", &xsd("integer")), value("1.0", &xsd("decimal"))),
            (value("1/2", &rational), value("0.50", &xsd("decimal"))),
            (value("-0", &xsd("integer")), value("0", &xsd("decimal"))),
            (value("NaN", &xsd("double")), value("NaN", &xsd("double"))),
            (value("1E400", &xsd("double")), value("INF", &xsd("double"))),
            (value("0fb7", &xsd("hexBinary")), value("0FB7", &xsd("hexBinary"))),
            (
                value("Q Q = =", &xsd("base64Binary")),
                value("QQ==", &xsd("base64Binary")),
            ),
            (value("a", &xsd("token")), value("a@", &format!("{RDF}PlainLiteral"))),
            (
                value("1999-12-31T24:00:00Z", &xsd("dateTime")),
                value("2000-01-01T00:00:00+00:00", &xsd("dateTimeStamp")),
            ),
            (
                value("-0001-12-31T24:00:00", &xsd("dateTime")),
                value("0000-01-01T00:00:00", &xsd("dateTime")),
            ),
        ];
        for (a, b) in same {
            assert_eq!(a, b);
        }
        let different = [
            (value("0", &xsd("double")), value("-0", &xsd("double"))),
            (value("1", &xsd("double")), value("1", &xsd("float"))),
            (value("1", &xsd("double")), value("1", &xsd("integer"))),
            (value("a", &xsd("string")), value("a", &xsd("anyURI"))),
            (value("00", &xsd("hexBinary")), value("AA==", &xsd("base64Binary"))),
            (
                value("2000-01-01T01:00:00+01:00", &xsd("dateTime")),
                value("2000-01-01T00:00:00Z", &xsd("dateTime")),
            ),
        ];
        for (a, b) in different {
            assert_ne!(a, b);
        }
    }

    #[test]
    fn big_numbers() {
        assert_eq!(Big::gcd(&Big::parse("12"), &Big::parse("18")), Big::parse("6"));
        assert_eq!(
            Big::parse("1000").divmod(&Big::parse("7")),
            (Big::parse("142"), Big::parse("6"))
        );
        assert_eq!(Big::parse("999").add_one(), Big::parse("1000"));
        assert_eq!(Big::parse("1000").sub(&Big::parse("1")), Big::parse("999"));
        assert!(only_twos_and_fives(&Big::parse("1600")));
        assert!(!only_twos_and_fives(&Big::parse("30")));
    }

    #[test]
    fn xml_checks_tags() {
        for (value, ok) in [
            ("<a>t</a>", true),
            ("plain text", true),
            ("<a><b/></a>", true),
            ("<a x=\"1\">t</a>", true),
            ("<!-- c --><a/>", true),
            ("<a>", false),
            ("<a>t</b>", false),
            ("</a>", false),
            ("<a", false),
        ] {
            assert_eq!(xml(value), ok, "{value:?}");
        }
    }
}
