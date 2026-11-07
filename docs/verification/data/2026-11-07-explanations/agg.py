"""Aggregate the alternating main/explain bench: median and middle half per side."""
import glob, json, os, statistics, sys

here = os.path.join(os.path.dirname(os.path.abspath(__file__)), "bench")

def quart(v):
    v = sorted(v)
    n = len(v)
    return v[n // 4], statistics.median(v), v[(3 * n - 1) // 4]

rows = {}
for f in glob.glob(os.path.join(here, "*-*-*-*.json")):
    cfg, th, side, _ = os.path.basename(f)[:-5].split("-")
    for r in json.load(open(f))["results"]:
        for run in r.get("runs") or []:
            if run.get("status") != "ok":
                continue
            k = (cfg, th, r["scenario"])
            d = rows.setdefault(k, {}).setdefault(side, {"load": [], "reason": [], "total": [], "mem": []})
            d["load"].append(run["load_ms"])
            d["reason"].append(run["reason_ms"])
            d["total"].append(run["load_ms"] + run["reason_ms"])
            d["mem"].append(run["peak_rss_bytes"] / 2**20)

print("| configuration | threads | scenario | measure | main median [middle half] | explain median [middle half] | delta | within main's spread |")
print("|---|---|---|---|---|---|---|---|")
for k in sorted(rows):
    m, e = rows[k].get("main"), rows[k].get("explain")
    if not m or not e:
        continue
    for meas, unit in [("load", "ms"), ("reason", "ms"), ("total", "ms"), ("mem", "MiB")]:
        ml, mm, mh = quart(m[meas]); el, em, eh = quart(e[meas])
        lo, hi = min(m[meas]), max(m[meas])
        within = "yes" if lo <= em <= hi else "no"
        print(f"| {k[0]} | {k[1]} | {k[2]} | {meas} ({unit}) | {mm:.2f} [{ml:.2f}, {mh:.2f}] | {em:.2f} [{el:.2f}, {eh:.2f}] | {100*(em-mm)/mm:+.1f}% | {within} |")
