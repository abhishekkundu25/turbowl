# 09: The OWL 2 datatype map

Status: todo. Blocked by: 04.

The value space and lexical mapping of every OWL 2 RL datatype (OWL 2 Profiles §4.2) in one table that every datatype rule reads. dt-not-type checks all of them, in the core and the reference evaluator. The datatype minimal inputs and generator shapes get their own files.

## Acceptance
- [ ] A unit table of edge cases for each datatype (bounds, whitespace, signs, exponents, time zones, the integer subtypes inside decimals), in the core and the reference evaluator alike.
- [ ] The RDF-MT datatype cases pass, at least the 37 of today, and `WebOnt-I4.6-005-Direct` and `WebOnt-equivalentClass-008-Direct` pass if their missing rule is dt-not-type.
- [ ] dt-not-type agrees with OWL-RL apart from faults F3 and F5, and with the reference evaluator on 20,000 chaos iterations with literals of every datatype.
- [ ] A `scaling_*` test: checking n literals is linear.
