import json, sys, os, statistics as st
out = sys.argv[1]
loads = {}
for line in open(os.path.join(out, 'load.txt')):
    side, th, i, rest = line.split(' ', 3)
    loads[(side, th, int(i))] = rest.strip().strip('{} ').split()[0]
med = {}
counts = set()
for th, n in (('all', 10), ('one', 5)):
    for side in ('before', 'after'):
        rs, ps, steps, step2, stepsum = [], [], [], [], []
        print(f'== {side} {th}')
        for i in range(1, n + 1):
            f = os.path.join(out, f'{side}-{th}-{i}.json')
            if not os.path.exists(f):
                continue
            r = json.load(open(f))['results'][0]['runs'][0]
            p = r['profile']
            rs.append(r['reason_ms']); ps.append(r['peak_rss_bytes'] / 1e6)
            sm = p['steps']['ms']
            counts.add(json.dumps([r['closure'], p['steps']['pairs'], p['steps']['new'], p['rounds']['delta'], p['rounds']['emitted']]))
            step2.append(sm[1]); stepsum.append(sum(sm))
            print(f"  run {i:2}: step2 {sm[1]:5.1f} steps {sum(sm):5.1f}  reason {r['reason_ms']:7.1f} ms  peak {r['peak_rss_bytes']/1e6:6.1f} MB  load1 {loads.get((side, th, i))}  closure {r['closure']}  step_ms {p['step_ms']:.1f} sort_ms {p['sort_ms']:.1f} rules_ms {p['rules_ms']:.1f}")
        if rs:
            med[(side, th)] = st.median(rs)
            print(f'  median reason {st.median(rs):.1f} ms (min {min(rs):.1f} max {max(rs):.1f})  median peak {st.median(ps):.1f} MB  largest {max(ps):.1f} MB  median step2 {st.median(step2):.1f} ms  median steps {st.median(stepsum):.1f} ms')
for side in ('before', 'after'):
    if (side, 'all') in med and (side, 'one') in med:
        print(f"{side}: one/all = {med[(side,'one')]:.1f}/{med[(side,'all')]:.1f} = {med[(side,'one')]/med[(side,'all')]:.2f} times")
print('distinct (closure, step pairs/new, round delta/emitted) across all runs:', len(counts))
