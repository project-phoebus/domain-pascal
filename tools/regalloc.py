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
    tools/regalloc.py callargs [--min N] [--max N]
                                         # 12.4 R4(c): values last read by
                                         # a call's arguments, scratch or
                                         # callee-saved
    tools/regalloc.py callparams [N] [--min N] [--max N]
                                         # bead domain-pascal-9j1z: parameter
                                         # loads last used by a call's
                                         # argument push, by call position
                                         # (prefix, conditional part, after
                                         # a join), other paths and class
    tools/regalloc.py slots [RULE[:KEY]] [--min N] [--max N]
                                         # 12.4 R4(b): the register of slot
                                         # loads under each rule (slot_
                                         # predictions), by routine shape;
                                         # with RULE, its misses
    tools/regalloc.py slotpass [N] [--min N] [--max N]
                                         # bead domain-pascal-i24w: single
                                         # slot loads after the first
                                         # branch or call: candidate
                                         # registers' states taken or
                                         # passed over, the rules' hits on
                                         # the loads bead 178's rule
                                         # refuses (with alternate
                                         # halves), pass_rule's shape
    tools/regalloc.py orders [--split even|odd] [--tie rev] [--min N] [--max N]
                                         # bead domain-pascal-jjh: allocator
                                         # orders (Chow-Hennessy, Chaitin/
                                         # Briggs, usage count, two-level)
                                         # simulated over colU's graph
    tools/regalloc.py simmisses MODEL [loo|sim] [N] [--class d|a] ...
                                         # the values MODEL mispredicts
    tools/regalloc.py csaved [--min N] [--max N]
                                         # routines that use callee-saved
                                         # registers, with and without calls
    tools/regalloc.py ptrcopies [N]      # 12.2 "Pointers in data registers":
                                         # each `movea.l Dn,An' copy, made
                                         # while an address register still
                                         # holds a copy of the same value
                                         # or not (N examples)
    tools/regalloc.py paramlocal [N] [--class d|a]
                                         # bead domain-pascal-ue7: a held
                                         # parameter load and a held other
                                         # value that could swap registers:
                                         # which is lower (N examples)
    tools/regalloc.py ptrdregs [N]       # the same: parameters loaded into a
                                         # D register and dereferenced
                                         # through copies, by their other
                                         # uses and whether an address
                                         # register was free over their range

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
    tail = {'S': 'stmt', 'K': 'block', 'P': 'pdom', 'C': 'cond', 'A': 'arg',
            'O': 'argo'}.get(f[-1:]) if f else None
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
            if stmt in ('arg', 'argo'):
                rt.tails[key] = arg_tail(rt, v, outer=stmt == 'argo')
            else:
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


def stmt_end(x):
    """A store to memory that is not a push: the end of a statement."""
    return bool(x.ops) and is_mem(x.ops[-1]) and '%sp@-' not in x.ops[-1] and \
        not x.mn.startswith(('cmp', 'tst', 'btst', 'pea', 'jsr', 'jmp'))


def arg_call(rt, v):
    """(call, start) when every last use of v lies in the argument sequence
    of a call statement and v is not live across that call; else None.

    The argument sequence of a call is the run of instructions before it
    in its block after the previous statement: back to the last store to
    memory that is not a push, the last call, or the block start.  M$ARITH
    calls are arithmetic, not call statements, and do not count."""
    uses = [i for i in v.pts if v.reg in rt.ins[i].uses and rt.rd_in[i][v.reg] & v.keys]
    last = [i for i in uses if not (v.reg in rt.lout[i] and live_val_out(rt, v, i))]
    if not last:
        return None
    found = None
    for u in last:
        b = rt.block_of[u]
        blk = [j for j in rt.reach if rt.block_of[j] == b]
        k = blk.index(u)
        call = None
        for j in blk[k:]:
            x = rt.ins[j]
            if x.call:
                call = j
                break
            if x.kind or stmt_end(x):
                break
        if call is None or rt.ins[call].defs == {'d0', 'a0'}:
            return None
        start = blk[0]
        for j in reversed(blk[:blk.index(call)]):
            x = rt.ins[j]
            if x.call or stmt_end(x):
                start = j + 1
                break
        if found is None or found[0] != call:
            if found is not None:
                return None
            found = (call, start)
    return found


def arg_tail(rt, v, outer):
    """The instructions after v's last use up to and including the call
    whose arguments read it (arg_call); with `outer', only for values
    defined before that call statement."""
    ac = arg_call(rt, v)
    if not ac:
        return set()
    call, start = ac
    if outer and v.first >= start:
        return set()
    return {j for j in rt.reach if rt.block_of[j] == rt.block_of[call]
            and max(v.pts) < j <= call} | {call}


def last_uses(rt, v):
    """The instructions that read v and after which v is dead."""
    uses = [i for i in v.pts if v.reg in rt.ins[i].uses and rt.rd_in[i][v.reg] & v.keys]
    return [i for i in uses if not (v.reg in rt.lout[i] and live_val_out(rt, v, i))]


def is_push(x):
    """An argument push: `pea' or a move to -(SP)."""
    return x.mn == 'pea' or bool(x.ops and x.ops[-1] == '%sp@-' and x.mn.startswith('move'))


def push_call(rt, u):
    """The call (not M$ARITH) whose argument sequence holds instruction u
    (arg_call's sequence: up to the call, no branch or store to memory
    other than a push in between), else None."""
    b = rt.block_of[u]
    blk = [j for j in rt.reach if rt.block_of[j] == b]
    for j in blk[blk.index(u):]:
        x = rt.ins[j]
        if x.call:
            return None if x.defs == {'d0', 'a0'} else j
        if x.kind or stmt_end(x):
            return None
    return None


def dominators(rt):
    """{block: the blocks that dominate it} (blocks named by their first
    instruction's index)."""
    if not hasattr(rt, 'dom'):
        bl = sorted(set(rt.block_of.values()))
        bpred = {b: {rt.block_of[p] for p in rt.pred.get(b, [])} for b in bl}
        dom = {b: set(bl) for b in bl}
        dom[bl[0]] = {bl[0]}
        changed = True
        while changed:
            changed = False
            for b in bl[1:]:
                ps = [dom[p] for p in bpred[b]]
                n = (set.intersection(*ps) if ps else set()) | {b}
                if n != dom[b]:
                    dom[b], changed = n, True
        rt.dom = dom
    return rt.dom


