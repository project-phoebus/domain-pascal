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
    tools/resultsurvey.py ptrlocals [-v]  # those copied to An or stored to
                                          # a frame slot: later pushed by
                                          # value? what follows the copy?
    tools/resultsurvey.py parkd0          # routines that hold an A0 result
                                          # in D0 where a call follows
                                          # (evidence; no dpc option)
    tools/resultsurvey.py ptrhold [-v]    # every A0-function call: the first
                                          # read of A0 on its fall-through
                                          # path (D0 parking, A1, kept)
    tools/resultsurvey.py storetest       # a register stored to memory,
                                          # then the value tested: on the
                                          # store's flags, the register or
                                          # the memory
    tools/resultsurvey.py storebra [-v]   # a `bra' right after a store to
                                          # memory whose target is at a
                                          # `tst' of the object: lands on
                                          # the `tst', past it, or past the
                                          # branch after it too (-v: every
                                          # site)

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


def a0_reads(rs, by_start):
    """(routine, index of the first instruction reading A0) after each
    call of an A0 function whose caller reads A0 next (`callers')."""
    a0 = a0_functions(rs, by_start)
    for r in rs:
        ins = r.insns
        for i, (a, n, t) in enumerate(ins):
            m = nestsurvey._BSR.match(t) or nestsurvey._JSR.match(t)
            if not m or int(m.group(1), 16) not in a0 or not reads_after(ins, i, '%a0'):
                continue
            j = i + 1
            if j < len(ins) and _POP.match(ins[j][2]):
                j += 1
            yield r, j


def ptrlocals(rs, by_start, verbose):
    """Bead domain-pascal-500 (codegen-patterns 3, "Pointer locals"): for
    each `movea.l A0,An' / `move.l A0,(-k,A6)' after such a call, whether
    the routine's code after it pushes that register or slot by value
    (`move.l X,-(SP)'; not limited to the value's lifetime), and for the
    copies what the next instruction does with it."""
    c, ex = Counter(), {}
    for r, j in a0_reads(rs, by_start):
        t = r.insns[j][2]
        use = a0_use(t)
        if use not in ('copy An', 'store frame'):
            continue
        loc = operands(t)[1][1]
        pushed = any(operands(tk)[0] == 'movel' and operands(tk)[1] == [loc, '%sp@-']
                     for _, _, tk in r.insns[j + 1:])
        nxt = ''
        if use == 'copy An':
            mn, ops = operands(r.insns[j + 1][2])
            if mn.startswith('cmpa') and ops[:1] == ['#0']:
                nxt = 'then cmpa #0,%s' % ('An' if ops[1] == loc else ops[1])
            elif any(o.startswith(loc + '@') for o in ops):
                nxt = 'then deref An'
            elif any(o.startswith('%a0@') for o in ops):
                nxt = 'then deref A0'
        for key in ((use, 'pushed' if pushed else 'not pushed'), (use, nxt)):
            if key[1]:
                c[key] += 1
                ex.setdefault(key, []).append('0x%06X' % r.insns[j][0])
        if verbose:
            print('%-30s 0x%06X %-12s %-10s %s %s' % (name(r), r.insns[j][0], use, loc,
                                                       'pushed' if pushed else '', nxt))
    for k, v in sorted(c.items()):
        print('%-12s %-22s %3d  %s' % (k + (v, ' '.join(ex[k][:6]))))


