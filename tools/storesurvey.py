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
    tools/storesurvey.py sites [-v]                 # merged sites: one
                                                    # object or two?

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

`sites' (bead domain-pascal-2m0): the test of "the compiler never merges
two statements' stores; every merged site is one assignment of a 4-byte
(or larger) object".  MERGED_SITES lists the image's stores that ../source
reads as two or more narrower assignments merged, each with the objects
it covers (offset, size, name: from Apollo's inserts where the type is a
system type, else ../source's declarations) and the image instructions
that reach one of those objects alone (`apart').  The tool checks every
listed instruction text against the image; the objects, layouts and
`src' ranges are entered by hand from ../source's and Apollo's
declarations and are not checked.  It says which declared objects each
merged store covers (two or more, or part of one), and gives the verdict:
`two objects' when an access apart is in the image or the layout is
Apollo's (then no one assignment of the original covers the store: the
reference has no record constants: "A value must be a real, integer,
string, char, or set constant expression", ref 2.2.2.2, p. 2-12), else
`one object possible'; a merged copy whose source range overlaps another
merged copy's in the same type without being the same is `overlaps' (not
both can be one object of a type without variants: ROUTE_$SHORT_PORT
0xE69C16 against HINT_$ADD_NET 0xE49CA6, assuming both copy from the one
type route_$port_t).  REFUSAL_SITES lists, for
each statement of ../source (at bead time, ../source 980aa83a) that dpc
refused as "a store adjacent to the previous statement's store", the
image's instructions for the pair and whether they are merged.  Then the
counter-examples the other way: the unmerged constant pairs of `merge'
(every merge condition holds: one base, adjacent, even, constants, one
after the other, no branch target between), by map segment, and the
segments that show both.

`addr' (bead domain-pascal-j28d, codegen-patterns 5.2 "`addr(x)` as a
value"): addresses used as data in compiled code, over the instructions
of every compiled routine reachable from its start (a5survey.reached).
Two kinds of site:

- `areg': an address register read as a value (not as a base): `move.l
  An,Dm', `add.l An,Dm', `sub.l An,Dm', `cmp.l An,Dm', `cmpa.l X,An',
  `movea.l An,Am', `move.l An,<mem>', whose last definition in the
  straight-line run before it (no branch or call between) is
  `lea X,An' (not an update `lea (d,An),An') or `movea.l #imm,An'
  (a branch target in between is not looked for).
  Classed by the use, the destination (`dreg', `areg', `mem') and the
  `lea' source (`a5', `a6' the frame, `abs', `ptr' (d,An), `idx' with an
  index register, `imm' a `movea.l #').  A `movea.l A0,A0' (the filler)
  is not a use.
