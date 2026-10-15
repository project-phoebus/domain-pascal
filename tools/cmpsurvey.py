#!/usr/bin/env python3
"""cmpsurvey.py - comparisons and byte indices in the compiled code of the
SR10.2 image.

Evidence tool for docs/codegen-patterns.md sections 7.1 and 5.2 (beads
domain-pascal-aaj, -ji9).  Compiled routines are split and followed as
tools/casesurvey.py does (map symbols and `link.w A6' words, hand assembly
excluded, only the instructions reachable from the routine's start, in
address order).  Kinds:

    bidx    an indexed operand `(d,An,Dx.w|l)' whose index register was
            last written by a byte load `move.b X,Dx' (scaling `add.w
            Dx,Dx', `lsl #k', `mulu.w #c' in between skipped): how the
            byte was widened (`clr.w Dx' / `clr.l Dx' before the load,
            adjacent or with instructions between; `andi.w #255' after;
            `ext.w' after; none), the index size, the scale, and whether
            the byte came from a data register or from memory.
    mem2    `move.<s> M1,Dn; cmp.<s> M2,Dn' with M1, M2 memory: two
            variables compared, the first one loaded; the branch after.
            `both' counts `move M1,Dn; move M2,Dm; cmp Dm,Dn' (both
            loaded) for comparison.
    widen   `cmp.l Dm,Dn' where both registers were widened from words
            in the preceding instructions (`ext.l' after a word move:
            sign; `clr.l' then a word move: zero; `andi.l #0xffff':
            zero): the kinds of the two sides (destination first) and
            the branch.  `widen1' is `cmp.l <ea>,Dn' with only Dn widened
            (a word against a longword).
    cmpm    runs of `cmpm.<s> (Ay)+,(Ax)+' (records compared in memory):
            the size, the length of the run, the count (`moveq #k,Dn'
            among the four instructions before it), whether a `db<cc>'
            closes a loop, and the branches after each.
    cmpb    `cmp.b' and `cmpi.b' compares (char and byte values): the
            source kind; `cmpi.b #-1' (0xFF) separately.
    ord     word and longword compares (`cmp', `cmpi', `cmpa', `tst')
            followed directly by an ordered branch: the instruction, the
            source kind (imm, reg, mem) and whether the branch is signed
            (`blt' `ble' `bgt' `bge': S) or unsigned (`bcs' `bls' `bhi'
            `bcc': U).  Evidence for the sign of subranges
            (codegen-patterns 5; bead domain-pascal-5j9).
    callcmp the first instruction after a call (its pops and an A5
            restore skipped) when it compares D0, the call's result:
            `tst.<s> D0', `cmpi.<s> #c,D0', `cmp.<s> X,D0' (D0 the
            destination) or `cmp.<s> D0,X' (the source); the other
            operand's kind (zero, imm, reg, a memory operand by base:
            An, A5, A6, abs); and its use: the branch that follows
            (`b<cc>'), or `s<cc> Dn' then `tst.b Dn' and a branch
            (`s<cc>>tst', with Dn), or `s<cc> Dn' otherwise.  Evidence
            for comparisons of a function's result (codegen-patterns 7.1;
            bead domain-pascal-i1by).
    andor   `and.b'/`or.b' of two data registers or of memory into Dn:
            whether a call lies between the last write of Dn (the left
            operand formed) and the combination, i.e. whether the right
            operand calls (bead domain-pascal-1rwo).
    opflags a conditional branch or `s<cc>' whose tested value a data
            register operation just formed (bead domain-pascal-qlzu):
            `op Dn; b<cc>' (form `flags', the operation's own flags),
            `op Dn; tst.<s> Dn; b<cc>' (`tst'), `op Dn; cmpi.<s> #0,Dn;
            b<cc>' (`cmpi0'), `op Dn; move.<s> Dn,<mem>; b<cc>'
            (`store') and `op Dn; move.<s> Dn,<mem>; tst ...; b<cc>'
            (`store+tst').  `op' is an add, sub, and, or, eor (register,
            immediate or quick forms), a shift or rotate, neg, not, clr,
            ext, swap, mul, div, a move or moveq into Dn; the size of the
            operation and of the test, the branch's condition, whether it
            is a branch or an `s<cc>', and whether the tested instruction
            is a branch target (a join: the test cannot rely on the
            operation's flags there).  Evidence for comparisons of a
            computed value with 0 (codegen-patterns 7.1).

usage:
    tools/cmpsurvey.py summary          # the counts quoted in codegen-patterns
    tools/cmpsurvey.py list KIND        # one line per site of KIND

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import a5survey  # noqa: E402
import arithsurvey  # noqa: E402

KINDS = ('bidx', 'mem2', 'both', 'widen', 'widen1', 'cmpm', 'cmpb', 'ord', 'callcmp',
         'andor', 'opflags')

_IDX = re.compile(r'%a\d@\((-?\d+),%(d\d):([wl])\)')
_SCALE = re.compile(r'^(?:add([wl]) %(d\d),%(d\d)|ls[l]([wl]) #(\d),%(d\d)'
                    r'|mul[su]w #(\d+),%(d\d))$')
_MEM = r'(?:%a\d@[^,]*|%fp@\([^)]*\)|%a\d@\([^)]*\)|0x[0-9a-f]+|%pc@\([^)]*\))'
_BCC = re.compile(r'^(b[a-z]{2})[sw]? ')


def writes(t, d):
    """Whether instruction text `t' writes register `d' (approximately:
    `d' is its last operand, or it is a one-operand instruction on it)."""
    if re.search(r',%' + d + r'$', t) or re.match(r'^\w+ %' + d + r'$', t):
        return not t.startswith(('cmp', 'tst', 'btst'))
    return False


def bidx_sites(ins):
    """[(addr, widening, index size, scale, source)] for byte indices."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = _IDX.search(t)
        if not m:
            continue
        dx, isz = m.group(2), m.group(3)
        scale = 1
        j = i - 1
        while j >= 0 and i - j <= 8:
            tj = ins[j][2]
            s = _SCALE.match(tj)
            if s and (s.group(2) == dx == s.group(3) or s.group(6) == dx or s.group(8) == dx):
                if s.group(1):
                    scale *= 2
                elif s.group(5):
                    scale <<= int(s.group(5))
                else:
                    scale *= int(s.group(7))
                j -= 1
                continue
            if writes(tj, dx) or tj.startswith(('ext', 'andi')) and tj.endswith('%' + dx):
                break
            j -= 1
        if j < 0 or i - j > 8:
            continue
        tj = ins[j][2]
        wid = None
        k = j
        if re.match(r'^andiw #255,%' + dx + '$', tj):
            wid, k = 'andi.w', j - 1
        elif re.match(r'^extw %' + dx + '$', tj):
            wid, k = 'ext.w', j - 1
        if k < 0:
            continue
        mb = re.match(r'^moveb (.*),%' + dx + '$', ins[k][2])
        if not mb:
            continue
        src = 'reg' if re.match(r'^%d\d$', mb.group(1)) else 'mem'
        if wid is None:
            wid = 'none'
            for q in range(k - 1, max(k - 6, -1), -1):
                tq = ins[q][2]
                c = re.match(r'^clr([wl]) %' + dx + '$', tq)
                if c:
                    wid = 'clr.%s%s' % (c.group(1), '' if q == k - 1 else '(gap)')
                    break
                if writes(tq, dx) or _BCC.match(tq) or tq.startswith(('bra', 'jsr', 'bsr')):
                    break
        out.append((a, wid, isz, scale, src))
    return out


def mem2_sites(ins):
    """([(addr, size, branch)] of `move M1,Dn; cmp M2,Dn',
        [(addr, size, branch)] of `move M1,Dn; move M2,Dm; cmp Dm,Dn')."""
    one, both = [], []
    for i in range(1, len(ins)):
        t = ins[i][2]
        m = re.match(r'^cmp([bwl]) (' + _MEM + r'),%(d\d)$', t)
        br = _BCC.match(ins[i + 1][2]).group(1) if i + 1 < len(ins) and _BCC.match(
            ins[i + 1][2]) else '-'
        if m:
            s, dn = m.group(1), m.group(3)
            p = re.match(r'^move' + s + r' (' + _MEM + r'),%' + dn + '$', ins[i - 1][2])
            if p:
                one.append((ins[i - 1][0], s, br))
            continue
        m = re.match(r'^cmp([bwl]) %(d\d),%(d\d)$', t)
        if m and i >= 2:
            s, dm, dn = m.groups()
            l1 = re.match(r'^move' + s + r' (' + _MEM + r'),%(d\d)$', ins[i - 2][2])
            l2 = re.match(r'^move' + s + r' (' + _MEM + r'),%(d\d)$', ins[i - 1][2])
            if l1 and l2 and {l1.group(2), l2.group(2)} == {dm, dn}:
                both.append((ins[i - 2][0], s, br))
    return one, both


def widened(ins, i, d):
    """How register `d' was last written before ins[i], within 6
    instructions: 'sign', 'zero', 'long' (another write) or None."""
    for j in range(i - 1, max(i - 7, -1), -1):
        t = ins[j][2]
        if t == 'extl %' + d:
            return 'sign'
        if t == 'andil #65535,%' + d:
            return 'zero'
        if re.match(r'^movew .*,%' + d + '$', t):
            for q in range(j - 1, max(j - 5, -1), -1):
                if ins[q][2] == 'clrl %' + d:
                    return 'zero'
                if writes(ins[q][2], d):
                    break
            return None
        if writes(t, d):
            return 'long'
    return None


def widen_sites(ins):
    """([(addr, dst kind, src kind, branch)] of `cmp.l Dm,Dn' with both
    widened, [(addr, dst kind, src, branch)] of `cmp.l <ea>,Dn' with Dn
    widened only)."""
    two, one = [], []
    for i, (a, _, t) in enumerate(ins):
        br = _BCC.match(ins[i + 1][2]).group(1) if i + 1 < len(ins) and _BCC.match(
            ins[i + 1][2]) else '-'
        m = re.match(r'^cmpl %(d\d),%(d\d)$', t)
        if m:
            ks, kd = widened(ins, i, m.group(1)), widened(ins, i, m.group(2))
            if ks in ('sign', 'zero') and kd in ('sign', 'zero'):
                two.append((a, kd, ks, br))
                continue
            if kd in ('sign', 'zero') and ks == 'long':
                one.append((a, kd, 'reg', br))
            continue
        m = re.match(r'^cmpl (' + _MEM + r'),%(d\d)$', t)
        if m:
            kd = widened(ins, i, m.group(2))
            if kd in ('sign', 'zero'):
                one.append((a, kd, 'mem', br))
    return two, one


def cmpm_sites(ins):
    """[(addr, size, run length, count, loop, branches)] of cmpm runs;
    count is the `moveq #k' among the four instructions before the run
    (`k', the register left out), or '-'."""
    out = []
    i = 0
    while i < len(ins):
        m = re.match(r'^cmpm([bwl]) ', ins[i][2])
        if not m:
            i += 1
            continue
        a, s, n, brs = ins[i][0], m.group(1), 0, []
        count = '-'
        for j in range(i - 1, max(i - 5, -1), -1):
            q = re.match(r'^moveq #(-?\d+),%d\d$', ins[j][2])
            if q:
                count = '#' + q.group(1)
                break
        loop = False
        while i < len(ins) and re.match(r'^cmpm' + s + ' ', ins[i][2]):
            n += 1
            i += 1
            if i < len(ins):
                b = re.match(r'^(b[a-z]{2}|db[a-z]{1,2})[sw]? ', ins[i][2])
                if b:
                    brs.append(b.group(1))
                    loop = loop or b.group(1).startswith('db')
                    i += 1
        out.append((a, s, n, count, loop, '/'.join(brs)))
    return out


