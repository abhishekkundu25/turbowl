# Explanations after reasoning: how to say why a triple holds

Researched 2026-11-07. Repository state: branch `explain` at `1f50498`. The
question: how can TurbOWL answer "why is this triple inferred?" (and "why
is this a finding?") on demand, after a run, recording nothing during it,
so that reasoning speed and memory do not change at all? What do other
reasoners do, which algorithms give short, well-founded proofs without
stored derivation depths, and how should `owl:sameAs`, RDF lists and
inconsistencies appear in a proof?

Method. Source code was read at pinned commits: Nemo (`knowsys/nemo`
`7706c48`, 2026-11-04, 0.10.2-dev), Soufflé (`souffle-lang/souffle`
`2adf8c0`, 2026-10-31), GraphDB's proof plugin
(`Ontotext-AD/graphdb-proof-plugin` `9ce7cb4`), puli and ELK
(`liveontologies`, `master`). Docs read: RDFox 7.7, GraphDB 11.5, Stardog
"latest", Soufflé's provenance page, the Nemo docs (`knowsys/nemo-doc`
`1a69a04`). Papers were read in full text from the authors' PDFs or arXiv
(text extracted with `pypdf`). Load-bearing quotes were checked a second
time against the extracted text: Calautti et al. Proposition 28, RDFox's
note on equality, GraphDB's "90%" and `isSupported`, Stardog's "minimal
set", and Nemo's "adds no cost". `tools/nemo/nmo` is 0.10.1
(`tools/nemo/VERSION`); its `--help` lists the tracing flags that are on
`main`. Code in this repository is cited as `path:line` at `1f50498`.

Markers:

- **(analysis)**: my reasoning about this engine, not a claim from a
  source. Tests must confirm it.
- **(estimate)**: an expected cost I derived. It has not been measured.
- **(unverified)**: a claim I could not check against a primary source.

Nemo links are written `N/<path>#L<a>-L<b>`. They stand for
`https://github.com/knowsys/nemo/blob/7706c48f810249879cee1406b2c5882921584323/<path>#L<a>-L<b>`.

## What this means for TurbOWL

**The recommended design** (analysis). It is built from the sources below
and is not a published algorithm.

1. **The run does not change.** An explanation reads what a run already
   leaves:
   - the base, in the terms as written (`reasoner.rs:1399-1427`);
   - the store, in representatives;
   - the canon (`engine/canon.rs`), which knows the equality classes but
     not why they formed;
   - the findings.

   No field is added to `Store`, `Variable`, `Round` or a family. Speed
   and memory are unchanged by construction; one bench comparison
   confirms it (§0).
2. **A table of the W3C rules as data**, used by a backward matcher. For
   each rule the switches leave on, the table holds its body patterns, its
   head, its list atoms and its built-in conditions (the `dt-*` value
   checks). The matcher takes a bound head and lists every rule instance
   whose premises are all in the closure. Its join order puts the
   variables bound by the head first: Nemo's tracer became quadratic by
   keeping the forward order (§1.4), and the maintenance paper of Motik
   et al. shows bound-head queries touching n(n−1) useless facts (§2.1).
   FBF (ticket 23) needs the same primitive, so it is worth building once.
3. **Search: a relevance ball, then a forward pass in layers, with
   iterative deepening.**
   1. Collect every rule instance within backward distance d of the goal
      (the "ball").
   2. Over the ball, run a Horn pass forward from the base facts, layer by
      layer.
   3. If the goal is not reached, double d and repeat.

   This gives three guarantees:
   - The layer in which a fact first appears is its minimum proof depth
     (Calautti et al., Proposition 28; §3.5).
   - Each fact keeps one instance whose premises are in strictly earlier
     layers, so the proof is a DAG that is acyclic and has base leaves by
     construction.
   - The cost is linear in the ball's rule instances.

   Two limits apply, as RDFox's `explain` has: a maximum radius and a
   maximum number of instances per fact. A search the limits cut short
   still returns a sound, acyclic proof, but says that it may not be the
   shortest.
4. **Proofs are in the user's terms.** A proof uses the W3C rules over
   the terms the user wrote, with eq-sym, eq-trans and eq-rep-s/p/o as
   ordinary steps.
   - A membership test is three canon reads and a binary search.
   - A free variable enumerates the members of a class only when the
     class has two or more, so data without `owl:sameAs` pays nothing.
   - The rewriting never shows.

   No published system does this. RDFox cannot explain any fact when
   equality is on. Soufflé turns its union-find back into explicit rules
   (§4).
5. **Lists and inconsistencies.**
   - A list is one grouped premise, "the list (e1 … en) at h", whose
     2n `rdf:first`/`rdf:rest` triples fold away (§5).
   - A finding is explained as a rule instance with head `false` and the
     finding's terms bound (§6).
6. **Output.**
   - The model is a DAG: a list of nodes, each a triple with how it holds
     (asserted, an axiom, or a rule with premise ids and its depth). It is
     the shape RDFox's JSON and Nemo's JSON already use.
   - JSON for tools.
   - A text tree for people, with these rules:
     - shared subproofs are numbered and later referenced;
     - asserted leaves are marked;
     - a run of one transitive rule (scm-sco, prp-trp, eq-trans) is
       flattened into a chain;
     - list cells and equality chains are folded (§8).
7. **Tests** (§7):
   - on chaos inputs, every inferred triple and every finding is
     explainable;
   - the reference evaluator checks each step;
   - the proof's leaves alone entail the goal;
   - the depth equals the reference's first naive round;
   - depth strictly decreases along every edge;
   - the proof is the same on one thread and on all cores.

**Why not the alternatives.**

- **Stamping each tuple with its round or rule.** This is what Nemo and
  Soufflé do. It costs memory, and Soufflé measures 1.31× runtime and
  1.76× memory on average. TurbOWL's variables also merge their batches
  and lose rounds (§0).
- **Top-down search that blocks a premise already on the path.** This is
  what the Protégé plugin and puli do. It is not minimal, its results are
  path-dependent and cannot be memoised, and its unravelled tree can be
  exponential (§3.2).
- **A whole second run that stamps rounds.** This costs a full run. It
  stays an option for "explain everything" (§3.6).

**Expected cost** (estimate): milliseconds per triple for typical OWL RL
proofs, which are shallow with small fan-out. ELK's comparable on-demand
tracing takes 2 to 10 ms per subsumption (§3.4). The bad cases are long
transitive chains and large `owl:sameAs` classes, where the ball grows as
n² to n³. The limits are for those.

## Summary

