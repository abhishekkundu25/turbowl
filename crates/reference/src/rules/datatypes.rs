//! dt-not-type, in the `checks` switch: a literal not in the value space of
//! its own datatype, or of a datatype the closure types it with. The value
//! spaces are the datatype map's (`crate::datatype`); version 2's ticket 09
//! extends the map to every OWL 2 RL datatype, and this file with it.

use super::{EMPTY, Family, Found};
use crate::datatype;
use crate::evaluate::View;
use crate::vocab::*;
use oxrdf::Term;

pub(super) const FAMILY: Family = Family {
    diagnostic_rules: &["dt-not-type"],
    diagnose,
    ..EMPTY
};

fn diagnose(v: &View, found: &mut Vec<Found>) {
    // An ill-typed literal anywhere in the closure: the literal is not in
    // its own datatype's value space.
    for (s, p, o) in &v.all {
        for term in [s, p, o] {
            if let Term::Literal(literal) = term
                && datatype::well_formed(literal) == Some(false)
            {
                let own = Term::from(literal.datatype().into_owned());
                found.push(("dt-not-type", vec![term.clone(), own]));
            }
        }
    }
    // A literal the closure types with a datatype (by prp-rng or any other
    // rule) whose value space it is not in.
    for (x, class) in v.pairs(&iri(RDF_TYPE)) {
        if let (Term::Literal(literal), Term::NamedNode(class)) = (x, class)
            && datatype::in_value_space(literal, class.as_str()) == Some(false)
        {
            found.push(("dt-not-type", vec![x.clone(), Term::from(class.clone())]));
        }
    }
}
