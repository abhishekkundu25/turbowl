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
//! rule of the set reads), and literals of the OWL 2 RL datatypes and of others,
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
//! ([`Shape::version_1`]) and draw from the seed's stream exactly as version
//! 1 did, so the first part of every input is version 1's. A shape added
//! since belongs to its file's own stream (the seed mixed with the file's
//! name): after version 1's draws, each file with new shapes draws its
//! statements from that stream. A rule ticket that adds shapes to its file
//! changes no other file's draws and leaves version 1's part alone.

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
    /// since, which its file draws from its own stream ([`generate`]).
    version_1: Option<u8>,
    weight: usize,
    draw: fn(&mut Generator),
}

/// The files of a group (`None`: the core), each a name and its shapes. A
/// match, so a new switch draws nothing until it has its file.
fn files(group: Option<Switch>) -> &'static [(&'static str, &'static [Shape])] {
    match group {
        None => &[("core", self::core::SHAPES)],
        Some(Switch::Equality) => &[("equality", equality::SHAPES)],
        Some(Switch::PropertyChains) => &[("property_chains", property_chains::SHAPES)],
        Some(Switch::Checks) => &[("checks", checks::SHAPES), ("datatypes", datatypes::SHAPES)],
        Some(Switch::Identity) => &[("identity", identity::SHAPES)],
        Some(Switch::Schema) => &[("schema", schema::SHAPES)],
        Some(Switch::Trivial) => &[("trivial", trivial::SHAPES)],
        Some(Switch::LiteralValues) => &[("literal_values", literal_values::SHAPES)],
    }
}

/// Every file, in group order: the core, then [`Switch::ALL`].
fn all_files() -> impl Iterator<Item = &'static (&'static str, &'static [Shape])> {
    std::iter::once(None)
        .chain(Switch::ALL.into_iter().map(Some))
        .flat_map(files)
}

/// Version 1's shapes, in their places in version 1's table.
fn table() -> Vec<&'static Shape> {
    let mut shapes: Vec<&Shape> = all_files()
        .flat_map(|(_, shapes)| shapes.iter())
        .filter(|s| s.version_1.is_some())
        .collect();
    shapes.sort_by_key(|s| s.version_1);
    shapes
}

/// `size` statements drawn from the seed, then each file's new shapes from
/// its own stream: the same seed and size always give the same triples, in
/// the same order, without duplicates.
pub fn generate(seed: u64, size: usize) -> Vec<Triple> {
    generate_parts(seed, size).1
}

/// The input of `generate`, and how many of its triples version 1's draws
/// gave (the first ones).
fn generate_parts(seed: u64, size: usize) -> (usize, Vec<Triple>) {
    let mut g = Generator::new(Rng::new(seed), size);
    draw(&mut g, &table(), size);
    let version_1 = g.out.len();
    for (name, shapes) in all_files() {
        let new: Vec<&Shape> = shapes.iter().filter(|s| s.version_1.is_none()).collect();
        draw_own(&mut g, seed, name, &new, extra(size));
    }
    (version_1, g.out)
}

/// How many statements a file with new shapes draws from its own stream.
fn extra(size: usize) -> usize {
    size.div_ceil(12)
}

/// `count` statements from `shapes`, by weight, with the generator's stream.
fn draw(g: &mut Generator, shapes: &[&Shape], count: usize) {
    let total: usize = shapes.iter().map(|s| s.weight).sum();
    if total == 0 {
        return;
    }
    for _ in 0..count {
        let mut roll = g.rng.below(total);
        let shape = shapes
            .iter()
            .find(|s| {
                let hit = roll < s.weight;
                roll = roll.saturating_sub(s.weight);
                hit
            })
            .expect("the roll is below the total weight");
        (shape.draw)(g);
    }
}

/// `count` statements from a file's new `shapes`, with the file's own
/// stream (the seed mixed with the file's name, FNV-1a), leaving the
/// generator's stream where it was. A file with no new shape draws nothing.
fn draw_own(g: &mut Generator, seed: u64, name: &str, shapes: &[&Shape], count: usize) {
    if shapes.is_empty() {
        return;
    }
    let salt = name.bytes().fold(0xCBF2_9CE4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01B3)
    });
    let main = std::mem::replace(&mut g.rng, Rng::new(seed ^ salt));
    draw(g, shapes, count);
    g.rng = main;
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
    /// to 48 as it draws them) begin with those of the generator of version
    /// 1: the N-Triples lines of version 1's part hash (FNV-1a, 64 bits) to
    /// the value that generator gave. New shapes draw from their files' own
    /// streams after it, so no ticket that adds a shape changes this value.
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
                let (version_1, input) = generate_parts(rng.next_u64(), size);
                for t in &input[..version_1] {
                    feed(&format!("{t} .\n"));
                }
                feed("#\n");
            }
        }
        assert_eq!(hash, 0x772F_272B_CFE6_C083, "the generator's inputs changed");
    }

    /// A file's new shapes draw from the file's own stream: the generator's
    /// stream is where it was before, and the same file and seed draw the
    /// same statements.
    #[test]
    fn new_shapes_draw_from_their_files_own_stream() {
        fn one(g: &mut Generator) {
            let (s, o) = (g.individual(), g.individual());
            g.add(&s, "http://example.org/q", &o);
        }
        let shape = Shape {
            version_1: None,
            weight: 1,
            draw: one,
        };
        let run = |name: &str| {
            let mut g = Generator::new(Rng::new(7), 24);
            let before = g.rng.clone().next_u64();
            draw_own(&mut g, 7, name, &[&shape], 5);
            (g.rng.clone().next_u64() == before, g.out)
        };
        let (kept, first) = run("identity");
        assert!(kept, "the generator's stream moved");
        assert!(!first.is_empty());
        assert_eq!(run("identity").1, first);
        assert_ne!(run("schema").1, first, "two files share a stream");
        // A file with no new shape draws nothing.
        let mut g = Generator::new(Rng::new(7), 24);
        draw_own(&mut g, 7, "identity", &[], 5);
        assert!(g.out.is_empty());
    }
}
