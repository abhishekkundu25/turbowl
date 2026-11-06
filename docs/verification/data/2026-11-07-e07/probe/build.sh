#!/usr/bin/env bash
# Builds the probe against one version of TurbOWL.
#
# Usage: build.sh <name> <repo> <commit> [patch]
#   name    a label for this side, e.g. before or after
#   repo    a TurbOWL git checkout (read only: `git archive` reads it)
#   commit  the commit to probe
#   patch   optional: a diff applied on top (the change under test)
#
# Makes work/<name>/tree (the source, with the marks of insert_marks.py)
# and work/<name>/probe (the probe crate), and builds
# work/<name>/probe/target/release/turbowl-probe.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
name="$1" repo="$2" commit="$3" patch="${4:-}"
work="$here/work/$name"
rm -rf "$work"
mkdir -p "$work/tree" "$work/probe/src"
git -C "$repo" archive "$commit" crates Cargo.toml Cargo.lock rust-toolchain.toml | tar -x -C "$work/tree"
if [ -n "$patch" ]; then
  (cd "$work/tree" && patch -p1 --quiet < "$patch")
fi
python3 -I "$here/insert_marks.py" "$work/tree"
sed "s|@TREE@|$work/tree|" "$here/crate/Cargo.toml.in" > "$work/probe/Cargo.toml"
cp "$here/crate/src/main.rs" "$work/probe/src/main.rs"
cp "$work/tree/Cargo.lock" "$work/tree/rust-toolchain.toml" "$work/probe/"
(cd "$work/probe" && cargo build --release --offline 2>&1 | tail -2)
echo "built $work/probe/target/release/turbowl-probe"
