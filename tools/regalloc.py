#!/usr/bin/env python3
"""regalloc.py - survey register choice and instruction order in the compiled
code of the SR10.2 image.

Evidence tool for docs/codegen-patterns.md section 12.4 (beads
domain-pascal-hy0, -6tn, -5cx): which register did Apollo's compiler give a
value, which registers were free there, and how are independent
instructions ordered inside a block?

Routines are found as in tools/loopalign.py (map symbols and `link.w A6'
words inside the map's `I' segments; hand assembly is skipped, section 0).
Only routines that start with `link.w A6' are analysed.  Each one is
disassembled with m68k-elf-objdump (68010), split into basic blocks
(instructions reachable from the entry only, so data after an `rts' is
ignored), and a register def/use model of each instruction is built
(D0-D7 and A0-A4; A5 is the module data base, A6 the frame, A7 the stack,
and they are left out).  Routines with an instruction the model does not
know, or with a computed `jmp' (a `case' table), are skipped and counted.

A *value* (a "web") is a maximal set of definitions of one register that
reach common uses; a two-address update (`add.l D1,D2', `lea (d,A1),A1',
`dbf') or an auto-increment extends the web of the value it updates.  A
load through a register into that register (`movea.l (4,A0),A0',
`move.w (2,A0,D1.w),D1') starts a new value: the compiler reused the
dying base register, which is what rule U (below) says it does not do.  For every value
the tool tests whether an allocation rule picks the register the compiler
chose, given the registers of all the other values.  A rule mispredicts
when it prefers another register or when, under the rule, the chosen
register itself would conflict.  Rules (`summary' prints all):

    lo     lowest register of the class not live and not written at the
           value's first definition (a linear scan in address order)
    lifo   of those, the one freed most recently in address order
    col    lowest register that no other value interferes with (Chaitin:
           one is defined where the other is live); a call writes D0/D1/
           A0/A1, M$ARITH calls only D0/A0
    colU   col, and a value never takes a register read by an instruction
           that defines it while the other value dies there (no
           "coalescing"; `move.l D1,D3; add.l x,D3' at 0xE177D0)
    colUS  colU, a register freed only after the statement holding the
           value's last use (proxy: the next store/call in the block)
    colUC  colU, a value last used in a block ending in a conditional
           branch holds its register up to the block's immediate
           post-dominator (the end of the `if')
    colEU  colU, every parameter load's value live from the entry
    colLU  colU, every parameter load's value live from the start of its
           block (or the last call before it in the block)
    lifoU  colU's candidates, most recently freed first
    colQU  colU, counting only values whose first definition comes earlier
           in address order (a greedy colouring in address order; col and
           colU accept *any* greedy order, so they are an upper bound for
           an allocator that colours values one by one)

Model limits: a byte/word write to a data register is a full definition,
except after `clr.l'/`moveq'/a byte or word write into the same register
in the same block (the zero-extension idiom `clr.l D1; move.w D0,D1').
D0/A0 written and never read before `rts' is a function result and is
read by every `rts' (a dead `(A0)+' or `dbf' write is not a result).  Single-register restores before `unlk' and the
0x2048 filler are not values.  All loads of one parameter slot (d,A6) of
one register class are one variable (they never conflict with each other).

usage:
    tools/regalloc.py summary [--min N] [--max N]  # hit rates of the rules
    tools/regalloc.py misses RULE [--min N] [--max N] [--class d|a]
                                         # values where RULE mispredicts
    tools/regalloc.py routine ADDR       # one routine: values, choices,
                                         # free sets (hex address)
    tools/regalloc.py corpus             # the section 12.4 corpus table
    tools/regalloc.py sched [N] [--strict] [--min N] [--max N]
                                         # adjacent true dependences that
                                         # an independent instruction could
                                         # have separated (N examples)
    tools/regalloc.py csaved [--min N] [--max N]
                                         # routines that use callee-saved
                                         # registers, with and without calls

--min/--max bound the routine size in bytes (default 0 and 0x10000).

Python 3 standard library + m68k-elf-objdump.  Read-only; writes only a
temporary file under tmp/ (through loopalign.disasm).
"""

import os
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import loopalign  # noqa: E402
import sau2map  # noqa: E402

SOURCE = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                      '..', '..', 'source', 'domain_os')

DREGS = ['d%d' % i for i in range(8)]
AREGS = ['a%d' % i for i in range(5)]
ALLOC = set(DREGS + AREGS)
SCRATCH = {'d0', 'd1', 'a0', 'a1'}
MARITH = (0xE0ABD4, 0xE0ADB0)
CC = r'(?:hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le|hs|lo)'
_BR = re.compile(r'^b(ra|sr|' + CC[3:-1] + r')[sbwl]?$')
_DB = re.compile(r'^db(?:f|t|ra|' + CC[3:-1] + r')$')
_SCC = re.compile(r'^s(?:t|f|' + CC[3:-1] + r')$')
_REG = re.compile(r'%(d[0-7]|a[0-7]|fp|sp)')


class Unknown(Exception):
    pass


def canon(r):
    return {'fp': 'a6', 'sp': 'a7'}.get(r, r)


