#!/usr/bin/env python3
"""basesurvey.py - constant data addresses held in registers across
statements, calls, joins and loops, in the compiled code of the SR10.2
image.

Evidence tool for docs/codegen-patterns.md 5.2 ("Another unit's array",
"Base kept across statements", "In loops"), beads domain-pascal-lnzy
(item 1: the base kept for a later statement; item 2: other parts of the
same variable through the base register) and domain-pascal-dw25 (another
unit's array in a loop).

Routines are modelled as in tools/regalloc.py and evaluated as in
tools/csesurvey.py (a register holds an expression; at a join it keeps
it only when every predecessor agrees; events since: `st' a store, `ptr'
a store through a pointer, `call', `marith', `br' a conditional branch
passed, `join', `back' a loop back-edge passed).  A *constant base* is a
register holding the address of a data object: `movea.l #K,An',
`move.l #K,Dn', `lea K.l,An' (K inside a data segment or section of the
map), or a copy of one.  Each *reference* of K is an instruction that
reads such a register as the base of a memory operand (`(An)', `(d,An)',
`(d,An,Dx)', a `lea'/`pea' of one); a register-to-register copy is not a
reference (it is followed to the reference it feeds).

Each load of K is classified by the state before it:

    first   K was not loaded or referenced on every path to the load
    held    a register still holds K (a reload while held)
    lost    K was loaded and referenced on every path, no register holds
            it any more (the register was given another value or a call
            clobbered it)

and each reference by how its register got K:

    use     the register's first reference since its load (part of the
            load's own statement)
    kept    a later reference through a register loaded (not copied)
            before: the base kept
    copy    through a register copied from another one (`movea.l D7,A1',
            TTY_$K_DRAIN_OUTPUT 0xE67B20)

with the events since the previous reference of K (any register; a load
counts as one), the register (scratch A0/A1/D0/D1 or callee-saved), the
instruction distance when known, whether the reference is to the same
element (the same displacement and index) as the previous one, and
whether it lies in a loop body (`L': inside the range of a backward
branch).

usage:
    tools/basesurvey.py summary [--min N] [--max N]
                                   # loads by class and event, kept and
                                   # copy references by event and register
    tools/basesurvey.py list CLASS [EVENTS]
                                   # the sites: CLASS is first, held, lost,
                                   # kept, copy (EVENTS as in summary,
                                   # e.g. `call' or `-')
    tools/basesurvey.py pairs [--all] [--min N] [--max N]
                                   # each later reference of a base after
                                   # an earlier one, no call between,
                                   # outside loops: one statement or two
                                   # (same_statement's heuristic), the two
                                   # references' kinds (element operand,
                                   # `lea' into the base's register or
                                   # another, ...), the event between,
                                   # whether another address register is
                                   # written between (Adef), kept or
                                   # reloaded (--all: not only element to
                                   # element)
    tools/basesurvey.py loops [--min N] [--max N]
                                   # per outermost loop and base with
                                   # element references in its body:
                                   # loaded in the body (L) or held from
                                   # before it (H), with whether the loop
                                   # is a `dbf' one and has calls
    tools/basesurvey.py routine ADDR
                                   # one routine's instructions with the
                                   # classes found

Python 3 standard library + m68k-elf-objdump.  Read-only; writes only a
temporary file under tmp/ (through loopalign.disasm).
"""

import os
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import csesurvey  # noqa: E402
import regalloc  # noqa: E402
import sau2map  # noqa: E402
from regalloc import Unknown, is_reg  # noqa: E402

REGS = csesurvey.REGS
SCRATCH = {'d0', 'd1', 'a0', 'a1'}
_MEM = csesurvey._MEM


def kval(e):
    """The constant address an expression is, or None."""
    if not isinstance(e, tuple) or not e or e[0] != 'k':
        return None
    v = e[1]
    if isinstance(v, int):
        return v
    if isinstance(v, str) and v.startswith('addr:abs:'):
        try:
            return int(v[len('addr:abs:'):], 16)
        except ValueError:
            return None
    return None


def mem_bases(x):
    """[(operand index, base register, displacement, index register or
    None)] for the memory operands of x addressed through A0-A4 or a data
    register cannot be; the base is one of a0-a4."""
    out = []
    for k, op in enumerate(x.ops):
        m = _MEM.match(op)
        if not m:
            continue
        base, disp, idx, _isz, _inc = m.groups()
        if base not in ('a0', 'a1', 'a2', 'a3', 'a4'):
            continue
        out.append((k, base, disp or '0', idx))
    return out


