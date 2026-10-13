#!/usr/bin/env python3
"""csesurvey.py - survey common-subexpression reuse in the compiled code of
the SR10.2 image.

Evidence tool for docs/codegen-patterns.md section 12.4 R7/R8 (bead
domain-pascal-g5q.7): which computed values did Apollo's compiler keep in
a register and reuse later, which did it compute again, and what lay
between?

Routines are found and modelled as in tools/regalloc.py (control flow,
calls, hand assembly skipped).  Each routine is then evaluated
symbolically, forward over its control-flow graph: every register holds
an *expression* over memory reads (`m'), constants (`k'), the values of
calls and of instructions the model does not know (`u', unique) and the
operators applied to them.  At a join, a register keeps its expression
only when every predecessor agrees.  Memory operands are canonical: the
address registers in them are replaced by the expressions they hold, so
`(4,A0)' after `movea.l (8,A6),A0' is `(4, m(8,A6))'.

A *computed* expression is one whose top is an operator (a zero- or
sign-extension, a shift, an add, a product, an `and', a `lea' of
something other than a constant or a frame/A5 slot).  Plain memory reads
and constants are not computed (R8 is about them; `reads' counts them).

Each register also carries the *events* that happened since it got its
expression, on any path (a union at joins):

    st    a store to memory (not a push), to a location that is not an
          operand of the expression
    kill  a store to a location the expression reads (exact match), or
          to its base through a pointer
    ptr   a store through a pointer (an address register other than A5,
          A6, A7), which may alias the expression's memory operands
    call  a `jsr'/`bsr' (M$ARITH entries: `marith')
    br    a conditional branch passed (the value lives on into one arm)
    join  a join (a block with several predecessors) passed
    back  a loop back-edge passed (the value is defined before a loop
          and used inside it)
    sto   the register's value was itself stored to memory: a later read
          of the register may stand for a read of that variable
          (codegen-patterns 12.5, stored locals), not a CSE

Two kinds of event are counted, by the kind of the expression:

    reuse     an instruction reads a register holding a computed
              expression, and some event other than `br' happened since
              the register got it (the use is in a later statement, or
              across a join); the events are reported
    recompute an instruction gives a register a computed expression that
              another register (or the same one) already held before it,
              with the events since
    lost      an instruction computes an expression that was computed on
              every path to it but is no longer in any register (its
              register was given another value), with the events since
              the earlier computation; not counted when the earlier value
              is part of an expression a register still holds (a step of
              a longer chain, the `lsl' of a ×12)

usage:
    tools/csesurvey.py summary [--min N] [--max N]
                                   # counts of reuse/recompute by kind and
                                   # by event set
    tools/csesurvey.py list reuse|recompute|lost [KIND] [--min N] [--max N]
                                   # the sites (KIND as in summary)
    tools/csesurvey.py routine ADDR
                                   # one routine: every instruction with
                                   # the computed expressions it reads/
                                   # gives
    tools/csesurvey.py agg [--min N] [--max N]
                                   # the same, by the strongest event
    tools/csesurvey.py corpus      # reuse/recompute in regalloc's corpus

Model limits: sizes are kept on operators but a word write into a
register that held a long constant 0 (`clr.l Dn; move.w x,Dn') is a
zero-extension, and other partial writes give an unknown.  A `jsr'/`bsr'
gives D0/D1/A0/A1 unknowns (M$ARITH only D0/A0).  Python 3 standard
library + m68k-elf-objdump.  Read-only; writes only a temporary file
under tmp/ (through loopalign.disasm).
"""

import os
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import regalloc  # noqa: E402
import sau2map  # noqa: E402
from regalloc import Unknown, is_reg  # noqa: E402

REGS = ['d%d' % i for i in range(8)] + ['a%d' % i for i in range(5)]
FIXED = {'a5': 'A5', 'a6': 'FP'}
_MEM = re.compile(r'^%(a[0-7]|fp|sp|pc)@(?:\(([^,()]*)(?:,%(d[0-7]|a[0-7]):([wl]))?\))?([+-])?$')
_ABS = re.compile(r'^0x[0-9a-f]+$')


def unlo(e):
    while isinstance(e, tuple) and e[0] == 'lo':
        e = e[2]
    return e


def computed(e):
    e = unlo(e)
    return isinstance(e, tuple) and e[0] in ('op', 'zx', 'sx', 'lea')


