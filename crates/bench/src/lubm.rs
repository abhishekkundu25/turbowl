//! LUBM-style university data: a deterministic generator shaped after the
//! Lehigh University Benchmark's data generator (UBA), and the univ-bench
//! ontology's axioms written out by hand as Turtle.
//!
//! This is not the official UBA output: the counts follow UBA's ranges, the
//! random choices come from a fixed seed here, and the names are neutral
//! (`http://example.org/...`). Numbers from it compare with other runs of this
//! generator only, not with published LUBM figures.

use std::io::{self, Write};

/// The ontology namespace.
pub(crate) const UB: &str = "http://example.org/univ-bench#";

/// The univ-bench ontology's class and property axioms that OWL 2 RL can use.
/// Left out: the two existentials on the right of a subclass axiom
/// (`GraduateStudent ⊑ ∃takesCourse.GraduateCourse`,
/// `ResearchAssistant ⊑ ∃worksFor.ResearchGroup`), which OWL 2 RL does not allow.
pub(crate) const ONTOLOGY: &str = r#"@prefix ub: <http://example.org/univ-bench#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

<http://example.org/univ-bench> a owl:Ontology ;
    rdfs:label "LUBM-style univ-bench ontology (written for turtle-reasoner's benchmark)" .

# Classes
ub:Organization a owl:Class .
ub:University a owl:Class ; rdfs:subClassOf ub:Organization .
ub:College a owl:Class ; rdfs:subClassOf ub:Organization .
ub:Department a owl:Class ; rdfs:subClassOf ub:Organization .
ub:Institute a owl:Class ; rdfs:subClassOf ub:Organization .
ub:Program a owl:Class ; rdfs:subClassOf ub:Organization .
ub:ResearchGroup a owl:Class ; rdfs:subClassOf ub:Organization .

ub:Person a owl:Class .
ub:Employee a owl:Class ;
    owl:equivalentClass [ a owl:Class ; owl:intersectionOf ( ub:Person
        [ a owl:Restriction ; owl:onProperty ub:worksFor ; owl:someValuesFrom ub:Organization ] ) ] .
ub:Faculty a owl:Class ; rdfs:subClassOf ub:Employee .
ub:Professor a owl:Class ; rdfs:subClassOf ub:Faculty .
ub:FullProfessor a owl:Class ; rdfs:subClassOf ub:Professor .
ub:AssociateProfessor a owl:Class ; rdfs:subClassOf ub:Professor .
ub:AssistantProfessor a owl:Class ; rdfs:subClassOf ub:Professor .
ub:VisitingProfessor a owl:Class ; rdfs:subClassOf ub:Professor .
ub:Chair a owl:Class ; rdfs:subClassOf ub:Professor ;
    owl:equivalentClass [ a owl:Class ; owl:intersectionOf ( ub:Person
        [ a owl:Restriction ; owl:onProperty ub:headOf ; owl:someValuesFrom ub:Department ] ) ] .
ub:Dean a owl:Class ; rdfs:subClassOf ub:Professor ;
    owl:equivalentClass [ a owl:Restriction ; owl:onProperty ub:headOf ; owl:someValuesFrom ub:College ] .
ub:Director a owl:Class ;
    owl:equivalentClass [ a owl:Class ; owl:intersectionOf ( ub:Person
        [ a owl:Restriction ; owl:onProperty ub:headOf ; owl:someValuesFrom ub:Program ] ) ] .
ub:Lecturer a owl:Class ; rdfs:subClassOf ub:Faculty .
ub:PostDoc a owl:Class ; rdfs:subClassOf ub:Faculty .
ub:AdministrativeStaff a owl:Class ; rdfs:subClassOf ub:Employee .
ub:ClericalStaff a owl:Class ; rdfs:subClassOf ub:AdministrativeStaff .
ub:SystemsStaff a owl:Class ; rdfs:subClassOf ub:AdministrativeStaff .
ub:Student a owl:Class ;
    owl:equivalentClass [ a owl:Class ; owl:intersectionOf ( ub:Person
        [ a owl:Restriction ; owl:onProperty ub:takesCourse ; owl:someValuesFrom ub:Course ] ) ] .
ub:UndergraduateStudent a owl:Class ; rdfs:subClassOf ub:Student .
ub:GraduateStudent a owl:Class ; rdfs:subClassOf ub:Person .
ub:ResearchAssistant a owl:Class ; rdfs:subClassOf ub:Person .
ub:TeachingAssistant a owl:Class ;
    owl:equivalentClass [ a owl:Class ; owl:intersectionOf ( ub:Person
        [ a owl:Restriction ; owl:onProperty ub:teachingAssistantOf ; owl:someValuesFrom ub:Course ] ) ] .