def loop_ranges(rt):
    """[(head, tail)] instruction-index ranges of backward branches."""
    out = []
    for i, ss in rt.succ.items():
        for s in ss:
            if s <= i:
                out.append((s, i))
    return out


class BaseSym(csesurvey.Sym):
    """csesurvey's model with constant bases tracked (see the module
    docstring).  isdata(K) says whether K is a data address."""

    def __init__(self, rt, isdata, extent=None):
        self.isdata = isdata
        self.extent = extent or (lambda k: k + 1)
        self.loops = loop_ranges(rt)
        self.brefs = []
        super().__init__(rt)

    def in_loop(self, i):
        return any(h <= i <= t for h, t in self.loops)

    def holders(self, st, k):
        return [q for q in REGS if kval(st.get(q)) == k]

    def step(self, i, st, ev):
        x = self.rt.ins[i]
        if self.record and i == self.rt.reach[0]:
            self.brefs = []
        refs = []           # (K, register, displacement, index) referenced
        for _k, b, disp, idx in mem_bases(x):
            k = kval(st.get(b))
            if k is not None and self.isdata(k):
                idxe = repr(st.get(idx)) if idx else None
                refs.append((k, b, disp, idxe))
        load = None         # (K, destination register)
        if x.mn in ('moveal', 'movel', 'lea') and len(x.ops) == 2:
            r = is_reg(x.ops[1])
            if r in REGS:
                if x.mn == 'lea':
                    src = x.ops[0]
                    if csesurvey._ABS.match(src):
                        k = int(src, 16)
                        if self.isdata(k):
                            load = (k, r)
                elif x.ops[0].startswith('#'):
                    try:
                        k = int(x.ops[0][1:], 0)
                    except ValueError:
                        k = None
                    if k is not None and self.isdata(k):
                        load = (k, r)
        copy = None         # (K, from, to)
        if x.mn in ('moveal', 'movel') and len(x.ops) == 2:
            a, b = is_reg(x.ops[0]), is_reg(x.ops[1])
            if a in REGS and b in REGS:
                k = kval(st.get(a))
                if k is not None and self.isdata(k):
                    copy = (k, a, b)
        if self.record:
            loop = self.in_loop(i)
            # absolute operands into an object a register holds the base of
            for op in x.ops:
                if not csesurvey._ABS.match(op):
                    continue
                a = int(op, 16)
                for q in REGS:
                    k = kval(st.get(q))
                    if k is not None and self.isdata(k) and k <= a < self.extent(k):
                        self.brefs.append(dict(cls='abs', i=i, k=k, reg=q, src=q,
                                               ev=ev.get(('LR', k), frozenset()),
                                               dist=None, same=None, loop=loop,
                                               regev=ev.get(q, frozenset()), idx=False))
                        break
            seen = set()
            for k, b, disp, idxe in refs:
                if (k, b) in seen:
                    continue
                seen.add((k, b))
                es = ev.get(('LR', k))
                es = frozenset() if es is None else es
                d = st.get(('LRI', k))
                dist = i - d if isinstance(d, int) else None
                same = st.get(('LRO', k)) == (disp, idxe) if ('LRO', k) in st else None
                org = st.get(('ORG', b))
                if org == 'copy':
                    cls = 'copy'
                elif 'fresh' in ev.get(('U', b), frozenset()):
                    cls = 'use'
                else:
                    cls = 'kept'
                src = st.get(('SRC', b), b)
                self.brefs.append(dict(cls=cls, i=i, k=k, reg=b, src=src, ev=es,
                                       dist=dist, same=same, loop=loop,
                                       prev=d if isinstance(d, int) else None,
                                       regev=ev.get(b, frozenset()),
                                       idx=idxe is not None))
            if load is not None:
                k, r = load
                hs = self.holders(st, k)
                if hs:
                    cls = 'held'
                elif ('LR', k) in st:
                    cls = 'lost'
                else:
                    cls = 'first'
                es = ev.get(('LR', k), frozenset())
                d = st.get(('LRI', k))
                src = hs[0] if hs else st.get(('LRR', k))
                killer = None
                if cls == 'lost' and isinstance(d, int) and src:
                    # what gave the last reference's register another value
                    # (straight-line, in address order)
                    for j in range(d, i):
                        if src in self.rt.ins[j].defs or self.rt.ins[j].call:
                            killer = j
                            break
                self.brefs.append(dict(cls=cls, i=i, k=k, reg=r, src=src, ev=es,
                                       dist=i - d if isinstance(d, int) else None,
                                       same=None, loop=self.in_loop(i),
                                       regev=ev.get(hs[0], frozenset()) if hs else frozenset(),
                                       idx=False, killer=killer,
                                       prev=d if isinstance(d, int) else None))
        super().step(i, st, ev)
        # registers given a new value lose their origin marks
        for r in REGS:
            if kval(st.get(r)) is None:
                for t in ('U', 'ORG', 'SRC'):
                    st.pop((t, r), None)
                    ev.pop((t, r), None)
        for k, b, disp, idxe in refs:
            st[('LR', k)] = ('lr', k)
            ev[('LR', k)] = frozenset()
            st[('LRI', k)] = i
            st[('LRR', k)] = b
            st[('LRO', k)] = (disp, idxe)
            if kval(st.get(b)) == k and ('U', b) in st:
                ev[('U', b)] = ev.get(('U', b), frozenset()) - {'fresh'}
        if load is not None:
            k, r = load
            st[r] = ('k', k)
            st[('LR', k)] = ('lr', k)
            ev[('LR', k)] = frozenset()
            st[('LRI', k)] = i
            st[('LRR', k)] = r
            st.pop(('LRO', k), None)
            st[('U', r)] = r
            ev[('U', r)] = frozenset({'fresh'})
            st[('ORG', r)] = 'load'
            st[('SRC', r)] = r
        if copy is not None:
            k, a, b = copy
            st[('U', b)] = b
            ev[('U', b)] = frozenset({'fresh'})
            st[('ORG', b)] = 'copy'
            st[('SRC', b)] = a


