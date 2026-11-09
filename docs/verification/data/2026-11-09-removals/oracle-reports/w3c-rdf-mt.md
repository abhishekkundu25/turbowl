| Case | Expects | Approved | Regime | Nemo | OWL-RL | ours |
|---|---|---|---|---|---|---|
| datatypes-intensional-xsd-integer-decimal-compatible | consistent | yes | RDFS | pass | pass | pass |
| datatypes-intensional-xsd-integer-string-incompatible | inconsistent | yes | RDFS | FAIL | FAIL | pass |
| datatypes-non-well-formed-literal-1 | consistent | yes | RDFS | pass | pass | pass |
| datatypes-non-well-formed-literal-2 | inconsistent | yes | RDFS | FAIL | pass | pass |
| datatypes-plain-literal-and-xsd-string | entails | yes | RDFS | pass | pass | pass |
| datatypes-range-clash | inconsistent | yes | RDFS | FAIL | FAIL | pass |
| datatypes-semantic-equivalence-between-datatypes | entails | yes | RDF | FAIL | FAIL | FAIL |
| datatypes-semantic-equivalence-within-type-1 | entails | yes | RDF | FAIL | FAIL | FAIL |
| datatypes-semantic-equivalence-within-type-2 | entails | yes | RDF | FAIL | FAIL | FAIL |
| datatypes-test008 | entails | yes | simple | pass | pass | pass |
| datatypes-test009 | not entails | yes | simple | pass | pass | pass |
| datatypes-test010 | inconsistent | yes | RDFS | FAIL | FAIL | pass |
| double-infinity | entails | no | RDF | FAIL | FAIL | FAIL |
| double-round-different | not entails | no | RDF | pass | pass | pass |
| double-round-same | entails | no | RDF | FAIL | FAIL | FAIL |
| double-zero | not entails | no | RDF | pass | pass | pass |
| float-infinity | entails | no | RDF | FAIL | FAIL | FAIL |
| float-round-different | not entails | no | RDF | pass | pass | pass |
| float-round-same | entails | no | RDF | FAIL | FAIL | FAIL |
| float-zero | not entails | no | RDF | pass | pass | pass |
| horst-01-subClassOf-intensional | not entails | yes | RDFS | pass | pass | pass |
| horst-01-subPropertyOf-intensional | not entails | yes | RDFS | pass | pass | pass |
| literal-type | entails | no | RDF | FAIL | FAIL | FAIL |
| pfps-10-non-well-formed-literal-1 | entails | yes | RDFS | FAIL | FAIL | FAIL |
| rdf-charmod-uris-test003 | not entails | yes | RDF | pass | pass | pass |
| rdf-charmod-uris-test004 | not entails | yes | RDF | pass | pass | pass |
| rdfms-seq-representation-test002 | entails | yes | RDFS | FAIL | FAIL | FAIL |
| rdfms-seq-representation-test003 | entails | yes | RDFS | FAIL | FAIL | FAIL |
| rdfms-seq-representation-test004 | entails | yes | RDFS | FAIL | FAIL | FAIL |
| rdfms-xmllang-test007a | not entails | yes | simple | pass | pass | pass |
| rdfms-xmllang-test007b | not entails | yes | simple | pass | pass | pass |
| rdfms-xmllang-test007c | not entails | yes | simple | pass | pass | pass |
| rdfs-container-membership-superProperty-test001 | not entails | yes | RDFS | pass | pass | pass |
| rdfs-domain-and-range-intensionality-domain | not entails | yes | RDFS | FAIL | FAIL | FAIL |
| rdfs-domain-and-range-intensionality-range | not entails | yes | RDFS | FAIL | FAIL | FAIL |
| rdfs-entailment-test001 | inconsistent | yes | RDFS | FAIL | pass | pass |
| rdfs-entailment-test002 | inconsistent | yes | RDFS | FAIL | FAIL | FAIL |
| rdfs-no-cycles-in-subClassOf-test001 | entails | yes | RDFS | pass | pass | pass |
| rdfs-no-cycles-in-subPropertyOf-test001 | entails | yes | RDFS | pass | pass | pass |
| rdfs-subClassOf-a-Property-test001 | consistent | yes | RDFS | pass | pass | pass |
| rdfs-subPropertyOf-semantics-test001 | entails | yes | RDFS | pass | pass | pass |
| statement-entailment-test001 | not entails | yes | RDF | pass | pass | pass |
| statement-entailment-test002 | not entails | yes | RDF | pass | pass | pass |
| statement-entailment-test003 | not entails | yes | RDFS | pass | pass | pass |
| statement-entailment-test004 | not entails | yes | RDF | pass | pass | pass |
| tex-01-language-tag-case-1 | entails | yes | RDF | pass | pass | pass |
| tex-01-language-tag-case-2 | entails | yes | RDF | pass | pass | pass |
| xmlsch-02-whitespace-facet-1 | not entails | yes | RDFS | pass | pass | pass |
| xmlsch-02-whitespace-facet-2 | inconsistent | yes | RDFS | FAIL | FAIL | pass |
| xmlsch-02-whitespace-facet-3 | entails | yes | RDFS | FAIL | FAIL | FAIL |
| xmlsch-02-whitespace-facet-4 | inconsistent | yes | RDFS | FAIL | FAIL | pass |