def call_position(rt, c):
    """Where call c lies (bead domain-pascal-9j1z): 'straight' (a routine
    without branches), 'prefix' (before the routine's first branch or
    branch target), 'cond' (in a conditional part: some path from the
    entry to the exit avoids it, and no join comes before it), 'join'
    (likewise, after a join: a block with two predecessors dominates it)
    or 'every' (on every path, after a join)."""
    br = [i for i in rt.reach if rt.ins[i].kind in ('bcc', 'br', 'db')]
    if not br:
        return 'straight'
    labs = [i for i in rt.reach if is_label(rt, i)]
    if c < min(br + labs):
        return 'prefix'
    ipdom_region(rt, rt.reach[0])           # builds rt.pdom
    cb = rt.block_of[c]
    merge = any(len(rt.pred.get(b, [])) > 1 for b in dominators(rt)[cb])
    if cb in rt.pdom[rt.block_of[rt.reach[0]]]:
        return 'every'
    return 'join' if merge else 'cond'


def call_param(rt, v):
    """Bead domain-pascal-9j1z (12.4 R4(c)): for a parameter load v that
    crosses no call and whose last use on some path is a push in the
    argument sequence of a call statement it does not belong to, the
    tuple (load, slot, position, paths, class, where, forced, call);
    else None.

    load: 'entry' (before the routine's first branch, branch target or
    call) or 'late'.  slot: 'once' (no other instruction reads the slot)
    or 'reload'.  position: call_position of the first such call.  paths:
    'dead' (every last use is such a push, of one call), 'multi' (of
    several calls), 'other' (another last use is not one: the value is
    live on another path).  where: 'csaved' or 'scratch'.  forced: no
    scratch register is free over the value's range by liveness alone
    (colU), so the choice says nothing about holding."""
    if not v.param or v.crosses_call:
        return None
    lu = last_uses(rt, v)
    calls = {u: push_call(rt, u) if is_push(rt.ins[u]) else None for u in lu}
    args = [u for u in lu if calls[u] is not None]
    if not args:
        return None
    c0 = min(calls[u] for u in args)
    if rt.block_of[v.first] == rt.block_of[c0] and v.first >= stmt_start(rt, c0):
        return None                         # defined inside the call statement
    br = [i for i in rt.reach if rt.ins[i].kind in ('bcc', 'br', 'db') or is_label(rt, i)
          or is_call(rt.ins[i])]
    load = 'entry' if v.first < min(br + [len(rt.ins)]) else 'late'
    slot = rt.ins[v.first].ops[0]
    reload = any(rt.ins[j].ops and rt.ins[j].ops[0] == slot for j in rt.reach if j not in v.defs)
    if len(args) < len(lu):
        paths = 'other'
    else:
        paths = 'multi' if len({calls[u] for u in args}) > 1 else 'dead'
    cand = [r for r in v.cls if not interferes_any(rt, v, r, use=True)]
    forced = not cand or cand[0] not in SCRATCH
    return ('entry' if load == 'entry' else 'late', 'reload' if reload else 'once',
            call_position(rt, c0), paths, v.reg[0],
            'scratch' if v.reg in SCRATCH else 'csaved', forced, c0)


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


# -- 12.4 R4(b): registers of by-address slot loads (bead domain-pascal-178)

def slot_values(rt):
    """Address values started by `movea.l (d,A6),An' with d > 0 (a
    parameter slot: by address, or a pointer by value)."""
    return [v for v in rt.values
            if v.param and v.reg[0] == 'a' and rt.ins[v.first].mn == 'moveal']


def is_label(rt, i):
    """Instruction i is the target of a branch (a join or a loop head)."""
    return any(rt.ins[p].target == rt.ins[i].addr and not rt.ins[p].call
               for p in rt.pred.get(i, []))


def is_call(x):
    """A call other than M$ARITH (which writes only D0 and A0, R3)."""
    return x.call and x.defs != {'d0', 'a0'}


def flow_sets(rt, step, init=frozenset(), meet='any'):
    """{i: set before instruction i}: a forward data flow over the CFG,
    `step(i, x, set) -> set' per instruction, sets joined by union ('any')
    or intersection ('all')."""
    D, out = {}, {}
    changed = True
    while changed:
        changed = False
        for i in rt.reach:
            ins = [out[p] for p in rt.pred.get(i, []) if p in out]
            if i == 0:
                ins.append(init)
            if not ins:
                continue
            d = frozenset.union(*ins) if meet == 'any' else frozenset.intersection(*ins)
            o = frozenset(step(i, rt.ins[i], d))
            if D.get(i) != d or out.get(i) != o:
                D[i], out[i] = d, o
                changed = True
    return D


def dirty_sets(rt, meet='any', calls='clean', reset=None):
    """{i: the address registers "dirty" before instruction i}: an earlier
    instruction left a value in them, on some path ('any') or on every
    path ('all').  A value used up by `(An)+' / `-(An)' leaves An clean.
    A call: 'clean' makes the registers it writes clean, 'keep' leaves the
    set as it is.  `reset': 'label' empties the set at a branch target,
    'block' at every block start."""
    AR = set(AREGS)

    def step(i, x, d):
        if (reset == 'block' and i in rt.leaders) or (reset == 'label' and is_label(rt, i)):
            d = frozenset()
        if x.call:
            return d - (x.defs & AR) if calls == 'clean' else d
        return (d | (x.defs & AR)) - (x.incdec & AR)
    return flow_sets(rt, step, meet=meet)


def written_sets(rt):
    """{i: registers written before i on some path by an instruction that
    is not a call}, and {i: True when a call (not M$ARITH) precedes i on
    some path} (as the pseudo-register 'call')."""
    def step(i, x, d):
        if x.call:
            return d | ({'call'} if is_call(x) else set())
        return d | (x.defs & set(AREGS))
    return flow_sets(rt, step)


def ext_back(rt, v):
    """v's load extended back to the start of its block, or to the last
    call before it there."""
    b = rt.block_of[v.first]
    back = set()
    for i in reversed([j for j in rt.reach if rt.block_of[j] == b and j < v.first]):
        if rt.ins[i].call:
            break
        back.add(i)
    return back


def free_stack(rt):
    """{i: free address registers before i, top first}: a stack of free
    registers in address order.  A new value takes its register off the
    stack; a value that dies stays "cached" (not returned) unless `(An)+'
    used it up, which puts An back among the never-used registers in
    numeric order; a call pushes the scratch registers it kills that are
    not on the stack, A0 first (so A1 ends on top)."""
    free, st = list(AREGS), {}
    for i in rt.reach:
        st[i] = list(free)
        x = rt.ins[i]
        if x.call:
            for r in [r for r in ('a0', 'a1') if r in x.defs and r not in free]:
                free.insert(0, r)
            continue
        for r in sorted(x.defs & set(AREGS)):
            if r in free:
                free.remove(r)
            if r in x.incdec:
                k = 0
                while k < len(free) and free[k] < r:
                    k += 1
                free.insert(k, r)
    return st


