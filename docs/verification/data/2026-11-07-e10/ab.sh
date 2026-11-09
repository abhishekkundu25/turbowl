#!/bin/bash
set -u
OUT=<e10>/out
mkdir -p $OUT
B=<e10>/before/target/release/turbowl-bench
A=<repo>/target/release/turbowl-bench
run() { # side bin threads i
  echo "$1 $3 $4 $(sysctl -n vm.loadavg)" >> $OUT/load.txt
  "$2" --engines turbowl --runs 1 --only lubm-50 --threads $3 --json $OUT/$1-$3-$4.json > $OUT/$1-$3-$4.txt 2>&1
}
for i in 1 2 3 4 5 6 7 8 9 10; do
  if [ $((i % 2)) = 1 ]; then run before $B all $i; run after $A all $i; else run after $A all $i; run before $B all $i; fi
done
for i in 1 2 3 4 5; do
  if [ $((i % 2)) = 1 ]; then run before $B one $i; run after $A one $i; else run after $A one $i; run before $B one $i; fi
done
echo done
