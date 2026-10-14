#!/usr/bin/env python3
"""boolsurvey.py - boolean `and'/`or' values, bit-0 tests and record
addresses held in a register, in the compiled code of the SR10.2 image.

Evidence tool for docs/codegen-patterns.md sections 7.1 ("Plain `and' and
`or'"), 8 (`odd') and 5.2 (`with'), bead domain-pascal-5aw.19.  Compiled
routines are split and followed as tools/cmpsurvey.py does (map symbols
and `link.w A6' words, hand assembly excluded, only the instructions
reachable from the routine's start, in address order).  Kinds:

    andor   `and.b'/`or.b' into a data register (`and.b <ea>,Dn'), the
            plain `and'/`or' of two booleans: how each operand was formed
            (the last write of the register among the six instructions
            before: `s<cc>' -> scc, `move.b <mem>,Dn' -> load, `move.b
            Dm,Dn' -> copy, `not.b' -> not+<the write before it>, an
            earlier `and.b'/`or.b' -> chain; a memory source -> mem), and
            what reads the result next (`bmi'/`bpl'/`beq'/`bne' straight
            on its flags, a further `and.b'/`or.b' (chain), `not.b' and
            what follows it, `move.b Dn,<mem>' (store, with the branch
            after it if any), `move.b Dn,Dm', a push, `tst.b').  Only
            sites with at least one boolean-formed operand (scc, not+scc,
            chain) or a sign branch after are counted, so byte arithmetic
            on chars is left out.
    bit0    `btst #0,<ea>' and what follows (branch on Z, `s<cc>'), by
            operand (a data register; memory at an odd or an even
            displacement).  Bit 0 of a register is `odd(x)' or member 0
            of a set held in it (ref 3.9.3: member k is bit k): the image
            cannot tell them apart without the source.
    scc     every `s<cc>' other than `st'/`sf' (a computed boolean): into
            memory or into a data register, and for a register what reads
            it next (`move.b Dn,<mem>' store, `move.b Dn,-(SP)' push,
            `move.b Dn,Dm', a branch, `tst.b', `and.b'/`or.b', other),
            and whether the instruction right before it sets the flags it
            reads (a compare or a test: `cmp', `tst', `btst'; bead
            domain-pascal-k1sv, codegen-patterns 3, "Comparisons as
            boolean results").
    elem    `lea (0,An,Dx.<s>),Am' that forms an element address, and how
            many memory operands `(d,Am)' use Am before it is written
            again or a branch, call or label intervenes; the displacements
            (a 1-based array's -lo*size folded into them, NETLOG_$LOG_IT
            0xE71BCA..0xE71BFC, eleven fields through A0, the shape of
            `with a[i] do').

usage:
    tools/boolsurvey.py summary         # the counts quoted in codegen-patterns
    tools/boolsurvey.py list KIND       # one line per site of KIND

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

KINDS = ('andor', 'bit0', 'scc', 'elem')

_LOGIC = re.compile(r'^(and|or)b (\S+),%(d\d)$')
_SCC = re.compile(r'^s(?:t|f|hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le) %(d\d)$')
_BR = re.compile(r'^b(mi|pl|eq|ne|ge|lt|gt|le|hi|ls|cc|cs)[sw]? ')
_LEA_ELEM = re.compile(r'^lea %(a\d)@\(0,%(d\d):([wl])\),%(a\d)$')
_FLOW = re.compile(r'^(b[a-z]{2}[sw]?|bra[sw]?|jsr|bsr[sw]?|jmp|rts|db[a-z]+) ')


def writes(t, d):
    """Whether instruction text t writes data register %d (its last
    operand, for the forms that matter here)."""
    return re.search(r',%' + d + r'$', t) is not None or re.match(
        r'^[a-z]+ %' + d + r'$', t) is not None


def former(ins, i, d, depth=0):
    """How the value of %d read at ins[i] was formed."""
    for k in range(i - 1, max(-1, i - 7), -1):
        t = ins[k][2]
        if _BR.match(t) or t.startswith(('jsr', 'bsr', 'bra')):
            return 'flow'
        if not writes(t, d):
            continue
        if _SCC.match(t):
            return 'scc'
        if t == 'notb %' + d:
            return 'not+' + (former(ins, k, d, depth + 1) if depth < 2 else '?')
        m = _LOGIC.match(t)
        if m and m.group(3) == d:
            return 'chain'
        if re.match(r'^moveb %d\d,%' + d + '$', t):
            return 'copy'
        if t.startswith('moveb '):
            return 'load'
        return 'other'
    return 'none'


def reader(ins, i, d):
    """What reads the result of ins[i] (in %d) next."""
    if i + 1 >= len(ins):
        return 'end'
    t = ins[i + 1][2]
    b = _BR.match(t)
    if b:
        return 'b' + b.group(1)
    m = _LOGIC.match(t)
    if m and (m.group(3) == d or m.group(2) == '%' + d):
        return 'chain'
    if t == 'notb %' + d:
        return 'not,' + reader(ins, i + 1, d)
    if t == 'tstb %' + d:
        return 'tst'
    m = re.match(r'^moveb %' + d + r',(\S+)$', t)
    if m:
        dst = m.group(1)
        if dst == '%sp@-':
            return 'push'
        if re.match(r'^%d\d$', dst):
            return 'move'
        nxt = _BR.match(ins[i + 2][2]) if i + 2 < len(ins) else None
        return 'store' + (',b' + nxt.group(1) if nxt else '')
    return 'other:' + t.split()[0]


def andor_sites(ins):
    """[(addr, op, dst-form, src-form, reader)]."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = _LOGIC.match(t)
        if not m:
            continue
        op, src, d = m.groups()
        if src.startswith('#'):
            continue
        dform = former(ins, i, d)
        sform = former(ins, i, src[1:]) if re.match(r'^%d\d$', src) else 'mem'
        rd = reader(ins, i, d)
        boolish = {'scc', 'chain', 'not+scc'}
        if not (dform in boolish or sform in boolish
                or rd in ('bmi', 'bpl', 'not,bmi', 'not,bpl')):
            continue
        out.append((a, op, dform, sform, rd))
    return out