# Dirty-set variants: name -> dirty_sets arguments.
DIRTY_VARIANTS = {'yc': ('any', 'clean', None), 'lc': ('all', 'clean', None),
                  'yk': ('any', 'keep', None), 'lk': ('all', 'keep', None),
                  'ycL': ('any', 'clean', 'label'), 'ycB': ('any', 'clean', 'block')}
SLOT_RULES = ('colU',) + tuple(DIRTY_VARIANTS) + ('ext', 'stack', 'pass', 'dpc')


def slot_predictions(rt, v, dirty, written, stack):
    """{rule: register} for slot value v (codegen-patterns 12.4 R4(b)).

    colU: the lowest register free over the range (R1-R3).  The dirty
    variants: the lowest colU candidate not dirty at the load (dirty_sets;
    first letter y/l = dirty on any / all paths; then c/k = a call cleans
    the registers it writes / keeps them; L/B = emptied at branch targets
    / block starts); a reload of the same slot into the register holding
    it is allowed.  ext: the load's range extended back to its block start
    or the last call.  stack: free_stack's top candidate.  pass: pass_rule
    (None outside its shape).  dpc: what `dpc' implements (beads
    domain-pascal-178, -i24w): colU when that register is clean on every
    path (yc); when it is dirty on some path, the yc pick if every dirty
    register was written since a call earlier in the load's block
    (since_call), else the pass pick (a slot no other instruction names),
    else 'refused'; and 'refused' for A0
    after a call while A1 was written before."""
    cand = [c for c in v.cls if not interferes_any(rt, v, c, use=True)]
    p = {'colU': cand[0] if cand else None}
    slot = rt.ins[v.first].ops[0]

    def same(c):
        ds = rt.rd_in[v.first][c]
        return bool(ds) and all(d != 'entry' and rt.ins[d].mn == 'moveal' and
                                rt.ins[d].ops[0] == slot for d, _ in ds)
    for k, d in dirty.items():
        dd = d.get(v.first, frozenset())
        ok = [c for c in cand if c not in dd or same(c)]
        p[k] = ok[0] if ok else p['colU']
    back = ext_back(rt, v)
    ok = [c for c in cand if not any(
        w.reg == c and w is not v and w.var != v.var and (back & w.pts)
        for w in rt.values + rt.livein)]
    p['ext'] = ok[0] if ok else p['colU']
    ok = [c for c in cand if same(c)] + [c for c in stack.get(v.first, []) if c in cand]
    p['stack'] = ok[0] if ok else p['colU']
    p['pass'] = pass_rule(cand, slot_states(rt, v, cand))
    w = written.get(v.first, frozenset())
    yc = dirty['yc'].get(v.first, frozenset())
    pick = p['colU']
    if pick in yc:
        # straight code since a call in the load's block that wrote every
        # dirty register: the straight-line rule, restarted at the call
        local = since_call(rt, v.first)
        pick = p['yc'] if local is not None and yc <= local else 'refused'
        if pick == 'refused' and sum(slot in x.text for x in rt.ins) == 1:
            # bead domain-pascal-i24w: past dead slot loads to a register
            # never written or used up, for a slot no other instruction
            # names (a slot named elsewhere too reuses the register of a
            # dead load of the same slot, 0xE39E78; dpc refuses it)
            pick = p['pass'] or 'refused'
    if pick == 'a0' and 'call' in w and 'a1' in w:
        pick = 'refused'
    p['dpc'] = pick
    return p


def reach_kind(rt, d, r):
    """What a definition `d' (rd_in's (index, reg) first element) left in
    address register r at a later load (bead domain-pascal-i24w): 'entry'
    (no instruction wrote r on that path), 'call' (a call clobbered it),
    'incdec' (written only by `(An)+' / `-(An)': used up), 'slot' (a
    parameter slot load `movea.l (d,A6),An', d > 0), or a finer kind of
    other value: 'local' (`movea.l (-d,A6),An'), 'const', 'copy'
    (`movea.l Rn,An', `lea (An),Am'), 'loadidx' / 'load' (through a
    register, with / without an index), 'leaidx' (`lea' with an index
    register), 'leastat' (`lea' of A5, A6, PC or absolute), 'leaoff'
    (`lea (d,An),Am'), 'arith' (`addq', `adda', ...), 'other:MN'."""
    if d == 'entry':
        return 'entry'
    x = rt.ins[d]
    if x.call:
        return 'call'
    if r in x.incdec:
        return 'incdec'
    src = x.ops[0] if x.ops else ''
    if x.mn == 'moveal':
        if re.fullmatch(r'%fp@\(\d+\)', src):
            return 'slot'
        if re.fullmatch(r'%fp@\(-\d+\)', src):
            return 'local'
        if src.startswith('#'):
            return 'const'
        if is_reg(src):
            return 'copy'
        return 'loadidx' if '%d' in src else 'load'
    if x.mn == 'lea':
        if re.fullmatch(r'%a\d@', src):
            return 'copy'
        if '%d' in src:
            return 'leaidx'
        if re.match(r'%(a5|fp|pc)@|0x', src):
            return 'leastat'
        return 'leaoff'
    if x.mn in ('addql', 'subql', 'addal', 'addaw', 'subal', 'subaw'):
        return 'arith'
    return 'other:' + x.mn


def slot_states(rt, v, cand):
    """{register: frozenset of reach_kind} for the candidate registers of
    slot value v at its load: what the definitions reaching it on every
    path left there."""
    return {c: frozenset(reach_kind(rt, d, r) for d, r in rt.rd_in[v.first][c])
            for c in cand}


def pass_rule(cand, states):
    """12.4 R4(b), bead domain-pascal-i24w: the lowest candidate register
    that no instruction wrote on any path ({'entry'}) or that a block copy
    used up ({'incdec'}), when it is not the lowest candidate and every
    candidate below it holds a dead parameter slot load on every path
    ({'slot'}).  None when the load is not of that shape."""
    for c in cand:
        s = states[c]
        if s in ({'entry'}, {'incdec'}):
            return c if c != cand[0] else None
        if s != {'slot'}:
            return None
    return None


def since_call(rt, i):
    """The registers written between the last call before i in i's block
    and i (None when no call precedes i there)."""
    blk = [j for j in rt.reach if rt.block_of[j] == rt.block_of[i] and j < i]
    calls = [j for j in blk if rt.ins[j].call]
    if not calls:
        return None
    out = set()
    for j in blk:
        if j > max(calls):
            out |= rt.ins[j].defs
    return out


