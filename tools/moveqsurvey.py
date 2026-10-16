#!/usr/bin/env python3
"""moveqsurvey.py - small nonzero constants stored to memory in the
compiled code of the SR10.2 image: `move.<s> #n,<ea>' or `moveq #n,Dk'
and `move.<s> Dk,<ea>'.

Evidence tool for docs/codegen-patterns.md section 13 ("Stores of
constants"), bead domain-pascal-53xi.  Compiled routines are split and
followed as tools/nilsurvey.py does (tools/arithsurvey.py `routines',
tools/a5survey.py `reached'); the def/use model of an instruction is
tools/regalloc.py's `Insn'.  A `moveq #n,Dk' is followed along every path
from it until Dk is written again (a call clobbers D0/D1, an M$ARITH call
D0 only; an `rts' ends a path, flagged `rts' for D0, which may be a
function result; a `case' `jmp' ends it flagged `jmp').  Every
instruction on the way that reads Dk is a *use*: a store (`move.<s>
Dk,<mem>', not a push), a push (`move.<s> Dk,-(SP)'), or `other' (an
operand of arithmetic, a compare, a copy into a register, ...).  An
`add'/`sub' etc. into Dk itself ends the path.

Kinds (`list KIND'):

    imm     `move.<s> #n,<mem>', n in -128..127 and nonzero (and, for
            `summary', the counts of every value class); the D registers
            free at the store (not live after it), and its place in a
            run of constant stores (`run_position', below)
    moveq   each `moveq #n,Dk' whose value is stored to memory at least
            once: the stores (size, operand kind), other uses, the
            register and the lowest D register free at the `moveq'
    word    as `imm' and `moveq' for word stores (`move.w')
    byte    as `imm' and `moveq' for byte stores (`move.b')
    push    longword pushes of small nonzero constants: `pea (n).w',
            `move.l #n,-(SP)' and a `moveq' value pushed
    chain   each `moveq' that follows, in one straight-line run (no
            branch, branch target or call between), a `moveq' whose value
            only longword stores read: its register against the first's
            (a run of constant stores keeps its constants apart, and a
            constant stored again comes from the same register: those
            are `moveq' values with several stores)

`moveq' sites also say whether the path to the first store is
`straight' or crosses a branch target (`join': arms that share the store,
tail merging), and where each store stands in a run of constant stores
(`run_position': `lower', `upper', `desc', `same-*' for a neighbour
storing the same register, `alone').

Operand kinds are tools/nilsurvey.py's `ea_kind' (frame, unit (A5),
abs, areg (through an address register), postinc, indexed).

usage:
    tools/moveqsurvey.py summary       # the counts quoted in codegen-patterns
    tools/moveqsurvey.py list KIND     # one line per site of KIND

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import regalloc  # noqa: E402
from nilsurvey import ea_kind  # noqa: E402

KINDS = ('imm', 'moveq', 'word', 'byte', 'push', 'chain')
DREGS = ['d%d' % i for i in range(8)]

_MOVEQ = re.compile(r'^moveq #(-?\d+),%(d\d)$')
_IMM = re.compile(r'^move([bwl]) #(-?(?:0x)?[0-9a-f]+),(\S+)$')
_CONST = re.compile(r'^(?:clr([bwl])|move([bwl]) #-?(?:0x)?[0-9a-f]+,|st) ?(\S+)$')
_BASE = re.compile(r'^(%(?:a\d|fp)@|0x)(?:\((-?\d+)\))?([0-9a-f]*)$')
SIZES = {'b': 1, 'w': 2, 'l': 4}


def small(v):
    return -128 <= v <= 127 and v != 0


def vclass(v):
    return '0' if v == 0 else ('small' if -128 <= v <= 127 else 'large')


def signed(v, size):
    """V (an immediate as objdump prints it) as a signed SIZE value."""
    bits = 8 * SIZES[size]
    v &= (1 << bits) - 1
    return v - (1 << bits) if v >> (bits - 1) else v


def is_mem(op):
    return not re.fullmatch(r'%(?:[ad]\d|sp|fp)', op)


class Flow:
    """The instructions of one routine (a5survey.reached) with the
    def/use model of regalloc.Insn and fall-through/branch successors."""

    def __init__(self, ins):
        self.x = [regalloc.Insn(a, n, t) for a, n, t in ins]
        self.at = {x.addr: i for i, x in enumerate(self.x)}

    def succ(self, i):
        x = self.x[i]
        if x.kind in ('ret', 'jmp') or x.bad:
            return []
        out = []
        if x.target is not None and not x.call and x.target in self.at:
            out.append(self.at[x.target])
        if (x.kind != 'br' and i + 1 < len(self.x)
                and self.x[i + 1].addr == x.addr + x.n):
            out.append(i + 1)
        return out

    def clobbers(self, x):
        if not x.call:
            return set()
        t = x.target
        if t is None and x.ops and re.fullmatch(r'0x[0-9a-f]+', x.ops[0]):
            t = int(x.ops[0], 16)
        if t is not None and regalloc.MARITH[0] <= t < regalloc.MARITH[1]:
            return {'d0', 'a0'}
        return regalloc.SCRATCH

    def uses(self, i, reg):
        """([index of each instruction reading REG's value defined at
        ins[i]], flags): see the module comment."""
        seen, work, res, flags = set(), list(self.succ(i)), [], set()
        while work:
            j = work.pop()
            if j in seen:
                continue
            seen.add(j)
            x = self.x[j]
            if x.bad:
                flags.add('unknown')
                continue
            if reg in x.uses and x.kind != 'ret':
                res.append(j)
                if reg in x.defs:
                    continue
            elif reg in x.defs:
                continue
            if reg in self.clobbers(x):
                continue
            if x.kind == 'ret':
                if reg == 'd0':
                    flags.add('rts')
                continue
            if x.kind == 'jmp':
                flags.add('jmp')
                continue
            work.extend(self.succ(j))
        return sorted(res), flags

    def live_after(self, i, reg):
        """Whether REG's value after ins[i] is read (an `rts' does not
        count: whether D0 is a function's result is not modelled)."""
        u, flags = self.uses(i, reg)
        return bool(u) or 'jmp' in flags or 'unknown' in flags

    def joined(self, i, j):
        """`straight' when ins[i+1..j] is a fall-through run with no branch
        target in it (ins[j] reached from ins[i] only), else `join' (the
        store shared by several paths: tail merging, codegen-patterns 7)."""
        if j <= i:
            return 'join'               # reached by a branch back
        targets = self.targets()
        for k in range(i + 1, j + 1):
            x = self.x[k]
            if x.addr in targets or self.x[k - 1].kind or (
                    self.x[k - 1].addr + self.x[k - 1].n != x.addr):
                return 'join'
        return 'straight'

    def targets(self):
        if not hasattr(self, '_targets'):
            self._targets = {x.target for x in self.x if x.target is not None and not x.call}
        return self._targets

    def free(self, i, but=()):
        """The D registers not live after ins[i] (REG of `but' counted free)."""
        return [r for r in DREGS if r in but or not self.live_after(i, r)]


