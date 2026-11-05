//! The public datasets (`docs/research/competitors-2026-11.md` §8), each an
//! opt-in scenario: the files a run loads, where `scripts/fetch-data.sh` puts
//! them, and the fact counts the papers publish for it (§1.2), before and
//! after reasoning. A paper's count is for its own program on that data; with
//! equality, AX counts every `owl:sameAs` consequence (as the bench does) and
//! REW one representative per class.
//!
//! Each name ends in the program: `-l` (the OWL 2 RL part of the ontology),
//! `-le` (L with hand-written chain rules) or `-u` (the upper bound), as
//! Oxford ships them in OWL (AAAI 2014).

use crate::scenario::{Fetched, Scenario};

/// An opt-in, full-run scenario on fetched files.
const fn dataset(
    name: &'static str,
    about: &'static str,
    files: &'static [&'static str],
    fetch: &'static str,
    published: &'static str,
) -> Scenario {
    Scenario::fetched(
        name,
        about,
        Fetched {
            files,
            fetch,
            published,
        },
    )
}

const NONE: &str = "none published";

pub(crate) const DATASETS: &[Scenario] = &[
    // Fetched by default.
    dataset(
        "claros-l",
        "Claros (Oxford, 18.8 M triples) with its L program",
        &["oxford/claros/Claros_L.owl", "oxford/claros/Claros.ttl"],
        "",
        "18.8 M -> 95.5 M (AAAI 2014); 18.8 M -> 73.8 M (AIJ 2019); L-eq 19 M -> 102 M AX, 79.7 M REW (AAAI 2015)",
    ),
    dataset(
        "claros-le",
        "Claros with its LE program (L plus chain rules: large cliques)",
        &["oxford/claros/Claros_LE.owl", "oxford/claros/Claros.ttl"],
        "",
        "18.8 M -> 555.1 M (AAAI 2014); -> 539.2 M (ISWC 2015); -> 533.3 M (AIJ 2019)",
    ),
    dataset(
        "claros-u",
        "Claros with its U program (the upper bound)",
        &["oxford/claros/Claros_U.owl", "oxford/claros/Claros.ttl"],
        "",
        NONE,
    ),
    dataset(
        "uobm-010-l",
        "UOBM, 10 universities (Oxford, 2.2 M triples), with its L program",
        &["oxford/uobm/UOBM_L.owl", "oxford/uobm/UOBM-010.ttl"],
        "",
        "none published (UOBM L only at 1,000 universities)",
    ),
    dataset(
        "uobm-010-u",
        "UOBM, 10 universities, with its U program",
        &["oxford/uobm/UOBM_U.owl", "oxford/uobm/UOBM-010.ttl"],
        "",
        "2.2 M -> 35.6 M (AAAI 2014); U-eq 2.2 M -> 36 M AX, 9.7 M REW (AAAI 2015)",
    ),
    dataset(
        "opencyc",
        "OpenCyc (Oxford, AAAI 2015: 2.4 M triples, its OWL axioms included)",
        &["oxford/opencyc/OpenCyc.ttl"],
        "",
        "2.4 M -> 1,176 M AX, 142 M REW (AAAI 2015, OpenCyc-eq: 261,067 rules)",
    ),
    dataset(
        "owl2bench-rl-1",
        "OWL2Bench's OWL 2 RL ontology with 1 university of data",
        &["owl2bench/OWL2RL-1.owl"],
        "",
        "no closure published; its RL ABox: 50,131 axioms (OWL2Bench, ISWC 2020)",
    ),
    dataset(
        "owl2bench-rl-2",
        "OWL2Bench's OWL 2 RL ontology with 2 universities of data",
        &["owl2bench/OWL2RL-2.owl"],
        "",
        NONE,
    ),
    dataset(
        "owl2bench-rl-5",
        "OWL2Bench's OWL 2 RL ontology with 5 universities of data",
        &["owl2bench/OWL2RL-5.owl"],
        "",
        NONE,
    ),
    // Generated on demand by UBA, the official LUBM generator.
    dataset(
        "lubm-uba-10-l",
        "LUBM from UBA 1.7, 10 universities (seed 0, index 0), with Oxford's L program",
        &["oxford/lubm/LUBM_L.owl", "uba/lubm-10.owl"],
        "--lubm 10",
        "none published with this data and program",
    ),
    dataset(
        "lubm-uba-100-l",
        "LUBM from UBA 1.7, 100 universities, with the L program",
        &["oxford/lubm/LUBM_L.owl", "uba/lubm-100.owl"],
        "--lubm 100",
        "none published with this data and program",
    ),
    dataset(
        "lubm-uba-500-l",
        "LUBM from UBA 1.7, 500 universities, with the L program",
        &["oxford/lubm/LUBM_L.owl", "uba/lubm-500.owl"],
        "--lubm 500",
        "66,751,196 -> 91,128,727 facts (IJCAI 2023; its seed and index unstated)",
    ),
    // Large: fetched only with --large.
    dataset(
        "lubm-01k-l",
        "LUBM, 1,000 universities (Oxford, 133.6 M triples), with its L program",
        &["oxford/lubm/LUBM_L.owl", "oxford/lubm-01k/LUBM-01K.ttl"],
        "--large",
        "133.6 M -> 182.4 M (AAAI 2014)",
    ),
    dataset(
        "lubm-01k-le",
        "LUBM, 1,000 universities, with its LE program",
        &["oxford/lubm/LUBM_LE.owl", "oxford/lubm-01k/LUBM-01K.ttl"],
        "--large",
        "133.6 M -> 332.6 M (AAAI 2014)",
    ),
    dataset(
        "lubm-01k-u",
        "LUBM, 1,000 universities, with its U program",
        &["oxford/lubm/LUBM_U.owl", "oxford/lubm-01k/LUBM-01K.ttl"],
        "--large",
        "133.6 M -> 219.0 M (AAAI 2014)",
    ),
    dataset(
        "uobm-01k-l",
        "UOBM, 1,000 universities (Oxford, 254.8 M triples), with its L program",
        &["oxford/uobm/UOBM_L.owl", "oxford/uobm-01k/UOBM-01K.ttl"],
        "--large",
        "254.8 M -> 429.6 M (AAAI 2014); -> 426.2 M (AIJ 2019)",
    ),
    dataset(
        "uobm-01k-u",
        "UOBM, 1,000 universities, with its U program",
        &["oxford/uobm/UOBM_U.owl", "oxford/uobm-01k/UOBM-01K.ttl"],
        "--large",
        NONE,
    ),
    dataset(
        "dbpedia-l",
        "DBpedia, the 2014 subset (Oxford, 112.7 M triples), with its L program",
        &["oxford/dbpedia/DBpedia_L.owl", "oxford/dbpedia/DBPedia.ttl"],
        "--large",
        "112.7 M -> 118.3 M (AAAI 2014); L-eq 113 M -> 139 M AX, 136 M REW (AAAI 2015)",
    ),
    dataset(
        "dbpedia-le",
        "DBpedia, the 2014 subset, with its LE program",
        &["oxford/dbpedia/DBpedia_LE.owl", "oxford/dbpedia/DBPedia.ttl"],
        "--large",
        "112.7 M -> 1,529.7 M (AAAI 2014); -> 1.5 G (ISWC 2015)",
    ),
    dataset(
        "uniprot",
        "UniProt (Oxford, AAAI 2015: 123 M triples), data only: no OWL form of its program ships",
        &["oxford/uniprot/UniProt.ttl"],
        "--large",
        "123 M -> 228 M AX, 228 M REW (AAAI 2015, UniProt-eq: 451 rules, not this OWL 2 RL run)",
    ),
];