def slot_shape(rt):
    """'straight', 'branches', 'calls' or 'both' (M$ARITH calls do not
    count), and the index of the first branch, call or label."""
    calls = [i for i in rt.calls if is_call(rt.ins[i])]
    br = [i for i in rt.reach if rt.ins[i].kind in ('bcc', 'br', 'db')]
    lab = [i for i in rt.reach if is_label(rt, i)]
    shape = {(False, False): 'straight', (True, False): 'branches',
             (False, True): 'calls', (True, True): 'both'}[(bool(br), bool(calls))]
    return shape, min(br + calls + lab + [len(rt.ins)])


def stmt_start(rt, i):
    """The first instruction of i's "statement": after the last call or
    store to memory that is not a push (stmt_end) before i in its block,
    else the block's first instruction."""
    b = rt.block_of[i]
    s = b
    for j in rt.reach:
        if j >= i:
            break
        if rt.block_of[j] == b and (rt.ins[j].call or stmt_end(rt.ins[j])):
            s = j + 1
    return s


def pass_variants(rt, flow):
    """The dirty-set variants slot_pass_survey scores besides
    DIRTY_VARIANTS: {name: {i: set}}.  E: the parameter slot loads before
    the routine's first branch, call or label (its entry loads) do not
    make a register dirty; S (any / all path): the set at the start of the
    load's statement (stmt_start), so that the registers the load's own
    statement wrote count as free."""
    AR = set(AREGS)

    def step_e(i, x, d):
        if x.call:
            return d - (x.defs & AR)
        if i < flow and reach_kind(rt, i, None) == 'slot':
            return d
        return (d | (x.defs & AR)) - (x.incdec & AR)
    out = {'ycE': flow_sets(rt, step_e), 'lcE': flow_sets(rt, step_e, meet='all')}
    for k, meet in (('ycS', 'any'), ('lcS', 'all')):
        d = dirty_sets(rt, meet)
        out[k] = {i: d.get(stmt_start(rt, i), frozenset()) for i in d}
    return out


PASS_RULES = ('colU', 'yc', 'lc', 'ycL', 'ycE', 'lcE', 'ycS', 'lcS', 'stack', 'pass')


def slot_pass_survey(done):
    """Bead domain-pascal-i24w: the slot loads of slot_survey after the
    first branch, call or label, read once ('single'), as [(rt, v,
    predictions, states, refused)], predictions extended with
    pass_variants' rules, states slot_states', refused: the pick of
    domain-pascal-178's rule ('dpc' without the pass rule) is 'refused'
    because the lowest free register is dirty on some path."""
    res = []
    for rt, v, shape, pos, use, p in slot_survey(done):
        if pos != 'after' or use != 'single':
            continue
        if getattr(rt, '_pv', None) is None:
            rt._pv = pass_variants(rt, slot_shape(rt)[1])
        cand = [c for c in v.cls if not interferes_any(rt, v, c, use=True)]
        for k, d in rt._pv.items():
            dd = d.get(v.first, frozenset())
            ok = [c for c in cand if c not in dd]
            p[k] = ok[0] if ok else p['colU']
        st = slot_states(rt, v, cand)
        yc = dirty_sets(rt).get(v.first, frozenset())
        local = since_call(rt, v.first)
        refused = bool(cand) and cand[0] in yc and not (local is not None and yc <= local)
        res.append((rt, v, p, st, refused))
    return res


def slot_survey(done):
    """[(rt, v, shape, position, use, predictions)] for every slot value
    of the routines in `done'.  position: 'before' or 'after' the first
    branch, call or label; use: 'single' (the slot is named by this load
    only, its value used in one block and not across a call: what `dpc'
    loads where it is needed), 'reload' (likewise, the slot named
    elsewhere too), 'held' (the rest)."""
    res = []
    for seg, rt in done:
        vs = slot_values(rt)
        if not vs:
            continue
        shape, flow = slot_shape(rt)
        dirty = {k: dirty_sets(rt, *a) for k, a in DIRTY_VARIANTS.items()}
        written = written_sets(rt)
        stack = free_stack(rt)
        for v in vs:
            uses = [i for i in v.pts if v.reg in rt.ins[i].uses and rt.rd_in[i][v.reg] & v.keys]
            slot = rt.ins[v.first].ops[0]
            single = sum(slot in x.text for x in rt.ins) == 1
            once = len({rt.block_of[i] for i in v.pts}) == 1 and not v.crosses_call
            use = ('single' if single else 'reload') if once and uses else 'held'
            res.append((rt, v, shape, 'after' if v.first > flow else 'before', use,
                        slot_predictions(rt, v, dirty, written, stack)))
    return res


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


def ptr_copies(rt):
    """[(kind, insn)] for each `movea.l Dn,An' whose data register has a
    definition on every path: 'fresh' when no address register holds a
    copy of the same value there (on every path), 'same' when An itself
    does, 'other' when another one does."""
    out = []
    for i in rt.reach:
        x = rt.ins[i]
        if x.mn != 'moveal' or not re.fullmatch(r'%d[0-7]', x.ops[0]):
            continue
        dn, am = x.ops[0][1:], x.ops[1][1:]
        dsrc = rt.rd_in[i][dn]
        if not dsrc or any(k[0] == 'entry' for k in dsrc):
            continue
        holders = []
        for a in AREGS:
            ds = rt.rd_in[i][a]
            if not ds or any(k[0] == 'entry' for k in ds):
                continue
            if all(rt.ins[j].mn == 'moveal' and rt.ins[j].ops[0] == '%' + dn
                   and rt.rd_in[j][dn] == dsrc for j, _ in ds):
                holders.append(a)
        out.append(('fresh' if not holders else 'same' if am in holders else 'other', x))
    return out


def value_uses(rt, v):
    return [p for p in sorted(v.pts)
            if v.reg in rt.ins[p].uses and rt.rd_in[p][v.reg] & v.keys]