def use_kind(x, reg):
    """`store.<s> <kind>', `push.<s>' or `other <mnemonic>' for a use of REG."""
    m = re.fullmatch(r'move([bwl])', x.mn)
    if m and len(x.ops) == 2 and is_mem(x.ops[1]):
        if reg in regalloc.ea_regs(x.ops[1]):
            return 'other index'
        if x.ops[0] == '%' + reg:
            if x.ops[1] == '%sp@-':
                return 'push.' + m.group(1)
            return 'store.%s %s' % (m.group(1), ea_kind(x.ops[1]))
    return 'other ' + x.mn


def base_off(op):
    """(base, offset) of a memory operand, or None (indexed and the rest)."""
    m = _BASE.fullmatch(op)
    if not m:
        return None
    if m.group(1) == '0x':
        return ('abs', int('0x' + m.group(3), 16))
    return (m.group(1), int(m.group(2) or 0))


def const_store(x):
    """(size, operand) of a constant store (`clr', `move #', `st'), or None."""
    m = _CONST.fullmatch(x.text)
    if not m or not is_mem(m.group(3)) or m.group(3) == '%sp@-':
        return None
    return (m.group(1) or m.group(2) or 'b'), m.group(3)


_STORE = re.compile(r'^(?:clr([bwl])|move([bwl]) (\S+),|st) ?(\S+)$')


