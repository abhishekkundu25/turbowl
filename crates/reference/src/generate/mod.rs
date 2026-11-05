//! Small random ontologies and data, the same for the same seed.
//!
//! A run draws `size` statements from a weighted table of shapes, over pools
//! of classes, object properties, data properties and individuals whose
//! sizes grow with `size`. The shapes cover every rule family: class and
//! property hierarchies, equivalences, domains and ranges, property
//! characteristics, property chains (well-formed, cyclic, with shared tails,
//! with IRI cells), intersections, unions, restrictions, `owl:sameAs`
//! (between individuals, classes or properties, and on vocabulary terms: a
//! term made equal to a rule constant such as `rdf:type`, two rule
//! constants made equal, a list cell made equal to `rdf:nil` or to another
//! cell), disjointness and complements, container membership triples (which no
//! rule of the set reads), and literals of checked and unchecked datatypes,
//! well-formed or not. Some individuals, every list
//! cell and every restriction are blank nodes.
//!
//! Literals only ever appear as objects of data properties (and of
//! `owl:hasValue` on one), so object-property rules (inverse, symmetric,
//! transitive) never meet a literal. Literals still reach subject positions
//! inside the evaluator: a data property's range types them (prp-rng), and
//! a functional data property makes two of them the same (prp-fp).
//!
//! # Groups and files
//!
//! The shapes are split as the rules are (`src/rules/`): one file per
//! switch group (`core.rs`, then [`Switch::name`] with `_` for `-`), and
//! `datatypes.rs` for the literals and the datatypes of the `checks` group's
//! dt-not-type. Each file lists its [`Shape`]s; [`files`] says which files a
//! group has. The generator draws from every group's shapes, whatever the
//! configuration a caller then evaluates.
//!
//! Version 1's shapes keep their place in version 1's table
//! ([`Shape::version_1`]), so every seed gives the same input as in version
//! 1 until a group adds a shape. A shape added since is drawn after them, in
//! group order. A group with no shape draws nothing from the random
//! generator.

mod checks;
mod core;
mod datatypes;
mod equality;
mod identity;
mod literal_values;
mod property_chains;
mod schema;
mod trivial;

use crate::vocab::*;
use oxrdf::{BlankNode, NamedNode, NamedOrBlankNode, Term, Triple};
use std::collections::HashSet;
use turbowl::Switch;

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

/// One shape of statement: how often it is drawn, against the others, and
/// what it adds.
struct Shape {
    /// Its place in version 1's table (0 to 20), or `None` for a shape added
    /// since. Version 1's shapes are drawn in that order, which keeps every
    /// seed's input; the others come after them, in group order.
    version_1: Option<u8>,
    weight: usize,
    draw: fn(&mut Generator),
}

/// The files of a group (`None`: the core), each a list of shapes. A match,
/// so a new switch draws nothing until it has its file.
fn files(group: Option<Switch>) -> &'static [&'static [Shape]] {
    match group {
        None => &[self::core::SHAPES],
        Some(Switch::Equality) => &[equality::SHAPES],
        Some(Switch::PropertyChains) => &[property_chains::SHAPES],
        Some(Switch::Checks) => &[checks::SHAPES, datatypes::SHAPES],
        Some(Switch::Identity) => &[identity::SHAPES],
        Some(Switch::Schema) => &[schema::SHAPES],
        Some(Switch::Trivial) => &[trivial::SHAPES],
        Some(Switch::LiteralValues) => &[literal_values::SHAPES],
    }
}

/// Every shape, in the order a draw reads them: version 1's in their places,
/// then the others in group order (the core, then [`Switch::ALL`]).
fn table() -> Vec<&'static Shape> {
    let groups = std::iter::once(None).chain(Switch::ALL.into_iter().map(Some));
    let mut shapes: Vec<&Shape> = groups.flat_map(files).flat_map(|file| file.iter()).collect();
    shapes.sort_by_key(|s| s.version_1.map_or(u16::MAX, u16::from));
    shapes
}

/// `size` statements drawn from the seed: the same seed and size always give
/// the same triples, in the same order, without duplicates.
pub fn generate(seed: u64, size: usize) -> Vec<Triple> {
    let table = table();
    let total: usize = table.iter().map(|s| s.weight).sum();
    let mut g = Generator::new(Rng::new(seed), size);
    for _ in 0..size {
        let mut roll = g.rng.below(total);
        let shape = table
            .iter()
            .find(|s| {
                let hit = roll < s.weight;
                roll = roll.saturating_sub(s.weight);
                hit
            })
            .expect("the roll is below the total weight");
        (shape.draw)(&mut g);
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
                let value = if data {
                    datatypes::literal(self)
                } else {
                    self.individual()
                };
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Version 1's shapes hold the places 0 to 20 of its table, once each.
    #[test]
    fn version_1_shapes_keep_their_places() {
        let places: Vec<u8> = table().iter().filter_map(|s| s.version_1).collect();
        assert_eq!(places, (0..=20).collect::<Vec<u8>>());
    }

    /// The inputs of the chaos test's first 300 seeds (both seeds, sizes 1
    /// to 48 as it draws them) are those of the generator of version 1:
    /// their N-Triples lines hash (FNV-1a, 64 bits) to the value that
    /// generator gave. A group with no shape draws nothing, so splitting
    /// the shapes into files changed no input. A ticket that adds a shape
    /// changes the inputs on purpose and replaces this value.
    #[test]
    fn the_first_300_seeds_give_version_1_inputs() {
        let mut hash = 0xCBF2_9CE4_8422_2325_u64;
        let mut feed = |text: &str| {
            for b in text.bytes() {
                hash ^= u64::from(b);
                hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
            }
        };
        for seed in [0x5EED_0005, 0x5EED_0105] {
            for i in 0..300 {
                let mut rng = Rng::at(seed, i);
                let size = 1 + rng.below(48);
                for t in generate(rng.next_u64(), size) {
                    feed(&format!("{t} .\n"));
                }
                feed("#\n");
            }
        }
        assert_eq!(hash, 0x772F_272B_CFE6_C083, "the generator's inputs changed");
    }
}