1. **Nobody explains rewritten equality after the fact.**
   - RDFox: "Facts in data stores where `equality` … is set to anything
     other than `off` cannot currently be explained using this command"
     ([rdfox-shell, explain](https://docs.oxfordsemantic.tech/rdfox-shell.html#explain),
     Note).
   - Soufflé, with provenance on, replaces each `eqrel` union-find with
     explicit transitivity, symmetry and reflexivity rules (§2.2).
   - The closest published algorithm is B/F≈'s `checkProvability`: it
     proves facts against a rewritten store but builds no proof (IJCAI
     2015, Alg. 4; §2.1).
2. **The systems with cheap shortest proofs store something per fact.**
   - Soufflé stores a rule number and a height per tuple: 1.31× runtime
     and 1.76× memory on average (TOPLAS 2020, abstract).
   - Nemo reads each fact's step from per-step delta tables that its store
     keeps anyway. It "adds no cost to reasoning" (KR 2024 §2), but it
     returns the first match and no shortest proof.
   - GraphDB rejected recording the support of each statement: that "track
     information" takes "up to 90% of the storage" on large data. It
     re-derives backward instead (§2.3).
3. **With nothing stored, shortest proofs come from search.**
   - Minimum depth is polynomial: the first round of the immediate
     consequence operator (Calautti et al., Prop. 28).
   - Minimum tree size is polynomial (Alrabbaa et al., LPAR 2020,
     Thm. 16).
   - Minimum DAG size is NP-complete (same paper, Thm. 11).
   - A ball of relevant rule instances plus a layered pass gets the first
     two at a cost linear in the ball (§3).
4. **ELK has done this for EL since 2014.** After saturation it re-runs
   the inferences restricted to the conclusions asked about. That costs
   2 to 10 ms per traced subsumption on GO, GALEN and SNOMED CT. Tracing
   everything up front ran out of memory on SNOMED CT (ISWC 2014,
   Tables 2–3; §3.4).
5. **Lists have no standard presentation.**
   - RDFox and Nemo show the internal helper rules a list compiles to.
   - Protégé and Horridge show the OWL axiom.
   - Since TurbOWL's rules are the W3C table, a proof should name the W3C
     rule and show the list as one folded premise (§5).
6. **Inconsistency.**
   - Rule engines treat a contradiction as a fact (RDFox's `owl:Nothing`)
     or have no explanation for it (GraphDB's consistency checks derive
     nothing).
   - OWL tools turn it into the entailment `owl:Thing ⊑ owl:Nothing` and
     compute justifications (owlexplanation; Stardog's `-i`).
   - For TurbOWL, a finding is one rule instance whose body triples get
     ordinary proofs (§6).
7. **Testing.**
   - ELK tests that the goal is derivable from asserted axioms through the
     returned inferences, and that no conclusion is dead.
   - Soufflé and Nemo test against golden outputs.
   - TurbOWL can do better: the reference evaluator gives an independent
     check of every step, of the leaves' sufficiency, and of the depth
     (§7).
8. **Presentation.**
   - A DAG with shared subproofs is the honest form. Nemo moved from a
     tree to a DAG in PR #418.
   - People read trees, expanded on demand: Soufflé's `setdepth` and
     `subproof`, Protégé, and Evonne's collapsed "magic vertex".
   - Minimal-height proofs in real programs exceed 200 nodes (Zhao et al.),
     so folding matters (§8).

## 0. Starting point

What a run leaves, and what it does not:

- **The base**, deduplicated, by predicate, in the term ids as written:
  sorted `(s, o)` pairs plus a hash set of recent additions
  (`reasoner.rs:1399-1427`). `Base::contains` answers "is this triple
  asserted?" in the user's terms. A snapshot restores it
  (`Base::restored`), so explanations can work after loading one
  (analysis).
- **The store**, one table per predicate, every tuple in representatives
  (`engine/store.rs:1-24`), generalized triples included.
- **The canon**: each id's representative, and each class's members
  (`engine/canon.rs:21-40`). It records no reason for a merge.
- **No rounds.** A `Variable` keeps "batches seen by earlier rounds, sizes
  at least doubling towards the front (datafrog's tiering)"
  (`engine/variable.rs:16-24`). Batches merge, so the round in which a
  tuple appeared is lost. Keeping it would cost at least 4 bytes per tuple
  and a change to the hot path. The goal rules that out.
- **The findings**, in representatives, as `(code, terms)`
  (`rules/mod.rs:113-131`). Their public `Diagnostic`s expand each term to
  every member of its class.
- **The family facts**, the heads of rules with an empty body (cls-thing,
  cls-nothing1, `rules/mod.rs:75-79`). They are inferred, never base, so a
  proof leaf is either asserted or one of these axioms.
- **The rules are Rust code**, not data (`rules/mod.rs:155-167`). Nothing
  can run them backward. The same rules exist as data twice, in
  `oracles/rules/*.rls` (Nemo syntax, with helper predicates and
  oracle-specific encodings) and in `crates/reference` (naive loops). By
  its own doc, the core shares no rule code with either
  (`crates/reference/src/lib.rs:9-12`).
- **FBF** (ticket 23, `docs/tickets/v2/23-fbf.md`) plans "a bounded
  backward search per family" for "another proof" of a tuple. That is the
  same backward matcher this note needs, in representatives.

Explanations were out of scope for version 1 (`docs/specs/v1.md:278`) and
for version 2's phase 1 (`docs/specs/v2-1-scoreboard-and-rules.md:317`).

## 1. Nemo

### 1.1 What it records

Nothing is recorded for tracing. Tracing reuses bookkeeping the engine
always keeps:

- **The rule applied at each step.** `rule_history: Vec<usize>`, "For
  each step the rule index of the applied rule", with one rule per step
  (N/nemo/src/execution/execution_engine.rs#L80-L81, #L156-L158, #L237).
- **One table per step and predicate.** These are the semi-naive delta
  tables (N/nemo/src/table_manager.rs#L66-L70, #L127-L130). After 8 of
  them (`MAX_FRAGMENTATION`), a union table is built in addition. I found
  no code that drops the single tables (execution_engine.rs#L263-L292;
  table_manager.rs#L132-L142).

A fact's step is the step of the table that holds it: a linear scan of the
predicate's tables, each a trie lookup (table_manager.rs#L747-L757). The
KR 2024 paper says the delta tables help "the tracing engine to
re-construct traces", and that tracing "is very efficient and adds no cost
to reasoning" (Ivliev et al., KR 2024, §2). The memory is paid by the
store's design either way. TurbOWL's merged batches do not keep it (§0).

### 1.2 How a trace is rebuilt

`trace_recursive` (N/nemo/src/execution/execution_engine/tracing/simple.rs#L38-L188)
works top-down, on demand:

1. Find the fact's step s. A fact of an input predicate, or one from
   step 0, is `Input`.
2. Take the one rule `rule_history[s]`.
3. Unify each head atom of that rule with the fact; existential variables
   are skipped.
4. Run the rule's body plan with the bound head variables as a filter
   (N/nemo/src/execution/planning/strategy/tracing.rs#L43-L80). The body
   reads only tables of steps before s (`UnionRange::Old`, tracing.rs#L94-L100).
5. Take the first match (`execute_plan_first_match`), and recurse on each
   positive body atom.

A memo table keyed by fact makes the result a DAG. PR #418 changed it
"from a tree to a DAG" (`ExecutionTrace { facts: IndexSet<TracedFact> }`,
…/tracing/simple/storage.rs#L127-L136).

**The cycle guard** is the strict step order alone, and it is enough:
steps are well-founded.

**What is chosen.** The proof is the first match of the rule that created
the fact. It is not a shortest proof, and Nemo offers no other (issue #677:
a trace shows one derivation, "not … all possible derivations").

Two further query APIs exist for the browser:

- **`TreeForTableQuery`** gives one tree for a set of facts of one
  predicate (N/nemo/src/execution/tracing/tree_query.rs#L15-L62). Its
  engine tries the cartesian product of every fact's matches until all
  children trace. That is exponential in the worst case
  (…/tracing/tree_query.rs#L390-L432 in the engine).
- **`TableEntriesForTreeNodesQuery`** takes a pattern of rules and returns,
  for each node, every fact that fits some trace
  (N/nemo/src/execution/tracing/node_query.rs#L77-L196). It works
  set-at-a-time: bottom-up, then top-down.

### 1.3 The user surface

- **CLI** (N/nemo-cli/src/cli.rs#L158-L190): `--trace <FACTS>` ("Facts for
  which a derivation trace should be computed"), `--trace-input-file`,
  `--trace-output`, `--trace-all-idb-facts` (new in 0.10.1), and
  `--trace-tree`/`--trace-node`, which read a JSON query from a file.
  Tracing runs after reasoning (N/nemo-cli/src/main.rs#L264). Without
  `--trace-output` it prints an ASCII tree; with it, it writes JSON.
- **Output types** (N/nemo/src/execution/tracing/trace.rs):
  - an ASCII tree (#L138-L143), which expands the DAG with repeated
    subtrees;
  - JSON `{finalConclusion, inferences: [{rule, conclusion, premises}]}`,
    with input facts as rule `"Asserted"` (#L246-L262);
  - GraphML of the DAG for Evonne (#L420-L456).
- **Python**: `engine.trace(fact) -> NemoTrace`, one fact per call
  (N/nemo-python/src/lib.rs#L440-L455).
- **Nemo Web**: an "Explain this inference" button per row opens the
  Nemo Explanation Visualizer (knowsys/nemo-web `fad0533`,
  `PredicateResults.tsx` L92-103). The docs call it "experimental"
  (nemo-doc `installation/browser.md`).

### 1.4 Cost and the issues that show it

Per fact, the cost is one trie lookup per delta table of its predicate, one
first-match join per head, and recursion over the DAG's distinct facts. The
issues show what goes wrong:

- **#818 (closed):** tracing is quadratic. The plan keeps the forward
  variable order, and bound variables are only a filter. With
  `q(?X) :- e(?X,?Y), e(?Y,?Z)` and n = 20,000 it took 35.7 s. PR #819,
  which puts bound variables first (0.62 s), was closed unmerged.
- **#697:** tracing 192k facts took 22 h, and about 3 h after PR #701.
- **#816:** the program is cloned once per traced fact
  (simple.rs#L268). The fix, PR #817, was closed unmerged.
- **Unverified:** a reporter wrote "there's a rewrite in progress". I
  could not find it.

**Lesson for TurbOWL:** the backward matcher must plan with the head's
bound variables first.

### 1.5 Negation, aggregates, nulls, built-ins

- **Negated atoms** are checked as filters and do not appear in a trace.
- **min and max aggregates** are supported (maintainer on #469).
- **Existential variables** are ignored in head unification, so the body
  match found need not be the trigger that created the null. This is my
  reading of simple.rs#L98-L102.
- **Filters and built-ins** stay in the tracing plan (tracing.rs#L62-L66).

Only built-ins matter for TurbOWL: the `dt-*` value checks.

## 2. Other systems

### 2.1 RDFox

**What it outputs.**
- The shell command is
  `explain [shortest | to-explicit | exhaustive] [<max_depth> [<max_rule_instances>]] <fact>`.
  `shortest` prints "just one shortest derivation (in terms of height)";
  `to-explicit` prints all derivations down to explicit facts;
  `exhaustive` also explains explicit facts
  ([rdfox-shell, explain](https://docs.oxfordsemantic.tech/rdfox-shell.html#explain)).
- The text form is an indented tree: each derived fact, then the rule and
  its binding (`{ ?x -> :kiki }`), with leaves marked `EXPLICIT`
  ([reasoning §"Explaining reasoning results"](https://docs.oxfordsemantic.tech/reasoning.html#explaining-reasoning-results)).
- Proofs show RDFox's internal rules, never the OWL axioms: user rules,
  rules translated from axioms, and "special rules", which include equality
  axiomatised as a reflexive, symmetric and transitive relation (same
  section).
- The web console always shows a shortest proof.

**The REST form.** `GET /datastores/<name>/explanation` takes `fact`,
`type`, `max-distance-from-root` and `max-rule-instances-per-fact`. It
returns `application/x.explanation+json`: a DAG keyed by fact id, root 0
([APIs, "Explaining fact derivation"](https://docs.oxfordsemantic.tech/apis.html#explaining-fact-derivation)).
- Each fact has `type` (explicit, derived or false), `distance-from-root`,
  `proof-height` ("least number of rule applications"),
  `rule-instances-complete`, `shortest-proof-rule-instance` and its
  `rule-instances`.
- Each rule instance has `rule`, `grounded-rule-structured` and
  `body-facts`.
- `proof-height` becomes null when the limits cut the search.

**How it works.** The docs do not say whether anything is recorded during
reasoning. Listing every rule instance of a fact suggests backward matching
over the store (unverified).

**Equality.** Facts cannot be explained when `equality` is anything but
`off` (see the quote in the Summary). The equality modes are `off`,
`noUNA` and `UNA`
([data stores, equality](https://docs.oxfordsemantic.tech/data-stores.html#equality)).

**Cost.** No figures are published. The only controls are the two limits.

**The Oxford papers.** "Handling owl:sameAs via Rewriting" (AAAI 2015,
<https://arxiv.org/abs/1411.3622>) says nothing about explanation. Its
answers are the expansion Tρ = {⟨s,p,o⟩ | ⟨ρ(s),ρ(p),ρ(o)⟩ ∈ T} (p. 3),
and facts that become outdated when ρ changes are marked and removed, so
no derivation of an unrewritten fact survives. The two maintenance papers
hold the nearest algorithms:

- **B/F≈** (Motik, Nenov, Piro, Horrocks, IJCAI 2015,
  <https://www.cs.ox.ac.uk/boris.motik/pubs/mnph15incremental-BF-sameAs.pdf>).
  `checkProvability(F)` (Alg. 4, p. 5) backward-chains over the rewritten
  materialisation:
  - It matches each rewritten rule whose head matches F and evaluates the
    body in the store.
  - It checks `c≈c` for each constant of F (replacement rules ≈1–≈3).
  - For `c≈c` it recurses on every fact that mentions c (reflexivity, ≈4).

  A forward `saturate()` then fixes what is proved (§3.1–3.2, pp. 3–4). It
  proves facts in representatives; it builds no proof in the user's terms.
- **FBF** (Motik et al., "Maintenance of datalog materialisations
  revisited", AIJ 269, 2019,
  <https://www.cs.ox.ac.uk/boris.motik/pubs/mnph19maintenance-revisited.pdf>).
  - Backward chaining unifies a fact with the rule heads and evaluates the
    partly bound body (Ex. 22, p. 24).
  - Naive recursion with a "no fact proves itself" check can make (n−1)!
    calls (Ex. 23, p. 25). FBF therefore checks each fact at most once,
    in a set C, and proves forward over the proved set P, so the order
    does not matter.
  - A bound-head query can still touch n(n−1) useless facts (Ex. 38,
    §9.3, p. 34).
  - The paper sets equality aside (§10, p. 35).

  This is the closest published relative of the ball and layered pass in
  §3.5.

### 2.2 Soufflé

**What it outputs.**
- `-t/--provenance [none | explain | explore]` turns provenance on
  (`src/MainDriver.cpp:726` at `2adf8c0`). An older `subtreeHeights` mode
  (1.7.1 to 2.0.0) is gone.
- The interactive shell offers `setdepth` (default 4), `explain`,
  `subproof`, `explainnegation`, `rule`, `measure`, `output`, `format`
  and `query` (`src/include/souffle/provenance/Explain.h:84-208`).
- The output is an inference-rule tree with `(R<n>)` labels. Branches
  deeper than the set depth become `subproof` labels, which the `subproof`
  command expands; `format json` is also available
  ([provenance](https://souffle-lang.github.io/provenance)).
- For non-existence, the user picks the rule and the free variables,
  because "it is not technically feasible to automatically generate
  explanations for non-existence" (same page).

**How it works** (Zhao, Subotić, Scholz, "Debugging Large-scale Datalog: A
Scalable Provenance Evaluation Strategy", ACM TOPLAS 42(2), 2020,
<https://souffle-lang.github.io/pdf/toplas20.pdf>).
- Each tuple carries `@rule` and `@height`, the height being one more than
  the highest body tuple. A lower height found later replaces the
  annotation (§4.1, p. 7:22). Theorem 1 of the arXiv version says the
  annotations are minimal heights (<https://arxiv.org/abs/1907.05045>,
  p. 12).
- A proof is rebuilt top-down, one level per query: the tuple's rule is
  evaluated with the head bound and h(tᵢ) < h(t) on each body atom, and
  the first solution is taken (§3.3, pp. 7:16–17;
  `src/ast2ram/provenance/SubproofGenerator.cpp:140-155`).
- The result is one minimal-height tree per tuple. The paper argues that
  plain top-down search gives "no guarantees for minimality of height"
  (arXiv §3, p. 7).

**Equality.** With provenance on, the `ExpandEqrels` pass replaces each
`eqrel` union-find with explicit transitivity, symmetry and reflexivity
rules (`src/MainDriver.cpp:477`,
`src/ast/transform/ExpandEqrels.cpp:64-110`).

**Cost** (TOPLAS 2020).
- 1.31× runtime and 1.76× memory "on average" (abstract).
- On Doop with 8 threads: geometric means 1.24× and 1.31× runtime, 1.44×
  and 1.46× memory (Table 2, p. 7:27).
- On Ddisasm: 1.39× runtime and 2.60× memory, worst at 2.20× and 3.37×,
  from extra indexes (Table 4, p. 7:29).
- Storing direct subproofs naively cost up to 100× memory on a
  2,000-tuple transitive closure (p. 7:26).
- A proof query takes under 1 ms per node. 20 levels exceed 15,000 nodes,
  and full trees exceed height 200 (§5.2–5.3).

### 2.3 GraphDB

**What it outputs**
([inference, proof plugin](https://graphdb.ontotext.com/documentation/11.5/inference.html#proof-plugin)).
- `?ctx proof:explain (?s ?p ?o)` binds a request node. `proof:rule` gives
  the rule name or `"explicit"`; `proof:subject`, `proof:predicate`,
  `proof:object` and `proof:context` give each premise.
- There is one row per premise per solution, for every rule that derives
  the statement.
- It goes one level deep. Deeper levels need more queries.
- Solutions in which the statement is its own premise are skipped.

**How it works.** It calls the inferencer's `isSupported`, with a handler
that collects each rule solution (`ProofPlugin.java:39-40, 411, 450-480`
at `9ce7cb4`). That is the retraction check: `isSupported` "uses queries
that contain the premises of the rules, and the variables of the rules are
preliminarily bound using the statement in question … this method performs
backward chaining"
([reasoning, retraction](https://graphdb.ontotext.com/documentation/11.5/reasoning.html#retraction-of-assertions)).

**Why it does not record.** The same page rejects recording, for each
statement, what it supports: "track information inflates more rapidly than
the inferred closure — in the case of large datasets up to 90% of the
storage is required just to store the track information".

**Limits.**
- `owl:sameAs` uses a representative per class. How the plugin explains
  expanded statements is unverified: an `isDerivedFromSameAs` flag is
  computed and never used (`ProofPlugin.java:298-316, 389-404`).
- Consistency checks "do not have any consequences", so there is nothing
  to explain (my reading; unverified).
- No cost figures are published.

### 2.4 Stardog

**What it outputs**
([`stardog reasoning explain`](https://docs.stardog.com/stardog-cli-reference/reasoning/reasoning-explain)).
- `stardog reasoning explain [-i] [-l N] [-m] [-o TEXT|TREE|<RDF format>] <db> <triple|file>`
  returns "the minimal set of asserted statements in the database that are
  needed for the inference to hold". That is a justification, not a proof.
- `TREE` groups the statements under the inferred ones. `-l` (default 1)
  asks for more than one explanation, `-m` merges them, and `-i` explains
  why the database is inconsistent.
- In Java, `StardogExplainer` has `proof()`, `proofs()`, `countLimit`,
  `mergeExplanations`, a 30 s default `timeLimit` and a special
  `INCONSISTENCY` axiom
  ([javadoc](https://docs.stardog.com/javadoc/snarl/com/complexible/stardog/reasoning/StardogExplainer.html)).

**How it works.** Stardog "does not materialize inferences"; it rewrites
queries ([inference engine](https://docs.stardog.com/inference-engine/)),
so every explanation is computed on demand. The algorithm, and whether the
set is really minimal, is not documented (unverified).

**Equality.** `owl:sameAs` is "computed and indexed eagerly", and one
canonical individual is returned
([advanced reasoning, "Same As Reasoning"](https://docs.stardog.com/inference-engine/advanced-reasoning-features)).
How it appears in a proof is unverified.

## 3. Algorithms: backward proof search over a materialised closure

### 3.1 The problem

The closure C is known, and so is the base B ⊆ C. For a goal g ∈ C, find a
proof:

- a DAG with g as its one sink;
- each node a fact of C;
- each inner node the head of a rule instance whose body facts are its
  children;
- each leaf in B, or the head of an empty-body rule;
- acyclic.

Alrabbaa et al. call this a grounded, sound, acyclic derivation structure
with one sink (LPAR 2020, Defs. 2 and 4, p. 5,
<https://arxiv.org/abs/2004.08311>). Every fact of C has one, since C is
the least fixpoint. The difficulty is finding a well-founded one, and a
short one, without knowing when each fact was derived.

### 3.2 Top-down over the closure, cycles blocked on the path

**The method.** For the goal, each rule whose head unifies gives a body
query over C. Each match is an inference whose premises are in C: the
"inference oracle" of Kazakov and Klinov (ISWC 2014, §3.2, p. 203,
<https://rd.springer.com/content/pdf/10.1007/978-3-319-11915-1_13.pdf>).
These inferences form a hypergraph that may be cyclic. For example, eq-sym
proves `a sameAs b` from `b sameAs a` and back.

**How it is made acyclic.**
- The Protégé proof plugin drops any inference with a premise already on
  the path from the root. It then hides "dead" nodes, those whose label
  cannot be derived once the inferences that conclude path facts are
  removed: a linear-time check per node (Kazakov, Klinov, Stupnikov,
  DL 2017, §4.2, p. 10,
  <https://www.ceur-ws.org/Vol-1879/paper31.pdf>).
- puli does the same: `AcyclicProofNode.java` blocks every ancestor, and
  `InferenceDerivabilityChecker.java` says "a conclusion is derivable if
  it is not blocked and a conclusion of an inference whose all premises
  are (recursively) derivable"
  (<https://github.com/liveontologies/puli>).

**Cost.** Derivability depends on the path, so it cannot be memoised per
fact. The unravelled tree can be exponential, and nothing makes it short.
FBF's (n−1)! example (§2.1) is the same trap.

### 3.3 Stored stamps: rebuilding from a known order

**Nemo** (step per fact, §1) and **Soufflé** (height per tuple, §2.2) both
query one level at a time, with premises bounded by the stamp: a step
before s, or a height below h(t). The strict order makes any proof found
well-founded:

- With heights, it is also of minimal height (Zhao et al., Thm. 1).
- With steps, it is just the first one found.

Both cost memory at run time (Soufflé 1.76× on average). TurbOWL cannot
read stamps it never wrote (§0).

### 3.4 ELK: goal-directed tracing after saturation

**The algorithm** (Kazakov and Klinov, ISWC 2014).
- Algorithm 3 (p. 204) re-saturates a set Exp of conclusions by forward
  chaining. Premises outside Exp act as a "set of support". It applies
  only the inferences whose conclusions are in Exp, and fills a map from
  each conclusion to its inferences.
- Theorem 2 (p. 205): the map's keys are exactly Exp, and each value holds
  every inference with that conclusion.
- Keeping only the first inference per conclusion gives one proof.
- Exp is a partition, all subsumptions with the same left side (§4,
  p. 206). Partitions are traced recursively as premises lead into them.

**Cost.**

| Ontology | Saturation | Full tracing (Table 2, p. 208) | Per subsumption (Table 3, p. 209) |
|---|---|---|---|
| GO | 1,877 ms, 546 MB | 5,004 ms, 1,102 MB | 2.0 ms |
| GALEN | 1,326 ms, 722 MB | 3,786 ms, 765 MB | 1.9 ms |
| SNOMED CT | 16,968 ms, 924 MB | out of memory | 10.1 ms |

On SNOMED CT, a subsumption traced 788 inferences on average and used
386.

**Caveats.**
- The tracing "guarantees only that all proofs performed by ELK will be
  generated, not all proofs supported by the calculus" ("Advancing ELK",
  DL 2015, p. 1, <https://www.ceur-ws.org/Vol-1350/paper-27.pdf>).
- OWL RL triples have no small natural partition like "same left side".
- First-inference-per-conclusion is well-founded within one partition.
  Across partitions, a chosen inference in A can use a fact of B whose
  chosen inference uses A's fact (analysis). A global guarantee needs the
  traced set to be closed backward, which is what §3.5 builds.

The lesson for TurbOWL: re-running inference after the run, restricted to
what the question touches, is fast in practice and stores nothing during
the run.

### 3.5 The relevance ball and a layered forward pass (recommended)

**Backward: the ball.** Elhalawati, Krötzsch and Mennicke define the
downward closure A↓ of a fact in the graph of rule instances. It captures
every proof tree of A (Prop. 2), and they build it on demand by joining
each rule's body with the head bound, starting at the goal ("An Existential
Rule Framework for Computing Why-Provenance On-Demand for Datalog",
RuleML+RR 2022, §3,
<https://iccl.inf.tu-dresden.de/w/images/f/f6/An_Existential_Rule_Framework_for_Computing_Why_Provenance_On_Demand_for_Datalog_%282%29.pdf>).
- They give each fact a node id during the run, through an existential
  copy rule. In TurbOWL the triple itself is the id, so nothing is needed.
- They compute all why-provenance with FPsolve, which "took hours or even
  days" (§6.1). One proof needs none of that.
- Building A↓ is magic sets applied to a provenance query (Bancilhon,
  Maier, Sagiv, Ullman, PODS 1986; Beeri and Ramakrishnan, J. Logic
  Programming 1991; cited from memory, unverified).

**Forward: the layers.** Treat the ball as a propositional Horn program:
one clause per rule instance.
1. Give each clause a counter of premises not yet derived.
2. Mark the base facts and the empty-body heads at layer 0.
3. Process facts layer by layer. When a clause's counter reaches zero,
   its head, if still unmarked, goes in the next layer and keeps that
   clause.

This is the linear-time Horn satisfiability method of Dowling and Gallier
(J. Logic Programming 1(3), 1984; cited from memory, unverified).

**Why the layer is the minimum depth.** Calautti, Livshits, Pieris and
Schneider ("The Complexity of Why-Provenance for Datalog Queries", arXiv
2303.12773, App. C, Prop. 28, pp. 19–20, <https://arxiv.org/abs/2303.12773>):
the minimal depth over all proof DAGs "can be computed in polynomial time",
because "the step at which a fact α is first obtained" by the immediate
consequence operator "coincides with min-dag-depth".

**Why the result is well-founded and minimal** (analysis).
- Following each fact's kept clause from the goal gives a DAG in which
  every edge goes to a strictly lower layer. It is acyclic and grounded
  by construction.
- Its depth is the goal's layer, the minimum depth of any proof inside
  the ball. When the ball is all of A↓, that is the minimum over all
  proofs.
- Ties can go to a second measure. Minimum tree size (one plus the sum of
  the children) is a "monotone recursive" measure that a Dijkstra-like
  algorithm optimises in polynomial time (Alrabbaa, Baader, Borgwardt,
  Koopmann, Kovtunova, CADE 2021, Def. 9, Lemma 10, Algorithm 1,
  Thm. 12, <https://arxiv.org/abs/2104.13138>). Knuth's 1977
  generalisation of Dijkstra to superior functions covers both depth and
  size (IPL 6(1); cited from memory, unverified).

**Cost.**
- Building the ball costs one backward match per fact in it.
- The pass is O(clauses) for depth, and O(clauses · log facts) with a heap
  for size.
- A cheap goal stays cheap, since the ball is bounded by what the
  question touches.

**Related work** (Deutch, Gilad, Moskovitch, "Selective provenance for
Datalog programs using top-k queries", PVLDB 8(12), 2015, §5.1,
Algorithm 2, <https://amirgilad.github.io/publication/vldb15/VLDB15.pdf>).
It finds the top-1 tree by a greedy bottom-up pass with weights in an
ordered monoid, run alongside a re-evaluation of an instrumented program.
- That costs 3% to 21% over semi-naive evaluation, and 38% for one
  six-node pattern.
- Full provenance was infeasible beyond 1.6M tuples (§6.2).

### 3.6 Iterative deepening, and the whole-run alternative

**Iterative deepening** (analysis).
- A proof of depth at most d uses only facts within backward distance d of
  the goal. Build the ball to radius d and run the layered pass, with every
  unexpanded frontier fact that is not a leaf treated as underivable. The
  goal is reached exactly when a proof of depth at most d exists.
- Doubling d (1, 2, 4, …) keeps the total work within a small constant of
  the last round's.
- The pass computes exact layers inside the last ball, so the proof found
  has minimum depth even though d jumped past it.
- A top-down search with a memo on (fact, k), meaning "derivable within
  depth k", is the same thing seen from above, at O(d · clauses).
- Proofs whose depth strictly decreases need no blocking on the path.
  That is what §3.2 lacks.

**The whole-run alternative.** Re-run the materialisation once, from the
base, in an evaluator that stamps each fact with its first round, and then
answer each question by Soufflé's one-level query.
- It is worth it only for "explain everything" (a bulk export).
- It costs a full run, plus 4 bytes per tuple while it lives.
- The stamps give minimum depth only if each round sees only earlier
  rounds' facts. TurbOWL's engine already works that way: rules read the
  store as the last round left it (`rules/mod.rs:34-46`).

### 3.7 What is hard, and what is not

| Question | Complexity | Source |
|---|---|---|
| Some proof of g | linear in the ground rule instances (Horn) | Dowling–Gallier 1984 (unverified) |
| Minimum-depth proof | polynomial: the first round of the immediate consequence operator | Calautti et al., Prop. 28 |
| Minimum tree size | polynomial | Alrabbaa et al., LPAR 2020, Thm. 16 |
| Minimum DAG size (distinct facts) | NP-complete, even for polynomial derivers | Alrabbaa et al., LPAR 2020, Thm. 11; Flatt et al., FMCAD 2022, §I, for congruence closure |
| Is D′ ⊆ B the support of some proof tree (why-provenance)? | NP-complete for recursive Datalog | Calautti et al., Thm. 3 |
| … restricted to minimal-depth or unambiguous trees | still NP-complete | Calautti et al., Thms. 27 and 14 |

The hard results are about all proofs or about smallest DAGs. One proof of
minimum depth, or of minimum tree size, is cheap. In practice the smallest
tree proof was always the unravelling of the smallest proof, on 1,573 ELK
tasks (Alrabbaa et al., LPAR 2020, §5, p. 11).

### 3.8 The recommendation, and its risks

Use §3.5 with §3.6's deepening (the design at the top). It records
nothing, is well-founded by construction, gives minimum depth, costs time
linear in the ball, and is easy to test (§7).

**Risks** (analysis):

- **Transitive chains make the ball dense.** For `a sco z` on a chain of n
  classes, there are n candidate midpoints at each level: O(n²) facts and
  O(n³) instances. The minimum-depth proof is a balanced tree of depth
  about log₂ n, which reads worse than a chain. Two remedies:
  - Present runs of one transitive rule as a flat chain (§8). Depth
    becomes a search measure, not a layout.
  - Keep RDFox's per-fact instance limit. With 50 classes there are about
    125,000 instances (estimate): fine.
- **Large `owl:sameAs` classes** multiply the ball by the class size
  (§4).
- **The rule table must match the families.** A rule missing from the
  table makes facts unexplainable; a wrong rule makes proofs unsound. The
  tests in §7 catch both.

## 4. `owl:sameAs` with one representative per class

### 4.1 What the store knows

- The canon knows the classes, not why they formed (§0).
- The base holds every asserted `owl:sameAs` triple in the user's terms.
- A derived `owl:sameAs` is kept only as its class's `r owl:sameAs r`, or
  as a merge. The derived ones come from prp-fp, prp-ifp, prp-key,
  cls-maxc2, cls-maxqc3/4, and prp-spo1 or prp-symp where `owl:sameAs`
  is the property.
- Every stored tuple stands for the product of its terms' classes
  (`rules/equality.rs:22-31`).

### 4.2 The design: proofs over the W3C rules, in the user's terms (analysis)

Work on facts in the user's terms. Membership is
`store.contains(find(s), find(p), find(o))`. The rule table holds the five
eq-* rules as written (W3C Table 4, as `oracles/rules/equality.rls` writes
them):

- **eq-sym:** `y sameAs x ← x sameAs y`.
- **eq-trans:** `x sameAs z ← x sameAs y, y sameAs z`. Here y ranges over
  the members of the class.
- **eq-rep-s:** `s' p o ← s sameAs s', s p o`. Here s ranges over the
  members of the class of s'. eq-rep-p and eq-rep-o work the same way.

A fact that holds only through the rewriting gets an ordinary proof. With
`ex:a ex:p ex:c` asked, `ex:b ex:p ex:c` and `ex:a owl:sameAs ex:b`
asserted, the proof is:

```
ex:a ex:p ex:c                        [eq-rep-s]
├── ex:b owl:sameAs ex:a              [eq-sym]
│   └── ex:a owl:sameAs ex:b          asserted
└── ex:b ex:p ex:c                    asserted
```

**Why in the user's terms, not in representatives.**
- The user asked about the terms they wrote.
- The reference evaluator closes `owl:sameAs` with the eq-* rules and no
  representatives (`crates/reference/src/lib.rs:13-15`), so it can check
  every step (§7).
- ADR 0003 already makes the rewriting invisible in the output. Proofs
  should keep that promise.

**Cost.**
- A variable that a rule body shares binds to the members of a class only
  when the class has two or more members. Data with no `owl:sameAs` costs
  exactly what proofs in representatives would.
- A class of n members has n² `sameAs` facts, each with n eq-trans
  instances, so a goal deep in a large class builds a ball of O(n³)
  (estimate). The instance limit per fact bounds it. A class of 10,000
  members is rare in RL data but not impossible (dt-eq on many equal
  literals).

**Well-foundedness is what makes equality hard.** The reason for a link
can itself need the class. Take prp-fp: `ex:u ex:f y1` and `ex:u ex:f y2`
give `y1 sameAs y2`. If `ex:u ex:f y2` holds only through eq-rep-o from
`y1 sameAs y2`, that is a cycle. The layered pass rules such proofs out
by construction. A shortcut that searches a graph of equality links
directly would not.

**A proof-producing union-find would be the fast way, but only by
recording.**
- Nieuwenhuis and Oliveras keep a proof forest beside the union-find, one
  tree per class, whose edges are the equalities that justified each
  merge. Explaining a = b walks the path between them ("Proof-Producing
  Congruence Closure", RTA 2005,
  <https://www.cs.upc.edu/~roberto/papers/rta05.pdf>).
- egg does this, opt-in (`EGraph::with_explanations_enabled`,
  <https://docs.rs/egg/0.11.0/egg/struct.EGraph.html>). Flatt et al.
  state "no algorithmic overhead to maintaining it", while minimal proofs
  are NP-complete, and give an O(n log n) greedy for small ones ("Small
  Proofs from Congruence Closure", FMCAD 2022, §II, §V-B,
  <https://arxiv.org/abs/2209.03398>).
- In TurbOWL a forest would add memory to the canon during the run, which
  the goal forbids. It is an option only if a later decision accepts that
  cost.

**Presentation.** A run of eq-sym and eq-trans steps folds into one chain,
`x = a = b = y`, each link shown with its own reason: asserted, or the rule
that gave it (prp-fp with its two premises, and so on). This is display
only, over the proof the pass found (§8).

### 4.3 Special cases (analysis)

- **Vocabulary terms made equal** (a restart, ADR 0003) and **`rdf:nil`
  given a class** need nothing special. In the user's terms the W3C rules
  apply as written: a cell whose `rdf:rest` is equal to `rdf:nil` gets
  `rdf:rest rdf:nil` by eq-rep-o.
- **eq-ref** (switch `trivial`) is a rule like any other:
  `s sameAs s ← s p o`, and the same for p and o. Each premise is any
  triple that mentions the term.
- **Findings** are kept in representatives. A `Diagnostic` names one
  choice of members, and its explanation is about those members (§6).

### 4.4 What others do

- **RDFox:** no explanation when equality is on (§2.1). B/F≈'s
  `checkProvability` is the only published backward check against a
  rewritten store, and it gives a yes or no, not a proof in the user's
  terms.
- **Soufflé:** expands the union-find into explicit rules, so it pays the
  n² closure in order to explain (§2.2).
- **GraphDB and Stardog:** unverified (§2.3, §2.4).

## 5. Rule bodies with RDF lists

**The rules.** `LIST[h, e1, …, en]` abbreviates 2n triples (W3C OWL 2
Profiles §4.3, Table 3, <https://www.w3.org/TR/owl2-profiles/>). The
rules that read a list:

- deriving: cls-int1, cls-int2, cls-uni, cls-oo, prp-key, prp-spo2,
  scm-int, scm-uni;
- concluding `false`: eq-diff2/3, prp-adp, cax-adc.

TurbOWL reads lists as paths from the head to `rdf:nil`, every cell with
an `rdf:first`. A cell may have several firsts and rests, and a cyclic
list matches nothing (`rules/class/lists.rs:4-30`;
`oracles/rules/core.rls:87-108`).

**Backward matching** (analysis).
- A list atom enumerates the well-formed paths from h, each path one
  instance.
- For prp-spo2, the nodes u1 … u(n−1) are free: the match is a path search
  along p1 … pn from u0 to un, bounded by fan-out.
- For prp-key, both individuals share one value of each key property.
- Path enumeration needs a limit, since branching cells multiply paths.
  The instance limit covers it.

**How others present lists.**
- RDFox shows internal rules, so a list shows up as whatever its
  translation built (§2.1).
- Nemo's trace of TurbOWL's oracle program would show the helper
  predicates `listCell`, `wellFormed` and `allTypes` as proof steps
  (`oracles/rules/core.rls:92-111`).
- Horridge's justifications and the Protégé plugin show the OWL axiom
  (`C EquivalentTo A and B`), not triples (thesis, §1.3.3; DL 2017 §4).

**For TurbOWL** (analysis). The step is named after the W3C rule (cls-int1)
and lists, as one grouped premise, "the list (ex:A ex:B) at _:l". Its 2n
triples are shown folded and written in Turtle collection syntax
(`ex:C owl:intersectionOf (ex:A ex:B)`). Each list triple is still a node:
almost always asserted, but derivable (through eq-rep, for example), so it
can be expanded. The per-member premises (`y rdf:type ex:A`, …) stay
ordinary children.

## 6. Explaining inconsistency findings

**The rules.** The W3C rules that conclude `false`:

- eq-diff1–3 (Table 4);
- prp-irp, prp-asyp, prp-pdw, prp-adp, prp-npa1/2 (Table 5);
- cls-nothing2, cls-com, cls-maxc1, cls-maxqc1/2 (Table 6);
- cax-dw, cax-adc (Table 7);
- dt-not-type (Table 8).

All are in OWL 2 Profiles §4.3. The `checks` and `datatype` families
report them as `Finding { code, terms }` (`rules/mod.rs:113-131`). They are
in the oracle program as one relation per rule
(`oracles/rules/checks.rls:6-11`).

**What others do.**
- OWL tools turn inconsistency into an entailment. owlexplanation sets
  the entailment to `SubClassOf(owl:Thing, owl:Nothing)`
  (`ConsistencyEntailmentChecker.java`) and finds justifications by
  black-box expansion and contraction
  (`InconsistentOntologyExplanationGeneratorFactory.java`,
  <https://github.com/matthewhorridge/owlexplanation>).
- Horridge notes that an inconsistent ontology has no seed signature, so
  expansion by signature cannot start (thesis §6.3, p. 110,
  <https://www.bcs.org/media/2146/dd-2012-matthew-horridge.pdf>).
- ELK proves ⊤ ⊑ ⊥ as an ordinary goal (`ProofTest.java`,
  `inconsistentOwlThing`, in `liveontologies/elk-reasoner`).
- Stardog has `-i` and an `INCONSISTENCY` axiom (§2.4). RDFox has
  `owl:Nothing` facts, but explaining them is unverified (§2.1).

**For TurbOWL** (analysis).
- A `Diagnostic` names its rule and its terms, in the user's terms. Its
  explanation is the backward match of that rule with those terms bound
  (for cax-dw, `x rdf:type c1`, `x rdf:type c2` and `c1 owl:disjointWith
  c2`), followed by an ordinary proof of each body triple.
- dt-not-type's body is the literal's typing triple plus a built-in value
  check. The check is shown as a condition, not a premise.
- OWL 2 RL's rules do not explode on `false`, so the other proofs in an
  inconsistent closure keep their meaning, unlike ELK's.
- A finding kept across a restart still follows from the base
  (`reasoner.rs:59-63`), so it is explainable after the run.

## 7. Testing proofs

**What others test.**

- **ELK, provability:** `TestUtils.provabilityTest` checks, with puli's
  checker, that the goal is derivable from asserted axioms through the
  returned inferences, and that `getNonDerivableConclusions()` is empty
  (`elk-proofs/src/test/java/org/semanticweb/elk/owl/inferences/TestUtils.java`).
- **ELK, completeness:** `ProofTestUtils.proofCompletenessTest` enumerates
  every repair (minimal hitting set) of the proofs, removes it from the
  ontology, and asserts that the entailment is gone ("Not all proofs were
  found!") (`elk-owlapi/src/test/java/org/semanticweb/elk/owlapi/proofs/`).
- **Soufflé:** golden files. `tests/provenance/path/` holds a program, a
  script (`explain path("a","d")`) and the expected tree.
- **Nemo:** `trace_ascii` and `trace_json` in
  N/nemo/src/execution/tracing/trace.rs.
- **The formal properties** a proof must have: grounded, sound, one sink,
  acyclic (Alrabbaa et al., LPAR 2020, Defs. 2 and 4).

**For TurbOWL** (analysis). Each check, and what it would need:

| Property | How | Needs |
|---|---|---|
| The root is the question | equality | nothing |
| Each leaf is asserted or an empty-body axiom | `Base::contains`, the families' facts | nothing |
| Acyclic and well-founded | depth strictly decreases along each edge | nothing |
| Each step is an instance of its named rule | one round of the reference evaluator's rule over the step's premises yields the conclusion | a single-rule step in `crates/reference` (today it runs families) |
| The leaves alone entail the goal | `turbowl_reference::evaluate_within(leaves, switches)` contains the goal | nothing |
| The depth is minimal | the goal's depth equals the round in which the reference's naive evaluation first derives it (Calautti et al., Prop. 28) | first-round numbers from the reference |
| Every inferred triple and every finding is explainable | for chaos inputs, explain all of them | nothing |
| Determinism | the same proof on one thread and on all cores, and before and after a snapshot | nothing |
| Parity across adapters | Python's and the CLI's proofs equal the core's on the minimal cases | the minimal cases (`crates/python/examples/minimal_cases.rs`) |
| Reasoning unchanged | the bench before and after, by the loop's rules (`docs/specs/experiments.md`) | a quiet machine |
| Cost | a `scaling_*` test: explaining a goal of fixed depth costs about the same whatever the closure size | a generator of such goals |

Two notes on these checks:

- **Explaining every inferred triple also checks the closure** (analysis).
  If a triple of the closure has no proof inside its full ball, either the
  rule table lacks a rule or the closure holds a triple it should not.
  Chaos would then flag a soundness bug that the oracles' set equality
  might also have caught, but now with a pointer to the triple.
- **Nemo can serve as a weaker outside check.** `nmo --trace` over the
  oracle program gives a derivation for the same triple; its input facts
  must also entail the goal. That adds little over the reference
  evaluator, so it is optional.

## 8. Output formats and presentation

| System | Form | Notes |
|---|---|---|
| RDFox | indented text tree; JSON DAG keyed by fact id | `EXPLICIT` leaves; rule with its binding; shortest by default in the console (§2.1) |
| Nemo | ASCII tree; JSON `{finalConclusion, inferences}`; GraphML DAG for Evonne | input facts as rule `"Asserted"` (§1.3) |
| Soufflé | inference-rule tree, cut at a set depth with `subproof` labels; JSON | one level per query underneath (§2.2) |
| GraphDB | SPARQL rows: rule and premises, one level | the user walks down by hand (§2.3) |
| Stardog | a set of asserted statements; optional TREE | justification-like (§2.4) |
| Protégé proof plugin | expandable tree, cycles removed per path, dead nodes hidden | recursive expansion up to a limit; a limit on inferences per conclusion; tooltips with sample rule instances (DL 2017 §4.2, p. 11) |
| Evonne | tree, linear, and bidirectional layouts | "the entire proof is collapsed into a magic vertex that links the conclusion directly to its justification", steps pulled out on demand (IJCAR 2022 §3, <https://arxiv.org/abs/2205.09583>) |

**Justifications and proofs.**
- A justification is "J ⊆ O, J ⊨ η, and for all J′ ⊊ J, J′ ⊭ η" (Horridge,
  thesis, Def. 1, p. 45). Justifications "are merely the premises of a
  proof and … do not articulate the … reasoning steps" (§1.3.3).
- Laconic justifications keep only the relevant parts of axioms (Horridge,
  Parsia, Sattler, ISWC 2008, Def. 2,
  <https://link.springer.com/content/pdf/10.1007/978-3-540-88564-1_21.pdf>).
- The leaves of a proof are a superset of some justification (ELK,
  ISWC 2014, §5). Shrinking them to one costs repeated reasoning (black-box
  contraction), so it is out of scope here.

**Making proofs smaller.**
- Evonne's signature-based condensation hides subproofs of axioms that use
  only known terms. It shrank 770 of 2,000 proofs, 91 of them to one step
  (§4.2, §5.2).
- Minimal-height proofs in Doop exceed 200 nodes (Zhao et al., arXiv p. 2),
  so a proof needs folding and expansion on demand, not a full dump.

**For TurbOWL** (analysis):

- **The model.** A `Proof` is a node list with a root. Each node is a
  triple (or `false` with a finding's code and terms), with:
  - how it holds: `asserted`, `axiom(rule)`, or `rule(name, premises,
    conditions)`;
  - its depth;
  - a flag for a generalized triple, since a proof may pass through a
    literal subject that the output never shows.

  The JSON is that list, close to RDFox's and Nemo's.
- **The text tree.** Each line is a triple in prefixed form with
  `[rule]` or `asserted`. A shared subproof is printed once with a number,
  and later uses read "see [3]". Several things fold by default:
  - a list (§5);
  - an equality chain (§4.2);
  - a run of one transitive rule, flattened into a chain: `ex:A ⊑ ex:B ⊑
    ex:C ⊑ ex:D [scm-sco ×2]`;
  - a schema-only subproof, all of whose facts are schema triples,
    collapsed to one line, in the spirit of Evonne's condensation;
  - anything past a depth, shown as Soufflé's `setdepth` does.

  An example:

  ```
  ex:alice rdf:type ex:Person                      [cax-sco]
  ├── ex:alice rdf:type ex:Student                 [prp-dom]
  │   ├── ex:alice ex:enrolledIn ex:course1        asserted
  │   └── ex:enrolledIn rdfs:domain ex:Student     asserted
  └── ex:Student rdfs:subClassOf ex:Person         [scm-sco ×1, schema]
  ```
- **The interface**, added only as each adapter needs it (`AGENTS.md`):
  - `Reasoner::explain(triple) -> Option<Proof>`, `None` when the triple is
    not in the closure;
  - `Reasoner::explain_diagnostic(&Diagnostic) -> Proof`;
  - the CLI's `turbowl explain` with text and `--json`;
  - Python's `Reasoner.explain`.

## Open questions

1. **How large are balls on real data?** Measure them for random inferred
   triples on brick-small1, brick-buildings and lubm-50, then set the
   radius and instance limits. Measure before claiming "milliseconds".
2. **Should the reference evaluator get a single-rule step and
   first-round numbers?** §7's step and depth checks need them. That is a
   change to `crates/reference`, outside this note.
3. **One backward matcher for FBF and explanations?** FBF works in
   representatives inside a run; explanations work in the user's terms
   after one. Is one matcher, with the lifting to the user's terms as a
   layer, simpler than two? Decide when ticket 23 starts.
4. **Should editors also get GraphDB's view**, every rule instance one
   level down (RDFox's `to-explicit`)? The ball already holds them.
5. **Should the run keep a proof forest beside the canon?** This would be
   behind a switch, for equality-heavy users, and the goal forbids it as
   the default (§4.2).
6. **Are proofs through generalized triples acceptable to users,** or
   should the text form explain the step around them?

## Sources read

Code (read, never built or run):

- Nemo, `knowsys/nemo` at `7706c48`: `nemo/src/execution/execution_engine.rs`,
  `nemo/src/table_manager.rs`, `nemo/src/execution/execution_engine/tracing/`
  (`simple.rs`, `simple/storage.rs`, `tree_query.rs`, `node_query.rs`),
  `nemo/src/execution/tracing/` (`trace.rs`, `tree_query.rs`,
  `node_query.rs`, `resolve_origin.rs`), `nemo/src/execution/planning/`,
  `nemo-cli/src/{cli,main,tracing}.rs`, `nemo-python/src/lib.rs`,
  `nemo-wasm/src/lib.rs`, <https://github.com/knowsys/nemo>; issues and PRs
  #363, #418, #469, #677, #697, #701, #774, #816–#822; `knowsys/nemo-web`
  `fad0533`; `knowsys/nemo-doc` `1a69a04`.
- Soufflé, `souffle-lang/souffle` at `2adf8c0`: `src/MainDriver.cpp`,
  `src/include/souffle/provenance/Explain.h`,
  `src/include/souffle/provenance/ExplainProvenanceImpl.h`,
  `src/ast2ram/provenance/SubproofGenerator.cpp`,
  `src/ast/transform/ExpandEqrels.cpp`, `tests/provenance/`,
  <https://github.com/souffle-lang/souffle>.
- GraphDB proof plugin, `Ontotext-AD/graphdb-proof-plugin` at `9ce7cb4`:
  `src/main/java/com/ontotext/trree/plugin/proof/ProofPlugin.java`,
  <https://github.com/Ontotext-AD/graphdb-proof-plugin>.
- puli, <https://github.com/liveontologies/puli>: `AcyclicProofNode.java`,
  `InferenceDerivabilityChecker.java`.
- ELK, <https://github.com/liveontologies/elk-reasoner>: `TestUtils.java`,
  `ProofTestUtils.java`, `ProofCompletenessTest.java`, `ProofTest.java`.
- owlexplanation, <https://github.com/matthewhorridge/owlexplanation>:
  `ConsistencyEntailmentChecker.java`,
  `InconsistentOntologyExplanationGeneratorFactory.java`,
  `BlackBoxExplanationGenerator2_TestCase.java`.
- Evonne, `frontend/public/js/proof/data/process-data.js` (its input
  format; whether it is exactly GraphML is unverified).

Documentation (read 2026-11-07):

- RDFox 7.7: <https://docs.oxfordsemantic.tech/rdfox-shell.html#explain>,
  <https://docs.oxfordsemantic.tech/reasoning.html#explaining-reasoning-results>,
  <https://docs.oxfordsemantic.tech/apis.html#explaining-fact-derivation>,
  <https://docs.oxfordsemantic.tech/data-stores.html#equality>.
- Soufflé: <https://souffle-lang.github.io/provenance>.
- GraphDB 11.5:
  <https://graphdb.ontotext.com/documentation/11.5/inference.html#proof-plugin>,
  <https://graphdb.ontotext.com/documentation/11.5/reasoning.html#retraction-of-assertions>.
- Stardog:
  <https://docs.stardog.com/stardog-cli-reference/reasoning/reasoning-explain>,
  <https://docs.stardog.com/inference-engine/>,
  <https://docs.stardog.com/inference-engine/advanced-reasoning-features>,
  <https://docs.stardog.com/javadoc/snarl/com/complexible/stardog/reasoning/StardogExplainer.html>.
- egg 0.11.0: <https://docs.rs/egg/0.11.0/egg/struct.EGraph.html>.
- W3C, OWL 2 Web Ontology Language Profiles (Second Edition), §4.3,
  <https://www.w3.org/TR/owl2-profiles/>.

Papers (full text unless marked):

- Ivliev, Gerlach, Meusel, Steinberg, Krötzsch, "Nemo: Your Friendly and
  Versatile Rule Reasoning Toolkit", KR 2024,
  <https://iccl.inf.tu-dresden.de/w/images/f/fb/KR-2024-CR.pdf>.
- Ivliev et al., "Nemo: First Glimpse of a New Rule Engine", ICLP 2023
  (no mention of tracing), <https://arxiv.org/abs/2308.15897>.
- Gerlach, Ivliev, Méndez, Meusel, Dachselt, Krötzsch, "EvonNemo",
  XLoKR 2024,
  <https://iccl.inf.tu-dresden.de/w/images/5/56/2024-XLoKR-Submission-5-camera-ready.pdf>.
- Elhalawati, Krötzsch, Mennicke, "An Existential Rule Framework for
  Computing Why-Provenance On-Demand for Datalog", RuleML+RR 2022 (link in
  §3.5).
- Zhao, Subotić, Scholz, "Debugging Large-scale Datalog: A Scalable
  Provenance Evaluation Strategy", ACM TOPLAS 42(2), 2020,
  <https://souffle-lang.github.io/pdf/toplas20.pdf>; arXiv version
  <https://arxiv.org/abs/1907.05045>.
- Motik, Nenov, Piro, Horrocks, "Handling owl:sameAs via Rewriting",
  AAAI 2015, <https://arxiv.org/abs/1411.3622>.
- Motik, Nenov, Piro, Horrocks, "Combining Rewriting and Incremental
  Materialisation Maintenance for Datalog Programs with Equality",
  IJCAI 2015,
  <https://www.cs.ox.ac.uk/boris.motik/pubs/mnph15incremental-BF-sameAs.pdf>.
- Motik, Nenov, Piro, Horrocks, "Maintenance of datalog materialisations
  revisited", Artif. Intell. 269 (2019),
  <https://www.cs.ox.ac.uk/boris.motik/pubs/mnph19maintenance-revisited.pdf>.
- Kazakov, Klinov, "Goal-Directed Tracing of Inferences in EL
  Ontologies", ISWC 2014,
  <https://rd.springer.com/content/pdf/10.1007/978-3-319-11915-1_13.pdf>.
- Kazakov, Klinov, Stupnikov, "Towards Reusable Explanation Services in
  Protégé", DL 2017, <https://www.ceur-ws.org/Vol-1879/paper31.pdf>.
- Kazakov, Klinov, "Advancing ELK", DL 2015,
  <https://www.ceur-ws.org/Vol-1350/paper-27.pdf>.
- Calautti, Livshits, Pieris, Schneider, "The Complexity of
  Why-Provenance for Datalog Queries", <https://arxiv.org/abs/2303.12773>
  (v1).
- Deutch, Gilad, Moskovitch, "Selective provenance for Datalog programs
  using top-k queries", PVLDB 8(12), 2015 (link in §3.5).
- Alrabbaa, Baader, Borgwardt, Koopmann, Kovtunova, "Finding Small Proofs
  for Description Logic Entailments: Theory and Practice", LPAR 2020,
  <https://arxiv.org/abs/2004.08311>.
- Alrabbaa, Baader, Borgwardt, Koopmann, Kovtunova, "Finding Good Proofs
  for Description Logic Entailments Using Recursive Quality Measures",
  CADE 2021, <https://arxiv.org/abs/2104.13138>.
- Alrabbaa et al., "Evonne: Interactive Proof Visualization for
  Description Logics", IJCAR 2022, <https://arxiv.org/abs/2205.09583>.
- Horridge, "Justification Based Explanation in Ontologies", PhD thesis,
  Manchester 2011,
  <https://www.bcs.org/media/2146/dd-2012-matthew-horridge.pdf>.
- Horridge, Parsia, Sattler, "Laconic and Precise Justifications in OWL",
  ISWC 2008 (link in §8).
- Flatt, Coward, Willsey, Tatlock, Panchekha, "Small Proofs from
  Congruence Closure", FMCAD 2022, <https://arxiv.org/abs/2209.03398>.
- Nieuwenhuis, Oliveras, "Proof-Producing Congruence Closure", RTA 2005,
  <https://www.cs.upc.edu/~roberto/papers/rta05.pdf> (abstract and
  secondary descriptions only; the explanation mechanism is as the FMCAD
  paper describes it).

Cited from memory, not fetched (unverified): Dowling and Gallier, "Linear-time
algorithms for testing the satisfiability of propositional Horn formulae",
J. Logic Programming 1(3), 1984; Knuth, "A generalization of Dijkstra's
algorithm", Inf. Process. Lett. 6(1), 1977; Bancilhon, Maier, Sagiv,
Ullman, "Magic sets and other strange ways to implement logic programs",
PODS 1986; Beeri and Ramakrishnan, "On the power of magic", J. Logic
Programming 1991; Bourgaux et al., KR 2022 (the origin of the proof-tree
classes Calautti et al. use).