def cmpb_sites(ins):
    """[(addr, form)] of byte compares."""
    out = []
    for a, _, t in ins:
        if re.match(r'^cmpib #-1,', t):
            out.append((a, 'cmpi.b #-1'))
        elif re.match(r'^cmpib ', t):
            out.append((a, 'cmpi.b'))
        elif re.match(r'^cmpb %d\d,', t):
            out.append((a, 'cmp.b reg'))
        elif re.match(r'^cmpb ', t):
            out.append((a, 'cmp.b mem'))
    return out


_ORD = re.compile(r'^(cmpl|cmpil|cmpal|cmpw|cmpiw|cmpaw|tstl|tstw) (.*)$')
_SIGNED = ('blt', 'ble', 'bgt', 'bge')
_UNSIGNED = ('bcs', 'bls', 'bhi', 'bcc')


def ord_sites(ins):
    """[(addr, insn, source kind, S|U)] of word and longword compares
    followed by an ordered branch."""
    out = []
    for i, (a, _, t) in enumerate(ins[:-1]):
        m = _ORD.match(t)
        b = _BCC.match(ins[i + 1][2] + ' ')
        if not m or not b:
            continue
        cls = 'S' if b.group(1) in _SIGNED else 'U' if b.group(1) in _UNSIGNED else None
        if cls is None:
            continue
        src = m.group(2).split(',')[0]
        kind = ('imm' if src.startswith('#') else
                'reg' if re.match(r'^%[da]\d$', src) else 'mem')
        out.append((a, m.group(1), kind, cls))
    return out


