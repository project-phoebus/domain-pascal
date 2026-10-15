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
    ebase   every indexed operand `(d,An,Dx.<s>)' whose base An is A0-A4
            (codegen-patterns 5.2, bead domain-pascal-rlnm): how An was
            set within the block (`movea.l #imm': `imm=sym' when the
            immediate is a map symbol or a data segment's start, `imm+'
            otherwise; `lea abs.l'; a parameter's slot `movea.l (d,A6)'
            with d > 0, a local's with d < 0; a pointer of the unit
            `movea.l (d,A5)'; a pointer field `movea.l (d,Am)'; a copy
            `movea.l Rn'; another `lea'; `held' when not set in the
            block), the displacement (zero or not), and where the base's
            set-up sits among the index register's instructions: the
            chain of instructions that form Dx (back from the operand to
            its first full write in the block: a load, a register copy,
            `clr', `moveq'; through the temporaries of a shift-and-add),
            as `k/n', k of the n chain instructions before the base, and
            the kind of the chain's first instruction (`none' when Dx is
            not written in full in the block: an index held from before);
            then whether the base's place fits the rule of codegen-patterns
            5.2 under 12.4 R11 (`_fit'), `loop' or `straight' (inside a
            backward branch's range or not), and for a `movea.l #' base the
            immediate and the displacement.

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

KINDS = ('andor', 'bit0', 'scc', 'elem', 'ebase')

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


_IDX = re.compile(r'%(a[0-4])@\((-?[0-9a-fx]+),%(d\d):([wl])\)')
_DREG = re.compile(r'^%d\d$')
_FULL = ('movew', 'movel', 'moveq', 'clrw', 'clrl')


def _disp(x):
    """An indexed operand's displacement as objdump prints it: hex without
    `0x', a negative one in 16 digits."""
    v = int(x, 16)
    return v - (1 << 64) if len(x) == 16 else v


def _dst(t):
    """The register an instruction text writes as its last operand
    (`%d0', `%a1'), or None."""
    m = re.search(r'(?:,|^[a-z]+ )%([ad]\d)$', t)
    return m.group(1) if m else None


def _base_kind(t, sym):
    """How `t' sets an address register (see `ebase')."""
    m = re.match(r'^moveal #(\d+),', t)
    if m:
        return 'imm=sym' if sym(int(m.group(1))) else 'imm+'
    if re.match(r'^lea 0x[0-9a-f]+,', t):
        return 'lea-abs'
    m = re.match(r'^moveal %fp@\((-?\d+)\),', t)
    if m:
        return 'slot' if int(m.group(1)) > 0 else 'local'
    if re.match(r'^moveal %a5@', t):
        return 'unit-ptr'
    if re.match(r'^moveal %a\d@', t):
        return 'field-ptr'
    if re.match(r'^moveal %[ad]\d,', t):
        return 'copy'
    if t.startswith('lea '):
        return 'lea'
    return 'other:' + t.split()[0]


def ebase_sites(ins, labels=frozenset(), sym=lambda a: False):
    """[(addr, base kind, displacement zero or not, `k/n', chain start)]
    of the indexed operands through A0-A4 (see `ebase')."""
    out = []
    loops = [(int(g.group(1), 16), x) for x, _, u in ins
             for g in [re.search(r' (?:%d\d,)?0x([0-9a-f]+)$', u)]
             if g and (_BR.match(u) or u.startswith(('bra', 'db')))
             and int(g.group(1), 16) <= x]
    for i, (a, _, t) in enumerate(ins):
        m = _IDX.search(t)
        if not m:
            continue
        an, d, dx = m.group(1), _disp(m.group(2)), m.group(3)
        bs = i
        while bs > 0 and ins[bs][0] not in labels and not _FLOW.match(ins[bs - 1][2]):
            bs -= 1
        b = next((k for k in range(i - 1, bs - 1, -1) if _dst(ins[k][2]) == an), None)
        kind = _base_kind(ins[b][2], sym) if b is not None else 'held'
        regs, chain, start = {dx}, [], 'none'
        for k in range(i - 1, bs - 1, -1):
            u = ins[k][2]
            r = _dst(u)
            if r not in regs:
                continue
            chain.append(k)
            op = u.split()[0]
            src = u.split()[1].split(',')[0] if ' ' in u and ',' in u else ''
            if op in _FULL and not (m.group(4) == 'l' and op == 'movew' and r == dx):
                regs.discard(r)
                if not regs:
                    start = ('clr' if op.startswith('clr') else 'moveq' if op == 'moveq'
                             else 'copy' if _DREG.match(src) else 'load')
                    break
                if _DREG.match(src):
                    regs.add(src[1:])
            elif _DREG.match(src):
                regs.add(src[1:])
        before = sum(1 for k in chain if b is not None and k < b)
        imm = re.match(r'^moveal #(\d+),', ins[b][2]) if b is not None else None
        out.append((a, kind, 'd0' if d == 0 else 'd', '%d/%d' % (before, len(chain)),
                    start, _fit(ins, bs, b, sorted(chain), start, before),
                    'loop' if any(t0 <= a <= b0 for t0, b0 in loops) else 'straight',
                    '0x%X%+d' % (int(imm.group(1)), d) if imm else ''))
    return out


def _reads(t):
    """The registers instruction text t reads, roughly: every register of
    its source operand and of its destination's address."""
    ops = t.split(' ', 1)[1] if ' ' in t else ''
    src, _, dst = ops.rpartition(',') if ',' in ops else ('', '', ops)
    regs = set(re.findall(r'%([ad]\d|fp|sp)', src))
    regs |= set(re.findall(r'%([ad]\d|fp|sp)@', dst))
    regs |= set(re.findall(r',%([ad]\d):', dst))
    return regs


_WHOLE = ('movel', 'moveq', 'clrl', 'moveal', 'lea')


def _rw(t):
    """(reads, writes) of instruction text t, registers only: a byte or
    word write to a data register, and arithmetic, also read it."""
    w = _dst(t)
    r = _reads(t)
    op = t.split()[0]
    if w and op not in _WHOLE and not (w.startswith('a') and op.startswith('move')):
        r.add(w)
    return r, ({w} if w else set())


def _r11(gen, prev, window=3):
    """The order R11 (codegen-patterns 12.4) gives the instruction texts
    `gen' (in generation order) after `prev' (the text placed just
    before, or None): top-down, the first ready instruction among the
    first `window' not placed that does not read a register the one just
    placed writes, else the first ready one.  -> the indices of `gen'."""
    rw = [_rw(t) for t in gen]

    def dep(i, j):
        (ri, wi), (rj, wj) = rw[i], rw[j]
        return bool(wi & rj or ri & wj or wi & wj)
    order, left = [], list(range(len(gen)))
    last = _rw(prev)[1] if prev else set()
    while left:
        cands = left[:window]
        ready = [j for j in cands if not any(dep(i, j) for i in left if i < j)]
        pick = next((j for j in ready if not last & rw[j][0]), ready[0])
        order.append(pick)
        left.remove(pick)
        last = rw[pick][1]
    return order


_SCALE = re.compile(r'^(?:(?:lsl|asl)[wl] #\d+|(?:add|sub)[wl] %d\d|neg[wl] %d\d|mul[us]w #\d+|'
                    r'move[wl] %d\d)(?:,%d\d)?$')


def _fit(ins, bs, b, chain, start, before):
    """How the base's place fits the rule of codegen-patterns 5.2 under the
    scheduler of 12.4 R11 (W = 3), simulated over the index's chain, the
    base and the instruction placed before them.  The chain is split into
    the index's value and its scaling (the trailing shifts, adds, copies,
    `neg' and `mulu'/`muls' by a constant; a copy that starts it belongs
    to the value).  Generation orders: the value, the base, the scaling
    (`val'); the whole chain, then the base (`all').  The rule: `all'
    when the scaling has a `neg' (2^s (2^k - 1), section 8) or three adds
    or more (the bit-by-bit ×26 of 0xE71BBA), `val' otherwise.  ->
    'fit' or 'nofit' (the rule), with `/val' or `/all' when the other
    order fits instead, and `:s0', `:s1' or `:s2' for a scaling of no,
    one, or two instructions or more (with no or one, the two orders give
    the same under R11); `-' where it says nothing (a base not set in the
    block or computed, an index not formed in full in the block)."""
    if b is None or start not in ('load', 'copy', 'clr') or not chain:
        return '-'
    first = min(b, chain[0])
    prev = ins[first - 1][2] if first > bs else None
    texts = [ins[k][2] for k in chain]
    nv = len(texts)
    while nv > 0 and _SCALE.match(texts[nv - 1]):
        nv -= 1
    if nv == 0 and start == 'clr':
        nv = 2
    nv = max(nv, 1)
    while nv < len(texts) and texts[nv].startswith('move'):
        nv += 1
    scaling = texts[nv:]
    other = any(t.startswith('neg') for t in scaling) or sum(
        t.startswith('add') for t in scaling) >= 3
    fits = []
    for gen in (texts[:nv] + [ins[b][2]] + scaling, texts + [ins[b][2]]):
        order = _r11(gen, prev)
        fits.append(order.index(gen.index(ins[b][2])) == before)
    want, alt = (fits[1], fits[0]) if other else (fits[0], fits[1])
    tag = 's%d' % min(len(scaling), 2)
    if want:
        return 'fit:' + tag
    return 'nofit' + ('/' + ('val' if other else 'all') if alt else '') + ':' + tag


def branch_targets(ins):
    return frozenset(int(b.group(1), 16) for _, _, t in ins
                     for b in [re.search(r' 0x([0-9a-f]+)$', t)]
                     if b and (_BR.match(t) or t.startswith('bra')))


def scan():
    import a5survey  # noqa: E402
    import arithsurvey  # noqa: E402
    import sau2map  # noqa: E402
    m = sau2map.load()
    starts = frozenset(s.addr for s in m.symbols) | frozenset(
        g.addr for g in m.segments + m.sections if g.kind.startswith('D'))

    def sym(a):
        return a in starts
    res = {k: [] for k in KINDS}
    for r0, r1, name in arithsurvey.routines():
        ins = a5survey.reached(r0, r1)
        res['andor'] += [(r0, name) + s for s in andor_sites(ins)]
        res['bit0'] += [(r0, name) + s for s in bit0_sites(ins)]
        res['scc'] += [(r0, name) + s for s in scc_sites(ins)]
        res['elem'] += [(r0, name) + s for s in elem_sites(ins, branch_targets(ins))]
        res['ebase'] += [(r0, name) + s for s in ebase_sites(ins, branch_targets(ins), sym)]
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
    counts('indexed operands through A0-A4 (base set-up, displacement, base after k of n '
           'index instructions, the index chain\'s start)', res['ebase'],
           lambda s: (s[3], s[4], s[5], s[6]))
    counts('indexed operands through A0-A4: the base\'s place against the rule (base '
           'set-up, fit)', res['ebase'], lambda s: (s[3], s[7]))
    counts('indexed operands through A0-A4 in a loop or not (base set-up, place)',
           res['ebase'], lambda s: (s[3], s[8]))


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
