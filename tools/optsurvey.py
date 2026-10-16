#!/usr/bin/env python3
"""optsurvey.py - which of ref 6.4.24's `-opt' optimisations each compiled
map segment of the SR10.2 image shows, and whether they cluster by segment
as one `-opt' level per compilation would.

Evidence tool for docs/opt-levels.md (bead domain-pascal-g728).  The
reference (6.4.24, pp. 6-26..6-32) lists, per level (each level includes
the lower ones):

    -dba    expressions rearranged for fewest registers; short branches;
            constant expressions folded; CSE within one statement;
            variables stored to memory after every statement
    -opt 0  values remain in registers across statements; identical
            instruction sequences merged (cross-jumping)
    -opt 1  some global CSE; dead code eliminated; multiplication by a
            constant as shifts and adds; "simple transformations for
            speed"; assignment statements merged
    -opt 2  constants substituted for reaching definitions
    -opt 3  redundant assignments eliminated; global register allocation;
            instruction reordering; loop invariants hoisted; strength
            reduction; live analysis of locals; extensive global CSE;
            inline expansion of %begin_inline routines
    -opt 4  automatic inline expansion

Routines are those of tools/loopalign.py (every compiled map `I' segment,
hand assembly left out); the code features are counted on those that
tools/regalloc.py models (a `link.w A6', no `case' jump table).  Per
routine the tool counts the following *features*; each is a sign that an
optimisation was done (+) or was not done (-) at that site:

  merge+ / apart-      constant stores merged (tools/storesurvey.py
                       MERGED_SITES, kind const) / adjacent constant pairs
                       stored apart with every merge condition met
                       (storesurvey.adjacent_pairs, even low address)
  cmerge+ / capart-    the same for copies of adjacent memory
  align                loop-head alignment class (tools/loopalign.py
                       align_classes: al aligning, na not, ? unknown)
  a5                   A5 letter (tools/a5survey.py letter: S s n i .)
  cse+                 a computed value (extension, shift, product, sum,
                       `lea') read from its register after a store, a join
                       or a call since it was computed, not a stored local
                       (csesurvey `reuse', events other than `sto'; global
                       CSE, opt 1 and 3)
  keep+                a plain memory value read from a register after a
                       store, a join or a call since its load (a value
                       held across statements, opt 0)
  reread-              a plain memory value loaded again while a register
                       still holds it and nothing since may have written
                       it (no store to it, no store through a pointer and
                       no call unless the location is an unescaped frame
                       slot): a value not kept (csesurvey.Sym)
  streload-            within a block, `move Rn,L' then a load of L while
                       Rn still holds it (a -dba signature), L a frame
                       slot
  kreread-             within a block, a constant stored to a frame slot
                       read again from the slot (the constant was not
                       propagated, opt 2)
  shmul+ / mulimm      a product by a constant as shifts and adds /
                       `mulu.w'/`muls.w #k'; both by the popcount of k
  kfold-               an operator applied to two numeric constants of
                       16 bits (a constant expression or a reaching
                       constant not folded; `addi.l #0' and addresses,
                       link-time constants, left out)
  deadbra-             an unreachable `bra' right after an unconditional
                       transfer (dead code not eliminated, opt 1)
  duptail-             two paths into one join whose last instructions
                       before it are identical (not merged, opt 0)
  xjpop+               a `bra' from right after a call to a stack pop at
                       a join (a shared pop: merged tails, opt 0)
  deadst-              a store to an unescaped frame slot that nothing in
                       the routine reads (an assignment not eliminated,
                       opt 3); a frame escapes through `pea'/`lea' of a
                       slot, an indexed frame operand, A6 copied (a static
                       link) or a `bsr' to a routine that reads its
                       caller's frame through (A6) (DYN_LINK)
  overst-              within a block, a store to an unescaped frame slot
                       overwritten before any read of it
  loops                innermost natural loops (an edge to a block that
                       dominates its source; a loop entered by a `bra' to
                       its test has the test as head); outer: the others.
                       The loop features below count innermost loops.
  invld-               loops loading a loop-invariant location into a
                       register inside the loop: a frame slot or an A5
                       offset the loop never writes (invld_nc: in a loop
                       with no call and no store through a pointer)
  invaddr-             loops forming a constant address inside the loop
                       (`lea abs', `movea.l #abs', `move.l #abs,Dn');
                       invaddr_nc: in a loop without a call (the flag
                       `invaddr' uses these).  Mostly the absolute array
                       base idiom `movea.l #abs,An; lea (An,Dn.l)': a
                       register choice on the 68010, not clearly a hoist
                       left undone
  srmiss-              loops that scale a value derived from an induction
                       register (live at the head, written in the loop
                       only by `addq/subq #k' or `dbf') by a shift, an add
                       to itself or a multiply by a constant, and use it
                       as an index or add it to an address register
  srstep+              loops (more than two instructions) with a stepped
                       pointer: an address register, or a pointer in a
                       frame slot, stepped by an element size of 2 or more
                       (`addq.l #k,An', `lea (k,An),An', word/longword
                       `(An)+') and defined in the loop by steps only.
                       Strength reduction or the source's own pointer
                       arithmetic: the image does not tell them apart
  lfrm-                loops that read and write an unescaped frame slot
                       (a local not register-allocated inside a loop)
  inlcand-             a module-local routine with a frame, of 0x30 bytes
                       or less, that one `bsr' site calls and nothing
                       outside its segment names (not inlined: opt < 4 or
                       %begin_noinline)

usage:
    tools/optsurvey.py summary          # totals and per-feature segment
                                        # counts
    tools/optsurvey.py segments [-v]    # one row per segment (written also
                                        # to tmp/optsurvey/segments.tsv)
    tools/optsurvey.py routines [SEG]   # one row per routine (tsv to
                                        # tmp/optsurvey/routines.tsv)
    tools/optsurvey.py cooc [--routine] # 2x2 tables and Fisher p of every
                                        # pair of segment (or routine)
                                        # presence flags
    tools/optsurvey.py conc             # is each flag more uniform within
                                        # segments than chance (mixed
                                        # segments, routines shuffled over
                                        # segments)?
    tools/optsurvey.py pairs            # segment-level association of the
                                        # knobs (merges, cmerges, aligns,
                                        # a5always) with every flag, beyond
                                        # routine-level co-occurrence;
                                        # --neg: the signs of a missing
                                        # optimisation against each other
    tools/optsurvey.py a5table          # segments by constant store merging
                                        # and A5 mode
    tools/optsurvey.py tell [-v]        # segments by the highest level
                                        # their code gives evidence of
    tools/optsurvey.py sites FEATURE    # the sites of one feature

The scan is cached in tmp/optsurvey/cache.pickle (`--refresh' rebuilds
it).  Python 3 standard library + m68k-elf-objdump.  Read-only apart
from tmp/.
"""

import math
import os
import pickle
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import regalloc  # noqa: E402
import csesurvey  # noqa: E402
from csesurvey import unlo, REGS  # noqa: E402

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'tmp', 'optsurvey')
CACHE = os.path.join(OUT, 'cache.pickle')

_FP = re.compile(r'^%fp@\((-?\d+)\)$')
_FPANY = re.compile(r'%fp@')
_A5 = re.compile(r'^%a5@\((-?\d+)\)$|^%a5@$')
_ABS = re.compile(r'^0x([0-9a-f]+)$')
_PTR = re.compile(r'^%a[0-4]@')
_SCC = re.compile(r'^s(t|f|hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le)$')
_IDX = re.compile(r'%(a[0-7]|fp|pc)@\([^,()]*,%(d[0-7]|a[0-7]):([wl])\)')