def ptr_dregs(rt):
    """[(key, text)] for each value in a D register read by at least one
    `movea.l Dn,An' copy.  key: 'param' (a longword parameter load) or
    'other'; its other uses ('push' = `move.l Dn,-(SP)', 'cmp', 'arith',
    'store' = to memory, 'move' = to a register, '-' none); '1copy' or
    'copies'; 'Afree' when some address register A0-A4 is free over its
    range (colU's interference, the value's own copies left out), else
    'Afull'."""
    out = []
    for v in rt.values:
        if v.reg[0] != 'd':
            continue
        r = '%' + v.reg
        kinds = []
        for p in value_uses(rt, v):
            x = rt.ins[p]
            if x.mn == 'moveal' and x.ops[0] == r:
                kinds.append('copy')
            elif x.mn == 'movel' and x.ops[0] == r and x.ops[1] == '%sp@-':
                kinds.append('push')
            elif (x.mn.startswith('cmp') or x.mn.startswith('tst')) and r in x.ops:
                kinds.append('cmp')
            elif x.mn.startswith('move') and x.ops[0] == r:
                kinds.append('move' if is_reg(x.ops[1]) else 'store')
            else:
                kinds.append('arith')
        n = kinds.count('copy')
        if not n:
            continue

        def is_copy(w):
            return all(rt.ins[d].mn == 'moveal' and rt.ins[d].ops[0] == r
                       and rt.rd_in[d][v.reg] & v.keys for d in w.defs)
        free = []
        saved = rt.values
        for c in AREGS:
            rt.values = [w for w in saved if not (w.reg == c and is_copy(w))]
            try:
                if not interferes_any(rt, v, c, use=True):
                    free.append(c)
            finally:
                rt.values = saved
        param = v.param and rt.ins[v.first].mn == 'movel'
        other = ','.join(sorted(set(k for k in kinds if k != 'copy'))) or '-'
        key = ('param' if param else 'other', other, '1copy' if n == 1 else 'copies',
               'Afree' if free else 'Afull')
        out.append((key, '%06X:%s%s' % (rt.ins[v.first].addr, v.reg,
                                         '/' + ','.join(free) if free else '')))
    return out


def def_kind(rt, v):
    """How a value starts: 'slot' (a parameter load), 'callres' (a copy
    of A0/D0 right after a call: a call result kept), 'copy' (another
    register copy), 'const', 'load' (through a register or from A5/an
    absolute address), 'lea', 'other'."""
    if v.param:
        return 'slot'
    x = rt.ins[v.first]
    src = x.ops[0] if x.ops else ''
    if x.mn.startswith('move') and src in ('%a0', '%d0'):
        j = v.first - 1
        while j >= 0 and rt.ins[j].ops and rt.ins[j].ops[-1] == '%sp' and not rt.ins[j].call:
            j -= 1                          # the caller's pop
        if j >= 0 and rt.ins[j].call:
            return 'callres'
    if x.mn.startswith('move') and is_reg(src):
        return 'copy'
    if x.mn in ('moveq', 'moveql', 'clrl', 'clrw', 'clrb') or src.startswith('#'):
        return 'const'
    if x.mn == 'lea':
        return 'lea'
    if x.mn.startswith('move'):
        return 'load'
    return 'other'


def param_pairs(rt, g=None):
    """Bead domain-pascal-ue7: pairs of interfering values of one class,
    both in callee-saved registers (D2-D7 / A2-A4), one a parameter load
    (P) and the other not (L), that could swap registers (each register
    is free of the other's remaining neighbours and not forbidden).
    [(P, L, key)], key = (class, which is lower: 'L<P' or 'P<L', which is
    defined first: 'Pfirst'/'Lfirst', P crosses a call, L crosses a call,
    L's def_kind)."""
    g = g or Graph(rt)
    vs = rt.values
    out = []
    for kp, P in enumerate(vs):
        if not P.param or P.reg in SCRATCH:
            continue
        for kl in g.nb[kp]:
            L = vs[kl]
            if L.param or L.reg in SCRATCH or L.cls is not P.cls:
                continue
            others_p = {vs[j].reg for j in g.nb[kp] if j != kl}
            others_l = {vs[j].reg for j in g.nb[kl] if j != kp}
            if (L.reg == P.reg or L.reg in others_p or L.reg in g.forbid[kp]
                    or P.reg in others_l or P.reg in g.forbid[kl]):
                continue
            key = (P.reg[0], 'L<P' if L.reg < P.reg else 'P<L',
                   'Pfirst' if P.first < L.first else 'Lfirst',
                   'Pcall' if P.crosses_call else 'Pnocall',
                   'Lcall' if L.crosses_call else 'Lnocall', def_kind(rt, L))
            out.append((P, L, key))
    return out


# -- allocator simulations (bead domain-pascal-jjh) ----------------------
#
# The values of a routine and colU's interference, as a graph that does not
# depend on the registers the original chose: an edge joins two values of
# one class that overlap (Chaitin) or where one is defined by an
# instruction reading the other (R2, "U"); values of one parameter slot
# are not joined.  `forbid' holds the registers a value can never take:
# those a call inside its range writes (R3) and those of occupants that
# are not allocation decisions (call results, partly undefined values,
# rt.livein).  An order is then scored two ways:
#   loo  the value gets the lowest register its neighbours placed *earlier
#        in the order* leave free, the neighbours keeping the original's
#        registers (order-free = colU).  loo in address order is close
#        to colQU but not equal: colQU also refuses a register read by
#        the defining instruction when its value is defined later in
#        address order (a loop-carried read), and lets a load through
#        its own dying base keep that base;
#   sim  a full simulation: the neighbours keep the registers the
#        simulation gave them (errors compound).

class Graph:
    def __init__(self, rt):
        self.rt = rt
        vs = rt.values
        self.idx = {id(v): k for k, v in enumerate(vs)}
        self.nb = [set() for _ in vs]
        self.forbid = [set() for _ in vs]
        reads = [self._reads(v) for v in vs]
        for k, v in enumerate(vs):
            for p in v.pts:
                x = rt.ins[p]
                if x.call and v.reg in rt.lout[p]:
                    self.forbid[k] |= x.defs & set(v.cls)
            for w in rt.livein:
                if w.reg in v.cls and (overlap(rt, v, w, v.pts, w.pts) or
                                       any(d in v.pts and d in reads[k] for d in w.defs) or
                                       any(w.reg in rt.ins[d].uses and rt.rd_in[d][w.reg] & w.keys
                                           for d in v.defs)):
                    self.forbid[k].add(w.reg)
            # registers the defining instruction reads that hold no value
            # (e.g. a static link live at the entry)
            for d in v.defs:
                for r in rt.ins[d].uses & set(v.cls):
                    if r != v.reg and not any(rt.rd_in[d][r] & u.keys
                                              for u in vs + rt.livein if u.reg == r):
                        self.forbid[k].add(r)
        for k, v in enumerate(vs):
            for j in range(k + 1, len(vs)):
                w = vs[j]
                if w.cls is not v.cls or w.var == v.var:
                    continue
                if (any(d in reads[k] for d in w.defs) or any(d in reads[j] for d in v.defs)
                        or overlap(rt, v, w, v.pts, w.pts)):
                    self.nb[k].add(j)
                    self.nb[j].add(k)

    def _reads(self, v):
        return {p for p in v.pts if v.reg in self.rt.ins[p].uses
                and self.rt.rd_in[p][v.reg] & v.keys}

    def pick(self, k, taken):
        v = self.rt.values[k]
        return next((c for c in v.cls if c not in taken and c not in self.forbid[k]), None)