def split_ops(s):
    ops, depth, cur = [], 0, ''
    for ch in s:
        if ch == '(':
            depth += 1
        elif ch == ')':
            depth -= 1
        if ch == ',' and depth == 0:
            ops.append(cur.strip())
            cur = ''
        else:
            cur += ch
    if cur.strip():
        ops.append(cur.strip())
    return ops


def ea_regs(op):
    """Registers read to form an operand's address (not its value)."""
    return {canon(r) for r in _REG.findall(op)}


def is_reg(op):
    m = re.fullmatch(r'%(d[0-7]|a[0-7]|fp|sp)', op)
    return canon(m.group(1)) if m else None


class Insn:
    def __init__(self, addr, n, text):
        self.addr, self.n, self.text = addr, n, text
        parts = text.split(None, 1)
        self.mn = parts[0]
        self.ops = split_ops(parts[1]) if len(parts) > 1 else []
        self.uses, self.defs = set(), set()
        self.partial = set()     # defs that are byte/word writes to Dn
        self.upd = set()         # registers updated in place (two-address
                                 # arithmetic, (An)+/-(An), dbf, lea (d,An),An):
                                 # the new contents continue the old value
        self.incdec = set()      # registers written only by (An)+ / -(An)
        self.call = False
        self.kind = None         # 'br', 'bcc', 'ret', 'jmp', 'db'
        self.target = None
        self.bad = None          # set when the model does not know it
        try:
            self.model()
        except (Unknown, ValueError, IndexError) as e:
            self.bad = str(e) or self.text
            self.kind = 'ret'

    def src(self, op):
        r = is_reg(op)
        if r:
            self.uses.add(r)
        else:
            self.uses |= ea_regs(op)
            self.autoinc(op)

    def autoinc(self, op):
        m = re.fullmatch(r'%(a[0-7]|sp|fp)@[-+]', op)
        if m:
            r = canon(m.group(1))
            self.defs.add(r)
            self.upd.add(r)
            self.incdec.add(r)

    def dst(self, op, rmw=False, size='l'):
        r = is_reg(op)
        if r:
            self.defs.add(r)
            self.incdec.discard(r)
            if rmw:
                self.uses.add(r)
                self.upd.add(r)
            elif size in 'bw' and r[0] == 'd':
                self.partial.add(r)
        else:
            self.uses |= ea_regs(op)
            self.autoinc(op)

    def model(self):
        mn, ops = self.mn, self.ops
        size = mn[-1] if mn[-1] in 'bwl' else 'l'
        m = _BR.match(mn)
        if m:
            t = int(ops[0], 16)
            if m.group(1) == 'sr':
                self.call = True
                self.target = t
                return
            self.kind = 'br' if m.group(1) == 'ra' else 'bcc'
            self.target = t
            return
        if _DB.match(mn):
            self.kind = 'db'
            self.dst(ops[0], rmw=True)
            self.target = int(ops[1], 16)
            return
        if mn in ('rts', 'rte', 'rtd'):
            self.kind = 'ret'
            return
        if mn in ('linkw', 'unlk', 'nop', 'moveml', 'movemw', 'trap', 'illegal'):
            if mn == 'moveml' and not any('%fp@' in o or '%sp@' in o for o in ops):
                raise Unknown(self.text)
            return
        if mn == 'jsr':
            self.call = True
            self.uses |= ea_regs(ops[0])
            return
        if mn == 'jmp':
            if '%pc@(' in ops[0] and ',' in ops[0]:
                raise Unknown('computed jmp')
            self.kind = 'jmp'
            self.uses |= ea_regs(ops[0])
            return
        if _SCC.match(mn):
            self.dst(ops[0], size='b')
            return
        base = mn[:-1] if mn[-1] in 'bwl' else mn
        if base in ('move', 'movea', 'movel', 'movew', 'moveq', 'moveal', 'moveaw'):
            if ops[0] in ('%sr', '%ccr', '%usp'):
                self.dst(ops[1], size=size)
                return
            if ops[1] in ('%sr', '%ccr', '%usp'):
                self.src(ops[0])
                return
            self.src(ops[0])
            self.dst(ops[1], size='l' if base in ('moveq', 'movea') else size)
            return
        if mn in ('moveq', 'moveql'):
            self.dst(ops[1])
            return
        if base == 'lea':
            self.uses |= ea_regs(ops[0])
            self.dst(ops[1])
            r = is_reg(ops[1])
            if r and re.match(r'%' + r + r'@', ops[0]):
                self.upd.add(r)                 # `lea (d,A1),A1' = adda #d,A1
            return
        if base == 'pea':
            self.uses |= ea_regs(ops[0])
            return
        if base == 'clr':
            self.dst(ops[0], size=size)
            return
        if base in ('add', 'adda', 'addi', 'addq', 'addx', 'sub', 'suba', 'subi', 'subq',
                    'subx', 'and', 'andi', 'or', 'ori', 'eor', 'eori', 'mulu', 'muls',
                    'divu', 'divs', 'bset', 'bclr', 'bchg', 'abcd', 'sbcd'):
            if ops[-1] in ('%sr', '%ccr'):
                return
            if base in ('sub', 'suba', 'eor') and ops[0] == ops[1] and is_reg(ops[0]):
                self.dst(ops[1])                # `suba.l A0,A0': a clear
                return
            self.src(ops[0])
            self.dst(ops[1], rmw=True)
            return
        if base in ('cmp', 'cmpa', 'cmpi', 'cmpm', 'btst', 'chk'):
            self.src(ops[0])
            self.src(ops[1])
            return
        if base in ('tst', 'tas'):
            self.src(ops[0])
            return
        if base in ('lsl', 'lsr', 'asl', 'asr', 'rol', 'ror', 'roxl', 'roxr'):
            if len(ops) == 1:
                self.uses |= ea_regs(ops[0])
                return
            self.src(ops[0])
            self.dst(ops[1], rmw=True)
            return
        if base in ('neg', 'negx', 'not', 'ext', 'swap', 'nbcd'):
            self.dst(ops[0], rmw=True)
            return
        if base == 'exg':
            for o in ops:
                self.dst(o, rmw=True)
            return
        raise Unknown(self.text)


