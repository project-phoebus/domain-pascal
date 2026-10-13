#!/usr/bin/env python3
"""resultsurvey.py - how compiled functions of the SR10.2 image deliver
their result.

Evidence tool for docs/codegen-patterns.md section 3 (beads
domain-pascal-5aw.11, domain-pascal-8aj).  Routines are split as
tools/nestsurvey.py does (its cache, tmp/nestsurvey.cache).  A routine is
a *D0 function* when some caller reads D0 right after calling it
(nestsurvey.functions), an *A0 function* when some caller reads A0 there.

For every function, each epilogue (`unlk', with the register restore in
front of it) is classed by the instruction that falls into it (`fall'):

    reg Dn      `move.<s> Dn,D0' (`movea.l An,A0'): the result was held in
                another register up to the end
    frame -k    `move.<s> (-k,A6),D0': the result was in its frame slot
    const       `moveq'/`clr'/`move #' into the result register
    call        a `jsr'/`bsr' (with its pop): the tail wrapper
    direct      another instruction writing the result register
    other       anything else (the result register set earlier)

and the branches into the epilogue (`in') are counted.  For `reg' and
`frame', the routine is described further: how many instructions write
that register or slot (`defs'), how many read it before the final move
(`reads'), whether a call lies between the first write and the end
(`call-after'), whether a write lies after the last call (`def-after'),
and the branch count.

usage:
    tools/resultsurvey.py summary         # the counts quoted in section 3
    tools/resultsurvey.py list [CLASS]    # one line per function epilogue
    tools/resultsurvey.py entry           # D0 functions of one epilogue:
                                          # the registers of the parameter
                                          # loads before the first call or
                                          # branch, by parameter order
    tools/resultsurvey.py pointer         # A0 functions: how A0 is set at
                                          # the end, and whether D0 is too
    tools/resultsurvey.py pushes          # a local stored from Dn and later
                                          # pushed by value: from Dn or from
                                          # its slot (codegen-patterns 12.5)
    tools/resultsurvey.py callers [-v]    # calls of A0 functions whose
                                          # caller reads A0 next: what that
                                          # first read is (-v: every site)

`pushes' looks at every `move.<s> Dn,(-k,A6)' into the `link' area and
follows the instructions after it in address order up to a push `move.<s>
X,-(SP)' of that local, noting calls on the way and giving up at a write
of Dn or of the slot, its address taken, a backward branch, or a label
some branch from before the store targets.  The store is a call result when a `jsr'/`bsr'
(and its pop) comes right before it.

`callers' (bead domain-pascal-ae9) takes every call of an A0 function
after which the caller reads A0 (`reads_after') and classes that first
read: `copy An' (`movea.l A0,An'; `held' when a call follows before An is
next written or the routine ends), `copy Dn', `store frame' (`move.l
A0,(-k,A6)'), `store' (another memory destination), `push' (`move.l
A0,-(SP)' or `pea (A0)'), `nil test' (`cmpa.w #0,A0' / `tst.l'), `base'
(A0 as the base of another operand: `(d,A0)', `(A0)'), `other'; and
whether the call was popped before it.

Python 3 standard library + m68k-elf-objdump (through nestsurvey).
Read-only.
"""

import re
import sys
from collections import Counter

import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import nestsurvey  # noqa: E402
from nestsurvey import operands  # noqa: E402

_CALL = re.compile(r'^(jsr|bsr[sw]?)\b')
_BR = re.compile(r'^(b(?:ra|hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le)[sw]?|db\w+)\s+0x([0-9a-f]+)')
_FP = re.compile(r'^%fp@\((-\d+)\)$')
_POP = re.compile(r'^(addq[wl]\s+#\d+,%sp|lea %sp@\(\d+\),%sp)$')


def reads_after(insns, i, reg):
    """Whether the instruction after the call at insns[i] (after its pop)
    reads `reg' ('%a0'): as a source or the base of an operand."""
    j = i + 1
    if j < len(insns) and _POP.match(insns[j][2]):
        j += 1
    if j >= len(insns):
        return False
    t = insns[j][2]
    if t.startswith(('jsr', 'bsr', 'rts', 'unlk')):
        return False
    return nestsurvey.reads(t, reg)


