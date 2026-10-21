#!/usr/bin/env python3
"""elemholdsurvey.py - survey whether Apollo's compiler holds an array
element's address across a conditional test into the code after it.

Evidence tool for docs/codegen-patterns.md section 12.4 R7 (bead
domain-pascal-59st).  IO_$ENABLE_DEVICE 0xE1A876 computes `lea
(0,A5,D0.w),A2', tests `tst.l (0x70,A2)' and, in the then-part, calls
through `movea.l (0x70,A2),A0' (0xE1A884): the element's address is held
across the test.  Is that the rule, and what ends it?

Routines are found and modelled as in tools/csesurvey.py (regalloc's
routines, symbolic evaluation: every register holds an expression over
memory reads).  A *test site* is a conditional branch whose condition
codes come from a `tst'/`cmp'/`cmpi'/`btst' (in the same block) with a
memory operand that is an *element*: either indexed, `(d,An,Dx)' (form
`idx'), or through an address register holding an indexed `lea' (form
`lea').  The element is its base and index expression (any field).  The
code after the branch is split into regions:

    then   the fall-through, up to the branch target (a forward branch)
    else   from the target up to the target of a `bra' that ends `then'
           and jumps past it (if-then-else)
    next   the code after that (or after the target): the next statement
    loop   a backward branch: from the target (a loop's body)

Within each region (at most LIMIT instructions; it stops at a call only
for `next'), every use of the same element (same base and index
expression) is classified:

    held   through the register that held the element's address at the
           test, not written since (the address is held)
    lea    a new `lea' of the element (the address is computed again)
    idx    an indexed operand (the address is formed again; `ix=held' when
           the index register still holds its value from the test,
           `ix=new' when it was computed again)

and, for `held' and `lea', whether anything was stored (`st'), stored
through a pointer (`ptr'), or called (`call') between the test and the
use.  Sites where the element's register is written before any use are
`reg-lost' (the register was given something else first).

usage:
    tools/elemholdsurvey.py summary          # counts by test form, region,
                                              # use form
    tools/elemholdsurvey.py list [FORM REGION USE]
                                              # the sites
    tools/elemholdsurvey.py routine ADDR      # the sites of one routine
    tools/elemholdsurvey.py calls             # every `jsr (An)' whose
                                              # pointer is read through
                                              # an element: the `lea'
                                              # right before the load or
                                              # earlier

Model limits: those of csesurvey (sizes on operators, unknown partial
writes); routines with a `case' table are skipped.  Python 3 standard
library + m68k-elf-objdump.  Read-only; writes only a temporary file
under tmp/ (through loopalign.disasm).
"""

import os
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import csesurvey  # noqa: E402
import regalloc  # noqa: E402
import sau2map  # noqa: E402
from regalloc import Unknown, is_reg  # noqa: E402

LIMIT = 24
TESTS = ('tst', 'cmp', 'cmpi', 'cmpa', 'btst')


class Snap(csesurvey.Sym):
    """csesurvey's evaluation, keeping the state before each instruction."""

    def step(self, i, st, ev):
        if self.record:
            self.states[i] = dict(st)
        return super().step(i, st, ev)

    def run(self):
        self.states = {}
        super().run()


def parse_loc(loc):
    """(base, disp, index) of an indexed canonical location, else None."""
    if not loc.endswith(']'):
        return None
    depth, k = 0, len(loc) - 1
    while k >= 0:
        if loc[k] == ']':
            depth += 1
        elif loc[k] == '[':
            depth -= 1
            if depth == 0:
                break
        k -= 1
    if k < 0:
        return None
    head, idx = loc[:k], loc[k + 1:-1]
    p = head.rfind('+')
    if p < 0:
        return None
    try:
        # objdump prints a brief extension word's displacement in hex
        # without a prefix (`%a5@(1c,%d0:w)')
        disp = int(head[p + 1:], 16)
    except ValueError:
        return None
    return head[:p], disp, idx


