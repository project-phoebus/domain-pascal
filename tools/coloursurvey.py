#!/usr/bin/env python3
"""coloursurvey.py DIS [NAMES] - could a colouring order explain the data
registers of a per-routine disassembly: the image's (tools/alldis.py SAU CPU
OUT) or dpc's (tools/objdis.py OBJ...), restricted to the routine names in
the file NAMES when given.  docs/register-model.md, "Step 4: the register a
new value takes".

Builds webs from real reaching definitions and liveness over the routine's
branches (no `jmp` through a table, at most 700 instructions), colours them
lowest-first in several orders, and scores each order against the registers
used.  Webs that stay put: a call's result, and an unread write of D0 (a
function's result).  A value dying where another is defined does not
interfere with it (a load through a base, 12.4 R2); a copy's two sides do.

Then, per decision in first-definition order, given the registers actually
used by earlier webs: was it the lowest free one and, if not, was that
register taken by an overlapping later value (an order could explain it) or
free over the whole range (no order could)?

dpc is the control: its allocator is lowest-free in first-definition order,
so that order must score far above the others on it.

COLOUR_RANGES widens the ranges, to test whether the allocator saw more than
liveness: i = first to last point; s = on to the next store, branch or call;
l = through any loop touched.
"""
import collections
import os
import random
import re
import sys

import cachesurvey as cs
import getregsurvey as g

