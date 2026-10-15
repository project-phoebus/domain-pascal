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
    tools/storesurvey.py merge [-v]                 # adjacent stores and
                                                    # longword constants

`merge' (bead domain-pascal-2m0, codegen-patterns 13 "Store merging"):
pairs of stores to adjacent memory of one base, made one after the other
(`move', `clr', `st'/`sf' to memory other than a push or `(An)+'; only
register set-up between them, no store, call, branch or branch target),
whose sizes add up to a word or a longword: the stores the original
compiler did not merge (ref 3.15.3 p. 3-57).  Pairs of a word and a
longword are counted apart (FIM_$BUILD_DF 0xE0A896 is such a pair of
constants merged and split again at the other boundary; one with the
word at the low address is not).  Each pair is classified by
its sizes, the kind of base (`a5', `a6' the frame, `abs', `reg' an
address register), the values (`zero', `const', `reg' a computed
register, `mem'), the order of the addresses (`asc', `desc') and the
offset of the pair's low address modulo 4 (for `reg', from the
register).  A pair whose base register is loaded again between the two
stores is not counted.  Then the candidates for *merged* stores: a
`move.l #imm' (or `moveq #imm,Dn; move.l Dn') with both halves non-zero
(of a longword object these values are unusual; the merged pairs found
are all such), and a `move.w #imm' with both bytes non-zero, listed
for checking against ../source (a `clr.l' of two words cannot be told
from a longword store and is not listed).

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


_SIZE = {'b': 1, 'w': 2, 'l': 4}


def mem_store(x):
    """(size, value kind, value text, destination) of a plain store to
    memory (`move', `clr', `st'/`sf'), or None."""
    mn, ops = x.mn, x.ops
    if re.fullmatch(r'move[bwl]', mn) and len(ops) == 2:
        src, dst = ops
        if src in ('%sr', '%ccr', '%usp'):
            return None
        if src.startswith('#'):
            v = int(src[1:], 0)
            kind = 'zero' if v == 0 else 'const'
        elif regalloc.is_reg(src):
            kind = 'reg'
        else:
            kind = 'mem'
        size = _SIZE[mn[-1]]
    elif re.fullmatch(r'clr[bwl]', mn) and len(ops) == 1:
        dst, kind, src, size = ops[0], 'zero', '#0', _SIZE[mn[-1]]
    elif mn in ('st', 'sf') and len(ops) == 1:
        dst, size = ops[0], 1
        kind, src = ('const', '#255') if mn == 'st' else ('zero', '#0')
    else:
        return None
    if not regalloc.is_mem(dst) or dst.endswith('@-') or dst.endswith('@+'):
        return None
    return size, kind, src, dst


def moveq_const(ins, i, reg):
    """The constant a `moveq #n,reg' between the previous store or call
    and i loaded into reg (and nothing redefined it since), or None."""
    for j in range(i - 1, -1, -1):
        y = ins[j]
        if y.call or y.kind or mem_store(y):
            return None
        if reg in y.defs:
            if y.mn in ('moveq', 'moveql') and y.ops[1] == '%' + reg:
                return int(y.ops[0][1:], 0)
            return None
    return None


def base_def(ins, i, reg):
    """The text of the last instruction before i (in address order) that
    writes reg, or '' (A5, A6, absolute bases, or none found)."""
    if reg in ('a5', 'a6', 'a7', 'abs'):
        return ''
    for j in range(i - 1, -1, -1):
        if reg in ins[j].defs:
            return ins[j].text
    return ''


def adjacent_pairs(ins, sums=(2, 4)):
    """[dict] of the pairs of consecutive stores to adjacent memory of
    one base whose sizes add up to one of `sums' (see the module doc,
    `merge')."""
    labels = {y.target for y in ins if y.target is not None}
    res = []
    prev = None          # (index, size, kind, value, base, offset)
    for i, x in enumerate(ins):
        if i and x.addr in labels:
            prev = None
        st = mem_store(x)
        if st is None:
            if x.call or x.kind or (x.ops and regalloc.is_mem(x.ops[-1])
                                    and not x.mn.startswith(('cmp', 'tst', 'btst', 'pea',
                                                             'lea', 'jmp'))
                                    and not x.ops[-1].endswith(('@-', '@+'))):
                prev = None
            elif prev and prev[4] in x.defs:
                prev = None          # the base register loaded again
            continue
        size, kind, src, dst = st
        bo = base_off(dst)
        if bo is None:
            prev = None
            continue
        if kind == 'reg':
            c = moveq_const(ins, i, regalloc.is_reg(src))
            if c is not None:
                kind = 'zero' if c == 0 else 'const'
        if prev:
            j, psize, pkind, psrc, pbase, poff = prev
            base, off = bo
            if base == pbase and psize + size in sums and (
                    poff + psize == off or off + size == poff):
                lo = min(off, poff)
                mod = lo % 4
                copy = False
                if pkind == 'mem' and kind == 'mem':
                    a, b = base_off(psrc), base_off(src)
                    copy = bool(a and b and a[0] == b[0] and b[1] - a[1] == off - poff)
                res.append({'addr': ins[j].addr, 'sizes': '%d+%d' % (psize, size),
                            'basedef': base_def(ins, j, base),
                            'base': base_kind(base), 'reg': base,
                            'values': '%s+%s' % (pkind, kind),
                            'order': 'asc' if off > poff else 'desc', 'mod4': mod,
                            'copy': copy,
                            'text': '%s; %s' % (ins[j].text, x.text)})
        prev = (i, size, kind, src, bo[0], bo[1])
    return res


