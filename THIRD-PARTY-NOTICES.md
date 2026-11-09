# Third-party notices

Parts of TurbOWL are ported from other projects. Those parts keep their
authors' copyright and license; this file carries the notices their
licenses require.

## Oxigraph: oxttl 0.2.5

- Source: <https://github.com/oxigraph/oxigraph> (`lib/oxttl`).
- License: MIT OR Apache-2.0. TurbOWL uses the ported code under the MIT
  license, whose text follows.
- Ported into:
  - `crates/turbowl/src/io/lexer.rs`: oxttl's N-Triples and Turtle lexer.
  - `crates/turbowl/src/io/turtle.rs`: oxttl's Turtle recognizer.
  - `crates/turbowl/src/io/ntriples.rs`: oxttl's N-Triples recognizer.

```text
Copyright (c) 2018 Oxigraph developers

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

## datafrog 2.0.1

- Source: <https://github.com/rust-lang/datafrog>.
- License: MIT OR Apache-2.0. TurbOWL uses the adapted code under the MIT
  license, whose text follows.
- Adapted into:
  - `crates/turbowl/src/engine/relation.rs`: `gallop`, from datafrog's
    `src/join.rs`.
  - `crates/turbowl/src/engine/leapjoin.rs`: the leaper interface (`count`,
    `propose`, `intersect`), after datafrog's `src/treefrog.rs`.

```text
Copyright (c) Frank McSherry, The Rust Project Developers and the Datafrog
Developers

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

## LUBM and the univ-bench ontology

The benchmark's LUBM-style generator (`crates/bench/src/lubm.rs`) writes out
the univ-bench ontology's axioms by hand and follows the published counts
of the Lehigh University Benchmark by Yuanbo Guo, Zhengxiang Pan and Jeff
Heflin (Lehigh University). It contains no code from the benchmark's data
generator (UBA).