def kind(e):
    e = unlo(e)
    if e[0] in ('zx', 'sx'):
        return e[0]
    if e[0] == 'lea':
        return 'lea'
    op = e[1]
    if op in ('lsl', 'asl'):
        return 'shl'
    if op in ('add', 'sub'):
        # a product as shifts and adds (×12: `lsl; move; add; add')
        if any(isinstance(unlo(x), tuple) and unlo(x)[0] == 'op' and unlo(x)[1] == 'shl'
               for x in e[3:]):
            return 'mulsh'
        return op
    return op


def fixed_base(loc):
    """A location off the frame, A5, an absolute address or a register
    holding a constant address (an array of the unit's data or of another
    module's)."""
    return loc.startswith(('FP', 'A5', 'abs', 'PC', "('k'"))


def split_loc(loc):
    """(base, indexed) of a canonical location."""
    k = loc.rfind('+', 0, loc.find('[') if '[' in loc else len(loc))
    return loc[:k], '[' in loc


def may_alias(a, b):
    if a == b:
        return True
    (ba, ia), (bb, ib) = split_loc(a), split_loc(b)
    return ba == bb and (ia or ib)


def contains(e, sub):
    """Whether sub is a proper part of e (a step of a longer chain)."""
    if not isinstance(e, tuple):
        return False
    for x in e[1:]:
        if isinstance(x, tuple) and (unlo(x) == sub or contains(x, sub)):
            return True
    return False


def leaves(e, out=None):
    """The memory locations an expression reads (canonical strings), and
    'ptr' when one of them is addressed through a computed pointer."""
    if out is None:
        out = set()
    if not isinstance(e, tuple):
        return out
    if e[0] == 'm':
        out.add(e[2])
        if not fixed_base(e[2]):
            out.add('ptr')
    for x in e[1:]:
        if isinstance(x, tuple):
            leaves(x, out)
    return out


