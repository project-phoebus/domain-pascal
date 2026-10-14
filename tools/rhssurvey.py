#!/usr/bin/env python3
"""rhssurvey.py - is Apollo's register order "the assignment's target before
the values first used in its right-hand side"?  (bead domain-pascal-u6di;
docs/codegen-patterns.md 12.4 R1b)

R1b: a pointer local L kept from a call's result (`movea.l A0,An' after the
call) gets a lower callee-saved register than an entry-loaded parameter P
that interferes with it, at the six sites where P dies in the assignment's
block.  Hypothesis tested here: the original allocates walking each
statement top-down, the target of `L := f(args)' before any value whose
first source-level use is inside the right-hand side; a parameter's entry
load is scheduling, so its "first use" is its first read after the load.

Pairs (P, L), as in tools/regalloc.py param_pairs: one register class, both
in callee-saved registers (D2-D7 / A2-A4), interfering (colU's graph), P a
parameter load (`(d,A6)' source) loaded at the entry (in the entry block,
before any call), L any other value defined after P.  `all': every such
pair; `swap': only pairs that could exchange registers (param_pairs' test).

L's statement, in machine code:
  call results (L's first definition is `move* A0|D0,Rn' right after a
  call and its pop; regalloc.def_kind 'callres'): the anchor is the call;
  M$ARITH calls (32-bit `*', `div', `mod') are counted as non-call
  right-hand sides ('arith');
  other values: the anchor is L's first definition.
  The statement starts after the previous statement in the anchor's block:
  the last call (and its pop and result copies), the last store to memory
  that is not a push (regalloc.stmt_end), or the block start ("wide").
  "push": for calls, from the first argument push (`-(SP)' destination,
  `pea', the `subq.l #2,SP' pad) at or after the wide start; the ue7
  review's alternative.  "def": for other values, L's defining instruction
  alone.
P's first use: its first read (any instruction reading P's register with
P's definition reaching it: a push, a `pea (An)', a dereference, a copy, a
compare) in address order.  Category: 'before' (first use before the
statement start), 'rhs' (inside start..anchor), 'after' (after the
anchor), 'none' (never read).

usage:
    tools/rhssurvey.py table [--min N] [--max N] [--split even|odd] [N]
    tools/rhssurvey.py site ADDR...    # pairs whose P or L starts at ADDR

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import sys
from collections import Counter, defaultdict
from math import comb

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import regalloc  # noqa: E402
from regalloc import SCRATCH, def_kind, stmt_end, value_uses  # noqa: E402


def is_push(x):
    """An argument push: `-(SP)' destination, `pea', or the 2-byte pad."""
    if x.mn.startswith('pea'):
        return True
    if x.ops and x.ops[-1] == '%sp@-':
        return True
    return x.mn.startswith('subq') and x.ops == ['#2', '%sp']


def is_pop(x):
    return bool(x.ops) and x.ops[-1] == '%sp' and not x.call and not x.mn.startswith('subq')


def call_of(rt, v):
    """The index of the call whose result v's first definition copies, or
    None (regalloc.def_kind 'callres')."""
    if def_kind(rt, v) != 'callres':
        return None
    j = v.first - 1
    while j >= 0 and not rt.ins[j].call:
        j -= 1
    return j


def is_arith(rt, c):
    x = rt.ins[c]
    t = x.target if x.target is not None else (
        int(x.ops[0], 16) if x.ops and x.ops[0].startswith('0x') else None)
    return t is not None and regalloc.MARITH[0] <= t < regalloc.MARITH[1]


def block_list(rt, i):
    b = rt.block_of[i]
    return [j for j in rt.reach if rt.block_of[j] == b]