def a0_functions(rs, by_start):
    fs = set()
    for r in rs:
        for i, (a, n, t) in enumerate(r.insns):
            m = nestsurvey._BSR.match(t) or nestsurvey._JSR.match(t)
            if m and int(m.group(1), 16) in by_start and reads_after(r.insns, i, '%a0'):
                fs.add(int(m.group(1), 16))
    return fs


def is_restore(t, link):
    mn, ops = operands(t)
    if mn not in ('moveml', 'movel', 'moveal') or len(ops) != 2:
        return False
    m = _FP.match(ops[0])
    return bool(m) and int(m.group(1)) <= link - 4 and ops[1].startswith('%')


def targets(insns):
    c = Counter()
    for a, n, t in insns:
        m = _BR.match(t)
        if m:
            c[int(m.group(2), 16)] += 1
    return c


def classify(t, reg):
    """Class of the instruction t falling into the epilogue, for result
    register reg ('%d0' or '%a0')."""
    mn, ops = operands(t)
    if _CALL.match(t):
        return 'call', ''
    if not ops or not nestsurvey.writes(t, reg):
        return 'other', ''
    if mn.startswith(('moveq', 'clr')) or (len(ops) == 2 and ops[0].startswith('#')):
        return 'const', ''
    if mn.startswith('move') and len(ops) == 2:
        if re.fullmatch(r'%[ad][0-7]', ops[0]):
            return 'reg', ops[0][1:]
        m = _FP.match(ops[0])
        if m:
            return 'frame', m.group(1)
    return 'direct', ''


class Epi:
    pass


def epilogues(r, reg):
    """[Epi] for the `unlk's of r."""
    tg = targets(r.insns)
    out = []
    ins = r.insns
    for k, (a, n, t) in enumerate(ins):
        if not t.startswith('unlk'):
            continue
        e = k
        while e > 0 and is_restore(ins[e - 1][2], r.link):
            e -= 1
        ea = ins[e][0]
        x = Epi()
        x.addr = ea
        x.into = sum(tg[ins[j][0]] for j in range(e, k + 1))
        p = e - 1
        if p < 0:
            continue
        pt = ins[p][2]
        if _POP.match(pt) and p > 0 and _CALL.match(ins[p - 1][2]):
            p -= 1
            pt = ins[p][2]
        x.p, x.pt = ins[p][0], pt
        x.cls, x.what = classify(pt, reg)
        if pt.startswith('bra') or pt.startswith('rts'):
            x.cls, x.what = 'none', ''
        x.p_in = tg[ins[p][0]]
        out.append(x)
    return out


def describe(r, x):
    """Properties of the register or slot of a `reg'/`frame' epilogue."""
    ins = r.insns
    if x.cls == 'reg':
        name = '%' + x.what

        def d(t):
            return nestsurvey.writes(t, name) or (
                operands(t)[0].startswith(('add', 'sub', 'and', 'or', 'eor', 'not', 'neg',
                                           'lsl', 'lsr', 'asl', 'asr', 'ext', 'swap'))
                and operands(t)[1] and operands(t)[1][-1] == name)

        def u(t):
            return nestsurvey.reads(t, name)
    else:
        slot = '%%fp@(%s)' % x.what

        def d(t):
            ops = operands(t)[1]
            return bool(ops) and ops[-1] == slot and not t.startswith(('tst', 'cmp', 'pea'))

        def u(t):
            ops = operands(t)[1]
            return slot in ops[:-1] or (t.startswith(('tst', 'cmp', 'pea')) and slot in ops)
    defs = [a for a, n, t in ins if a != x.p and d(t)]
    reads = [a for a, n, t in ins if a < x.p and u(t)]
    calls = [a for a, n, t in ins if _CALL.match(t)]
    first = min(defs) if defs else None
    call_after = any(first is not None and first < c < x.p for c in calls)
    last_call = max([c for c in calls if c < x.p], default=None)
    def_after = any(last_call is None or a > last_call for a in defs)
    brs = sum(1 for a, n, t in ins if _BR.match(t))
    taken = any(t.startswith(('pea', 'lea')) and x.cls == 'frame'
                and '%%fp@(%s)' % x.what in t for a, n, t in ins)
    return dict(defs=len(defs), reads=len(reads), call_after=call_after,
                def_after=def_after, calls=len(calls), brs=brs, addr_taken=taken)


def name(r):
    return '0x%06X%s' % (r.r0, ' ' + r.name if r.name else '')