def element(sy, op, st):
    """(key, form, reg) of a memory operand that is an element, else None.
    key = (base, index); form 'idx' or 'lea' (reg = the address register
    holding the lea, or the index register for 'idx')."""
    m = csesurvey._MEM.match(op)
    if not m:
        return None
    base, disp, idx, isz, inc = m.groups()
    if inc:
        return None
    if idx:
        loc = sy.loc(op, st)
        p = parse_loc(loc) if loc else None
        if p:
            return (p[0], p[2]), 'idx', regalloc.canon(idx)
        return None
    if base in ('a5', 'fp', 'sp', 'pc', 'a6', 'a7'):
        return None
    e = csesurvey.unlo(st.get(base))
    if isinstance(e, tuple) and e[0] == 'lea':
        p = parse_loc(e[1])
        if p:
            return (p[0], p[2]), 'lea', base
    return None


def test_operand(rt, sy, i):
    """The element a conditional branch at i tests: (key, form, reg, j)."""
    j = i - 1
    while j >= 0 and j in sy.states:
        x = rt.ins[j]
        if j != i - 1 and rt.pred.get(j + 1) and len(rt.pred[j + 1]) > 1:
            return None
        base = x.mn[:-1] if x.mn[-1] in 'bwl' else x.mn
        if base in TESTS:
            for op in x.ops:
                el = element(sy, op, sy.states[j])
                if el:
                    return el + (j,)
            return None
        if regalloc.sets_cc(x):
            return None
        j -= 1
    return None


def regions(rt, i):
    """[(name, [insn indices])] after the conditional branch at i."""
    x = rt.ins[i]
    t = rt.at.get(x.target)
    if t is None:
        return []
    out = []
    if t <= i:
        out.append(('loop', list(range(t, min(i, t + LIMIT)))))
        out.append(('next', list(range(i + 1, min(len(rt.ins), i + 1 + LIMIT)))))
        return out
    then = list(range(i + 1, t))
    out.append(('then', then[:LIMIT]))
    last = rt.ins[t - 1]
    u = rt.at.get(last.target) if last.kind == 'br' else None
    if u is not None and u > t:
        out.append(('else', list(range(t, u))[:LIMIT]))
        out.append(('next', list(range(u, min(len(rt.ins), u + LIMIT)))))
    else:
        out.append(('next', list(range(t, min(len(rt.ins), t + LIMIT)))))
    return out


def classify(rt, sy, key, form, reg, idxs, region):
    """The uses of element key in the instructions idxs -> [(use, events,
    addr)]."""
    uses = []
    written = set()
    events = set()
    for j in idxs:
        if j not in sy.states:
            break
        x = rt.ins[j]
        st = sy.states[j]
        for op in x.ops:
            el = element(sy, op, st)
            if not el or el[0] != key:
                continue
            if el[1] == 'lea':
                if el[2] == reg and form == 'lea' and reg not in written:
                    u = 'held'
                else:
                    u = 'via-new'
            elif x.mn == 'lea':
                u = 'lea'
            else:
                u = 'idx ix=%s' % ('new' if el[2] in written or el[2] != reg
                                   and form == 'idx' else 'held')
            uses.append((u, ','.join(sorted(events)), x.addr))
        if x.call:
            events.add('call')
            if region == 'next':
                break
        mn = x.mn[:-1] if x.mn[-1] in 'bwl' else x.mn
        if mn in ('move', 'clr', 'add', 'sub', 'and', 'or', 'addq', 'subq', 'bset',
                  'bclr', 'neg', 'not') and x.ops and not is_reg(x.ops[-1]) \
                and not x.ops[-1].startswith('%sp@-'):
            m = csesurvey._MEM.match(x.ops[-1])
            if m and m.group(1) not in ('a5', 'fp', 'a6', 'sp', 'a7') \
                    and not csesurvey.unlo(st.get(m.group(1), ('?',)))[0] == 'lea':
                events.add('ptr')
            else:
                events.add('st')
        written |= x.defs
    return uses