_POP = re.compile(r'^(addql #\d,%sp|addqw #\d,%sp|addaw #\d+,%sp|addal #\d+,%sp'
                  r'|lea %sp@\(\d+\),%sp|moveal %fp@\(-?\d+\),%a5)$')
_CALL = re.compile(r'^(jsr|bsr[sw]?) ')


def operand_kind(x):
    """The kind of a compare's other operand."""
    if x.startswith('#'):
        return 'imm'
    if re.match(r'^%[da]\d$', x):
        return 'reg'
    m = re.match(r'^%(a\d|fp|pc)@', x)
    if m:
        return {'a5': 'mem-A5', 'fp': 'mem-A6', 'pc': 'mem-PC'}.get(m.group(1), 'mem-An')
    return 'mem-abs'


def callcmp_sites(ins):
    """[(addr, insn, D0 side, other kind, use, s<cc> register)] of
    compares of a call's D0 right after the call.  Calls of M$ARITH
    entries are left out: they compute `*', `div' and `mod'
    (codegen-patterns 11), not a function's result."""
    out = []
    for i, (_, _, t) in enumerate(ins):
        if not _CALL.match(t):
            continue
        m = re.match(r'^jsr 0x([0-9a-f]+)$', t)
        if m and int(m.group(1), 16) in arithsurvey.ENTRIES:
            continue
        j = i + 1
        while j < len(ins) and _POP.match(ins[j][2]):
            j += 1
        if j >= len(ins):
            continue
        u = ins[j][2]
        m = re.match(r'^tst([bwl]) %d0$', u)
        if m:
            k = ('tst.' + m.group(1), 'dst', 'zero')
        else:
            m = re.match(r'^(cmpi?)([bwl]) (.*),%d0$', u)
            if m:
                k = (m.group(1) + '.' + m.group(2), 'dst', operand_kind(m.group(3)))
            else:
                m = re.match(r'^cmp([bwl]) %d0,(.*)$', u)
                if not m:
                    continue
                k = ('cmp.' + m.group(1), 'src', operand_kind(m.group(2)))
        nxt = ins[j + 1][2] if j + 1 < len(ins) else ''
        b = _BCC.match(nxt + ' ')
        sc = re.match(r'^s([a-z]{1,2}) %(d\d)$', nxt)
        if b:
            use, reg = b.group(1), '-'
        elif sc and sc.group(1) not in ('ub', 'ubq', 'ubx'):
            reg = sc.group(2)
            after = [x[2] for x in ins[j + 2:j + 4]]
            tested = (len(after) == 2 and after[0] == 'tstb %' + reg
                      and _BCC.match(after[1] + ' '))
            use = 's<cc>>tst' if tested else 's<cc>'
        else:
            use, reg = 'other', '-'
        out.append((ins[j][0],) + k + (use, reg))
    return out