class Routine:
    """One compiled routine: blocks, liveness, values (webs)."""

    def __init__(self, r0, r1):
        self.r0, self.r1 = r0, r1
        raw = loopalign.disasm(r0, r1 - r0)
        self.ins = []
        for a, n, t in raw:
            self.ins.append(Insn(a, n, t))
        self.at = {x.addr: i for i, x in enumerate(self.ins)}
        self.tails = {}
        self.cfg()
        self.epilogue()
        self.zext()
        self.liveness()
        self.result()
        self.webs()

    # -- control flow ------------------------------------------------------
    def succ_of(self, i):
        x = self.ins[i]
        out = []
        if x.kind in ('ret', 'jmp'):
            return out
        if x.target is not None and not x.call:
            if x.target not in self.at:
                if x.kind == 'br':
                    return out        # tail branch out of the routine
                raise Unknown('branch out of routine at 0x%X' % x.addr)
            out.append(self.at[x.target])
        if x.kind != 'br' and i + 1 < len(self.ins):
            out.append(i + 1)
        return out

    def cfg(self):
        seen, work = set(), [0]
        while work:
            i = work.pop()
            if i in seen:
                continue
            seen.add(i)
            work.extend(self.succ_of(i))
        self.reach = sorted(seen)
        for i in self.reach:
            if self.ins[i].bad:
                raise Unknown(self.ins[i].bad)
        self.succ = {i: self.succ_of(i) for i in self.reach}
        self.pred = defaultdict(list)
        for i, ss in self.succ.items():
            for s in ss:
                self.pred[s].append(i)
        leaders = {0}
        for i in self.reach:
            x = self.ins[i]
            if x.target is not None and not x.call and x.target in self.at:
                leaders.add(self.at[x.target])
            if x.kind and i + 1 < len(self.ins):
                leaders.add(i + 1)
        self.leaders = leaders
        self.block_of = {}
        b = -1
        for i in self.reach:
            if i in leaders or b < 0 or self.reach[self.reach.index(i) - 1] != i - 1:
                b = i
            self.block_of[i] = b
        self.calls = [i for i in self.reach if self.ins[i].call]
        for i in self.calls:
            x = self.ins[i]
            t = x.target if x.target is not None else (
                int(x.ops[0], 16) if x.ops and re.fullmatch(r'0x[0-9a-f]+', x.ops[0]) else None)
            # M$ARITH (map `I E0ABD4 M$ARITH size = 1DC') is treated by the
            # compiler as clobbering D0 and A0 only (A1 is read before and
            # after `jsr M$OIU$WLW' at 0xE3CB8E/0xE3CB9E; every entry of
            # ../source/domain_os/math/sau2/arith.s writes only D0/A0)
            x.defs |= {'d0', 'a0'} if t is not None and MARITH[0] <= t < MARITH[1] else SCRATCH
            # a nested routine gets its static link in A1 (section 6)
            j = i - 1
            if j >= 0 and self.ins[j].text.startswith('moveal %fp,%a1'):
                self.ins[i].uses.add('a1')

    def epilogue(self):
        """A single-register restore before `unlk' (`move.l (-n,A6),D2',
        `movea.l (-n,A6),A5') is not a value."""
        for i in self.reach:
            x = self.ins[i]
            if (x.mn in ('movel', 'moveal') and re.fullmatch(r'%fp@\(-\d+\)', x.ops[0])
                    and i + 1 < len(self.ins) and self.ins[i + 1].mn == 'unlk'):
                x.defs, x.uses = set(), set()
            if loopalign.word(x.addr) == 0x2048:      # `movea.l A0,A0' filler
                x.defs, x.uses = set(), set()

    def result(self):
        """A function result: D0 or A0 written and never read before an
        `rts' (a dead definition, calls excepted) is taken to be read by
        every `rts' of the routine."""
        res = set()
        for i in self.reach:
            x = self.ins[i]
            if x.call or x.kind == 'db':
                continue
            for r in x.defs & {'d0', 'a0'}:
                # a dead `(A0)+' (the last step of a block copy) is no result
                if r not in self.lout[i] and r not in x.incdec:
                    res.add(r)
        if res:
            self.result_regs = res
            for i in self.reach:
                if self.ins[i].kind == 'ret':
                    self.ins[i].uses |= res
            self.liveness()
        else:
            self.result_regs = set()

    def zext(self):
        """`clr.l Dn; ...; move.w x,Dn' in one block: the word write merges."""
        for i in self.reach:
            x = self.ins[i]
            for r in list(x.partial):
                j = i - 1
                while j >= 0 and j in self.block_of and self.block_of[j] == self.block_of[i]:
                    y = self.ins[j]
                    if r in y.defs:
                        if y.mn in ('clrl', 'moveq', 'moveql') or (r in y.partial):
                            x.uses.add(r)
                            x.upd.add(r)
                        break
                    if r in y.uses:
                        break
                    j -= 1

    def liveness(self):
        self.lin = {i: set() for i in self.reach}
        self.lout = {i: set() for i in self.reach}
        changed = True
        while changed:
            changed = False
            for i in reversed(self.reach):
                x = self.ins[i]
                out = set()
                for s in self.succ[i]:
                    out |= self.lin[s]
                inn = (out - x.defs) | x.uses
                if out != self.lout[i] or inn != self.lin[i]:
                    self.lout[i], self.lin[i] = out, inn
                    changed = True

    def webs(self):
        # reaching definitions per register
        rd_in = {i: defaultdict(frozenset) for i in self.reach}
        rd_out = {i: defaultdict(frozenset) for i in self.reach}
        entry = defaultdict(frozenset, {r: frozenset([('entry', r)]) for r in ALLOC})
        changed = True
        while changed:
            changed = False
            for i in self.reach:
                inn = defaultdict(frozenset)
                ps = self.pred.get(i, [])
                srcs = [rd_out[p] for p in ps] + ([entry] if i == 0 else [])
                for s in srcs:
                    for r, ds in s.items():
                        inn[r] = inn[r] | ds
                out = defaultdict(frozenset, inn)
                for r in self.ins[i].defs:
                    out[r] = frozenset([(i, r)])
                if inn != rd_in[i] or out != rd_out[i]:
                    rd_in[i], rd_out[i] = inn, out
                    changed = True
        self.rd_in = rd_in
        parent = {}

        def find(a):
            parent.setdefault(a, a)
            while parent[a] != a:
                parent[a] = parent[parent[a]]
                a = parent[a]
            return a

        def union(a, b):
            parent[find(a)] = find(b)
        for i in self.reach:
            x = self.ins[i]
            for r in x.defs & ALLOC:
                find((i, r))
            for r in x.uses & ALLOC:
                ds = list(rd_in[i][r])
                if x.kind == 'ret':
                    continue                    # results: one value per def
                for d in ds[1:]:
                    union(ds[0], d)
                if r in x.upd and ds:           # two-address update
                    union((i, r), ds[0])
        groups = defaultdict(list)
        for k in list(parent):
            groups[find(k)].append(k)
        self.values = []
        self.livein = []
        for ks in groups.values():
            reg = ks[0][1]
            entry = any(k[0] == 'entry' for k in ks)
            defs = sorted(k[0] for k in ks if k[0] != 'entry')
            if not defs:
                continue                      # live-in only (static link)
            ks_set = set(ks)
            pts = set(defs)
            for i in self.reach:
                if reg in self.lout[i] and rd_out_has(rd_in, i, self.ins[i], reg, ks_set):
                    pts.add(i)
                if reg in self.lin[i] and rd_in[i][reg] & ks_set:
                    pts.add(i)
            if all(self.ins[d].call for d in defs) and not any(
                    reg in self.ins[i].uses for i in pts):
                continue                      # call clobber only
            v = Value(self, reg, defs, pts, ks_set)
            if all(self.ins[d].call for d in defs):
                # a call's result (D0/A0) read later: an occupant only
                self.livein.append(v)
                continue
            if entry:
                # also reaches a use undefined on some path (e.g. a result
                # register left unset on an error path): an occupant for
                # interference, not an allocation decision
                v.partial_entry = True
                self.livein.append(v)
            else:
                self.values.append(v)
        self.values.sort(key=lambda v: v.first)