def evkey(es, keep_br=False):
    s = sorted(es if keep_br else es - {'br'})
    return '+'.join(s) if s else '-'


def strongest(es):
    """The strongest event: call (marith counted as call) > back > join >
    ptr > st > br > none."""
    es = set(es)
    if 'marith' in es:
        es.add('call')
    for o in ('call', 'back', 'join', 'ptr', 'st', 'br'):
        if o in es:
            return o
    return '-'


def isdata_fn(m):
    cache = {}

    def isdata(k):
        if k in cache:
            return cache[k]
        seg = m.segment_containing(k)
        ok = k >= 0xE00000 and seg is not None and seg.kind == 'D'
        cache[k] = ok
        return ok
    return isdata


def extent_fn(m):
    """K -> the end of the object at K: the next map symbol, or the end
    of K's segment if that is closer."""
    cache = {}

    def extent(k):
        if k not in cache:
            seg = m.segment_containing(k)
            nxt = m.next_symbol_addr(k)
            ends = [e for e in (nxt, seg.end if seg else None) if e is not None]
            cache[k] = min(ends) if ends else k + 1
        return cache[k]
    return extent


def survey(done, m):
    """-> [(routine, BaseSym)] for the routines the model takes."""
    isdata = isdata_fn(m)
    extent = extent_fn(m)
    out = []
    skipped = 0
    for _seg, rt in done:
        try:
            out.append((rt, BaseSym(rt, isdata, extent)))
        except Unknown:
            skipped += 1
    return out, skipped


def rclass(r):
    return 'scr' if r in SCRATCH else 'cs'


def tally(syms):
    c = Counter()
    ex = defaultdict(list)
    for rt, sy in syms:
        for b in sy.brefs:
            cls = b['cls']
            if cls == 'use':
                c['use'] += 1
                continue
            ev = strongest(b['ev'])
            if cls in ('kept', 'copy'):
                reg = b['src'] if cls == 'copy' else b['reg']
                key = (cls, ev, rclass(reg) + ('D' if reg and reg[0] == 'd' else 'A')
                       + ('x' if b['idx'] else ''), 'L' if b['loop'] else 'S')
            elif cls == 'abs':
                key = (cls, ev, rclass(b['reg']), 'L' if b['loop'] else 'S')
            elif cls == 'held':
                key = (cls, ev, rclass(b['src']), 'L' if b['loop'] else 'S')
            elif cls == 'lost':
                key = (cls, ev, rclass(b['src']) if b['src'] else '?',
                       'L' if b['loop'] else 'S')
            else:
                key = (cls, '-', '', 'L' if b['loop'] else 'S')
            c[key] += 1
            ex[key].append('%06X' % rt.ins[b['i']].addr)
    return c, ex