READ_ONLY = ('cmp', 'cmpa', 'cmpi', 'tst', 'btst', 'chk', 'pea', 'jsr', 'jmp')
RMW = ('add', 'adda', 'addi', 'addq', 'addx', 'sub', 'suba', 'subi', 'subq', 'subx',
       'and', 'andi', 'or', 'ori', 'eor', 'eori', 'neg', 'negx', 'not', 'bset', 'bclr',
       'bchg', 'lsl', 'lsr', 'asl', 'asr', 'rol', 'ror', 'roxl', 'roxr', 'nbcd', 'tas')
WRITE_ONLY = ('move', 'movea', 'clr', 'moveq', 'lea')
SCALE = ('lsl', 'asl')


# -- instruction helpers -------------------------------------------------

def base_of(mn):
    """The mnemonic without its size letter (`movew' -> `move')."""
    if mn in ('spl', 'sl') or _SCC.match(mn):
        return mn
    if mn[-1] in 'bwl' and len(mn) > 2:
        return mn[:-1]
    return mn


def size_of(mn):
    """Operand size in bytes from the mnemonic (2 when unknown)."""
    if mn.startswith(('moveq', 'lea', 'pea')):
        return 4
    if _SCC.match(mn):
        return 1
    if mn[:4] in ('bset', 'bclr', 'bchg', 'btst'):
        return 1
    return {'b': 1, 'w': 2, 'l': 4}.get(mn[-1], 2)


def fixed_loc(op):
    """('fp', off) / ('a5', off) / ('abs', addr) for an operand at a fixed
    address, else None."""
    m = _FP.match(op)
    if m:
        return ('fp', int(m.group(1)))
    m = _A5.match(op)
    if m:
        return ('a5', int(m.group(1) or 0))
    m = _ABS.match(op)
    if m:
        return ('abs', int(m.group(1), 16))
    return None


def access(x):
    """[(operand, 'r' | 'w' | 'rw')] for the memory operands of an
    instruction (pushes and pops left out)."""
    mn, ops = x.mn, x.ops
    b = base_of(mn)
    out = []
    if x.kind is not None or (x.target is not None and not x.call):
        return out
    mem = [(k, o) for k, o in enumerate(ops) if regalloc.is_mem(o)]
    for k, o in mem:
        if o.startswith('%sp@'):
            continue
        last = k == len(ops) - 1
        if b in ('lea', 'pea', 'jsr', 'jmp'):
            out.append((o, 'a'))            # address only
        elif mn.startswith('movem'):
            out.append((o, 'r' if k == 0 else 'w'))
        elif not last or b in READ_ONLY:
            out.append((o, 'r'))
        elif b in WRITE_ONLY or _SCC.match(b):
            out.append((o, 'w'))
        elif b in RMW:
            out.append((o, 'rw'))
        else:
            out.append((o, 'r'))
    return out


def overlaps(a, n, b, m):
    return a < b + m and b < a + n


def movem_count(op):
    """Number of registers in a movem register list (`%d2-%d4/%a2')."""
    n = 0
    for part in op.split('/'):
        m = re.fullmatch(r'%([da])(\d)-%([da])(\d)', part)
        if m:
            n += int(m.group(4)) - int(m.group(2)) + 1 + (8 if m.group(1) != m.group(3) else 0)
        elif part.startswith('%'):
            n += 1
    return n


# -- frame slots -----------------------------------------------------------

def frame_info(rt):
    """(escaped, reads, writes): whether the frame's address escapes (a
    `pea'/`lea' of a slot, A6 copied, an indexed frame operand, a movem
    other than the save/restore); the reads and writes of fixed negative
    frame slots as [(index, off, size)] over the reachable code."""
    escaped = False
    reads, writes = [], []
    for i in rt.reach:
        x = rt.ins[i]
        t = x.text
        if re.search(r'(^| )%fp,', t) and x.mn != 'linkw':
            escaped = True                  # A6 copied (a static link)
        if re.search(r'%fp@\([^)]*,', t):
            escaped = True
        if x.mn in ('lea', 'pea') and _FPANY.search(t):
            escaped = True
        if x.mn.startswith('movem'):
            for k, o in enumerate(x.ops):
                m = _FP.match(o)
                if m:
                    n = movem_count(x.ops[1 - k]) * (4 if x.mn.endswith('l') else 2)
                    (reads if k == 0 else writes).append((i, int(m.group(1)), n))
            continue
        n = size_of(x.mn)
        for o, how in access(x):
            m = _FP.match(o)
            if not m or int(m.group(1)) >= 0:
                continue
            if 'r' in how:
                reads.append((i, int(m.group(1)), n))
            if 'w' in how:
                writes.append((i, int(m.group(1)), n))
    return escaped, reads, writes


# routine starts that read their caller's frame through the dynamic link
# (`movea.l (A6),An': a nested routine that only its parent calls,
# codegen-patterns 6); filled by scan()
DYN_LINK = set()


def nested_parent(rt):
    """Whether the routine's frame may be read by a nested routine: it
    passes A6 (`movea.l A6,A1') or it `bsr's to a routine of DYN_LINK."""
    for i in rt.reach:
        x = rt.ins[i]
        if re.match(r'^moveal %fp,', x.text):
            return True
        if x.call and x.target in DYN_LINK:
            return True
    return False


def frame_escapes(rt):
    """Whether something other than the routine's own fixed-slot
    instructions may read or write its frame (frame_info's escape, or a
    nested routine reaching it)."""
    return frame_info(rt)[0] or nested_parent(rt)


def dead_stores(rt):
    """Sites (addr) of stores to unescaped negative frame slots that no
    instruction of the routine reads."""
    escaped, reads, writes = frame_info(rt)
    if escaped or nested_parent(rt):
        return []
    out = []
    for i, off, n in writes:
        if rt.ins[i].mn.startswith('movem'):
            continue
        if not any(overlaps(off, n, o2, n2) for _, o2, n2 in reads):
            out.append(rt.ins[i].addr)
    return out


def blocks(rt):
    """[[index]] the basic blocks of the reachable code, in order."""
    out, cur, b = [], [], None
    for i in rt.reach:
        if rt.block_of[i] != b:
            if cur:
                out.append(cur)
            cur, b = [], rt.block_of[i]
        cur.append(i)
    if cur:
        out.append(cur)
    return out


def overwritten_stores(rt):
    """Sites of a store to an unescaped frame slot that a later store in
    the same block overwrites (same or covering slot) with no read of
    the slot between (and no call between when the frame escaped)."""
    escaped = frame_escapes(rt)
    if escaped or nested_parent(rt):
        return []
    out = []
    for blk in blocks(rt):
        pending = {}        # (off, n) -> addr
        for i in blk:
            x = rt.ins[i]
            n = size_of(x.mn)
            for o, how in access(x):
                m = _FP.match(o)
                if not m:
                    if how in ('a',):
                        pending.clear()
                    continue
                off = int(m.group(1))
                if 'r' in how or how == 'a':
                    for k in [k for k in pending if overlaps(off, n, *k)]:
                        del pending[k]
            if x.mn.startswith('movem'):
                pending.clear()
                continue
            for o, how in access(x):
                m = _FP.match(o)
                if m and how == 'w' and int(m.group(1)) < 0:
                    off = int(m.group(1))
                    for (o2, n2), a in list(pending.items()):
                        if off <= o2 and o2 + n2 <= off + n:
                            out.append(a)
                            del pending[(o2, n2)]
                    pending[(off, n)] = x.addr
    return out