def rd_out_has(rd_in, i, x, reg, ks):
    if reg in x.defs:
        return (i, reg) in ks
    return bool(rd_in[i][reg] & ks)


class Value:
    def __init__(self, rt, reg, defs, pts, keys):
        self.rt, self.reg, self.defs, self.pts, self.keys = rt, reg, defs, pts, keys
        self.first = min(defs)
        self.last = max(pts)
        self.cls = DREGS if reg[0] == 'd' else AREGS
        x = rt.ins[self.first]
        self.text = x.text
        pm = (x.mn.startswith('move') and x.ops and
              re.fullmatch(r'%fp@\((\d+)\)', x.ops[0]))
        self.param = bool(pm)
        # all loads of one parameter slot are one variable
        self.var = ('param', pm.group(1), reg[0]) if pm else ('value', id(self))
        self.crosses_call = any(rt.ins[p].call and reg in rt.lout[p] and p not in defs
                                for p in pts)
        self.blocks = {rt.block_of[p] for p in pts}


RULES = ('lo', 'lifo', 'col', 'colU', 'colUS', 'colUC', 'colEU', 'colLU', 'lifoU', 'colQU')


def free_at(rt, v):
    """Registers of v's class that no other value occupies at v's first
    definition (not live in or out there, not defined there)."""
    i = v.first
    return [r for r in v.cls if r == v.reg or not (
        r in rt.lout[i] or r in rt.lin[i] or r in rt.ins[i].defs)]


