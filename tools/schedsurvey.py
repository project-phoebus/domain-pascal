#!/usr/bin/env python3
"""schedsurvey.py - test candidate instruction-scheduling rules against the
compiled code of the SR10.2 image.

Evidence tool for docs/codegen-patterns.md section 12.4 R9-R11 (bead
domain-pascal-g5q.8): which list-scheduling rule could have produced the
order of the instructions inside the image's basic blocks?

Routines, blocks and the register model come from tools/regalloc.py.  Each
block is cut into runs at barriers (calls, SR instructions, writes of
A5/A6, a call's pop) and its final branch is left out; the dependence
graph of a run is built as in crates/dpc-m68k/src/sched.rs, with a choice
of memory model:

    alias   a store is ordered against the loads and stores it may alias
            (distinct (d,A6) slots, (d,A5) offsets and absolute addresses
            do not alias; anything through A0-A4 aliases all memory)
    nodis   no disambiguation: every load and store after a store, every
            store after a load
    stores  a store (other than a push) is ordered against every
            instruction (dpc's model)

In all three, pushes are ordered against every memory access, the flags
reader after its setter (and the run's last setter last), and a call's
pad after an earlier store.

A *list rule* places one instruction at a time, from the top (`td') or
from the bottom (`bu'), and avoids putting next to the instruction just
placed one that "interlocks" with it:

    raw      reads a register (A7 included) the other writes
    noa7     the same without A7
    addr     reads, to form an address, a register the other writes
    mem      both access memory
    rawmem   raw or mem

Whatever the order in which the compiler generated the instructions, such
a rule never leaves an interlocked pair adjacent when another instruction
was ready and would not have interlocked.  `summary' counts, for every
adjacent interlocked pair of the image, whether some other instruction was
ready there (`avoid': the rule is contradicted, whatever the generation
order) or not (`forced').  A window on the candidates (section 12.4 R11)
weakens this test, since what is in the window depends on the generation
order, which the image does not show.

usage:
    tools/schedsurvey.py summary [--min N] [--max N]
    tools/schedsurvey.py examples MODEL KIND td|bu [N] [--min N] [--max N]

--min/--max bound the routine size in bytes (default 0x30 and 0x150, the
range of regalloc.py's survey).

Python 3 standard library + m68k-elf-objdump.  Read-only; writes only a
temporary file under tmp/ (through loopalign.disasm).
"""

import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import regalloc as ra  # noqa: E402

SIZE = {'b': 1, 'w': 2, 'l': 4}
MODELS = ('alias', 'nodis', 'stores')
KINDS = ('raw', 'noa7', 'addr', 'mem', 'rawmem')
_SCC = re.compile(r's(?:t|f|hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le)$')


def loc(op, size):
    """The memory an operand addresses: (class, lo, hi), or None for a
    register or an immediate.  Classes: f frame, 5 module data, abs
    absolute, sp stack, pc code constant, p through A0-A4."""
    if op.startswith('#') or ra.is_reg(op) or op in ('%sr', '%ccr', '%usp'):
        return None
    for cls, reg in (('f', 'fp'), ('5', 'a5')):
        m = re.fullmatch(r'%' + reg + r'@(?:\((-?\d+)\))?', op)
        if m:
            d = int(m.group(1) or 0)
            return (cls, d, d + size)
        if op.startswith('%' + reg + '@'):
            return (cls, None, None)
    m = re.fullmatch(r'0x([0-9a-f]+)', op)
    if m:
        d = int(m.group(1), 16)
        return ('abs', d, d + size)
    if op.startswith('%sp@'):
        return ('sp', None, None)
    if op.startswith('%pc@'):
        return ('pc', None, None)
    return ('p', None, None)


def alias(a, b):
    """Whether accesses a and b may overlap (pushes alias everything)."""
    if 'pc' in (a[0], b[0]):
        return False
    if 'sp' in (a[0], b[0]) or 'p' in (a[0], b[0]):
        return True
    if a[0] != b[0]:
        return False
    if a[1] is None or b[1] is None:
        return True
    return a[1] < b[2] and b[1] < a[2]


