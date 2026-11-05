//! The OWL 2 RL datatype map (OWL 2 Profiles §4.2; OWL 2 Structural
//! Specification §4): one table, [`MAP`], that every datatype rule reads.
//! For each datatype it gives how its lexical forms are read (the
//! lexical-to-value mapping of its primitive, [`Lexical`]) and which values
//! its value space holds ([`Space`]). The inclusions between value spaces
//! follow from the values: `"5"^^xsd:int` reads as the number 5, which
//! `xsd:integer`, `xsd:decimal`, `owl:rational` and `owl:real` all hold.
//!
//! # The value spaces
//!
//! - The numbers (`owl:real` and below): a real with a literal form is a
//!   decimal, or a fraction no decimal equals (from `owl:rational`).
//!   `xsd:integer` holds the decimals with no fraction, and its subtypes
//!   those in their ranges. `owl:real` has no lexical form.
//! - `xsd:double` and `xsd:float`: IEEE values, disjoint from each other and
//!   from the reals. -0 and +0 are two values; NaN is one value.
//! - The strings: `xsd:string` holds every string, its subtypes the strings
//!   their patterns match. `rdf:langString` holds the language-tagged
//!   strings, `rdf:PlainLiteral` both.
//! - `xsd:boolean`, `xsd:hexBinary`, `xsd:base64Binary`, `xsd:anyURI`,
//!   `rdf:XMLLiteral`: each its own space. `xsd:dateTime` holds the date-
//!   times, `xsd:dateTimeStamp` those with a time zone offset.
//! - `rdfs:Literal` holds every value. It is the top data range (OWL 2
//!   Structural Specification §5.2), not a datatype with lexical forms, so
//!   a literal typed `rdfs:Literal` is not read: its value is unknown here.
//!
//! # For the rules
//!
//! - [`value_of`]: a literal's value, or whether it is ill-typed (its form is
//!   outside its datatype's lexical space, or reads to a value outside its
//!   value space), or unknown (not a literal of a datatype of the map).
//! - `==` on [`Value`] is the identity of data values, which dt-eq and
//!   dt-diff compare: `"1"^^xsd:integer` and `"1.0"^^xsd:decimal` are one
//!   value; `"0"^^xsd:double` and `"-0"^^xsd:double` are two; two
//!   date-times with different offsets are two, even for the same instant
//!   (equal but not identical, Structural Specification §4).
//! - [`Datatype::holds`]: whether a datatype's value space holds a value
//!   (dt-not-type), and [`Value::datatypes`]: every datatype whose value
//!   space holds it (dt-type2).
//!
//! Two departures from XML Schema, both version 1's: any string is an
//! `xsd:string` (XML Schema leaves out the characters XML does not allow,
//! such as U+0000), and `rdf:XMLLiteral` is checked for balanced tags only.

use super::lexical;
use crate::term::{Term, TermId, vocab};
use std::borrow::Cow;

/// The datatypes of the map, in [`MAP`]'s order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Datatype {
    RdfsLiteral,
    PlainLiteral,
    LangString,
    XmlLiteral,
    Real,
    Rational,
    Decimal,
    Integer,
    NonNegativeInteger,
    NonPositiveInteger,
    PositiveInteger,
    NegativeInteger,
    Long,
    Int,
    Short,
    Byte,
    UnsignedLong,
    UnsignedInt,
    UnsignedShort,
    UnsignedByte,
    Double,
    Float,
    String,
    NormalizedString,
    Token,
    Language,
    Name,
    NcName,
    NmToken,
    Boolean,
    HexBinary,
    Base64Binary,
    AnyUri,
    DateTime,
    DateTimeStamp,
}

/// How a datatype's lexical forms are read.
#[derive(Debug, Clone, Copy)]
enum Lexical {
    /// Not read: the value of such a literal is unknown (`rdfs:Literal`).
    Unread,
    /// No lexical form is in the lexical space (`owl:real`).
    Empty,
    Integer,
    Decimal,
    Rational,
    Double,
    Float,
    /// The form is the string.
    String,
    PlainLiteral,
    /// The form and the literal's language tag.
    LangString,
    XmlLiteral,
    Boolean,
    HexBinary,
    Base64Binary,
    AnyUri,
    DateTime,
}