def any_store(x):
    """(size, source or None, operand) of a store to memory other than a
    push (`clr', `move', `st'), or None."""
    m = _STORE.fullmatch(x.text)
    if not m or not is_mem(m.group(4)) or m.group(4) == '%sp@-':
        return None
    return (m.group(1) or m.group(2) or 'b'), m.group(3), m.group(4)


def store_value(fl, j):
    """What the store ins[j] stores, when it is a constant: `#n' (`move
    #n'), `0' (`clr'), `st', or `moveq #n,Dk@ADDR' for a register a
    `moveq' loaded in the straight-line code before it (up to 6
    instructions back); None for any other value."""
    st = any_store(fl.x[j])
    if st is None:
        return None
    if st[1] is None:
        return 'st' if fl.x[j].mn == 'st' else '0'
    if st[1].startswith('#'):
        return st[1]
    m = re.fullmatch(r'%(d\d)', st[1])
    if not m:
        return None
    targets = fl.targets()
    for k in range(j - 1, max(-1, j - 7), -1):
        y = fl.x[k]
        if y.kind or y.call or fl.x[k + 1].addr in targets:
            return None
        q = _MOVEQ.match(y.text)
        if q and q.group(2) == m.group(1):
            return 'moveq #%s,%s@%X' % (q.group(1), q.group(2), y.addr)
        if m.group(1) in y.defs:
            return None
    return None


def neighbour_store(fl, i, step):
    """The index of the store next to ins[i] in the straight-line code
    (STEP -1 before it, +1 after it), skipping up to 4 instructions that
    store nothing (a `moveq', an address register's set-up), or None."""
    targets = fl.targets()
    j = i
    for _ in range(5):
        if step < 0:
            if j == 0 or fl.x[j].addr in targets or fl.x[j - 1].kind or fl.x[j - 1].call:
                return None
            j -= 1
        else:
            if fl.x[j].kind or fl.x[j].call or j + 1 >= len(fl.x):
                return None
            j += 1
            if fl.x[j].addr in targets:
                return None
        if any_store(fl.x[j]) is not None:
            return j
        if fl.x[j].text.split(None, 1)[0] in ('movel', 'movew', 'moveb'):
            return None             # a load into a register: another statement
    return None


def run_position(fl, i):
    """Where the store ins[i] stands in a run of constant stores off one
    base (each store's value a constant, [`store_value']; the stores one
    after the other in the straight-line code): `lower' when the next
    store stores a constant to the bytes just above it, `upper' when the
    previous one stores a constant just below it, `desc' for descending
    neighbours, `alone' otherwise; `same-' when the neighbour stores the
    same `moveq' register's value."""
    me = any_store(fl.x[i])
    b = me and base_off(me[2])
    v = store_value(fl, i)
    if not b or v is None:
        return 'alone'
    lo, hi = b[1], b[1] + SIZES[me[0]]
    pos = []
    for step, where in ((1, 'next'), (-1, 'prev')):
        j = neighbour_store(fl, i, step)
        if j is None:
            continue
        o = any_store(fl.x[j])
        ob = base_off(o[2])
        w = store_value(fl, j)
        if not ob or ob[0] != b[0] or w is None:
            continue
        same = 'same-' if w == v and v.startswith('moveq') else ''
        if where == 'next' and ob[1] == hi:
            pos.append(same + 'lower')
        elif where == 'prev' and ob[1] + SIZES[o[0]] == lo:
            pos.append(same + 'upper')
        elif ob[1] == hi or ob[1] + SIZES[o[0]] == lo:
            pos.append(same + 'desc')
    return '+'.join(pos) or 'alone'


