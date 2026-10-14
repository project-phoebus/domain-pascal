#!/usr/bin/env python3
"""storesurvey.py - stores whose destination and value both need a
register: which side the compiler set up first (bead domain-pascal-k1sv).

Evidence tool for docs/codegen-patterns.md 12.4 ("Stores whose
destination and value both need a register").  Routines and blocks are
those of tools/regalloc.py (compiled code only, `link.w A6' routines,
reachable instructions, basic blocks).  A *store* is a `move.<s> SRC,DST'
whose DST is memory other than a push (`-(SP)').  Each side's *setup* is
the set of instructions of the store's block that compute the registers
the side reads (DST: its address registers; SRC: its address registers,
or the data register itself when SRC is one), followed back through the
block transitively (an instruction's own register operands are set up by
the instructions before it), back to the previous store or call (a
statement boundary, codegen-patterns 12.4 R10).  A5, A6 and A7 are not set-up registers.
Only stores where both sides have a non-empty setup, and the two setups
share no instruction, are counted.

Per store: the shapes (DST `ptr' = `(An)'/`(d,An)', `idx' = with an
index register, `abs'; SRC `dreg' a computed data register, `ptr',
`idx'), which side's setup starts first in address order, the sizes of
the setups (instructions), and whether the two interleave (the later
side's first instruction comes before the earlier side's last).

usage:
    tools/storesurvey.py summary [--min N] [--max N]
    tools/storesurvey.py list [--min N] [--max N]   # one line per store

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import regalloc  # noqa: E402

SETUP = set(regalloc.DREGS) | set(regalloc.AREGS)


def shape(op):
    """`dreg', `areg', `ptr', `idx', `abs' or `imm' for an operand."""
    r = regalloc.is_reg(op)
    if r:
        return 'dreg' if r[0] == 'd' else 'areg'
    if op.startswith('#'):
        return 'imm'
    if '@' not in op:
        return 'abs'
    return 'idx' if re.search(r',%[ad]\d', op) else 'ptr'


def side_regs(op):
    """The set-up registers an operand reads (its value for a register)."""
    r = regalloc.is_reg(op)
    if r:
        return {r} & SETUP
    return regalloc.ea_regs(op) & SETUP


def is_store(y):
    """A write to memory other than a push (a statement's end, R10)."""
    if y.call:
        return True
    if not y.ops or y.mn.startswith(('cmp', 'tst', 'btst', 'pea', 'lea')):
        return False
    d = y.ops[-1]
    return regalloc.is_mem(d) and not d.endswith('@-')


def setup(rt, blk, k, regs):
    """Indices (into rt.ins) of the instructions of blk before position k
    that compute regs, transitively, back to the previous store or call
    (the statement before, R10)."""
    out, want = set(), set(regs)
    for p in range(k - 1, -1, -1):
        y = rt.ins[blk[p]]
        if is_store(y):
            break
        hit = y.defs & want
        if not hit:
            continue
        out.add(blk[p])
        # a full (non-update) definition ends the register's chain
        for r in hit:
            if r not in y.upd and r not in y.partial:
                want.discard(r)
        want |= y.uses & SETUP
    return out


def stores(rt, blks=None):
    """[(addr, dst shape, src shape, first, n_dst, n_src, interleaved,
    text)] (blks: the blocks, default regalloc.blocks(rt))."""
    res = []
    for blk in (regalloc.blocks(rt) if blks is None else blks):
        for k, i in enumerate(blk):
            x = rt.ins[i]
            if not re.match(r'^move[bwl]$', x.mn) or len(x.ops) != 2:
                continue
            src, dst = x.ops
            if not regalloc.is_mem(dst) or dst.endswith('@-') or dst.endswith('@+'):
                continue
            if src.endswith('@+') or src.endswith('@-'):
                continue
            d = setup(rt, blk, k, side_regs(dst))
            s = setup(rt, blk, k, side_regs(src))
            if not d or not s or (d & s):
                continue
            # a call in between: the value is a call's result (not counted)
            lo = min(d | s)
            if any(rt.ins[j].call for j in blk if lo <= j < i):
                continue
            first = 'dst' if min(d) < min(s) else 'src'
            a, b = (d, s) if first == 'dst' else (s, d)
            inter = min(b) < max(a)
            res.append((x.addr, shape(dst), shape(src), first, len(d), len(s),
                        inter, x.text))
    return res


def predicted(n_dst, n_src):
    """The side dpc sets up first: the one with more instructions, the
    value (`src') on a tie."""
    return 'dst' if n_dst > n_src else 'src'


def main(argv):
    lo, hi, _, argv = regalloc.opts(argv)
    if not argv or argv[0] not in ('summary', 'list'):
        print(__doc__)
        return 2
    m, done, skipped = regalloc.load_all(lo, hi)
    allst = []
    for seg, rt in done:
        allst += stores(rt)
    if argv[0] == 'list':
        for s in allst:
            print('%06X dst=%-4s src=%-4s first=%s setup dst %d src %d%s  %s' % (
                s[0], s[1], s[2], s[3], s[4], s[5], ' interleaved' if s[6] else '', s[7]))
        return 0
    print('routines %d (skipped %d); stores with both sides set up: %d' % (
        len(done), sum(skipped.values()), len(allst)))
    c = Counter((s[1], s[2], s[3]) for s in allst)
    print('by shape (dst, src) and the side set up first:')
    for (dsh, ssh) in sorted({(k[0], k[1]) for k in c}):
        nd, ns = c[dsh, ssh, 'dst'], c[dsh, ssh, 'src']
        ex = [s for s in allst if s[1] == dsh and s[2] == ssh]
        print('  dst=%-4s src=%-4s  dst first %4d  src first %4d   e.g. %s' % (
            dsh, ssh, nd, ns, ', '.join('0x%06X' % s[0] for s in ex[:4])))
    big = Counter()
    for s in allst:
        bigger = 'dst' if s[4] > s[5] else 'src' if s[5] > s[4] else 'equal'
        big[bigger, s[3]] += 1
    print('the larger setup (instructions) and the side set up first:')
    for b in ('dst', 'src', 'equal'):
        print('  larger %-5s  dst first %4d  src first %4d' % (b, big[b, 'dst'], big[b, 'src']))
    rule = sum(1 for s in allst if s[3] == predicted(s[4], s[5]))
    print('rule "the side with more set-up instructions first, the value on a tie": '
          '%d of %d' % (rule, len(allst)))
    print('interleaved setups: %d' % sum(1 for s in allst if s[6]))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