def report(c, ex, n=6):
    print('first uses of a load (not counted below): %d' % c['use'])
    for cls in ('first', 'held', 'lost', 'kept', 'copy', 'abs'):
        ks = sorted((k for k in c if isinstance(k, tuple) and k[0] == cls),
                    key=lambda k: (k[3], k[1], k[2]))
        tot = sum(c[k] for k in ks)
        print('%s: %d' % (cls, tot))
        for k in ks:
            print('  %-5s %-3s %-1s %5d  %s' % (k[1], k[2], k[3], c[k], ' '.join(ex[k][:n])))


_LEA = re.compile(r'^%(a[0-4])@\(([-0-9a-f]+)(?:,%(d[0-7]):[wl])?\)$')


def rkind(x):
    """How instruction x references a constant base: `load' (the
    `movea.l #K'/`move.l #K'/`lea K.l' itself), `leaX-self'/`leaX-other'
    (an element's address, `lea (d,An,Dx),Am', into the base's register
    or another), `leaF-self'/`leaF-other' (a field's, `lea (d,An),Am'),
    `pea', `opndX' (an element operand `(d,An,Dx)') or `opnd' (another
    memory operand through the base)."""
    if x.mn in ('moveal', 'movel', 'lea') and len(x.ops) == 2 and (
            x.ops[0].startswith('#') or (x.mn == 'lea' and csesurvey._ABS.match(x.ops[0]))):
        return 'load'
    if x.mn == 'lea' and len(x.ops) == 2:
        m = _LEA.match(x.ops[0])
        if m:
            k = 'leaX' if m.group(3) else 'leaF'
            return k + ('-self' if x.ops[1][1:] == m.group(1) else '-other')
    if x.mn == 'pea':
        return 'pea'
    return 'opndX' if any(',%d' in o for o in x.ops if '@(' in o) else 'opnd'


_NOSTORE = ('cmp', 'cmpa', 'cmpi', 'cmpm', 'tst', 'btst', 'pea', 'lea', 'chk', 'jsr',
            'bsr', 'link', 'unlk', 'movem')


def is_store(x):
    """Whether x writes memory other than the stack (a push): its last
    operand a memory operand not through SP, its mnemonic one that writes
    that operand."""
    if not x.ops or x.call or x.kind is not None:
        return False
    mn = x.mn.rstrip('bwl') if x.mn not in ('bclr', 'bsr') else x.mn
    if mn in _NOSTORE or x.mn.startswith(('cmp', 'tst', 'btst', 'movem')):
        return False
    op = x.ops[-1]
    m = _MEM.match(op)
    if m:
        return m.group(1) != 'sp'
    return bool(csesurvey._ABS.match(op))


def is_label(rt, j):
    """Whether instruction j is a branch target (a predecessor other than
    the instruction before it)."""
    return any(p != j - 1 for p in rt.pred.get(j, []))


def _src_postinc(x):
    return len(x.ops) == 2 and x.ops[0].endswith('@+')


def same_statement(rt, p, n):
    """Whether references p and n (instruction indices, p < n) look like
    one statement.  A heuristic over instructions (bead
    domain-pascal-lnzy review): a statement boundary lies between them
    when an instruction strictly between writes memory other than the
    stack, when one in (p, n] is a branch target, or when p itself writes
    memory, except where n is the next instruction and both move from
    one `(Am)+' source (a block copy's halves, PROC2_$FORK 0xE72B0A/
    0xE72B0E) or n writes the operand p wrote, its registers not written
    between (a bit field updated in two steps, XPD_$SET_DEBUGGER
    0xE5BD6E/0xE5BD7C; also counted so: 0xE5B856/0xE5B85A `clr.l' then
    `ori.w #128' of one element).  A read and the
    write of one element (`x[i] := x[i] + 1', AUDIT_$SUSPEND 0xE70DCE)
    and the operands of one condition (0xE0526C; an `and then',
    UNMAP_CASE+0x660 0xE54788) are one statement.  Two statements
    with nothing between but register work and a store (`a[i] := 0;
    a[j] := 0' with both indices in registers) cannot be told from a
    block copy's halves without the `(Am)+'."""
    if p >= n:
        return False
    for j in range(p + 1, n):
        if is_store(rt.ins[j]):
            return False
    for j in range(p + 1, n + 1):
        if is_label(rt, j):
            return False
    x, y = rt.ins[p], rt.ins[n]
    if is_store(x):
        if n == p + 1 and _src_postinc(x) and _src_postinc(y) and x.ops[0] == y.ops[0]:
            return True
        if (is_store(y) and x.ops[-1] == y.ops[-1]
                and not any(regalloc.ea_regs(x.ops[-1]) & rt.ins[j].defs
                            for j in range(p + 1, n))):
            return True
        return False
    return True