# -- within-block reloads --------------------------------------------------

def block_reloads(rt):
    """Counter and sites of within-block reloads of fixed locations
    (frame, A5, absolute): `streload_<base>' a load of L while the register
    stored to L still holds it, `ldreload_<base>' a load of L while the
    register last loaded from it still holds it, `kreread_<base>' a read
    of L after a constant was stored to it."""
    escaped = frame_escapes(rt)
    c = Counter()
    sites = defaultdict(list)
    for blk in blocks(rt):
        known = {}          # loc -> (reg or None, size, kind, value)
        for i in blk:
            x = rt.ins[i]
            b = base_of(x.mn)
            n = size_of(x.mn)
            if x.call:
                for loc in list(known):
                    reg = known[loc][0]
                    if loc[0] != 'fp' or escaped or reg in regalloc.SCRATCH:
                        del known[loc]
                continue
            acc = access(x)
            # reads
            for o, how in acc:
                if 'r' not in how:
                    continue
                loc = fixed_loc(o)
                if loc is None or loc not in known:
                    continue
                reg, sz, kind, _ = known[loc]
                if sz != n:
                    continue
                if kind == 'k':
                    c['kreread_' + loc[0]] += 1
                    sites['kreread_' + loc[0]].append(x.addr)
                elif b in ('move', 'movea') and regalloc.is_reg(x.ops[-1]) and o == x.ops[0]:
                    key = ('streload_' if kind == 'st' else 'ldreload_') + loc[0]
                    c[key] += 1
                    sites[key].append(x.addr)
            # registers written
            for loc in [k for k, v in known.items() if v[0] is not None and v[0] in x.defs]:
                del known[loc]
            # memory written
            for o, how in acc:
                if 'w' not in how:
                    continue
                loc = fixed_loc(o)
                if loc is None:
                    if _PTR.match(o):
                        for k in list(known):
                            if k[0] != 'fp' or escaped:
                                del known[k]
                    continue
                for k in list(known):
                    if k[0] == loc[0] and overlaps(k[1], known[k][1], loc[1], n):
                        del known[k]
                if how == 'w':
                    src = x.ops[0] if len(x.ops) == 2 else None
                    r = regalloc.is_reg(src) if src else None
                    if b in ('move', 'movea') and r:
                        known[loc] = (r, n, 'st', None)
                    elif b == 'move' and src and src.startswith('#'):
                        known[loc] = (None, n, 'k', src)
                    elif b == 'clr':
                        known[loc] = (None, n, 'k', '#0')
            if b in ('move', 'movea') and len(x.ops) == 2:
                loc = fixed_loc(x.ops[0])
                r = regalloc.is_reg(x.ops[1])
                if loc and r and loc not in known:
                    known[loc] = (r, n, 'ld', None)
    return c, sites


# -- symbolic: rereads, kept values, products, folding ---------------------

def lin(e):
    """(base, k) when the expression is k times one value built with
    shifts, adds and subtracts, else None."""
    e = unlo(e)
    if not isinstance(e, tuple):
        return None
    if e[0] == 'op' and e[1] == 'shl':
        a, s = lin(e[3]), unlo(e[4])
        if a and isinstance(s, tuple) and s[0] == 'k' and isinstance(s[1], int):
            return (a[0], a[1] << s[1])
        return None
    if e[0] == 'op' and e[1] in ('add', 'sub'):
        a, b = lin(e[3]), lin(e[4])
        if a and b and a[0] == b[0]:
            if e[1] == 'add':
                return (a[0], a[1] + b[1])
            # sub: (dst - src); csesurvey keeps (dst, src) order for sub
            return (a[0], a[1] - b[1])
        return None
    if e[0] == 'k':
        return None
    return (e, 1)


SAFE = {'st', 'join', 'back', 'sto', 'br'}


def is_const(e):
    """A numeric constant expression (not an address)."""
    e = unlo(e)
    return isinstance(e, tuple) and e[0] == 'k' and isinstance(e[1], int)


class OptSym(csesurvey.Sym):
    """csesurvey.Sym recording plain-value rereads and keeps, products by
    shifts and adds, and operators on two constants."""

    def __init__(self, rt, escaped):
        self.escaped = escaped
        self.extra = []      # (kind, insn index, info)
        super().__init__(rt)

    def step(self, i, st, ev):
        x = self.rt.ins[i]
        mn, ops = x.mn, x.ops
        b = base_of(mn)
        if self.record:
            # plain values read from a register after statement events
            for r in sorted(x.uses & set(REGS)):
                e = unlo(st.get(r))
                if isinstance(e, tuple) and e[0] == 'm' and x.kind != 'ret':
                    es = ev.get(r, frozenset()) - {'br'}
                    if es & {'st', 'join', 'call', 'ptr', 'back'} and 'sto' not in es:
                        self.extra.append(('keep', i, ('call' in es)))
            # a load of a plain value some register already holds
            if b in ('move', 'movea') and len(ops) == 2 and regalloc.is_reg(ops[1]) \
                    and regalloc.is_mem(ops[0]) and not ops[0].endswith(('@+', '@-')) \
                    and not ops[0].startswith('%pc@'):
                sz = mn[-1] if mn[-1] in 'bwl' else 'l'
                try:
                    v = self.read(ops[0], sz, st)
                except csesurvey.Unknown:
                    v = None
                if v and v[0] == 'm':
                    held = [q for q in REGS if unlo(st.get(q)) == v and q != regalloc.is_reg(ops[1])
                            or (q == regalloc.is_reg(ops[1]) and st.get(q) == v)]
                    for q in held:
                        es = ev.get(q, frozenset())
                        safe = set(SAFE)
                        if v[2].startswith('FP') and not self.escaped:
                            safe |= {'call', 'ptr'}
                        if es <= safe:
                            self.extra.append(('reread', i, tuple(sorted(es - {'br'}))))
                            break
        super().step(i, st, ev)
        if self.record:
            for r in sorted(x.defs & set(REGS)):
                e = unlo(st.get(r))
                if not isinstance(e, tuple) or e[0] != 'op':
                    continue
                if len(e) == 5 and all(is_const(a) and -0x8000 <= unlo(a)[1] <= 0xFFFF
                                       for a in e[3:]) \
                        and not (re.match(r'^(add|sub)i?[wl] #0,', x.text)):
                    # (absolute addresses and `addi.l #0' are link-time
                    # constants: relocations the compiler cannot fold)
                    self.extra.append(('kfold', i, e[1]))
                if e[1] in ('add', 'sub', 'shl'):
                    li = lin(e)
                    if li and li[1] > 2 and (e[1] != 'shl'):
                        self.extra.append(('shmul', i, li[1]))


def product_sites(sym):
    """The shift-and-add products of `sym' (OptSym): the last step of each
    chain only (the next definition of the register does not extend it)."""
    rt = sym.rt
    steps = [(i, k) for t, i, k in sym.extra if t == 'shmul']
    idx = {i for i, _ in steps}
    out = []
    for i, k in steps:
        r = [q for q in rt.ins[i].defs & set(REGS)]
        nxt = next((j for j in rt.reach if j > i and set(r) & rt.ins[j].defs), None)
        if nxt in idx and set(r) & set(rt.ins[nxt].uses):
            continue
        out.append((rt.ins[i].addr, k))
    return out


