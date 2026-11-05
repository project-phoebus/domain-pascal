#!/usr/bin/env python3
"""getregsurvey.py DIS [NAMES] - which data register a new value takes, over
a per-routine disassembly: the image's (tools/alldis.py SAU CPU OUT) or
dpc's (tools/objdis.py OBJ...), restricted to the routine names in the file
NAMES when given.  docs/register-model.md, "Step 4: the register a new value
takes".

An event is a pure write of a new value (`move`, `moveq`, `clr`) into a data
register, where at least one other register could have taken it: dead at
that point, D0/D1 or already saved by the routine, and neither read nor
written over the new value's range in linear order.  A value live across a
call, or with a backward branch into its range, is no event: colouring may
have had reasons the linear range does not show.

Table 1: did the value take the lowest such register, by what that register
last held.  Table 2: how often each selection policy names the register the
value took.

The ranges are linear, so a variable whose register is written again later
on another path can look dead when it is not.  The counts bound the effect;
they are not a mechanism.
"""
import collections
import re
import sys

import cachesurvey as cs

DREGS = ['%%d%d' % i for i in range(8)]
SCRATCH = ('%d0', '%d1')
REG = re.compile(r'%[ad][0-7]')
BRANCH = re.compile(r'b(eq|ne|cc|cs|hi|ls|gt|ge|lt|le|mi|pl|ra)[sw]?$')


def is_reg(x):
    return REG.fullmatch(x) is not None


def cut(ins):
    for i, x in enumerate(ins):
        if x[1] == 'rts':
            return ins[:i + 1]
    return ins


def saved(ins):
    s = set()
    for _, m, o in ins[1:3]:
        if m == 'moveml' and o.endswith('%sp@-'):
            for part in o.split(',')[0].split('/'):
                g = re.findall(r'%([ad])(\d)', part)
                if len(g) == 2:
                    s |= {'%%%s%d' % (g[0][0], i)
                          for i in range(int(g[0][1]), int(g[1][1]) + 1)}
                else:
                    s.add('%' + g[0][0] + g[0][1])
        elif m == 'movel' and re.fullmatch(r'%d[2-7],%sp@-', o):
            s.add(o[:3])
    return s


def reads_writes(m, o):
    ops = cs.split_ops(o) if o else []
    if not ops or m == 'moveml':
        return set(), set()
    d = ops[-1]
    dst, rd = set(), set()
    pure = (m.startswith(('movel', 'movew', 'moveb', 'moveal', 'moveq', 'clr', 'lea'))
            or (m.startswith('s') and not m.startswith(('sub', 'swap'))))
    writes = not m.startswith(('cmp', 'tst', 'btst', 'bf', 'pea', 'jsr', 'jmp', 'b', 'db'))
    if is_reg(d) and writes:
        dst.add(d)
        if not pure:
            rd.add(d)
    for x in (ops[:-1] if is_reg(d) and writes else ops):
        rd |= set(REG.findall(x))
    if m.startswith('db'):
        rd.add(ops[0])
        dst.add(ops[0])
    return rd, dst


def is_call(m):
    return m == 'jsr' or m.startswith('bsr')