def adef_between(rt, p, n):
    """Whether an address register A0-A4 is given a value strictly
    between p and n other than by a load of a constant base (a value the
    allocator places: a pointer, a `lea'; the base's own register when a
    reload follows)."""
    return any(r in rt.ins[j].defs and rkind(rt.ins[j]) != 'load'
               for j in range(p + 1, n) for r in ('a0', 'a1', 'a2', 'a3', 'a4'))


def pairs(syms, elem=True):
    """Each later reference of a constant base after an earlier one (no
    call between, outside loops): (the earlier reference's kind, the
    later one's, the strongest event between) -> outcome, `kept' (the
    register read again, or a copy of it) or `reload' (`movea.l #K'
    again; `reload-held' while a register still holds K).  With elem,
    only element to element (`opndX'/`leaX' both)."""
    c = Counter()
    ex = defaultdict(list)
    for rt, sy in syms:
        for b in sy.brefs:
            if b['cls'] not in ('kept', 'lost', 'held', 'copy') or b['loop']:
                continue
            ev = strongest(b['ev'])
            if ev == 'call' or b.get('prev') is None:
                continue
            pk = rkind(rt.ins[b['prev']])
            ni = b['i']
            if b['cls'] in ('lost', 'held'):
                nx = [u for u in sy.brefs if u['cls'] == 'use' and u['reg'] == b['reg']
                      and u['i'] > b['i']]
                ni = min(nx, key=lambda u: u['i'])['i'] if nx else None
                nk = rkind(rt.ins[ni]) if nx else '?'
            else:
                nk = rkind(rt.ins[b['i']])
            if elem and not (pk.startswith(('opndX', 'leaX'))
                             and nk.startswith(('opndX', 'leaX'))):
                continue
            out = {'kept': 'kept', 'copy': 'kept', 'lost': 'reload',
                   'held': 'reload-held'}[b['cls']]
            st = 'one' if ni is not None and same_statement(rt, b['prev'], ni) else 'two'
            ad = 'adef' if adef_between(rt, b['prev'], b['i']) else '-'
            key = (st, pk, nk.split('-')[0], ev, ad, out)
            c[key] += 1
            ex[key].append('%06X' % rt.ins[b['i']].addr)
    return c, ex


def outer_loops(sy):
    merged = {}
    for h, t in sy.loops:
        merged[h] = max(merged.get(h, t), t)
    return sorted((h, t) for h, t in merged.items()
                  if not any(h2 <= h and t <= t2 and (h2, t2) != (h, t)
                             for h2, t2 in merged.items()))