# -- control flow: dead code, tails ----------------------------------------

def dead_bras(rt):
    """Addresses of `bra's not reached from the entry that follow an
    unconditional transfer that is reached, and branch into reached code."""
    reach = set(rt.reach)
    out = []
    for i in range(1, len(rt.ins)):
        x, p = rt.ins[i], rt.ins[i - 1]
        if i in reach or (i - 1) not in reach or p.kind not in ('br', 'ret', 'jmp'):
            continue
        if x.kind == 'br' and x.target in rt.at and rt.at[x.target] in reach:
            out.append(x.addr)
    return out


def is_pop(x):
    return bool(re.match(r'^(addqw #\d,%sp|addql #\d,%sp|lea %sp@\(\d+\),%sp|addaw #\d+,%sp)$',
                         x.text))


def join_tails(rt):
    """(duptail, xjpop): joins two of whose paths end in identical
    instructions (one through a `bra'), and `bra's from right after a call
    to a pop at a join."""
    reach = set(rt.reach)
    dup, xj = [], []
    for t in rt.reach:
        preds = rt.pred.get(t, [])
        if len(preds) < 2:
            continue
        tails = []
        for p in preds:
            px = rt.ins[p]
            if px.kind == 'br' and px.target == rt.ins[t].addr:
                q = p - 1
                if q in reach and rt.ins[q].kind is None and p not in rt.leaders:
                    tails.append((rt.ins[q].text, 'bra', rt.ins[q].addr))
                    if rt.ins[q].call and is_pop(rt.ins[t]):
                        xj.append(px.addr)
            elif p == t - 1 and px.kind is None:
                tails.append((px.text, 'fall', px.addr))
        seen = {}
        for text, how, a in tails:
            if text in seen and 'bra' in (how, seen[text][0]):
                dup.append(a)
            seen.setdefault(text, (how, a))
    return dup, xj


# -- loops -----------------------------------------------------------------

def natural_loops(rt):
    """[(head index, back-edge index, body set)] for every back edge: an
    edge into a block that dominates the edge's source (a loop entered by
    a `bra' to its test has the test as its head)."""
    dom = regalloc.dominators(rt)
    out = []
    for i in rt.reach:
        for s in rt.succ[i]:
            if rt.block_of.get(s) == s and s in dom.get(rt.block_of[i], ()):
                body = {s, i}
                work = [i]
                while work:
                    k = work.pop()
                    if k == s:
                        continue
                    for p in rt.pred.get(k, []):
                        if p not in body:
                            body.add(p)
                            work.append(p)
                out.append((s, i, body))
    # one loop per head: the union of its back edges' bodies
    by = {}
    for s, i, body in out:
        if s in by:
            by[s] = (s, max(by[s][1], i), by[s][2] | body)
        else:
            by[s] = (s, i, body)
    return [by[s] for s in sorted(by)]


_STEPA = re.compile(r'^(addql|subql|addqw|subqw) #(\d),%(a[0-4])$'
                    r'|^lea %(a[0-4])@\((-?\d+)\),%(a[0-4])$'
                    r'|^(addaw|addal|subaw|subal) #(\d+),%(a[0-4])$')
_STEPD = re.compile(r'^(addq|subq|addi|subi|add|sub)[wl] #(\d+),%(d[0-7])$')
_STEPF = re.compile(r'^(addql|subql) #[2-8],%fp@\(-\d+\)$')


def induction_regs(rt, h, ins):
    """Data registers that are induction variables of the loop with head
    index h and body instructions ins: live at the head, and written in the
    body only by `addq/subq/addi/subi #k' (or a `dbf')."""
    steps, other = set(), set()
    for x in ins:
        m = _STEPD.match(x.text)
        if m:
            steps.add(m.group(3))
            continue
        if x.kind == 'db':
            steps.add(regalloc.is_reg(x.ops[0]))
            continue
        other |= x.defs
    live = rt.lin.get(h, set()) if hasattr(rt, 'lin') else steps
    return {r for r in steps - other if r in live}


def scaled_index_use(ins, ind):
    """Whether the loop body `ins' (in address order) scales a value
    derived from an induction register of `ind' (a shift by a constant,
    an add to itself, a multiply by a constant) and uses the product as
    an index or adds it to an address register: an element address
    computed from the index in every iteration, not strength-reduced."""
    taint, scaled = set(ind), set()
    for x in ins:
        b = base_of(x.mn)
        for m in _IDX.finditer(x.text):
            if m.group(2) in scaled:
                return True
        if len(x.ops) == 2:
            src, dst = regalloc.is_reg(x.ops[0]), regalloc.is_reg(x.ops[1])
        else:
            src, dst = None, regalloc.is_reg(x.ops[0]) if x.ops else None
        if b in ('adda', 'add', 'suba', 'sub') and src in scaled and (dst or '')[:1] == 'a':
            return True
        keep = set()
        if dst and b in ('move', 'movea') and src in taint:
            taint.add(dst)
            if src in scaled:
                scaled.add(dst)
            else:
                scaled.discard(dst)
            keep.add(dst)
        elif dst in taint and (b in SCALE and x.ops[0].startswith('#')
                               or b == 'add' and src == dst
                               or b in ('mulu', 'muls') and x.ops[0].startswith('#')):
            scaled.add(dst)
            keep.add(dst)
        elif dst in taint and (b in ('ext', 'add', 'sub', 'addq', 'subq', 'addi', 'subi')
                               and src != dst):
            keep.add(dst)                   # still affine in the index
        for r in x.defs - keep:
            taint.discard(r)
            scaled.discard(r)
    return False


def is_step(x):
    """An address register (or a frame-held pointer) stepped by an
    element size of 2 or more: `addq.l #k,An', `lea (k,An),An',
    `adda #k,An', `addq.l #k,(d,A6)', or a word/longword `(An)+'."""
    if x.kind is not None or x.call:
        return False
    m = _STEPA.match(x.text)
    if m:
        if m.group(2) is not None:
            return m.group(2) != '1'
        if m.group(4) is not None:
            return m.group(4) == m.group(6) and int(m.group(5)) not in (0, 1, -1)
        return int(m.group(8)) > 1
    if _STEPF.match(x.text):
        return True
    return size_of(x.mn) > 1 and bool(re.search(r'%a[0-4]@\+', x.text))


def pure_steps(ins):
    """The pointers the loop body `ins' steps (is_step) and defines in no
    other way: address registers whose only definitions in the loop are
    steps, frame slots whose only writes are steps.  Pointers that the
    source also assigns inside the loop (`p := p^.next' next to a step)
    are left out."""
    stepped, other = set(), set()
    fstep, fother = set(), set()
    for x in ins:
        if is_step(x):
            m = _STEPF.match(x.text)
            if m:
                fstep.add(x.ops[-1])
                continue
            regs = {r for r in x.defs if r[0] == 'a' and r not in ('a5', 'a6', 'a7')}
            stepped |= regs
            other |= x.defs - regs
            continue
        other |= x.defs
        for o, how in access(x):
            if 'w' in how and _FP.match(o):
                fother.add(o)
    return (stepped - other) | (fstep - fother)