def imm_sites(fl, size):
    """[(addr, value, kind, run position, free D registers)] of
    `move.<size> #n,<mem>' stores (pushes left out), every value."""
    out = []
    for i, x in enumerate(fl.x):
        m = _IMM.match(x.text)
        if not m or m.group(1) != size or not is_mem(m.group(3)) or m.group(3) == '%sp@-':
            continue
        v = signed(int(m.group(2), 0), size)
        free = fl.free(i) if small(v) else []
        out.append((x.addr, v, ea_kind(m.group(3)), run_position(fl, i), free))
    return out


def moveq_sites(fl):
    """[(addr, value, reg, [use kinds], flags, lowest free D)] of every
    `moveq' whose value is read at least once."""
    out = []
    for i, x in enumerate(fl.x):
        m = _MOVEQ.match(x.text)
        if not m:
            continue
        reg = m.group(2)
        us, flags = fl.uses(i, reg)
        if not us:
            continue
        free = fl.free(i, but=(reg,))
        runs = [run_position(fl, j) for j in us if use_kind(fl.x[j], reg).startswith('store.')]
        out.append((x.addr, int(m.group(1)), reg, [use_kind(fl.x[j], reg) for j in us],
                    flags, free[0] if free else '-', fl.joined(i, us[0]),
                    '/'.join(runs) or '-'))
    return out


def chain_pairs(fl):
    """[(addr, reg, value, uses, prev addr, prev reg, prev value)]: each
    `moveq' after a `moveq' whose value only longword stores read, in one
    straight-line run (no branch, branch target or call between them):
    the register the second takes against the first's."""
    out = []
    prev = None
    targets = fl.targets()
    for i, x in enumerate(fl.x):
        if x.addr in targets:
            prev = None
        if x.kind or x.call:
            prev = None
            continue
        m = _MOVEQ.match(x.text)
        if not m:
            continue
        reg = m.group(2)
        us, _flags = fl.uses(i, reg)
        kinds = [use_kind(fl.x[j], reg) for j in us]
        if prev:
            out.append((x.addr, reg, int(m.group(1)), kinds) + prev)
        if kinds and all(k.startswith('store.l') for k in kinds):
            prev = (x.addr, reg, int(m.group(1)))
        else:
            prev = None
    return out


def push_sites(fl):
    """[(addr, form, value)] of the small nonzero longword pushes."""
    out = []
    for i, x in enumerate(fl.x):
        t = x.text
        m = re.fullmatch(r'pea (-?(?:0x)?[0-9a-f]+)', t)
        if m:
            v = signed(int(m.group(1), 0), 'w')
            if small(v):
                out.append((x.addr, 'pea (n).w', v))
            continue
        m = re.fullmatch(r'movel #(-?(?:0x)?[0-9a-f]+),%sp@-', t)
        if m:
            v = signed(int(m.group(1), 0), 'l')
            if small(v):
                out.append((x.addr, 'move.l #n', v))
            continue
    for a, v, reg, uses, *_rest in moveq_sites(fl):
        if 'push.l' in uses and small(v):
            out.append((a, 'moveq; move.l Dk' + (' (+other uses)' if set(uses) != {'push.l'}
                                                 else ''), v))
    return out


def scan():
    import a5survey  # noqa: E402
    import arithsurvey  # noqa: E402
    res = {k: [] for k in KINDS}
    for r0, r1, rname in arithsurvey.routines():
        fl = Flow(a5survey.reached(r0, r1))
        res['imm'] += [(r0, rname) + s for s in imm_sites(fl, 'l')]
        res['word'] += [(r0, rname) + s for s in imm_sites(fl, 'w')]
        res['byte'] += [(r0, rname) + s for s in imm_sites(fl, 'b')]
        res['moveq'] += [(r0, rname) + s for s in moveq_sites(fl)]
        res['push'] += [(r0, rname) + s for s in push_sites(fl)]
        res['chain'] += [(r0, rname) + s for s in chain_pairs(fl)]
    return res