class Sym:
    """Symbolic evaluation of one routine (a regalloc.Routine)."""

    def __init__(self, rt):
        self.rt = rt
        self.uniq = 0
        self.events = []        # (type, insn index, expr, events, other)
        self.record = False
        self.run()

    def fresh(self, i, r):
        self.uniq += 1
        return ('u', i, r, self.uniq)

    # -- operands ---------------------------------------------------------
    def loc(self, op, st):
        """Canonical memory location of an operand, or None for a push/pop."""
        if _ABS.match(op):
            return 'abs:' + op[2:]
        m = _MEM.match(op)
        if not m:
            raise Unknown('operand ' + op)
        base, disp, idx, isz, inc = m.groups()
        base = regalloc.canon(base) if base != 'pc' else 'pc'
        if base == 'a7':
            return None
        if base == 'pc':
            b = 'PC'
        elif base in FIXED:
            b = FIXED[base]
        else:
            b = repr(st.get(base, ('?', base)))
        s = b + '+' + (disp or '0')
        if idx:
            s += '[' + repr(st.get(idx, ('?', idx))) + ':' + isz + ']'
        return s

    def read(self, op, size, st):
        r = is_reg(op)
        if r:
            return self.reg(r, size, st)
        if op.startswith('#'):
            try:
                return ('k', int(op[1:], 0))
            except ValueError:
                return ('k', op[1:])
        loc = self.loc(op, st)
        if loc is None:
            return ('?', 'pop')
        return ('m', size, loc)

    @staticmethod
    def reg(r, size, st):
        """Register r read as `size': the low part of a partial write is
        the value written."""
        e = st.get(r, ('?', r))
        if isinstance(e, tuple) and e[0] == 'lo':
            if size in 'bw' and (size == e[1] or size == 'b'):
                return e[2]
            return ('?', r)
        return e

    # -- transfer ---------------------------------------------------------
    def step(self, i, st, ev):
        """Apply instruction i to state st (reg -> expr) and ev (reg ->
        frozenset of events); record reuse/recompute."""
        x = self.rt.ins[i]
        mn, ops = x.mn, x.ops
        size = mn[-1] if mn[-1] in 'bwl' else 'l'
        base = mn[:-1] if mn[-1] in 'bwl' and mn not in ('mulu', 'muls') else mn
        # register reads: reuse events
        for r in sorted(x.uses & set(REGS)):
            e = st.get(r)
            if e is not None and computed(e):
                es = ev.get(r, frozenset()) - {'br'}
                if es and self.record and not (x.kind == 'ret'):
                    self.events.append(('reuse', i, e, ev.get(r, frozenset()), r))
        out = {}            # reg -> new expr

        def setr(r, e):
            out[r] = e

        def binop(op, a, b, sz):
            return ('op', op, sz, a, b)
        if x.call:
            t = x.target if x.target is not None else None
            marith = False
            if t is None and ops and _ABS.match(ops[0]):
                t = int(ops[0], 16)
            if t is not None and regalloc.MARITH[0] <= t < regalloc.MARITH[1]:
                marith = True
            for r in (('d0', 'a0') if marith else ('d0', 'd1', 'a0', 'a1')):
                setr(r, self.fresh(i, r))
            tag = 'marith' if marith else 'call'
            for r in list(ev):
                ev[r] = ev[r] | {tag}
        elif x.kind in ('br', 'bcc', 'ret', 'jmp'):
            pass
        elif x.kind == 'db':
            r = is_reg(ops[0])
            setr(r, self.fresh(i, r))
        elif base in ('move', 'movea', 'moveq') or mn in ('moveq', 'moveql'):
            if ops[0] in ('%sr', '%ccr', '%usp'):
                r = is_reg(ops[1])
                if r:
                    setr(r, self.fresh(i, r))
                else:
                    self.store(i, ops[1], st, ev)
            elif ops[1] in ('%sr', '%ccr', '%usp'):
                pass
            else:
                sz = 'l' if mn.startswith('moveq') else size
                v = self.read(ops[0], sz, st)
                r = is_reg(ops[1])
                if r:
                    if base == 'movea' and sz == 'w':
                        v = ('sx', 'w', v)
                    if r[0] == 'd' and sz in 'bw':
                        old = st.get(r)
                        if old == ('k', 0):
                            v = ('zx', sz, v)
                        else:
                            v = ('lo', sz, v)
                    setr(r, v)
                else:
                    self.store(i, ops[1], st, ev)
                    q = is_reg(ops[0])
                    if q in REGS and self.loc(ops[1], st) is not None:
                        # the value now also lives in memory: a later read
                        # of the register may stand for a read of that
                        # variable (12.5, stored locals), not a CSE
                        ev[q] = ev.get(q, frozenset()) | {'sto'}
        elif base == 'clr':
            r = is_reg(ops[0])
            if r:
                setr(r, ('k', 0) if size == 'l' else self.fresh(i, r))
            else:
                self.store(i, ops[0], st, ev)
        elif base == 'lea':
            r = is_reg(ops[1])
            loc = self.loc(ops[0], st)
            if loc is None:
                setr(r, self.fresh(i, r))
            elif loc.startswith(('FP', 'A5', 'PC', 'abs')) and '[' not in loc:
                setr(r, ('k', 'addr:' + loc))
            else:
                setr(r, ('lea', loc))
        elif base == 'pea':
            pass
        elif base in ('add', 'adda', 'addi', 'addq', 'sub', 'suba', 'subi', 'subq',
                      'and', 'andi', 'or', 'ori', 'eor', 'eori', 'mulu', 'muls',
                      'divu', 'divs'):
            if ops[-1] in ('%sr', '%ccr'):
                pass
            else:
                op = base.rstrip('aiq')
                if op == '':
                    op = base
                sz = 'w' if base in ('mulu', 'muls', 'divu', 'divs') else size
                r = is_reg(ops[1])
                if r and base in ('sub', 'suba', 'eor') and ops[0] == ops[1]:
                    setr(r, ('k', 0))
                elif r:
                    rsz = 'w' if base in ('mulu', 'muls', 'divu', 'divs') else size
                    a = self.reg(r, rsz, st) if r[0] == 'd' else st.get(r, ('?', r))
                    b = self.read(ops[0], sz, st)
                    if op in ('add', 'and', 'or', 'eor', 'mulu', 'muls'):
                        a, b = sorted((a, b), key=repr)
                    v = binop(op, a, b, sz)
                    if r[0] == 'd' and size in 'bw' and base not in ('mulu', 'muls', 'divu',
                                                                       'divs'):
                        v = ('lo', size, v)
                    setr(r, v)
                else:
                    self.store(i, ops[1], st, ev)
        elif base in ('lsl', 'lsr', 'asl', 'asr', 'rol', 'ror', 'roxl', 'roxr'):
            if len(ops) == 1:
                self.store(i, ops[0], st, ev)
            else:
                r = is_reg(ops[1])
                op = 'shl' if base in ('lsl', 'asl') else base
                v = binop(op, self.reg(r, size, st), self.read(ops[0], 'w', st), size)
                setr(r, ('lo', size, v) if size in 'bw' else v)
        elif base in ('neg', 'not', 'negx'):
            r = is_reg(ops[0])
            if r:
                v = ('op', base, size, self.reg(r, size, st))
                setr(r, ('lo', size, v) if size in 'bw' else v)
            else:
                self.store(i, ops[0], st, ev)
        elif base == 'ext':
            r = is_reg(ops[0])
            fr = 'b' if size == 'w' else 'w'
            v = ('sx', fr, self.reg(r, fr, st))
            setr(r, ('lo', 'w', v) if size == 'w' else v)
        elif base in ('cmp', 'cmpa', 'cmpi', 'cmpm', 'tst', 'btst', 'chk'):
            pass
        elif mn.startswith('s') and re.match(regalloc._SCC.pattern, mn):
            r = is_reg(ops[0])
            if r:
                setr(r, self.fresh(i, r))
            else:
                self.store(i, ops[0], st, ev)
        else:
            for r in x.defs & set(REGS):
                setr(r, self.fresh(i, r))
            if ops and not is_reg(ops[-1]) and mn not in ('linkw', 'unlk', 'moveml', 'movemw',
                                                         'trap', 'nop'):
                if mn in ('bset', 'bclr', 'bchg', 'bsetb', 'bclrb', 'bchgb', 'tas', 'tasb'):
                    self.store(i, ops[-1], st, ev)
        # (An)+ / -(An): the register now holds an unknown
        for r in x.incdec & set(REGS):
            if r not in out:
                out[r] = self.fresh(i, r)
        # recompute: a computed expression given that some register held
        copy = base in ('move', 'movea') and len(ops) == 2 and is_reg(ops[0])
        for r, e in out.items():
            if computed(e) and not copy:
                held = [q for q in REGS if unlo(st.get(q)) == unlo(e)]
                if held and self.record:
                    q = held[0]
                    self.events.append(('recompute', i, e, ev.get(q, frozenset()), q))
                key = ('AV', unlo(e))
                if (not held and key in st and self.record
                        and not any(contains(st.get(q), key[1]) for q in REGS)):
                    # computed before on every path, no longer in a register
                    self.events.append(('lost', i, e, ev.get(key, frozenset()), r))
        for r, e in out.items():
            st[r] = e
            ev[r] = frozenset()
            if computed(e) and not copy:
                st[('AV', unlo(e))] = unlo(e)
                ev[('AV', unlo(e))] = frozenset()

    def store(self, i, op, st, ev):
        loc = self.loc(op, st)
        if loc is None:
            return            # a push
        through = not fixed_base(loc)
        for r, e in st.items():
            if not isinstance(e, tuple):
                continue
            lv = leaves(e)
            if any(may_alias(loc, v) for v in lv if v != 'ptr'):
                tag = 'kill'
            elif through:
                tag = 'ptr'
            else:
                tag = 'st'
            ev[r] = ev.get(r, frozenset()) | {tag}

    def run(self):
        rt = self.rt
        order = rt.reach
        sin = {}
        evin = {}
        start = order[0]
        sin[start] = {}
        evin[start] = {}
        work = [start]
        visits = Counter()
        while work:
            i = min(work)
            work.remove(i)
            visits[i] += 1
            if visits[i] > 50:
                raise Unknown('no fixpoint')
            st = dict(sin[i])
            ev = dict(evin[i])
            self.step(i, st, ev)
            x = rt.ins[i]
            succ = rt.succ[i]
            for s in succ:
                add = set()
                if x.kind == 'bcc' or x.kind == 'db':
                    add.add('br')
                if s <= i:
                    add.add('back')
                if len(rt.pred.get(s, [])) > 1:
                    add.add('join')
                nev = {r: v | add for r, v in ev.items()}
                if s not in sin:
                    sin[s], evin[s] = dict(st), nev
                    work.append(s)
                    continue
                ms = {r: e for r, e in sin[s].items() if st.get(r) == e}
                me = {r: evin[s].get(r, frozenset()) | nev.get(r, frozenset()) for r in ms}
                if ms != sin[s] or me != {r: evin[s].get(r, frozenset()) for r in ms}:
                    sin[s], evin[s] = ms, me
                    if s not in work:
                        work.append(s)
        # one final pass over the fixpoint states records the events
        self.events = []
        self.record = True
        for i in order:
            if i in sin:
                self.step(i, dict(sin[i]), dict(evin[i]))
        self.final = self.events