def is_abs_addr(x):
    """`lea abs,An', or `movea.l #k,An' / `move.l #k,Dn' of a kernel
    address (0xE00000..0xEFFFFF): a constant address formed."""
    m = re.match(r'^(lea (0x[0-9a-f]+),%a[0-4]|moveal #(\d+|0x[0-9a-f]+),%a[0-4]'
                 r'|movel #(\d+|0x[0-9a-f]+),%d[0-7])$', x.text)
    if not m:
        return False
    if m.group(2):
        return True
    v = int(m.group(3) or m.group(4), 0)
    return 0xE00000 <= v < 0xF00000




def loop_features(rt, loops=None):
    """Counter over the routine's innermost loops (no other loop's head
    in the body): loops, invld, invld_nc, invaddr, srmiss, srstep, lfrm,
    and the sites per feature (the loop heads; invld the loads)."""
    escaped = frame_escapes(rt)
    c = Counter()
    sites = defaultdict(list)
    loops = loops if loops is not None else natural_loops(rt)
    heads = {h for h, _, _ in loops}
    for h, e, body in loops:
        if (heads - {h}) & body:
            c['outer'] += 1
            continue
        head = rt.ins[h].addr
        c['loops'] += 1
        bi = sorted(body)
        ins = [rt.ins[i] for i in bi]
        has_call = any(x.call for x in ins)
        ptr_store = any(how != 'r' and how != 'a' and _PTR.match(o)
                        for x in ins for o, how in access(x))
        written = []
        for x in ins:
            n = size_of(x.mn)
            for o, how in access(x):
                if 'w' in how:
                    loc = fixed_loc(o)
                    if loc:
                        written.append((loc, n))
        # invariant loads into a register
        inv = False
        for x in ins:
            if base_of(x.mn) not in ('move', 'movea') or len(x.ops) != 2 \
                    or not regalloc.is_reg(x.ops[1]):
                continue
            loc = fixed_loc(x.ops[0])
            if loc is None or loc[0] == 'abs':
                continue
            n = size_of(x.mn)
            if any(l[0] == loc[0] and overlaps(l[1], m, loc[1], n) for l, m in written):
                continue
            if loc[0] == 'a5' and (has_call or ptr_store):
                continue
            if loc[0] == 'fp' and escaped and (has_call or ptr_store):
                continue
            inv = True
            sites['invld'].append(x.addr)
        if inv:
            c['invld'] += 1
            if not has_call and not ptr_store:
                c['invld_nc'] += 1
        if any(is_abs_addr(x) for x in ins):
            c['invaddr'] += 1
            sites['invaddr'].append(head)
            if not has_call:
                c['invaddr_nc'] += 1
                sites['invaddr_nc'].append(head)
        # induction variables and their scaling
        ind = induction_regs(rt, h, ins)
        miss = scaled_index_use(ins, ind)
        if miss:
            c['srmiss'] += 1
            sites['srmiss'].append(head)
        if len(ins) > 2 and pure_steps(ins):
            c['srstep'] += 1
            sites['srstep'].append(head)
        # a frame local read and written in the loop
        rd, wr = set(), set()
        for x in ins:
            for o, how in access(x):
                m = _FP.match(o)
                if m and int(m.group(1)) < 0:
                    if 'r' in how:
                        rd.add(int(m.group(1)))
                    if 'w' in how:
                        wr.add(int(m.group(1)))
        if not escaped and rd & wr:
            c['lfrm'] += 1
            sites['lfrm'].append(head)
    return c, sites


# -- the survey ------------------------------------------------------------

def routine_features(rt):
    """(Counter, {feature: [addr]}) for one regalloc.Routine (no store
    merging, alignment or A5: those come from the other tools)."""
    c = Counter()
    sites = defaultdict(list)
    escaped = frame_escapes(rt)
    for k, v in (('deadst', dead_stores(rt)), ('overst', overwritten_stores(rt)),
                 ('deadbra', dead_bras(rt))):
        c[k] += len(v)
        sites[k] += v
    dup, xj = join_tails(rt)
    c['duptail'] += len(dup)
    sites['duptail'] += dup
    c['xjpop'] += len(xj)
    sites['xjpop'] += xj
    bc, bs = block_reloads(rt)
    c.update(bc)
    for k, v in bs.items():
        sites[k] += v
    lc, ls = loop_features(rt)
    c.update(lc)
    for k, v in ls.items():
        sites[k] += v
    for x in (rt.ins[i] for i in rt.reach):
        m = re.match(r'^mul([us])w #(-?\d+),', x.text)
        if m:
            k = int(m.group(2)) & 0xFFFF
            key = 'mulimm_p%d' % min(bin(k).count('1'), 3)
            c[key] += 1
            sites[key].append(x.addr)
    try:
        sy = OptSym(rt, escaped)
    except (csesurvey.Unknown, regalloc.Unknown, KeyError, IndexError, ValueError):
        c['sym_skipped'] += 1
        return c, sites
    for t, i, info in sy.extra:
        if t in ('reread', 'kfold'):
            c[t] += 1
            sites[t].append(rt.ins[i].addr)
        elif t == 'keep':
            c['keep'] += 1
            if info:
                c['keep_call'] += 1
    for t, i, e, es, r in sy.final:
        if t == 'reuse' and 'sto' not in es:
            c['cse'] += 1
            sites.setdefault('cse', []).append(rt.ins[i].addr)
    for a, k in product_sites(sy):
        key = 'shmul_p%d' % min(bin(k).count('1'), 3)
        c[key] += 1
        sites[key].append(a)
    return c, sites


def merge_features(m, rs):
    """{routine start: Counter} of merge/apart/cmerge/capart (storesurvey)."""
    import loopalign
    import storesurvey
    out = defaultdict(Counter)
    starts = sorted(r0 for _, r0, _ in rs)
    import bisect

    def owner(a):
        k = bisect.bisect_right(starts, a) - 1
        return starts[k] if k >= 0 else None
    for s in storesurvey.MERGED_SITES:
        out[owner(s['addr'])]['merge' if s['kind'] == 'const' else 'cmerge'] += 1
    for seg, r0, r1 in rs:
        ins = [regalloc.Insn(a, k, t) for a, k, t in loopalign.disasm(r0, r1 - r0)]
        for p in storesurvey.adjacent_pairs(ins):
            if p['mod4'] % 2:
                continue
            if all(v in ('zero', 'const') for v in p['values'].split('+')):
                out[r0]['apart'] += 1
            elif p['copy']:
                out[r0]['capart'] += 1
    return out


class Row:
    """One routine of the survey."""

    def __init__(self, seg, segaddr, r0, r1, name):
        self.seg, self.segaddr, self.r0, self.r1, self.name = seg, segaddr, r0, r1, name
        self.c = Counter()
        self.sites = {}
        self.align = '?'
        self.a5 = '?'
        self.modelled = False


# the cache pickles Rows as optsurvey.Row whether this file runs as a script
# or is imported
Row.__module__ = 'optsurvey'
sys.modules.setdefault('optsurvey', sys.modules[__name__])


