#!/usr/bin/env python3
"""threadsurvey.py - branches to a `bra' and their threading in the
compiled code of the SR10.2 image.

Evidence tool for docs/codegen-patterns.md sections 7.1 and 7.2 (bead
domain-pascal-o6p).  Compiled routines are split and followed as
tools/cmpsurvey.py does (tools/arithsurvey.routines, a5survey.reached:
only the instructions reachable from the routine's start).  Kinds:

    tobra   a branch (`b<cc>', `bra', `dbf') or a `case' jump table
            entry (`table') whose target is a `bra':
            what the plain layout of a part ending in an `if' would give.
    thread  a forward branch at p to T over a `bra T' at q (p < q < T)
            whose next instruction (q + len) is the target of a branch
            from before p: the `bra' ends an outer `then' part, the
            branch at p ends or skips an inner part inside it, and goes
            straight to the outer join T, not to the `bra'.  The branch
            (`bra' or `b<cc>'), its size, whether a `.s' would reach the
            `bra' (q) as well, and what T is (`epi': the epilogue's
            `movem.l (-k,A6)' or `unlk'; else `other').
    dead    an unconditional transfer (`bra', `rts', `jmp') followed by a
            `bra' (opcode 0x60xx) that nothing reaches: a `bra' left in
            place after the branches to it were threaded.
    loop    a loop closed by a backward `b<cc>' at q to H: how its head is
            entered.  `bra': the instruction before H (the 0x2048 filler
            skipped) is a `bra' into the loop (the plain `while', 7.2);
            `threaded': the instruction before H is another unconditional
            transfer and the loop is entered by a branch from outside to
            its test (TTY_$K_PUT 0xE67C0E); `unentered': neither;
            `fall': entered by falling through (`repeat'); `gap': nothing
            reached ends right before the head.  Then the head's
            alignment: `F' the 0x2048 filler before it (whether or not
            the filler is reached), `A' 4-aligned, `U' at 2 mod 4.  A
            backward `b<cc>' to shared code (a `goto' label) counts too.
    next    a branch (`bra', `b<cc>') to the instruction right after it
            (bead domain-pascal-pkle: an `exit' ending the body of a
            loop with no test, codegen-patterns 7.2).
    over    a conditional branch over one `bra' to the instruction
            after that `bra' (codegen-patterns 7.1, 7.2).
    noepi   a routine starting with `link' none of whose reached
            instructions is `rts', `rtd', `unlk' or `jmp': the address
            and text of its last reached instruction (a loop with no
            way out, codegen-patterns 7.2).

usage:
    tools/threadsurvey.py summary          # the counts quoted in codegen-patterns
    tools/threadsurvey.py list KIND        # one line per site of KIND

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import a5survey  # noqa: E402
import arithsurvey  # noqa: E402
import loopalign  # noqa: E402

KINDS = ('tobra', 'thread', 'dead', 'loop', 'next', 'over', 'noepi')

_TABLE = re.compile(r'^jmp %pc@\(0x([0-9a-f]+),%d\d:w\)')
_BR = re.compile(r'^(bra|b(?:hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le)|db\w+)([sw]?)\s+'
                 r'(?:%d\d,)?0x([0-9a-f]+)$')


def branch(t):
    """(mnemonic, size letter, target) of a branch text, or None."""
    m = _BR.match(t)
    return (m.group(1), m.group(2), int(m.group(3), 16)) if m else None


def uncond(t):
    return (t.startswith('bra') or t.startswith('rts') or t.startswith('rte')
            or t.startswith('rtd') or t.startswith('jmp'))


def is_epi(t):
    return t.startswith('moveml %fp@(') or t.startswith('unlk')


def tobra_sites(ins, r=None):
    """[(addr, mnemonic, target)] of the branches to a `bra' (with r =
    (r0, r1), the routine's bounds, `case' table entries too: mnemonic
    `table', addr the `jmp')."""
    at = {a: t for a, _, t in ins}
    out = []
    prev = []
    for a, _, t in ins:
        b = branch(t)
        if b and b[2] in at and at[b[2]].startswith('bra'):
            out.append((a, b[0] + b[1], b[2]))
        tb = _TABLE.match(t)
        if tb and r is not None:
            for e in a5survey.table_entries(int(tb.group(1), 16), prev, *r):
                if e in at and at[e].startswith('bra'):
                    out.append((a, 'table', e))
        prev.append(t)
    return out


def thread_sites(ins):
    """[(addr, kind, size, short_to_bra, target_kind)] (see `thread')."""
    at = {a: (n, t) for a, n, t in ins}
    brs = [(a, branch(t)) for a, _, t in ins if branch(t)]
    out = []
    for p, (mn, sz, tg) in brs:
        if tg <= p or mn.startswith('db'):
            continue
        for q, (mq, _, tq) in brs:
            if not (p < q < tg) or mq != 'bra' or tq != tg:
                continue
            nxt = q + at[q][0]
            if not any(r < p and t2 == nxt for r, (_, _, t2) in brs):
                continue
            disp = q - (p + 2)
            kind = 'bra' if mn == 'bra' else 'bcc'
            tk = 'epi' if tg in at and is_epi(at[tg][1]) else 'other'
            out.append((p, kind, sz, -128 <= disp <= 127, tk))
            break
    return out