def rows(rs, fs, reg):
    for r in rs:
        if r.r0 not in fs:
            continue
        es = epilogues(r, reg)
        for x in es:
            info = describe(r, x) if x.cls in ('reg', 'frame') else None
            yield r, es, x, info


def fmt(r, es, x, info):
    s = '%-40s link #%-4d epi@%06X %-6s %-5s in=%d p_in=%d  %s' % (
        name(r), r.link, x.addr, x.cls, x.what, x.into, x.p_in, x.pt)
    if info:
        s += '  [defs=%(defs)d reads=%(reads)d call-after=%(call_after)s def-after=%(def_after)s ' \
             'calls=%(calls)d brs=%(brs)d addr=%(addr_taken)s]' % info
    return s


def summary(rs, by_start):
    fs = nestsurvey.functions(rs, by_start)
    a0 = a0_functions(rs, by_start)
    print('D0 functions: %d (one epilogue: %d); A0 functions: %d; both: %d' % (
        len(fs), sum(1 for r in rs if r.r0 in fs and len(epilogues(r, '%d0')) == 1),
        len(a0), len(fs & a0)))
    c = Counter()
    sub = Counter()
    for r, es, x, info in rows(rs, fs, '%d0'):
        c[x.cls] += 1
        if info:
            reg = x.what if x.cls == 'frame' else ('scratch' if x.what in ('d1',) else
                                                     'callee-saved' if x.what[0] == 'd' else x.what)
            sub[(x.cls, reg if x.cls == 'reg' else 'slot', 'call-after' if info['call_after'] else
                 'no-call-after', 'multi-def' if info['defs'] > 1 else 'one-def',
                 'read' if info['reads'] else 'unread')] += 1
    print('D0 epilogues by the instruction falling in: ' + ', '.join(
        '%s %d' % kv for kv in c.most_common()))
    for k, v in sorted(sub.items(), key=lambda kv: (-kv[1], kv[0])):
        print('  %-5s %-12s %-13s %-9s %-6s %d' % (k + (v,)))
    c = Counter()
    for r, es, x, info in rows(rs, a0, '%a0'):
        c[x.cls] += 1
    print('A0 epilogues: ' + ', '.join('%s %d' % kv for kv in c.most_common()))


def listing(rs, by_start, cls):
    fs = nestsurvey.functions(rs, by_start)
    for r, es, x, info in rows(rs, fs, '%d0'):
        if cls is None or x.cls == cls:
            print(fmt(r, es, x, info))


def pointer(rs, by_start):
    a0 = a0_functions(rs, by_start)
    fs = nestsurvey.functions(rs, by_start)
    for r, es, x, info in rows(rs, a0, '%a0'):
        print(('D0+ ' if r.r0 in fs else '    ') + fmt(r, es, x, info))


_PARAM = re.compile(r'^move([bwl]|al) %fp@\((\d+)\),(%[ad]\d)$')


def entry(rs, by_start):
    """Per D0 function: the registers its parameter loads take before the
    first call, branch or store to memory, in parameter order."""
    fs = nestsurvey.functions(rs, by_start)
    c = Counter()
    for r in rs:
        if r.r0 not in fs:
            continue
        loads = []
        for a, n, t in r.insns[1:]:
            if _CALL.match(t) or _BR.match(t):
                break
            m = _PARAM.match(t)
            if m:
                loads.append((int(m.group(2)), m.group(3)[1:], a))
        dl = [x for x in loads if x[1][0] == 'd']
        if len(dl) < 2:
            continue
        by_off = [x[1] for x in sorted(dl)]
        key = 'ascending' if by_off == sorted(by_off) else 'other'
        c[key] += 1
        print('%-40s %-9s %s' % (name(r), key, ' '.join(
            '(%d)->%s@%06X' % x for x in dl)))
    print(dict(c))


