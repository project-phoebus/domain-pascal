#!/usr/bin/env python3
"""padsurvey.py - where a routine's entry loads fall relative to its first
call's argument pad, in the compiled code of the SR10.2 image (bead
domain-pascal-dfjh; codegen-patterns 12.4 R11, "The pad and the last
entry load").

Routines are those of tools/regalloc.py (map `I' segments, hand assembly
skipped, starting with `link.w A6').  The *entry block* of a routine is
the code after its prologue (`link`, the register save: `movem.l`,
`move.l Rn,-(SP)` or `pea (An)`) up to its first call (`jsr`, `bsr`);
a routine whose first branch, `rts` or branch target comes before that
call is left out.  Each instruction of the block is given a letter:

    A   the module data base, `lea abs,A5' (`movea.l #,A5')
    P   a pad, `subq.l #2,SP' (other sizes and `subq.w' are `P<n>')
    V   a parameter load into a data register, `move.<s> (d,A6),Dn',
        d >= 8 (a by-value parameter, or a slot held in Dn)
    S   a slot load into an address register, `movea.l (d,A6),An', d >= 8
    H   a load into a register through an address register an `S' of
        the block set (a held copy: PROC2_$SET_SERVER 0xE4147C)
    L   the dynamic link, `movea.l (A6),An'
    U   a push (`-(SP)', `pea')
    O   anything else

V, S, H and L are in upper case when the destination is callee-saved
(D2-D7, A2-A4: a value held across the call, an entry load) and in lower
case when it is scratch (an argument's temporary, or the pointer a held
copy is read through).

usage:
    tools/padsurvey.py summary          # the counts
    tools/padsurvey.py rule             # rule P (see `moves') against
                                        # every entry block with a pad:
                                        # the held load right after the
                                        # pad, or the instruction right
                                        # before it; contradictions listed
    tools/padsurvey.py list [PATTERN]   # one line per routine (the
                                        # pattern of loads and pads, the
                                        # whole block), optionally only
                                        # those whose load/pad pattern is
                                        # PATTERN (e.g. `PV')

Python 3 standard library + m68k-elf-objdump.  Read-only; writes only a
temporary file under tmp/ (through loopalign.disasm).
"""

import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import loopalign  # noqa: E402
import regalloc  # noqa: E402

_FP = r'%fp@\((\d+)\)'
_LOAD_D = re.compile(r'^move[bwl]\s+' + _FP + r',%(d\d)$')
_LOAD_A = re.compile(r'^moveal\s+' + _FP + r',%(a[0-4])$')
_LINK = re.compile(r'^moveal\s+%fp@,%(a[0-4])$')
_PAD = re.compile(r'^subq([lw])\s+#(\d+),%sp$')
_THRU = re.compile(r'^move[abwl]*\s+%(a[0-4])@(?:\(-?\d+\))?,%([ad]\d)$')
_BR = re.compile(r'^(bra|b(?:hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le)|db\w+)[sw]?\s+(?:%d\d,)?0x([0-9a-f]+)')


def held(c, reg):
    """c for a callee-saved destination (D2-D7, A2-A4: a value held
    across the call), c in lower case for a scratch one (an argument's
    temporary, or a pointer a held copy is read through)."""
    return c if reg[1] in '234567' else c.lower()


def classify(t, slot_regs):
    if re.match(r'^(lea\s+0x[0-9a-f]+|moveal\s+#0x[0-9a-f]+),%a5$', t):
        return 'A'
    m = _PAD.match(t)
    if m:
        return 'P' if (m.group(1), m.group(2)) == ('l', '2') else 'P%s%s' % (m.group(1), m.group(2))
    m = _LOAD_D.match(t)
    if m and int(m.group(1)) >= 8:
        return held('V', m.group(2))
    m = _LOAD_A.match(t)
    if m and int(m.group(1)) >= 8:
        slot_regs.add(m.group(2))
        return held('S', m.group(2))
    m = _THRU.match(t)
    if m and m.group(1) in slot_regs:
        return held('H', m.group(2))
    m = _LINK.match(t)
    if m:
        return held('L', m.group(1))
    if t.endswith('%sp@-') or t.startswith('pea'):
        return 'U'
    return 'O'


def entry_block(r0, r1):
    """(letters, insns) of the entry block of the routine r0..r1, or None."""
    ins = loopalign.disasm(r0, r1 - r0)
    targets = set()
    for a, n, t in ins:
        b = _BR.match(t)
        if b:
            targets.add(int(b.group(2), 16))
    i = 1                                   # past `link'
    if i < len(ins) and (ins[i][2].startswith('moveml') and ins[i][2].endswith('%sp@-')):
        i += 1
    while i < len(ins) and (re.match(r'^movel\s+%[ad]\d,%sp@-$', ins[i][2])
                            or re.match(r'^pea %a[2-5]@$', ins[i][2])):
        i += 1
    letters, body = [], []
    slot_regs = set()
    for a, n, t in ins[i:]:
        if a in targets:
            return None
        if t.startswith('jsr') or t.startswith('bsr'):
            return letters, body
        if _BR.match(t) or t.startswith('rts') or t.startswith('jmp') or t.startswith('unlk'):
            return None
        letters.append(classify(t, slot_regs))
        body.append((a, t))
    return None


LOADS = {'V', 'S', 'H', 'L', 'v', 's', 'h', 'l'}


def survey():
    _, rts = regalloc.routines(0, 0x100000)
    res = []
    for seg, r0, r1 in rts:
        eb = entry_block(r0, r1)
        if eb is None:
            continue
        res.append((seg, r0, eb[0], eb[1]))
    return res


def key(letters):
    return ''.join(x[0] if x.startswith('P') and x == 'P' else x for x in letters
                   if x in LOADS or x.startswith('P') or x == 'A')