def stmt_start(rt, anchor, push=False):
    """The first instruction of the statement holding `anchor' (module
    docstring: "wide", or "push" for a call's argument pushes)."""
    blk = block_list(rt, anchor)
    k = blk.index(anchor)
    start, after_call = blk[0], False
    for j in reversed(blk[:k]):
        x = rt.ins[j]
        if x.call or stmt_end(x):
            start, after_call = j + 1, x.call
            break
    if after_call:
        # the previous call's pop and result copies belong to it
        kk = blk.index(start) if start in blk else k
        while kk < k:
            x = rt.ins[blk[kk]]
            if is_pop(x) or (x.mn.startswith('move') and x.ops
                             and x.ops[0] in ('%d0', '%a0') and regalloc.is_reg(x.ops[-1])):
                kk += 1
                continue
            break
        start = blk[kk]
    if push:
        run = [j for j in blk if start <= j < anchor and is_push(rt.ins[j])]
        start = run[0] if run else anchor
    return start


def entry_loaded(rt, v):
    """A parameter load in the entry block, before any call."""
    if rt.block_of[v.first] != rt.reach[0]:
        return False
    return not any(c < v.first for c in rt.calls)


def lkind(rt, L):
    c = call_of(rt, L)
    if c is not None:
        return 'arith' if is_arith(rt, c) else 'call', c
    return def_kind(rt, L), None


def classify(rt, P, L, mode):
    """'before' / 'rhs' / 'after' / 'none': where P's first use lies
    relative to L's statement.  mode: 'wide', 'push' (calls) or 'def'."""
    kind, c = lkind(rt, L)
    anchor = c if kind == 'call' else L.first
    if mode == 'def' and kind != 'call':
        s = L.first
    else:
        s = stmt_start(rt, anchor, push=(mode == 'push' and kind == 'call'))
    uses = value_uses(rt, P)
    if not uses:
        return 'none'
    u0 = min(uses)
    if u0 < s:
        return 'before'
    if u0 <= anchor:
        return 'rhs'
    return 'after'


def fate(rt, P, L):
    """'dies': every read of P after L's definition is in the block of
    that definition (R1b's condition); else 'lives'."""
    b = rt.block_of[L.first]
    later = [u for u in value_uses(rt, P) if u > L.first]
    return 'dies' if all(rt.block_of[u] == b for u in later) else 'lives'


def derivation(rt, P, L):
    """'derived' when L's first definition reads P (`L := P^.f',
    `L := addr(P^.f)', `L := P'), else 'indep'.  In 'def' mode a non-call
    'rhs' pair is 'derived' by construction; in 'wide' mode 'indep' rhs
    pairs include P reads in other register-only statements of the block
    (e.g. 0xE00CBC/0xE00CC0, the static link load interleaved)."""
    return 'derived' if L.first in value_uses(rt, P) else 'indep'


def pairs(rt, swap_only=False):
    """[(P, L, swappable)] as in the module docstring."""
    g = regalloc.Graph(rt)
    vs = rt.values
    out = []
    for kp, P in enumerate(vs):
        if not P.param or P.reg in SCRATCH or not entry_loaded(rt, P):
            continue
        for kl in g.nb[kp]:
            L = vs[kl]
            if L.param or L.reg in SCRATCH or L.cls is not P.cls or L.first < P.first:
                continue
            if L.reg == P.reg:
                continue
            others_p = {vs[j].reg for j in g.nb[kp] if j != kl}
            others_l = {vs[j].reg for j in g.nb[kl] if j != kp}
            sw = not (L.reg in others_p or L.reg in g.forbid[kp]
                      or P.reg in others_l or P.reg in g.forbid[kl])
            if swap_only and not sw:
                continue
            out.append((P, L, sw))
    return out


def binom_tail(k, n, p):
    """P(X >= k), X ~ Binomial(n, p)."""
    return sum(comb(n, i) * p ** i * (1 - p) ** (n - i) for i in range(k, n + 1))