def ptrhold_sites(rs, by_start):
    """Bead domain-pascal-dgb (codegen-patterns 3, "Pointer locals"): every
    call of an A0 function, followed along its fall-through path (past
    conditional branches, up to an unconditional branch, a call, the
    end, or a write of A0) to the first instruction that reads A0 (not
    only the next one, as `callers' does).  Classed by that read: `D0'
    (`move.l A0,D0'), `Dn', `A1' (`movea.l A0,A1'), `An' (callee-saved),
    `kept' (A0 read in place: a base, a store, a test), each with
    `next' when it is the first instruction after the call (and its pop)
    or `later'.  For `D0', how D0 is next read on the same path:
    `movea.l D0,An' (`->An') or otherwise, and whether a call comes first.
    Yields (routine, address of the read, class, next/later, tail, text)."""
    a0 = a0_functions(rs, by_start)
    for r in rs:
        ins = r.insns
        for i, (a, n, t) in enumerate(ins):
            m = nestsurvey._BSR.match(t) or nestsurvey._JSR.match(t)
            if not m or int(m.group(1), 16) not in a0:
                continue
            j = i + 1
            if j < len(ins) and _POP.match(ins[j][2]):
                j += 1
            first = j
            while j < len(ins):
                tj = ins[j][2]
                if nestsurvey.reads(tj, '%a0') or _CALL.match(tj) or tj.startswith(
                        ('rts', 'unlk', 'bra', 'jmp')) or nestsurvey.writes(tj, '%a0'):
                    break
                j += 1
            if j >= len(ins) or not nestsurvey.reads(ins[j][2], '%a0'):
                continue
            tj = ins[j][2]
            mn, ops = operands(tj)
            if mn == 'movel' and ops == ['%a0', '%d0']:
                k = 'D0'
            elif mn == 'movel' and ops[0] == '%a0' and re.fullmatch(r'%d[1-7]', ops[1]):
                k = 'Dn'
            elif mn == 'moveal' and ops == ['%a0', '%a1']:
                k = 'A1'
            elif mn == 'moveal' and ops[0] == '%a0' and re.fullmatch(r'%a[2-5]', ops[1]):
                k = 'An'
            else:
                k = 'kept'
            how = 'next' if j == first else 'later'
            tail = ''
            if k == 'D0':
                called = False
                for ak, nk, tk in ins[j + 1:]:
                    if _CALL.match(tk):
                        called = True
                    if nestsurvey.reads(tk, '%d0'):
                        mk, ok = operands(tk)
                        tail = ('->' + ok[1][1:].upper() if mk == 'moveal' and ok[0] == '%d0'
                                else 'other') + (' after call' if called else '')
                        break
                    if nestsurvey.writes(tk, '%d0') or tk.startswith(('rts', 'unlk')):
                        tail = 'unread'
                        break
            yield r, ins[j][0], k, how, tail, tj


def ptrhold(rs, by_start, verbose):
    """`ptrhold': ptrhold_sites counted by (class, next/later, tail)."""
    c, ex = Counter(), {}
    for r, a, k, how, tail, tj in ptrhold_sites(rs, by_start):
        key = (k, how, tail)
        c[key] += 1
        ex.setdefault(key, []).append('0x%06X' % a)
        if verbose:
            print('%-30s 0x%06X %-5s %-6s %-14s %s' % (name(r), a, k, how, tail, tj))
    for key, v in sorted(c.items()):
        print('%-5s %-6s %-16s %3d  %s' % (key + (v, ' '.join(ex[key][:6]))))


def park_d0_routines(rs, by_start):
    """The starts of the routines that hold a call's pointer result in D0
    and then move it to a callee-saved register (`move.l A0,D0' right
    after the call, D0 next read by `movea.l D0,An', n 2..5, with no call
    between; bead domain-pascal-bks; codegen-patterns 3, "Pointer locals
    held in D0").  14 routines, the TTY_$K_* and SIO_$K_* ones
    (TTY_$K_FLUSH_INPUT 0xE1C0A8).  Evidence only: `dpc --park-d0', which
    these sites once selected, was retired with the integer reading
    (bead domain-pascal-78n2), under which they are integer locals."""
    return {r.r0 for r, _a, k, how, tail, _t in ptrhold_sites(rs, by_start)
            if k == 'D0' and how == 'next' and re.fullmatch(r'->A[2-5]', tail)}