- `imm': `add', `sub', `cmp' (and `addi', `subi', `cmpi') of a longword
  immediate into a data register, and `cmpa.l #imm,An', the immediate in
  the window 0xD00000..0xFFFFFF, by encoding (`ea' the
  immediate-EA form `d0bc'/`90bc'/`b0bc', what objdump writes `addl
  #'/`subl #'/`cmpl #'; `i' the ADDI/SUBI/CMPI form) and whether the
  immediate is a map symbol's own address (`sym') or not (`num').

    tools/storesurvey.py addr [-v]     # -v lists the sites

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


# ------------------------------------------------------------- sites
# `sites' (bead domain-pascal-2m0): see the module doc.  Each merged site:
# addr, the image's instructions from there, the stores they make
# (offset, size) from the base, the objects of the declared layout
# (offset, size, name), where the layout comes from, and the image's
# accesses of one of the objects alone (addr, instruction).
APOLLO = 'Apollo'
_TTY = '../source tty/tty.ins.pas tty_$desc_t'
_TTY_BUF = ((0x2CA, 2, 'input_head'), (0x2CC, 2, 'input.head'), (0x2CE, 2, 'input.tail'),
            (0x2D0, 2, 'input.size'), (0x3D2, 2, 'output.head'), (0x3D4, 2, 'output.tail'),
            (0x3D6, 2, 'output.size'))
_TTY_I_INIT = tuple((a, 'movew #%d,%%a4@(%d)' % (v, o)) for a, v, o in (
    (0xE3332E, 1, 0x2CA), (0xE33334, 1, 0x2CC), (0xE3333A, 1, 0x2CE),
    (0xE33340, 256, 0x2D0), (0xE33346, 1, 0x3D2), (0xE3334C, 1, 0x3D4),
    (0xE33352, 256, 0x3D6)))
_ACL_HDR = ((0x22, 2, 'owner_slot'), (0x24, 2, 'group_slot'), (0x26, 2, 'org_slot'))
_ACL_APART = ((0xE45384, 'movew %d2,%a1@(34)'), (0xE45562, 'movew %d2,%a1@(36)'))
_OPEN_OPT = ((-40, 2, 'opt.socket'), (-38, 2, 'opt.flags'))
_HEAD_TAIL = ((0, 2, 'head'), (2, 2, 'tail'))
_PROC1_PCB = ((0x56, 2, 'inh_count'), (0x58, 2, 'sw_bsr'))
_PROC1_APART = ((0xE15284, 'movew %d0,%a4@(86)'), (0xE15290, 'movew %d0,%a4@(88)'))
_NO_PAS = 'unknown (no Pascal in ../source)'


def _site(addr, kind, texts, stores, layout, decl, apart=(), note='', src=None):
    return dict(addr=addr, kind=kind, texts=texts, stores=stores, layout=layout,
                decl=decl, apart=apart, note=note, src=src)


def _PORT(off):
    return ('route_$port_t', off, 4)


# route_$port_t's active (0x2C), port_type (0x2E), socket (0x30) reached
# alone: MAC_$OPEN tests active's value, MAC_$DEMUX pushes socket and
# port_type apart.
_ROUTE_APART = ((0xE0B8FE, 'movew %a3@(2c,%d1:w),%d2'), (0xE0BCFA, 'movew %a3@(48),%sp@-'),
                (0xE0BCFE, 'movew %a3@(46),%sp@-'))


MERGED_SITES = (
    # constants
    _site(0xE1BFE4, 'const', ('movel #65537,%a2@(56)',), ((0x38, 4),),
          ((0x38, 2, 'break_mode.break_type'), (0x3A, 2, 'break_mode.count')),
          APOLLO + ' sys/ins/tty.ins.pas tty_$input_break_mode_t (tag, count: pinteger)',
          ((0xE1C04C, 'clrw %a2@(56)'), (0xE1BD86, 'movew %a0@(58),%d2'))),
    _site(0xE67322, 'const',
          ('movel #65537,%a2@(714)', 'movel #65792,%a2@(718)',
           'movel #65537,%a2@(978)', 'movew #256,%a2@(982)'),
          ((0x2CA, 4), (0x2CE, 4), (0x3D2, 4), (0x3D6, 2)), _TTY_BUF, _TTY, _TTY_I_INIT,
          'crosses input_head|input.head; one buffer type split tail+size (input) '
          'and head+tail (output); TTY_$I_INIT stores all seven apart'),
    _site(0xE67340, 'const', ('clrl %a2@(86)',), ((0x56, 4),),
          ((0x56, 2, 'saved_input_flags'), (0x58, 2, 'column')), _TTY,
          ((0xE332CC, 'clrw %a0@(88)'), (0xE332D0, 'clrw %a0@(86)'))),
    _site(0xE176B0, 'const', ('movel #65537,%a0@',), ((0, 4),), _HEAD_TAIL,
          '../source log/log.ins.pas',
          ((0xE176C0, 'movew %a0@,%d1'), (0xE176D8, 'cmpw %a0@(2),%d1'))),
    _site(0xE3017C, 'const', ('moveq #1,%d0', 'movel %d0,%a0@'), ((0, 4),), _HEAD_TAIL,
          '../source log/log.ins.pas', ((0xE30172, 'tstw %a0@(2)'),)),
    _site(0xE33374, 'const', ('clrl %a0@(88)',), ((0x58, 4),),
          ((0x58, 2, 'term_$tpad_buffer.head'), (0x5A, 2, 'term_$tpad_buffer.tail')),
          '../source kbd/kbd.ins.pas'),
    _site(0xE3337C, 'const', ('clrl %a2@(58)',), ((0x3A, 4),),
          ((0x38, 2, 'state'), (0x3A, 2, 'sub_state'), (0x3C, 2, 'kbd_type_idx'),
           (0x3E, 2, 'pending_mode')), '../source kbd/kbd.ins.pas',
          ((0xE1CB20, 'movew %a4@(58),%d4'), (0xE1CBF2, 'movew %d4,%a4@(58)'),
           (0xE1CE2A, 'movew %a2@(60),%d1')),
          '0x38 and 0x3E, cleared just before and after, stay apart'),
    _site(0xE333B6, 'const', ('movel #65537,%a2@(88)', 'movew #64,%a2@(92)'),
          ((0x58, 4), (0x5C, 2)),
          ((0x58, 2, 'ring.head'), (0x5A, 2, 'ring.tail'), (0x5C, 2, 'ring.size')),
          '../source kbd/kbd.ins.pas',
          ((0xE1CBEE, 'movew %d2,%a4@(88)'), (0xE1CC30, 'movew %a2@(90),%d1'))),
    _site(0xE333C4, 'const', ('movel #65537,%a2@(158)', 'movew #64,%a2@(162)'),
          ((0x9E, 4), (0xA2, 2)),
          ((0x9E, 2, 'txbuf.head'), (0xA0, 2, 'txbuf.tail'), (0xA2, 2, 'txbuf.size')),
          '../source kbd/kbd.ins.pas'),
    _site(0xE14DA6, 'const', ('movel #65552,%a2@(86)',), ((0x56, 4),), _PROC1_PCB,
          '../source proc1/proc1.ins.pas', _PROC1_APART),
    _site(0xE2F998, 'const', ('movel #65552,%a0@(86)',), ((0x56, 4),), _PROC1_PCB,
          '../source proc1/proc1.ins.pas', _PROC1_APART),
    _site(0xE15004, 'const',
          ('lea %a2@(2),%a0', 'clrl %a0@+', 'clrl %a0@+', 'clrl %a0@+', 'clrw %a0@+'),
          ((2, 4), (6, 4), (10, 4), (14, 2)),
          ((2, 2, 'usr'), (4, 4, 'upc'), (8, 4, 'usp'), (12, 4, 'usb')),
          '../source proc1/proc1.ins.pas',
          ((0xE14FDA, 'pea %a2@(8)'), (0xE14FDE, 'pea %a2@(12)'),
           (0xE14FE2, 'pea %a2@(4)'), (0xE14FE6, 'pea %a2@(2)')),
          '14 bytes cleared l,l,l,w; the fields are w,l,l,l'),
    _site(0xE65EBC, 'const', ('movel #65726,%fp@(-540)', 'clrw %fp@(-536)'),
          ((-540, 4), (-536, 2)), (), _NO_PAS),
    _site(0xE65ED0, 'const', ('movel #196678,%fp@(-512)', 'clrl %fp@(-508)'),
          ((-512, 4), (-508, 4)), (), _NO_PAS),
    _site(0xE2F590, 'const', ('movel #65537,%a0@(4)',), ((4, 4),),
          ((4, 2, 'n_link_addrs'), (6, 2, 'n_net_addrs'), (8, 2, 'n_words')),
          '../source route/route.ins.pas'),
    _site(0xE0A896, 'const', ('clrl %a2@(84)', 'clrw %a2@(88)'), ((0x54, 4), (0x58, 2)),
          ((0x54, 2, 'kernel_sr_pc.sr'), (0x56, 4, 'kernel_sr_pc.pc')),
          '../source fim/fim.ins.pas fim_$sr_pc_t',
          ((0xE0A87A, 'movew %d1,%a2@(84)'), (0xE0A87E, 'movel %fp@(-62),%a2@(86)')),
          'a word and a longword cleared l+w; the other arm stores w, l'),
    _site(0xE4521E, 'const', ('clrl %a1@(34)', 'clrw %a1@(38)'), ((0x22, 4), (0x26, 2)),
          _ACL_HDR, '../source acl/acl.pas', _ACL_APART),
    _site(0xE45C1C, 'const', ('clrl %a1@(34)', 'clrw %a1@(38)'), ((0x22, 4), (0x26, 2)),
          _ACL_HDR, '../source acl/acl.pas', _ACL_APART,
          'the bytes 0x28, 0x29 cleared apart after it (0xE45C38, 0xE45C3C)'),
    _site(0xE44EBE, 'const', ('clrl %a2@(34)', 'clrw %a2@(38)'), ((0x22, 4), (0x26, 2)),
          _ACL_HDR, '../source acl/acl.pas (acl.ins.pas: reserved_22, integer32)', _ACL_APART),
    _site(0xE47DD0, 'const', ('clrl %a1@(34)', 'clrw %a1@(38)'), ((0x22, 4), (0x26, 2)),
          _ACL_HDR, '../source acl/acl.pas (acl.ins.pas: reserved_22, integer32)', _ACL_APART),
    _site(0xE00BA8, 'const', ('movel #77135874,%fp@(-40)',), ((-40, 4),), _OPEN_OPT,
          '../source xns/xns.ins.pas', ((0xE00BD8, 'movew %fp@(-38),%a5@(20)'),)),
    _site(0xE15ABA, 'const', ('movel #65538,%fp@(-40)',), ((-40, 4),), _OPEN_OPT,
          '../source xns/xns.ins.pas', ((0xE15AE8, 'movew %fp@(-38),%a5@(3172)'),)),
    _site(0xE178D2, 'const', ('movel #196616,%fp@(-40)',), ((-40, 4),), _OPEN_OPT,
          '../source xns/xns.ins.pas', ((0xE178F0, 'movew %fp@(-38),%a5@(118)'),)),
    _site(0xE71032, 'const', ('clrl %a3@(66)',), ((0x42, 4),),
          ((0x42, 2, 'uppid'), (0x44, 2, 'upgid')), '../source audit/audit.ins.pas',
          ((0xE71012, 'pea %a3@(66)'), (0xE71016, 'pea %a3@(68)'))),
    _site(0xE0EA1A, 'const', ('clrl %a4@(1000)',), ((1000, 4),),
          ((1000, 2, 'data_len'), (1002, 2, 'hdr_len')), '../source netbuf/netbuf.ins.pas',
          (), 'every access to 1000 is a longword (0xE0EE9A, 0xE16108, 0xE16308)'),
    _site(0xE59886, 'const', ('movel #65535,%a3@(10)',), ((10, 4),),
          ((10, 2, 'reserved2'), (12, 2, 'reserved3')), '../source msg/msg.ins.pas'),
    _site(0xE595EE, 'const', ('movel #65535,%a3@(10)',), ((10, 4),),
          ((10, 2, 'reserved2'), (12, 2, 'reserved3')), '../source msg/msg.ins.pas'),
    _site(0xE68EDA, 'const', ('movel #65536,%a2@(18)',), ((18, 4),),
          ((18, 2, 'port_type'), (20, 2, 'port_socket')), '../source rip/rip.ins.pas',
          ((0xE68ECA, 'movew %a4@(46),%a2@(18)'), (0xE68ED0, 'movew %a4@(48),%a2@(20)')),
          'the other arm copies the two words apart'),
    _site(0xE3E1FC, 'const',
          ('lea %pc@(0xe3e21e),%a0', 'lea %a3@,%a1', 'movel %a0@+,%a1@+',
           'movel %a0@+,%a1@+', 'movew %a0@+,%a1@+'),
          ((0, 4), (4, 4), (8, 2)),
          ((0, 4, 'l_00'), (4, 2, 'w_04'), (6, 2, 'w_06'), (8, 2, 'w_08')),
          '../source flp/flp.ins.pas', ((0xE3E208, 'movew #1,%a3@(6)'),),
          '10 bytes copied from a code constant (0xE3E21E)'),
    _site(0xE19D8A, 'const',
          ('clrl %fp@(-24)', 'lea %fp@(-20),%a1', 'clrl %a1@+', 'clrl %a1@+',
           'clrl %a1@+', 'clrl %a1@+', 'clrw %a1@+'),
          ((0, 4), (4, 4), (8, 4), (12, 4), (16, 4), (20, 2)),
          ((0, 4, 'xfer_other'), (4, 4, 'xfer_op2'), (8, 2, 'not_ready'), (10, 2, 'w_0a'),
           (12, 2, 'seek_errors'), (14, 2, 'equip_checks'), (16, 2, 'w_10'),
           (18, 2, 'data_checks'), (20, 2, 'dma_overruns')),
          '../source win/win.ins.pas win_$cnt_t (offsets from the local)',
          ((0xE1910E, 'addqw #1,%a5@(72)'),), 'WIN_$CNT (A5+0x40) bumps not_ready alone'),
    # copies: `src' is the source's (type, offset, size)
    _site(0xE69C16, 'copy', ('movel %a0@(44),%a1@(4)',), ((4, 4),),
          ((4, 2, 'short_info.status'), (6, 2, 'short_info.port_type')),
          '../source route/route.ins.pas route_$short_port_t', _ROUTE_APART,
          src=_PORT(0x2C)),
    _site(0xE49CA6, 'copy', ('movel %a1@(46),%a0@(8)',), ((8, 4),),
          ((8, 2, 'port_type'), (10, 2, 'socket')), '../source hint/hint.ins.pas',
          _ROUTE_APART, src=_PORT(0x2E)),
    _site(0xE311D8, 'copy', ('movel %a2@(46),%a0@(8)',), ((8, 4),),
          ((8, 2, 'port_type'), (10, 2, 'socket')), '../source hint/hint.ins.pas',
          _ROUTE_APART, src=_PORT(0x2E)),
    _site(0xE6905E, 'copy', ('movel %a4@(46),%fp@(-14)',), ((-14, 4),),
          ((-14, 2, 'd_buf.port_type'), (-12, 2, 'd_buf.port_socket')),
          '../source rip/rip.ins.pas', _ROUTE_APART, src=_PORT(0x2E)),
    _site(0xE16108, 'copy', ('movel %a1@(1000),%a2@(42)',), ((42, 4),),
          ((42, 2, 'info.data_len'), (44, 2, 'info.hdr_len')), '../source sock/sock.ins.pas',
          src=('netbuf_$hdr_page_t', 1000, 4)),
    _site(0xE1628A, 'copy', ('movel %a2@(42),%a0@(1000)',), ((1000, 4),),
          ((1000, 2, 'data_len'), (1002, 2, 'hdr_len')), '../source netbuf/netbuf.ins.pas',
          src=('sock info', 42, 4)),
)

# The statements dpc refused as "a store adjacent to the previous
# statement's store" in ../source 980aa83a (the second statement's line;
# rip/table.pas:175 and netlog/log_it.pas:156 are the next pairs of their
# routines, behind the first refusal), with the
# image's instructions for the pair: `merged' (one reference, or a block
# store, for both), `apart', or `not a pair' (the image's values are not
# a constant or a copy pair: registers, or sources not adjacent, where
# dpc's frame or slots differ).
REFUSAL_SITES = (
    ('acl/convert_image.pas:134', 'const', 0xE44EC6, ('clrb %a2@(40)', 'clrb %a2@(41)'),
     'apart'),
    ('acl/copy.pas:202', 'const', 0xE4941E, ('moveb #15,%fp@(-140)', 'moveb #7,%fp@(-139)'),
     'apart'),
    ('acl/def_acldata.pas:85', 'const', 0xE47916,
     ('moveb #16,%a0@(24)', 'moveb #16,%a0@(25)'), 'apart'),
    ('acl/image_internal.pas:291', 'const', 0xE47DD8, ('clrb %a1@(40)', 'clrb %a1@(41)'),
     'apart'),
    ('acl/prim_create_internal.pas:422', 'const', 0xE4521E, ('clrl %a1@(34)',), 'merged'),
    ('app/std_open.pas:80', 'const', 0xE00BA8, ('movel #77135874,%fp@(-40)',), 'merged'),
    ('area/init.pas:104', 'const', 0xE2F3C0, ('clrw %a0@(1506)', 'clrw %a0@(1504)'),
     'apart'),
    ('audit/log_event_s.pas:253', 'const', 0xE71032, ('clrl %a3@(66)',), 'merged'),
    ('disk/get_mnt_info.pas:116', 'copy', 0xE6BEFE,
     ('movew %a3@(-42),%a1@(10)', 'movew %a3@(-34),%a1@(12)'), 'not a pair'),
    ('disk/pv_mount.pas:55', 'const', 0xE6C9FA, ('movew #1,%fp@(-26)', 'movew #1,%fp@(-24)'),
     'apart'),
    ('ec/ec2_wait.pas:551', 'const', 0xE4281C, ('clrw %a0@(6)', 'clrw %a0@(8)'), 'apart'),
    ('flp/dinit.pas:157', 'const', 0xE3E1FC, ('lea %pc@(0xe3e21e),%a0',), 'merged'),
    ('flp/flp_do_io.pas:163', 'const', 0xE3DE4A,
     ('movew #25,%a5@(306)', 'movew #500,%a5@(304)'), 'apart'),
    ('hint/add_net.pas:53', 'copy', 0xE49CA6, ('movel %a1@(46),%a0@(8)',), 'merged'),
    ('hint/clear_hintfile.pas:130', 'copy', 0xE311D8, ('movel %a2@(46),%a0@(8)',), 'merged'),
    ('kbd/init.pas:80', 'const', 0xE33374, ('clrl %a0@(88)',), 'merged'),
    ('log/add.pas:244', 'const', 0xE176B0, ('movel #65537,%a0@',), 'merged'),
    ('log/init.pas:357', 'const', 0xE3017C, ('moveq #1,%d0', 'movel %d0,%a0@'), 'merged'),
    ('mac_os/init.pas:169', 'const', 0xE2F590, ('movel #65537,%a0@(4)',), 'merged'),
    ('msg/rcv_contig.pas:245', 'const', 0xE59886, ('movel #65535,%a3@(10)',), 'merged'),
    ('msg/rcv_internal.pas:183', 'const', 0xE595EE, ('movel #65535,%a3@(10)',), 'merged'),
    ('net_io/create_port.pas:210', 'copy', 0xE5A574,
     ('movew %d2,%a2@(46)', 'movew %d3,%a2@(48)'), 'not a pair'),
    ('netbuf/add_pages.pas:200', 'const', 0xE0EA1A, ('clrl %a4@(1000)',), 'merged'),
    ('netlog/cntl.pas:166', 'const', 0xE719A4, ('clrw %a5@(112)', 'clrw %a5@(114)'), 'apart'),
    ('netlog/log_it.pas:154', 'copy', 0xE71BF4, ('movew %d5,%a0@(-8)', 'movew %d6,%a0@(-6)'),
     'not a pair'),
    ('netlog/log_it.pas:156', 'copy', 0xE71BFC,
     ('movew %fp@(22),%a0@(-4)', 'movew %fp@(24),%a0@(-2)'), 'apart'),
    ('pmap/init_timers.pas:74', 'const', 0xE2F8BA,
     ('clrw %a0@(52)', 'movel #468750,%a0@(54)'), 'apart'),
    ('pmap/init_ws_scan.pas:80', 'const', 0xE1465E,
     ('clrw %a2@(48)', 'movel #250000,%a2@(50)'), 'apart'),
    ('proc1/bind.pas:126', 'const', 0xE14DA6, ('movel #65552,%a2@(86)',), 'merged'),
    ('proc1/get_info.pas:102', 'const', 0xE15004, ('lea %a2@(2),%a0', 'clrl %a0@+'),
     'merged'),
    ('proc1/init_loadav.pas:68', 'const', 0xE14CBC,
     ('clrw %fp@(-20)', 'movel #1250000,%fp@(-18)'), 'apart'),
    ('ring/svc_read.pas:161', 'const', 0xE775AE, ('movew #1,%fp@(-126)', 'clrw %fp@(-124)'),
     'apart'),
    ('rip/port.pas:160', 'const', 0xE15ABA, ('movel #65538,%fp@(-40)',), 'merged'),
    ('rip/send.pas:304', 'const', 0xE8711C, ('clrb %a0@(4)', 'moveb #1,%a0@(5)'), 'apart'),
    ('rip/table.pas:171', 'copy', 0xE68ECA,
     ('movew %a4@(46),%a2@(18)', 'movew %a4@(48),%a2@(20)'), 'apart'),
    ('rip/table.pas:175', 'const', 0xE68EDA, ('movel #65536,%a2@(18)',), 'merged'),
    ('rip/table.pas:342', 'copy', 0xE6905E, ('movel %a4@(46),%fp@(-14)',), 'merged'),
    ('route/short_port.pas:47', 'copy', 0xE69C16, ('movel %a0@(44),%a1@(4)',), 'merged'),
    ('sock/get.pas:118', 'copy', 0xE16108, ('movel %a1@(1000),%a2@(42)',), 'merged'),
    ('sock/put.pas:271', 'copy', 0xE1628A, ('movel %a2@(42),%a0@(1000)',), 'merged'),
    ('tpad/re_range.pas:111', 'const', 0xE69A68,
     ('movew #256,%a1@(-30)', 'movew #256,%a1@(-28)'), 'apart'),
    ('tpad/set_cursor.pas:122', 'const', 0xE69902, ('clrw %a5@(378)', 'clrw %a5@(376)'),
     'apart'),
    ('tty/i_init.pas:293', 'const', 0xE332CC, ('clrw %a0@(88)', 'clrw %a0@(86)'), 'apart'),
    ('tty/i_set_raw_mode.pas:188', 'const', 0xE1BFE4, ('movel #65537,%a2@(56)',), 'merged'),
    ('tty/k_reset.pas:142', 'const', 0xE67322, ('movel #65537,%a2@(714)',), 'merged'),
    ('vtoc/old_to_new.pas:120', 'copy', 0xE19DCA,
     ('moveb %a0@,%a1@', 'moveb %a0@(1),%a1@(1)'), 'apart'),
    ('win/get_stats.pas:80', 'const', 0xE19D8A, ('clrl %fp@(-24)',), 'merged'),
    ('win/reinit_drive.pas:156', 'const', 0xE1955C,
     ('movew #-1,%a5@(116)', 'movew #-1,%a5@(114)'), 'apart'),
    ('xns/error_send.pas:170', 'const', 0xE178D2, ('movel #196616,%fp@(-40)',), 'merged'),
    ('xns/idp_init.pas:129', 'const', 0xE302A0, ('movew #225,%a1@(214)', 'clrw %a1@(216)'),
     'apart'),
    ('xns/idp_misc.pas:106', 'const', 0xE18F8C, ('movew #225,%a3@(214)', 'clrw %a3@(216)'),
     'apart'),
)


def covers(layout, off, size):
    """[(name, 'whole' | 'part')] of the objects of `layout' ((offset,
    size, name), ...) that the store of `size' bytes at `off' reaches."""
    res = []
    for o, n, name in layout:
        lo, hi = max(o, off), min(o + n, off + size)
        if lo < hi:
            res.append((name, 'whole' if (lo, hi) == (o, o + n) else 'part'))
    return res


def one_object(layout, stores):
    """Whether every store covers exactly one whole object of `layout'
    (None for an empty, unknown, layout)."""
    if not layout:
        return None
    return all(len(c) == 1 and c[0][1] == 'whole'
               for c in (covers(layout, o, n) for o, n in stores))


def texts_match(ins, texts):
    """Whether the instructions `ins' ([(addr, len, text)]) begin with
    `texts'."""
    return len(ins) >= len(texts) and all(t == x[2] for t, x in zip(texts, ins))


def overlapping(sites):
    """{addr: [addr, ...]}: for each merged copy, the other merged copies
    whose source range in the same type overlaps its own without being
    the same (then not both can be one object of that type)."""
    res = {}
    for s in sites:
        if not s.get('src'):
            continue
        t, o, n = s['src']
        for u in sites:
            if u is s or not u.get('src'):
                continue
            t2, o2, n2 = u['src']
            if t == t2 and (o, n) != (o2, n2) and max(o, o2) < min(o + n, o2 + n2):
                res.setdefault(s['addr'], []).append(u['addr'])
    return res


def site_verdict(site, apart_ok, overlaps=()):
    """`two objects' or `one object possible' for a MERGED_SITES entry
    whose `apart' instructions were found in the image (apart_ok) or not.
    A merged copy whose source overlaps another's (`overlapping') is
    `overlaps': not all of them are one object of the source's type."""
    if site['kind'] == 'copy':
        return 'overlaps' if overlaps else 'one object possible'
    if one_object(site['layout'], site['stores']) is False and (
            (site['apart'] and apart_ok) or site['decl'].startswith(APOLLO)):
        return 'two objects'
    return 'one object possible'


def segment_rows(merged, apart, seg_of):
    """{segment: [merged, apart]}: how many of the addresses `merged' and
    `apart' each segment (seg_of(addr)) holds."""
    rows = {}
    for i, addrs in enumerate((merged, apart)):
        for a in addrs:
            rows.setdefault(seg_of(a), [0, 0])[i] += 1
    return rows


def sites_main(argv):
    import sau2map
    verbose = '-v' in argv
    m = sau2map.load()
    _, rs = regalloc.routines()

    def where(a):
        for seg, r0, r1 in rs:
            if r0 <= a < r1:
                names = m.names_at(r0)
                return names[0] if names else 'module-local 0x%X' % r0
        return m.describe(a)

    def at(a, n):
        return loopalign.disasm(a, 0x40)[:n]

    def seg_of(a):
        s = m.segment_containing(a)
        return '%s 0x%X' % (s.name, s.addr) if s else '?'

    bad = 0
    print('merged sites (ref 3.15.3 p. 3-57): the declared objects each store covers')
    verdicts = Counter()
    over = overlapping(MERGED_SITES)
    for s in MERGED_SITES:
        ok = texts_match(at(s['addr'], len(s['texts'])), s['texts'])
        apart_ok = all(texts_match(at(a, 1), (t,)) for a, t in s['apart'])
        bad += (not ok) + (not apart_ok)
        v = site_verdict(s, apart_ok, over.get(s['addr'], ()))
        verdicts[s['kind'], v] += 1
        cov = '; '.join('+'.join('%s%s' % (n, '' if w == 'whole' else '(part)')
                                 for n, w in covers(s['layout'], o, k)) or '%d..%d' % (o, o + k)
                        for o, k in s['stores'])
        print('0x%06X %-5s %-28s mod4=%d %-20s%s' % (
            s['addr'], s['kind'], where(s['addr']), s['stores'][0][0] % 4, v,
            '' if ok and apart_ok else ' [IMAGE DIFFERS]'))
        if verbose:
            print('    %s' % ' | '.join(s['texts']))
            print('    covers: %s' % cov)
            print('    layout: %s' % s['decl'])
            if s['apart']:
                print('    apart:  %s' % ', '.join('0x%06X %s' % x for x in s['apart']))
            if s['src']:
                print('    source: %s 0x%X..0x%X%s' % (
                    s['src'][0], s['src'][1], s['src'][1] + s['src'][2] - 1,
                    '; overlaps ' + ', '.join('0x%06X' % a for a in over[s['addr']])
                    if s['addr'] in over else ''))
            if s['note']:
                print('    note:   %s' % s['note'])
    print('verdicts: %s' % ', '.join('%s %s %d' % (k + (verdicts[k],))
                                       for k in sorted(verdicts)))
    print()
    print('../source statements dpc refused (980aa83a); the image at the pair:')
    rc = Counter()
    for f, kind, a, texts, v in REFUSAL_SITES:
        ok = texts_match(at(a, len(texts)), texts)
        bad += not ok
        rc[kind, v] += 1
        print('  %-34s %-5s 0x%06X %-10s %s%s' % (f, kind, a, v, ' | '.join(texts),
                                                '' if ok else '  [IMAGE DIFFERS]'))
    print('by kind: %s' % ', '.join('%s %s %d' % (k + (rc[k],)) for k in sorted(rc)))
    print()
    pairs = []
    for seg, r0, r1 in rs:
        ins = [regalloc.Insn(a, k, t) for a, k, t in loopalign.disasm(r0, r1 - r0)]
        pairs += adjacent_pairs(ins)
    const = [p for p in pairs if p['mod4'] % 2 == 0
             and all(x in ('zero', 'const') for x in p['values'].split('+'))]
    copies = [p for p in pairs if p['mod4'] % 2 == 0 and p['copy']]
    by = Counter((p['sizes'], p['order']) for p in const)
    print('unmerged constant pairs, every merge condition met: %d (%s)' % (
        len(const), ', '.join('%s %s %d' % (k + (by[k],)) for k in sorted(by))))
    print('unmerged copies of adjacent memory: %d' % len(copies))
    for title, kind, ap in (('constants', 'const', const), ('copies', 'copy', copies)):
        rows = segment_rows([s['addr'] for s in MERGED_SITES if s['kind'] == kind],
                            [p['addr'] for p in ap], seg_of)
        both = sorted(k for k, (x, y) in rows.items() if x and y)
        print('%s: segments with a merge %d, with a pair apart %d, with both %d%s' % (
            title, sum(1 for x, y in rows.values() if x), sum(1 for x, y in rows.values() if y),
            len(both), (': ' + ', '.join('%s (%d/%d)' % ((k,) + tuple(rows[k])) for k in both))
            if both else ''))
        if verbose:
            for k in sorted(rows):
                print('    %-26s merged %2d apart %3d' % ((k,) + tuple(rows[k])))
    if bad:
        print('%d listed instruction(s) differ from the image' % bad)
    return 1 if bad else 0


_AREG_USES = ('movel', 'addl', 'subl', 'cmpl', 'cmpal', 'moveal')
_IMM = re.compile(r'(addl|subl|cmpl|addil|subil|cmpil|cmpal) #(0x[0-9a-f]+|-?\d+),(%[ad]\d)$')


def addr_def_kind(y):
    """The kind of address a defining `lea X,An' / `movea.l #imm,An'
    forms (see the module doc, `addr'), or None for another definition."""
    if y.mn == 'moveal' and y.ops[0].startswith('#'):
        return 'imm'
    if y.mn != 'lea':
        return None
    r = regalloc.is_reg(y.ops[1])
    if r in y.upd:
        return None
    src = y.ops[0]
    if '%a5@' in src:
        return 'a5'
    if '%fp@' in src:
        return 'a6'
    if '%pc@' in src:
        return 'pc'
    if '@' not in src:
        return 'abs'
    return 'idx' if re.search(r',%[ad]\d', src) else 'ptr'


def addr_sites(ins, is_sym=lambda v: False):
    """[dict] of the address-as-value sites of a routine's reachable
    instructions (regalloc.Insn, in address order; see the module doc,
    `addr').  is_sym(v): whether v is a map symbol's own address."""
    res = []
    for i, x in enumerate(ins):
        if len(x.ops) != 2:
            continue
        m = _IMM.match(x.text)
        if m:
            v = int(m.group(2), 0) & 0xffffffff
            if 0xD00000 <= v < 0x1000000:
                op = m.group(1)
                res.append({'kind': 'imm', 'addr': x.addr,
                            'use': 'cmpa' if op == 'cmpal' else op.rstrip('il'),
                            'enc': 'i' if op.endswith('il') and op != 'cmpal' else 'ea',
                            'sym': 'sym' if is_sym(v) else 'num', 'value': v,
                            'text': x.text})
            continue
        r = regalloc.is_reg(x.ops[0])
        if (x.mn not in _AREG_USES or not r or r[0] != 'a' or r in ('a5', 'a6', 'a7')
                or x.ops[0] == x.ops[1]):
            continue
        if x.mn in ('cmpal', 'moveal') and regalloc.is_reg(x.ops[1]) is None:
            continue
        d = None
        for j in range(i - 1, -1, -1):
            y = ins[j]
            if y.kind or y.call or ins[j + 1].addr != y.addr + y.n:
                break
            if r in y.defs:
                d = y
                break
        k = addr_def_kind(d) if d else None
        if k is None:
            continue
        dst = regalloc.is_reg(x.ops[1])
        dk = 'dreg' if dst and dst[0] == 'd' else 'areg' if dst else 'mem'
        res.append({'kind': 'areg', 'addr': x.addr, 'use': x.mn, 'dst': dk, 'def': k,
                    'text': d.text + '; ' + x.text})
    return res


def addr_main(argv):
    import a5survey
    verbose = '-v' in argv
    m, rs = regalloc.routines()
    syms = {s.addr for s in m.symbols}
    sites = []
    for seg, r0, r1 in rs:
        raw = a5survey.reached(r0, r1)
        raw = sorted(raw.values()) if isinstance(raw, dict) else sorted(raw)
        ins = [regalloc.Insn(a, n, t) for a, n, t in raw]
        for s in addr_sites(ins, lambda v: v in syms):
            s['routine'] = r0
            sites.append(s)
    if verbose:
        for s in sites:
            if s['kind'] == 'imm':
                print('imm  %06X (r %06X) %-4s %-2s %-3s %-24s %s' % (
                    s['addr'], s['routine'], s['use'], s['enc'], s['sym'],
                    m.describe(s['value']), s['text']))
            else:
                print('areg %06X (r %06X) %-6s %-4s %-4s %s' % (
                    s['addr'], s['routine'], s['use'], s['dst'], s['def'], s['text']))
        return 0
    print('routines %d' % len(rs))
    print('address registers read as values after `lea\' / `movea.l #\' '
          '(use, destination, definition):')
    c = Counter((s['use'], s['dst'], s['def']) for s in sites if s['kind'] == 'areg')
    for k in sorted(c):
        ex = [s for s in sites if s['kind'] == 'areg' and (s['use'], s['dst'], s['def']) == k]
        print('  %-6s %-4s %-4s %5d   e.g. %s' % (
            k + (c[k], ', '.join('0x%06X' % s['addr'] for s in ex[:4]))))
    print('longword immediates in 0xD00000..0xFFFFFF in add/sub/cmp (op, encoding, '
          'symbol):')
    c = Counter((s['use'], s['enc'], s['sym']) for s in sites if s['kind'] == 'imm')
    for k in sorted(c):
        ex = [s for s in sites if s['kind'] == 'imm' and (s['use'], s['enc'], s['sym']) == k]
        print('  %-4s %-2s %-3s %5d   e.g. %s' % (
            k + (c[k], ', '.join('0x%06X' % s['addr'] for s in ex[:4]))))
    return 0


def main(argv):
    if argv and argv[0] == 'addr':
        return addr_main(argv[1:])
    if argv and argv[0] == 'blocks':
        return blocks_main(argv[1:])
    if argv and argv[0] == 'merge':
        return merge_main(argv[1:])
    if argv and argv[0] == 'sites':
        return sites_main(argv[1:])
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