def pushes(rs):
    c = Counter()
    ex = {}
    for r in rs:
        ins = r.insns
        for i, (a, n, t) in enumerate(ins):
            mn, ops = operands(t)
            m = re.fullmatch(r'move([bwl])', mn)
            if not m or len(ops) != 2 or not re.fullmatch(r'%d[0-7]', ops[0]):
                continue
            mm = _FP.match(ops[1])
            if not mm or int(mm.group(1)) < r.link:
                continue
            reg, slot, sz = ops[0], ops[1], m.group(1)
            call = i > 0 and (_CALL.match(ins[i - 1][2]) or (
                i > 1 and _POP.match(ins[i - 1][2]) and _CALL.match(ins[i - 2][2])))
            before = {int(b.group(2), 16) for _, _, tk in ins[:i] for b in [_BR.match(tk)] if b}
            crossed = False
            for aj, nj, tj in ins[i + 1:]:
                if aj in before:
                    break
                mj, oj = operands(tj)
                if mj == 'move' + sz and len(oj) == 2 and oj[1] == '%sp@-' and oj[0] in (slot, reg):
                    key = ('call result' if call else 'computed',
                           'call between' if crossed else 'no call',
                           'from the register' if oj[0] == reg else 'from the slot')
                    c[key] += 1
                    ex.setdefault(key, []).append('0x%06X' % aj)
                    break
                if _CALL.match(tj):
                    crossed = True
                    continue
                b = _BR.match(tj)
                if (tj.startswith(('rts', 'unlk', 'jmp'))
                        or (b and int(b.group(2), 16) <= a)
                        or nestsurvey.writes(tj, reg) or (oj and oj[-1] in (reg, slot))
                        or (mj.startswith(('pea', 'lea')) and slot in oj)):
                    break
    for k, v in sorted(c.items()):
        print('%-12s %-13s %-18s %3d  %s' % (k + (v, ' '.join(ex[k][:6]))))


def a0_use(t):
    """Class of instruction t, the first to read A0 after a call."""
    mn, ops = operands(t)
    if mn == 'moveal' and ops[0] == '%a0' and re.fullmatch(r'%a[1-7]', ops[1]):
        return 'copy An'
    if mn == 'movel' and ops[0] == '%a0' and re.fullmatch(r'%d[0-7]', ops[1]):
        return 'copy Dn'
    if mn == 'movel' and ops[0] == '%a0' and ops[1] == '%sp@-':
        return 'push'
    if mn == 'pea' and ops[0] == '%a0@':
        return 'push'
    if mn == 'movel' and ops[0] == '%a0' and _FP.match(ops[1]):
        return 'store frame'
    if mn == 'movel' and ops[0] == '%a0':
        return 'store'
    if (mn.startswith('cmpa') and ops == ['#0', '%a0']) or (mn == 'tstl' and ops == ['%a0']):
        return 'nil test'
    if any(o.startswith('%a0@') for o in ops):
        return 'base'
    return 'other'


def callers(rs, by_start, verbose):
    a0 = a0_functions(rs, by_start)
    c = Counter()
    ex = {}
    for r in rs:
        ins = r.insns
        for i, (a, n, t) in enumerate(ins):
            m = nestsurvey._BSR.match(t) or nestsurvey._JSR.match(t)
            if not m or int(m.group(1), 16) not in a0 or not reads_after(ins, i, '%a0'):
                continue
            j = i + 1
            popped = j < len(ins) and bool(_POP.match(ins[j][2]))
            j += popped
            use = a0_use(ins[j][2])
            if use == 'copy An':
                reg = operands(ins[j][2])[1][1]
                held = False
                for ak, nk, tk in ins[j + 1:]:
                    if _CALL.match(tk):
                        held = True
                        break
                    if nestsurvey.writes(tk, reg) or tk.startswith(('rts', 'unlk')):
                        break
                use += ' %s%s' % (reg[1:], ' held' if held else '')
            key = (use, 'popped' if popped else 'no pop')
            c[key] += 1
            callee = by_start[int(m.group(1), 16)]
            ex.setdefault(key, []).append('0x%06X' % ins[j][0])
            if verbose:
                print('%-40s 0x%06X -> %-28s %-16s %-7s %s' % (
                    name(r), a, name(callee), use, 'popped' if popped else '', ins[j][2]))
    for k, v in sorted(c.items()):
        print('%-16s %-7s %3d  %s' % (k + (v, ' '.join(ex[k][:6]))))


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    rs = nestsurvey.load()
    by_start, _ = nestsurvey.analyse(rs)
    cmd = argv[0]
    if cmd == 'summary':
        summary(rs, by_start)
    elif cmd == 'list':
        listing(rs, by_start, argv[1] if len(argv) > 1 else None)
    elif cmd == 'pointer':
        pointer(rs, by_start)
    elif cmd == 'entry':
        entry(rs, by_start)
    elif cmd == 'pushes':
        pushes(rs)
    elif cmd == 'callers':
        callers(rs, by_start, '-v' in argv[1:])
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