ub:Work a owl:Class .
ub:Course a owl:Class ; rdfs:subClassOf ub:Work .
ub:GraduateCourse a owl:Class ; rdfs:subClassOf ub:Course .
ub:Research a owl:Class ; rdfs:subClassOf ub:Work .

ub:Publication a owl:Class .
ub:Article a owl:Class ; rdfs:subClassOf ub:Publication .
ub:JournalArticle a owl:Class ; rdfs:subClassOf ub:Article .
ub:ConferencePaper a owl:Class ; rdfs:subClassOf ub:Article .
ub:TechnicalReport a owl:Class ; rdfs:subClassOf ub:Article .
ub:Book a owl:Class ; rdfs:subClassOf ub:Publication .
ub:Manual a owl:Class ; rdfs:subClassOf ub:Publication .
ub:Software a owl:Class ; rdfs:subClassOf ub:Publication .
ub:Specification a owl:Class ; rdfs:subClassOf ub:Publication .
ub:UnofficialPublication a owl:Class ; rdfs:subClassOf ub:Publication .

ub:Schedule a owl:Class .

# Object properties
ub:advisor a owl:ObjectProperty ; rdfs:domain ub:Person ; rdfs:range ub:Professor .
ub:affiliatedOrganizationOf a owl:ObjectProperty ; rdfs:domain ub:Organization ; rdfs:range ub:Organization .
ub:affiliateOf a owl:ObjectProperty ; rdfs:domain ub:Organization ; rdfs:range ub:Person .
ub:degreeFrom a owl:ObjectProperty ; rdfs:domain ub:Person ; rdfs:range ub:University ;
    owl:inverseOf ub:hasAlumnus .
ub:undergraduateDegreeFrom a owl:ObjectProperty ; rdfs:subPropertyOf ub:degreeFrom ;
    rdfs:domain ub:Person ; rdfs:range ub:University .
ub:mastersDegreeFrom a owl:ObjectProperty ; rdfs:subPropertyOf ub:degreeFrom ;
    rdfs:domain ub:Person ; rdfs:range ub:University .
ub:doctoralDegreeFrom a owl:ObjectProperty ; rdfs:subPropertyOf ub:degreeFrom ;
    rdfs:domain ub:Person ; rdfs:range ub:University .
ub:hasAlumnus a owl:ObjectProperty ; rdfs:domain ub:University ; rdfs:range ub:Person ;
    owl:inverseOf ub:degreeFrom .
ub:memberOf a owl:ObjectProperty ; owl:inverseOf ub:member .
ub:member a owl:ObjectProperty ; rdfs:domain ub:Organization ; rdfs:range ub:Person .
ub:worksFor a owl:ObjectProperty ; rdfs:subPropertyOf ub:memberOf .
ub:headOf a owl:ObjectProperty ; rdfs:subPropertyOf ub:worksFor .
ub:listedCourse a owl:ObjectProperty ; rdfs:domain ub:Schedule ; rdfs:range ub:Course .
ub:orgPublication a owl:ObjectProperty ; rdfs:domain ub:Organization ; rdfs:range ub:Publication .
ub:publicationAuthor a owl:ObjectProperty ; rdfs:domain ub:Publication ; rdfs:range ub:Person .
ub:publicationResearch a owl:ObjectProperty ; rdfs:domain ub:Publication ; rdfs:range ub:Research .
ub:researchProject a owl:ObjectProperty ; rdfs:domain ub:ResearchGroup ; rdfs:range ub:Research .
ub:softwareDocumentation a owl:ObjectProperty ; rdfs:domain ub:Software ; rdfs:range ub:Publication .
ub:subOrganizationOf a owl:TransitiveProperty ; rdfs:domain ub:Organization ; rdfs:range ub:Organization .
ub:takesCourse a owl:ObjectProperty .
ub:teacherOf a owl:ObjectProperty ; rdfs:domain ub:Faculty ; rdfs:range ub:Course .
ub:teachingAssistantOf a owl:ObjectProperty ; rdfs:domain ub:TeachingAssistant ; rdfs:range ub:Course .