def stores_of(s, size):
    return [u for u in s[5] if u.startswith('store.' + size)]


def first(sites, n=4):
    return ', '.join('0x%X' % s[2] for s in sites[:n])


def counts(title, sites, key):
    print('%s: %d' % (title, len(sites)))
    for k, v in sorted(Counter(key(s) for s in sites).items(), key=lambda kv: (-kv[1], kv[0])):
        print('  %-52s %4d  e.g. %s' % (' '.join(str(x) for x in k), v,
                                        first([s for s in sites if key(s) == k])))


def moveq_class(s, size):
    """`only' (every use a SIZE store), `+push', `+other', `mixed sizes'."""
    us = s[5]
    if all(u.startswith('store.' + size) for u in us):
        return 'stores only'
    if any(u.startswith('store.') and not u.startswith('store.' + size) for u in us):
        return 'other sizes too'
    if any(u.startswith('push') for u in us) and all(
            u.startswith('store.') or u.startswith('push') for u in us):
        return 'pushed too'
    return 'other uses too'


def summary_size(res, size, title):
    imm = res[{'l': 'imm', 'w': 'word', 'b': 'byte'}[size]]
    counts('%s stores `move.%s #n,<mem>\' by value class and operand' % (title, size),
           imm, lambda s: (vclass(s[3]), s[4]))
    sm = [s for s in imm if small(s[3])]
    counts('  the small nonzero ones (-128..127) by run position, D registers free',
           sm, lambda s: (s[5], 'd0 free' if 'd0' in s[6] else 'd0 live',
                          'some D free' if s[6] else 'no D free'))
    mq = [s for s in res['moveq'] if stores_of(s, size) and small(s[3])]
    counts('%s stores from `moveq #n,Dk\' values (the values stored, by their uses, '
           'straight or at a join)' % title, mq,
           lambda s: (moveq_class(s, size), '%d store(s)' % min(len(stores_of(s, size)), 3),
                      s[8]))
    st = [s for s in mq if s[8] == 'straight' and moveq_class(s, size) == 'stores only']
    counts('  straight, stores only: the stores by operand',
           [s + (u,) for s in st for u in stores_of(s, size)], lambda s: (s[-1],))
    counts('  straight, stores only: run positions of the stores', st, lambda s: (s[9],))
    counts('  straight, stores only: register chosen against the lowest D free at '
           'the moveq', st, lambda s: (s[4], 'lowest' if s[4] == s[7] else
                                       'lowest free %s' % s[7]))


def summary(res):
    summary_size(res, 'l', 'longword')
    print()
    counts('a moveq after a moveq whose value only longword stores read, in one '
           'straight-line run (the second\'s uses, its register)', res['chain'],
           lambda s: ('stores only' if s[5] and all(u.startswith('store.l') for u in s[5])
                      else 'other uses', 'same register' if s[3] == s[7] else 'another register',
                      'same value' if s[4] == s[8] else 'another value'))
    print()
    summary_size(res, 'w', 'word')
    print()
    summary_size(res, 'b', 'byte')
    print()
    counts('longword pushes of a small nonzero constant (form)', res['push'],
           lambda s: (s[3],))


def main(argv):
    if not argv or argv[0] not in ('summary', 'list') or (
            argv[0] == 'list' and (len(argv) < 2 or argv[1] not in KINDS)):
        print(__doc__)
        return 2
    res = scan()
    if argv[0] == 'summary':
        summary(res)
        return 0
    k = argv[1]
    for s in res[k]:
        if k in ('imm', 'word', 'byte') and not small(s[3]):
            continue
        if k == 'moveq' and not any(u.startswith('store.') for u in s[5]):
            continue
        rest = list(s[3:])
        if k == 'chain':
            rest[3] = 'after 0x%X' % rest[3]
        print('0x%06X %-28s %s' % (s[2], '%06X %s' % (s[0], s[1]),
                                  ' '.join(str(x) for x in rest)))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