def merged_candidates(ins):
    """[dict] of the stores of a longword constant with both halves
    non-zero, or of a word constant with both bytes non-zero (see the
    module doc, `merge')."""
    res = []
    for i, x in enumerate(ins):
        st = mem_store(x)
        if st is None:
            continue
        size, kind, src, dst = st
        if size == 1 or base_off(dst) is None:
            continue
        if kind == 'reg':
            v = moveq_const(ins, i, regalloc.is_reg(src))
        elif src.startswith('#'):
            v = int(src[1:], 0)
        else:
            v = None
        if v is None:
            continue
        half = 16 if size == 4 else 8
        mask = (1 << half) - 1
        hi, lo = (v >> half) & mask, v & mask
        if hi and lo:
            res.append({'addr': x.addr, 'size': size, 'base': base_kind(base_off(dst)[0]),
                        'value': v & ((1 << 2 * half) - 1), 'text': x.text})
    return res


def merge_main(argv):
    verbose = '-v' in argv
    _, rs = regalloc.routines()
    pairs, cands = [], []
    for seg, r0, r1 in rs:
        ins = [regalloc.Insn(a, k, t) for a, k, t in loopalign.disasm(r0, r1 - r0)]
        for p in adjacent_pairs(ins):
            p['routine'] = r0
            pairs.append(p)
        for c in merged_candidates(ins):
            c['routine'] = r0
            cands.append(c)
    print('routines %d; unmerged adjacent store pairs: %d' % (len(rs), len(pairs)))
    key = ('sizes', 'base', 'values', 'order', 'mod4')
    by = Counter(tuple(p[k] for k in key) for p in pairs)
    print('sizes base values      order mod4  count  e.g.')
    for k in sorted(by, key=str):
        ex = [p for p in pairs if tuple(p[f] for f in key) == k]
        print('%-5s %-4s %-12s %-5s %d %6d  %s' % (k + (by[k], ', '.join(
            '0x%06X' % p['addr'] for p in ex[:3]))))
    def cls(p):
        if p['mod4'] % 2:
            return 'odd'
        v = p['values'].split('+')
        if all(x in ('zero', 'const') for x in v):
            return 'constants'
        if p['copy']:
            return 'copy'
        return 'other'
    byk = Counter(cls(p) for p in pairs)
    print('by class: constants (low address even) %d, adjacent memory copied %d, '
          'other values %d, low byte odd %d' % (byk['constants'], byk['copy'],
                                                byk['other'], byk['odd']))
    six = [p for seg, r0, r1 in rs for p in adjacent_pairs(
        [regalloc.Insn(a, k, t) for a, k, t in loopalign.disasm(r0, r1 - r0)], (6,))]
    sixc = [p for p in six if all(v in ('zero', 'const') for v in p['values'].split('+'))]
    wlo = [p for p in sixc if p['sizes'] == ('2+4' if p['order'] == 'asc' else '4+2')]
    print('word and longword pairs: %d, of two constants %d, the word at the low address '
          '%d (%s)' % (len(six), len(sixc), len(wlo),
                       ', '.join('0x%06X' % p['addr'] for p in wlo[:4])))
    print('merge candidates (constant halves both non-zero): %d' % len(cands))
    byc = Counter((c['size'], c['base']) for c in cands)
    for k in sorted(byc):
        print('  size %d base %-4s %4d' % (k + (byc[k],)))
    if verbose:
        for p in pairs:
            print('pair %06X (r %06X) %s %s %s %s mod4=%d %s  %s   [%s]' % (
                p['addr'], p['routine'], p['sizes'], p['base'], p['values'],
                p['order'], p['mod4'], cls(p), p['text'], p['basedef']))
        for c in cands:
            print('cand %06X (r %06X) %d %s %#x  %s' % (
                c['addr'], c['routine'], c['size'], c['base'], c['value'], c['text']))
    return 0


def main(argv):
    if argv and argv[0] == 'blocks':
        return blocks_main(argv[1:])
    if argv and argv[0] == 'merge':
        return merge_main(argv[1:])
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
