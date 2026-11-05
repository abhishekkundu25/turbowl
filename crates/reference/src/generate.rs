//! Small random ontologies and data, the same for the same seed.
//!
//! A run draws `size` statements from a weighted table of kinds, over pools
//! of classes, object properties, data properties and individuals whose
//! sizes grow with `size`. The kinds cover every rule family: class and
//! property hierarchies, equivalences, domains and ranges, property
//! characteristics, property chains (well-formed, cyclic, with shared tails,
//! with IRI cells), intersections, unions, restrictions, `owl:sameAs`,
//! disjointness and complements, container membership triples (which no
//! rule of the set reads), and literals of checked and unchecked datatypes,
//! well-formed or not. Some individuals, every list
//! cell and every restriction are blank nodes.
//!
//! Literals only ever appear as objects of data properties (and of
//! `owl:hasValue` on one), so object-property rules (inverse, symmetric,
//! transitive) never meet a literal. Literals still reach subject positions
//! inside the evaluator: a data property's range types them (prp-rng), and
//! a functional data property makes two of them the same (prp-fp).

use crate::vocab::*;
use oxrdf::{BlankNode, Literal, NamedNode, NamedOrBlankNode, Term, Triple};
use std::collections::HashSet;

/// A small seeded generator (xorshift64*), for the generator and the chaos
/// test's splits and removals.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// A generator for `seed`, mixed through splitmix64 so that neighbouring
    /// seeds start far apart (xorshift stays at 0, so the state never is).
    pub fn new(seed: u64) -> Rng {
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Rng((z ^ (z >> 31)) | 1)
    }

    /// The generator of iteration `i` of a run seeded `seed`.
    pub fn at(seed: u64, i: usize) -> Rng {
        Rng::new(seed ^ (i as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number in `0..n` (0 when `n` is 0).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// True one time in `n`.
    pub fn one_in(&mut self, n: usize) -> bool {
        self.below(n) == 0
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

const EX: &str = "http://example.org/";

/// Literals the data properties take: well-formed and ill-formed values of
/// the checked datatypes, and values of unchecked ones.
const LITERALS: &[(&str, &str)] = &[
    ("42", XSD_INT),
    ("-7", XSD_INT),
    (" 42", XSD_INT),
    ("2147483648", XSD_INT),
    ("+5", XSD_INTEGER),
    ("12", XSD_INTEGER),
    ("abc", XSD_INTEGER),
    ("hello", XSD_STRING),
    ("", XSD_STRING),
    ("<a>t</a>", RDF_XML_LITERAL),
    ("<a>t</b>", RDF_XML_LITERAL),
    ("plain", RDF_XML_LITERAL),
    ("3.5", XSD_DECIMAL),
    ("1", "http://example.org/dt"),
];

/// Datatypes a data property's range may name: the five dt-not-type checks
/// and two it does not.
const DATATYPES: &[&str] = &[
    XSD_INTEGER,
    XSD_INT,
    XSD_STRING,
    RDF_LANG_STRING,
    RDF_XML_LITERAL,
    XSD_DECIMAL,
    "http://example.org/dt",
];

/// Container membership predicates, and names like them (`_0`, `_a`): no
/// rule of the set gives them a meaning, so they derive nothing.
const MEMBERSHIP: &[&str] = &["_1", "_2", "_3", "_01", "_0", "_a"];

/// `size` statements drawn from the seed: the same seed and size always give
/// the same triples, in the same order, without duplicates.
pub fn generate(seed: u64, size: usize) -> Vec<Triple> {
    let mut g = Generator::new(Rng::new(seed), size);
    for _ in 0..size {
        g.statement();
    }
    g.out
}

struct Generator {
    rng: Rng,
    classes: Vec<Term>,
    object_properties: Vec<Term>,
    data_properties: Vec<Term>,
    individuals: Vec<Term>,
    /// The cells of every list made so far, head first, for shared tails.
    lists: Vec<Vec<Term>>,
    blank: usize,
    out: Vec<Triple>,
    seen: HashSet<Triple>,
}

impl Generator {
    fn new(rng: Rng, size: usize) -> Generator {
        let mut g = Generator {
            rng,
            classes: (0..3 + size / 8).map(|i| ex(&format!("C{i}"))).collect(),
            object_properties: (0..2 + size / 10).map(|i| ex(&format!("p{i}"))).collect(),
            data_properties: (0..1 + size / 16).map(|i| ex(&format!("d{i}"))).collect(),
            individuals: (0..3 + size / 6).map(|i| ex(&format!("i{i}"))).collect(),
            lists: Vec::new(),
            blank: 0,
            out: Vec::new(),
            seen: HashSet::new(),
        };
        for i in 0..1 + size / 20 {
            let node = Term::from(BlankNode::new_unchecked(format!("x{i}")));
            g.individuals.push(node);
        }
        g
    }

    fn add(&mut self, s: &Term, p: &str, o: &Term) {
        self.add_with(s, &named(p), o);
    }

    fn add_with(&mut self, s: &Term, p: &NamedNode, o: &Term) {
        let subject = NamedOrBlankNode::try_from(s.clone()).expect("the generator makes no literal subject");
        let triple = Triple::new(subject, p.clone(), o.clone());
        if self.seen.insert(triple.clone()) {
            self.out.push(triple);
        }
    }

    fn fresh(&mut self) -> Term {
        self.blank += 1;
        Term::from(BlankNode::new_unchecked(format!("g{}", self.blank)))
    }

    fn class(&mut self) -> Term {
        self.rng.pick(&self.classes).clone()
    }

    /// A class, now and then `owl:Thing`: for the places where `owl:Thing`
    /// is harmless. Under `owl:Thing` (`owl:Thing subClassOf C`), a
    /// restriction would apply to every individual typed `owl:Thing`, and
    /// one `owl:hasValue` on an inverse-functional property then makes all
    /// of them the same.
    fn superclass(&mut self) -> Term {
        if self.rng.one_in(20) {
            iri(OWL_THING)
        } else {
            self.class()
        }
    }

    fn object_property(&mut self) -> Term {
        self.rng.pick(&self.object_properties).clone()
    }

    fn data_property(&mut self) -> Term {
        self.rng.pick(&self.data_properties).clone()
    }

    /// A property of either kind, mostly object properties.
    fn property(&mut self) -> Term {
        if self.rng.one_in(4) {
            self.data_property()
        } else {
            self.object_property()
        }
    }

    fn individual(&mut self) -> Term {
        self.rng.pick(&self.individuals).clone()
    }

    fn literal(&mut self) -> Term {
        match self.rng.below(LITERALS.len() + 2) {
            n if n < LITERALS.len() => {
                let (value, datatype) = LITERALS[n];
                Term::from(Literal::new_typed_literal(value, named(datatype)))
            }
            n if n == LITERALS.len() => Term::from(Literal::new_language_tagged_literal_unchecked("chat", "fr")),
            _ => Term::from(Literal::new_simple_literal("x")),
        }
    }

    /// A restriction blank node: `owl:someValuesFrom` (a class or
    /// `owl:Thing`), `owl:allValuesFrom` or `owl:hasValue` on one property.
    fn restriction(&mut self) -> Term {
        let r = self.fresh();
        self.add(&r, RDF_TYPE, &iri(OWL_RESTRICTION));
        let data = self.rng.one_in(4);
        let p = if data {
            self.data_property()
        } else {
            self.object_property()
        };
        self.add(&r, OWL_ON_PROPERTY, &p);
        match self.rng.below(4) {
            0 => {
                let filler = if self.rng.one_in(2) {
                    iri(OWL_THING)
                } else {
                    self.class()
                };
                self.add(&r, OWL_SOME_VALUES_FROM, &filler);
            }
            1 => {
                let filler = self.superclass();
                self.add(&r, OWL_SOME_VALUES_FROM, &filler);
            }
            2 => {
                let filler = self.superclass();
                self.add(&r, OWL_ALL_VALUES_FROM, &filler);
            }
            _ => {
                let value = if data { self.literal() } else { self.individual() };
                self.add(&r, OWL_HAS_VALUE, &value);
            }
        }
        r
    }

    /// A list of `members`: blank cells ending in `rdf:nil`, now and then
    /// with IRI cells, or sharing the tail of an earlier list.
    fn list(&mut self, members: &[Term]) -> Term {
        let iri_cells = self.rng.one_in(8);
        let mut cells: Vec<Term> = Vec::new();
        for _ in members {
            let cell = if iri_cells {
                self.blank += 1;
                ex(&format!("list{}", self.blank))
            } else {
                self.fresh()
            };
            cells.push(cell);
        }
        let shared = if !self.lists.is_empty() && self.rng.one_in(6) {
            let earlier = self.rng.pick(&self.lists).clone();
            Some(earlier[self.rng.below(earlier.len())].clone())
        } else {
            None
        };
        for (i, (cell, member)) in cells.iter().zip(members).enumerate() {
            self.add(cell, RDF_FIRST, member);
            let rest = match (cells.get(i + 1), &shared) {
                (Some(next), _) => next.clone(),
                (None, Some(tail)) => tail.clone(),
                (None, None) => iri(RDF_NIL),
            };
            self.add(cell, RDF_REST, &rest);
        }
        self.lists.push(cells.clone());
        cells[0].clone()
    }

    /// A list whose last cell's rest points back into it: never reaches nil.
    fn cyclic_list(&mut self, members: &[Term]) -> Term {
        let cells: Vec<Term> = members.iter().map(|_| self.fresh()).collect();
        let back = self.rng.below(cells.len());
        for (i, (cell, member)) in cells.iter().zip(members).enumerate() {
            self.add(cell, RDF_FIRST, member);
            let rest = cells.get(i + 1).unwrap_or(&cells[back]).clone();
            self.add(cell, RDF_REST, &rest);
        }
        cells[0].clone()
    }

    fn statement(&mut self) {
        // Weights: data first, then the schema, then the rarer constructs.
        const KINDS: &[(u8, usize)] = &[
            (0, 14), // i a C
            (1, 14), // i p j
            (2, 6),  // i d literal
            (3, 6),  // C subClassOf D
            (4, 2),  // C equivalentClass D
            (5, 3),  // p domain C
            (6, 3),  // p range C, or d range datatype
            (7, 6),  // property characteristic
            (8, 3),  // subPropertyOf
            (9, 2),  // equivalentProperty
            (10, 2), // inverseOf
            (11, 3), // propertyChainAxiom
            (12, 2), // intersectionOf
            (13, 2), // unionOf
            (14, 5), // a restriction, tied to a class or an individual
            (15, 4), // sameAs
            (16, 2), // disjointWith
            (17, 1), // propertyDisjointWith
            (18, 1), // complementOf
            (19, 1), // i a owl:Nothing
            (20, 2), // container membership
        ];
        let total: usize = KINDS.iter().map(|(_, w)| w).sum();
        let mut roll = self.rng.below(total);
        let kind = KINDS
            .iter()
            .find(|(_, w)| {
                let hit = roll < *w;
                roll = roll.saturating_sub(*w);
                hit
            })
            .map_or(0, |(k, _)| *k);
        match kind {
            0 => {
                let (i, c) = (self.individual(), self.superclass());
                self.add(&i, RDF_TYPE, &c);
            }
            1 => {
                let (i, p, j) = (self.individual(), self.object_property(), self.individual());
                self.add(&i, p_str(&p), &j);
            }
            2 => {
                let (i, d, l) = (self.individual(), self.data_property(), self.literal());
                self.add(&i, p_str(&d), &l);
            }
            3 => {
                let (c, d) = (self.class(), self.superclass());
                self.add(&c, RDFS_SUB_CLASS_OF, &d);
            }
            4 => {
                let (c, d) = (self.class(), self.class());
                self.add(&c, OWL_EQUIVALENT_CLASS, &d);
            }
            5 => {
                let (p, c) = (self.property(), self.superclass());
                self.add(&p, RDFS_DOMAIN, &c);
            }
            6 => {
                if self.rng.one_in(2) {
                    let (d, t) = (self.data_property(), iri(self.rng.pick(DATATYPES)));
                    self.add(&d, RDFS_RANGE, &t);
                } else {
                    let (p, c) = (self.object_property(), self.class());
                    self.add(&p, RDFS_RANGE, &c);
                }
            }
            7 => {
                let (p, characteristic) = match self.rng.below(7) {
                    0 => (self.object_property(), OWL_SYMMETRIC_PROPERTY),
                    1 => (self.object_property(), OWL_TRANSITIVE_PROPERTY),
                    2 => (self.object_property(), OWL_FUNCTIONAL_PROPERTY),
                    3 => (self.data_property(), OWL_FUNCTIONAL_PROPERTY),
                    4 => (self.object_property(), OWL_INVERSE_FUNCTIONAL_PROPERTY),
                    5 => (self.object_property(), OWL_ASYMMETRIC_PROPERTY),
                    _ => (self.object_property(), OWL_IRREFLEXIVE_PROPERTY),
                };
                self.add(&p, RDF_TYPE, &iri(characteristic));
            }
            8 | 9 => {
                let data = self.rng.one_in(4);
                let (p, q) = if data {
                    (self.data_property(), self.data_property())
                } else {
                    (self.object_property(), self.object_property())
                };
                let axiom = if kind == 8 {
                    RDFS_SUB_PROPERTY_OF
                } else {
                    OWL_EQUIVALENT_PROPERTY
                };
                self.add(&p, axiom, &q);
            }
            10 => {
                let (p, q) = (self.object_property(), self.object_property());
                self.add(&p, OWL_INVERSE_OF, &q);
            }
            11 => {
                let p = self.object_property();
                let members: Vec<Term> = (0..1 + self.rng.below(4)).map(|_| self.object_property()).collect();
                let head = if self.rng.one_in(8) {
                    self.cyclic_list(&members)
                } else {
                    self.list(&members)
                };
                self.add(&p, OWL_PROPERTY_CHAIN_AXIOM, &head);
            }
            12 | 13 => {
                let c = self.class();
                let members: Vec<Term> = (0..2 + self.rng.below(2))
                    .map(|_| {
                        if self.rng.one_in(4) {
                            self.restriction()
                        } else {
                            self.superclass()
                        }
                    })
                    .collect();
                let head = self.list(&members);
                let axiom = if kind == 12 { OWL_INTERSECTION_OF } else { OWL_UNION_OF };
                self.add(&c, axiom, &head);
            }
            14 => {
                let r = self.restriction();
                match self.rng.below(4) {
                    0 => {
                        let c = self.class();
                        self.add(&r, RDFS_SUB_CLASS_OF, &c);
                    }
                    1 => {
                        let c = self.class();
                        self.add(&c, RDFS_SUB_CLASS_OF, &r);
                    }
                    2 => {
                        let c = self.class();
                        self.add(&c, OWL_EQUIVALENT_CLASS, &r);
                    }
                    _ => {
                        let i = self.individual();
                        self.add(&i, RDF_TYPE, &r);
                    }
                }
            }
            15 => {
                // Mostly individuals, chained into cliques; now and then two
                // classes or two properties of the same kind.
                let (a, b) = match self.rng.below(10) {
                    0 => (self.class(), self.class()),
                    1 => (self.object_property(), self.object_property()),
                    _ => (self.individual(), self.individual()),
                };
                self.add(&a, OWL_SAME_AS, &b);
            }
            16 => {
                let (c, d) = (self.class(), self.superclass());
                self.add(&c, OWL_DISJOINT_WITH, &d);
            }
            17 => {
                let (p, q) = (self.property(), self.property());
                self.add(&p, OWL_PROPERTY_DISJOINT_WITH, &q);
            }
            18 => {
                let (c, d) = (self.class(), self.superclass());
                self.add(&c, OWL_COMPLEMENT_OF, &d);
            }
            19 => {
                let i = self.individual();
                self.add(&i, RDF_TYPE, &iri(OWL_NOTHING));
            }
            _ => {
                let container = self.fresh();
                let p = named(&format!("{RDF}{}", self.rng.pick(MEMBERSHIP)));
                let i = self.individual();
                self.add_with(&container, &p, &i);
            }
        }
    }
}

fn ex(local: &str) -> Term {
    iri(&format!("{EX}{local}"))
}

fn p_str(p: &Term) -> &str {
    match p {
        Term::NamedNode(node) => node.as_str(),
        _ => unreachable!("properties are IRIs"),
    }
}