# Datatype properties
ub:age a owl:DatatypeProperty ; rdfs:domain ub:Person .
ub:emailAddress a owl:DatatypeProperty ; rdfs:domain ub:Person .
ub:name a owl:DatatypeProperty .
ub:officeNumber a owl:DatatypeProperty .
ub:publicationDate a owl:DatatypeProperty ; rdfs:domain ub:Publication .
ub:researchInterest a owl:DatatypeProperty .
ub:softwareVersion a owl:DatatypeProperty ; rdfs:domain ub:Software .
ub:telephone a owl:DatatypeProperty ; rdfs:domain ub:Person .
ub:tenured a owl:DatatypeProperty ; rdfs:domain ub:Professor .
ub:title a owl:DatatypeProperty ; rdfs:domain ub:Person .
"#;

/// The data namespace.
const DATA: &str = "http://example.org/lubm/";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// UBA picks degree-granting universities from this many, created or not.
const DEGREE_UNIVERSITIES: u64 = 1000;

/// SplitMix64: small, fast, and the same on every platform.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// A number in `lo..=hi`.
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

struct Out<W: Write> {
    w: W,
    triples: u64,
}

impl<W: Write> Out<W> {
    fn iri(&mut self, s: &str, p: &str, o: &str) -> io::Result<()> {
        self.triples += 1;
        writeln!(self.w, "<{s}> <{p}> <{o}> .")
    }
    fn ub(&mut self, s: &str, p: &str, o: &str) -> io::Result<()> {
        self.triples += 1;
        writeln!(self.w, "<{s}> <{UB}{p}> <{o}> .")
    }
    fn class(&mut self, s: &str, class: &str) -> io::Result<()> {
        self.iri(s, RDF_TYPE, &format!("{UB}{class}"))
    }
    fn lit(&mut self, s: &str, p: &str, value: &str) -> io::Result<()> {
        self.triples += 1;
        writeln!(self.w, "<{s}> <{UB}{p}> \"{value}\" .")
    }
}

fn university(u: u64) -> String {
    format!("{DATA}u{u}")
}

/// Writes the data of `universities` universities as N-Triples (also valid
/// Turtle) and returns the number of triples. The same count gives the same
/// bytes on every machine.
pub(crate) fn generate(universities: u64, w: impl Write) -> io::Result<u64> {
    let mut out = Out { w, triples: 0 };
    let mut rng = Rng(0x4c55_424d ^ universities);
    for u in 0..universities {
        let uni = university(u);
        out.class(&uni, "University")?;
        out.lit(&uni, "name", &format!("University{u}"))?;
        for d in 0..rng.range(15, 25) {
            department(&mut out, &mut rng, u, d)?;
        }
    }
    out.w.flush()?;
    Ok(out.triples)
}