def sites(rt, sy):
    out = []
    for i in rt.reach:
        x = rt.ins[i]
        if x.kind != 'bcc':
            continue
        t = test_operand(rt, sy, i)
        if not t:
            continue
        key, form, reg, j = t
        before = set()
        for name, idxs in regions(rt, i):
            us = classify(rt, sy, key, form, reg, idxs, name)
            if name == 'next' and before:
                us = [(u, ','.join(sorted(set(filter(None, e.split(','))) | before)), a)
                      for u, e, a in us]
            if any(rt.ins[k].call for k in idxs):
                before.add('call-in-' + name)
            if us:
                out.append((form, name, us[0][0], us[0][1], rt.ins[j].addr, us[0][2],
                            key))
    return out


def short(s, n=40):
    """A short form of a canonical base or index."""
    s = re.sub(r"\('m', '([bwl])', '([^']*)'\)", r"m\1(\2)", s)
    s = re.sub(r"\('k', '?([^')]*)'?\)", r"k(\1)", s)
    s = s.replace("'", '')
    return s if len(s) <= n else s[:n] + '..'


def elem_calls(rt, sy):
    """The `jsr (An)' of a routine whose pointer is loaded from an
    element: [(jsr addr, form)], form `idx' (an indexed operand), `lea'
    (through the element's `lea' right before the load) or `lea-earlier'
    (a `lea' made before that: held)."""
    out = []
    for i in rt.reach:
        x = rt.ins[i]
        if x.mn != 'jsr' or not re.fullmatch(r'%a[0-7]@', x.ops[0]):
            continue
        r = x.ops[0][1:3]
        k = i - 1
        while k >= 0 and r not in rt.ins[k].defs:
            k -= 1
        y = rt.ins[k]
        if k < 0 or not y.mn.startswith('movea') or k not in sy.states:
            continue
        el = element(sy, y.ops[0], sy.states[k])
        if not el:
            continue
        if el[1] == 'idx':
            out.append((x.addr, 'idx'))
            continue
        d = k - 1
        while d >= 0 and el[2] not in rt.ins[d].defs:
            d -= 1
        out.append((x.addr, 'lea' if d == k - 1 else 'lea-earlier'))
    return out


def run_all(lo, hi):
    _, done, skipped = regalloc.load_all(lo, hi)
    res = []
    n = 0
    for seg, rt in done:
        try:
            sy = Snap(rt)
        except Unknown:
            continue
        n += 1
        for s in sites(rt, sy):
            res.append((rt.r0,) + s)
    return n, res


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    lo, hi, _, argv = regalloc.opts(argv)
    m = sau2map.load()
    cmd = argv[0]
    if cmd == 'routine':
        r0 = int(argv[1], 16)
        rt = regalloc.one(m, r0)
        sy = Snap(rt)
        for s in sites(rt, sy):
            print('test %06X %-3s %-4s use %06X %-12s [%s]  %s' % (
                s[4], s[0], s[1], s[5], s[2], s[3], s[6][1][:60]))
        return 0
    if cmd == 'calls':
        _, done, _ = regalloc.load_all(lo, hi)
        c = defaultdict(list)
        for seg, rt in done:
            try:
                sy = Snap(rt)
            except Unknown:
                continue
            for a, f in elem_calls(rt, sy):
                c[f].append('%06X' % a)
        for f in sorted(c):
            print('%-12s %3d  %s' % (f, len(c[f]), ' '.join(c[f])))
        return 0
    n, res = run_all(lo, hi)
    if cmd == 'summary':
        c = Counter((r[1], r[2], r[3].split()[0], r[4]) for r in res)
        print('routines analysed %d, sites %d' % (n, len(res)))
        for k in sorted(c):
            print('  test=%-3s %-4s %-8s [%s] %d' % (k + (c[k],)))
        return 0
    if cmd == 'list':
        flt = argv[1:]
        for r in res:
            f = (r[1], r[2], r[3].split()[0])
            if flt and tuple(flt[:len(f)]) != f[:len(flt)]:
                continue
            print('%06X %-28s test %06X %-3s %-4s use %06X %-12s [%s] base=%s idx=%s' % (
                r[0], m.describe(r[0])[:28], r[5], r[1], r[2], r[6], r[3], r[4],
                short(r[7][0]), short(r[7][1])))
        return 0
    print(__doc__)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