def rule_spec(name):
    """col[E|L][U][S|K|P|C] / lifo...: E = parameter loads live from the
    entry, L = from the start of their block (or the last call in it),
    U = no register read by the defining instruction, S/K/P = a register
    is free only after the statement (next store/call), the block, or the
    immediate post-dominator of the block holding the value's last use;
    Q = only values defined earlier (by first definition address) count,
    i.e. a greedy colouring in address order."""
    base = 'lifo' if name.startswith('lifo') else 'col'
    f = name[len(base):]
    tail = {'S': 'stmt', 'K': 'block', 'P': 'pdom', 'C': 'cond'}.get(f[-1:]) if f else None
    ent = 'blk' if 'L' in f else ('E' in f)
    return base, ent, 'U' in f, 'B' in f, tail, 'Q' in f


def predictions(rt, v, last_free):
    """{rule: predicted register} for value v (rules: RULES, rule_spec)."""
    free = free_at(rt, v)
    lo = next((r for r in free), None)
    p = {'lo': lo, 'lifo': lifo_pick(v, free, last_free)}
    cache = {}
    for name in RULES:
        if name in p:
            continue
        base, ent, use, blk, tail, seq = rule_spec(name)
        key = (ent, use, blk, tail, seq)
        if key not in cache:
            cache[key] = [c for c in v.cls
                          if not interferes_any(rt, v, c, blk, ent, use, tail or False, seq)]
        cand = cache[key]
        p[name] = (cand[0] if cand else None) if base == 'col' else lifo_pick(v, cand, last_free)
    return p, free


def lifo_pick(v, cand, last_free):
    """The candidate freed most recently (ties: the one whose value was
    allocated last); registers never used yet: lowest first."""
    used = [r for r in cand if r in last_free]
    if used:
        return max(used, key=lambda r: last_free[r])
    return cand[0] if cand else None


def pts_of(rt, v, ent, stmt=False):
    pts = v.pts
    if ent == 'blk' and v.param:
        # from the start of the block (or the last call before it in the
        # block) that holds the load
        b = rt.block_of[v.first]
        back = set()
        for i in reversed([j for j in rt.reach if rt.block_of[j] == b and j < v.first]):
            if rt.ins[i].call:
                break
            back.add(i)
        pts = pts | back
    elif ent and v.param:
        pts = pts | {i for i in rt.reach if i < v.first}
    if stmt:
        key = (id(v), stmt)
        if key not in rt.tails:
            rt.tails[key] = stmt_tail(rt, max(v.pts), stmt)
        pts = pts | rt.tails[key]
    return pts


def ipdom_region(rt, i):
    """The instructions after i up to the immediate post-dominator of i's
    block: for a last use in an `if' condition, the whole `if' statement
    (both arms) up to the join."""
    if not hasattr(rt, 'pdom'):
        bl = blocks(rt)
        first = {b[0]: b for b in bl}
        bsucc = {}
        for b in bl:
            bsucc[b[0]] = sorted({rt.block_of[s] for s in rt.succ[b[-1]]})
        ids = list(first)
        exit_ = 'X'
        pd = {b: set(ids) | {exit_} for b in ids}
        changed = True
        while changed:
            changed = False
            for b in reversed(ids):
                ss = bsucc[b] or [exit_]
                new = set.intersection(*[(pd[t] if t != exit_ else {exit_}) for t in ss]) | {b}
                if new != pd[b]:
                    pd[b] = new
                    changed = True
        ipd = {}
        for b in ids:
            cands = pd[b] - {b}
            # the immediate one: the candidate post-dominated by all others
            ipd[b] = next((c for c in cands if all(c == d or (d in pd.get(c, {exit_}) if c != exit_ else False) or d == exit_ for d in cands) and c != exit_), None)
        rt.pdom, rt.bsucc, rt.bfirst, rt.ipd = pd, bsucc, first, ipd
    b = rt.block_of[i]
    stop = rt.ipd.get(b)
    out = {j for j in rt.bfirst[b] if j > i}
    seen, work = {b}, list(rt.bsucc[b])
    while work:
        c = work.pop()
        if c in seen or c == stop:
            continue
        seen.add(c)
        out |= set(rt.bfirst[c])
        work.extend(rt.bsucc[c])
    return out