def survey(done):
    """Counter over (class, L kind, filter, mode, category, 'L<P'|'P<L')
    and examples per key without the last element."""
    tot, ex = Counter(), defaultdict(list)
    for seg, rt in done:
        for P, L, sw in pairs(rt):
            kind, c = lkind(rt, L)
            grp = kind if kind in ('call', 'arith') else 'noncall'
            lo = 'L<P' if L.reg < P.reg else 'P<L'
            for mode in (('wide', 'push') if kind == 'call' else ('wide', 'def')):
                cat = classify(rt, P, L, mode)
                for filt in (('all', 'swap') if sw else ('all',)):
                    key = (P.reg[0], grp, filt, mode, cat)
                    tot[key + (lo,)] += 1
                    ex[key].append('%06X:%s/%06X:%s%s' % (
                        rt.ins[P.first].addr, P.reg, rt.ins[L.first].addr, L.reg,
                        '' if lo == 'L<P' else '!'))
                    if grp == 'call':
                        key2 = (P.reg[0], 'call:' + fate(rt, P, L), filt, mode, cat)
                        tot[key2 + (lo,)] += 1
                        ex[key2].append(ex[key][-1])
                    if grp == 'noncall':
                        key2 = (P.reg[0], 'nc:' + kind, filt, mode, cat)
                        tot[key2 + (lo,)] += 1
                        key3 = (P.reg[0], 'nc:' + derivation(rt, P, L), filt, mode, cat)
                        tot[key3 + (lo,)] += 1
                        ex[key3].append(ex[key][-1])
    return tot, ex


def main(argv):
    lo, hi, cls, rest = regalloc.opts(argv)
    split = None
    if '--split' in rest:
        k = rest.index('--split')
        split = rest[k + 1]
        rest = rest[:k] + rest[k + 2:]
    cmd = rest[0] if rest else 'table'
    m, done, skipped = regalloc.load_all(lo, hi)
    done = sorted(done, key=lambda d: d[1].r0)
    if split:
        done = [d for n, d in enumerate(done) if n % 2 == (split == 'odd')]
    if cmd == 'site':
        addrs = {int(a, 16) for a in rest[1:]}
        for seg, rt in done:
            if not any(rt.r0 <= a < rt.r1 for a in addrs):
                continue
            for P, L, sw in pairs(rt):
                if not addrs & {rt.ins[P.first].addr, rt.ins[L.first].addr}:
                    continue
                kind, c = lkind(rt, L)
                modes = ('wide', 'push') if kind == 'call' else ('wide', 'def')
                print('%s P %06X %s  L %06X %s (%s%s)  swap=%s  %s' % (
                    m.describe(rt.r0)[:28], rt.ins[P.first].addr, P.reg,
                    rt.ins[L.first].addr, L.reg, kind,
                    ' call %06X' % rt.ins[c].addr if c is not None else '', sw,
                    ' '.join('%s=%s' % (md, classify(rt, P, L, md)) for md in modes)))
        return 0
    show = int(rest[1]) if len(rest) > 1 else 4
    tot, ex = survey(done)
    print('routines %d (skipped %s)%s' % (len(done), dict(skipped),
                                          ', split ' + split if split else ''))
    print('%-46s %5s %5s  examples (! = P lower)' % ('class/L/filter/mode/P first use',
                                                       'L<P', 'P<L'))
    keys = sorted({k[:-1] for k in tot})
    for k in keys:
        a, b = tot[k + ('L<P',)], tot[k + ('P<L',)]
        print('%-46s %5d %5d  %s' % ('/'.join(k), a, b, ' '.join(ex.get(k, [])[:show])))
    # the hypothesis: rhs against the pooled rest of the same class/filter/mode
    print()
    print('rhs vs rest (same class/L/filter/mode): P(X>=rhs L<P | rest rate)')
    for k in keys:
        if k[4] != 'rhs' or ':' in k[1]:
            continue
        a, b = tot[k + ('L<P',)], tot[k + ('P<L',)]
        ra = sum(tot[k[:4] + (c, 'L<P')] for c in ('before', 'after', 'none'))
        rb = sum(tot[k[:4] + (c, 'P<L')] for c in ('before', 'after', 'none'))
        rate = ra / (ra + rb) if ra + rb else 0.5
        print('%-30s rhs %3d/%-3d rest %3d/%-3d rate %.2f  p=%.3g' % (
            '/'.join(k[:4]), a, b, ra, rb, rate, binom_tail(a, a + b, rate) if a + b else 1))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
