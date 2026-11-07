# 01: A triple term is an opaque term

**What to build:** TurbOWL compiles and reasons correctly when oxrdf is
built with its `rdf-12` feature, which is how turtle-lsp builds it. An RDF
1.2 triple term (`<<( s p o )>>`) is one more kind of term, interned like a
literal: a rule may pass it along as a whole (for example as the object of
`rdf:reifies`, or through `eq-rep-o`), but no rule looks inside it, and the
triple inside is not asserted. With `rdf-12` off, nothing changes: the same
code, the same closure, the same speed.

Two triple terms are the same term exactly when oxrdf says the terms are
equal. Blank nodes inside a triple term belong to the triple's document, as
other blank nodes do. A proof that passes through a triple term shows it as
the term it is (`Statement` already holds `oxrdf::Term`).

**Blocked by:** None (can start immediately).

**Status:** todo

- [x] TurbOWL has an `rdf-12` feature that turns on `oxrdf/rdf-12`, so its
      own tests build the way turtle-lsp builds it; the crate doc's feature
      list names it.
- [x] `cargo build -p turbowl --no-default-features --features rdf-12`
      and the default build both compile with no warnings; clippy is clean
      on both.
- [x] The wasm32 check builds with and without `rdf-12`.
- [x] Every `match` on an oxrdf term or subject in `crates/turbowl/src`
      handles triple terms without a wildcard arm, so a new oxrdf variant is
      a compile error again.
- [x] New tests, run with `--features rdf-12`: a triple term survives
      `set_base`, `reason()` and `inferred()` unchanged; `eq-rep-o` carries
      a triple term across `owl:sameAs`; a triple term with a blank node in
      two documents gives two different terms; `explain` of an inferred
      triple whose object is a triple term returns a proof the reference
      checker (`turbowl_reference::check_proof`) accepts; a triple term
      never makes the triple inside it part of the closure.
- [ ] `timeout 120 cargo test --release --locked -p turbowl` and
      `-p turbowl-oracle` pass, with and without `rdf-12`.
- [ ] The reasoning benchmark is unchanged with `rdf-12` off (the
      orchestrator measures it on a quiet machine; the slice does not run
      it).
- [x] `CHANGELOG.md` has the entry.