def stmt_tail(rt, i, tail='stmt'):
    if tail == 'pdom':
        return ipdom_region(rt, i)
    if tail == 'cond':
        b = [j for j in rt.reach if rt.block_of[j] == rt.block_of[i]]
        if rt.ins[b[-1]].kind == 'bcc':
            return ipdom_region(rt, i)
        return set()
    """Instructions after i up to the end of its "statement", taken as the
    next store to memory (not a push), call, or the end of the block."""
    out = set()
    b = rt.block_of[i]
    k = rt.reach.index(i)
    for j in rt.reach[k + 1:]:
        if rt.block_of[j] != b:
            break
        out.add(j)
        x = rt.ins[j]
        if tail == 'block':
            continue
        if x.call or (x.ops and is_mem(x.ops[-1]) and '%sp@-' not in x.ops[-1]
                      and not x.mn.startswith(('cmp', 'tst', 'btst', 'pea', 'jmp'))):
            break
    return out


def interferes_any(rt, v, c, blk=False, ent=False, use=False, stmt=False, seq=False):
    """Would value v conflict with another value on register c under the
    rule's ranges?  For c == v.reg this checks the original's own choice
    against the rule (a rule that makes it conflict mispredicts too)."""
    if use and c != v.reg and any(c in rt.ins[d].uses for d in v.defs):
        return True
    vp = pts_of(rt, v, ent, stmt)
    vb = {rt.block_of[p] for p in vp}
    glob = len(vb) > 1
    for w in rt.values + rt.livein:
        if w.reg != c or w is v or w.var == v.var:
            continue
        if seq and w in rt.values and w.first > v.first:
            continue                    # colQ: not allocated yet
        if use and any(v.reg in rt.ins[d].uses for d in w.defs if d in v.pts):
            return True
        wp = pts_of(rt, w, ent, stmt)
        if blk and glob:
            wb = {rt.block_of[p] for p in wp}
            if len(wb) > 1 and wb & vb:
                return True
            continue
        if overlap(rt, v, w, vp, wp):
            return True
    # clobbers by calls inside v's live range
    for p in vp:
        x = rt.ins[p]
        if x.call and c in x.defs and (v.reg in rt.lout[p] or p not in v.pts):
            return True
    return False


def overlap(rt, v, w, vp, wp):
    """Chaitin: one is defined at a point where the other is live out.
    A value extended to the entry (colE) is live at every extended point."""
    for a, ap, b, bp in ((v, vp, w, wp), (w, wp, v, vp)):
        for d in a.defs:
            if d in bp and (d not in b.pts or (b.reg in rt.lout[d] and live_val_out(rt, b, d))):
                return True
    return False


def live_val_out(rt, b, p):
    x = rt.ins[p]
    if b.reg in x.defs:
        return p in b.defs
    return bool(rt.rd_in[p][b.reg] & b.keys)


def analyse(rt):
    """[(value, {rule: prediction}, free)] in address order of first
    definition."""
    out = []
    ends = sorted(((v.last, v.first, v.reg) for v in rt.values))
    for v in sorted(rt.values, key=lambda v: v.first):
        lf = {r: (t, f) for t, f, r in ends if t < v.first}
        p, free = predictions(rt, v, lf)
        out.append((v, p, free))
    return out


def routines(lo=0, hi=0x10000):
    m = sau2map.load()
    res = []
    for seg in m.segments:
        if seg.kind != 'I' or not (loopalign.LO <= seg.addr < loopalign.HI) or seg.size == 0:
            continue
        if loopalign.is_asm(seg):
            continue
        st = loopalign.routine_starts(m, seg)
        for r0, r1 in zip(st, st[1:] + [min(seg.end, loopalign.HI)]):
            if loopalign.word(r0) != 0x4E56:
                continue
            if lo <= r1 - r0 <= hi:
                res.append((seg, r0, r1))
    return m, res


_cfiles = None


def cfile(addr):
    """The ../source C file whose header says `Original address: 0x<addr>'."""
    global _cfiles
    if _cfiles is None:
        _cfiles = {}
        pat = re.compile(r'Original address:?\s*0x([0-9a-fA-F]+)')
        for d, _, fs in os.walk(SOURCE):
            for f in fs:
                if not f.endswith('.c'):
                    continue
                p = os.path.join(d, f)
                try:
                    with open(p, errors='replace') as fh:
                        for a in pat.findall(fh.read()):
                            _cfiles.setdefault(int(a, 16), os.path.relpath(p, SOURCE))
                except OSError:
                    pass
    return _cfiles.get(addr, CFILE_EXTRA.get(addr))


# C files whose header does not carry `Original address'
CFILE_EXTRA = {0xE17800: 'log/read.c', 0xE146B4: 'pmap/purge_ws.c',
               0xE0F27C: 'network/network.h (declaration only)'}


def is_mem(op):
    """An operand that addresses memory (not a register or an immediate)."""
    return not (op.startswith('#') or is_reg(op) or op in ('%sr', '%ccr', '%usp'))


NOCC = ('movea', 'lea', 'pea', 'adda', 'suba', 'jsr', 'bsr', 'link', 'unlk', 'movem',
        'exg', 'swap')


