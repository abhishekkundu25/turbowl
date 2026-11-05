//! The datatype map's edge cases: for each datatype, lexical forms at the
//! edges of its lexical space (bounds, whitespace, signs, exponents, special
//! float values, time zones); the value spaces each value is in (the
//! integer subtypes inside the decimals); and the identity of values. The
//! reference evaluator's map (`crates/reference/src/datatype.rs`) passes
//! the same table, and chaos draws literals from it.

use super::lexical;
use super::map::{Datatype, Reading, Value, value_of};
use crate::term::{Term, TermId, vocab};
use std::collections::BTreeSet;

fn read(lexical: &str, datatype: TermId) -> Reading<'_> {
    value_of(Term::Literal {
        lexical,
        datatype,
        language: None,
    })
}

fn value(lexical: &str, datatype: TermId) -> Value<'_> {
    match read(lexical, datatype) {
        Reading::Value(v) => v,
        other => panic!("{lexical:?}^^{datatype:?}: {other:?}"),
    }
}

/// Lexical forms and whether each is well-typed, per datatype.
#[rustfmt::skip]
const LEXICAL: &[(TermId, &[(&str, bool)])] = &[
    (vocab::OWL_REAL, &[("1", false), ("1.5", false), ("1/3", false), ("", false)]),
    (vocab::OWL_RATIONAL, &[
        ("1/3", true), ("-1/3", true), ("+1/3", true), ("0/5", true), ("-0/5", true), ("4/2", true),
        ("007/010", true), ("123456789012345678901234567890123456789012/7", true),
        ("1/0", false), ("1/00", false), ("1/-3", false), ("1/+3", false), ("1 / 3", false), (" 1/3", false),
        ("1/3 ", false), ("1", false), ("1.5/2", false), ("/3", false), ("1/", false), ("1//3", false),
        ("1/3/5", false), ("", false),
    ]),
    (vocab::XSD_DECIMAL, &[
        ("0", true), ("-0", true), ("+0.0", true), ("1.", true), (".5", true), ("-.5", true),
        ("007.500", true), ("123456789012345678901234567890.123456789", true),
        (".", false), ("", false), ("+", false), ("-.", false), ("1e3", false), ("1E3", false),
        (" 1", false), ("1 ", false), ("1,5", false), ("INF", false), ("NaN", false), ("1.2.3", false),
        ("--1", false), ("+-1", false), ("\u{663}", false), ("1_000", false),
    ]),
    (vocab::XSD_INTEGER, &[
        ("+5", true), ("-7", true), ("42", true), ("007", true), ("0", true), ("-0", true),
        ("99999999999999999999999", true),
        (" 42", false), ("42 ", false), ("4 2", false), ("abc", false), ("", false), ("+", false),
        ("-", false), ("+-5", false), ("1.0", false), ("1.", false), ("1e3", false), ("\u{663}", false),
    ]),
    (vocab::XSD_NON_NEGATIVE_INTEGER, &[
        ("0", true), ("-0", true), ("+0", true), ("1", true),
        ("99999999999999999999999999999999999999999", true), ("-1", false),
        ("-99999999999999999999999999999999999999999", false),
    ]),
    (vocab::XSD_NON_POSITIVE_INTEGER, &[
        ("0", true), ("-0", true), ("+0", true), ("-1", true),
        ("-99999999999999999999999999999999999999999", true), ("1", false),
        ("99999999999999999999999999999999999999999", false),
    ]),
    (vocab::XSD_POSITIVE_INTEGER, &[("1", true), ("+0001", true), ("0", false), ("-0", false), ("-1", false)]),
    (vocab::XSD_NEGATIVE_INTEGER, &[("-1", true), ("-0001", true), ("0", false), ("-0", false), ("1", false)]),
    (vocab::XSD_LONG, &[
        ("9223372036854775807", true), ("-9223372036854775808", true),
        ("9223372036854775808", false), ("-9223372036854775809", false),
    ]),
    (vocab::XSD_INT, &[
        ("-7", true), ("+7", true), ("2147483647", true), ("+2147483647", true), ("-2147483648", true),
        ("0002147483647", true), ("-0002147483648", true), ("-0", true),
        ("2147483648", false), ("-2147483649", false), ("4294967296", false), ("99999999999", false),
        ("99999999999999999999999", false), (" 42", false), (" 3 ", false), ("", false), ("abc", false),
    ]),
    (vocab::XSD_SHORT, &[("32767", true), ("-32768", true), ("32768", false), ("-32769", false)]),
    (vocab::XSD_BYTE, &[("127", true), ("-128", true), ("128", false), ("-129", false), ("1.0", false)]),
    (vocab::XSD_UNSIGNED_LONG, &[
        ("18446744073709551615", true), ("0", true), ("-0", true),
        ("18446744073709551616", false), ("-1", false),
    ]),
    (vocab::XSD_UNSIGNED_INT, &[("4294967295", true), ("4294967296", false), ("-1", false)]),
    (vocab::XSD_UNSIGNED_SHORT, &[("65535", true), ("65536", false)]),
    (vocab::XSD_UNSIGNED_BYTE, &[("255", true), ("+0", true), ("256", false), ("-1", false)]),
    (vocab::XSD_DOUBLE, &[
        ("1", true), ("-1.5E-3", true), ("1e5", true), ("1.e5", true), (".5e-1", true), ("+1E+400", true),
        ("1E400", true), ("1e-400", true), ("-0", true), ("0.0e0", true), ("INF", true), ("+INF", true),
        ("-INF", true), ("NaN", true), ("9007199254740993", true), ("1e00000000000000000000000001", true),
        ("inf", false), ("Infinity", false), ("nan", false), ("-NaN", false), ("+NaN", false), ("1e", false),
        ("e5", false), (".e5", false), ("1e5.0", false), (" 1", false), ("1 ", false), ("", false),
        (".", false), ("1E+", false), ("0x10", false), ("1d", false), ("1f", false), ("- 1", false),
    ]),
    (vocab::XSD_FLOAT, &[
        ("1", true), ("1E40", true), ("3.4028235E38", true), ("-1e-50", true), ("INF", true), ("NaN", true),
        ("16777217", true), ("Inf", false), (" 1", false), ("1.5f", false), ("", false),
    ]),
    (vocab::XSD_STRING, &[("", true), (" x ", true), ("a\tb\nc", true), ("<a>", true)]),
    (vocab::XSD_NORMALIZED_STRING, &[
        ("", true), ("a b", true), (" a  b ", true), ("a\tb", false), ("a\nb", false), ("a\rb", false),
    ]),
    (vocab::XSD_TOKEN, &[
        ("", true), ("a b", true), ("a", true), (" a", false), ("a ", false), ("a  b", false), ("a\tb", false),
    ]),
    (vocab::XSD_LANGUAGE, &[
        ("en", true), ("en-GB", true), ("zh-Hant-TW", true), ("x-1", true), ("abcdefgh", true),
        ("abcdefghi", false), ("en-", false), ("-en", false), ("", false), ("en_GB", false),
        ("en-abcdefghi", false), ("1en", false), (" en", false), ("en--GB", false),
    ]),
    (vocab::XSD_NAME, &[
        ("a", true), ("_a", true), (":a", true), ("a:b", true), ("a-1.b", true), ("\u{e9}t\u{e9}", true),
        ("a\u{b7}b", true), ("1a", false), ("-a", false), (".a", false), ("a b", false), ("", false),
        ("\u{b7}a", false), ("a,b", false),
    ]),
    (vocab::XSD_NCNAME, &[
        ("a", true), ("_a-1", true), ("a:b", false), (":a", false), ("1a", false), ("", false),
    ]),
    (vocab::XSD_NMTOKEN, &[
        ("1a", true), ("-.", true), ("a:b", true), ("\u{b7}", true), ("", false), ("a b", false),
        ("a,b", false), (" a", false),
    ]),
    (vocab::XSD_BOOLEAN, &[
        ("true", true), ("false", true), ("1", true), ("0", true), ("TRUE", false), ("True", false),
        ("yes", false), (" true", false), ("", false), ("01", false), ("-0", false),
    ]),
    (vocab::XSD_HEX_BINARY, &[
        ("", true), ("0FB7", true), ("0fb7", true), ("0FB", false), ("0G", false), (" 0F", false),
        ("0F ", false), ("0x0F", false),
    ]),
    (vocab::XSD_BASE64_BINARY, &[
        ("", true), ("QQ==", true), ("QUI=", true), ("QUJD", true), ("QUJDRA==", true), ("Q Q = =", true),
        ("QUJD RA==", true), ("QQ= =", true), ("a+/9", true),
        ("QR==", false), ("QUJ=", false), ("QUI", false), ("Q===", false), ("=QQQ", false), (" QQ==", false),
        ("QQ== ", false), ("QU  I=", false), ("QQ=A", false), ("QUJD!A==", false), ("QQ", false),
        ("Q\tQ==", false),
    ]),
    (vocab::XSD_ANY_URI, &[("http://example.org/", true), ("", true), ("relative/path", true), ("?q", true), (":x", true), ("a b", true)]),
    (vocab::XSD_DATE_TIME, &[
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
    (vocab::XSD_DATE_TIME_STAMP, &[
        ("2000-01-01T00:00:00Z", true), ("2000-01-01T00:00:00-05:00", true), ("2000-01-01T00:00:00", false),
        ("2000-02-30T00:00:00Z", false),
    ]),
    (vocab::RDF_PLAIN_LITERAL, &[
        ("abc@", true), ("abc@en", true), ("abc@EN-gb", true), ("a@b@en", true), ("@", true), ("@en", true),
        ("abc", false), ("", false), ("abc@en-", false), ("abc@1en", false), ("abc@en gb", false),
        ("abc@en@", true),
    ]),
    (vocab::RDF_XML_LITERAL, &[
        ("<a>t</a>", true), ("plain text", true), ("", true), ("<a><b/></a>", true), ("<a x=\"1\">t</a>", true),
        ("<a >t</a >", true), ("<!-- c --><a/>", true), ("<![CDATA[<a>]]>", true), ("<?pi x?><a/>", true),
        ("<!DOCTYPE a><a/>", true), ("<a>", false), ("<a>t</b>", false), ("</a>", false), ("<a", false),
        ("<!-- c", false), ("<a><b></a></b>", false), ("<", false),
    ]),
];

#[test]
fn lexical_forms_at_the_edges_of_each_datatype() {
    let mut failures = Vec::new();
    for &(datatype, cases) in LEXICAL {
        for &(lexical, ok) in cases {
            let got = read(lexical, datatype);
            if matches!(got, Reading::Value(_)) != ok || got == Reading::Unknown {
                failures.push(format!("{lexical:?}^^{}: {got:?}", vocab::NAMES[datatype.index()]));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_datatype_of_the_map_has_edge_cases() {
    let covered: BTreeSet<_> = LEXICAL.iter().map(|(d, _)| d.0).collect();
    for d in Datatype::ALL {
        let exempt = matches!(d, Datatype::RdfsLiteral | Datatype::LangString);
        assert!(exempt || covered.contains(&d.id().0), "{d:?}");
    }
}

#[test]
fn language_strings_and_literals_outside_the_map() {
    let tagged = |tag| {
        value_of(Term::Literal {
            lexical: "chat",
            datatype: vocab::RDF_LANG_STRING,
            language: tag,
        })
    };
    assert!(matches!(tagged(Some("fr")), Reading::Value(Value::LangString { .. })));
    // No parser makes one, but a caller can insert it.
    assert_eq!(tagged(None), Reading::IllTyped);
    // rdfs:Literal is the top data range: its literals are not read.
    assert_eq!(read("x", vocab::RDFS_LITERAL), Reading::Unknown);
    assert_eq!(read("x", vocab::RDF_TYPE), Reading::Unknown);
    assert_eq!(value_of(Term::Iri("http://example.org/x")), Reading::Unknown);
}

/// The value spaces each value is in: every inclusion of the map.
#[test]
fn the_value_spaces_that_hold_each_value() {
    use Datatype::*;
    const NUMBER: &[Datatype] = &[RdfsLiteral, Real, Rational, Decimal];
    const INTEGER: &[Datatype] = &[RdfsLiteral, Real, Rational, Decimal, Integer];
    const SIGNED: &[Datatype] = &[Long, Int, Short, Byte];
    const UNSIGNED: &[Datatype] = &[UnsignedLong, UnsignedInt, UnsignedShort, UnsignedByte];
    let all = |parts: &[&[Datatype]]| -> BTreeSet<std::string::String> {
        parts.iter().flat_map(|p| p.iter()).map(|d| format!("{d:?}")).collect()
    };
    #[rustfmt::skip]
    let cases: Vec<(&str, TermId, BTreeSet<std::string::String>)> = vec![
        ("5", vocab::XSD_INT, all(&[INTEGER, &[NonNegativeInteger, PositiveInteger], SIGNED, UNSIGNED])),
        ("+5.000", vocab::XSD_DECIMAL, all(&[INTEGER, &[NonNegativeInteger, PositiveInteger], SIGNED, UNSIGNED])),
        ("10/2", vocab::OWL_RATIONAL, all(&[INTEGER, &[NonNegativeInteger, PositiveInteger], SIGNED, UNSIGNED])),
        ("-5", vocab::XSD_INTEGER, all(&[INTEGER, &[NonPositiveInteger, NegativeInteger], SIGNED])),
        ("0", vocab::XSD_UNSIGNED_BYTE, all(&[INTEGER, &[NonNegativeInteger, NonPositiveInteger], SIGNED, UNSIGNED])),
        ("-0.0", vocab::XSD_DECIMAL, all(&[INTEGER, &[NonNegativeInteger, NonPositiveInteger], SIGNED, UNSIGNED])),
        ("300", vocab::XSD_INTEGER, all(&[INTEGER, &[NonNegativeInteger, PositiveInteger, Long, Int, Short],
            &[UnsignedLong, UnsignedInt, UnsignedShort]])),
        ("-129", vocab::XSD_SHORT, all(&[INTEGER, &[NonPositiveInteger, NegativeInteger, Long, Int, Short]])),
        ("4294967296", vocab::XSD_INTEGER, all(&[INTEGER, &[NonNegativeInteger, PositiveInteger, Long, UnsignedLong]])),
        ("18446744073709551616", vocab::XSD_INTEGER, all(&[INTEGER, &[NonNegativeInteger, PositiveInteger]])),
        ("1.5", vocab::XSD_DECIMAL, all(&[NUMBER])),
        ("3/2", vocab::OWL_RATIONAL, all(&[NUMBER])),
        ("1/3", vocab::OWL_RATIONAL, all(&[&[RdfsLiteral, Real, Rational]])),
        ("1", vocab::XSD_DOUBLE, all(&[&[RdfsLiteral, Double]])),
        ("NaN", vocab::XSD_FLOAT, all(&[&[RdfsLiteral, Float]])),
        ("abc", vocab::XSD_STRING, all(&[&[RdfsLiteral, PlainLiteral, String, NormalizedString, Token, Language, Name, NcName, NmToken]])),
        ("a b", vocab::XSD_NORMALIZED_STRING, all(&[&[RdfsLiteral, PlainLiteral, String, NormalizedString, Token]])),
        ("a:b", vocab::XSD_NAME, all(&[&[RdfsLiteral, PlainLiteral, String, NormalizedString, Token, Name, NmToken]])),
        ("1a", vocab::XSD_NMTOKEN, all(&[&[RdfsLiteral, PlainLiteral, String, NormalizedString, Token, NmToken]])),
        (" a", vocab::XSD_STRING, all(&[&[RdfsLiteral, PlainLiteral, String, NormalizedString]])),
        ("a\n", vocab::XSD_STRING, all(&[&[RdfsLiteral, PlainLiteral, String]])),
        ("chat@FR", vocab::RDF_PLAIN_LITERAL, all(&[&[RdfsLiteral, PlainLiteral, LangString]])),
        ("true", vocab::XSD_BOOLEAN, all(&[&[RdfsLiteral, Boolean]])),
        ("0F", vocab::XSD_HEX_BINARY, all(&[&[RdfsLiteral, HexBinary]])),
        ("QQ==", vocab::XSD_BASE64_BINARY, all(&[&[RdfsLiteral, Base64Binary]])),
        ("http://example.org/", vocab::XSD_ANY_URI, all(&[&[RdfsLiteral, AnyUri]])),
        ("2000-01-01T00:00:00Z", vocab::XSD_DATE_TIME, all(&[&[RdfsLiteral, DateTime, DateTimeStamp]])),
        ("2000-01-01T00:00:00", vocab::XSD_DATE_TIME, all(&[&[RdfsLiteral, DateTime]])),
        ("<a/>", vocab::RDF_XML_LITERAL, all(&[&[RdfsLiteral, XmlLiteral]])),
    ];
    for (lexical, datatype, want) in cases {
        let got: BTreeSet<std::string::String> =
            value(lexical, datatype).datatypes().map(|d| format!("{d:?}")).collect();
        assert_eq!(got, want, "{lexical:?}^^{}", vocab::NAMES[datatype.index()]);
    }
}

/// Identity of values, as dt-eq and dt-diff read it: each group is one
/// value, and no two groups are.
#[test]
fn values_are_identical_by_value_not_by_form() {
    use vocab::*;
    let tagged = |text: &'static str, tag: &'static str| {
        let Reading::Value(v) = value_of(Term::Literal {
            lexical: text,
            datatype: RDF_LANG_STRING,
            language: Some(tag),
        }) else {
            unreachable!()
        };
        v
    };
    let big = "123456789012345678901234567890123456789012345";
    let (padded, tenfold) = (format!("000{big}.000"), format!("{big}0/10"));
    #[rustfmt::skip]
    let groups: Vec<Vec<Value<'_>>> = vec![
        vec![value("1", XSD_INTEGER), value("1.0", XSD_DECIMAL), value("01", XSD_INT), value("+1", XSD_BYTE),
             value("1/1", OWL_RATIONAL), value("2/2", OWL_RATIONAL), value("1.000", XSD_DECIMAL),
             value("1", XSD_UNSIGNED_BYTE), value("1", XSD_POSITIVE_INTEGER)],
        vec![value("0", XSD_INTEGER), value("-0", XSD_INTEGER), value("-0.0", XSD_DECIMAL), value("0/7", OWL_RATIONAL),
             value(".0", XSD_DECIMAL)],
        vec![value("0.5", XSD_DECIMAL), value("1/2", OWL_RATIONAL), value(".50", XSD_DECIMAL)],
        vec![value("-0.25", XSD_DECIMAL), value("-1/4", OWL_RATIONAL), value("-3/12", OWL_RATIONAL)],
        vec![value("0.125", XSD_DECIMAL), value("1/8", OWL_RATIONAL)],
        vec![value("1/3", OWL_RATIONAL), value("2/6", OWL_RATIONAL)],
        vec![value("-1/3", OWL_RATIONAL)],
        vec![value("100000000000000000000000000000000000000001/3", OWL_RATIONAL),
             value("200000000000000000000000000000000000000002/6", OWL_RATIONAL)],
        vec![value(big, XSD_INTEGER), value(&padded, XSD_DECIMAL), value(&tenfold, OWL_RATIONAL)],
        vec![value("1", XSD_DOUBLE), value("1.0", XSD_DOUBLE), value("1e0", XSD_DOUBLE), value("0.1E1", XSD_DOUBLE)],
        vec![value("1", XSD_FLOAT), value("1.00000001", XSD_FLOAT)],
        vec![value("0", XSD_DOUBLE), value("0.0e5", XSD_DOUBLE)],
        vec![value("-0", XSD_DOUBLE), value("-1e-400", XSD_DOUBLE)],
        vec![value("NaN", XSD_DOUBLE)],
        vec![value("NaN", XSD_FLOAT)],
        vec![value("INF", XSD_DOUBLE), value("+INF", XSD_DOUBLE), value("1E400", XSD_DOUBLE)],
        vec![value("INF", XSD_FLOAT), value("1E39", XSD_FLOAT)],
        vec![value("9007199254740993", XSD_DOUBLE), value("9007199254740992", XSD_DOUBLE)],
        vec![value("16777217", XSD_FLOAT), value("16777216", XSD_FLOAT)],
        vec![value("a", XSD_STRING), value("a", XSD_TOKEN), value("a", XSD_NCNAME), value("a@", RDF_PLAIN_LITERAL)],
        vec![value("a", XSD_ANY_URI)],
        vec![tagged("a", "EN"), tagged("a", "en"), value("a@En", RDF_PLAIN_LITERAL)],
        vec![tagged("a", "fr")],
        vec![value("0fb7", XSD_HEX_BINARY), value("0FB7", XSD_HEX_BINARY)],
        vec![value("QQ==", XSD_BASE64_BINARY), value("Q Q = =", XSD_BASE64_BINARY)],
        vec![value("true", XSD_BOOLEAN), value("1", XSD_BOOLEAN)],
        vec![value("false", XSD_BOOLEAN), value("0", XSD_BOOLEAN)],
        vec![value("2000-01-01T24:00:00", XSD_DATE_TIME), value("2000-01-02T00:00:00", XSD_DATE_TIME),
             value("2000-01-02T00:00:00.000", XSD_DATE_TIME)],
        vec![value("1999-12-31T24:00:00Z", XSD_DATE_TIME), value("2000-01-01T00:00:00Z", XSD_DATE_TIME),
             value("2000-01-01T00:00:00+00:00", XSD_DATE_TIME), value("2000-01-01T00:00:00-00:00", XSD_DATE_TIME_STAMP)],
        // The same instant with another offset: equal, not identical.
        vec![value("2000-01-01T01:00:00+01:00", XSD_DATE_TIME)],
        vec![value("2000-01-01T00:00:00", XSD_DATE_TIME)],
        vec![value("2000-01-01T00:00:00.5", XSD_DATE_TIME), value("2000-01-01T00:00:00.500", XSD_DATE_TIME)],
        vec![value("2000-02-28T24:00:00", XSD_DATE_TIME), value("2000-02-29T00:00:00", XSD_DATE_TIME)],
        vec![value("2001-02-28T24:00:00", XSD_DATE_TIME), value("2001-03-01T00:00:00", XSD_DATE_TIME)],
        vec![value("-0001-12-31T24:00:00", XSD_DATE_TIME), value("0000-01-01T00:00:00", XSD_DATE_TIME),
             value("-0000-01-01T00:00:00", XSD_DATE_TIME)],
        vec![value("9999-12-31T24:00:00", XSD_DATE_TIME), value("10000-01-01T00:00:00", XSD_DATE_TIME)],
        vec![value("-1000-12-31T24:00:00", XSD_DATE_TIME), value("-0999-01-01T00:00:00", XSD_DATE_TIME)],
        vec![value("<a/>", RDF_XML_LITERAL)],
    ];
    for (i, group) in groups.iter().enumerate() {
        for v in group {
            assert_eq!(v, &group[0], "group {i}");
        }
        for (j, other) in groups.iter().enumerate().skip(i + 1) {
            assert_ne!(group[0], other[0], "groups {i} and {j}");
        }
    }
}

#[test]
fn rationals_read_in_lowest_terms() {
    use super::map::Real;
    let real = |s| match lexical::rational(s) {
        Some(Real::Decimal { negative, int, frac }) => format!("{}{int}.{frac}", if negative { "-" } else { "" }),
        Some(Real::Fraction { negative, num, den }) => format!("{}{num}/{den}", if negative { "-" } else { "" }),
        None => "ill-typed".to_owned(),
    };
    for (lexical, value) in [
        ("1/3", "1/3"),
        ("-6/9", "-2/3"),
        ("3/6", ".5"),
        ("7/1", "7."),
        ("-7/20", "-.35"),
        ("1/1024", ".0009765625"),
        ("10/4", "2.5"),
        ("0/3", "."),
        ("-0/3", "."),
        ("1/0", "ill-typed"),
        (
            "12345678901234567890123456789012345678901234567890/10",
            "1234567890123456789012345678901234567890123456789.",
        ),
    ] {
        assert_eq!(real(lexical), value, "{lexical}");
    }
}