fn department<W: Write>(out: &mut Out<W>, rng: &mut Rng, u: u64, d: u64) -> io::Result<()> {
    let uni = university(u);
    let dept = format!("{uni}/d{d}");
    out.class(&dept, "Department")?;
    out.ub(&dept, "subOrganizationOf", &uni)?;
    out.lit(&dept, "name", &format!("Department{d}"))?;

    for g in 0..rng.range(10, 20) {
        let group = format!("{dept}/ResearchGroup{g}");
        out.class(&group, "ResearchGroup")?;
        out.ub(&group, "subOrganizationOf", &dept)?;
    }
    let groups = |rng: &mut Rng| rng.below(10);

    // Faculty by rank: (class, how many, publications each).
    let ranks = [
        ("FullProfessor", rng.range(7, 10), (15, 20)),
        ("AssociateProfessor", rng.range(10, 14), (10, 18)),
        ("AssistantProfessor", rng.range(8, 11), (5, 10)),
        ("Lecturer", rng.range(5, 7), (0, 5)),
    ];
    let mut professors = Vec::new();
    let mut publications = Vec::new();
    let (mut courses, mut grad_courses) = (0u64, 0u64);
    for (class, count, (pub_lo, pub_hi)) in ranks {
        for i in 0..count {
            let person = format!("{dept}/{class}{i}");
            out.class(&person, class)?;
            if class == "FullProfessor" && i == 0 {
                out.ub(&person, "headOf", &dept)?;
            } else {
                out.ub(&person, "worksFor", &dept)?;
            }
            out.lit(&person, "name", &format!("{class}{i}"))?;
            out.lit(&person, "emailAddress", &format!("{class}{i}@d{d}.u{u}.example.org"))?;
            out.lit(&person, "telephone", &format!("{:010}", rng.next() % 10_000_000_000))?;
            out.lit(&person, "researchInterest", &format!("Research{}", rng.below(30)))?;
            for degree in ["undergraduateDegreeFrom", "mastersDegreeFrom", "doctoralDegreeFrom"] {
                out.ub(&person, degree, &university(rng.below(DEGREE_UNIVERSITIES)))?;
            }
            for _ in 0..rng.range(1, 2) {
                out.ub(&person, "teacherOf", &format!("{dept}/Course{courses}"))?;
                out.class(&format!("{dept}/Course{courses}"), "Course")?;
                courses += 1;
            }
            for _ in 0..rng.range(1, 2) {
                let course = format!("{dept}/GraduateCourse{grad_courses}");
                out.ub(&person, "teacherOf", &course)?;
                out.class(&course, "GraduateCourse")?;
                grad_courses += 1;
            }
            for p in 0..rng.range(pub_lo, pub_hi) {
                let publication = format!("{person}/Publication{p}");
                out.class(&publication, "Publication")?;
                out.lit(&publication, "name", &format!("Publication{p}"))?;
                out.ub(&publication, "publicationAuthor", &person)?;
                publications.push(publication);
            }
            if class != "Lecturer" {
                professors.push(person);
            }
        }
    }
    let faculty: u64 = ranks.iter().map(|r| r.1).sum();

    for s in 0..faculty * rng.range(8, 14) {
        let student = format!("{dept}/UndergraduateStudent{s}");
        out.class(&student, "UndergraduateStudent")?;
        out.ub(&student, "memberOf", &dept)?;
        out.lit(&student, "name", &format!("UndergraduateStudent{s}"))?;
        out.lit(
            &student,
            "emailAddress",
            &format!("UndergraduateStudent{s}@d{d}.u{u}.example.org"),
        )?;
        out.lit(&student, "telephone", &format!("{:010}", rng.next() % 10_000_000_000))?;
        for _ in 0..rng.range(2, 4) {
            out.ub(&student, "takesCourse", &format!("{dept}/Course{}", rng.below(courses)))?;
        }
        if rng.below(5) == 0 {
            let advisor = &professors[rng.below(professors.len() as u64) as usize];
            out.ub(&student, "advisor", advisor)?;
        }
    }

    for s in 0..faculty * rng.range(3, 4) {
        let student = format!("{dept}/GraduateStudent{s}");
        out.class(&student, "GraduateStudent")?;
        out.ub(&student, "memberOf", &dept)?;
        out.lit(&student, "name", &format!("GraduateStudent{s}"))?;
        out.lit(
            &student,
            "emailAddress",
            &format!("GraduateStudent{s}@d{d}.u{u}.example.org"),
        )?;
        out.lit(&student, "telephone", &format!("{:010}", rng.next() % 10_000_000_000))?;
        out.ub(
            &student,
            "undergraduateDegreeFrom",
            &university(rng.below(DEGREE_UNIVERSITIES)),
        )?;
        for _ in 0..rng.range(1, 3) {
            out.ub(
                &student,
                "takesCourse",
                &format!("{dept}/GraduateCourse{}", rng.below(grad_courses)),
            )?;
        }
        let advisor = &professors[rng.below(professors.len() as u64) as usize];
        out.ub(&student, "advisor", advisor)?;
        if rng.below(5) == 0 {
            out.class(&student, "TeachingAssistant")?;
            out.ub(
                &student,
                "teachingAssistantOf",
                &format!("{dept}/Course{}", rng.below(courses)),
            )?;
        }
        if rng.below(4) == 0 {
            out.class(&student, "ResearchAssistant")?;
            out.ub(&student, "worksFor", &format!("{dept}/ResearchGroup{}", groups(rng)))?;
        }
        for _ in 0..rng.range(0, 5) {
            let publication = &publications[rng.below(publications.len() as u64) as usize];
            out.ub(publication, "publicationAuthor", &student)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_count_gives_same_bytes() {
        let (mut a, mut b) = (Vec::new(), Vec::new());
        let na = generate(1, &mut a).unwrap();
        let nb = generate(1, &mut b).unwrap();
        assert_eq!(a, b);
        assert_eq!(na, nb);
        assert_eq!(na as usize, a.iter().filter(|&&c| c == b'\n').count());
    }

    #[test]
    fn one_university_is_lubm_sized() {
        let n = generate(1, io::sink()).unwrap();
        assert!((80_000..200_000).contains(&n), "{n} triples");
    }

    #[test]
    fn data_and_ontology_parse() {
        let mut data = Vec::new();
        generate(1, &mut data).unwrap();
        for t in oxttl::TurtleParser::new().for_slice(&data) {
            t.unwrap();
        }
        let ontology: Vec<_> = oxttl::TurtleParser::new()
            .for_slice(ONTOLOGY)
            .collect::<Result<_, _>>()
            .unwrap();
        let n = ontology.len();
        assert!(n > 100, "{n} ontology triples");
    }
}
