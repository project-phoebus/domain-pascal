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

`blocks' (bead domain-pascal-fbg, codegen-patterns 5.2): small block
copies, runs of adjacent `move.l'/`move.w' that move consecutive memory
to consecutive memory.  Two kinds of run are found over the linear code
of every compiled routine (no routine is skipped for the register model):
`pi', the source post-incremented (`(Am)+', the same Am throughout), the
destination `(An)+' (`pi-pi') or consecutive displacements from one base
(`pi-disp', `(An)' counting as displacement 0); and `disp', both sides
consecutive displacements (or absolute addresses) from one base each.  A
`pi' run followed by `dbf' is a copy loop and is left out.  Each run is
given its size in bytes, its move sizes (`l+l', `l+w'), the kinds of
base (`a5', `a6' (the frame), `abs', `reg' an address register other
than those), and, where both the source's post-incremented register (or
the source base register) and the destination base register are set up
in the run's statement (storesurvey's setup, back to the previous store,
call or branch), which side's setup starts first.  A `disp' run cannot
be told apart from adjacent assignments of the fields of a record.

usage:
    tools/storesurvey.py summary [--min N] [--max N]
    tools/storesurvey.py list [--min N] [--max N]   # one line per store
    tools/storesurvey.py blocks [SIZE ...] [-v]     # small block copies,
                                                    # all sizes or SIZE
                                                    # bytes; -v lists them

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import loopalign  # noqa: E402
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


_BO = re.compile(r'^%(a[0-7]|fp|sp)@(?:\((-?(?:0x)?[0-9a-f]+)\))?$')
_ABS = re.compile(r'^(0x[0-9a-f]+|[0-9]+)$')


def base_off(op):
    """(base, offset) for `(An)', `(d,An)' or an absolute address (base
    `abs'), else None."""
    m = _BO.match(op)
    if m:
        return regalloc.canon(m.group(1)), int(m.group(2) or '0', 0)
    m = _ABS.match(op)
    if m:
        return 'abs', int(m.group(1), 0)
    return None


def base_kind(b):
    return {'a5': 'a5', 'a6': 'a6', 'abs': 'abs'}.get(b, 'reg')


_MSIZE = {'movel': 4, 'movew': 2, 'moveb': 1}


def window_start(ins, i):
    """The first instruction after the last branch, call or label-free
    control transfer before i (a straight-line window)."""
    j = i
    while j > 0:
        y = ins[j - 1]
        if y.kind or y.call:
            break
        j -= 1
    return j


def side_setup(ins, i, reg):
    """Indices of the instructions before i that set up reg (a
    storesurvey setup over the straight-line window), or an empty set."""
    if reg in ('a5', 'a6', 'a7', 'abs'):
        return set()
    w = window_start(ins, i)
    return setup(FakeIns(ins), list(range(w, len(ins))), i - w, {reg})


class FakeIns:
    def __init__(self, ins):
        self.ins = ins


def small_copies(ins):
    """[dict] of the small block copy runs of a routine's instructions
    (regalloc.Insn); see the module doc, `blocks'."""
    res = []
    i = 0
    n = len(ins)
    while i < n:
        x = ins[i]
        if x.mn not in ('movel', 'movew') or len(x.ops) != 2:
            i += 1
            continue
        src, dst = x.ops
        pm = re.fullmatch(r'%(a[0-7])@\+', src)
        run = []
        if pm:
            j = i
            while (j < n and ins[j].mn in ('movel', 'movew') and len(ins[j].ops) == 2
                   and ins[j].ops[0] == src):
                run.append(ins[j])
                j += 1
            if j < n and ins[j].mn.startswith('db'):
                i = j + 1
                continue
            dsts = [y.ops[1] for y in run]
            dreg = re.fullmatch(r'%(a[0-7])@\+', dsts[0])
            if dreg and all(d == dsts[0] for d in dsts):
                form, db = 'pi-pi', dreg.group(1)
            else:
                bo = [base_off(d) for d in dsts]
                off, ok = None, all(bo) and len({b[0] for b in bo}) == 1
                if ok:
                    off = bo[0][1]
                    for y, b in zip(run, bo):
                        if b[1] != off:
                            ok = False
                        off += _MSIZE[y.mn]
                form, db = ('pi-disp', bo[0][0]) if ok else ('pi-other', None)
            sb = pm.group(1)
        else:
            sbo, dbo = base_off(src), base_off(dst)
            if not sbo or not dbo or dbo[0] == 'a7':
                i += 1
                continue
            so, do = sbo[1], dbo[1]
            j = i
            while j < n and ins[j].mn in ('movel', 'movew') and len(ins[j].ops) == 2:
                a, b = base_off(ins[j].ops[0]), base_off(ins[j].ops[1])
                if a != (sbo[0], so) or b != (dbo[0], do):
                    break
                run.append(ins[j])
                so += _MSIZE[ins[j].mn]
                do += _MSIZE[ins[j].mn]
                j += 1
            if len(run) < 2:
                i += 1
                continue
            form, sb, db = 'disp', sbo[0], dbo[0]
        size = sum(_MSIZE[y.mn] for y in run)
        ds = side_setup(ins, i, db) if db else set()
        ss = side_setup(ins, i, sb)
        first = None
        if ds and ss and not (ds & ss):
            first = 'dst' if min(ds) < min(ss) else 'src'
        res.append({'addr': x.addr, 'size': size, 'form': form,
                    'moves': '+'.join(y.mn[-1] for y in run),
                    'src': base_kind(sb), 'dst': base_kind(db) if db else '?',
                    'sreg': sb, 'dreg': db, 'first': first,
                    'setup': [ins[k].text for k in sorted(ds | ss)],
                    'text': '; '.join(y.text for y in run)})
        i += len(run)
    return res


def blocks_main(argv):
    verbose = '-v' in argv
    sizes = {int(a, 0) for a in argv if a != '-v'}
    _, rs = regalloc.routines()
    allc = []
    for seg, r0, r1 in rs:
        ins = [regalloc.Insn(a, k, t) for a, k, t in loopalign.disasm(r0, r1 - r0)]
        for c in small_copies(ins):
            if not sizes or c['size'] in sizes:
                c['routine'] = r0
                allc.append(c)
    if verbose:
        for c in allc:
            print('%06X (r %06X) %2d %-8s %-5s src=%-3s dst=%-3s first=%-4s %s   | %s' % (
                c['addr'], c['routine'], c['size'], c['form'], c['moves'], c['src'],
                c['dst'], c['first'], c['text'], '; '.join(c['setup'])))
        return 0
    print('routines %d; small copy runs: %d' % (len(rs), len(allc)))
    by = Counter((c['size'], c['form'], c['moves'], c['src'], c['dst']) for c in allc)
    print('size form     moves  src  dst   count   dst-first src-first  e.g.')
    for k in sorted(by):
        ex = [c for c in allc if (c['size'], c['form'], c['moves'], c['src'], c['dst']) == k]
        f = Counter(c['first'] for c in ex)
        print('%4d %-8s %-6s %-4s %-4s %6d   %6d %6d     %s' % (
            k + (by[k], f['dst'], f['src'],
                 ', '.join('0x%06X' % c['addr'] for c in ex[:3]))))
    return 0


def main(argv):
    if argv and argv[0] == 'blocks':
        return blocks_main(argv[1:])
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
