# 10: Literal values and typing

Status: done. Blocked by: 08, 09.

The `literal-values` group, off by default: dt-eq merges literals with equal values (through the canon), and dt-diff makes two literals with different values that become `owl:sameAs` an eq-diff1 finding, without listing the `owl:differentFrom` triples. dt-type2 joins the `trivial` group: each literal is typed with every datatype whose value space holds its value.

Every datatype rule ranges over the datatypes OWL 2 RL supports, as Table 8 says ("each datatype dt supported in OWL 2 RL"): the 32 of OWL 2 Profiles, Section 4.2.1, `Datatype::OWL_2_RL`. Ticket 09 checked all 35 of the map, so dt-not-type changes too: `owl:real` and `owl:rational`, which OWL 2 RL excludes, are read like a datatype outside the map (a literal of either has no known value: never ill-typed, never typed by dt-type2, never merged by dt-eq), and `rdf:langString` is not checked as a datatype. A language-tagged literal keeps its value, since OWL 2 reads it as an `rdf:PlainLiteral`. dt-type1 already types only the 32. New minimal inputs for the range go at the end of `crates/oracle/src/cases/datatypes.rs`, whose existing cases ticket 26 edits.

## Acceptance
- [x] One minimal input per rule, with its expected inferences or findings; Nemo (where it can judge), OWL-RL and the reference evaluator agree with it.
- [x] The generator draws the new shapes (a test counts them over the first 300 seeds), and 20,000 full and 20,000 incremental chaos iterations pass with the group on.
- [x] `"1"^^xsd:integer` and `"01"^^xsd:integer` are one class, and so are `"1"^^xsd:integer` and `"1.0"^^xsd:decimal`; `"1"^^xsd:integer owl:sameAs "2"^^xsd:integer` is reported.
- [x] No `owl:differentFrom` triple between literals appears in any closure, and the W3C runner lists this departure.
- [x] A `scaling_*` test: dt-eq over n literals is linear.
- [x] dt-type2, dt-not-type, dt-eq and dt-diff range over `Datatype::OWL_2_RL` in the core, the reference evaluator and the rule source: `"1"^^owl:real` and `"1/3"^^owl:rational` give no finding, and `"chat"@fr rdf:type xsd:string` still does.
