"""Summarizes the probe's outputs, side by side.

Usage: python3 -I summarize.py <out dir> <threads> <side>...

For each side and run: the lifetime peak at the end of reasoning, the
reasoning time, the most in use at a mark; each first index build of more
than 100,000 pairs (its table, size, and the footprint, lifetime peak and
bytes in use it added, and its time); each index sort of more than 100,000
pairs (same); and each step's and round's footprint growth.
"""
import pathlib
import re
import statistics
import sys

out, threads, sides = pathlib.Path(sys.argv[1]), sys.argv[2], sys.argv[3:]


def section(text, title):
    m = re.search(rf"^{title}.*?\n(.*?)\n\n", text, re.S | re.M)
    rows = m.group(1).splitlines()[1:] if m else []
    return [r.split() for r in rows]


summary = {}
for side in sides:
    files = sorted(out.glob(f"probe-{side}-{threads}-*.txt"), key=lambda p: int(p.stem.rsplit("-", 1)[1]))
    print(f"=== {side} ({len(files)} runs)")
    peaks, times, builds, sorts, phases = [], [], {}, {}, {}
    for f in files:
        text = f.read_text()
        load = re.search(r"load (.*)", text).group(1)
        peak = float(re.search(r"after reason: .*?lifetime peak ([\d.]+) MB", text).group(1))
        took = float(re.search(r"reason took ([\d.]+) ms", text).group(1))
        used = float(re.search(r"most in use at a mark: ([\d.]+) MB", text).group(1))
        peaks.append(peak)
        times.append(took)
        print(f"{f.name}: lifetime peak {peak:.1f} MB, reason {took:.1f} ms, most in use {used:.1f} MB, load {load}")
        for t, n, _mb, _at, foot, life, used_, _alloc, ms in section(text, "INDEX BUILDS"):
            if int(n) > 100_000:
                builds.setdefault((t, n), []).append((float(foot), float(life), float(used_), float(ms)))
        k = 0
        for t, n, _at, foot, life, used_, ms in section(text, "INDEX SORTS"):
            if int(n) > 100_000:
                k += 1
                sorts.setdefault((k, t, n), []).append((float(foot), float(life), float(used_), float(ms)))
        for row in section(text, "PHASES"):
            name = " ".join(row[:2])
            phases.setdefault(name, []).append(float(row[5]))
    print(f"  median lifetime peak {statistics.median(peaks):.1f} MB, median reason {statistics.median(times):.1f} ms")
    print("  first index builds (table, pairs): +foot, +life, +used, ms, mean of runs")
    for (t, n), rows in builds.items():
        means = [statistics.mean(c) for c in zip(*rows)]
        print(f"    t{t} {n:>8}: +foot {means[0]:5.1f}  +life {means[1]:5.1f}  +used {means[2]:5.1f}  {means[3]:5.2f} ms")
    print("  index sorts (in order, table, pairs): +foot, +life, +used, ms, mean of runs")
    for (k, t, n), rows in sorts.items():
        means = [statistics.mean(c) for c in zip(*rows)]
        print(f"    #{k} t{t} {n:>8}: +foot {means[0]:5.1f}  +life {means[1]:5.1f}  +used {means[2]:5.1f}  {means[3]:5.2f} ms")
    print("  phases: +foot, mean of runs")
    print("    " + ", ".join(f"{name} {statistics.mean(v):+.1f}" for name, v in phases.items()))
