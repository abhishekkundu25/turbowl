//! The lexical spaces of the datatype map's primitives, and their lexical
//! mappings: each function reads a lexical form, exactly as written (RDF
//! applies no whitespace processing, so `" 3 "^^xsd:int` is ill-typed), and
//! gives its value, or `None` when the form is outside the lexical space.
//! The grammars are XML Schema 1.1 Part 2's; rdf:PlainLiteral's is its own
//! specification's.
//!
//! A value borrows from the lexical form wherever its canonical form is a
//! part of it, so the common literals are read without allocating.

use super::map::{DateTime, Decimal, Value};
use std::borrow::Cow;

/// An optional sign: whether it is `-`, and the rest.
fn sign(s: &str) -> (bool, &str) {
    match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    }
}

fn digits(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_digit())
}

/// A decimal from its sign and its integer and fraction digits.
fn decimal_value<'a>(negative: bool, int: &'a str, frac: &'a str) -> Decimal<'a> {
    let int = int.trim_start_matches('0');
    let frac = frac.trim_end_matches('0');
    Decimal {
        negative: negative && !(int.is_empty() && frac.is_empty()),
        int: Cow::Borrowed(int),
        frac: Cow::Borrowed(frac),
    }
}

/// `xsd:integer`: `(\+|-)?[0-9]+`.
pub(super) fn integer(s: &str) -> Option<Decimal<'_>> {
    let (negative, body) = sign(s);
    (!body.is_empty() && digits(body)).then(|| decimal_value(negative, body, ""))
}

/// `xsd:decimal`: `(\+|-)?([0-9]+(\.[0-9]*)?|\.[0-9]+)`.
pub(super) fn decimal(s: &str) -> Option<Decimal<'_>> {
    let (negative, body) = sign(s);
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    (digits(int) && digits(frac) && !(int.is_empty() && frac.is_empty())).then(|| decimal_value(negative, int, frac))
}

/// The numeral part of `xsd:double` and `xsd:float`:
/// `(\+|-)?([0-9]+(\.[0-9]*)?|\.[0-9]+)([Ee](\+|-)?[0-9]+)?`.
fn float_numeral(s: &str) -> bool {
    let (mantissa, exponent) = match s.find(['e', 'E']) {
        Some(at) => (&s[..at], Some(&s[at + 1..])),
        None => (s, None),
    };
    decimal(mantissa).is_some()
        && exponent.is_none_or(|e| {
            let e = sign(e).1;
            !e.is_empty() && digits(e)
        })
}

/// `xsd:double`: a numeral rounded to the nearest double (past the largest,
/// to an infinity; a negative numeral rounding to zero gives -0), or `INF`,
/// `+INF`, `-INF`, `NaN`. The value is the double's bits, with one NaN.
pub(super) fn double(s: &str) -> Option<u64> {
    let v = match s {
        "INF" | "+INF" => f64::INFINITY,
        "-INF" => f64::NEG_INFINITY,
        "NaN" => f64::NAN,
        _ if float_numeral(s) => s.parse().ok()?,
        _ => return None,
    };
    Some(if v.is_nan() { f64::NAN.to_bits() } else { v.to_bits() })
}

/// `xsd:float`: as [`double`], rounded to a float directly from the numeral.
pub(super) fn float(s: &str) -> Option<u32> {
    let v = match s {
        "INF" | "+INF" => f32::INFINITY,
        "-INF" => f32::NEG_INFINITY,
        "NaN" => f32::NAN,
        _ if float_numeral(s) => s.parse().ok()?,
        _ => return None,
    };
    Some(if v.is_nan() { f32::NAN.to_bits() } else { v.to_bits() })
}

/// `xsd:boolean`: `true`, `false`, `1`, `0`.
pub(super) fn boolean(s: &str) -> Option<bool> {
    match s {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

/// `xsd:hexBinary`: `([0-9a-fA-F]{2})*`. The value is the upper-case form.
pub(super) fn hex_binary(s: &str) -> Option<Cow<'_, str>> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(if s.bytes().any(|b| b.is_ascii_lowercase()) {
        Cow::Owned(s.to_ascii_uppercase())
    } else {
        Cow::Borrowed(s)
    })
}