def andor_sites(ins):
    """[(addr, op, right operand calls: 'call'|'-')] of `and.b'/`or.b'
    into a data register."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = re.match(r'^(and|or)b (.*),%(d\d)$', t)
        if not m or m.group(2).startswith('#'):
            continue
        dn, call = m.group(3), '-'
        for j in range(i - 1, -1, -1):
            tj = ins[j][2]
            if _CALL.match(tj):
                if dn in ('d0', 'd1'):
                    break   # the call itself writes Dn: a function result
                call = 'call'
                continue
            if writes(tj, dn):
                break
        out.append((a, m.group(1), call))
    return out


# A data register operation that sets the flags from its result: (op
# family, size, register).  objdump spellings: `addqw', `andil', `lsrl',
# `extl', `moveq'.
_OPDN = re.compile(r'^(add|sub|and|or|eor|lsl|lsr|asl|asr|rol|ror|roxl|roxr|neg|negx|not'
                   r'|clr|ext|move|mul[su]|div[su])(q|i|x)?([bwl])? (?:.*,)?%(d\d)$')
_SWAP = re.compile(r'^swap %(d\d)$')
_MOVEQ = re.compile(r'^moveq #-?\d+,%(d\d)$')
_CONDS = ('eq', 'ne', 'mi', 'pl', 'lt', 'ge', 'le', 'gt', 'cs', 'cc', 'hi', 'ls', 'vs', 'vc')
_COND = re.compile(r'^(?:b(' + '|'.join(_CONDS) + r')[sw]? 0x|s(' + '|'.join(_CONDS)
                   + r') %d\d$)')
_STORE = re.compile(r'^move([bwl]) %(d\d),(' + _MEM + r')$')


def op_of(t):
    """(family, size, register) of a flag-setting data register
    operation, or None.  `mulu.w'/`muls.w' and `divu.w'/`divs.w' are word
    operations whose register is a longword; moveq is a longword."""
    m = _MOVEQ.match(t)
    if m:
        return ('moveq', 'l', m.group(1))
    m = _SWAP.match(t)
    if m:
        return ('swap', 'l', m.group(1))
    m = _OPDN.match(t)
    if not m or (m.group(1) == 'move' and m.group(2)):
        return None
    fam, size = m.group(1), m.group(3)
    if fam.startswith(('mul', 'div')):
        return (fam, 'w', m.group(4))
    if size is None:
        return None
    return (fam, size, m.group(4))


# SMD_UTILS, left out of `opflags': its code saves registers with `movem'
# before the `link' and takes A5 from A0 (smd_$write_str_clip_impl
# 0xE70390), SMD_$INVERT_DISP 0xE70376 is frameless with SP-relative
# arguments: hand assembly by codegen-patterns section 0's signs, though
# loopalign.HAND_ASM does not list it.
_OPFLAGS_SKIP = (0xE702F4, 0xE707FC)