/// Which values a value space holds.
#[derive(Debug, Clone, Copy)]
enum Space {
    /// Every value (`rdfs:Literal`).
    All,
    /// Every real number (`owl:real`, `owl:rational`).
    Real,
    Decimal,
    /// The integers from `min` to `max`, each bound included.
    Integer {
        min: Option<i128>,
        max: Option<i128>,
    },
    Double,
    Float,
    /// The strings the pattern matches.
    String(fn(&str) -> bool),
    /// The strings and the language-tagged strings.
    PlainLiteral,
    LangString,
    XmlLiteral,
    Boolean,
    HexBinary,
    Base64Binary,
    AnyUri,
    /// The date-times; with `zoned`, only those with an offset.
    DateTime {
        zoned: bool,
    },
}

/// One datatype of the map.
#[derive(Debug)]
struct Definition {
    datatype: Datatype,
    id: TermId,
    lexical: Lexical,
    space: Space,
}

const fn def(datatype: Datatype, id: TermId, lexical: Lexical, space: Space) -> Definition {
    Definition {
        datatype,
        id,
        lexical,
        space,
    }
}

const fn integers(min: Option<i128>, max: Option<i128>) -> Space {
    Space::Integer { min, max }
}

fn any_string(_: &str) -> bool {
    true
}

/// The OWL 2 RL datatype map, in [`Datatype`]'s order. `rdf:langString` is
/// RDF 1.1's, which version 1 checked; OWL 2 RL names it through
/// `rdf:PlainLiteral`.
#[rustfmt::skip]
const MAP: [Definition; 35] = {
    use Datatype as D;
    use Lexical as L;
    [
        def(D::RdfsLiteral, vocab::RDFS_LITERAL, L::Unread, Space::All),
        def(D::PlainLiteral, vocab::RDF_PLAIN_LITERAL, L::PlainLiteral, Space::PlainLiteral),
        def(D::LangString, vocab::RDF_LANG_STRING, L::LangString, Space::LangString),
        def(D::XmlLiteral, vocab::RDF_XML_LITERAL, L::XmlLiteral, Space::XmlLiteral),
        def(D::Real, vocab::OWL_REAL, L::Empty, Space::Real),
        def(D::Rational, vocab::OWL_RATIONAL, L::Rational, Space::Real),
        def(D::Decimal, vocab::XSD_DECIMAL, L::Decimal, Space::Decimal),
        def(D::Integer, vocab::XSD_INTEGER, L::Integer, integers(None, None)),
        def(D::NonNegativeInteger, vocab::XSD_NON_NEGATIVE_INTEGER, L::Integer, integers(Some(0), None)),
        def(D::NonPositiveInteger, vocab::XSD_NON_POSITIVE_INTEGER, L::Integer, integers(None, Some(0))),
        def(D::PositiveInteger, vocab::XSD_POSITIVE_INTEGER, L::Integer, integers(Some(1), None)),
        def(D::NegativeInteger, vocab::XSD_NEGATIVE_INTEGER, L::Integer, integers(None, Some(-1))),
        def(D::Long, vocab::XSD_LONG, L::Integer, integers(Some(i64::MIN as i128), Some(i64::MAX as i128))),
        def(D::Int, vocab::XSD_INT, L::Integer, integers(Some(i32::MIN as i128), Some(i32::MAX as i128))),
        def(D::Short, vocab::XSD_SHORT, L::Integer, integers(Some(i16::MIN as i128), Some(i16::MAX as i128))),
        def(D::Byte, vocab::XSD_BYTE, L::Integer, integers(Some(i8::MIN as i128), Some(i8::MAX as i128))),
        def(D::UnsignedLong, vocab::XSD_UNSIGNED_LONG, L::Integer, integers(Some(0), Some(u64::MAX as i128))),
        def(D::UnsignedInt, vocab::XSD_UNSIGNED_INT, L::Integer, integers(Some(0), Some(u32::MAX as i128))),
        def(D::UnsignedShort, vocab::XSD_UNSIGNED_SHORT, L::Integer, integers(Some(0), Some(u16::MAX as i128))),
        def(D::UnsignedByte, vocab::XSD_UNSIGNED_BYTE, L::Integer, integers(Some(0), Some(u8::MAX as i128))),
        def(D::Double, vocab::XSD_DOUBLE, L::Double, Space::Double),
        def(D::Float, vocab::XSD_FLOAT, L::Float, Space::Float),
        def(D::String, vocab::XSD_STRING, L::String, Space::String(any_string)),
        def(D::NormalizedString, vocab::XSD_NORMALIZED_STRING, L::String, Space::String(lexical::normalized_string)),
        def(D::Token, vocab::XSD_TOKEN, L::String, Space::String(lexical::token)),
        def(D::Language, vocab::XSD_LANGUAGE, L::String, Space::String(lexical::language)),
        def(D::Name, vocab::XSD_NAME, L::String, Space::String(lexical::name)),
        def(D::NcName, vocab::XSD_NCNAME, L::String, Space::String(lexical::ncname)),
        def(D::NmToken, vocab::XSD_NMTOKEN, L::String, Space::String(lexical::nmtoken)),
        def(D::Boolean, vocab::XSD_BOOLEAN, L::Boolean, Space::Boolean),
        def(D::HexBinary, vocab::XSD_HEX_BINARY, L::HexBinary, Space::HexBinary),
        def(D::Base64Binary, vocab::XSD_BASE64_BINARY, L::Base64Binary, Space::Base64Binary),
        def(D::AnyUri, vocab::XSD_ANY_URI, L::AnyUri, Space::AnyUri),
        def(D::DateTime, vocab::XSD_DATE_TIME, L::DateTime, Space::DateTime { zoned: false }),
        def(D::DateTimeStamp, vocab::XSD_DATE_TIME_STAMP, L::DateTime, Space::DateTime { zoned: true }),
    ]
};

