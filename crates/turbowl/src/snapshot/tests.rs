//! The snapshot format: the checksum's test vectors, exact round trips in
//! every state, and refusals of every damaged, truncated or crafted input.

use super::*;
use crate::{Error, Format, Reasoner, Summary};
use oxrdf::Triple;

/// XXH64's published test vectors: the empty input of the xxHash
/// specification, and those of python-xxhash's documentation.
#[test]
fn xxh64_gives_the_published_test_vectors() {
    assert_eq!(xxh64(b"", 0), 0xEF46_DB37_51D8_E999);
    assert_eq!(xxh64(b"a", 0), 0xD24E_C4F1_A98C_6E5B);
    assert_eq!(xxh64(b"abc", 0), 0x44BC_2CF5_AD77_0999);
    assert_eq!(
        xxh64(b"Nobody inspects the spammish repetition", 0),
        0xFBCE_A83C_8A37_8BF1
    );
    assert_eq!(xxh64(b"xxhash", 20_141_025), 0xB559_B98D_844E_0635);
}

/// Fed in pieces, XXH64 gives what it gives in one piece, wherever the
/// pieces split.
#[test]
fn xxh64_in_pieces_is_xxh64_whole() {
    let bytes: Vec<u8> = (0..200u32).map(|i| (i * 31 % 251) as u8).collect();
    for len in [0, 1, 3, 4, 7, 8, 31, 32, 33, 63, 64, 65, 100, 200] {
        let whole = xxh64(&bytes[..len], 7);
        for split in 0..=len {
            let mut h = Xxh64::new(7);
            h.update(&bytes[..split]);
            h.update(&bytes[split..len]);
            assert_eq!(h.digest(), whole, "len {len}, split {split}");
        }
        let mut h = Xxh64::new(7);
        for b in &bytes[..len] {
            h.update(std::slice::from_ref(b));
        }
        assert_eq!(h.digest(), whole, "len {len}, byte by byte");
    }
}

const PREFIXES: &str = "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ex: <http://example.org/> .\n";