def loop_depth(rt):
    """{instruction index: loop nesting depth}: a back edge (a branch to an
    earlier or the same address) makes the range [target, branch] a loop."""
    heads = {}
    for i in rt.reach:
        for s in rt.succ[i]:
            if s <= i:
                heads[s] = max(heads.get(s, s), i)
    return {i: sum(1 for s, e in heads.items() if s <= i <= e) for i in rt.reach}


def refs(rt, v, depth, w):
    """Chow-Hennessy savings: each definition and use weighted w**depth."""
    occ = set(v.defs) | {p for p in v.pts if v.reg in rt.ins[p].uses
                         and rt.rd_in[p][v.reg] & v.keys}
    return sum(w ** depth[p] for p in occ)


def is_global(rt, v):
    return len(v.blocks) > 1 or v.crosses_call


def kfree(g, k):
    return len(g.rt.values[k].cls) - len(g.forbid[k])


def order_priority(g, prio, tie, defer=False):
    """Values by descending priority (ties: address order, or reverse
    with tie=-1); with `defer', unconstrained values (fewer neighbours
    than free registers) last, as Chow-Hennessy colour them."""
    vs = g.rt.values
    ks = sorted(range(len(vs)), key=lambda k: (-prio(vs[k]), tie * vs[k].first))
    if defer:
        con = [k for k in ks if len(g.nb[k]) >= kfree(g, k)]
        ks = con + [k for k in ks if k not in con]
    return ks


def order_simplify(g, tie, pick='addr', optimistic=True, cost=None):
    """Chaitin/Briggs: remove a node with degree < k (its class's free
    registers) while one exists (choice: lowest address, `pick'='deg'
    smallest degree first); when blocked, remove the node of least
    cost/degree (Briggs: optimistic, coloured if possible; Chaitin: a
    spill, returned in `spilled').  Select order: reverse removal."""
    vs = g.rt.values
    left = set(range(len(vs)))
    deg = {k: len(g.nb[k]) for k in left}
    stack, spilled = [], set()
    while left:
        ok = [k for k in left if deg[k] < kfree(g, k)]
        if ok:
            if pick == 'deg':
                k = min(ok, key=lambda k: (deg[k], tie * vs[k].first))
            else:
                k = min(ok, key=lambda k: tie * vs[k].first)
        else:
            k = min(left, key=lambda k: ((cost(vs[k]) if cost else 1) / max(deg[k], 1),
                                         tie * vs[k].first))
            if not optimistic:
                spilled.add(k)
        left.discard(k)
        stack.append(k)
        for j in g.nb[k]:
            if j in left:
                deg[j] -= 1
    return stack[::-1], spilled


def run_order(g, order, spilled=()):
    """{value index: (loo prediction, sim prediction)} for an order."""
    vs = g.rt.values
    pos = {k: n for n, k in enumerate(order)}
    sim, out = {}, {}
    for k in order:
        loo = g.pick(k, {vs[j].reg for j in g.nb[k] if pos[j] < pos[k]})
        s = None if k in spilled else g.pick(k, {sim[j] for j in g.nb[k] if j in sim})
        if s is not None:
            sim[k] = s
        out[k] = (loo, s)
    return out


def models(rt, g, tie=1):
    """{model name: (order, spilled)}."""
    vs = rt.values
    depth = loop_depth(rt)
    out = {'addr': (sorted(range(len(vs)), key=lambda k: vs[k].first), ()),
           'raddr': (sorted(range(len(vs)), key=lambda k: -vs[k].first), ()),
           # bead domain-pascal-ue7: by the end of the live range, latest
           # first (a backward linear scan), ties in address order
           'rlast': (sorted(range(len(vs)), key=lambda k: (-vs[k].last, vs[k].first)), ())}
    for w in (1, 10):
        cnt = {id(v): refs(rt, v, depth, w) for v in vs}
        sw = '' if w == 1 else 'w'
        out['use' + sw] = (order_priority(g, lambda v: cnt[id(v)], tie), ())
        for size in ('i', 'b'):
            def prio(v, size=size):
                return cnt[id(v)] / (len(v.pts) if size == 'i' else len(v.blocks))
            for defer in (False, True):
                name = 'ch%s%s%s' % (size, sw, 'd' if defer else '')
                out[name] = (order_priority(g, prio, tie, defer), ())
    cnt = {id(v): refs(rt, v, depth, 10) for v in vs}
    for pick in ('addr', 'deg'):
        for opt in (True, False):
            name = ('briggs' if opt else 'chaitin') + ('' if pick == 'addr' else '-deg')
            out[name] = order_simplify(g, tie, pick, opt, lambda v: cnt[id(v)])
    # two-level: global values (live in more than one block or across a
    # call) first in the given order, then the block-local ones in address
    # order
    for base in ('addr', 'chbwd', 'briggs', 'use'):
        o, sp = out[base]
        glob = [k for k in o if is_global(rt, vs[k])]
        loc = sorted((k for k in o if not is_global(rt, vs[k])), key=lambda k: vs[k].first)
        out['2l-' + base] = (glob + loc, sp)
        # the other way round: block-local values first (address order)
        out['lf-' + base] = (loc + glob, sp)
    # bead domain-pascal-ue7: a parameter load ordered by its first use
    # rather than its load (`fu'); values live in more than one block
    # (crossing a call not counted) before the single-block ones (`2lb-')
    def key_fu(k):
        v = vs[k]
        if v.param:
            us = value_uses(rt, v)
            return us[0] if us else v.first
        return v.first
    multi = [k for k in range(len(vs)) if len(vs[k].blocks) > 1]
    single = [k for k in range(len(vs)) if len(vs[k].blocks) <= 1]
    out['fu'] = (sorted(range(len(vs)), key=lambda k: (key_fu(k), vs[k].first)), ())
    for name, key in (('addr', lambda k: vs[k].first),
                      ('fu', lambda k: (key_fu(k), vs[k].first))):
        out['2lb-' + name] = (sorted(multi, key=key) + sorted(single, key=key), ())
    return out