/// The datatype of each vocabulary id, from [`MAP`].
const BY_ID: [Option<Datatype>; vocab::IRIS.len()] = {
    let mut by_id = [None; vocab::IRIS.len()];
    let mut i = 0;
    while i < MAP.len() {
        by_id[MAP[i].id.0 as usize] = Some(MAP[i].datatype);
        i += 1;
    }
    by_id
};

impl Datatype {
    /// Every datatype of the map, in its order.
    pub(crate) const ALL: [Datatype; 35] = {
        let mut all = [Datatype::RdfsLiteral; 35];
        let mut i = 0;
        while i < MAP.len() {
            // [`Datatype::definition`] reads the table at the variant's place.
            assert!(MAP[i].datatype as usize == i, "MAP is in Datatype's order");
            all[i] = MAP[i].datatype;
            i += 1;
        }
        all
    };

    const fn definition(self) -> &'static Definition {
        &MAP[self as usize]
    }

    /// The datatype a term id names, if it is one of the map's.
    #[inline]
    pub(crate) fn of(id: TermId) -> Option<Datatype> {
        BY_ID.get(id.index()).copied().flatten()
    }

    /// The datatype's IRI, as a vocabulary id (const: dt-type1's facts,
    /// `rules/trivial.rs`, are made from [`Datatype::ALL`]).
    pub(crate) const fn id(self) -> TermId {
        self.definition().id
    }

    /// Whether the datatype's value space holds `value`.
    pub(crate) fn holds(self, value: &Value<'_>) -> bool {
        match (self.definition().space, value) {
            (Space::All, _)
            | (Space::Real, Value::Real(_))
            | (Space::Decimal, Value::Real(Real::Decimal { .. }))
            | (Space::Double, Value::Double(_))
            | (Space::Float, Value::Float(_))
            | (Space::PlainLiteral, Value::String(_) | Value::LangString { .. })
            | (Space::LangString, Value::LangString { .. })
            | (Space::XmlLiteral, Value::XmlLiteral(_))
            | (Space::Boolean, Value::Boolean(_))
            | (Space::HexBinary, Value::HexBinary(_))
            | (Space::Base64Binary, Value::Base64Binary(_))
            | (Space::AnyUri, Value::AnyUri(_)) => true,
            (Space::Integer { min, max }, Value::Real(Real::Decimal { negative, int, frac })) => {
                frac.is_empty() && {
                    // A bound is at most 2^64, so a value of more digits
                    // than i128 holds is past every bound on its side.
                    let v: Option<i128> = int.parse().ok().map(|v: i128| if *negative { -v } else { v });
                    let v = if int.is_empty() { Some(0) } else { v };
                    min.is_none_or(|min| v.map_or(!negative, |v| v >= min))
                        && max.is_none_or(|max| v.map_or(*negative, |v| v <= max))
                }
            }
            (Space::String(pattern), Value::String(s)) => pattern(s),
            (Space::DateTime { zoned }, Value::DateTime(d)) => !zoned || d.offset.is_some(),
            _ => false,
        }
    }

    /// The datatype's lexical-to-value mapping: the value of the lexical
    /// form (and language tag) in this datatype, or `None` when the literal
    /// is ill-typed. `Unread` for `rdfs:Literal`.
    fn read<'a>(self, lexical: &'a str, language: Option<&'a str>) -> Reading<'a> {
        let value = match self.definition().lexical {
            Lexical::Unread => return Reading::Unknown,
            Lexical::Empty => None,
            Lexical::Integer => lexical::integer(lexical).map(Value::Real),
            Lexical::Decimal => lexical::decimal(lexical).map(Value::Real),
            Lexical::Rational => lexical::rational(lexical).map(Value::Real),
            Lexical::Double => lexical::double(lexical).map(Value::Double),
            Lexical::Float => lexical::float(lexical).map(Value::Float),
            Lexical::String => Some(Value::String(lexical)),
            Lexical::PlainLiteral => lexical::plain_literal(lexical),
            Lexical::LangString => language.map(|tag| Value::LangString {
                text: lexical,
                tag: lexical::lower_case(tag),
            }),
            Lexical::XmlLiteral => lexical::xml(lexical).then_some(Value::XmlLiteral(lexical)),
            Lexical::Boolean => lexical::boolean(lexical).map(Value::Boolean),
            Lexical::HexBinary => lexical::hex_binary(lexical).map(Value::HexBinary),
            Lexical::Base64Binary => lexical::base64_binary(lexical).map(Value::Base64Binary),
            Lexical::AnyUri => Some(Value::AnyUri(lexical)),
            Lexical::DateTime => lexical::date_time(lexical).map(Value::DateTime),
        };
        match value {
            Some(v) if self.holds(&v) => Reading::Value(v),
            _ => Reading::IllTyped,
        }
    }
}