rs=cs.parse(sys.argv[1]); keep=set(open(sys.argv[2]).read().split()) if len(sys.argv)>2 else None
D=g.DREGS
def analyse(ins):
    n=len(ins); idx={x[0]:i for i,x in enumerate(ins)}
    succ=[[] for _ in range(n)]; rw=[]
    for k,(a,m,o) in enumerate(ins):
        r,w=g.reads_writes(m,o)
        r={x for x in r if x[1]=='d'}; w={x for x in w if x[1]=='d'}
        # clr then a partial load: one value
        if k and m in('movew','moveb') and w:
            pm,po=ins[k-1][1],ins[k-1][2]
            if pm.startswith('clr') and po in w: r=r|w
        if m=='jsr' or m.startswith('bsr'):
            r=set(); w={'%d0','%d1'}
        rw.append((r,w))
        if m=='rts': continue
        tgt=None
        if g.BRANCH.match(m) or m.startswith('db'):
            t=o.split(',')[-1].strip().split(' ')[0]
            if re.fullmatch(r'(0x)?[0-9a-f]+',t): tgt=idx.get(int(t,16))
            if tgt is None: return None
            succ[k].append(tgt)
        if m in('bras','braw','bra'): continue
        if m=='jmp': return None
        if k+1<n: succ[k].append(k+1)
    # reaching definitions per register: bit per def
    defs=[(k,r) for k,(_,w) in enumerate(rw) for r in w]
    did={d:i for i,d in enumerate(defs)}
    byreg=collections.defaultdict(int)
    for (k,r),i in did.items(): byreg[r]|=1<<i
    pred=[[] for _ in range(n)]
    for k,ss in enumerate(succ):
        for s in ss: pred[s].append(k)
    IN=[0]*n; OUT=[0]*n; ch=True
    while ch:
        ch=False
        for k in range(n):
            i=0
            for p in pred[k]: i|=OUT[p]
            o=i
            for r in rw[k][1]:
                o&=~byreg[r]; o|=1<<did[(k,r)]
            if i!=IN[k] or o!=OUT[k]: IN[k]=i; OUT[k]=o; ch=True
    # webs
    par=list(range(len(defs)))
    def find(x):
        while par[x]!=x: par[x]=par[par[x]]; x=par[x]
        return x
    uses=collections.Counter()
    for k,(r0,_) in enumerate(rw):
        for r in r0:
            ds=[i for i in range(len(defs)) if IN[k]>>i&1 and defs[i][1]==r]
            for i in ds[1:]: par[find(i)]=find(ds[0])
            for i in ds: uses[i]+=1
    webs=collections.defaultdict(list)
    for i in range(len(defs)): webs[find(i)].append(i)
    # liveness per register, then per web: points where a def of the web reaches and the register is live
    livein=[set() for _ in range(n)]; ch=True
    while ch:
        ch=False
        for k in range(n-1,-1,-1):
            o=set()
            for s in succ[k]: o|=livein[s]
            i=(o-rw[k][1])|rw[k][0]
            if i!=livein[k]: livein[k]=i; ch=True
    W=[]
    for root,ids in webs.items():
        r=defs[ids[0]][1]; mask=0
        for i in ids: mask|=1<<i
        # a value occupies its register after its definition and after each point it stays live:
        # a value dying at k and one defined at k do not overlap (a load through a base, 12.4 R2)
        pts=set(defs[i][0] for i in ids); ncall=False; u=sum(uses[i] for i in ids)
        for k in range(n):
            if OUT[k]&mask and any(r in livein[s] for s in succ[k]): pts.add(k)
        for k in pts:
            m=ins[k][1]
            if (m=='jsr' or m.startswith('bsr')) and any(s<n and r in livein[s] for s in succ[k]) and not any(defs[i][0]==k for i in ids): ncall=True
        fromcall=any(ins[defs[i][0]][1]=='jsr' or ins[defs[i][0]][1].startswith('bsr') for i in ids)
        # coarser ranges (MODE): what if the allocator saw more than liveness?
        mode=os.environ.get('COLOUR_RANGES','')
        if pts and mode:
            lo,hi=min(pts),max(pts)
            def boundary(j):
                m,o=ins[j][1],ins[j][2]
                if g.BRANCH.match(m) or m.startswith('db') or m=='jsr' or m.startswith('bsr') or m=='rts': return True
                ops=cs.split_ops(o) if o else []
                return len(ops)==2 and not re.fullmatch(r'%[ad][0-7]',ops[1]) and not m.startswith(('cmp','tst','btst','pea'))
            if 'i' in mode:
                pts=set(range(lo,hi+1))
            if 's' in mode:
                j=hi
                while j<n-1 and not boundary(j): j+=1
                pts|=set(range(hi,j+1))
            if 'l' in mode:
                # to the end of any loop the range touches
                for k,ss in enumerate(succ):
                    for t in ss:
                        if t<k and t<=hi and lo<=k:
                            pts|=set(range(t,k+1))
        # the instructions that read this value as a plain register source of a copy
        copies=set()
        for k,(r0,_) in enumerate(rw):
            if r in r0 and IN[k]&mask and ins[k][1] in('movel','movew','moveb'):
                ops=cs.split_ops(ins[k][2])
                if len(ops)==2 and ops[0]==r and re.fullmatch(r'%d[0-7]',ops[1]): copies.add(k)
        W.append(dict(copies=copies,pts_defs={defs[i][0] for i in ids},reg=r,pts=pts,first=min(pts),last=max(pts),uses=u,cross=ncall,fix=fromcall or (u==0 and r=='%d0'),dead=u==0,ndefs=len(ids)))
    return [w for w in W if not (w['dead'] and not w['fix'])]
def adjacency(W):
    adj={i:{j for j in range(len(W)) if j!=i and W[i]['pts']&W[j]['pts']} for i in range(len(W))}
    for i,w in enumerate(W):
        for j,x in enumerate(W):
            if i!=j and w['copies']&x['pts_defs']: adj[i].add(j); adj[j].add(i)
    return adj
def colour(W,order):
    col={}
    adj=adjacency(W)
    for i,w in enumerate(W):
        if w['fix']: col[i]=w['reg']
    for i in order:
        if i in col: continue
        t={col[j] for j in adj[i] if j in col}
        for r in D:
            if r in t: continue
            if W[i]['cross'] and r in('%d0','%d1'): continue
            col[i]=r; break
        else: col[i]=None
    return col
def simplify(W):
    adj=adjacency(W)
    left=set(range(len(W))); st=[]
    while left:
        i=min(left,key=lambda x:(len(adj[x]&left),x)); st.append(i); left.discard(i)
    return st[::-1]