/// Classes, a property with a domain and a range, a disjointness broken,
/// `owl:sameAs` merges (one of a property), an ill-typed literal, a
/// language tag, blank nodes and a list.
const FIRST: &str = "ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C .
    ex:A owl:disjointWith ex:D .
    ex:p rdfs:domain ex:A ; rdfs:range ex:B .
    ex:x ex:p ex:y . ex:y ex:p ex:z . ex:x a ex:D .
    ex:x owl:sameAs ex:x2 . ex:x2 owl:sameAs ex:x3 .
    ex:q owl:sameAs ex:p .
    ex:z ex:age \"twelve\"^^xsd:integer ; rdfs:label \"zed\"@en-GB .
    _:b ex:p ex:x . ex:l ex:items ( ex:x _:c \"one\" ) .";

/// More of the same, and a merge with a vocabulary term (a restart).
const SECOND: &str = "ex:w ex:p ex:x . ex:z owl:sameAs ex:w . ex:C owl:sameAs ex:E .
    ex:sub owl:sameAs rdfs:subClassOf . ex:E ex:sub ex:F . _:b ex:q ex:w .";

fn load(r: &mut Reasoner, turtle: &str) {
    r.load_str(&format!("{PREFIXES}{turtle}"), Format::Turtle, None)
        .unwrap_or_else(|e| panic!("{e}"));
}

fn parse(turtle: &str) -> Vec<Triple> {
    oxttl::TurtleParser::new()
        .for_slice(format!("{PREFIXES}{turtle}").as_bytes())
        .collect::<Result<_, _>>()
        .unwrap()
}

fn bytes(r: &Reasoner) -> Vec<u8> {
    let mut out = Vec::new();
    r.write_snapshot(&mut out).unwrap();
    out
}

/// The reasoner `r`'s snapshot holds; saving it again gives the same bytes.
fn reload(r: &Reasoner) -> Reasoner {
    let saved = bytes(r);
    let loaded = Reasoner::from_snapshot_bytes(&saved).unwrap_or_else(|e| panic!("{e}"));
    assert!(bytes(&loaded) == saved, "saving a loaded snapshot gives the same bytes");
    loaded
}

type Shown = (Vec<Triple>, Vec<Triple>, Vec<crate::Diagnostic>, Switches, String);

/// What the interface shows of a reasoner, in the order it shows it.
fn shown(r: &Reasoner) -> Shown {
    let mut written = Vec::new();
    r.write(&mut written, Format::NTriples, false).unwrap();
    (
        r.triples().collect(),
        r.inferred().collect(),
        r.diagnostics().to_vec(),
        r.switches(),
        String::from_utf8(written).unwrap(),
    )
}

/// A reasoner with everything: two documents, inserted triples, merges, a
/// restart, findings, all switches on.
fn rich() -> Reasoner {
    let mut r = Reasoner::with_switches(Switches::complete());
    load(&mut r, FIRST);
    r.reason();
    load(&mut r, SECOND);
    r.insert(parse("ex:i ex:p ex:j . _:n ex:p ex:i .").iter().map(Triple::as_ref));
    r.reason();
    r
}

/// Two reasoners stay alike under the same changes and runs.
fn same_from_here(mut a: Reasoner, mut b: Reasoner) {
    let steps: [&dyn Fn(&mut Reasoner); 5] = [
        &|r| load(r, "ex:k ex:p ex:x . ex:k owl:sameAs ex:k2 . ex:k2 a ex:D ."),
        &|r| r.insert(parse("ex:m ex:age \"x\"^^xsd:integer .").iter().map(Triple::as_ref)),
        &|r| r.remove(parse("ex:x ex:p ex:y .").iter().map(Triple::as_ref)),
        &|r| r.set_switches(Switches::default()),
        // A new document (with labelled blank nodes only: a parse labels
        // anonymous ones at random).
        &|r| load(r, "_:b ex:p ex:x . ex:t ex:p _:b . ex:t owl:sameAs ex:t2 ."),
    ];
    assert_eq!(shown(&a), shown(&b));
    for (i, step) in steps.iter().enumerate() {
        step(&mut a);
        step(&mut b);
        let (sa, sb) = (a.reason(), b.reason());
        assert_eq!(sa, sb, "step {i}");
        assert_eq!(shown(&a), shown(&b), "step {i}");
        assert!(bytes(&a) == bytes(&b), "step {i}: the same state");
    }
}

#[test]
fn a_round_trip_gives_the_same_reasoner() {
    let r = rich();
    let loaded = reload(&r);
    let shown_r = shown(&r);
    assert!(shown_r.2.len() >= 2, "the input has findings: {:?}", shown_r.2);
    assert!(r.store().canon().merges() >= 3, "the input merges classes");
    assert_eq!(shown(&loaded), shown_r);
    same_from_here(r, loaded);
}

#[test]
fn a_round_trip_keeps_a_run_that_is_due() {
    // Additions queued: an incremental run due.
    let mut r = rich();
    load(&mut r, "ex:v ex:p ex:x . ex:v owl:sameAs ex:v2 .");
    same_from_here(reload(&r), r);

    // A removal: a full run due, and the reads show the last run's closure.
    let mut r = rich();
    load(&mut r, "ex:v ex:p ex:x .");
    r.remove(parse("ex:y ex:p ex:z .").iter().map(Triple::as_ref));
    same_from_here(reload(&r), r);

    // New switches: a full run due.
    let mut r = rich();
    r.set_switches(Switches::default());
    same_from_here(reload(&r), r);

    // Never run; and empty.
    let mut r = Reasoner::new();
    load(&mut r, FIRST);
    same_from_here(reload(&r), r);
    same_from_here(reload(&Reasoner::new()), Reasoner::new());
}

#[test]
fn a_loaded_reasoner_runs_nothing_again() {
    let mut r = rich();
    let summary = r.reason();
    let mut loaded = reload(&r);
    assert_eq!(loaded.reason(), Summary { rounds: 0, ..summary });
    assert_eq!(loaded.store().rounds(), r.store().rounds());
}

/// The documents loaded are kept: loading one again changes nothing (its
/// blank nodes would be new nodes otherwise).
#[test]
fn a_loaded_reasoner_knows_its_documents() {
    let r = rich();
    let mut loaded = reload(&r);
    let before = loaded.reason();
    load(&mut loaded, FIRST);
    assert_eq!(loaded.reason(), Summary { rounds: 0, ..before });
}

/// A snapshot whose probe is not this build's has keys that name no
/// document here: they are dropped, as a removal drops them.
#[test]
fn keys_of_another_hashing_are_dropped() {
    let mut r = Reasoner::new();
    load(&mut r, FIRST);
    let base = r.reason().base;
    let mut saved = bytes(&r);
    // The probe is the first field of its section.
    let docs = find_section(&saved, Section::Documents);
    saved[docs] ^= 1;
    fix_checksum(&mut saved);
    let mut loaded = Reasoner::from_snapshot_bytes(&saved).unwrap();
    load(&mut loaded, FIRST);
    assert!(
        loaded.reason().base > base,
        "the document is loaded again, its blank nodes new"
    );
}

/// The offset of a section's payload.
fn find_section(bytes: &[u8], section: Section) -> usize {
    let mut at = FIXED + 2 + PRODUCER.len();
    for s in SECTIONS {
        let len = u64::from_le_bytes(bytes[at + 4..at + 12].try_into().unwrap()) as usize;
        assert_eq!(bytes[at..at + 4], s.tag());
        if s == section {
            return at + 12;
        }
        at += 12 + len;
    }
    unreachable!("every section is found")
}

fn fix_checksum(bytes: &mut [u8]) {
    let end = bytes.len() - TRAILER;
    let sum = xxh64(&bytes[..end], 0);
    bytes[end..].copy_from_slice(&sum.to_le_bytes());
}

fn refusal(bytes: &[u8]) -> String {
    match Reasoner::from_snapshot_bytes(bytes) {
        Ok(_) => panic!("loaded"),
        Err(Error::Snapshot { path: None, message }) => message,
        Err(e) => panic!("another error: {e}"),
    }
}

/// A small snapshot, with a document, a merge, a finding, an object index
/// and an incremental run due.
fn small() -> Vec<u8> {
    let mut r = Reasoner::new();
    load(
        &mut r,
        "ex:A owl:disjointWith ex:B . ex:x a ex:A , ex:B . ex:x owl:sameAs ex:y .
         ex:p rdfs:range ex:A . ex:y ex:p _:b .",
    );
    let s = r.reason();
    assert!(s.diagnostics > 0 && r.store().canon().merges() > 0);
    assert!(r.store().tables().any(|(_, t)| t.by_object().is_some()));
    load(&mut r, "ex:z ex:p ex:x .");
    bytes(&r)
}

#[test]
fn every_changed_byte_is_refused() {
    let mut saved = small();
    assert!(saved.len() < 16_384, "a small snapshot: {} bytes", saved.len());
    Reasoner::from_snapshot_bytes(&saved).unwrap();
    for at in 0..saved.len() {
        let was = saved[at];
        for value in 0..=255u8 {
            if value == was {
                continue;
            }
            saved[at] = value;
            assert!(
                Reasoner::from_snapshot_bytes(&saved).is_err(),
                "byte {at} changed from {was} to {value} is not refused"
            );
        }
        saved[at] = was;
    }
}

#[test]
fn every_truncation_is_refused() {
    let saved = small();
    for len in 0..saved.len() {
        let message = refusal(&saved[..len]);
        if len >= FIXED {
            assert!(message.contains("truncated"), "{len}: {message}");
        }
    }
    let mut longer = saved.clone();
    longer.push(0);
    assert!(refusal(&longer).contains("past its end"));
}

#[test]
fn the_refusals_say_why() {
    let saved = small();
    assert_eq!(
        refusal(b"@prefix ex: <http://example.org/> ."),
        "not a TurbOWL snapshot"
    );

    // A snapshot of version 1 (the base as triples), and of a later one.
    for version in [1u32, 3] {
        let mut other = saved.clone();
        other[8..12].copy_from_slice(&version.to_le_bytes());
        assert_eq!(
            refusal(&other),
            format!("the snapshot has format version {version}; this TurbOWL reads version 2")
        );
    }

    let mut damaged = saved.clone();
    let middle = damaged.len() / 2;
    damaged[middle] ^= 0x10;
    assert_eq!(
        refusal(&damaged),
        "the snapshot is damaged: its checksum does not match its contents"
    );

    // Another TurbOWL version: the producer's last character changed.
    let mut older = saved.clone();
    older[FIXED + 2 + PRODUCER.len() - 1] ^= 1;
    fix_checksum(&mut older);
    let message = refusal(&older);
    assert!(
        message.starts_with("the snapshot was saved by turbowl ") && message.contains(PRODUCER),
        "{message}"
    );

    let mut short = saved.clone();
    short.truncate(100);
    assert_eq!(
        refusal(&short),
        format!("the snapshot is truncated: 100 of its {} bytes", saved.len())
    );
}

/// Bytes that pass the checksum but were not written by a reasoner: every
/// single-byte change, with the checksum made right again, is refused or
/// loads a reasoner that works. None panics.
#[test]
fn crafted_contents_never_panic() {
    for saved in [small(), bytes(&rich())] {
        crafted_never_panic(&saved);
    }
}

fn crafted_never_panic(saved: &[u8]) {
    let mut crafted = saved.to_vec();
    let (mut refused, mut loaded) = (0, 0);
    for at in FIXED..saved.len() - TRAILER {
        for change in [0x01u8, 0x80, 0xFF] {
            crafted[at] = saved[at] ^ change;
            fix_checksum(&mut crafted);
            let outcome = std::panic::catch_unwind(|| match Reasoner::from_snapshot_bytes(&crafted) {
                Err(Error::Snapshot { .. }) => false,
                Err(e) => panic!("another error: {e}"),
                Ok(mut r) => {
                    // Usable: the reads, a run, a change, a run, a save.
                    let _ = shown(&r);
                    r.reason();
                    r.insert(parse("ex:n ex:p ex:x .").iter().map(Triple::as_ref));
                    r.reason();
                    let _ = shown(&r);
                    let _ = bytes(&r);
                    true
                }
            });
            match outcome {
                Ok(true) => loaded += 1,
                Ok(false) => refused += 1,
                Err(_) => panic!("byte {at} changed by {change:#x} panics"),
            }
        }
        crafted[at] = saved[at];
    }
    println!("{} bytes: {refused} refused, {loaded} loaded", saved.len());
    assert!(refused > loaded, "{refused} refused, {loaded} loaded");
}

/// The checks the loader makes, each on a snapshot crafted to break it,
/// name the section and what is wrong.
#[test]
fn malformed_parts_are_named() {
    let saved = small();
    let malformed = |at: usize, bytes: &[u8]| {
        let mut crafted = saved.clone();
        crafted[at..at + bytes.len()].copy_from_slice(bytes);
        fix_checksum(&mut crafted);
        refusal(&crafted)
    };
    let conf = find_section(&saved, Section::Conf);
    let m = malformed(conf, &[200]);
    assert!(m.starts_with("the snapshot is malformed: CONF"), "{m}");
    let terms = find_section(&saved, Section::Terms);
    // The first term's kind: an IRI made a literal is not the vocabulary.
    let first = terms + 4 + 8 + 8;
    let m = malformed(first, &[KIND_TYPED]);
    assert!(m.contains("TERM") && m.contains("another vocabulary"), "{m}");
    let m = malformed(first, &[9]);
    assert!(m.contains("unknown kind"), "{m}");
    // The count of terms, past what the bytes hold.
    let m = malformed(terms + 4, &u64::MAX.to_le_bytes());
    assert!(m.contains("a count of"), "{m}");
    let base = find_section(&saved, Section::Base);
    // The first base triple's subject: past the dictionary.
    let m = malformed(base + 8, &u32::MAX.to_le_bytes());
    assert!(m.contains("BASE") && m.contains("an id"), "{m}");
    let store = find_section(&saved, Section::Store);
    // The first table's predicate: past the dictionary.
    let m = malformed(store + 16, &u32::MAX.to_le_bytes());
    assert!(m.contains("STOR"), "{m}");

    // The queue emptied, and the state says a run is due.
    let queue = find_section(&saved, Section::Queue);
    let mut crafted = saved.clone();
    let count = u64::from_le_bytes(crafted[queue..queue + 8].try_into().unwrap()) as usize;
    assert!(count > 0, "the small snapshot has an incremental run due");
    crafted.drain(queue + 8..queue + 8 + 12 * count);
    crafted[queue..queue + 8].copy_from_slice(&0u64.to_le_bytes());
    crafted[queue - 8..queue].copy_from_slice(&8u64.to_le_bytes());
    let total = crafted.len() as u64;
    crafted[12..20].copy_from_slice(&total.to_le_bytes());
    fix_checksum(&mut crafted);
    let m = refusal(&crafted);
    assert!(m.contains("incremental run is due"), "{m}");
}

/// Saving to a file writes a temporary file and renames it; a failed save
/// leaves the old file and no temporary one.
#[test]
fn a_save_is_atomic() {
    let dir = std::env::temp_dir().join(format!("turbowl-snapshot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("state.snapshot");
    let r = rich();
    r.save_snapshot(&path).unwrap();
    assert!(std::fs::read(&path).unwrap() == bytes(&r));
    let loaded = Reasoner::from_snapshot(&path).unwrap();
    assert_eq!(shown(&loaded), shown(&r));
    // Saving again replaces it.
    loaded.save_snapshot(&path).unwrap();
    assert!(std::fs::read(&path).unwrap() == bytes(&r));

    // A write that fails half-way changes nothing.
    let failed = save(&path, |file| {
        file.write_all(b"half")?;
        Err(io::Error::other("the disk is full"))
    });
    assert!(failed.is_err());
    assert!(std::fs::read(&path).unwrap() == bytes(&r));
    let names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["state.snapshot"]);

    // A file that is not a snapshot names its path.
    std::fs::write(&path, "not one").unwrap();
    let e = Reasoner::from_snapshot(&path).unwrap_err();
    assert!(matches!(e, Error::Snapshot { path: Some(_), .. }));
    assert_eq!(e.to_string(), format!("{}: not a TurbOWL snapshot", path.display()));
    let missing = Reasoner::from_snapshot(&dir.join("missing")).unwrap_err();
    assert!(matches!(missing, Error::Io { path: Some(_), .. }), "{missing}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Every code a finding can load with is a diagnostic's, listed once.
#[test]
fn the_codes_are_the_diagnostics_codes() {
    let mut sorted = CODES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted, CODES, "sorted, each once");
    let table = include_str!("../diagnostics.rs");
    for code in CODES {
        assert!(
            table.contains(&format!("`{code}`")),
            "{code} is in the diagnostics' table"
        );
    }
}

/// A snapshot large enough that its base and its batches decode in chunks
/// on the workers: the same reasoner back, and an order broken inside a
/// chunk or across two, or an id out of range, refused.
#[test]
fn a_large_snapshot_decodes_in_chunks() {
    let n = 2 * crate::engine::PARALLEL_FROM;
    let triples: Vec<Triple> = (0..n)
        .map(|i| {
            Triple::new(
                oxrdf::NamedNode::new_unchecked(format!("http://example.org/i{i}")),
                oxrdf::NamedNode::new_unchecked("http://example.org/p"),
                oxrdf::NamedNode::new_unchecked(format!("http://example.org/i{}", i + 1)),
            )
        })
        .collect();
    let mut r = Reasoner::new();
    r.insert(triples.iter().map(Triple::as_ref));
    r.reason();
    let loaded = reload(&r);
    assert_eq!(shown(&loaded), shown(&r));

    let saved = bytes(&r);
    // One predicate: its id and its count of pairs, then the pairs.
    let at = find_section(&saved, Section::Base);
    assert_eq!(saved[at..at + 8], 1u64.to_le_bytes());
    assert_eq!(saved[at + 12..at + 20], (n as u64).to_le_bytes());
    let base = at + 20;
    let swapped = |a: usize| {
        let mut crafted = saved.clone();
        let (x, y) = (base + 8 * a, base + 8 * (a + 1));
        let first: Vec<u8> = crafted[x..y].to_vec();
        crafted.copy_within(y..y + 8, x);
        crafted[y..y + 8].copy_from_slice(&first);
        fix_checksum(&mut crafted);
        refusal(&crafted)
    };
    // Chunks are 4,096 pairs or more: 4,095 and 4,096 are in two.
    for a in [10, 4_095, n - 2] {
        let m = swapped(a);
        assert!(m.contains(&format!("pair {} is not after", a + 1)), "{a}: {m}");
        assert!(m.contains("BASE") && m.contains("(predicate "), "{a}: {m}");
    }
    let mut crafted = saved.clone();
    let far = base + 8 * (n - 100);
    crafted[far..far + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    fix_checksum(&mut crafted);
    assert!(refusal(&crafted).contains("an id 4294967295"));
}

/// The base round-trips by predicate: merged, with triples still in its
/// hash set, and after a removal empties a predicate; the loaded base is
/// the saved one, and the changes then do on both what they do on one.
#[test]
fn a_round_trip_keeps_the_base() {
    let mut r = rich();
    // A few triples in the hash set, of a known predicate and a new one.
    r.insert(parse("ex:g ex:p ex:h . ex:g ex:new ex:h .").iter().map(Triple::as_ref));
    let (saved, held) = r.base_by_predicate();
    assert!(held >= 2 && saved.len() > 3, "{held} triples in the hash set");
    let loaded = reload(&r);
    assert_eq!(loaded.base_by_predicate(), (saved, 0), "written merged");
    same_from_here(r, loaded);

    let mut r = rich();
    r.remove(parse("ex:z rdfs:label \"zed\"@en-GB .").iter().map(Triple::as_ref));
    let label = r
        .terms()
        .get_iri("http://www.w3.org/2000/01/rdf-schema#label")
        .unwrap()
        .0;
    let (saved, _) = r.base_by_predicate();
    assert!(saved.iter().all(|(p, _)| p != label), "the predicate is gone");
    let loaded = reload(&r);
    assert_eq!(loaded.base_by_predicate(), (saved, 0));
    same_from_here(r, loaded);
}

/// The base's checks, each on a snapshot crafted to break it, name the
/// section and what is wrong.
#[test]
fn a_malformed_base_is_named() {
    let saved = small();
    let at = find_section(&saved, Section::Base);
    let predicates = u64::from_le_bytes(saved[at..at + 8].try_into().unwrap());
    assert!(predicates >= 2, "the small snapshot's base has predicates");
    let crafted = |at: usize, bytes: &[u8]| {
        let mut crafted = saved.clone();
        crafted[at..at + bytes.len()].copy_from_slice(bytes);
        fix_checksum(&mut crafted);
        refusal(&crafted)
    };
    let first = at + 8;
    let count = u64::from_le_bytes(saved[first + 4..first + 12].try_into().unwrap()) as usize;
    let second = first + 12 + 8 * count;
    // The second predicate made the first's.
    let m = crafted(second, &saved[first..first + 4]);
    assert!(
        m.contains("BASE") && m.contains("is not after the one before it"),
        "{m}"
    );
    // The first predicate's count made 0.
    let m = crafted(first + 4, &0u64.to_le_bytes());
    assert!(m.contains("BASE") && m.contains("has no pairs"), "{m}");
    // An object past the dictionary.
    let m = crafted(first + 12 + 4, &u32::MAX.to_le_bytes());
    assert!(m.contains("BASE") && m.contains("an id 4294967295"), "{m}");
}