/// What the map says of a term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reading<'a> {
    /// Not a literal, or a literal whose datatype is not in the map (or is
    /// `rdfs:Literal`): its value is unknown.
    Unknown,
    /// A literal of a datatype of the map that has no value: its form is
    /// outside the datatype's lexical space, or reads to a value outside
    /// its value space.
    IllTyped,
    Value(Value<'a>),
}

/// The value of a term, as the map reads it.
#[inline]
pub(crate) fn value_of(term: Term<'_>) -> Reading<'_> {
    let Term::Literal {
        lexical,
        datatype,
        language,
    } = term
    else {
        return Reading::Unknown;
    };
    match Datatype::of(datatype) {
        Some(d) => d.read(lexical, language),
        None => Reading::Unknown,
    }
}

/// A data value, borrowed from the literal it was read from where it can be.
/// `==` is the identity of data values (see the module doc).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Value<'a> {
    Real(Real<'a>),
    /// The bits of the double; one NaN.
    Double(u64),
    /// The bits of the float; one NaN.
    Float(u32),
    String(&'a str),
    /// The tag in lower case.
    LangString {
        text: &'a str,
        tag: Cow<'a, str>,
    },
    XmlLiteral(&'a str),
    Boolean(bool),
    /// The hexadecimal digits in upper case.
    HexBinary(Cow<'a, str>),
    /// The base64 symbols without spaces.
    Base64Binary(Cow<'a, str>),
    AnyUri(&'a str),
    DateTime(DateTime<'a>),
}

impl Value<'_> {
    /// Every datatype whose value space holds this value, in the map's order.
    #[cfg_attr(not(test), expect(dead_code, reason = "dt-type2 (ticket 10) reads it"))]
    pub(crate) fn datatypes(&self) -> impl Iterator<Item = Datatype> + '_ {
        Datatype::ALL.into_iter().filter(|d| d.holds(self))
    }
}

/// A real number with a literal form, in one canonical form, so `==` is
/// numeric equality.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Real<'a> {
    /// A decimal: the integer digits without leading zeros and the fraction
    /// digits without trailing zeros (each empty when zero); zero is not
    /// negative.
    Decimal {
        negative: bool,
        int: Cow<'a, str>,
        frac: Cow<'a, str>,
    },
    /// A rational no decimal equals, in lowest terms: the denominator has a
    /// prime factor other than 2 and 5.
    Fraction {
        negative: bool,
        num: Box<str>,
        den: Box<str>,
    },
}

/// An `xsd:dateTime` value: its fields as written, with `24:00:00` read as
/// the next day's start. With the same offset, the same fields are the same
/// instant, so `==` is identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct DateTime<'a> {
    pub(super) negative: bool,
    /// The year's digits without leading zeros (empty for year 0).
    pub(super) year: Cow<'a, str>,
    pub(super) month: u8,
    pub(super) day: u8,
    pub(super) hour: u8,
    pub(super) minute: u8,
    pub(super) second: u8,
    /// The fraction of the second, without trailing zeros.
    pub(super) frac: &'a str,
    /// The time zone offset in minutes, if any.
    pub(crate) offset: Option<i16>,
}
