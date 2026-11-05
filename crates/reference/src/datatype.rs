//! The five datatypes `reasonable` checks, and nothing else.
//!
//! `RDFS.DATATYPE` asks whether a literal's lexical form is well-formed for
//! its own datatype; `RDFS.DATATYPE_RANGE` asks whether an object is in the
//! value space of a property's range. Any other datatype answers `None`:
//! not checked.

use crate::vocab::{RDF_LANG_STRING, RDF_XML_LITERAL, XSD_INT, XSD_INTEGER, XSD_STRING};
use oxrdf::{Literal, Term};

/// Whether `literal` is well-formed for its datatype, or `None` when the
/// datatype is not one of the five.
pub(crate) fn well_formed(literal: &Literal) -> Option<bool> {
    well_formed_as(literal, literal.datatype().as_str())
}

fn well_formed_as(literal: &Literal, datatype: &str) -> Option<bool> {
    let lexical = literal.value();
    Some(match datatype {
        XSD_STRING => true,
        XSD_INTEGER => integer(lexical),
        XSD_INT => integer(lexical) && lexical.parse::<i32>().is_ok(),
        RDF_LANG_STRING => literal.language().is_some(),
        RDF_XML_LITERAL => xml(lexical),
        _ => return None,
    })
}

/// Whether `term` is in the value space of the datatype `range`, or `None`
/// when it is not checked. `xsd:int` is the only subtype of `xsd:integer`
/// accepted.
///
/// A term that is not a literal is in no value space, whatever `range` is:
/// `reasonable` answers `false` before it looks at the range
/// (`term_in_datatype_space`, `lib/src/reasoner.rs:2397-2406` at
/// `fecee2b`), so an IRI or blank-node object of a property with a class
/// range (`ex:p rdfs:range ex:C . ex:a ex:p ex:b`) is a range clash. That
/// is a bug, but no documented difference (D1 to D10) covers it, so it is
/// reproduced here; making it one is a decision for the parity spec.
pub(crate) fn in_value_space(term: &Term, range: &str) -> Option<bool> {
    let Term::Literal(literal) = term else {
        return Some(false);
    };
    let datatype = literal.datatype().as_str();
    let well_formed = || well_formed(literal) == Some(true);
    Some(match range {
        XSD_STRING => literal.language().is_none() && datatype == XSD_STRING,
        XSD_INTEGER => (datatype == XSD_INTEGER || datatype == XSD_INT) && well_formed(),
        XSD_INT => datatype == XSD_INT && well_formed(),
        RDF_LANG_STRING => literal.language().is_some(),
        RDF_XML_LITERAL => datatype == RDF_XML_LITERAL && well_formed(),
        _ => return None,
    })
}

/// An optional sign, then one or more ASCII digits.
fn integer(lexical: &str) -> bool {
    let digits = lexical.strip_prefix(['+', '-']).unwrap_or(lexical);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// `reasonable` reads the value with quick-xml, checking end-tag names, to
/// the end. This is the same test for the tags that matter: an end tag must
/// close the innermost open element, and a markup construct must be closed
/// by its `>`. Elements still open at the end are accepted (quick-xml's
/// reader does not report them), and so is plain text.
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
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxrdf::NamedNode;

    fn typed(value: &str, datatype: &str) -> Literal {
        Literal::new_typed_literal(value, NamedNode::new_unchecked(datatype))
    }

    #[test]
    fn integers_follow_reasonable() {
        for (value, ok) in [
            ("+5", true),
            ("-7", true),
            ("42", true),
            (" 42", false),
            ("abc", false),
            ("", false),
        ] {
            assert_eq!(well_formed(&typed(value, XSD_INTEGER)), Some(ok), "{value:?}");
        }
        for (value, ok) in [
            ("-7", true),
            ("2147483647", true),
            ("2147483648", false),
            (" 42", false),
        ] {
            assert_eq!(well_formed(&typed(value, XSD_INT)), Some(ok), "{value:?}");
        }
        assert_eq!(well_formed(&typed("x", "http://example.org/dt")), None);
    }

    #[test]
    fn xml_checks_end_tag_names_only() {
        for (value, ok) in [
            ("<a>t</a>", true),
            ("plain text", true),
            ("<a><b/></a>", true),
            ("<a x=\"1\">t</a>", true),
            ("<!-- c --><a/>", true),
            ("<a>", true),
            ("<a>t</b>", false),
            ("</a>", false),
            ("<a", false),
        ] {
            assert_eq!(xml(value), ok, "{value:?}");
        }
    }

    #[test]
    fn value_spaces() {
        let int = Term::from(typed("5", XSD_INT));
        assert_eq!(in_value_space(&int, XSD_INTEGER), Some(true));
        assert_eq!(in_value_space(&int, XSD_STRING), Some(false));
        let iri = Term::from(NamedNode::new_unchecked("http://example.org/a"));
        assert_eq!(in_value_space(&iri, XSD_STRING), Some(false));
        // As `reasonable`: a non-literal clashes with any range.
        assert_eq!(in_value_space(&iri, "http://example.org/C"), Some(false));
        let lit = Term::from(typed("1", XSD_INT));
        assert_eq!(in_value_space(&lit, "http://example.org/C"), None);
        let long = Term::from(typed("5", "http://www.w3.org/2001/XMLSchema#long"));
        assert_eq!(in_value_space(&long, XSD_INTEGER), Some(false));
    }
}