/// `xsd:base64Binary` (XML Schema 1.1, §3.3.16): groups of four symbols of
/// `[A-Za-z0-9+/]`, the last group possibly padded (`xx?=` with `?` in
/// `[AEIMQUYcgkosw048]`, or `x?==` with `?` in `[AQgw]`, so the padding bits
/// are zero), and each symbol but the last optionally followed by one
/// space. The value is the form without spaces: each octet sequence has
/// exactly one.
pub(super) fn base64_binary(s: &str) -> Option<Cow<'_, str>> {
    if s.starts_with(' ') || s.ends_with(' ') || s.contains("  ") {
        return None;
    }
    let symbols: Cow<'_, str> = if s.contains(' ') {
        Cow::Owned(s.replace(' ', ""))
    } else {
        Cow::Borrowed(s)
    };
    let b = symbols.as_bytes();
    if !b.len().is_multiple_of(4) {
        return None;
    }
    let padding = b.iter().rev().take_while(|&&c| c == b'=').count();
    let data = &b[..b.len() - padding];
    let symbol = |c: &u8| c.is_ascii_alphanumeric() || *c == b'+' || *c == b'/';
    let ok = data.iter().all(symbol)
        && match padding {
            0 => true,
            1 => b"AEIMQUYcgkosw048".contains(&data[data.len() - 1]),
            2 => b"AQgw".contains(&data[data.len() - 1]),
            _ => false,
        };
    ok.then_some(symbols)
}

/// `rdf:PlainLiteral`: `text@tag`, split at the last `@`; an empty tag
/// gives the string `text`, a tag of `xsd:language`'s pattern the
/// language-tagged string with the tag in lower case.
pub(super) fn plain_literal(s: &str) -> Option<Value<'_>> {
    let (text, tag) = s.rsplit_once('@')?;
    if tag.is_empty() {
        Some(Value::String(text))
    } else if language(tag) {
        Some(Value::LangString {
            text,
            tag: lower_case(tag),
        })
    } else {
        None
    }
}

/// A language tag in lower case, borrowed when it is already.
pub(super) fn lower_case(tag: &str) -> Cow<'_, str> {
    if tag.bytes().any(|b| b.is_ascii_uppercase()) {
        Cow::Owned(tag.to_ascii_lowercase())
    } else {
        Cow::Borrowed(tag)
    }
}

// The string datatypes: the lexical space is the value space, so each is
// the set of strings its pattern matches (XML Schema 1.1, §3.4). Any string
// is an `xsd:string`.

/// `xsd:normalizedString`: no carriage return, line feed or tab.
pub(super) fn normalized_string(s: &str) -> bool {
    !s.contains(['\r', '\n', '\t'])
}

/// `xsd:token`: a normalized string with no leading or trailing space and
/// no two spaces in a row.
pub(super) fn token(s: &str) -> bool {
    normalized_string(s) && !s.starts_with(' ') && !s.ends_with(' ') && !s.contains("  ")
}

/// `xsd:language`: `[a-zA-Z]{1,8}(-[a-zA-Z0-9]{1,8})*`.
pub(super) fn language(s: &str) -> bool {
    let mut parts = s.split('-');
    let first = parts.next().unwrap_or("");
    let part = |p: &str, ok: fn(&u8) -> bool| (1..=8).contains(&p.len()) && p.bytes().all(|b| ok(&b));
    part(first, u8::is_ascii_alphabetic) && parts.all(|p| part(p, u8::is_ascii_alphanumeric))
}

/// `xsd:NMTOKEN`: one or more XML name characters.
pub(super) fn nmtoken(s: &str) -> bool {
    !s.is_empty() && s.chars().all(name_char)
}

/// `xsd:Name`: an XML name start character, then name characters.
pub(super) fn name(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(name_start_char) && chars.all(name_char)
}

/// `xsd:NCName`: a name without a colon.
pub(super) fn ncname(s: &str) -> bool {
    name(s) && !s.contains(':')
}

/// XML 1.0 (fifth edition) `NameStartChar`.
fn name_start_char(c: char) -> bool {
    matches!(c,
        ':' | 'A'..='Z' | '_' | 'a'..='z'
        | '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}'
        | '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}' | '\u{10000}'..='\u{EFFFF}')
}