def sets_cc(x):
    base = x.mn[:-1] if x.mn[-1] in 'bwl' else x.mn
    if x.kind or x.call or base in NOCC or x.mn in ('moveml', 'linkw'):
        return False
    if base in ('addq', 'subq') and x.ops and is_reg(x.ops[-1]) and is_reg(x.ops[-1])[0] == 'a':
        return False
    return True


STRICT = False


def deps(rt, blk):
    """Dependence predecessors within one block (list of insn indices):
    register RAW/WAR/WAW (A7 included), memory (stores ordered against
    every memory access, loads against stores), calls as barriers, and the
    condition codes read by a final branch or an Scc."""
    pred = {i: set() for i in blk}
    for k, i in enumerate(blk):
        x = rt.ins[i]
        xr = x.uses | ea_regs(' '.join(x.ops))
        xw = set(x.defs)
        xm = [o for o in x.ops if is_mem(o)] if not x.kind else []
        xld = bool(xm) and not (x.mn.startswith('move') and len(x.ops) == 2 and not is_mem(x.ops[0]))
        xst = bool(xm) and not (x.mn.split()[0][:3] in ('cmp', 'tst', 'bts') and True) and (
            is_mem(x.ops[-1]) if x.ops else False)
        if x.mn.startswith(('cmp', 'tst', 'btst')):
            xst = False
        if x.mn == 'pea' or (x.ops and '%sp@-' in x.ops[-1]):
            xst = True
        for j in blk[:k]:
            y = rt.ins[j]
            yr = y.uses | ea_regs(' '.join(y.ops))
            yw = set(y.defs)
            ym = [o for o in y.ops if is_mem(o)] if not y.kind else []
            yst = (y.mn == 'pea' or (y.ops and '%sp@-' in y.ops[-1]) or
                   (bool(ym) and not y.mn.startswith(('cmp', 'tst', 'btst'))
                    and y.ops and is_mem(y.ops[-1])))
            yld = bool(ym)
            if (yw & xr) or (yr & xw) or (yw & xw):
                pred[i].add(j)
            elif STRICT and (('a7' in xw and (yld or yst)) or ('a7' in yw and (xld or xst or xm))):
                pred[i].add(j)
            elif y.mn in ('unlk', 'moveml') or x.mn in ('unlk', 'moveml'):
                pred[i].add(j)
            elif x.call or y.call:
                pred[i].add(j)
            elif (xst and (yld or yst)) or (yst and (xld or xm)):
                pred[i].add(j)
        if x.kind == 'bcc' or re.match(r's(?:t|f|hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le)$', x.mn):
            for j in reversed(blk[:k]):
                if sets_cc(rt.ins[j]):
                    pred[i].add(j)
                    for j2 in blk[:blk.index(j)]:
                        if sets_cc(rt.ins[j2]):
                            pred[j].add(j2)
                    break
        if x.kind:
            pred[i] |= set(blk[:k])
    return pred


def blocks(rt):
    out = defaultdict(list)
    for i in rt.reach:
        out[rt.block_of[i]].append(i)
    return [out[b] for b in sorted(out)]


def sched_survey(done, m, show=0):
    """Adjacent true dependences (x then y, y reads a register x wrote)
    where some later instruction of the block was already free to go
    between them, vs. where none was."""
    c = Counter()
    shown = 0
    for seg, rt in done:
        for blk in blocks(rt):
            if len(blk) < 3:
                continue
            pred = deps(rt, blk)
            for k in range(len(blk) - 1):
                i, j = blk[k], blk[k + 1]
                x, y = rt.ins[i], rt.ins[j]
                if y.kind or not (x.defs & y.uses & ALLOC) or y.call:
                    continue
                done_set = set(blk[:k + 1])
                alt = [l for l in blk[k + 2:] if not rt.ins[l].kind and pred[l] <= done_set]
                kind = 'load' if any(is_mem(o) for o in x.ops[:1]) and x.mn.startswith('move') else 'other'
                c[kind, 'avoidable' if alt else 'forced'] += 1
                if alt and show and shown < show:
                    shown += 1
                    print('%06X %-40s -> %-30s could insert %06X %s' % (
                        x.addr, x.text, y.text, rt.ins[alt[0]].addr, rt.ins[alt[0]].text))
    return c


# The corpus of codegen-patterns section 12.4: routine starts, studied by
# hand against their C reconstructions in ../source/domain_os.
CORPUS = [
    0xE17778, 0xE17800, 0xE0DCC8, 0xE07050, 0xE00F16, 0xE0A96C, 0xE0A9C2,
    0xE0AA24, 0xE071EA, 0xE0B1BC, 0xE0B202, 0xE0CA3A, 0xE0CA7A, 0xE01BEE,
    0xE0283C, 0xE07C22, 0xE41468, 0xE15830, 0xE14876, 0xE127E6, 0xE0F27C,
    0xE16C5E, 0xE148CA, 0xE148F8, 0xE07E80, 0xE146B4, 0xE0B174, 0xE70ABA,
    0xE1359C, 0xE6B0BC, 0xE7630C, 0xE1C9FC, 0xE54E10, 0xE488B6, 0xE59FB2,
    0xE3EF38, 0xE190BC, 0xE751F0, 0xE65904, 0xE49138, 0xE0C6E2, 0xE18FD6,
    0xE152E4, 0xE0D8F0,
]


