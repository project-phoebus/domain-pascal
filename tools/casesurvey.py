#!/usr/bin/env python3
"""casesurvey.py - `case' statements in the compiled code of the SR10.2 image.

Evidence tool for docs/codegen-patterns.md section 7, `case' (bead
domain-pascal-5aw.4).  Compiled routines are split and followed the way
tools/a5survey.py does (map symbols and `link.w A6' words, hand assembly
excluded, only the instructions reachable from the routine's start;
jump tables followed through a5survey.table_entries).  Two forms are
found:

    T  jump table: `cmpi #N,Dn; bcc DEFAULT; add.w Dn,Dn;
       move.w (T,PC,Dn.w),Dn; jmp (T,PC,Dn.w)'.  The labels counted are
       the entries that do not go to DEFAULT: a lower bound when an empty
       arm (or an arm the compiler merged with the default) shares it.
    C  compare chain: two or more `cmpi #k,Dn; beq' (or `tst Dn; beq' for
       label 0; a leading `move ...,Dn; beq' for label 0 first) on one
       register (or on the source of a copy `move Dm,Dn; beq'), then a
       `bra'.  Runs whose `beq's all go to one place and
       are not followed by a `bra' are conditions (`x in [a, b]'-like),
       not counted.

For each site: the labels n, their range r (max - min + 1), r/n, the
number of distinct arms.

usage:
    tools/casesurvey.py list       # one line per site, sorted by n, r
    tools/casesurvey.py summary    # the counts quoted in codegen-patterns 7

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import a5survey  # noqa: E402
import loopalign  # noqa: E402
import sau2map  # noqa: E402

_CMP = re.compile(r'^(?:cmpi([bwl]) #(-?\d+),%(d\d)|tst([bwl]) %(d\d))$')
_LOAD = re.compile(r'^move([bwl]) .*,%(d\d)$')
_BEQ = re.compile(r'^beq[sw]?\s+0x([0-9a-f]+)')
_BRA = re.compile(r'^bra[sw]?\s+0x([0-9a-f]+)')
_BCC = re.compile(r'^b(?:cc|hi|cs|ls)[sw]?\s+0x([0-9a-f]+)')
_JMP = re.compile(r'^jmp %pc@\(0x([0-9a-f]+),%d(\d):w\)')


def chain_at(ins, i):
    """The compare chain starting at ins[i], as ([(label, target)], index
    after the chain), or None.  `ins' is [(addr, len, text)]."""
    run, j, reg = [], i, None
    m = _LOAD.match(ins[i][2])
    regs = []
    if m and i + 1 < len(ins) and _BEQ.match(ins[i + 1][2]):
        # A copy `move.w %d0,%d1; beq' tests the value of both registers;
        # the compares may go on in the source (0xE59EF6, 0xE75656: `move.w
        # D0,D1; beq; cmpi.w #1,D0').
        regs = [(m.group(2), m.group(1))]
        src = re.match(r'^move[bwl] %(d\d),', ins[i][2])
        if src:
            regs.append((src.group(1), m.group(1)))
        run.append((0, int(_BEQ.match(ins[i + 1][2]).group(1), 16)))
        j += 2
    while j + 1 < len(ins):
        c, b = _CMP.match(ins[j][2]), _BEQ.match(ins[j + 1][2])
        if not (c and b):
            break
        r = (c.group(3), c.group(1)) if c.group(3) else (c.group(5), c.group(4))
        if reg is None and (not regs or r in regs):
            reg = r
        elif reg != r:
            break
        run.append((int(c.group(2)) if c.group(2) else 0, int(b.group(1), 16)))
        j += 2
    if len(run) < 2:
        return None
    return run, j


def sites():
    """[(kind, addr, routine name, labels, range, arms, width)]."""
    m = sau2map.load()
    out = []
    for seg in m.segments:
        if (seg.kind != 'I' or not (loopalign.LO <= seg.addr < loopalign.HI)
                or not seg.size or loopalign.is_asm(seg)):
            continue
        st = loopalign.routine_starts(m, seg)
        end = min(seg.end, loopalign.HI)
        for r0, r1 in zip(st, st[1:] + [end]):
            if r1 <= r0:
                continue
            names = m.names_at(r0)
            name = names[0] if names else 'module-local'
            ins = a5survey.reached(r0, r1)
            i = 0
            while i < len(ins):
                t = ins[i][2]
                jm = _JMP.match(t)
                if jm:
                    ents = a5survey.table_entries(
                        int(jm.group(1), 16), [x[2] for x in ins[:i]], r0, r1)
                    bcc = [x for x in ins[max(0, i - 7):i] if _BCC.match(x[2])]
                    dflt = int(_BCC.match(bcc[-1][2]).group(1), 16) if bcc else None
                    lab = sum(1 for e in ents if e != dflt)
                    out.append(('T', ins[i][0], name, lab, len(ents),
                                len(set(ents) - {dflt}), None))
                    i += 1
                    continue
                ch = chain_at(ins, i)
                if ch:
                    run, j = ch
                    after = ins[j][2] if j < len(ins) else ''
                    if _BRA.match(after) and len({tg for _, tg in run}) > 1:
                        ks = [k for k, _ in run]
                        out.append(('C', ins[i][0], name, len(ks), max(ks) - min(ks) + 1,
                                    len({tg for _, tg in run}), None))
                    i = j
                    continue
                i += 1
    return out


def summary(rows):
    t = [r for r in rows if r[0] == 'T']
    c = [r for r in rows if r[0] == 'C']
    print('jump tables: %d; compare chains: %d' % (len(t), len(c)))
    small = [r for r in c if r[3] <= 3]
    print('chains of 2-3 labels: %d (of them dense, r = n: %d)' % (
        len(small), sum(1 for r in small if r[4] == r[3])))
    print('tables with fewer than 4 counted labels: %d (lower bounds; see list)' % sum(
        1 for r in t if r[3] < 4))
    big = [r for r in c if r[3] >= 4]
    if t:
        worst = max(t, key=lambda r: r[4] / max(r[3], 1))
        print('tables: largest r/n %.2f (0x%X %s, %d/%d)' % (
            worst[4] / worst[3], worst[1], worst[2], worst[4], worst[3]))
        t4 = [r for r in t if r[3] >= 4]
        if t4:
            worst = max(t4, key=lambda r: r[4] / r[3])
            print('tables of 4 or more labels: largest r/n %.2f (0x%X %s, %d/%d)' % (
                worst[4] / worst[3], worst[1], worst[2], worst[4], worst[3]))
    for r in sorted(big, key=lambda r: r[4] / r[3]):
        print('chain of %d labels: r/n %.2f (0x%X %s, %d/%d)' % (
            r[3], r[4] / r[3], r[1], r[2], r[4], r[3]))


def main(argv):
    if not argv or argv[0] not in ('list', 'summary'):
        print(__doc__)
        return 2
    rows = sites()
    if argv[0] == 'list':
        for r in sorted(rows, key=lambda r: (r[3], r[4])):
            print('%s 0x%06X %-32s n=%-3d r=%-8d r/n=%-8.2f arms=%d' % (
                r[0], r[1], r[2], r[3], r[4], r[4] / max(r[3], 1), r[5]))
    else:
        summary(rows)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