_REL = {'eq': 'eq/ne', 'ne': 'eq/ne', 'mi': 'mi/pl', 'pl': 'mi/pl', 'lt': 'signed', 'ge': 'signed',
        'le': 'signed', 'gt': 'signed', 'cs': 'unsigned', 'cc': 'unsigned', 'hi': 'unsigned',
        'ls': 'unsigned', 'vs': 'overflow', 'vc': 'overflow'}


def opflags_class(site):
    """(operation family, relation class, form, same size|size differs,
    join) of an `opflags' site (as opflags_sites gives it)."""
    _, fam, size, form, tsize, cond, _, join = site
    return (fam, _REL[cond], form, 'same' if size == tsize else 'size', join)


def opflags_sites(ins):
    """[(addr of the operation, family, op size, form, test size, cond,
    b|s, join)] of conditional branches and `s<cc>' after a data register
    operation (see the module docstring, kind `opflags').  The
    instructions must be adjacent in memory."""
    targets = set()
    for _, _, t in ins:
        m = re.match(r'^b[a-z]{2,3}[sw]? 0x([0-9a-f]+)$', t)
        if m:
            targets.add(int(m.group(1), 16))
    out = []

    def adjacent(i, j):
        return all(ins[k][0] + ins[k][1] == ins[k + 1][0] for k in range(i, j))

    for i, (a, _, t) in enumerate(ins):
        op = op_of(t)
        if op is None:
            continue
        fam, size, d = op
        nxt = [x[2] for x in ins[i + 1:i + 4]]
        if not nxt:
            continue

        def cond(k):
            if i + k >= len(ins) or not adjacent(i, i + k):
                return None
            m = _COND.match(ins[i + k][2])
            if not m:
                return None
            return (m.group(1) or m.group(2), 'b' if m.group(1) else 's')

        def join(k):
            return 'join' if any(ins[i + q][0] in targets for q in range(1, k + 1)) else '-'

        c = cond(1)
        if c:
            out.append((a, fam, size, 'flags', size) + c + (join(1),))
            continue
        t1 = nxt[0]
        m = re.match(r'^tst([bwl]) %' + d + '$', t1)
        if m and cond(2):
            out.append((a, fam, size, 'tst', m.group(1)) + cond(2) + (join(1),))
            continue
        m = re.match(r'^cmpi([bwl]) #0,%' + d + '$', t1)
        if m and cond(2):
            out.append((a, fam, size, 'cmpi0', m.group(1)) + cond(2) + (join(1),))
            continue
        m = _STORE.match(t1)
        if m and m.group(2) == d:
            if cond(2):
                out.append((a, fam, size, 'store', m.group(1)) + cond(2) + (join(1),))
                continue
            if len(nxt) > 1:
                q = re.match(r'^tst([bwl]) (.*)$', nxt[1])
                if q and q.group(2) in ('%' + d, m.group(3)) and cond(3):
                    out.append((a, fam, size, 'store+tst', q.group(1)) + cond(3)
                               + (join(2),))
    return out


