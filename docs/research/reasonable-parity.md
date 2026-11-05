# `reasonable`: feature inventory and parity checklist

Source: <https://github.com/gtfierro/reasonable>, branch `master`, commit
`fecee2b6d3713f97c86aad56e099674dcacea75b` (2026-09-17, "Include the
workspace Cargo.toml in uv's cache keys"). Licence BSD-3-Clause
(`Cargo.toml:15`). Workspace version `0.5.0-a1` (`Cargo.toml:10`); crates.io
lists `reasonable` and `reasonable-cli` at max version `0.5.0-a1`, max stable
`0.4.4` (crates.io API, read 2026-11-05).

Every citation below is `path:line` in that commit. Only the code, tests,
docs and manifests were read. Nothing was built or run. Claims marked
**(from code)** are read off the source and no test in the repository
covers them; check them against a built oracle before you rely on them.

Workspace layout: `lib/` (crate `reasonable`), `cli/` (crate
`reasonable-cli`, binary `reasonable`), `python/` (crate `pyreasonable`,
cdylib `reasonable`), plus `reasonable/__init__.py`, `benches/`,
`example_models/`, `docs/` (Sphinx) and `scripts/` (`Cargo.toml:1-7`).

---

## 1. Rules

### 1.1 Engine shape

- One datafrog `Iteration` (`iter1`) holds every rule. Rules are not
  stratified. All rules run inside `while self.iter1.changed()`
  (`lib/src/reasoner.rs:1343-2209`). An outer `while changed` loop wraps it
  but always runs once, because `changed = false` is set unconditionally
  (`lib/src/reasoner.rs:1339`, `:2218`).
- Triple store: `all_triples_input: Variable<(s,(p,o))>`. Three index
  variables are rebuilt from it each round: `spo` keyed by s, `pso` keyed by
  p, and `osp` keyed by o (`lib/src/reasoner.rs:1350-1361`).
- Vocabulary constants are single-row static `Relation<(URI,())>` values,
  such as `rdf_type_relation` and `owl_sameas_relation`
  (`lib/src/reasoner.rs:315-340`, `:463-493`). Schema relations are
  extracted by joining `pso` with them, for example
  `owl_same_as = pso ⋈ {owl:sameAs}` (`lib/src/reasoner.rs:1447-1448`).
- `rdf_type` holds (inst, class) and `rdf_type_inv` holds (class, inst)
  (`:1363-1369`). Each `rdf_type` tuple is also inserted into the side
  `HashSet` `instances` (`:1364`).
- Several rules emit the sentinel triple `(0,(0,0))` when a join condition
  fails (`:1591`, `:1611`, `:2144`, `:2169`, `:2192`, `:2206`). Id 0 is the
  placeholder IRI `urn:_` (`lib/src/index.rs:13-19`). Output drops every
  triple with a 0 component (`lib/src/reasoner.rs:2228`).
- Inconsistency rules collect violations into local `Vec`s inside the join
  closure, then call `add_error` (`:1549-1555`, `:1567-1573`, `:1791-1810`,
  `:1827-1861`, `:1907-1924`). They derive no triples.

### 1.2 Extra rule that is not OWL 2 RL

- **Every subject is typed `owl:Thing`**:
  `all_triples_input ⊇ { (s, rdf:type, owl:Thing) | (s,_,_) ∈ spo }`
  (`lib/src/reasoner.rs:1344-1348`). This covers classes, properties, list
  cells, restriction blank nodes, and literal subjects created internally.
  It does not cover nodes that occur only as objects. The W3C RL rules have
  no such rule.

### 1.3 Rule-by-rule status (W3C OWL 2 RL, Tables 4–9)

"Diag" means the rule emits a diagnostic and no triples.

**Equality (Table 4)**

| Rule | Status | How it is coded |
|---|---|---|
| eq-ref | not implemented | Commented out (`lib/src/reasoner.rs:1494-1507`); test `#[ignore]` (`lib/src/tests.rs:67-79`) |
| eq-sym | implemented | `all_triples_input ← spo ⋈_x owl_same_as`, emits `(y sameAs x)` (`:1509-1515`) |
| eq-trans | implemented | `owl_same_as ⋈_y owl_same_as`, emits `(x sameAs z)` (`:1536-1543`). With eq-sym this also yields `x sameAs x` for every node that has a sameAs edge. |
| eq-rep-s | implemented | `spo ⋈ owl_same_as` (`:1517-1522`) |
| eq-rep-p | implemented | `pso ⋈ owl_same_as` (`:1523-1528`) |
| eq-rep-o | implemented | `osp ⋈ owl_same_as` (`:1529-1534`) |
| eq-diff1/2/3 | not implemented | `owl:differentFrom` and `owl:AllDifferent` are never interned (`:431-461`) |

**Property axioms (Table 5)**

| Rule | Status | How it is coded |
|---|---|---|
| prp-ap | not implemented | — |
| prp-dom | implemented | `prp_dom = pso ⋈ {rdfs:domain}`, then `prp_dom ⋈_p pso` emits `(s type C)` (`:1371-1381`) |
| prp-rng | implemented | Same shape, emits `(o type C)` (`:1383-1393`). Literal objects get `rdf:type` internally; those triples are dropped at output (§2.4). |
| prp-fp | implemented | `prp_fp_1` holds functional properties (`:1576-1580`); `prp_fp_2 = prp_fp_1 ⋈ pso` (`:1581-1582`); then `prp_fp_2 ⋈_p pso` keeps pairs with `x1==x2` and emits `y1 sameAs y2` (`:1583-1594`). The join is on p only, so its cost is quadratic in the number of p-triples. It also emits `y sameAs y` for every object of a functional property. |
| prp-ifp | **implemented with a bug** | Same shape keyed on p (`:1596-1614`). The guard `match y1 { y2 => …, _ => … }` binds a new `y2` and always matches (`:1609-1612`). Every pair of subjects that use the property becomes `owl:sameAs`, even when the objects differ. Reported as issue #64; PR #65 (open) replaces it with `if y1 == y2`. The test checks only the positive case (`lib/src/tests.rs:364-385`). |
| prp-irp | diag | `prp_irp_1 = owl_irreflexive ⋈ pso`; flags `s==o && s>0` (`:1545-1555`); message at `:1834-1841` |
| prp-symp | implemented | `symmetric_properties ⋈ pso` (`:1863-1871`) |
| prp-asyp | diag | `prp_asyp_1` keys `(x,y,p)` and `prp_asyp_2` keys `(y,x,p)`; their join flags violations (`:1557-1573`); message at `:1842-1850`. `x p y` plus `y p x` produces two diagnostics, (x,y) and (y,x). `x p x` alone also fires. |
| prp-trp | implemented | `prp_trp_1 ((y,p),x)` ⋈ `prp_trp_2 ((y,p),z)` (`:1873-1884`) |
| prp-spo1 | implemented | `prp_spo1_1 = pso ⋈ {rdfs:subPropertyOf}`, then `⋈ pso` (`:1616-1626`) |
| prp-spo2 | implemented (feature `prp-spo2`, on by default) | General n-hop. `ChainVars` (`:209-272`) builds `chain(L,u,v)` with a base, a step and an emit rule (`:1636-1727`). Only list cells reachable from an axiom's head are considered (`:1657-1677`). A cyclic `rdf:rest` terminates (test `lib/src/tests.rs:750`). |
| prp-eqp1 / prp-eqp2 | implemented | `equivalent_properties` and `equivalent_properties_2` (`:1483-1492`), each joined with `pso` (`:1886-1903`) |
| prp-pdw | diag | `prp_pdw_1 ((x,y,p2),p1)` ⋈ `prp_pdw_2` (`:1812-1833`); message at `:1851-1861` |
| prp-adp | not implemented | — |
| prp-inv1 / prp-inv2 | implemented | `owl_inv1` and `owl_inv2` (`:1395-1397`), each joined with `pso` (`:1729-1739`) |
| prp-key | not implemented | `owl:hasKey` is not interned |
| prp-npa1 / prp-npa2 | not implemented | — |

**Classes (Table 6)**

| Rule | Status | How it is coded |
|---|---|---|
| cls-thing | implemented | Seed triple `owl:Thing a owl:Class` (`:569-574`) |
| cls-nothing1 | implemented | Seed triple `owl:Nothing a owl:Class` (`:569-574`). `set_base_triples` adds both seeds again (`:911-914`). |
| cls-nothing2 | diag | `rdf_type_inv ⋈ {owl:Nothing}` (`:1905-1924`) |
| cls-int1 | implemented (procedural) | Builds `class → [inst]` from the `instances` HashSet each round. For each entry in the `intersections` HashMap, it takes the members of the smallest class and checks they are in every other class (`:1926-1991`). |
| cls-int2 | implemented | `rdf_type_inv ⋈ owl_intersection_of`; the list is read from `DisjointSets` (`:1993-2007`) |
| cls-uni | implemented | The `unions` HashMap gives (member, union) pairs into `union_mem_var`, then `⋈ rdf_type_inv` (`:2009-2027`) |
| cls-com | **partial: no triples, no diagnostic** | Builds `cls_com_1` and `cls_com_2` (`:2034-2054`), but the `add_error("cls-com", …)` call is commented out (`:2043-2051`). `OWLRL.CLS_COM` is mapped (`:63`) but nothing emits it. |
| cls-svf1 | implemented | `cls_svf1_1 (p,(x,y))` ⋈ `pso` gives `cls_svf1_2`, then `⋈ rdf_type` with check `class==y` (`:2174-2195`) |
| cls-svf2 | implemented | Fires only when the filler is the IRI `owl:Thing` (`:2197-2208`) |
| cls-avf | implemented | `cls_avf_1`, then `cls_avf_2 = rdf_type_inv ⋈ cls_avf_1`, then `⋈ spo` with check `p1==p2` (`:2149-2172`) |
| cls-hv1 | implemented | `owl_has_value ⋈ owl_on_property`, then `⋈ rdf_type_inv` (`:2117-2126`) |
| cls-hv2 | implemented | `cls_hv2_1 (p,(y,x))` ⋈ `pso` with check `value==obj` (`:2128-2147`) |
| cls-maxc1/2, cls-maxqc1–4 | not implemented | `owl:maxCardinality` and `owl:maxQualifiedCardinality` are not interned |
| cls-oo | not implemented | `owl:oneOf` is not interned |

**Class axioms (Table 7)**

| Rule | Status | How it is coded |
|---|---|---|
| cax-sco | implemented | `cax_sco_1 (c1,c2)` ⋈ `cax_sco_2 (class,inst)` (`:1741-1755`) |
| cax-eqc1 / cax-eqc2 | implemented | `rdf_type_inv ⋈ owl_equivalent_class`. That relation holds both directions (`:1463-1472`, `:1776-1782`). |
| cax-dw | diag | `cax_dw_1 = rdf_type_inv ⋈ owl_disjoint_with`, then `cax_dw_2 = rdf_type_inv ⋈ cax_dw_1` with `inst1==inst2` (`:1784-1810`). Only the asserted direction of `owl:disjointWith` is checked, so `A dw B` gives one diagnostic, worded "is both A and B". |
| cax-adc | not implemented | `owl:AllDisjointClasses` is not interned |

**Datatypes (Table 8)**: `dt-type1`, `dt-type2`, `dt-eq`, `dt-diff` and
`dt-not-type` are not implemented. The `RDFS.DATATYPE` diagnostic (§3) is a
narrow form of `dt-not-type` for five datatypes. Literals are compared as
terms (lexical form plus datatype or language), never by value
(`lib/src/index.rs:22-30`).

**Schema vocabulary (Table 9)**

| Rule | Status | How it is coded |
|---|---|---|
| scm-sco (= rdfs11) | implemented | `rdfs11_1 (c2,c1)` ⋈ `cax_sco_1 (c2,c3)` (`:1757-1765`) |
| scm-eqc1 | implemented | `owl_equivalent_class` (both directions) gives `c1 subClassOf c2` (`:1767-1774`) |
| scm-eqc2 (mutual subClassOf ⇒ equivalentClass) | **not implemented** | No rule emits `owl:equivalentClass`. `c2 equivalentClass c1` is never emitted as a triple either. |
| scm-cls, scm-op, scm-dp, scm-spo, scm-eqp1/2, scm-dom1/2, scm-rng1/2, scm-hv, scm-svf1/2, scm-avf1/2, scm-int, scm-uni | not implemented | `rdfs:subPropertyOf` is used only by prp-spo1 (`:1616-1626`), so the closure of `subPropertyOf` triples is never materialised. |

**RDFS (RDF 1.1 Semantics)**: rdfs2/rdfs3 = prp-dom/prp-rng;
rdfs7 = prp-spo1; rdfs9 = cax-sco; rdfs11 = scm-sco. "rdfs12" is coded as
the container-membership axioms below. rdfs1, rdfs4a, rdfs4b, rdfs5, rdfs6,
rdfs8, rdfs10 and rdfs13 are not implemented.

**Container-membership axioms** (`lib/src/reasoner.rs:997-1046`, called at
`:1309`). For every predicate IRI matching `rdf:_N` with N > 0, ASCII digits
only (`:2327-2335`), the reasoner adds three triples: `rdf:_N a rdf:Property`,
`rdf:_N a rdfs:ContainerMembershipProperty` and
`rdf:_N rdfs:subPropertyOf rdfs:member`. It adds them to the base, so they
survive `clear()`. Only IRIs used in predicate position trigger the axioms.
This is not W3C rdfs12 (`?x a rdfs:ContainerMembershipProperty ⇒ ?x
subPropertyOf rdfs:member`): that rule is not implemented for other
properties.

### 1.4 Lists (`rdf:first` / `rdf:rest`)

- **`DisjointSets`** (`lib/src/disjoint_sets.rs:14-75`) is built once per
  `reason()` call from the triples present at the start of the call
  (`lib/src/reasoner.rs:1321-1323`, `:1327`). It unions each cell with its
  `rdf:rest` tail, ignoring `rdf:nil` (`disjoint_sets.rs:49-58`), and groups
  `rdf:first` values by set representative (`:59-68`). Consequences
  **(from code)**:
  - List order is lost, and duplicate members are kept.
  - Two lists that share a tail cell merge into one set, so cls-int1,
    cls-int2 and cls-uni see the union of both lists' members.
  - List triples derived during reasoning are not seen until the next
    `reason()` call.
- cls-int1 and cls-uni read the `intersections` and `unions` maps, which are
  `HashMap<class, list>` (`lib/src/reasoner.rs:1403`, `:1409`). If a class
  has two `owl:intersectionOf` or `owl:unionOf` lists, only the last one
  inserted is used. cls-int2 uses the full `owl_intersection_of` variable,
  so it sees every list.
- prp-spo2 does not use `DisjointSets`. It walks the list in datalog with
  order preserved (`:1636-1727`). Tests cover shared suffixes, order,
  length 1, 3-hop and arity sweeps (`lib/src/tests.rs:408-866`).

### 1.5 README rule tables against the code

The README tables are at `README.md:187-280`. The Sphinx copy is
`docs/owl-rules/index.rst:20-274`.

| Item | README / docs | Code |
|---|---|---|
| cls-com | "yes, throws exception" (`README.md:247`); docs say "partial, emits a diagnostic" (`docs/owl-rules/index.rst:188-190`) | No triples and no diagnostic (`lib/src/reasoner.rs:2043-2051`) |
| scm-eqc2 | "`owl:equivalentClass` → `rdfs:subClassOf` (other direction)" (`README.md:276`) | That behaviour is W3C scm-eqc1. W3C scm-eqc2 (mutual subClassOf ⇒ equivalentClass) is absent. |
| prp-ifp | "yes" (`README.md:220`) | Over-derives sameAs (issue #64) |
| prp-irp, prp-asyp, prp-pdw, cls-nothing2, cax-dw | "throws exception" (`README.md:221,223,229,243,268`) | Nothing is thrown; they record diagnostics (§3) |
| rdfs-datatype | "yes" (`README.md:193`); docs say "partial" (`docs/owl-rules/index.rst:39-44`) and also "Datatype semantics are not currently implemented" (`:271-274`) | Diagnostics only, for 5 datatypes |
| rdfs12 | "container membership …" (`README.md:192`) | Predicate-triggered axioms, not W3C rdfs12 (§1.3) |
| Rust example | `use ::reasonable::owl::Reasoner;` (`README.md:135`) | No `owl` module exists; the module is `reasoner` (`lib/src/lib.rs:20-26`) |
| CLI help block | Shows only `-o` (`README.md:28-40`) | Five options (§5.2) |
| Inputs | `docs/cli/index.rst:26` says "Turtle or N3"; the deb description says "Turtle, RDF/XML, and N-Triples" (`cli/Cargo.toml:34-39`) | `.ttl` → Turtle, `.n3` → **N-Triples parser**, anything else → error (`lib/src/reasoner.rs:1248-1264`) |
| Diagnostic list | 7 codes (`README.md:63-70`) | Also `OWLRL.CLS_COM` (never emitted) and `OWLRL.UNKNOWN` (fallback) (`lib/src/reasoner.rs:57-67`) |
| Python version | Docs say 3.9+ (`docs/python-api/index.rst:6`) | `requires-python = ">=3.8"` (`python/pyproject.toml:13`); abi3 floor is py39 (`python/Cargo.toml:28`) |

---

## 2. Data model, input and output

### 2.1 Interning

- `URI = u32`, `KeyedTriple = (URI, (URI, URI))` (`lib/src/common.rs:11-12`).
- `URIIndex` is a bijection `HashMap<oxrdf::Term, u32>` plus `Vec<Term>`.
  Ids are dense and assigned in first-seen order. Id 0 is reserved for
  `urn:_` (`lib/src/index.rs:6-40`). IRIs, blank nodes and literals share
  one id space.
- The vocabulary is interned first, in a fixed order, in `Reasoner::new`
  (`lib/src/reasoner.rs:432-461`, plus `owl:propertyChainAxiom` at `:514`).
- Literal identity is `oxrdf::Literal` equality: lexical form plus datatype
  or language. `"1"^^xsd:integer` and `"01"^^xsd:integer` get different ids.
- A blank node's identity is its label. Labels are global across files
  (§2.2).

### 2.2 Parsing

- `load_file(path)`: `.ttl` goes to `rio_turtle::TurtleParser` with base
  `None`; `.n3` goes to `rio_turtle::NTriplesParser`. Any other extension
  returns `io::Error("no parser for file (only ttl and n3)")`. The extension
  check is case-sensitive `ends_with` (`lib/src/reasoner.rs:1245-1264`).
  Triples go through an `oxrdf::Graph`, so duplicates in a file are removed
  (`:1247-1275`). There is no N3, RDF/XML, N-Quads, TriG or stdin input.
  The sample `.n3` files are N-Triples (`example_models/small1.n3`,
  `example_models/ontologies/Brick.n3`), and
  `scripts/convert_to_n3.py:11-12` writes N-Triples under a `.n3` name.
- RDF-star terms panic with `"no rdf*"` (`lib/src/common.rs:103,127`). Under
  feature `rdf-12`, `oxrdf_to_rio` also panics on a quoted triple
  (`:177-180`).
- **Blank-node collisions across files (from code).** Each `load_file`
  call starts a new rio parser. Rio labels anonymous nodes `riog` plus a
  counter that starts at 1 for each parser (`rio_turtle` 0.8.6
  `src/utils.rs:252-266`; the workspace requires `^0.7.1`, which was not
  inspected). Explicit labels such as `_:b0` pass through unchanged
  (`lib/src/common.rs:100-101`). Two files that both use `[]` or `_:b0`
  therefore share blank nodes inside `reasonable`.
- `load_triples_str` accepts IRIs only, through `NamedNode::new`. A triple
  with an invalid IRI is dropped silently (`lib/src/reasoner.rs:834-844`).

### 2.3 Output content

- `reason()` stores the **whole closure**: base triples, seed triples and
  inferred triples. There is no inferred-only view. The closure is read from
  `spo.stable`, triples with a 0 component are dropped, and the rest are
  sorted by `(s,p,o)` id (`lib/src/reasoner.rs:2221-2234`).
- Each id is turned back into a `Term`, then into an `oxrdf::Triple` through
  `make_triple`. A triple with a literal (or blank-node) predicate or a
  literal subject fails; the failure is logged at `error!` level and the
  triple is skipped (`:2247-2264`, `lib/src/common.rs:74-93`). Examples of
  skipped triples: `"lit" rdf:type C` from prp-rng, and `"a" owl:sameAs "b"`
  from prp-fp on literal values.
- Even on empty input the output is not empty **(from code)**. It contains
  `owl:Thing a owl:Class`, `owl:Nothing a owl:Class`, `owl:Thing a owl:Thing`
  and `owl:Nothing a owl:Thing` (seeds at `:569-574`, plus the Thing rule at
  `:1344-1348`). The test checks the first two (`lib/src/tests.rs:978-990`).

### 2.4 Serialisation (`dump_file`)

- `dump_file(path)` writes `view_output()` with `rio_turtle::TurtleFormatter`
  (`lib/src/reasoner.rs:1196-1236`). The output has no prefixes and uses full
  IRIs. Consecutive triples with the same subject are grouped with `;` and
  `,` (rio 0.8.6 `src/formatters.rs:142-167`). The order is id order, which
  depends on interning order.
- **Language tags are lost (from code).** `oxrdf_to_rio` writes any literal
  whose datatype is not `xsd:string` as `Typed`
  (`lib/src/common.rs:147-153`). A language-tagged literal has datatype
  `rdf:langString`, so it is written as `"v"^^<…#langString>` and the tag is
  dropped. The `LanguageTaggedString` branch (`:155-160`) is never reached.

---

## 3. Diagnostics

### 3.1 Structure

`ReasoningError { code: String, rule: String, severity: DiagnosticSeverity,
message: String }`, with getters `code()`, `rule()`, `severity()` and
`message()` (`lib/src/reasoner.rs:43-89`). Severity is always `Error`
(`:72`). `Display` prints `"{severity}[{rule}]: {message}"`, for example
`error[cax-dw]: …` (`:91-99`). No subject, triple or provenance field
exists (TODO at `:52`).

`add_error` (`:977-995`) does the following in order:
1. Returns if `options.collect_diagnostics` is false.
2. Builds the error.
3. With `options.dedupe` (default true), drops it when `(code, message)` is
   already in `seen_diags`.
4. Drops it when `errors.len() >= options.max_diagnostics`.
5. Pushes it.

Defaults are `collect_diagnostics = true`, `max_diagnostics = None` and
`dedupe = true` (`:111-119`). Diagnostics build up across incremental
`reason()` calls and are reset only by `clear()` (`:726-727`).

### 3.2 Codes

Terms in messages use oxrdf `Display`: `<iri>`, `_:id`, `"lit"^^<dt>`.

| Code | Rule | Fires when | Message format | Site |
|---|---|---|---|---|
| `OWLRL.CAX_DW` | `cax-dw` | `?i a C1`, `?i a C2`, and `C1 owl:disjointWith C2` asserted or derived; only the stated direction is checked | `inst {i} is both {C1} and {C2} (disjoint classes)` | `lib/src/reasoner.rs:1784-1810` |
| `OWLRL.PRP_PDW` | `prp-pdw` | `p1 owl:propertyDisjointWith p2`, `x p1 y`, `x p2 y` | `property {p1} is disjoint with {p2} for pair ({x} {p1} {y})` | `:1812-1833`, `:1851-1861` |
| `OWLRL.CLS_NOTHING` | `cls-nothing2` | `?x a owl:Nothing` | `instance {x} is owl:Nothing (unsatisfiable)` | `:1905-1924` |
| `OWLRL.PRP_ASYP` | `prp-asyp` | `p a owl:AsymmetricProperty`, `x p y`, `y p x` (one diagnostic per ordered pair) | `property {p} of {x} and {y} is asymmetric` | `:1557-1573`, `:1842-1850` |
| `OWLRL.PRP_IRP` | `prp-irp` | `p a owl:IrreflexiveProperty`, `x p x` | `property {p} of {x} is irreflexive` | `:1545-1555`, `:1834-1841` |
| `OWLRL.CLS_COM` | `cls-com` | **Never.** The call is commented out. | — | `:63`, `:2043-2051` |
| `RDFS.DATATYPE` | `rdfs-datatype` | An object literal is ill-formed for its datatype (table below) | `ill-formed literal {lit} used in triple {s} {p} {o}` | `:1131-1154` |
| `RDFS.DATATYPE_RANGE` | `rdfs-datatype-range` | `p rdfs:range D`, `s p o`, D is a recognised datatype, and o is not in D. A **non-literal** object also clashes (`:2404-2406`). | `range clash: object {o} is not in datatype {D} for predicate {p}` | `:1156-1175` |
| `OWLRL.UNKNOWN` | any other rule string | Fallback only; never produced | — | `:66` |

Datatype checks (`lib/src/reasoner.rs:2337-2426`). Any other datatype IRI
returns `None` and is not checked.

| Datatype | Well-formed lexical form (`RDFS.DATATYPE`) | Value-space test for range D (`RDFS.DATATYPE_RANGE`) |
|---|---|---|
| `xsd:string` | always well-formed | Literal with no language tag and datatype `xsd:string` |
| `xsd:integer` | optional `+`/`-`, then one or more ASCII digits; no whitespace | Literal datatype is `xsd:integer` or `xsd:int` and well-formed. `xsd:long`, `xsd:nonNegativeInteger` and other subtypes clash. |
| `xsd:int` | integer form that also parses as `i32` | Literal datatype is `xsd:int` and well-formed |
| `rdf:langString` | has a language tag | Has a language tag |
| `rdf:XMLLiteral` | quick-xml reads to EOF with `check_end_names` (plain text passes) | Datatype is `rdf:XMLLiteral` and well-formed |

The datatype scan runs once, after the fixpoint, over the closure
(`lib/src/reasoner.rs:2235-2246`, `:1060-1129`). Inferred triples are
included: a literal that prp-spo1 copies to a property with a range is
checked. Only objects are checked.

### 3.3 CLI reporting (`cli/src/main.rs`)

- Diagnostics are printed to **stdout** after the output file is written
  (`:60-61`, `:63-116`). Logs go to stderr through `env_logger` at default
  level `info` (`:39`).
- `--error-format text` prints `{code} {rule}: {message}` per line (`:69-77`).
- `--error-format json` prints one array
  `[{"code":…,"rule":…,"severity":…,"message":…},…]` (`:78-100`).
  `ndjson` prints one such object per line (`:101-115`). Escaping handles
  only `\` and `"` (`:148-150`), so a message containing a newline (for
  example from a multi-line literal) produces invalid JSON **(from code)**.
- `--max-diagnostics N` caps how many are **printed**. Collection is not
  capped, because the CLI never calls `set_options` (`:64-66`).
- `--summary-only` prints only `diagnostics: {total}`, where total is
  uncapped (`:120-123`).
- `--fail-on X` (repeatable, comma-separated) makes the process exit with
  code 2 when any diagnostic's code or rule equals an entry,
  case-insensitively after trimming (`:124-146`). `OWLRL.CAX_DW` and
  `cax-dw` match; `CAX_DW` does not. Unknown entries are ignored silently.

---

## 4. Incremental reasoning

State kept between runs (`lib/src/reasoner.rs:274-422`):

- `base`: asserted triples plus seeds plus container axioms.
- `input`: the sorted closure after the last run.
- `is_materialized` and `pending_delta`.
- Every datafrog `Variable`, including the full `spo`/`pso`/`osp` contents.
- Side maps `instances`, `intersections`, `unions` and `complements`, plus
  `established_complementary_instances`, which is never read.
- `errors` and `seen_diags`, and the `index`. The index is never shrunk,
  even by `clear()`.

**Additions (`load_file`, `load_triples`, `load_triples_str` after the first
`reason()`).** The new triples are sorted and deduplicated against `input`
(the closure) with `get_unique` (`:2433-2454`), then appended to
`pending_delta` (`:846-849`, `:880-883`, `:1279-1281`). They are not added
to `base` until `reason()` runs. A triple that was already inferred never
reaches `base`, so `get_input()` and `get_base_triples()` omit it.

**`reason()` in incremental mode** (`:1311-1324`):
1. Inject container axioms, scanning `pending_delta` only (`:1008-1012`).
2. If `pending_delta` is empty, return. Output and diagnostics stay as they
   were (`:1313-1315`).
3. Drain the delta, append it to `base`, and extend `all_triples_input` with
   it. Datafrog then continues semi-naive evaluation from the existing
   stable relations.
4. Rebuild `DisjointSets` from `input + delta`.
5. Run the same fixpoint loop.
6. Compute the datatype-scan delta as `output \ input` (`:2239-2245`). Pass 2
   re-checks older triples only against newly declared `rdfs:range`
   (`:1098-1124`).

cls-int1 is not semi-naive. In every round it rescans every intersection
and every instance (`:1932-1991`).

**First `reason()`** runs a full materialisation and sets `is_materialized`
(`:1325-1331`). Before that, loads go straight into `base` and `input`
(`:850-855`, `:818-821`).

**Deletions.** There is no retraction API. `set_base_triples(triples)`
(`:902-975`) does the following:
1. Interns the new set, adds the two seeds, then sorts and deduplicates it.
2. Sorts the old `base` and diffs the two with a linear merge.
3. **Any removal** sets `base` to the new set and calls `clear()`, which
   throws away all derived state, diagnostics and side maps and rebuilds the
   `Iteration` and every `Variable` (`:721-807`). It returns `true`. The
   next `reason()` is a full run.
4. **Additions only**: replaces `base`, deduplicates the additions against
   `input` into `pending_delta` (or sets `input = base` before the first
   run), and returns `false`. No change returns `false`.

`reason_full()` is `clear()` followed by `reason()` (`:813-816`). The tests
check that incremental results equal a full run for most rule families
(`lib/src/tests.rs:2108-2324`), including `set_base_triples`
(`:2576-2697`).

**Spurious "removals" (from code).** `set_base_triples` compares against a
`base` that it does not deduplicate. `base` gets duplicates in two ways:

- An additions-only `set_base_triples` puts the added triples in `base`
  (`:965`), and the next `reason()` appends the same triples again
  (`:1318`).
- `load_triples` and `load_triples_str` sort without `dedup` before
  `add_base_triples` (`:845-854`).

Container axioms are also in `base` but never in the caller's graph
(`:1041`, `:1044`). In any of these cases the next `set_base_triples` sees
a "removal" and forces a full rerun. That happens even when the graph has
not changed: for example `update_graph(g)` followed by `reason()` and then
`update_graph(g)` again. The results are still correct; only the speed is
affected. No test covers a second `set_base_triples` after a `reason()`.

---

## 5. Public interfaces

### 5.1 Rust (crate `reasonable`; public modules `common`, `error`, `reasoner` at `lib/src/lib.rs:18-27`)

| Item | Signature | Site |
|---|---|---|
| `Reasoner::new` | `fn new() -> Self` (no `Default` impl) | `lib/src/reasoner.rs:427` |
| `load_file` | `fn load_file(&mut self, filename: &str) -> reasonable::error::Result<()>` | `:1245` |
| `load_triples` | `fn load_triples(&mut self, triples: Vec<oxrdf::Triple>)` | `:871` |
| `load_triples_str` | `fn load_triples_str(&mut self, Vec<(&'static str, &'static str, &'static str)>)` | `:834` |
| `set_base_triples` | `fn set_base_triples(&mut self, Vec<Triple>) -> bool` (true means removals, so a full rerun follows) | `:902` |
| `reason` | `fn reason(&mut self)` | `:1295` |
| `reason_full` | `fn reason_full(&mut self)` | `:813` |
| `clear` | `fn clear(&mut self)` | `:721` |
| `get_triples` | `fn get_triples(&self) -> Vec<Triple>` (full closure, cloned) | `:2278` |
| `view_output` | `fn view_output(&self) -> &[Triple]` | `:2283` |
| `get_triples_string` | `fn get_triples_string(&mut self) -> Vec<(String,String,String)>` (oxrdf `Display` strings, such as `<urn:a>`) | `:2313` |
| `get_input` | `fn get_input(&self) -> Vec<Triple>` (base, including seeds and container axioms) | `:2287` |
| `dump_file` | `fn dump_file(&mut self, filename: &str) -> Result<()>` | `:1196` |
| `errors` / `diagnostics` | `fn errors(&self) -> &[ReasoningError]` (aliases) | `:1179`, `:1184` |
| `set_options` | `fn set_options(&mut self, ReasonerOptions)` | `:1189` |
| `ReasonerOptions` | `{ collect_diagnostics: bool, max_diagnostics: Option<usize>, dedupe: bool }`, `Clone + Default` | `:101-119` |
| `ReasonerBuilder` | `new`, `with_file(impl Into<String>)`, `with_triples(Vec<Triple>)`, `with_triples_str(...)`, `build() -> Result<Reasoner>` (files, then str triples, then `Triple`s) | `:121-167` |
| `ReasoningError`, `DiagnosticSeverity` | §3.1 | `:24-99` |
| `get_unique` | `pub fn get_unique(src: &[KeyedTriple], rv: &mut Vec<KeyedTriple>)` | `:2433` |
| `common` | `URI`, `KeyedTriple`; macros `uri!`, `owl!`, `rdf!`, `rdfs!`, `node_relation!`; `make_triple`, `rio_to_oxrdf`, `oxrdf_to_rio` | `lib/src/common.rs:11-187` |
| `error::ReasonableError` | `ManagerError(String)`, `IO`, `IriParse`, `BNodeParse`, `Turtle(rio_turtle::TurtleError)`, `ChannelRecv`; `error::Result<T>` | `lib/src/error.rs:3-19` |

`Reasoner` holds `Rc<RefCell<…>>` fields, so it is `!Send` and `!Sync`
(`lib/src/reasoner.rs:349`, `:417-421`). All work runs on one thread; issue
#57 proposes parallelising it.

### 5.2 CLI (`reasonable`, `cli/src/main.rs`)

| Flag | Default | Meaning | Site |
|---|---|---|---|
| `<INPUT_FILES>...` | required | `.ttl` or `.n3` (N-Triples) files, loaded in order into one graph | `:20-21`, `:45-52` |
| `-o, --output-file` | `output.ttl` | Writes the whole closure as Turtle (§2.4) | `:22-23` |
| `--error-format` | `text` | `text`, `json` or `ndjson` | `:24-26` |
| `--fail-on` | none | Repeatable, comma-separated | `:27-29` |
| `--max-diagnostics` | unlimited | Print cap | `:30-32` |
| `--summary-only` | off | Prints `diagnostics: N` | `:33-35` |
| `-h`, `-V` | — | clap built-ins (`#[command(author, version, about)]`) | `:18` |

Exit codes:
- **0**: success.
- **2**: a `--fail-on` match (`:133-135`). clap usage errors also exit 2,
  which is clap's default.
- **101**: panic. `load_file` and `dump_file` failures are `unwrap()`ed
  (`:50`, `:61`), so a missing file, a parse error or an unsupported
  extension panics.

The CLI cannot write to stdout, emit inferred triples only, or choose an
output format.

### 5.3 Python (`python/src/pyreason.rs`, module `reasonable`)

The module exports `PyReasoner`, `__version__` (the Cargo version) and
`__package__ = "reasonable"` (`python/src/lib.rs:9-14`,
`reasonable/__init__.py:4-6`). `PyReasoner` is `#[pyclass(unsendable)]`
(`:157`).

| Method | Behaviour | Site |
|---|---|---|
| `PyReasoner()` | Creates a new `Reasoner` | `:164-170` |
| `load_file(file: str) -> None` | Raises `OSError` (`PyIOError`) on any error | `:173-178` |
| `from_graph(graph) -> None` | Calls `list(graph)` and reads items 0–2 of each tuple, then calls `load_triples`, which **appends** | `:195-201`, `:130-155` |
| `update_graph(graph) -> bool` | `set_base_triples`; returns True when removals were found | `:211-217` |
| `reason() -> list[tuple]` | Whole closure as rdflib terms | `:183-188` |
| `reason_full() -> list[tuple]` | `clear()` then `reason()` | `:228-233` |
| `clear() -> None` | — | `:221-223` |
| `get_base_triples() -> list[tuple]` | `get_input()` | `:236-240` |

No diagnostics are exposed to Python; issue #31 asks for this. The binding
has no `dump_file`, `set_options` or triples-only loader.

rdflib interop (`:21-111`):

- Input dispatch is on the Python type name: `URIRef`, `Literal`, `BNode`.
  Any other type is treated as an IRI.
- A `Literal` with a datatype becomes typed, one with a language becomes
  lang-tagged, and anything else becomes simple.
- Invalid IRIs, language tags or blank-node ids `unwrap()` and panic.
- A literal subject or a non-IRI predicate raises `Exception`
  (`make_triple`).
- Output: every non-lang literal is created as
  `Literal(value, None, datatype)`, including `xsd:string`. A plain
  `Literal("x")` going in comes back as `Literal("x", datatype=XSD.string)`,
  which rdflib does not treat as equal **(from code)**.

### 5.4 Cargo features

| Crate | Features | Site |
|---|---|---|
| `reasonable` | `default = ["prp-spo2"]`; `prp-spo2` (property chains); `rdf-12` (turns on `oxrdf/rdf-12` so the crate compiles; quoted triples panic) | `lib/Cargo.toml:22-34` |
| `reasonable-cli` | `default = ["prp-spo2"]`, forwarded to the library | `cli/Cargo.toml:17-21` |
| `pyreasonable` | `default = ["prp-spo2"]`; `abi3` (`pyo3/abi3-py39`) | `python/Cargo.toml:26-32` |

CI also runs `cargo test -p reasonable --no-default-features`
(`.github/workflows/ci.yml:44`).

---

## 6. OWL constructs: support summary

| Construct | Support |
|---|---|
| `owl:sameAs` | eq-sym, eq-trans and eq-rep-s/p/o. Fully materialised, with no canonical representative: n equal nodes produce n² sameAs triples and every triple is copied for each alias. No eq-ref, and no eq-diff (`owl:differentFrom` is ignored). |
| `rdf:first` / `rdf:rest` | Read through `DisjointSets` for intersection and union (unordered; shared tails merge, §1.4), and through datalog for property chains |
| `owl:intersectionOf` | cls-int1 (one list per class) and cls-int2. No scm-int. |
| `owl:unionOf` | cls-uni (one list per class). No scm-uni. |
| `owl:complementOf` | No inference and no diagnostic (§1.3) |
| `owl:hasValue` | cls-hv1, cls-hv2 |
| `owl:someValuesFrom` | cls-svf1; cls-svf2 only when the filler is the IRI `owl:Thing` |
| `owl:allValuesFrom` | cls-avf |
| `owl:maxCardinality`, `owl:maxQualifiedCardinality` | Not supported |
| `owl:propertyChainAxiom` | Supported, n ≥ 1 hops, behind feature `prp-spo2` |
| `owl:hasKey` | Not supported |
| `owl:oneOf`, `owl:AllDisjointClasses`, `owl:AllDisjointProperties`, `owl:NegativePropertyAssertion`, `owl:AllDifferent` | Not supported |
| `owl:inverseOf`, `owl:SymmetricProperty`, `owl:TransitiveProperty`, `owl:equivalentProperty`, `rdfs:subPropertyOf` | Supported for instance data; no scm-* schema closure |
| `owl:FunctionalProperty` | Supported |
| `owl:InverseFunctionalProperty` | Buggy (#64) |
| `owl:disjointWith`, `owl:propertyDisjointWith`, `owl:AsymmetricProperty`, `owl:IrreflexiveProperty`, `owl:Nothing` | Diagnostics |

---

## 7. Tests and benchmarks

### 7.1 Tests

- `lib/src/tests.rs`: 112 `#[test]` items. They cover each implemented rule
  with positive asserts: eq-* (`:82-167`), cax/scm (`:169-316`,
  `:1384-1563`), prp-* (`:318-976`), cls-* (`:978-1327`) and
  container/datatype diagnostics (`:1592-2034`). Incremental and full-run
  equivalence is covered at `:2105-2558` and `set_base_triples` at
  `:2560-2697`. Negative asserts are rare (for example
  `test_prp_fp_2`, `:341`). `test_error_asymmetric` asserts nothing
  (`:1328-1344`), and `test_cax_dw` accepts any error (`:1381`).
- W3C cases are hand-ported, not run from a manifest:
  - `New-Feature-ObjectPropertyChain-001`, `-BJP-003` and `-BJP-004`
    (`lib/src/tests.rs:444-529`).
  - rdf-mt `rdfms-seq-representation-test002/004` (`:1592-1628`).
  - There is no W3C OWL 2 test-suite runner.
- `cli/tests/cli_integration.rs`: 5 tests. They cover `--help`, writing the
  output file, exit code 2 on `--fail-on cax-dw` and on `prp-irp` with
  `ndjson`, and the JSON array shape (`:26-150`).
- `python/tests/test_pyreasoner.py`: 45 tests (one parametrised over
  n = 1..6, `:264`). They port the Rust rule tests and add `update_graph`,
  `clear`, `reason_full` and `get_base_triples` tests (`:512-634`).
- Fixtures:
  - `example_models/ontologies/Brick.n3`: Brick 1.1, 14,804 N-Triples lines.
  - `owl.n3`: 451 lines. `rdfs.ttl`: 109 lines.
  - `example_models/small1.n3`: 14 lines. `soda_hall.n3`: 3,775 lines.
  - `cli/tests/minimal-bfo-alice.ttl`.

### 7.2 Benchmarks

- `benches/my_benchmark.rs` (criterion):
  - `ontology_small1`: Brick plus small1, with owl.n3 and/or rdfs.ttl
    combinations (`:26-84`).
  - `reload_test`: Brick then small1 or soda_hall (`:86-122`).
  - `incremental_reason`: `brick_add_1_triple`,
    `brick_full_rematerialization` and `brick_then_small1` (`:124-190`).
- `benches/property_chain.rs`: unrelated lists, shared suffix, branching
  and incremental final-hop scenarios (`:169-188`).
- `scripts/bench_examples.sh` times the CLI against `scripts/reason_owlrl.py`
  (OWL-RL `DeductiveClosure`) over `example_models` and writes `bench.txt`.
- `benches/python/bench.py` compares owlrl, Allegro and `reasonable` (the
  owlready2 section is commented out) over `benches/python/buildings/*.ttl`
  (Brick v1.1 building models), with Brick, owl and rdfs loaded.
- Published figures:
  - `reasonable` is "about 7x faster than Allegro and 38x faster than
    OWLRL" on Brick models (`README.md:15`, `img/benchmark.png`,
    `docs/index.rst:46-47`).
  - The README log shows `Brick.n3 + small1.n3` taking 18.66 s in a 2023
    run (`README.md:45-53`).
  - Issue #57 reports LUBM-500 (69.1 M triples) taking 2,578 s to
    materialise on one core.

---

## 8. Known gaps and bugs

Open issues (GitHub API, read 2026-11-05):

- **#64** prp-ifp derives sameAs when objects differ. **#65** is the open
  fix PR.
- **#57** The fixpoint is single-threaded, and its cost grows faster than
  linearly at LUBM scale.
- **#54** Requests pyoxigraph interop. Today the bridge goes through rdflib
  tuples.
- **#31** Asks for inconsistency reporting. In Python, diagnostics are
  still not exposed.
- **#26** Results differ from GraphDB OWL-RL; the maintainer points to the
  unimplemented rules.
- **#23** Property chains. Now implemented, but the issue is still open.
- **#12** Requests wasm (§9).
- **#11** Asks for user rules. Not supported.
- **#17** An old broken pip release.

TODOs in code and docs:

- No provenance on diagnostics (`lib/src/reasoner.rs:52`).
- complementOf inference (`:2033`).
- "Implement remaining RDF/RDFS entailment" (`README.md:185`).
- `docs/ARCHITECTURE_REVIEW.md` lists sentinel `(0,0,0)` triples, `Rc`/
  `RefCell` state, `unwrap()` panics and the lack of a per-run runtime
  (`:84-128`). Parts of it describe older code: farmhash ids and
  `query.rs`.

Found by reading the code (§1–§5):

- prp-ifp bug.
- cls-com does nothing.
- scm-eqc2 missing.
- Language tags dropped by `dump_file`.
- Blank-node label collisions across files.
- Shared list tails merge in `DisjointSets`.
- Only one intersection or union list is used per class.
- Spurious full reruns from `set_base_triples`.
- Invalid JSON when a message contains a newline.
- `xsd:string`-typed literals in Python output.
- Panics on load errors in the CLI and on bad terms in Python.

---

## 9. Dependencies, MSRV and wasm

No `Cargo.lock` is committed; `.gitignore` lists it. Versions therefore
float within the semver ranges below.

| Crate | Requirement | Latest on crates.io (2026-11-05) | Site |
|---|---|---|---|
| `rio_turtle`, `rio_api` | `0.7.1` (^0.7) | 0.8.6 | `Cargo.toml:22-23` |
| `oxrdf` | `0.3.3` | 0.3.5 | `Cargo.toml:24` |
| `datafrog` | `2.0.1` | 2.0.1 | `Cargo.toml:25` |
| `disjoint-sets` | `0.4.2` (2018) | 0.4.2 | `Cargo.toml:26` |
| `log` / `env_logger` | `0.4` / `0.11` | — | `Cargo.toml:27-28` |
| `thiserror` | `1.0` | — | `lib/Cargo.toml:19` |
| `quick-xml` | `0.42` | 0.42.0 | `lib/Cargo.toml:20` |
| `clap` | `4.3.10` + derive | — | `Cargo.toml:30` |
| `pyo3`, `pyo3-ffi`, `pyo3-build-config` | `0.25` | — | `python/Cargo.toml:20-24` |
| `criterion` (dev) | `0.5` | — | `lib/Cargo.toml:37` |

- **MSRV** is not declared: there is no `rust-version` and no
  `rust-toolchain.toml`, and CI uses `stable`
  (`.github/actions/setup-rust/action.yml:4-7`, `ci.yml:30-32`). The code
  uses `let … else` (Rust 1.65) and `Option::is_some_and` (1.70)
  (`lib/src/reasoner.rs:1020`, `:2334`). The dependency floors were not
  checked. Edition 2021 (`Cargo.toml:18`).
- **wasm32-unknown-unknown.** The library uses no threads and no `Instant`.
  `std::fs` appears only in `load_file` and `dump_file`
  (`lib/src/reasoner.rs:1198`, `:1246`), which compile but fail at runtime
  on wasm. `oxrdf` 0.3 depends on `rand` (`>=0.8,<0.10`), which pulls in
  `getrandom`; on wasm32-unknown-unknown, `getrandom` builds only when a JS
  backend is turned on (the `js` feature, or the `wasm_js` backend for
  0.3). This is the one known build blocker **(not built)**. The CLI uses
  `std::time::Instant` (`cli/src/main.rs:8,53`), which panics on that
  target. A 2022 `wasm` branch exists; the maintainer said in #12 that he
  got it compiling there, with a "Context" class.

---

## Parity checklist

Each item is a test to run against a built `reasonable` at `fecee2b` with
default features. Compare closures as sets of `(s,p,o)` terms, with blank
nodes compared by label.

1. Given `Brick.n3 + small1.n3` (and each of the `+owl.n3`, `+rdfs.ttl` and
   `+owl.n3+rdfs.ttl` variants from `benches/my_benchmark.rs:26-84`), the
   output closure equals `reasonable`'s `get_triples()`.
2. Given `Brick.n3 + soda_hall.n3`, and each `benches/python/buildings/*.ttl`
   with Brick, owl and rdfs loaded, the closures are equal.
3. Empty input gives exactly `{owl:Thing a owl:Class, owl:Nothing a
   owl:Class, owl:Thing a owl:Thing, owl:Nothing a owl:Thing}`.
4. Every node that is the subject of an output triple has `a owl:Thing`. A
   node that occurs only as an object does not, unless another rule types
   it.
5. The output is the whole closure (base, seeds and inferred), with
   literal-subject triples left out.
6. Each implemented rule in §1.3 (eq-sym, eq-trans, eq-rep-s/p/o, prp-dom,
   prp-rng, prp-fp, prp-symp, prp-trp, prp-spo1, prp-spo2, prp-eqp1/2,
   prp-inv1/2, cls-int1/2, cls-uni, cls-svf1/2, cls-avf, cls-hv1/2,
   cax-sco, cax-eqc1/2, scm-sco, scm-eqc1) reproduces the corresponding
   `lib/src/tests.rs` case, and the 45 Python cases give the same closures.
7. None of the not-implemented rules (eq-ref, eq-diff*, prp-ap, prp-adp,
   prp-key, prp-npa*, cls-maxc*, cls-maxqc*, cls-oo, cax-adc, dt-*, scm-*
   other than sco and eqc1, rdfs4a/4b/5/6/8/10/13) adds triples. For
   example, `p1 subPropertyOf p2 . p2 subPropertyOf p3` yields no
   `p1 subPropertyOf p3`, and `A sco B . B sco A` yields no
   `A equivalentClass B`.
8. x sameAs y yields `x sameAs x`, `y sameAs y` and `y sameAs x`. A
   functional property gives `o sameAs o` for each object.
9. Property chains of length 1 to n compose in order. BJP-004 does not
   produce transitivity. Unreferenced lists do not compose. Cyclic lists
   terminate (`lib/src/tests.rs:444-866`).
10. Container axioms: any predicate `rdf:_N` with N > 0 adds the three
    axioms, and `x rdf:_N y` adds `x rdfs:member y`. `rdf:_0` and
    `rdf:_a` add nothing (`lib/src/tests.rs:1592-1745`).
11. Diagnostic `OWLRL.CAX_DW` fires on `A owl:disjointWith B . i a A, B`
    (`cli/tests/cli_integration.rs:68-71`). It fires once, with the message
    `inst <i> is both <A> and <B> (disjoint classes)`.
12. `OWLRL.PRP_PDW`, `OWLRL.PRP_ASYP` (two per symmetric pair),
    `OWLRL.PRP_IRP` and `OWLRL.CLS_NOTHING` fire on their minimal inputs
    with the exact messages in §3.2.
13. `OWLRL.CLS_COM` never fires, even for `c1 complementOf c2 . x a c1, c2`.
14. `RDFS.DATATYPE` fires for `" 42"^^xsd:int`, `"2147483648"^^xsd:int`,
    `"abc"^^xsd:integer` and mismatched XMLLiteral tags. It does not fire
    for `"+5"^^xsd:integer`, `"-7"^^xsd:int` or unknown datatypes
    (`lib/src/tests.rs:1747-1890`).
15. `RDFS.DATATYPE_RANGE` fires for a string or lang-string under an
    integer or string range, for an IRI object under a recognised datatype
    range, and retroactively when the range is added in a later batch. It
    does not fire for `xsd:int` under an `xsd:integer` range or for
    unrecognised ranges (`lib/src/tests.rs:1892-2034`).
16. Diagnostics are deduplicated on (code, message) and carry severity
    `error`. They persist across incremental runs and reset on
    `clear`/removal.
17. CLI: the text line format, the JSON array and the NDJSON object (keys
    in the order `code`, `rule`, `severity`, `message`); `--max-diagnostics`
    caps printing only; `--summary-only` prints `diagnostics: N`;
    `--fail-on` matches a code or rule case-insensitively and exits 2; the
    default output path is `output.ttl`; logs go to stderr and diagnostics
    to stdout.
18. Input: `.ttl` is Turtle and `.n3` is N-Triples. Other extensions are
    rejected. Duplicate triples in a file collapse.
19. Incremental: `load_*`, then `reason()`, then `load_*`, then `reason()`
    gives the same closure as one full run, for each family in
    `lib/src/tests.rs:2137-2390`. `reason()` with no delta is a no-op.
20. `set_base_triples` / `update_graph` returns false for additions only or
    no change after construction, and true when any base triple is gone. A
    retracted support removes its inferences after the next `reason()`.
21. Python: `PyReasoner` methods and return types as in §5.3;
    `load_file` raises `OSError`; terms come back as rdflib
    `URIRef`/`BNode`/`Literal`.
22. Building without `prp-spo2` derives no chain triples and leaves
    everything else unchanged.

## Where parity is not desirable

Match these only behind a compatibility flag, if at all.

1. **prp-ifp over-inference** (#64). Use the W3C rule (`y1 == y2`). An
   oracle diff on any input with an `owl:InverseFunctionalProperty` will
   differ; filter those sameAs triples and their eq-rep copies, or apply
   PR #65 to the oracle.
2. **cls-com silence.** Emitting `OWLRL.CLS_COM` is the documented intent
   (`docs/owl-rules/index.rst:188-190`).
3. **scm-eqc2 and the other scm-* rules missing.** Full OWL 2 RL needs
   them; they add triples that `reasonable` lacks.
4. **sameAs as a full quadratic closure.** Clique-size² sameAs triples, a
   copy of every triple for each alias, and `x sameAs x` only for nodes
   with a sameAs edge. A union-find representative scheme is preferable;
   then compare after expansion.
5. **prp-fp/prp-ifp joins keyed on p alone.** Their cost is quadratic per
   property. Join on (p, x) or (p, y) instead.
6. **Deletion resets everything.** Any removal (or a spurious one, §4)
   discards all state. Use DRed, counting, or at least deduplicate `base`
   so an unchanged graph does not trigger a full rerun.
7. **`owl:Thing` typing of every subject.** This is not RL. It types list
   cells, restrictions and properties as `owl:Thing` and inflates output.
   Keep it only as an opt-in compatibility mode if byte-level diffs
   matter.
8. **`DisjointSets` lists.** Shared tails merge, order is lost, and only one
   intersection or union list is used per class. Read lists structurally.
9. **Dropping language tags in `dump_file`**, and Python returning
   `xsd:string`-typed literals for plain ones.
10. **Blank-node labels shared across input files.** Scope labels per
    document.
11. **Datatype checks limited to 5 datatypes**, with `xsd:integer` subtypes
    other than `xsd:int` reported as range clashes. Use real XSD
    value-space subtyping.
12. **Literal-subject triples derived and then silently dropped**, with one
    `error!` log line each. Don't derive them, or keep them internal
    without logging.
13. **Panics on I/O, parse and term errors** (CLI `unwrap`, Python
    `unwrap`), and exit code 2 shared by clap usage errors and
    `--fail-on`.
14. **JSON escaping** that leaves newlines raw.
15. **Diagnostic messages without structured fields** (no subject or
    triple). Keep the four fields for compatibility, and add structured
    fields alongside them.