def evkey(es):
    s = sorted(es - {'br'})
    return '+'.join(s) if s else '-'


def operands(e):
    """F: the expression reads only the frame (locals, parameters); S:
    also the unit's data or absolute addresses (statics, other modules'
    variables); P: also memory through a pointer."""
    lv = leaves(e) - {'ptr'}
    if 'ptr' in leaves(e) or any(v.startswith("('k'") for v in lv):
        return 'P' if 'ptr' in leaves(e) else 'S'
    if any(v.startswith(('A5', 'abs')) for v in lv):
        return 'S'
    return 'F'


def survey(done, m):
    c = Counter()
    ex = defaultdict(list)
    for seg, rt in done:
        try:
            sy = Sym(rt)
        except Unknown:
            c['skipped'] += 1
            continue
        c['routines'] += 1
        for t, i, e, es, r in sy.final:
            k = (t, kind(e), evkey(es))
            c[k] += 1
            ex[k].append('%06X' % rt.ins[i].addr)
            k2 = ('opnd', t, operands(unlo(e)), evkey(es))
            c[k2] += 1
            ex[k2].append('%06X' % rt.ins[i].addr)
    return c, ex


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    lo, hi, _, argv = regalloc.opts(argv)
    cmd = argv[0]
    m = sau2map.load()
    if cmd == 'routine':
        r0 = int(argv[1], 16)
        rt = regalloc.one(m, r0)
        sy = Sym(rt)
        by = defaultdict(list)
        for t, i, e, es, r in sy.final:
            by[i].append('%s %s=%s since[%s]' % (t, r, short(e), ','.join(sorted(es))))
        print('%06X..%06X %s  C: %s' % (r0, rt.r1, m.describe(r0), regalloc.cfile(r0)))
        for i in rt.reach:
            x = rt.ins[i]
            print('  %06X %-34s %s' % (x.addr, x.text, ' | '.join(by.get(i, []))))
        return 0
    if cmd == 'corpus':
        done = []
        for r0 in regalloc.CORPUS:
            try:
                done.append((None, regalloc.one(m, r0)))
            except Unknown:
                pass
        c, ex = survey(done, m)
        report(c, ex, 99)
        return 0
    _, done, skipped = regalloc.load_all(lo, hi)
    c, ex = survey(done, m)
    if cmd == 'summary':
        print('routines analysed %d (model skipped %s; symbolic skipped %d)' % (
            c['routines'], dict(skipped), c['skipped']))
        report(c, ex, 6)
        return 0
    if cmd == 'agg':
        agg(c)
        return 0
    if cmd == 'list':
        t = argv[1]
        kf = argv[2] if len(argv) > 2 else None
        for k in sorted(ex):
            if k[0] == t and (kf is None or k[1] == kf):
                print('%-9s %-6s %-22s %4d  %s' % (k + (c[k], ' '.join(ex[k]))))
        return 0
    print(__doc__)
    return 2


