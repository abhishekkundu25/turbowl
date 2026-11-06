#!/usr/bin/env bash
# Runs the probe on lubm-50, alternating the sides, each run its own
# process, and saves each output as out/probe-<side>-<threads>-<i>.txt.
#
# Usage: run.sh <runs> <threads: all|one> <data dir> <side>...
#   data dir  holds univ-bench.ttl and lubm-v1-50.ttl (the bench's
#             target/bench-data after one lubm-50 bench run)
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
runs="$1" threads="$2" data="$3"; shift 3
mkdir -p "$here/out"
for i in $(seq 1 "$runs"); do
  for side in "$@"; do
    out="$here/out/probe-$side-$threads-$i.txt"
    load="$(sysctl -n vm.loadavg)"
    {
      echo "== probe $side $threads run $i, load $load"
      "$here/work/$side/probe/target/release/turbowl-probe" "$threads" \
        "$data/univ-bench.ttl" "$data/lubm-v1-50.ttl"
    } > "$out"
    sed -n '1,4p' "$out" | tail -2
  done
done
