import re,sys,collections
for f in sys.argv[1:]:
    rnd=0; agg=collections.defaultdict(lambda: collections.Counter())
    for line in open(f):
        line=line[line.find('E11'):] if 'E11' in line else ''
        if line.startswith('E11 property round start'): rnd+=1; continue
        m=re.match(r'E11S (\S+):(\d+) len=(\d+) chunks=(\d+)(.*)',line)
        if not m or rnd==0: continue
        fam=m.group(1).split('.')[0]; ln=int(m.group(3)); ch=int(m.group(4)); rest=m.group(5)
        kv=dict(re.findall(r'(\w+)=([\d.]+)',rest))
        a=agg[(rnd,fam)]
        a['splits']+=1; a['pairs']+=ln; a['chunks']+=ch
        if 'inline' in rest: a['inline']+=1
        a['wall']+=float(kv['wall_us'])
        if 'emit_sum_us' in kv:
            a['emit']+=float(kv['emit_sum_us']); a['compact']+=float(kv['compact_sum_us'])
            a['late']+=float(kv['wall_us'])-float(kv['last_end_us']); a['start']+=float(kv['last_start_us'])
    print('==',f.split('/')[-1])
    for (r,fam),a in sorted(agg.items()):
        if r>3: continue
        print(f"  round {r} {fam:9} splits {a['splits']:3} (inline {a['inline']:2}) pairs {a['pairs']:8} chunks {a['chunks']:4}  wall {a['wall']/1e3:6.2f} ms  emit_cpu {a['emit']/1e3:6.2f}  compact_cpu {a['compact']/1e3:6.2f}  last_chunk_start {a['start']/1e3:5.2f}  after_last_chunk {a['late']/1e3:5.2f}")