MODELS = ('addr', 'raddr', 'rlast', 'fu', '2lb-addr', '2lb-fu', 'use', 'usew', 'chi', 'chid', 'chb', 'chbd', 'chiw', 'chiwd',
          'chbw', 'chbwd', 'briggs', 'chaitin', 'briggs-deg', 'chaitin-deg',
          '2l-addr', '2l-chbwd', '2l-briggs', '2l-use',
          'lf-addr', 'lf-chbwd', 'lf-briggs', 'lf-use')


def sim_survey(done, tie=1):
    """[(rt, value, {model: (loo, sim)}, colU-from-the-graph)] for every
    value of the routines in `done'."""
    res = []
    for seg, rt in done:
        if not rt.values:
            continue
        g = Graph(rt)
        ms = models(rt, g, tie)
        runs = {name: run_order(g, *ms[name]) for name in MODELS}
        for k, v in enumerate(rt.values):
            free = g.pick(k, {rt.values[j].reg for j in g.nb[k]})
            res.append((rt, v, {name: runs[name][k] for name in MODELS}, free))
    return res


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
    if cmd == 'callargs':
        # 12.4 R4(c): values whose every last use is in the arguments of
        # one call (arg_call) and that do not cross it.  "outer": defined
        # before that call statement; counted only where liveness alone
        # (colU's ranges) leaves a scratch register free.  "inner":
        # temporaries computed inside the argument list.
        tot, ex = Counter(), defaultdict(list)
        for seg, rt in done:
            branchy = any(rt.ins[i].kind in ('bcc', 'br', 'db') for i in rt.reach)
            for v in rt.values:
                ac = arg_call(rt, v)
                if not ac or v.crosses_call:
                    continue
                where = 'scratch' if v.reg in SCRATCH else 'csaved'
                if v.first >= ac[1]:
                    tot['inner', where] += 1
                    continue
                if not any(v.reg in rt.ins[i].uses for i in v.pts if ac[1] <= i < ac[0]):
                    continue
                cand = [r for r in v.cls if not interferes_any(rt, v, r, use=True)]
                if not cand or cand[0] not in SCRATCH:
                    continue
                k = ('param' if v.param else 'other',
                     'branchy' if branchy else 'straight', where)
                tot[k] += 1
                ex[k].append('%06X' % rt.ins[v.first].addr)
        print('inner: scratch %d callee-saved %d' % (tot['inner', 'scratch'],
                                                     tot['inner', 'csaved']))
        for k in sorted(ex):
            print('%-6s %-8s %-8s %4d  %s' % (k + (tot[k], ' '.join(ex[k][:12]))))
        return 0
    if cmd == 'callparams':
        # bead domain-pascal-9j1z: parameter loads whose last use is an
        # argument push of a call (call_param), by load, slot, call
        # position, paths and class: callee-saved / scratch, and the
        # forced ones (no scratch register free by liveness) apart.
        show = int(argv[1]) if len(argv) > 1 else 10
        tot, ex = Counter(), defaultdict(list)
        for seg, rt in done:
            for v in rt.values:
                k = call_param(rt, v)
                if k is None:
                    continue
                key, col = k[:5], k[5] + ('-forced' if k[6] else '')
                tot[key, col] += 1
                ex[key, col].append('%06X' % rt.ins[v.first].addr)
        cols = ('csaved', 'scratch', 'csaved-forced', 'scratch-forced')
        print('%-36s ' % 'load/slot/position/paths/class' + ' '.join('%8s' % c[:8] for c in
                                                                    ('csaved', 'scratch',
                                                                     'cs-forc', 'sc-forc')))
        for key in sorted({k for k, _ in tot}):
            print('%-36s ' % '/'.join(key) + ' '.join('%8d' % tot[key, c] for c in cols) +
                  '  ' + ' | '.join(' '.join(ex[key, c][:show]) for c in cols[:2]
                                    if ex[key, c]))
        return 0
    if cmd == 'slots':
        # 12.4 R4(b): which rule picks the register of a slot load.
        # `slots RULE[:KEY]' also lists the loads RULE mispredicts (KEY:
        # only those whose shape/position/use has it).
        want = argv[1].split(':') if len(argv) > 1 else None
        tot = Counter()
        for rt, v, shape, pos, use, p in slot_survey(done):
            k = (shape, pos, use)
            tot[k, 'n'] += 1
            for r in SLOT_RULES:
                tot[k, r] += p[r] == v.reg
            tot[k, 'refused'] += p['dpc'] == 'refused'
            if want and p[want[0]] != v.reg and (len(want) == 1 or want[1] in k):
                print('%06X %-28s %-3s %s  %s' % (
                    rt.ins[v.first].addr, m.describe(rt.r0)[:28], v.reg,
                    ' '.join('%s=%s' % (r, p[r]) for r in SLOT_RULES if p[r] != v.reg),
                    v.text))
        cols = SLOT_RULES + ('refused',)
        print('%-24s %5s ' % ('shape/position/use', 'n') + ' '.join('%5s' % r for r in cols))
        for k in sorted({k for k, _ in tot}):
            print('%-24s %5d ' % ('/'.join(k), tot[k, 'n']) +
                  ' '.join('%5d' % tot[k, r] for r in cols))
        return 0
    if cmd == 'slotpass':
        # bead domain-pascal-i24w: the single slot loads after the first
        # branch, call or label.  (1) per candidate register's state at
        # the load (slot_states), how often the image takes it (the
        # image's register) or passes over it (a lower candidate);
        # (2) the rules' hits on the loads domain-pascal-178's rule
        # refuses, in all routines and in alternate halves (address
        # order); (3) pass_rule's shape: the kind of the register it
        # picks, the registers passed over, hits and misses.
        show = int(argv[1]) if len(argv) > 1 else 6
        res = slot_pass_survey(done)
        half = {rt.r0: n % 2 for n, rt in enumerate(sorted({id(r): r for r, *_ in res}.values(),
                                                           key=lambda r: r.r0))}
        acc, ex = Counter(), defaultdict(list)
        for rt, v, p, st, refused in res:
            cand = list(st)
            if v.reg not in cand:
                continue
            for c in cand[:cand.index(v.reg) + 1]:
                k = ('+'.join(sorted(st[c])), 'take' if c == v.reg else 'pass')
                acc[k] += 1
                ex[k].append('%06X:%s' % (rt.ins[v.first].addr, c))
        print('(1) candidate state: taken / passed over (examples)')
        for s in sorted({s for s, _ in acc},
                        key=lambda s: (-(acc[s, 'take'] + acc[s, 'pass']), s)):
            print('  %-24s take %4d pass %4d  %s | %s' % (
                s[:24], acc[s, 'take'], acc[s, 'pass'], ' '.join(ex[s, 'take'][:show]),
                ' '.join(ex[s, 'pass'][:show])))
        tot = Counter()
        for rt, v, p, st, refused in res:
            if not refused:
                continue
            h = half[rt.r0]
            tot['n'] += 1
            tot['n', h] += 1
            for r in PASS_RULES:
                tot[r] += p[r] == v.reg
                tot[r, h] += p[r] == v.reg
            tot['pass', 'cover'] += p['pass'] is not None
            tot['pass', 'cover', h] += p['pass'] is not None
        print('(2) loads refused by the bead domain-pascal-178 rule: %d (halves %d / %d)' % (
            tot['n'], tot['n', 0], tot['n', 1]))
        for r in PASS_RULES:
            print('  %-6s %4d %5.1f%%  halves %3d / %3d' % (
                r, tot[r], 100.0 * tot[r] / max(tot['n'], 1), tot[r, 0], tot[r, 1]))
        print('  pass covers %d (halves %d / %d)' % (
            tot['pass', 'cover'], tot['pass', 'cover', 0], tot['pass', 'cover', 1]))
        shp, sex = Counter(), defaultdict(list)
        for rt, v, p, st, refused in res:
            if p['pass'] is None:
                continue
            cand = list(st)
            k = ('+'.join(sorted(st[p['pass']])),
                 ','.join(cand[:cand.index(p['pass'])]), p['pass'], p['pass'] == v.reg)
            shp[k] += 1
            sex[k].append('%06X' % rt.ins[v.first].addr)
        print('(3) pass_rule: picked register kind / passed over / pick: hits, misses')
        for k in sorted({k[:3] for k in shp}):
            print('  %-8s %-12s %-3s hit %3d miss %3d  %s | %s' % (
                k + (shp[k + (True,)], shp[k + (False,)], ' '.join(sex[k + (True,)][:show]),
                     ' '.join(sex[k + (False,)][:show]))))
        print('  total hit %d miss %d' % (sum(n for k, n in shp.items() if k[3]),
                                         sum(n for k, n in shp.items() if not k[3])))
        return 0
    if cmd == 'ptrcopies':
        show = int(argv[1]) if len(argv) > 1 else 10
        tot, ex = Counter(), defaultdict(list)
        for seg, rt in done:
            for k, x in ptr_copies(rt):
                tot[k] += 1
                ex[k].append('%06X' % x.addr)
        for k in sorted(tot):
            print('%-12s %5d  %s' % (k, tot[k], ' '.join(ex[k][:show])))
        return 0
    if cmd == 'ptrdregs':
        show = int(argv[1]) if len(argv) > 1 else 10
        tot, ex = Counter(), defaultdict(list)
        for seg, rt in done:
            for k, text in ptr_dregs(rt):
                tot[k] += 1
                ex[k].append(text)
        print('(param or other value, its uses other than copies, copies, '
              'address register free over its range)')
        for k in sorted(tot):
            print('%-40s %5d  %s' % ('/'.join(k), tot[k], ' '.join(ex[k][:show])))
        return 0
    if cmd == 'paramlocal':
        # bead domain-pascal-ue7: a parameter load and another value, both
        # callee-saved, interfering, able to swap: which gets the lower
        # register?
        show = int(argv[1]) if len(argv) > 1 else 6
        tot, ex = Counter(), defaultdict(list)
        for seg, rt in done:
            for P, L, key in param_pairs(rt):
                if cls and key[0] != cls:
                    continue
                tot[key] += 1
                ex[key].append('%06X:%s/%06X:%s' % (rt.ins[P.first].addr, P.reg,
                                                     rt.ins[L.first].addr, L.reg))
        print('(class, lower, first defined, P call, L call, L kind)')
        for k in sorted(tot, key=lambda k: (k[0], k[2:], k[1])):
            print('%-44s %5d  %s' % ('/'.join(k), tot[k], ' '.join(ex[k][:show])))
        return 0
    if cmd in ('orders', 'simmisses'):
        # bead domain-pascal-jjh: allocator orders simulated over colU's
        # interference graph (Graph, models, sim_survey).
        tie = -1 if '--tie' in argv and argv[argv.index('--tie') + 1] == 'rev' else 1
        split = argv[argv.index('--split') + 1] if '--split' in argv else None
        if split:
            # alternate routines in address order (bit 1 of the start
            # address would split by the loop-alignment mode, section 7)
            done = [d for n, d in enumerate(sorted(done, key=lambda d: d[1].r0))
                    if n % 2 == (split == 'odd')]
        res = sim_survey(done, tie)
        if cmd == 'simmisses':
            # positional arguments, without --tie/--split and their values
            pos = [a for n, a in enumerate(argv)
                   if a not in ('--tie', '--split') and argv[n - 1] not in ('--tie', '--split')]
            name, mode = pos[1], (pos[2] if len(pos) > 2 else 'loo')
            show = int(pos[3]) if len(pos) > 3 else 40
            n = 0
            for rt, v, p, free in res:
                pr = p[name][0 if mode == 'loo' else 1]
                if pr != v.reg and (not cls or v.reg[0] == cls) and n < show:
                    n += 1
                    print('%06X %-24s %-3s pred=%-4s %s %s' % (
                        rt.ins[v.first].addr, m.describe(rt.r0)[:24], v.reg, pr,
                        'glob' if is_global(rt, v) else 'loc ', v.text))
            return 0
        groups = ('d', 'a', 'glob', 'loc', 'all')
        tot = Counter()
        for rt, v, p, free in res:
            for gname in (v.reg[0], 'glob' if is_global(rt, v) else 'loc', 'all'):
                tot[gname, 'n'] += 1
                tot[gname, 'colU'] += free == v.reg
                for name in MODELS:
                    tot[gname, name, 0] += p[name][0] == v.reg
                    tot[gname, name, 1] += p[name][1] == v.reg
        print('routines %d (skipped %s), tie %s%s' % (
            len(done), dict(skipped), 'reverse address' if tie < 0 else 'address',
            ', split ' + split if split else ''))
        print('%-12s ' % 'loo/sim %' + ' '.join('%11s' % ('%s %d' % (g, tot[g, 'n']))
                                               for g in groups))
        pc = lambda g, k: 100.0 * tot[k] / max(tot[g, 'n'], 1)  # noqa: E731
        print('%-12s ' % 'colU' + ' '.join('%11.1f' % pc(g, (g, 'colU')) for g in groups))
        for name in MODELS:
            print('%-12s ' % name + ' '.join('%5.1f/%5.1f' % (
                pc(g, (g, name, 0)), pc(g, (g, name, 1))) for g in groups))
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