class Info:
    """What one instruction does, for the dependence graph."""

    def __init__(self, x):
        self.x = x
        mn = x.mn
        sz = SIZE.get(mn[-1], 4)
        base = mn[:-1] if mn[-1] in 'bwl' else mn
        self.base = base
        self.r = set(x.uses) | ra.ea_regs(' '.join(x.ops))
        self.w = set(x.defs)
        self.loads, self.stores = [], []
        self.barrier = bool(x.call) or mn in ('unlk', 'moveml', 'linkw', 'trap') or any(
            o in ('%sr', '%ccr') for o in x.ops)
        if x.ops and x.ops[-1] in ('%a5', '%fp') and base in ('lea', 'movea', 'move'):
            self.barrier = True
        self.term = bool(x.kind)
        self.pad = x.text == 'subql #2,%sp'
        self.pop = base in ('addq', 'lea', 'adda') and bool(x.ops) and x.ops[-1] == '%sp' \
            and not self.pad
        self.push = (bool(x.ops) and x.ops[-1] == '%sp@-') or base == 'pea'
        if self.pad or self.pop or self.push:
            self.r.add('a7')
            self.w.add('a7')
        if not x.kind and not x.call:
            ops = x.ops
            if base == 'pea':
                self.stores.append(('sp', None, None))
            elif base in ('cmp', 'cmpa', 'cmpi', 'tst', 'btst', 'chk', 'cmpm'):
                self.loads += [lc for lc in (loc(o, sz) for o in ops) if lc]
            elif base == 'lea':
                pass
            elif base in ('move', 'movea', 'clr', 'moveq') or _SCC.match(mn):
                self.loads += [lc for lc in (loc(o, sz) for o in ops[:-1]) if lc]
                if ops and loc(ops[-1], sz):
                    self.stores.append(loc(ops[-1], sz))
            else:
                self.loads += [lc for lc in (loc(o, sz) for o in ops) if lc]
                if ops and loc(ops[-1], sz):
                    self.stores.append(loc(ops[-1], sz))
        self.mem = bool(self.loads or self.stores)
        self.real_store = any(s[0] != 'sp' for s in self.stores)
        self.cc_w = ra.sets_cc(x) and not (self.pad or self.pop)
        self.cc_r = x.kind == 'bcc' or bool(_SCC.match(mn)) or base in ('addx', 'subx', 'negx')


def depends(y, x, model):
    """Whether x (later) depends on y (earlier), flags aside."""
    if (y.w & x.r) or (y.r & x.w) or (y.w & x.w):
        return True
    if x.barrier or y.barrier or x.term:
        return True
    if model == 'stores' and (y.real_store or x.real_store):
        return True
    if model == 'nodis' and ((y.stores and x.mem) or (x.stores and y.mem)):
        return True
    if any(alias(a, b) for a in x.stores for b in y.loads + y.stores) or any(
            alias(a, b) for a in y.stores for b in x.loads):
        return True
    return x.pad and y.real_store


def deps(run, model):
    """Predecessor sets of a run, flags included."""
    n = len(run)
    pred = [set() for _ in range(n)]
    for k in range(n):
        for j in range(k):
            if depends(run[j], run[k], model):
                pred[k].add(j)
        if run[k].cc_r:
            for j in reversed(range(k)):
                if run[j].cc_w:
                    pred[k].add(j)
                    pred[j] |= {i for i in range(j) if run[i].cc_w}
                    break
        if run[k].cc_w:
            pred[k] |= {j for j in range(k) if run[j].cc_r}
    setters = [i for i, x in enumerate(run) if x.cc_w]
    if setters:
        pred[setters[-1]] |= set(setters[:-1])
    return pred