def loopbases(syms):
    """Per outermost loop and constant base with element references
    (indexed operands, `lea'/`pea' of one) in its body: `H' (every such
    reference through a register that got K before the loop's head: a
    back edge lies between, or a copy of a held register), `L' (all
    loaded in the body), `HL' (both); for H whether K's load before the
    loop is used before the head (`used-before') or not
    (`loaded-for-loop'); whether the loop ends with `dbf' (a `for'
    loop) and has calls.  -> [(rt, head, tail, K, kind, before, dbf,
    calls)]."""
    rows = []
    for rt, sy in syms:
        for h, t in outer_loops(sy):
            calls = sum(1 for i in range(h, t + 1) if rt.ins[i].call
                        and not (regalloc.MARITH[0] <= (rt.ins[i].target or 0)
                                 < regalloc.MARITH[1]))
            per = defaultdict(set)
            for b in sy.brefs:
                if h <= b['i'] <= t and b['cls'] in ('use', 'kept', 'copy') and b['idx']:
                    held = b['cls'] == 'copy' or 'back' in b['regev']
                    per[b['k']].add('H' if held else 'L')
            for k, v in sorted(per.items()):
                kind = 'H' if v == {'H'} else 'L' if v == {'L'} else 'HL'
                before = ''
                if 'H' in v:
                    lds = [b for b in sy.brefs if b['k'] == k and b['i'] < h
                           and b['cls'] in ('first', 'lost', 'held')]
                    if lds:
                        ld = max(lds, key=lambda b: b['i'])
                        used = any(ld['i'] < b['i'] < h and b['k'] == k
                                   and b['cls'] in ('use', 'kept', 'copy') for b in sy.brefs)
                        before = 'used-before' if used else 'loaded-for-loop'
                    else:
                        before = 'no-load-seen'
                rows.append((rt, h, t, k, kind, before, rt.ins[t].kind == 'db', calls))
    return rows


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    lo, hi, _, argv = regalloc.opts(argv)
    cmd = argv[0]
    m = sau2map.load()
    if cmd == 'routine':
        r0 = int(argv[1], 16)
        rt = regalloc.one(m, r0)
        sy = BaseSym(rt, isdata_fn(m), extent_fn(m))
        by = defaultdict(list)
        for b in sy.brefs:
            by[b['i']].append('%s %s K=%s ev[%s]%s' % (
                b['cls'], b['reg'], m.describe(b['k']), evkey(b['ev'], True),
                ' L' if b['loop'] else ''))
        print('%06X..%06X %s' % (r0, rt.r1, m.describe(r0)))
        for i in rt.reach:
            x = rt.ins[i]
            print('  %06X %-34s %s' % (x.addr, x.text, ' | '.join(by.get(i, []))))
        return 0
    _, done, skipped = regalloc.load_all(lo, hi)
    syms, sk = survey(done, m)
    if cmd == 'summary':
        print('routines analysed %d (model skipped %s; symbolic skipped %d)' % (
            len(syms), dict(skipped), sk))
        c, ex = tally(syms)
        report(c, ex)
        return 0
    if cmd == 'list':
        cls = argv[1]
        evf = argv[2] if len(argv) > 2 else None
        for rt, sy in syms:
            for b in sy.brefs:
                if b['cls'] != cls or (evf is not None and strongest(b['ev']) != evf):
                    continue
                print('%06X %-28s %-18s %-3s ev[%s] dist=%s same=%s%s%s  %s' % (
                    rt.ins[b['i']].addr, m.describe(rt.r0)[:28], m.describe(b['k'])[:18],
                    b['reg'], evkey(b['ev'], True), b['dist'], b['same'],
                    ' x' if b['idx'] else '',
                    ' L' if b['loop'] else '', rt.ins[b['i']].text)
                      + ('  [killed by %06X %s]' % (rt.ins[b['killer']].addr,
                                                     rt.ins[b['killer']].text)
                         if b.get('killer') is not None else ''))
        return 0
    if cmd == 'pairs':
        c, ex = pairs(syms, elem='--all' not in argv)
        print('%-4s %-11s %-6s %-5s %-5s %-11s %5s' % ('stmt', 'earlier', 'later', 'event',
                                                      'Adef', 'outcome', 'n'))
        for k in sorted(c):
            print('%-4s %-11s %-6s %-5s %-5s %-11s %5d  %s' % (k + (c[k], ' '.join(ex[k][:8]))))
        tot = Counter()
        for (st, pk, nk, ev, ad, out), n in c.items():
            tot[st, pk.split('-')[0] + ('-self' if pk.endswith('-self') else ''), ad, out] += n
        print()
        print('  stmt earlier     Adef  outcome')
        for k in sorted(tot):
            print('  %-4s %-11s %-5s %-11s %5d' % (k + (tot[k],)))
        return 0
    if cmd == 'loops':
        rows = loopbases(syms)
        c = Counter()
        for rt, h, t, k, kind, before, dbf, calls in rows:
            print('%06X..%06X %-26s %-20s %-2s %-16s %s calls=%d' % (
                rt.ins[h].addr, rt.ins[t].addr, m.describe(rt.r0)[:26],
                m.describe(k)[:20], kind, before, 'dbf  ' if dbf else 'other', calls))
            c[kind, before, 'dbf' if dbf else 'other', 'calls' if calls else 'nocall'] += 1
        print()
        for key, n in sorted(c.items()):
            print('  %-2s %-16s %-5s %-6s %4d' % (key + (n,)))
        return 0
    print(__doc__)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