def scan():
    import loopalign
    import a5survey
    # every routine (for merging, alignment, A5); align_classes compares
    # segments by identity, so loopalign's own map is used throughout
    m, sites, fillers, lroutines = loopalign.scan()
    al = loopalign.align_classes(m, loopalign.per_routine(sites, lroutines))
    for seg, r0, r1 in lroutines:
        if any(re.match(r'^moveal %fp@,%a[0-4]$', t) for _, _, t in loopalign.INSNS[r0]):
            DYN_LINK.add(r0)
    rs5, asm_hexes = a5survey.load()
    by5 = a5survey.analyse(rs5, asm_hexes, m)
    rows = {}
    for seg, r0, r1 in lroutines:
        names = m.names_at(r0)
        rows[r0] = Row(seg.name, seg.addr, r0, r1, names[0] if names else None)
        rows[r0].align = al.get(r0, '?')
        if r0 in by5:
            rows[r0].a5 = a5survey.letter(by5[r0])
    # inline candidates: a small module-local routine with a frame that
    # one `bsr' site calls and nothing names from outside (ref 6.4.24
    # p. 6-32: at -opt 4 the compiler selects routines to expand inline)
    sites_to = Counter(t for r in rs5 for t in r.bsrs)
    for r in rs5:
        if (not r.name and r.has_link and r.r1 - r.r0 <= 0x30 and not r.ext_refs
                and sites_to[r.r0] == 1 and r.r0 in rows):
            rows[r.r0].c['inlcand'] += 1
            rows[r.r0].sites['inlcand'] = [r.r0]
    mf = merge_features(m, lroutines)
    for r0, cc in mf.items():
        if r0 in rows:
            rows[r0].c.update(cc)
    _, done, _ = regalloc.load_all(0, 0x10000)
    for seg, rt in done:
        row = rows.get(rt.r0)
        if row is None:
            continue
        c, s = routine_features(rt)
        row.c.update(c)
        row.sites.update({k: v for k, v in s.items() if v})
        row.modelled = True
    return sorted(rows.values(), key=lambda r: r.r0)


def load(refresh=False):
    if not refresh and os.path.exists(CACHE):
        with open(CACHE, 'rb') as f:
            return pickle.load(f)
    rows = scan()
    os.makedirs(OUT, exist_ok=True)
    with open(CACHE, 'wb') as f:
        pickle.dump(rows, f)
    return rows


# -- aggregation and statistics --------------------------------------------

FEATURES = ('merge', 'apart', 'cmerge', 'capart', 'keep', 'keep_call', 'cse', 'reread',
            'streload_fp', 'ldreload_fp', 'ldreload_a5', 'ldreload_abs', 'kreread_fp',
            'kreread_a5', 'kreread_abs', 'shmul_p1', 'shmul_p2', 'shmul_p3', 'mulimm_p1',
            'mulimm_p2', 'mulimm_p3', 'kfold', 'deadbra', 'duptail', 'xjpop', 'deadst',
            'overst', 'loops', 'outer', 'invld', 'invld_nc', 'invaddr', 'invaddr_nc', 'srmiss',
            'srstep',
            'lfrm', 'inlcand')


def segments(rows):
    """{(segaddr, name): {'c': Counter, 'n': routines, 'mod': modelled,
    'align': Counter, 'a5': Counter, 'rows': [Row]}}."""
    out = {}
    for r in rows:
        s = out.setdefault((r.segaddr, r.seg), {'c': Counter(), 'n': 0, 'mod': 0,
                                                'align': Counter(), 'a5': Counter(),
                                                'a5named': Counter(), 'rows': []})
        s['c'].update(r.c)
        s['n'] += 1
        s['mod'] += r.modelled
        s['align'][r.align] += 1
        s['a5'][r.a5] += 1
        if r.name:
            s['a5named'][r.a5] += 1
        s['rows'].append(r)
    for s in out.values():
        s['a5seg'] = bool(s['a5']['S'] or s['a5']['s'])
    return out


def flags(s):
    """Presence flags of one segment (or of one routine's counters when
    given a dict with the same keys): name -> True/False/None (None: the
    segment has nothing to tell)."""
    c = s['c']
    al, a5 = s['align'], s['a5']
    f = {}
    f['merges'] = True if c['merge'] else (False if c['apart'] else None)
    f['cmerges'] = True if c['cmerge'] else (False if c['capart'] else None)
    f['aligns'] = True if al['al'] else (False if al['na'] else None)
    a5n = s.get('a5named', a5)
    f['a5always'] = True if a5n['s'] else (False if a5n['n'] and s.get('a5seg') else None)
    f['rereads'] = bool(c['reread']) if c['keep'] or c['reread'] else None
    f['cse'] = bool(c['cse']) if s['mod'] else None
    f['streload'] = bool(c['streload_fp']) if s['mod'] else None
    f['kreread'] = bool(c['kreread_fp']) if s['mod'] else None
    f['kfold'] = bool(c['kfold']) if s['mod'] else None
    f['deadbra'] = bool(c['deadbra']) if s['mod'] else None
    f['duptail'] = bool(c['duptail']) if s['mod'] else None
    f['xjpop'] = bool(c['xjpop']) if s['mod'] else None
    f['deadst'] = bool(c['deadst']) if s['mod'] else None
    f['overst'] = bool(c['overst']) if s['mod'] else None
    lp = c['loops']
    f['invld'] = bool(c['invld']) if lp else None
    f['invld_nc'] = bool(c['invld_nc']) if lp else None
    f['invaddr'] = bool(c['invaddr_nc']) if lp else None
    f['srmiss'] = bool(c['srmiss']) if lp else None
    f['srstep'] = bool(c['srstep']) if lp else None
    f['lfrm'] = bool(c['lfrm']) if lp else None
    f['inlcand'] = bool(c['inlcand'])
    return f


def routine_unit(r, a5seg=False):
    """A segment-like dict for one routine (for flags()); a5seg: the
    routine's segment loads A5 somewhere."""
    return {'c': r.c, 'n': 1, 'mod': int(r.modelled), 'align': Counter([r.align]),
            'a5': Counter([r.a5]), 'a5named': Counter([r.a5] if r.name else []),
            'a5seg': a5seg}


def fisher(a, b, c, d):
    """Two-sided Fisher exact p for the 2x2 table [[a, b], [c, d]]."""
    n = a + b + c + d
    r1, c1 = a + b, a + c

    def lp(x):
        return (math.lgamma(r1 + 1) + math.lgamma(n - r1 + 1) + math.lgamma(c1 + 1)
                + math.lgamma(n - c1 + 1) - math.lgamma(n + 1) - math.lgamma(x + 1)
                - math.lgamma(r1 - x + 1) - math.lgamma(c1 - x + 1)
                - math.lgamma(n - r1 - c1 + x + 1))
    lo, hi = max(0, r1 + c1 - n), min(r1, c1)
    p0 = lp(a)
    p = sum(math.exp(lp(x)) for x in range(lo, hi + 1) if lp(x) <= p0 + 1e-9)
    return min(1.0, p)


def table2(units, f, g):
    """(a, b, c, d) of flag f against flag g over units (flags dicts),
    units where either is None left out: a = both, b = f only, c = g only,
    d = neither."""
    a = b = c = d = 0
    for u in units:
        x, y = u.get(f), u.get(g)
        if x is None or y is None:
            continue
        if x and y:
            a += 1
        elif x:
            b += 1
        elif y:
            c += 1
        else:
            d += 1
    return a, b, c, d


FLAGS = ('merges', 'cmerges', 'aligns', 'a5always', 'cse', 'rereads', 'streload',
         'kreread', 'kfold', 'deadbra', 'duptail', 'xjpop', 'deadst', 'overst',
         'invld', 'invld_nc', 'invaddr', 'srmiss', 'srstep', 'lfrm', 'inlcand')