def scan():
    res = {k: [] for k in KINDS}
    for r0, r1, name in arithsurvey.routines():
        ins = a5survey.reached(r0, r1)
        res['bidx'] += [(r0, name) + s for s in bidx_sites(ins)]
        one, both = mem2_sites(ins)
        res['mem2'] += [(r0, name) + s for s in one]
        res['both'] += [(r0, name) + s for s in both]
        two, one = widen_sites(ins)
        res['widen'] += [(r0, name) + s for s in two]
        res['widen1'] += [(r0, name) + s for s in one]
        res['cmpm'] += [(r0, name) + s for s in cmpm_sites(ins)]
        res['cmpb'] += [(r0, name) + s for s in cmpb_sites(ins)]
        res['ord'] += [(r0, name) + s for s in ord_sites(ins)]
        res['callcmp'] += [(r0, name) + s for s in callcmp_sites(ins)]
        res['andor'] += [(r0, name) + s for s in andor_sites(ins)]
        if not _OPFLAGS_SKIP[0] <= r0 < _OPFLAGS_SKIP[1]:
            res['opflags'] += [(r0, name) + s for s in opflags_sites(ins)]
    return res


def first(sites, n=4):
    return ', '.join('0x%X' % s[2] for s in sites[:n])


def counts(title, sites, key):
    print('%s: %d' % (title, len(sites)))
    for k, v in sorted(Counter(key(s) for s in sites).items(), key=lambda kv: -kv[1]):
        print('  %-40s %4d  e.g. %s' % (' '.join(str(x) for x in k), v,
                                        first([s for s in sites if key(s) == k])))


def summary(res):
    counts('byte indices (widening, index size, scale, source)', res['bidx'],
           lambda s: (s[3], s[4], s[5], s[6]))
    counts('`move M1,Dn; cmp M2,Dn\' (size, branch)', res['mem2'], lambda s: (s[3], s[4]))
    counts('`move M1,Dn; move M2,Dm; cmp Dm,Dn\' (size, branch)', res['both'],
           lambda s: (s[3], s[4]))
    counts('`cmp.l Dm,Dn\' both widened (Dn, Dm, branch)', res['widen'],
           lambda s: (s[3], s[4], s[5]))
    counts('`cmp.l <ea>,Dn\' Dn widened only (Dn, ea, branch)', res['widen1'],
           lambda s: (s[3], s[4], s[5]))
    counts('cmpm runs (size, run, count, loop, branches)', res['cmpm'],
           lambda s: (s[3], s[4], s[5], s[6], s[7]))
    counts('byte compares', res['cmpb'], lambda s: (s[3],))
    counts('ordered branches after word/longword compares (insn, source, sign)',
           res['ord'], lambda s: (s[3], s[4], s[5]))
    counts('compares of a call\'s D0 (insn, D0 side, other operand, use)', res['callcmp'],
           lambda s: (s[3], s[4], s[5], s[6]))
    counts('  ... formed as a boolean and tested (other operand, s<cc> register)',
           [s for s in res['callcmp'] if s[6] == 's<cc>>tst'], lambda s: (s[5], s[7]))
    counts('and.b/or.b into Dn (op, right operand calls)', res['andor'], lambda s: (s[3], s[4]))
    counts('branches after a data register operation (op, size, form, test size, cond, b|s, '
           'join)', res['opflags'], lambda s: s[3:])
    counts('  ... by operation, relation, form, sizes and join', res['opflags'],
           lambda s: opflags_class(s[2:]))


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