def storetest(rs):
    """A register stored to memory (`move.<s> Rn,<mem>', not a push),
    then a test of the value by the next instruction: a conditional
    branch on the store's own flags, `tst'/`cmpa #0' of the register, or
    `tst' of the memory (bead domain-pascal-500)."""
    c, ex = Counter(), {}
    for r in rs:
        ins = r.insns
        for i, (a, n, t) in enumerate(ins[:-1]):
            mn, ops = operands(t)
            if mn not in ('movel', 'movew', 'moveb') or len(ops) != 2:
                continue
            src, dst = ops
            if not re.fullmatch(r'%[ad]\d', src) or re.fullmatch(r'%[ad]\d|%sp@-', dst):
                continue
            m2, o2 = operands(ins[i + 1][2])
            if re.match(r'b(eq|ne|mi|pl)', m2):
                k = 'flags'
            elif (m2.startswith('cmpa') and o2 == ['#0', src]) or (m2.startswith('tst') and o2 == [src]):
                k = 'register'
            elif m2.startswith('tst') and o2 == [dst]:
                k = 'memory'
            else:
                continue
            key = (src[1].upper() + 'n', 'frame' if _FP.match(dst) else 'memory', k)
            c[key] += 1
            ex.setdefault(key, []).append('0x%06X' % a)
    for k, v in sorted(c.items()):
        print('%-3s %-7s %-9s %3d  %s' % (k + (v, ' '.join(ex[k][:6]))))


def storebra(rs, verbose):
    """A `bra' right after `move.<s> X,<mem>', `clr.<s> <mem>' or `st'/`sf
    <mem>' (no push), whose target is a `tst.<s> <mem>' of that object
    (`to-tst'), the instruction after one (`past-tst'), or the one after a
    conditional branch that follows one (`past-branch'): by the store's
    kind (clr, imm, reg, mem, scc) and the branch the `tst' is followed by
    (bead domain-pascal-do4s, codegen-patterns 7.1)."""
    c, ex = Counter(), {}
    for r in rs:
        ins = sorted(r.insns)
        at = {a: i for i, (a, n, t) in enumerate(ins)}
        for i, (a, n, t) in enumerate(ins):
            mn, ops = operands(t)
            if not re.fullmatch(r'bra[sw]?', mn) or i == 0:
                continue
            pm, po = operands(ins[i - 1][2])
            m = re.fullmatch(r'(move|clr)([bwl])|(st|sf)', pm)
            if not m or not po:
                continue
            ea = po[-1]
            if re.fullmatch(r'%[ad]\d', ea) or ea == '%sp@-':
                continue
            if m.group(3):
                kind = 'scc'
            elif m.group(1) == 'clr':
                kind = 'clr'
            elif po[0].startswith('#'):
                kind = 'imm'
            elif re.fullmatch(r'%[ad]\d', po[0]):
                kind = 'reg'
            else:
                kind = 'mem'
            j = at.get(int(ops[0], 16))
            if j is None:
                continue

            def is_tst(k):
                if k < 0:
                    return False
                mm, oo = operands(ins[k][2])
                return mm.startswith('tst') and oo == [ea]
            if is_tst(j):
                cls, after = 'to-tst', operands(ins[j + 1][2])[0] if j + 1 < len(ins) else ''
            elif is_tst(j - 1):
                cls, after = 'past-tst', operands(ins[j][2])[0]
            elif is_tst(j - 2) and re.fullmatch(r'b(?!ra|sr)[a-z]{2}[sw]?', operands(ins[j - 1][2])[0]):
                cls, after = 'past-branch', operands(ins[j - 1][2])[0]
            else:
                continue
            key = (kind, cls, re.sub(r'[sw]$', '', after) if after.startswith('b') else after)
            c[key] += 1
            ex.setdefault(key, []).append('0x%06X' % a)
    for k, v in sorted(c.items()):
        e = ex[k] if verbose else ex[k][:6]
        print('%-4s %-12s %-6s %3d  %s' % (k + (v, ' '.join(e))))


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
    elif cmd == 'ptrlocals':
        ptrlocals(rs, by_start, '-v' in argv[1:])
    elif cmd == 'ptrhold':
        ptrhold(rs, by_start, '-v' in argv[1:])
    elif cmd == 'parkd0':
        for a in sorted(park_d0_routines(rs, by_start)):
            print('0x%06X %s' % (a, by_start[a].name or ''))
    elif cmd == 'storetest':
        storetest(rs)
    elif cmd == 'storebra':
        storebra(rs, '-v' in argv[1:])
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