def fmt_flag(v):
    return '.' if v is None else ('+' if v else '-')


# -- commands --------------------------------------------------------------

def cmd_summary(rows):
    segs = segments(rows)
    tot = Counter()
    for r in rows:
        tot.update(r.c)
    print('routines %d (modelled %d) in %d compiled segments' % (
        len(rows), sum(r.modelled for r in rows), len(segs)))
    print('totals: ' + ', '.join('%s %d' % (k, tot[k]) for k in FEATURES))
    fl = [flags(s) for s in segs.values()]
    print('segments by flag (+ present, - absent, . nothing to tell):')
    for f in FLAGS:
        c = Counter(fmt_flag(u[f]) for u in fl)
        print('  %-10s +%-4d -%-4d .%d' % (f, c['+'], c['-'], c['.']))


def cmd_segments(rows, verbose=False):
    segs = segments(rows)
    os.makedirs(OUT, exist_ok=True)
    cols = ['addr', 'segment', 'routines', 'modelled', 'align(al/na/?)', 'a5(S/s/n)'] + \
        list(FEATURES) + ['flags:' + ''.join(f[0] for f in FLAGS)]
    lines = ['\t'.join(cols)]
    print('%-6s %-18s %4s  %-8s %-8s %s' % ('addr', 'segment', 'n', 'al/na/?', 'S/s/n',
                                             ' '.join(f[:6] for f in FLAGS)))
    for (sa, name), s in sorted(segs.items()):
        f = flags(s)
        al, a5 = s['align'], s['a5']
        als = '%d/%d/%d' % (al['al'], al['na'], al['?'])
        a5s = '%d/%d/%d' % (a5['S'], a5['s'], a5['n'])
        print('%06X %-18s %4d  %-8s %-8s %s' % (sa, name[:18], s['n'], als, a5s,
                                                 ' '.join('%-6s' % fmt_flag(f[k]) for k in FLAGS)))
        if verbose:
            print('       ' + ', '.join('%s %d' % (k, s['c'][k]) for k in FEATURES if s['c'][k]))
        lines.append('\t'.join(['%06X' % sa, name, str(s['n']), str(s['mod']), als, a5s]
                               + [str(s['c'][k]) for k in FEATURES]
                               + [''.join(fmt_flag(f[k]) for k in FLAGS)]))
    with open(os.path.join(OUT, 'segments.tsv'), 'w') as fh:
        fh.write('\n'.join(lines) + '\n')
    print('flags: ' + ' '.join(FLAGS))
    print('written %s' % os.path.relpath(os.path.join(OUT, 'segments.tsv')))


def cmd_routines(rows, seg=None):
    os.makedirs(OUT, exist_ok=True)
    lines = ['\t'.join(['addr', 'segment', 'name', 'align', 'a5', 'modelled'] + list(FEATURES))]
    for r in rows:
        if seg and r.seg != seg:
            continue
        lines.append('\t'.join(['%06X' % r.r0, r.seg, r.name or '-', r.align, r.a5,
                                str(int(r.modelled))] + [str(r.c[k]) for k in FEATURES]))
        if seg:
            print('%06X %-28s %-2s %s  %s' % (r.r0, (r.name or 'module-local')[:28], r.align,
                                               r.a5, ', '.join('%s %d' % (k, r.c[k])
                                                               for k in FEATURES if r.c[k])))
    with open(os.path.join(OUT, 'routines.tsv'), 'w') as fh:
        fh.write('\n'.join(lines) + '\n')
    print('written %s' % os.path.relpath(os.path.join(OUT, 'routines.tsv')))


def cmd_cooc(rows, per_routine=False, pmax=0.05):
    if per_routine:
        units = [flags(routine_unit(r, s['a5seg'])) for s in segments(rows).values()
                 for r in s['rows']]
    else:
        units = [flags(s) for s in segments(rows).values()]
    print('pairs of %s flags (a both, b first only, c second only, d neither); '
          'two-sided Fisher p' % ('routine' if per_routine else 'segment'))
    res = []
    for i, f in enumerate(FLAGS):
        for g in FLAGS[i + 1:]:
            if f in ('invld', 'invld_nc') and g in ('invld', 'invld_nc'):
                continue
            a, b, c, d = table2(units, f, g)
            if a + b + c + d < 6:
                continue
            p = fisher(a, b, c, d)
            res.append((p, f, g, a, b, c, d))
    for p, f, g, a, b, c, d in sorted(res):
        mark = '*' if p < pmax else ' '
        print('%s %-10s %-10s a=%-3d b=%-3d c=%-3d d=%-3d p=%.3g %s' % (
            mark, f, g, a, b, c, d, p, 'pos' if a * d > b * c else 'neg'))


def concentration(groups, rng, n=2000):
    """(observed, null mean, p) for presence values grouped by segment
    (`groups': [[bool]] of the eligible routines of each segment): the
    number of *mixed* segments (some routines with the feature, some
    without), against the same number after shuffling the values over all
    eligible routines (segment sizes kept); p = P(null <= observed), small
    when segments are more uniform than chance gives (one setting per
    segment would give no mixed segment at all)."""
    flat = [v for g in groups for v in g]
    sizes = [len(g) for g in groups]

    def count(vals):
        k, out = 0, 0
        for z in sizes:
            part = vals[k:k + z]
            out += any(part) and not all(part)
            k += z
        return out
    obs = count(flat)
    tot, le = 0, 0
    vals = list(flat)
    for _ in range(n):
        rng.shuffle(vals)
        c = count(vals)
        tot += c
        le += c <= obs
    return obs, tot / n, le / n


def pearson(xs, ys, ws=None):
    ws = ws or [1] * len(xs)
    sw = sum(ws)
    if sw == 0:
        return 0.0
    mx = sum(w * x for w, x in zip(ws, xs)) / sw
    my = sum(w * y for w, y in zip(ws, ys)) / sw
    vx = sum(w * (x - mx) ** 2 for w, x in zip(ws, xs))
    vy = sum(w * (y - my) ** 2 for w, y in zip(ws, ys))
    if vx == 0 or vy == 0:
        return 0.0
    return sum(w * (x - mx) * (y - my) for w, x, y in zip(ws, xs, ys)) / math.sqrt(vx * vy)


def seg_corr(units, slots, f, g):
    """Correlation over segments of the rates of flags f and g among each
    segment's routines where the flag tells (units: routine flag dicts
    in slot order; slots: [(start, end)] per segment), weighted by the
    smaller of the two eligible counts; segments where either has no
    eligible routine are left out."""
    xs, ys, ws = [], [], []
    for a, b in slots:
        fx = [u[f] for u in units[a:b] if u[f] is not None]
        gy = [u[g] for u in units[a:b] if u[g] is not None]
        if not fx or not gy:
            continue
        xs.append(sum(fx) / len(fx))
        ys.append(sum(gy) / len(gy))
        ws.append(min(len(fx), len(gy)))
    return pearson(xs, ys, ws), len(xs)