def agg(c):
    """reuse/lost/recompute by kind and by the strongest event between:
    sto (the register's value was stored: forwarding of a variable, not
    a CSE) > call (or marith) > kill > back > ptr > join > st > none."""
    order = ('sto', 'call', 'kill', 'back', 'ptr', 'join', 'st', '-')
    t = Counter()
    for k, n in c.items():
        if not isinstance(k, tuple) or k[0] == 'opnd':
            continue
        es = set(k[2].split('+'))
        if 'marith' in es:
            es.add('call')
        top = next(o for o in order if o in es or o == '-')
        t[k[0], k[1], top] += n
    kinds = sorted({k[1] for k in t})
    print('%-6s %-9s ' % ('kind', 'event') + ' '.join('%9s' % x for x in
                                                     ('reuse', 'recompute', 'lost')))
    for kd in kinds:
        for o in order:
            row = [t[x, kd, o] for x in ('reuse', 'recompute', 'lost')]
            if any(row):
                print('%-6s %-9s ' % (kd, o) + ' '.join('%9d' % x for x in row))
    # by the operands' kind (F frame, S statics, P through a pointer),
    # without calls, for the events that may alias: a store through a
    # pointer (ptr) or to an operand (kill)
    print()
    print('operands, no call between: reuse/lost')
    u = Counter()
    for k, n in c.items():
        if not (isinstance(k, tuple) and k[0] == 'opnd'):
            continue
        es = set(k[3].split('+'))
        if es & {'call', 'marith', 'sto'}:
            continue
        ev = 'kill' if 'kill' in es else 'ptr' if 'ptr' in es else 'other'
        u[k[1], k[2], ev] += n
    for o in 'FSP':
        for ev in ('kill', 'ptr', 'other'):
            print('  %s %-6s reuse %4d  lost %4d  recompute %4d' % (
                o, ev, u['reuse', o, ev], u['lost', o, ev], u['recompute', o, ev]))


def short(e):
    s = repr(e)
    return s if len(s) < 90 else s[:87] + '...'


def report(c, ex, n):
    for t in ('reuse', 'recompute', 'lost'):
        print(t + ':')
        ks = sorted((k for k in c if isinstance(k, tuple) and k[0] == t),
                    key=lambda k: (k[1], -c[k]))
        for k in ks:
            print('  %-6s %-22s %5d  %s' % (k[1], k[2], c[k], ' '.join(ex[k][:n])))


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
