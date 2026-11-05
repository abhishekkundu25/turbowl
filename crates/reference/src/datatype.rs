//! The datatypes dt-not-type checks: `xsd:string`, `xsd:integer`,
//! `xsd:int`, `rdf:langString` and `rdf:XMLLiteral`. Any other datatype has
//! no known value space here, so nothing is ever found against it.
//!
//! dt-not-type fires for `lt rdf:type dt` when the literal `lt` is not in the
//! value space of `dt`. A literal is checked in two ways: against its own
//! datatype (an ill-typed literal, [`well_formed`]), and against any checked
//! datatype the closure types it with, by prp-rng or any other rule
//! ([`in_value_space`]).

use crate::vocab::{RDF_LANG_STRING, RDF_XML_LITERAL, XSD_INT, XSD_INTEGER, XSD_STRING};
use oxrdf::Literal;

/// Whether `literal` is well-formed for its own datatype, or `None` when
/// that datatype is not checked.
pub(crate) fn well_formed(literal: &Literal) -> Option<bool> {
    let lexical = literal.value();
    Some(match literal.datatype().as_str() {
        XSD_STRING => true,
        XSD_INTEGER => integer(lexical),
        XSD_INT => int(lexical),
        RDF_LANG_STRING => literal.language().is_some(),
        RDF_XML_LITERAL => xml(lexical),
        _ => return None,
    })
}

/// Whether the value of `literal` is in the value space of the datatype
/// `datatype`, or `None` when either the literal's own datatype or
/// `datatype` is not checked (its value, or the space, is unknown here).
///
/// The value spaces, as XML Schema and RDF 1.1 define them: an integer (of
/// `xsd:integer` or `xsd:int`) is in `xsd:integer`, and in `xsd:int` when
/// it fits 32 bits; strings, language-tagged strings and XML values are
/// each a space of their own. An ill-typed literal has no value, so it is
/// in no space.
pub(crate) fn in_value_space(literal: &Literal, datatype: &str) -> Option<bool> {
    let own = literal.datatype();
    let own = own.as_str();
    if !CHECKED.contains(&datatype) || !CHECKED.contains(&own) {
        return None;
    }
    if well_formed(literal) != Some(true) {
        return Some(false);
    }
    let lexical = literal.value();
    Some(match (own, datatype) {
        (XSD_INTEGER | XSD_INT, XSD_INTEGER) => true,
        (XSD_INTEGER | XSD_INT, XSD_INT) => int(lexical),
        (own, datatype) => own == datatype,
    })
}

const CHECKED: &[&str] = &[XSD_STRING, XSD_INTEGER, XSD_INT, RDF_LANG_STRING, RDF_XML_LITERAL];

/// An optional sign, then one or more ASCII digits.
fn integer(lexical: &str) -> bool {
    let digits = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// An integer from -2^31 to 2^31 - 1 (leading zeros allowed).
fn int(lexical: &str) -> bool {
    if !integer(lexical) {
        return false;
    }
    let (negative, digits) = match lexical.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, lexical.strip_prefix('+').unwrap_or(lexical)),
    };
    let digits = digits.trim_start_matches('0');
    let magnitude = if digits.is_empty() {
        Some(0)
    } else {
        digits.parse::<u64>().ok()
    };
    magnitude.is_some_and(|m| if negative { m <= 1 << 31 } else { m < 1 << 31 })
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use oxrdf::NamedNode;

    fn typed(value: &str, datatype: &str) -> Literal {
        Literal::new_typed_literal(value, NamedNode::new_unchecked(datatype))
    }

    #[test]
    fn integers() {
        for (value, ok) in [
            ("+5", true),
            ("-7", true),
            ("42", true),
            ("007", true),
            (" 42", false),
            ("abc", false),
            ("", false),
            ("+", false),
        ] {
            assert_eq!(well_formed(&typed(value, XSD_INTEGER)), Some(ok), "{value:?}");
        }
        for (value, ok) in [
            ("-7", true),
            ("+7", true),
            ("2147483647", true),
            ("-2147483648", true),
            ("0002147483647", true),
            ("2147483648", false),
            ("-2147483649", false),
            ("99999999999999999999999", false),
            (" 42", false),
        ] {
            assert_eq!(well_formed(&typed(value, XSD_INT)), Some(ok), "{value:?}");
        }
        assert_eq!(well_formed(&typed("x", "http://example.org/dt")), None);
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

    #[test]
    fn value_spaces() {
        let five_int = typed("5", XSD_INT);
        assert_eq!(in_value_space(&five_int, XSD_INTEGER), Some(true));
        assert_eq!(in_value_space(&five_int, XSD_STRING), Some(false));
        assert_eq!(in_value_space(&typed("5", XSD_INTEGER), XSD_INT), Some(true));
        assert_eq!(in_value_space(&typed("2147483648", XSD_INTEGER), XSD_INT), Some(false));
        assert_eq!(in_value_space(&typed("abc", XSD_INTEGER), XSD_INTEGER), Some(false));
        assert_eq!(
            in_value_space(&Literal::new_simple_literal("x"), XSD_STRING),
            Some(true)
        );
        assert_eq!(
            in_value_space(&Literal::new_simple_literal("x"), XSD_INTEGER),
            Some(false)
        );
        let tagged = Literal::new_language_tagged_literal_unchecked("x", "en");
        assert_eq!(in_value_space(&tagged, XSD_STRING), Some(false));
        assert_eq!(in_value_space(&tagged, RDF_LANG_STRING), Some(true));
        // Unchecked on either side: unknown.
        assert_eq!(in_value_space(&five_int, "http://example.org/C"), None);
        let decimal = typed("5", "http://www.w3.org/2001/XMLSchema#decimal");
        assert_eq!(in_value_space(&decimal, XSD_INTEGER), None);
    }
}