def summary():
    res = survey()
    print('routines with an entry block ending in a call: %d' % len(res))
    padded = [r for r in res if 'P' in r[2]]
    other = [r for r in res if any(x.startswith('P') and x != 'P' for x in r[2])]
    nopad = [r for r in res if not any(x.startswith('P') for x in r[2])]
    print('  with a 2-byte pad: %d; another pad: %d; no pad: %d'
          % (len(padded), len(other), len(nopad)))
    print('\n2-byte pad: load/pad pattern (A = A5 base)   count')
    for k, n in Counter(key(r[2]) for r in padded).most_common():
        print('  %-24s %d' % (k, n))
    print('\n2-byte pad, by number of held loads (upper case): position of '
          'the pad (loads before it / loads in all)')
    c = Counter()
    for r in padded:
        ks = [x for x in r[2] if x in 'VSHLP']
        if 'P' not in ks:
            continue
        p = ks.index('P')
        c[(len(ks) - 1, p)] += 1
    for (n, p), m in sorted(c.items()):
        print('  %d loads, %d before the pad: %d' % (n, p, m))
    print('\nA5 base relative to the pad: before %d, after %d'
          % (sum(1 for r in padded if 'A' in r[2] and r[2].index('A') < r[2].index('P')),
             sum(1 for r in padded if 'A' in r[2] and r[2].index('A') > r[2].index('P'))))
    print('\nother pads: pattern   count')
    for k, n in Counter(key(r[2]) for r in other).most_common():
        print('  %-24s %d' % (k, n))
    print('\nno pad: loads relative to the first push (pattern with U = first push)')
    c = Counter()
    for r in nopad:
        ks = [x for x in r[2] if x in LOADS or x == 'U']
        if 'U' in ks:
            u = ks.index('U')
            ks = [x for x in ks[:u] if x != 'U'] + ['U'] + [x for x in ks[u + 1:] if x != 'U']
        c[''.join(ks)] += 1
    for k, n in c.most_common(20):
        print('  %-24s %d' % (k, n))


def operands(t):
    parts = t.split(None, 1)
    return regalloc.split_ops(parts[1]) if len(parts) > 1 else []


def dst_reg(t):
    """The register t writes as its last operand (`d2', `a0'), or None."""
    ops = operands(t)
    if ops and re.match(r'^%[ad]\d$', ops[-1]):
        return ops[-1][1:]
    return None


def reads(t, reg):
    """Whether t reads register reg (`sp', `d2', ...): named anywhere but
    as a plain destination register."""
    ops = operands(t)
    if t.startswith('pea') and reg == 'sp':
        return True
    if not ops:
        return False
    srcs = ops[:-1] + ([ops[-1]] if not re.match(r'^%[ad]\d$', ops[-1]) else [])
    return any(re.search(r'%' + reg + r'\b', o) for o in srcs) or (
        reg == 'sp' and ops[-1] in ('%sp@-', '%sp@+'))


def moves(x, y):
    """Rule P (bead domain-pascal-dfjh): the pad is placed above the
    instruction x generated right before it when y, the one after it,
    names SP in an operand (`move.<s> src,-(SP)', `clr.<s> -(SP)'; not
    `pea', whose push is implicit) and does not read the register x
    writes; x is a register write (no store, no push, nothing on SP)."""
    r = dst_reg(x)
    ops = operands(y)
    return (r is not None and r not in ('sp', 'a5') and not reads(x, 'sp')
            and not y.startswith('pea') and any('%sp' in o for o in ops)
            and not reads(y, r))


def rule():
    """Test rule P on every entry block with a pad: a held load right
    after the pad (`moved') must satisfy it with the next instruction;
    one right before it (`kept') must not."""
    c = Counter()
    bad = []
    for seg, r0, letters, body in survey():
        if 'P' not in letters:
            continue
        p = letters.index('P')
        t = [x for _, x in body]
        if p + 2 < len(t) and letters[p + 1] in 'VSHL':
            ok = moves(t[p + 1], t[p + 2])
            c[('moved-' + letters[p + 1], ok)] += 1
            if not ok:
                bad.append(('moved', r0, t[p - 1:p + 3] if p else t[p:p + 3]))
        elif p > 0 and p + 1 < len(t):
            k = letters[p - 1]
            kind = ('kept-load-' + k if k in 'VSHL' else 'kept-' + (
                'store' if dst_reg(t[p - 1]) is None else
                'a5' if dst_reg(t[p - 1]) == 'a5' else 'regwrite'))
            ok = not moves(t[p - 1], t[p + 1])
            c[(kind, ok)] += 1
            if not ok:
                bad.append((kind, r0, t[p - 1:p + 2]))
        else:
            c[('pad first, nothing held after', True)] += 1
    for (k, ok), n in sorted(c.items()):
        print('  %-34s %-13s %d' % (k, 'consistent' if ok else 'CONTRADICTS', n))
    for k, r0, ts in bad:
        print('  %s %06X  %s' % (k, r0, '; '.join(ts)))


def listing(pat=None):
    for seg, r0, letters, body in survey():
        k = key(letters)
        if pat is not None and k != pat:
            continue
        if pat is None and not any(x in LOADS for x in letters):
            continue
        print('%06X %-10s %-14s %s' % (r0, seg.name[:10], k,
                                       '; '.join(t for _, t in body)[:150]))


def main(argv):
    if len(argv) >= 2 and argv[1] == 'summary':
        summary()
    elif len(argv) >= 2 and argv[1] == 'rule':
        rule()
    elif len(argv) >= 2 and argv[1] == 'list':
        listing(argv[2] if len(argv) > 2 else None)
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