def cmd_conc(rows, n=2000):
    import random
    rng = random.Random(728)
    segs = segments(rows)
    print('uniformity by segment: mixed segments (routines with and without the '
          'flag), observed vs. routines shuffled over segments (%d runs)' % n)
    for f in FLAGS:
        groups = []
        for s in segs.values():
            g = [x for x in (flags(routine_unit(r, s['a5seg']))[f] for r in s['rows'])
                 if x is not None]
            if g:
                groups.append(g)
        npres = sum(sum(g) for g in groups)
        if not npres:
            print('  %-10s no routine has it (%d routines can tell)' % (
                f, sum(len(g) for g in groups)))
            continue
        obs, mean, p = concentration(groups, rng, n)
        withf = sum(1 for g in groups if any(g))
        print('  %-10s routines %4d/%-4d segments with %3d of %-3d mixed %3d null %6.1f  '
              'p(<=)=%.4f' % (f, npres, sum(len(g) for g in groups), withf, len(groups), obs,
                              mean, p))


KNOBS = ('merges', 'cmerges', 'aligns', 'a5always')


NEGATIVE = ('rereads', 'streload', 'kreread', 'deadst', 'invld', 'invld_nc', 'invaddr',
            'lfrm', 'duptail', 'inlcand')


def cmd_pairs(rows, n=1000, first=KNOBS, second=FLAGS, min_routines=5):
    """Segment-level association of each knob with every other flag,
    beyond what routine-level co-occurrence gives: the correlation of the
    segments' rates, against whole routines shuffled over segment slots.
    p = P(|null - mean| >= |r - mean|), centred on the null mean.  Flags
    present in fewer than `min_routines' routines are skipped (one or two
    routines give spurious p)."""
    import random
    rng = random.Random(728)
    segs = sorted(segments(rows).items())
    units, slots = [], []
    for _, s in segs:
        a = len(units)
        units += [flags(routine_unit(r, s['a5seg'])) for r in s['rows']]
        slots.append((a, len(units)))
    held = Counter(f for u in units for f, v in u.items() if v)
    rare = sorted(f for f in set(first) | set(second) if held[f] < min_routines)
    print('segment rate correlation r (weighted), null mean and p (centred on the null '
          'mean); routines shuffled whole (%d runs)' % n)
    if rare:
        print('skipped, in fewer than %d routines: %s' % (
            min_routines, ', '.join('%s %d' % (f, held[f]) for f in rare)))
    done = set()
    tests = sig = 0
    for f in first:
        for g in second:
            if g == f or (g, f) in done or {f, g} <= {'invld', 'invld_nc'} \
                    or f in rare or g in rare:
                continue
            done.add((f, g))
            r, k = seg_corr(units, slots, f, g)
            if k < 6:
                continue
            perm = list(units)
            null = []
            for _ in range(n):
                rng.shuffle(perm)
                null.append(seg_corr(perm, slots, f, g)[0])
            mean = sum(null) / n
            p = sum(abs(q - mean) >= abs(r - mean) - 1e-12 for q in null) / n
            tests += 1
            sig += p < 0.05
            print('  %s %-9s %-10s r=%+.2f over %3d segments  null %+.2f  p=%.3f' % (
                '*' if p < 0.05 else ' ', f, g, r, k, mean, p))
    print('%d tests, %d with p < 0.05 (%.1f expected by chance)' % (tests, sig, tests * 0.05))


def a5_table(rows):
    """{(merge class, A5 class): segments}: merge class `merge' (merged
    constant stores only), `apart', `both'; A5 class over the named
    routines of segments that load A5: `used' (n, no s), `always' (s, no
    n), `mixed', `-' (neither n nor s, or no A5 load)."""
    out = Counter()
    for _, s in segments(rows).items():
        c = s['c']
        if not (c['merge'] or c['apart']):
            continue
        mc = 'both' if c['merge'] and c['apart'] else 'merge' if c['merge'] else 'apart'
        a5 = s['a5named']
        if not s['a5seg'] or not (a5['n'] or a5['s']):
            ac = '-'
        else:
            ac = 'mixed' if a5['n'] and a5['s'] else 'always' if a5['s'] else 'used'
        out[(mc, ac)] += 1
    return out


def cmd_a5table(rows):
    t = a5_table(rows)
    print('segments by constant store merging and A5 mode (named routines; `-\': '
          'no n or s routine, or no A5 load)')
    print('%-6s %6s %6s %6s %6s' % ('', 'used', 'always', 'mixed', '-'))
    for mc in ('merge', 'apart', 'both'):
        print('%-6s %6d %6d %6d %6d' % ((mc,) + tuple(t[(mc, a)] for a in
                                                       ('used', 'always', 'mixed', '-'))))
    a, b = t[('merge', 'used')], t[('merge', 'always')] + t[('merge', 'mixed')]
    c, d = t[('apart', 'used')], t[('apart', 'always')] + t[('apart', 'mixed')]
    print('purely used: merge %d of %d, apart %d of %d; Fisher p %.3g' % (
        a, a + b, c, c + d, fisher(a, b, c, d)))


ABOVE0 = ('cse', 'shmul', 'srstep')


def tell_class(c):
    """What a routine's or segment's counters show of the levels: `opt1+'
    (a global CSE, a shift-and-add product or a stepped pointer), `opt0'
    (only values kept across statements or a shared pop), `none'."""
    if c['cse'] or c['shmul_p1'] or c['shmul_p2'] or c['shmul_p3'] or c['srstep']:
        return 'opt1+'
    if c['keep'] or c['xjpop']:
        return 'opt0'
    return 'none'


def cmd_tell(rows, verbose=False):
    segs = segments(rows)
    k = Counter()
    lines = []
    for (sa, name), s in sorted(segs.items()):
        if not s['mod']:
            cl = 'unmodelled'
        else:
            cl = tell_class(s['c'])
        k[cl] += 1
        k[cl + ' routines'] += s['mod']
        if cl != 'opt1+':
            lines.append('  %06X %-18s %-10s %d routines, %d modelled' % (sa, name, cl, s['n'],
                                                                        s['mod']))
    print('segments by the highest level their code gives evidence of '
          '(opt1+: cse, shift-and-add product or stepped pointer; opt0: values kept '
          'across statements or a shared pop)')
    for cl in ('opt1+', 'opt0', 'none', 'unmodelled'):
        print('  %-10s %3d segments, %4d modelled routines' % (cl, k[cl], k[cl + ' routines']))
    if verbose:
        print('\n'.join(lines))


def cmd_sites(rows, feature):
    for r in rows:
        v = r.sites.get(feature)
        if v:
            print('%06X %-10s %-28s %s' % (r.r0, r.seg[:10], (r.name or 'module-local')[:28],
                                           ' '.join('%06X' % a for a in v)))


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    refresh = '--refresh' in argv
    argv = [a for a in argv if a != '--refresh']
    rows = load(refresh)
    cmd = argv[0]
    if cmd == 'summary':
        cmd_summary(rows)
    elif cmd == 'segments':
        cmd_segments(rows, '-v' in argv)
    elif cmd == 'routines':
        cmd_routines(rows, argv[1] if len(argv) > 1 else None)
    elif cmd == 'cooc':
        cmd_cooc(rows, '--routine' in argv)
    elif cmd == 'conc':
        cmd_conc(rows)
    elif cmd == 'pairs':
        if '--neg' in argv:
            cmd_pairs(rows, first=NEGATIVE, second=NEGATIVE)
        else:
            cmd_pairs(rows)
    elif cmd == 'a5table':
        cmd_a5table(rows)
    elif cmd == 'tell':
        cmd_tell(rows, '-v' in argv)
    elif cmd == 'sites' and len(argv) > 1:
        cmd_sites(rows, argv[1])
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