def bit0_sites(ins):
    """[(addr, operand kind, follower)]."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = re.match(r'^btst #0,(\S+)$', t)
        if not m:
            continue
        ea = m.group(1)
        if re.match(r'^%d\d$', ea):
            kind = 'reg'
        else:
            dm = re.search(r'\((-?\d+)\)$', ea)
            kind = 'mem-odd' if dm and int(dm.group(1)) % 2 else 'mem-even'
        nxt = ins[i + 1][2] if i + 1 < len(ins) else ''
        b = _BR.match(nxt)
        if b:
            fol = 'b' + b.group(1)
        elif _SCC.match(nxt):
            fol = nxt.split()[0]
        else:
            fol = 'other'
        out.append((a, kind, fol))
    return out


_SCC_ANY = re.compile(r'^s(hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le) (\S+)$')


def scc_sites(ins):
    """[(addr, destination, follower, compare before)] of the `s<cc>'
    other than `st'/`sf'."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = _SCC_ANY.match(t)
        if not m:
            continue
        dst = m.group(2)
        prev = ins[i - 1][2] if i > 0 else ''
        cmp_before = 'cmp' if re.match(r'^(cmp|tst|btst)', prev) else 'other'
        if not re.match(r'^%d\d$', dst):
            out.append((a, 'mem', '-', cmp_before))
            continue
        nxt = ins[i + 1][2] if i + 1 < len(ins) else ''
        mv = re.match(r'^moveb ' + dst + r',(\S+)$', nxt)
        if mv:
            d2 = mv.group(1)
            fol = ('push' if d2 == '%sp@-' else 'move' if re.match(r'^%d\d$', d2)
                   else 'store')
        elif _BR.match(nxt):
            fol = 'branch'
        elif nxt == 'tstb ' + dst:
            fol = 'tst'
        elif _LOGIC.match(nxt):
            fol = 'andor'
        else:
            fol = 'other'
        out.append((a, 'reg', fol, cmp_before))
    return out


def elem_sites(ins, labels=frozenset()):
    """[(addr, index size, uses, displacements)] of element addresses
    formed by `lea (0,An,Dx),Am' and used through Am."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = _LEA_ELEM.match(t)
        if not m:
            continue
        am = m.group(4)
        uses, disps = 0, []
        for k in range(i + 1, len(ins)):
            u = ins[k][2]
            if ins[k][0] in labels or _FLOW.match(u):
                break
            n = len(re.findall('%' + am + r'@', u))
            for dm in re.finditer('%' + am + r'@(?:\((-?\d+)\))?', u):
                disps.append(int(dm.group(1) or 0))
            uses += n
            if re.search(r',%' + am + r'$', u) and not re.search(r'%' + am + r'@[^,]*$', u):
                break
        out.append((a, m.group(3), uses, tuple(disps)))
    return out


def branch_targets(ins):
    return frozenset(int(b.group(1), 16) for _, _, t in ins
                     for b in [re.search(r' 0x([0-9a-f]+)$', t)]
                     if b and (_BR.match(t) or t.startswith('bra')))


def scan():
    import a5survey  # noqa: E402
    import arithsurvey  # noqa: E402
    res = {k: [] for k in KINDS}
    for r0, r1, name in arithsurvey.routines():
        ins = a5survey.reached(r0, r1)
        res['andor'] += [(r0, name) + s for s in andor_sites(ins)]
        res['bit0'] += [(r0, name) + s for s in bit0_sites(ins)]
        res['scc'] += [(r0, name) + s for s in scc_sites(ins)]
        res['elem'] += [(r0, name) + s for s in elem_sites(ins, branch_targets(ins))]
    return res


def first(sites, n=4):
    return ', '.join('0x%X' % s[2] for s in sites[:n])


def counts(title, sites, key):
    print('%s: %d' % (title, len(sites)))
    for k, v in sorted(Counter(key(s) for s in sites).items(), key=lambda kv: -kv[1]):
        print('  %-44s %4d  e.g. %s' % (' '.join(str(x) for x in k), v,
                                        first([s for s in sites if key(s) == k])))


def summary(res):
    counts('and.b/or.b of booleans (op, dst formed, src formed)', res['andor'],
           lambda s: (s[3], s[4], s[5]))
    counts('and.b/or.b of booleans: what reads the result', res['andor'],
           lambda s: (s[6],))
    counts('btst #0 (operand, follower)', res['bit0'], lambda s: (s[3], s[4]))
    counts('s<cc> other than st/sf (destination, follower, flags set by the insn before)',
           res['scc'], lambda s: (s[3], s[4], s[5]))
    counts('element addresses lea (0,An,Dx),Am (index size, uses through Am)',
           res['elem'], lambda s: (s[3], min(s[4], 3) if s[4] < 3 else '3+'))


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
            print('0x%06X %-28s %s' % (s[2], '%06X %s' % (s[0], s[1]),
                                      ' '.join(str(x) for x in s[3:])))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