ORDERS={
 'first def':lambda W:sorted(range(len(W)),key=lambda i:W[i]['first']),
 'first def, latest first':lambda W:sorted(range(len(W)),key=lambda i:-W[i]['first']),
 'last use':lambda W:sorted(range(len(W)),key=lambda i:W[i]['last']),
 'longest first':lambda W:sorted(range(len(W)),key=lambda i:(-len(W[i]['pts']),W[i]['first'])),
 'shortest first':lambda W:sorted(range(len(W)),key=lambda i:(len(W[i]['pts']),W[i]['first'])),
 'most uses first':lambda W:sorted(range(len(W)),key=lambda i:(-W[i]['uses'],W[i]['first'])),
 'fewest uses first':lambda W:sorted(range(len(W)),key=lambda i:(W[i]['uses'],W[i]['first'])),
 'most defs first':lambda W:sorted(range(len(W)),key=lambda i:(-W[i]['ndefs'],W[i]['first'])),
 'cross calls, then first def':lambda W:sorted(range(len(W)),key=lambda i:(not W[i]['cross'],W[i]['first'])),
 'cross calls, then longest':lambda W:sorted(range(len(W)),key=lambda i:(not W[i]['cross'],-len(W[i]['pts']),W[i]['first'])),
 'simplify (Chaitin)':simplify,
 'random':lambda W:random.Random(1).sample(range(len(W)),len(W)),
}
hit=collections.Counter(); tot=0; hitf=collections.Counter(); totf=0; whole=collections.Counter(); rout=0; skipped=0
for name,ins0 in rs.items():
    if keep is not None and name not in keep: continue
    if not ins0 or ins0[0][1]!='linkw': continue
    ins=g.cut(ins0)
    if len(ins)>700: skipped+=1; continue
    W=analyse(ins)
    if W is None: skipped+=1; continue
    free=[i for i,w in enumerate(W) if not w['fix']]
    if not free: continue
    rout+=1; tot+=len(free)
    for o,f in ORDERS.items():
        c=colour(W,f(W))
        h=sum(c[i]==W[i]['reg'] for i in free); hit[o]+=h
        whole[o]+=h==len(free)
print('routines',rout,'skipped',skipped,'webs not fixed',tot)
for o in sorted(ORDERS,key=lambda o:-hit[o]): print('%-30s %6d  %.3f   whole routine %d'%(o,hit[o],hit[o]/tot,whole[o]))

# --- per-decision: in first-def order, given the ACTUAL colours of earlier webs
T=collections.Counter(); conf=collections.Counter(); why=collections.Counter(); ex=collections.defaultdict(list)
for name,ins0 in rs.items():
    if keep is not None and name not in keep: continue
    if not ins0 or ins0[0][1]!='linkw': continue
    ins=g.cut(ins0)
    if len(ins)>700: continue
    W=analyse(ins)
    if W is None: continue
    adj=adjacency(W)
    order=sorted(range(len(W)),key=lambda i:W[i]['first'])
    used=set()
    for i in order:
        w=W[i]
        if w['fix']: used.add(i); continue
        earlier={W[j]['reg'] for j in adj[i] if j in used or W[j]['fix']}
        pred=next((r for r in D if r not in earlier and not (w['cross'] and r in('%d0','%d1'))),None)
        used.add(i); T['decisions']+=1
        if pred==w['reg']: T['lowest free among earlier']+=1; continue
        if pred is None: why['no register free (widened ranges)']+=1; continue
        later=[j for j in adj[i] if W[j]['reg']==pred and j not in used and not W[j]['fix']]
        conf[(pred,w['reg'])]+=1
        a=format(ins[w['first']][0],'x') if isinstance(ins[w['first']][0],int) else ins[w['first']][0]
        if later: k='lowest is taken by an overlapping later value (order)'
        elif w['reg']<pred: k='chose LOWER than lowest free?'
        else: k='lowest free over the whole range, passed over'
        why[k]+=1; ex[k].append('%s %s %s->%s'%(name,a,pred,w['reg']))
print(); print(dict(T))
for k,v in why.most_common(): print('%-58s %5d  %s'%(k,v,ex[k][:4]))
print(); 
for (a,b),v in conf.most_common(14): print(a,'->',b,v)