def dead_sites(ins, word=None):
    """[(addr of the dead `bra', the transfer before it)]."""
    word = word or loopalign.word
    have = {a for a, _, _ in ins}
    out = []
    for a, n, t in ins:
        nxt = a + n
        if uncond(t) and nxt not in have and (word(nxt) & 0xFF00) == 0x6000:
            out.append((nxt, t.split()[0]))
    return out


def loop_sites(ins, word=None):
    """[(addr of the backward branch, head, entry kind, filler)]: filler
    is whether the 0x2048 filler (`movea.l A0,A0') lies right before the
    head (unreached when the head is entered by a branch)."""
    word = word or loopalign.word
    idx = {a: i for i, (a, _, _) in enumerate(ins)}
    end = {a + n: i for i, (a, n, _) in enumerate(ins)}
    brs = [(a, branch(t)) for a, _, t in ins if branch(t)]
    out = []
    for q, (mn, _, h) in brs:
        if h >= q or mn == 'bra' or mn.startswith('db') or h not in idx:
            continue
        fill = word(h - 2) == 0x2048
        i = end.get(h - 2 if fill else h)
        if i is None and fill:
            fill, i = False, end.get(h)
        if i is None:
            kind = 'gap'      # nothing reached right before the head
        else:
            prev = ins[i][2]
            b = branch(prev)
            if b and b[0] == 'bra' and h < b[2] <= q:
                kind = 'bra'
            elif uncond(prev):
                ent = [r for r, (_, _, t2) in brs if (r < h or r > q) and h < t2 <= q]
                kind = 'threaded' if ent else 'unentered'
            else:
                kind = 'fall'
        out.append((q, h, kind, 'F' if fill else ('A' if h % 4 == 0 else 'U')))
    return out


def next_sites(ins):
    """[(addr, mnemonic)] of the branches to the next instruction."""
    out = []
    for a, n, t in ins:
        b = branch(t)
        if b and not b[0].startswith('db') and b[2] == a + n:
            out.append((a, b[0] + b[1]))
    return out


def over_sites(ins):
    """[(addr, mnemonic)] of the conditional branches over one `bra' (to
    the instruction right after it)."""
    out = []
    for (a, n, t), (a2, n2, t2) in zip(ins, ins[1:]):
        b = branch(t)
        if (b and b[0] != 'bra' and not b[0].startswith('db') and a2 == a + n
                and t2.startswith('bra') and b[2] == a2 + n2):
            out.append((a, b[0] + b[1]))
    return out


def noepi_sites(ins):
    """[(addr, text)] of the last reached instruction when none of `ins'
    returns (`rts', `rtd', `unlk', `jmp'); [] otherwise."""
    if not ins or any(t.startswith(('rts', 'rtd', 'unlk', 'jmp')) for _, _, t in ins):
        return []
    a, _, t = max(ins)
    return [(a, t)]


def scan():
    res = {k: [] for k in KINDS}
    for r0, r1, name in arithsurvey.routines():
        ins = a5survey.reached(r0, r1)
        res['tobra'] += [(r0, name) + s for s in tobra_sites(ins, (r0, r1))]
        res['thread'] += [(r0, name) + s for s in thread_sites(ins)]
        res['dead'] += [(r0, name) + s for s in dead_sites(ins)]
        res['loop'] += [(r0, name) + s for s in loop_sites(ins)]
        res['next'] += [(r0, name) + s for s in next_sites(ins)]
        res['over'] += [(r0, name) + s for s in over_sites(ins)]
        if loopalign.word(r0) == 0x4E56:
            res['noepi'] += [(r0, name) + s for s in noepi_sites(ins)]
    return res


def first(sites, n=4):
    return ', '.join('0x%X' % s[2] for s in sites[:n])


def counts(title, sites, key):
    print('%s: %d' % (title, len(sites)))
    for k, v in sorted(Counter(key(s) for s in sites).items(), key=lambda kv: -kv[1]):
        print('  %-40s %4d  e.g. %s' % (' '.join(str(x) for x in k), v,
                                        first([s for s in sites if key(s) == k])))


def summary(res):
    counts('branches to a `bra\' (branch)', res['tobra'], lambda s: (s[3],))
    counts('threaded past a `bra\' to its target (branch, size, .s reaches the bra, target)',
           res['thread'], lambda s: (s[3], s[4] or '-', 'near' if s[5] else 'far', s[6]))
    counts('unreached `bra\' after a transfer (transfer)', res['dead'], lambda s: (s[3],))
    counts('loops closed by a backward b<cc> (head entered by, head: F filler, A 4-aligned, U not)', res['loop'],
           lambda s: (s[4], s[5]))
    counts('branches to the next instruction (branch)', res['next'], lambda s: (s[3],))
    counts('conditional branches over one `bra\' (branch)', res['over'], lambda s: (s[3],))
    counts('routines with a `link\' and no reached return (last instruction)', res['noepi'],
           lambda s: (s[3].split()[0],))


def main(argv):
    if not argv or argv[0] not in ('summary', 'list') or (
            argv[0] == 'list' and (len(argv) < 2 or argv[1] not in KINDS)):
        print(__doc__)
        return 2
    res = scan()
    if argv[0] == 'summary':
        summary(res)
    else:
        for s in res[argv[1]]:
            print('0x%06X %-28s %s' % (s[2], '%06X %s' % (s[0], s[1]), ' '.join(
                '0x%X' % x if isinstance(x, int) and not isinstance(x, bool) else str(x)
                for x in s[3:])))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