def interlock(y, x, kind):
    """x placed right after y: does it interlock?"""
    if kind == 'raw':
        return bool(y.w & x.r & (ra.ALLOC | {'a7'}))
    if kind == 'noa7':
        return bool(y.w & x.r & ra.ALLOC)
    if kind == 'addr':
        return bool(y.w & ra.ea_regs(' '.join(o for o in x.x.ops if '@' in o)))
    if kind == 'mem':
        return y.mem and x.mem
    if kind == 'rawmem':
        return bool(y.w & x.r & ra.ALLOC) or (y.mem and x.mem)
    raise ValueError(kind)


def runs(rt):
    """The runs of each block: barrier-free, without the final branch, a
    call's pop starting a new one."""
    for blk in ra.blocks(rt):
        infos = [Info(rt.ins[i]) for i in blk]
        while infos and infos[-1].term:
            infos.pop()
        cur = []
        for k, x in enumerate(infos):
            if x.barrier or (x.pop and k > 0 and infos[k - 1].x.call):
                if cur:
                    yield cur
                cur = []
            else:
                cur.append(x)
        if cur:
            yield cur


def check(done, model, kind, bottom_up, show=0):
    """Counter of avoidable/forced interlocked adjacent pairs, examples."""
    c, ex = Counter(), []
    for seg, rt in done:
        for run in runs(rt):
            if len(run) < 3:
                continue
            pred = deps(run, model)
            n = len(run)
            succ = [set() for _ in range(n)]
            for j in range(n):
                for i in pred[j]:
                    succ[i].add(j)
            placed = set()
            steps = range(n) if not bottom_up else range(n - 1, -1, -1)
            for k in steps:
                if not bottom_up and k > 0 and interlock(run[k - 1], run[k], kind):
                    a, b = run[k - 1], run[k]
                    alt = [l for l in range(k + 1, n) if pred[l] <= placed
                           and not interlock(a, run[l], kind)]
                elif bottom_up and k < n - 1 and interlock(run[k], run[k + 1], kind):
                    a, b = run[k], run[k + 1]
                    alt = [l for l in range(k) if succ[l] <= placed
                           and not interlock(run[l], b, kind)]
                else:
                    placed.add(k)
                    continue
                c['avoid' if alt else 'forced'] += 1
                if alt and len(ex) < show:
                    ex.append('%06X %-32s %-30s other: %06X %s' % (
                        a.x.addr, a.x.text, b.x.text, run[alt[0]].x.addr, run[alt[0]].x.text))
                placed.add(k)
    return c, ex


def opts(argv):
    lo, hi, rest = 0x30, 0x150, []
    it = iter(argv)
    for a in it:
        if a == '--min':
            lo = int(next(it), 0)
        elif a == '--max':
            hi = int(next(it), 0)
        else:
            rest.append(a)
    return lo, hi, rest


def main(argv):
    lo, hi, argv = opts(argv)
    if not argv or argv[0] not in ('summary', 'examples'):
        print(__doc__)
        return 2
    m, done, skipped = ra.load_all(lo, hi)
    if argv[0] == 'summary':
        print('routines %d (skipped %d)' % (len(done), sum(skipped.values())))
        print('%-7s %-7s %14s %14s' % ('model', 'kind', 'top-down', 'bottom-up'))
        for model in MODELS:
            for kind in KINDS:
                cells = []
                for bu in (False, True):
                    c, _ = check(done, model, kind, bu)
                    cells.append('%5d / %5d' % (c['avoid'], c['avoid'] + c['forced']))
                print('%-7s %-7s %14s %14s' % (model, kind, cells[0], cells[1]))
        print('(avoidable / all interlocked adjacent pairs)')
        return 0
    model, kind, d = argv[1], argv[2], argv[3]
    n = int(argv[4]) if len(argv) > 4 else 30
    c, ex = check(done, model, kind, d == 'bu', show=n)
    print('\n'.join(ex))
    print(dict(c))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