def survey(rs, keep):
    held_tab = collections.Counter()
    hits, total = collections.Counter(), 0
    for n, ins0 in rs.items():
        if keep is not None and n not in keep:
            continue
        if not ins0 or ins0[0][1] != 'linkw':
            continue
        ins = cut(ins0)
        sv = saved(ins)
        rw = [reads_writes(m, o) for _, m, o in ins]
        index = {x[0]: i for i, x in enumerate(ins)}
        targets = collections.defaultdict(list)
        for k, (_, m, o) in enumerate(ins):
            if BRANCH.match(m) or m.startswith('db'):
                t = o.split(',')[-1].strip().split(' ')[0]
                if re.fullmatch(r'(0x)?[0-9a-f]+', t):
                    targets[int(t, 16)].append(k)

        def live_after(k, r):
            for j in range(k + 1, len(ins)):
                if r in rw[j][0]:
                    return True
                if r in rw[j][1]:
                    return False
                if is_call(ins[j][1]) and r in SCRATCH:
                    return False
            return False

        ent, held, lastdef, lastrd = {}, {}, {}, {}
        for k, (a, m, o) in enumerate(ins):
            if a in targets:
                for r in list(ent):
                    if any(s < ent[r][1] or s >= k for s in targets[a]):
                        del ent[r]
            if is_call(m):
                ent.clear()
                for r in SCRATCH:
                    held[r] = 'call'
                continue
            rd, dst = rw[k]
            ops = cs.split_ops(o) if o else []
            if (m.startswith(('movel', 'moveq', 'movew', 'moveb', 'clr'))
                    and len(dst) == 1 and k > 2):
                r = next(iter(dst))
                if r not in rd and r[1] == 'd':
                    end = k
                    for j in range(k + 1, len(ins)):
                        if r in rw[j][0]:
                            end = j
                        elif r in rw[j][1]:
                            break
                    loop = any(k <= index.get(t, -1) <= end and s > end
                               for t, ss in targets.items() for s in ss)
                    call = any(is_call(ins[j][1]) for j in range(k, end + 1))

                    def free_over(x):
                        return x == r or not any(
                            x in rw[j][0] or (x in rw[j][1] and j > k)
                            for j in range(k, end + 1))
                    dead = [x for x in DREGS
                            if x not in rd and not live_after(k - 1, x)
                            and (x in SCRATCH or x in sv) and free_over(x)]
                    if not call and not loop and len(dead) >= 2 and r in dead:
                        low = dead[0]
                        what = ('valid memory value (%s)' % ent[low][0]
                                if low in ent else held.get(low, 'never written'))
                        held_tab[what, r == low] += 1
                        wrote = lambda x: lastdef.get(x, -1)
                        used = lambda x: max(wrote(x), lastrd.get(x, -1))
                        preds = {
                            'lowest free': low,
                            'oldest write, ties lowest': min(dead, key=lambda x: (wrote(x), x)),
                            'oldest use, ties lowest': min(dead, key=lambda x: (used(x), x)),
                            'never written, then lowest': min(dead, key=lambda x: (wrote(x) >= 0, x)),
                        }
                        last = max(DREGS, key=wrote)
                        i = DREGS.index(last)
                        preds['next after the last written'] = next(
                            x for x in DREGS[i + 1:] + DREGS[:i + 1] if x in dead)
                        total += 1
                        for p, v in preds.items():
                            hits[p] += v == r
            for u in rd:
                lastrd[u] = k
            for d in dst:
                lastdef[d] = k
                ent.pop(d, None)
                for x in list(ent):
                    if d in REG.findall(ent[x][2]):
                        del ent[x]
                if m.startswith(('moveq', 'clr')) or (ops[0].startswith('#') and m.startswith('move')):
                    held[d] = 'constant'
                elif m.startswith(('movel', 'movew', 'moveb')) and is_reg(ops[0]):
                    held[d] = 'copy of a register'
                elif m.startswith(('movel', 'movew', 'moveb')):
                    held[d] = 'load, entry gone'
                elif m.startswith('s') and not m.startswith(('sub', 'swap')):
                    held[d] = 'scc'
                else:
                    held[d] = 'computed'
            if (len(ops) == 2 and not is_reg(ops[1]) and cs.cacheable(ops[1])
                    and not m.startswith(('cmp', 'tst', 'btst', 'bf', 'pea', 'lea'))):
                kd = cs.kind(ops[1])
                for x in list(ent):
                    e = ent[x][2]
                    if (e == ops[1] or (kd == 'pointer' and cs.kind(e) in ('pointer', 'unit', 'abs'))
                            or (kd in ('unit', 'abs') and cs.kind(e) == 'pointer')):
                        del ent[x]
            if m in ('movel', 'movew', 'moveb') and len(ops) == 2:
                if re.fullmatch(r'%d[0-7]', ops[1]) and cs.cacheable(ops[0]):
                    ent[ops[1]] = ('load', k, ops[0])
                elif is_reg(ops[0]) and cs.cacheable(ops[1]):
                    ent[ops[0]] = ('store', k, ops[1])
    return held_tab, hits, total


def main():
    rs = cs.parse(sys.argv[1])
    keep = set(open(sys.argv[2]).read().split()) if len(sys.argv) > 2 else None
    held_tab, hits, total = survey(rs, keep)
    for what in sorted({w for w, _ in held_tab}):
        print('%-28s lowest %5d  another %5d' % (what, held_tab[what, True], held_tab[what, False]))
    print()
    for p, h in hits.items():
        print('%-30s %5d / %5d  %.3f' % (p, h, total, h / total))


if __name__ == '__main__':
    main()
