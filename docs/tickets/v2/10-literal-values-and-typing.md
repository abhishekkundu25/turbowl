# 10: Literal values and typing

Status: todo. Blocked by: 08, 09.

The `literal-values` group, off by default: dt-eq merges literals with equal values (through the canon), and dt-diff makes two literals with different values that become `owl:sameAs` an eq-diff1 finding, without listing the `owl:differentFrom` triples. dt-type2 joins the `trivial` group: each literal is typed with every datatype whose value space holds its value.

## Acceptance
- [ ] One minimal input per rule, with its expected inferences or findings; Nemo (where it can judge), OWL-RL and the reference evaluator agree with it.
- [ ] The generator draws the new shapes (a test counts them over the first 300 seeds), and 20,000 full and 20,000 incremental chaos iterations pass with the group on.
- [ ] `"1"^^xsd:integer` and `"01"^^xsd:integer` are one class, and so are `"1"^^xsd:integer` and `"1.0"^^xsd:decimal`; `"1"^^xsd:integer owl:sameAs "2"^^xsd:integer` is reported.
- [ ] No `owl:differentFrom` triple between literals appears in any closure, and the W3C runner lists this departure.
- [ ] A `scaling_*` test: dt-eq over n literals is linear.