def one(m, r0):
    seg = m.segment_containing(r0)
    st = loopalign.routine_starts(m, seg)
    r1 = next((x for x in st if x > r0), min(seg.end, loopalign.HI))
    return Routine(r0, r1)


def load_all(lo, hi):
    m, rs = routines(lo, hi)
    done, skipped = [], Counter()
    for seg, r0, r1 in rs:
        try:
            done.append((seg, Routine(r0, r1)))
        except Unknown as e:
            skipped[str(e).split()[0]] += 1
        except (ValueError, IndexError, KeyError):
            skipped['parse'] += 1
    return m, done, skipped


def opts(argv):
    lo, hi, cls = 0, 0x10000, None
    rest = []
    it = iter(argv)
    for a in it:
        if a == '--min':
            lo = int(next(it), 0)
        elif a == '--max':
            hi = int(next(it), 0)
        elif a == '--class':
            cls = next(it).lower()
        else:
            rest.append(a)
    return lo, hi, cls, rest


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    lo, hi, cls, argv = opts(argv)
    cmd = argv[0]
    if cmd == 'corpus':
        m = sau2map.load()
        tot = Counter()
        print('%-6s %-4s %-26s %-40s %4s %5s %5s %s' % (
            'start', 'size', 'map name', 'C file', 'vals', 'calls', 'miss', 'colU misses'))
        for r0 in CORPUS:
            rt = one(m, r0)
            res = analyse(rt)
            miss = [(v, p) for v, p, f in res if p['colU'] != v.reg]
            tot['vals'] += len(res)
            tot['miss'] += len(miss)
            print('%06X %4X %-26s %-40s %4d %5d %5d %s' % (
                r0, rt.r1 - r0, (m.names_at(r0) or ['module-local'])[0][:26],
                (cfile(r0) or '-')[:40], len(res), len(rt.calls), len(miss),
                ' '.join('%06X:%s(%s)' % (rt.ins[v.first].addr, v.reg, p['colU'])
                         for v, p in miss)))
        print('total values %d, colU misses %d' % (tot['vals'], tot['miss']))
        return 0
    if cmd == 'routine':
        r0 = int(argv[1], 16)
        m = sau2map.load()
        rt = one(m, r0)
        print('%06X..%06X %s  C: %s' % (r0, rt.r1, m.describe(r0), cfile(r0)))
        print('  rules: ' + ' '.join(RULES) + '  (. hit, ! miss)')
        for v, p, free in analyse(rt):
            flag = ''.join(('.' if p[k] == v.reg else '!') for k in RULES)
            print('  %06X %-3s %s %s free=%-20s blk=%d call=%d  %s' % (
                rt.ins[v.first].addr, v.reg, flag,
                ' '.join('%s=%s' % (k, p[k]) for k in RULES if p[k] != v.reg),
                ','.join(free), len(v.blocks), v.crosses_call, v.text))
        return 0
    m, done, skipped = load_all(lo, hi)
    if cmd == 'summary':
        print('routines analysed %d, skipped %s' % (len(done), dict(skipped)))
        tot = Counter()
        for seg, rt in done:
            for v, p, free in analyse(rt):
                for k in (v.reg[0], v.reg[0] + ('-glob' if len(v.blocks) > 1 else '-loc'),
                          v.reg[0] + ('-param' if v.param else '-other')):
                    tot[k, 'n'] += 1
                    for name in RULES:
                        tot[k, name] += p[name] == v.reg
        print('%-8s %6s ' % ('class', 'values') + ' '.join('%12s' % r for r in RULES))
        for k in ('d', 'd-loc', 'd-glob', 'd-param', 'd-other',
                  'a', 'a-loc', 'a-glob', 'a-param', 'a-other'):
            n = tot[k, 'n']
            print('%-8s %6d ' % (k, n) + ' '.join(
                '%5d %5.1f%%' % (tot[k, r], 100.0 * tot[k, r] / max(n, 1)) for r in RULES))
        return 0
    if cmd == 'misses':
        rule = argv[1]
        for seg, rt in done:
            for v, p, free in analyse(rt):
                if cls and v.reg[0] != cls:
                    continue
                if p[rule] != v.reg:
                    print('%06X %-28s %-3s pred=%-3s free=%-20s %s' % (
                        rt.ins[v.first].addr, m.describe(rt.r0)[:28], v.reg, p[rule],
                        ','.join(free), v.text))
        return 0
    if cmd == 'sched':
        global STRICT
        if '--strict' in argv:
            STRICT = True
            argv.remove('--strict')
        c = sched_survey(done, m, int(argv[1]) if len(argv) > 1 else 0)
        for k in sorted(c):
            print(k, c[k])
        return 0
    if cmd == 'csaved':
        c = Counter()
        for seg, rt in done:
            regs = {v.reg for v in rt.values}
            cs = bool(regs - SCRATCH)
            c[('calls' if rt.calls else 'nocalls'), cs] += 1
        print(dict(c))
        return 0
    print(__doc__)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