Passed, of 51 runnable cases: Nemo 28; OWL-RL 30; ours 35;

In the complete configuration, the rule program (Nemo) passes 28; TurbOWL must pass each of them.

Nemo fails 23, with the rules they likely need that the program does not hold yet (from their vocabulary):
- datatypes-intensional-xsd-integer-string-incompatible (OWL-RL: FAIL): unknown
- datatypes-non-well-formed-literal-2 (OWL-RL: pass): dt-not-type
- datatypes-range-clash (OWL-RL: FAIL): dt-not-type
- datatypes-semantic-equivalence-between-datatypes (OWL-RL: FAIL): dt-not-type
- datatypes-semantic-equivalence-within-type-1 (OWL-RL: FAIL): dt-not-type
- datatypes-semantic-equivalence-within-type-2 (OWL-RL: FAIL): dt-not-type
- datatypes-test010 (OWL-RL: FAIL): dt-not-type
- double-infinity (OWL-RL: FAIL): dt-not-type
- double-round-same (OWL-RL: FAIL): dt-not-type
- float-infinity (OWL-RL: FAIL): dt-not-type
- float-round-same (OWL-RL: FAIL): dt-not-type
- literal-type (OWL-RL: FAIL): dt-not-type
- pfps-10-non-well-formed-literal-1 (OWL-RL: FAIL): dt-not-type
- rdfms-seq-representation-test002 (OWL-RL: FAIL): rdfs12 and RDFS axioms (container membership; not OWL 2 RL rules)
- rdfms-seq-representation-test003 (OWL-RL: FAIL): rdfs12 and RDFS axioms (container membership; not OWL 2 RL rules)
- rdfms-seq-representation-test004 (OWL-RL: FAIL): rdfs12 and RDFS axioms (container membership; not OWL 2 RL rules)
- rdfs-domain-and-range-intensionality-domain (OWL-RL: FAIL): unknown
- rdfs-domain-and-range-intensionality-range (OWL-RL: FAIL): unknown
- rdfs-entailment-test001 (OWL-RL: pass): dt-not-type
- rdfs-entailment-test002 (OWL-RL: FAIL): dt-not-type
- xmlsch-02-whitespace-facet-2 (OWL-RL: FAIL): dt-not-type
- xmlsch-02-whitespace-facet-3 (OWL-RL: FAIL): dt-not-type
- xmlsch-02-whitespace-facet-4 (OWL-RL: FAIL): dt-not-type

Departures from the W3C rule tables, by design:
- dt-diff: never listed (docs/specs/v2-1-scoreboard-and-rules.md): `lt1 owl:differentFrom lt2` for two literals with different values is in no closure, nor is anything a rule derives from it but eq-diff1, which reports two such literals (and, with the equality rules, every two terms equal to them) made owl:sameAs; the W3C rule set lists a triple for every two literals of the closure
