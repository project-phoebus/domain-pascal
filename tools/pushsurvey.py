#!/usr/bin/env python3
"""pushsurvey.py - runs of adjacent 2-byte constant argument slots at the
call sites of the compiled code of the SR10.2 image: which pairs are
pushed as one longword.

Evidence tool for docs/codegen-patterns.md section 1.5 ("Adjacent word
constants merged"), bead domain-pascal-9pq8.  Compiled routines are
split and followed as tools/pcsurvey.py does (its cache).  For every
`jsr abs.l' / `bsr' with a target in compiled code, the pushes just
before it are collected backwards (argument set-up into registers in
between is skipped; the pad `subq.l #2,SP' ends the list) and given
offsets from the callee's first argument, (8,A6).  The callee's reads of
its frame above 8 (`(d,A6)' operands and their sizes) tell a longword
constant push that is two 2-byte slots (both halves read as a byte or a
word, never as a longword) from a longword argument.  A slot is a
constant when it is pushed by `move.w #n' / `clr.w' / `st', or is a half
of such a merged longword; a byte slot is one the callee reads only with
`.b'.  A *run* is a maximal sequence of adjacent constant slots other
than `st' (`true', never merged).  Its shape is the pushes in push order
(right to left): `L' a merged pair, `W' a single `move.w' / `clr.w'.
`L?' is a longword whose halves are read but not both (one half
unread).

usage:
    tools/pushsurvey.py summary     # runs by length and shape, merged
                                    # pairs by alignment
    tools/pushsurvey.py list [N]    # one line per run of N or more slots
                                    # (default 3)

Python 3 standard library + m68k-elf-objdump.  Read-only (uses
tmp/pcsurvey.cache, see tools/pcsurvey.py).
"""

import os
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

_FP = re.compile(r'%fp@\((\d+)\)')
_CALL = re.compile(r'^(?:jsr|bsr[sw]?) (0x[0-9a-f]+)$')
_SETUP = re.compile(r'^(movea[wl]|lea) .*,%a[0-7]$|^move[wlb] .*,%d\d$|^ext[wl] |^moveq ')


def push_form(text, length):
    """(size, kind, value) of a push instruction, None for another one.
    kinds: pad, lc (longword constant), wc (word constant), st, x."""
    if text == 'subql #2,%sp':
        return (2, 'pad', None)
    if text.startswith('pea '):
        m = re.match(r'^pea (-?0x[0-9a-f]+|\d+)$', text)
        if m and length == 4:   # `pea (n).w'; `pea abs.l' is 6 bytes
            return (4, 'lc', int(m.group(1), 0) & 0xffffffff)
        return (4, 'x', text)
    if not text.endswith('%sp@-'):
        return None
    op = text.split()[0]
    if op == 'clrl':
        return (4, 'lc', 0)
    if op == 'clrw':
        return (2, 'wc', 0)
    if op == 'st':
        return (2, 'st', 0xff)
    m = re.match(r'^movel #(-?\d+),%sp@-$', text)
    if m:
        return (4, 'lc', int(m.group(1)) & 0xffffffff)
    m = re.match(r'^movew #(-?\d+),%sp@-$', text)
    if m:
        return (2, 'wc', int(m.group(1)) & 0xffff)
    if op.endswith('l'):
        return (4, 'x', text)
    return (2, 'x', text)


def call_slots(ins, i):
    """[(offset, push addr, size, kind, value)] of the pushes before the
    call ins[i], from (8,A6) up (the last push first).

    The walk back stops at any instruction that is neither a push nor
    register setup (e.g. `lsl.l` at 0xE4BB22), so such calls are
    undercounted or cut short; the summary's "from the SP before the
    pushes" column is only meaningful when the walk reached the pad or
    the first argument."""
    pushes = []
    j = i - 1
    while j >= 0:
        a, n, t = ins[j]
        if a + n != ins[j + 1][0]:
            break
        p = push_form(t, n)
        if p:
            pushes.append((a,) + p)
            if p[1] == 'pad':
                break
        elif not (_SETUP.match(t) and '%sp' not in t):
            break
        j -= 1
    out, off = [], 8
    for a, size, kind, v in pushes:
        out.append((off, a, size, kind, v))
        off += size
    return out