/// XML 1.0 (fifth edition) `NameChar`.
fn name_char(c: char) -> bool {
    name_start_char(c)
        || matches!(c, '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
}

/// `xsd:dateTime` (XML Schema 1.1, §3.3.7):
/// `-?([1-9][0-9]{3,}|0[0-9]{3})-MM-DDThh:mm:ss(\.s+)?(Z|(\+|-)hh:mm)?`,
/// with a day that exists in its month and year, hours 00 to 23 or the end
/// of day `24:00:00`, and offsets from -14:00 to +14:00. The value keeps the
/// fields, with `24:00:00` read as the start of the next day: two values
/// are identical when they are the same instant with the same offset
/// (OWL 2 Structural Specification §4.7).
pub(super) fn date_time(s: &str) -> Option<DateTime<'_>> {
    let (negative, rest) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let year_len = rest.bytes().take_while(u8::is_ascii_digit).count();
    if year_len < 4 || (year_len > 4 && rest.starts_with('0')) {
        return None;
    }
    let (year, rest) = rest.split_at(year_len);
    // `-MM-DDThh:mm:ss`, at fixed places.
    let b = rest.as_bytes();
    let two = |i: usize| -> Option<u8> {
        let (h, l) = (*b.get(i)?, *b.get(i + 1)?);
        (h.is_ascii_digit() && l.is_ascii_digit()).then(|| (h - b'0') * 10 + (l - b'0'))
    };
    let at = |i: usize, c: u8| b.get(i) == Some(&c);
    if !(at(0, b'-') && at(3, b'-') && at(6, b'T') && at(9, b':') && at(12, b':')) {
        return None;
    }
    let (month, day, hour, minute, second) = (two(1)?, two(4)?, two(7)?, two(10)?, two(13)?);
    let mut rest = &rest[15..];
    let mut frac = "";
    if let Some(after) = rest.strip_prefix('.') {
        let len = after.bytes().take_while(u8::is_ascii_digit).count();
        if len == 0 {
            return None;
        }
        frac = &after[..len];
        rest = &after[len..];
    }
    let offset = match rest {
        "" => None,
        "Z" => Some(0),
        _ => {
            let b = rest.as_bytes();
            let sign = match b[0] {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            let digit = |i: usize| b.get(i).filter(|c| c.is_ascii_digit()).map(|c| i16::from(c - b'0'));
            if b.len() != 6 || b[3] != b':' {
                return None;
            }
            let (hh, mm) = (digit(1)? * 10 + digit(2)?, digit(4)? * 10 + digit(5)?);
            if mm > 59 || hh > 14 || (hh == 14 && mm != 0) {
                return None;
            }
            Some(sign * (hh * 60 + mm))
        }
    };
    let year = year.trim_start_matches('0');
    let negative = negative && !year.is_empty();
    let frac = frac.trim_end_matches('0');
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    let end_of_day = hour == 24 && minute == 0 && second == 0 && frac.is_empty();
    if (hour > 23 && !end_of_day) || minute > 59 || second > 59 {
        return None;
    }
    let mut value = DateTime {
        negative,
        year: Cow::Borrowed(year),
        month,
        day,
        hour,
        minute,
        second,
        frac,
        offset,
    };
    if end_of_day {
        value.hour = 0;
        value.day += 1;
        if value.day > days_in_month(&value.year, value.month) {
            value.day = 1;
            value.month += 1;
            if value.month > 12 {
                value.month = 1;
                let (negative, year) = next_year(value.negative, &value.year);
                value.negative = negative;
                value.year = Cow::Owned(year);
            }
        }
    }
    Some(value)
}

/// The days of a month, in a year given by its digits (its sign does not
/// matter: -4 is a leap year as 4 is, in the proleptic calendar with a
/// year 0 that XML Schema 1.1 uses).
fn days_in_month(year: &str, month: u8) -> u8 {
    match month {
        2 => {
            let r = year.bytes().fold(0u32, |r, b| (r * 10 + u32::from(b - b'0')) % 400);
            if r.is_multiple_of(4) && (!r.is_multiple_of(100) || r == 0) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The year after `year` (digits without leading zeros, empty for 0, and
/// a sign).
fn next_year(negative: bool, year: &str) -> (bool, String) {
    let mut d: Vec<u8> = year.bytes().map(|b| b - b'0').collect();
    if negative {
        // -(m) + 1 = -(m - 1).
        for digit in d.iter_mut().rev() {
            if *digit > 0 {
                *digit -= 1;
                break;
            }
            *digit = 9;
        }
    } else {
        let mut carry = true;
        for digit in d.iter_mut().rev() {
            if !carry {
                break;
            }
            *digit = (*digit + 1) % 10;
            carry = *digit == 0;
        }
        if carry {
            d.insert(0, 1);
        }
    }
    let text: String = d.iter().map(|&b| char::from(b'0' + b)).collect();
    let text = text.trim_start_matches('0').to_owned();
    (negative && !text.is_empty(), text)
}

/// `rdf:XMLLiteral`: balanced XML content, checked for its tags only: every
/// end tag closes the innermost open element, every markup construct is
/// closed by its `>`, and no element is left open. Plain text is content
/// too. Entities and attribute syntax are not checked (version 1's check).
pub(super) fn xml(lexical: &str) -> bool {
    let mut open = Names::default();
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
            open.push(inner.split(char::is_whitespace).next().unwrap_or(""));
        }
    }
    open.is_empty()
}

/// The open elements' names: the first [`Names::INLINE`] in place, deeper
/// ones in a vector, so the usual XML literal allocates nothing.
#[derive(Debug, Default)]
struct Names<'a> {
    inline: [&'a str; Names::INLINE],
    len: usize,
    deeper: Vec<&'a str>,
}

impl<'a> Names<'a> {
    const INLINE: usize = 16;

    fn push(&mut self, name: &'a str) {
        if self.len < Self::INLINE {
            self.inline[self.len] = name;
            self.len += 1;
        } else {
            self.deeper.push(name);
        }
    }

    fn pop(&mut self) -> Option<&'a str> {
        self.deeper.pop().or_else(|| {
            self.len = self.len.checked_sub(1)?;
            Some(self.inline[self.len])
        })
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}
