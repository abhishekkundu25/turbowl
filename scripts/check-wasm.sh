#!/usr/bin/env bash
# The core builds for wasm32-unknown-unknown without the io feature and
# without the parallel one (WebAssembly has no threads: rounds run on the
# caller's thread, and rayon is not even a dependency there), with every
# rule family (property chains included: a switch, not a feature).
# oxrdf pulls in getrandom, which needs a backend on this target; a library
# does not pick one for its embedder, so this check uses the "custom"
# backend, as turtle-lsp's browser engine does (it defines
# __getrandom_v03_custom). An embedder may use "wasm_js" instead.
set -euo pipefail
cd "$(dirname "$0")/.."
RUSTFLAGS='--cfg getrandom_backend="custom"' \
  cargo build -p turbowl --no-default-features --target wasm32-unknown-unknown --release "$@"