def frame_reads(ins):
    """{offset: {size}} of a routine's reads of (d,A6), d >= 8.  Sizes
    b, w, l, `a' for `pea'."""
    out = defaultdict(set)
    for _, _, t in ins:
        for m in _FP.finditer(t):
            d = int(m.group(1))
            if d < 8:
                continue
            op = t.split()[0]
            if op == 'pea':
                out[d].add('a')
            elif op[-1] in 'bwl':
                out[d].add(op[-1])
            else:
                out[d].add('?')
    return out


def expand(slots, reads):
    """[(offset, width, value, push addr, form)]: a merged longword as two
    slots (forms 'merged', 'merged?'), a word or `st' push as one ('wc',
    'st'), others 'long', 'pad', 'x'.  width b/w from the callee's
    reads."""
    def width(r):
        return 'b' if r == {'b'} else 'w'
    out = []
    for off, a, size, kind, v in slots:
        if kind == 'lc':
            hi, lo = reads.get(off, set()), reads.get(off + 2, set())
            if hi and lo and (hi | lo) <= {'b', 'w'}:
                form = 'merged'
            elif (hi or lo) and (hi | lo) <= {'b', 'w'}:
                form = 'merged?'
            else:
                out.append((off, 'l', v, a, 'long'))
                continue
            out.append((off, width(hi), v >> 16, a, form))
            out.append((off + 2, width(lo), v & 0xffff, a, form))
        elif kind in ('wc', 'st'):
            out.append((off, width(reads.get(off, set())), v, a, kind))
        else:
            out.append((off, 'w' if size == 2 else 'l', v, a, kind))
    return out


def runs(entries):
    """Runs of adjacent constant slots other than `st', each a list of
    entries from the lowest offset (the leftmost argument)."""
    out, cur = [], []
    for e in entries:
        if e[4] in ('merged', 'merged?', 'wc'):
            cur.append(e)
            continue
        if cur:
            out.append(cur)
        cur = []
    if cur:
        out.append(cur)
    return out


def shape(run):
    """The pushes of a run in push order: 'L', 'L?', 'W'."""
    out, seen = [], set()
    for e in reversed(run):
        if e[3] in seen:
            continue
        seen.add(e[3])
        out.append({'merged': 'L', 'merged?': 'L?'}.get(e[4], 'W'))
    return ' '.join(out)


def scan():
    import pcsurvey
    m, segs = pcsurvey.load()
    routines = {}
    for s in segs:
        routines.update(s.insns)
    reads = {}
    sites = []
    for r0, ins in sorted(routines.items()):
        for i, (a, n, t) in enumerate(ins):
            c = _CALL.match(t)
            if not c:
                continue
            tgt = int(c.group(1), 16)
            if tgt not in routines:
                continue
            if tgt not in reads:
                reads[tgt] = frame_reads(routines[tgt])
            slots = call_slots(ins, i)
            sites.append((a, tgt, slots, expand(slots, reads[tgt])))
    return m, sites


def cmd_summary(m, sites, args):
    by = Counter()
    align = Counter()
    for a, tgt, slots, ent in sites:
        total = sum(s[2] for s in slots)
        for r in runs(ent):
            by[(len(r), shape(r))] += 1
        seen = set()
        for e in ent:
            if e[4] == 'merged' and e[3] not in seen:
                seen.add(e[3])
                align[((e[0] - 8) % 4, (e[0] - 8 - total) % 4)] += 1
    print('runs of constant 2-byte slots (length, shape in push order):')
    for (n, s), c in sorted(by.items()):
        print('  %3d  %d  %s' % (c, n, s))
    print('merged pairs by (offset from the first argument mod 4, '
          'offset from the SP before the pushes mod 4):')
    for k, c in sorted(align.items()):
        print('  %3d  %s' % (c, k))


def cmd_list(m, sites, args):
    least = int(args[0]) if args else 3
    for a, tgt, slots, ent in sites:
        for r in runs(ent):
            if len(r) < least:
                continue
            desc = ' '.join('%x:%s%s=%x' % (e[0], e[1], '*' if e[4].startswith('merged') else '',
                                            e[2]) for e in reversed(r))
            names = m.names_at(tgt)
            name = names[0] if names else None
            print('%06X call %06X%s n=%d [%s]  %s' % (
                r[-1][3], tgt, ' ' + name if name else '', len(r), shape(r), desc))


def main(argv):
    cmds = {'summary': cmd_summary, 'list': cmd_list}
    if len(argv) < 2 or argv[1] not in cmds:
        print(__doc__.split('usage:')[1].split('Python')[0].rstrip())
        return 2
    m, sites = scan()
    cmds[argv[1]](m, sites, argv[2:])
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
