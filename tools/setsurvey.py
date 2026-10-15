#!/usr/bin/env python3
"""setsurvey.py - set operations in the compiled code of the SR10.2 image.

Evidence tool for docs/codegen-patterns.md section 5.1 (bead
domain-pascal-7qo).  Compiled routines are split and followed as
tools/casesurvey.py does (map symbols and `link.w A6' words, hand assembly
excluded, only the instructions reachable from the routine's start).
Sets of one word or one longword are integers to the 68010, so most of
their code cannot be told from integer `&'/`!' code; the forms counted
here are the ones that only a set (or a packed boolean) gives:

    one     `[k]' with a run-time member: `clr.<s> Dn', then `bset Dk,Dn'
            (the element loaded in between or before), and what the
            register is used for next: `or Dn,<mem>' (s := s + [k]),
            `not Dn; and Dn,<mem>' (s := s - [k]), a push, other.
    memb    `bset'/`bclr'/`btst Dk,(0,An,Dx.w)' after the byte offset of
            member k in a set of more than a longword, `moveq/move.w
            #bits-1,Dx; sub.w Dk,Dx; lsr.w #3,Dx': the operation, the
            bits, whether a `bcs' guards the offset (the element's type
            reaches beyond the set), whether the set is a constant in the
            code (`lea (d,PC)').
    diff    `move <y>,Dn; not Dn; and <x>,Dn' followed by `beq'/`bne':
            `x - y <> []' (or the integer `x & ~y <> 0').
    loop    `dbf' loops whose body `and's or `or's longwords through
            `(An)+': operations on sets in memory.
    regmem  `moveq/move #c,Dn; and/or Dn,<mem>': a constant `and'ed or
            `or'ed into memory through a register (against `andi'/`ori'
            to memory, counted too).
    andc    `&' of a value held in a data register and a constant: the
            copy then `andi' (`move Dm,Dn; andi #c,Dn') against the
            constant first (`moveq/move #c,Dn; and Dm,Dn'), by whether
            `moveq' can load the constant.
    ctest   `btst #k' with a constant bit (codegen-patterns 5.1, "`in' on
            sets in memory, constant member"; bead domain-pascal-578): on
            memory (`mem', always a byte: the address's parity, the base
            kind), or on a data register loaded from memory by the last
            instruction that wrote it, within 5 instructions (`load': the
            load's width, the base kind, and whether the low-order part
            holding bit k at the same number is narrower than the load,
            `whole', or not, `fits').  The `whole' loads are the tests
            that do not read only that part: in the image they are on
            fields tested so in the whole routine, read by `lsr.b #1' for
            bit 0 elsewhere, and never on an indexed operand's byte.
            Also counted: `btst #k' for k = 7, 15, 31 (none: the sign is
            tested with `tst'), and the routines that test one field in
            both forms.
    onebit  every one-bit test in compiled code (bead domain-pascal-3xwx;
            codegen-patterns 7.1 "One bit", 8 `odd'): `btst #k' on a data
            register, by how the register was last written before it in
            the block (`held': not written since a label or a branch,
            call or return, a register-held variable or a value kept
            from before; `load.<s>/whole' or `/fits': loaded from memory
            at size s, more than or just the part holding bit k, as in
            ctest; `copy'; `op:<mnemonic>': computed); `btst #k' on memory
            (the byte's address parity); `lsr.<s> #1,Dn' followed by a
            carry branch or `scs'/`scc' (bit 0 through the carry); `tst.b
            Dn' right after a word or longword load into Dn, followed by
            a sign branch or `s<cc>' (bit 7 of the loaded value); an `and'
            with a one-bit mask (`andi #c,Dn', or `moveq/move #c,Dn; and
            <ea>,Dn') followed by a branch or an `s<cc>' (none).  For each:
            k, what reads the flags (`branch', `scc', other), and for an
            `s<cc>' whether it writes the tested register (`same') or
            another (`new'); whether the tested register is read again
            after the test (`reused').  Also: `btst #7', `#15' or `#31'
            (none).

usage:
    tools/setsurvey.py summary          # the counts quoted in codegen-patterns 5.1
    tools/setsurvey.py list KIND        # one line per site of KIND

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import a5survey  # noqa: E402
import arithsurvey  # noqa: E402

KINDS = ('one', 'memb', 'diff', 'loop', 'regmem', 'andc', 'ctest', 'onebit')

_CLR = re.compile(r'^clr([wl]) %(d\d)$')
_BITREG = re.compile(r'^b(set|clr|tst) %(d\d),%(d\d)$')
_BITIDX = re.compile(r'^b(set|clr|tst) %(d\d),%a\d@\(0,%(d\d):w\)$')
_TOP = re.compile(r'^(?:moveq|movew) #(\d+),%(d\d)$')
_IMMREG = re.compile(r'^(?:moveq|move[bwl]) #(-?\d+),%(d\d)$')


def fits_moveq(v):
    return -128 <= v <= 127


def one_sites(ins):
    """`clr; [load]; bset Dk,Dn' -> [(addr, width, use)]."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = _BITREG.match(t)
        if not m or m.group(1) != 'set':
            continue
        dn = m.group(3)
        width = None
        for j in range(i - 1, max(i - 4, -1), -1):
            c = _CLR.match(ins[j][2])
            if c and c.group(2) == dn:
                width = c.group(1)
                break
            if re.search(r'%' + dn + r'\b', ins[j][2]):
                break
        if width is None:
            continue
        use = 'other'
        nxt = [x[2] for x in ins[i + 1:i + 3]]
        if nxt and re.match(r'^or[wl] %' + dn + r',%(a\d|fp)', nxt[0]):
            use = 'or-mem'
        elif nxt and re.match(r'^or[wl] %' + dn + r',', nxt[0]):
            use = 'or-other'
        elif (len(nxt) == 2 and nxt[0] == 'not%s %%%s' % (width, dn)
              and re.match(r'^and[wl] %' + dn + r',%(a\d|fp)', nxt[1])):
            use = 'not-and-mem'
        elif nxt and re.match(r'^move[wl] %' + dn + r',%sp@-', nxt[0]):
            use = 'push'
        out.append((a, width, use))
    return out


def memb_sites(ins):
    """`#bits-1; sub.w; [bcs]; lsr.w #3; b<op> Dk,(0,An,Dx.w)' ->
    [(addr, op, bits, guarded, const_set)]."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = _BITIDX.match(t)
        if not m:
            continue
        dk, dx = m.group(2), m.group(3)
        bits = guarded = None
        const = False
        for j in range(i - 1, max(i - 10, -1), -1):
            tj = ins[j][2]
            if re.match(r'^lea %pc@', tj):
                const = True
            if re.match(r'^bcss? ', tj):
                guarded = True
            tm = _TOP.match(tj)
            if tm and tm.group(2) == dx:
                bits = int(tm.group(1)) + 1
                break
        if bits is None or not any(
                ins[k][2] == 'subw %%%s,%%%s' % (dk, dx) for k in range(max(i - 8, 0), i)):
            continue
        out.append((a, m.group(1), bits, bool(guarded), const))
    return out


def diff_sites(ins):
    """`move <y>,Dn; not Dn; and <x>,Dn; beq/bne' -> [addr]."""
    out = []
    for i in range(1, len(ins) - 2):
        m = re.match(r'^not([wl]) %(d\d)$', ins[i][2])
        if not m:
            continue
        s, dn = m.groups()
        if (re.match(r'^move' + s + r' .*,%' + dn + '$', ins[i - 1][2])
                and re.match(r'^and' + s + r' [^,]+,%' + dn + '$', ins[i + 1][2])
                and re.match(r'^b(eq|ne)', ins[i + 2][2])):
            out.append(ins[i - 1][0])
    return out


def loop_sites(ins):
    """`dbf' loops with an `and.l'/`or.l' through `(An)+' in the body."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = re.match(r'^dbf %d\d,0x([0-9a-f]+)$', t)
        if not m:
            continue
        head = int(m.group(1), 16)
        body = [x[2] for x in ins if head <= x[0] < a]
        if any(re.match(r'^(and|or)l (%a\d@\+,%d\d|%d\d,%a\d@\+)', b) for b in body):
            out.append(head)
    return out


def regmem_sites(ins):
    """(`#c,Dn; and/or Dn,<mem>' sites, `andi/ori #c,<mem>' sites)."""
    reg, imm = [], []
    for i, (a, _, t) in enumerate(ins):
        if re.match(r'^(and|or)i[bwl] #-?\d+,(?!%d|%sr)', t):
            imm.append(a)
        m = re.match(r'^(and|or)[bwl] %(d\d),(?!%d)', t)
        if m and i and _IMMREG.match(ins[i - 1][2]) \
                and _IMMREG.match(ins[i - 1][2]).group(2) == m.group(2):
            reg.append(ins[i - 1][0])
    return reg, imm


def andc_sites(ins):
    """[(addr, form, moveq-able)] for `&' of a held value and a constant."""
    out = []
    for i in range(1, len(ins)):
        p, t = ins[i - 1][2], ins[i][2]
        m = re.match(r'^andi([bwl]) #(-?\d+),%(d\d)$', t)
        if m and re.match(r'^move' + m.group(1) + r' %d\d,%' + m.group(3) + '$', p):
            out.append((ins[i - 1][0], 'copy+andi', fits_moveq(int(m.group(2)))))
        m = re.match(r'^and[bwl] %d\d,%(d\d)$', t)
        c = _IMMREG.match(p)
        if m and c and c.group(2) == m.group(1):
            out.append((ins[i - 1][0], 'const-first', fits_moveq(int(c.group(1)))))
    return out


_BTSTK = re.compile(r'^btst #(\d+),(.+)$')
_LOADD = re.compile(r'^move([bwl]) (.+),%(d\d)$')
_SIZE = {'b': 1, 'w': 2, 'l': 4}


def operand_kind(o):
    """(base kind, displacement or None) of a memory operand's text."""
    if ':' in o:
        return 'idx', None
    m = re.match(r'^%(a\d|fp)@(?:\((-?\d+)\))?$', o)
    if m:
        kind = {'fp': 'fp', 'a5': 'a5'}.get(m.group(1), 'An')
        return kind, int(m.group(2) or 0)
    m = re.match(r'^0x([0-9a-f]+)$', o)
    if m:
        return 'abs', int(m.group(1), 16)
    return 'other', None


def ctest_sites(ins):
    """`btst #k' on memory or on a register just loaded from memory ->
    [(addr, form, k, width, base kind, class, operand)]; class is the
    parity (`odd'/`even') of a memory byte, `whole'/`fits' for a load
    (above)."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = _BTSTK.match(t)
        if not m:
            continue
        k, opnd = int(m.group(1)), m.group(2)
        if not re.match(r'^%d\d$', opnd):
            if opnd.startswith('%a') or opnd.startswith('%fp') or opnd.startswith('0x'):
                kind, d = operand_kind(opnd)
                cls = '?' if d is None else ('odd' if d % 2 else 'even')
                out.append((a, 'mem', k, 'b', kind, cls, opnd))
            continue
        dn = opnd[1:]
        for j in range(i - 1, max(i - 6, -1), -1):
            tj = ins[j][2]
            if re.search(r',%' + dn + '$', tj):
                lm = _LOADD.match(tj)
                src = lm.group(2) if lm else ''
                if lm and ('@' in src or src.startswith('0x')):
                    w = lm.group(1)
                    kind, _ = operand_kind(src)
                    narrower = (w == 'w' and k < 8) or (w == 'l' and k < 16)
                    out.append((a, 'load', k, w, kind, 'whole' if narrower else 'fits', src))
                break
            if re.search(r'%' + dn + r'\b', tj) or re.match(r'^(b[a-z]+s? 0x|bra|jmp|rts|dbf)', tj):
                break
    return out


_SCCX = re.compile(r'^s(hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le) (\S+)$')
_BRX = re.compile(r'^b(hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le)[sw]? ')
_STOP = re.compile(r'^(b[a-z]{2}[sw]? |bra|jsr|bsr|jmp|rts|db[a-z]+ |unlk)')


def _writes(t, d):
    """Whether instruction text t writes %d (its last operand; `btst',
    `cmp' and `tst' only read it)."""
    if t.startswith(('btst', 'cmp', 'tst')):
        return False
    return re.search(r',%' + d + r'$', t) is not None or re.match(
        r'^[a-z]+ %' + d + r'$', t) is not None


def _source(ins, i, d, labels, k=None):
    """How %d, tested at ins[i], was last written in its block."""
    for j in range(i - 1, max(-1, i - 8), -1):
        t = ins[j][2]
        if _STOP.match(t):
            return 'held'
        if _writes(t, d):
            m = _LOADD.match(t)
            if m and m.group(3) == d:
                src = m.group(2)
                if '@' in src or src.startswith('0x'):
                    w = m.group(1)
                    if k is None:
                        return 'load.' + w
                    narrower = (w == 'w' and k < 8) or (w == 'l' and k < 16)
                    return 'load.%s/%s' % (w, 'whole' if narrower else 'fits')
                if re.match(r'^%d\d$', src):
                    return 'copy'
            return 'op:' + t.split()[0]
        if ins[j][0] in labels:
            return 'held'
    return 'held'


def _reader(ins, i, d):
    """(what reads the flags of ins[i], `same'/`new' for an s<cc>)."""
    t = ins[i + 1][2] if i + 1 < len(ins) else ''
    if _BRX.match(t):
        return 'branch', ''
    m = _SCCX.match(t)
    if m:
        return 'scc', 'same' if m.group(2) == '%' + d else 'new'
    return 'other', ''


def _reused(ins, i, d, labels):
    """Whether %d is read again after ins[i] in its block."""
    for j in range(i + 1, min(len(ins), i + 12)):
        t = ins[j][2]
        if j > i + 1 and ins[j][0] in labels:
            return False
        if re.search(r'%' + d + r'\b', t):
            return not (_writes(t, d) and t.count('%' + d) == 1
                        and re.match(r'^(move[bwl]?|moveq|clr[bwl]|s[a-z]{2}) ', t))
        if re.match(r'^(bra|jsr|bsr|jmp|rts|unlk)', t):
            return False
    return False


def onebit_sites(ins, labels=frozenset()):
    """Every one-bit test (KINDS `onebit') -> [(addr, form, k, source,
    reader, scc register, reused)]."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = _BTSTK.match(t)
        if m:
            k, opnd = int(m.group(1)), m.group(2)
            if re.match(r'^%d\d$', opnd):
                d = opnd[1:]
                rd, reg = _reader(ins, i, d)
                out.append((a, 'btst-reg', k, _source(ins, i, d, labels, k), rd, reg,
                            _reused(ins, i, d, labels)))
            else:
                kind, disp = operand_kind(opnd)
                par = '?' if disp is None else ('odd' if disp % 2 else 'even')
                rd, _ = _reader(ins, i, '')
                out.append((a, 'btst-mem', k, '%s/%s' % (kind, par), rd, '', False))
            continue
        m = re.match(r'^lsr([bwl]) #1,%(d\d)$', t)
        if m and i + 1 < len(ins) and re.match(r'^(b(cc|cs)[sw]? |s(cc|cs) )', ins[i + 1][2]):
            d = m.group(2)
            rd, reg = _reader(ins, i, d)
            out.append((a, 'lsr' + m.group(1) + '#1', 0, _source(ins, i, d, labels), rd, reg,
                        _reused(ins, i, d, labels)))
            continue
        m = re.match(r'^tstb %(d\d)$', t)
        if m and i + 1 < len(ins) and re.match(r'^(b(mi|pl)[sw]? |s(mi|pl) )', ins[i + 1][2]):
            d = m.group(1)
            src = _source(ins, i, d, labels)
            if src in ('load.w', 'load.l'):
                rd, reg = _reader(ins, i, d)
                out.append((a, 'tstb-of-load', 7, src, rd, reg, _reused(ins, i, d, labels)))
            continue
        m = re.match(r'^(?:andi?)([bwl]) (\S+),%(d\d)$', t)
        if m and i + 1 < len(ins):
            w, src, d = m.groups()
            c = None
            if src.startswith('#'):
                c = int(src[1:])
            elif re.match(r'^%d\d$', src) and i > 0:
                im = _IMMREG.match(ins[i - 1][2])
                if im and im.group(2) == src[1:]:
                    c = int(im.group(1))
            elif i > 0:
                im = _IMMREG.match(ins[i - 1][2])
                if im and im.group(2) == d:
                    c = int(im.group(1))
            if c is None:
                continue
            c &= {'b': 0xff, 'w': 0xffff, 'l': 0xffffffff}[w]
            rd, reg = _reader(ins, i, d)
            if c and not c & (c - 1) and rd != 'other':
                out.append((a, 'and-1bit', c.bit_length() - 1, w, rd, reg, False))
    return out


def scan():
    """{kind: [(routine start, routine name, site...)]}."""
    res = {k: [] for k in KINDS}
    res['immmem'] = []
    for r0, r1, name in arithsurvey.routines():
        ins = a5survey.reached(r0, r1)
        for s in one_sites(ins):
            res['one'].append((r0, name) + s)
        for s in memb_sites(ins):
            res['memb'].append((r0, name) + s)
        for a in diff_sites(ins):
            res['diff'].append((r0, name, a))
        for a in loop_sites(ins):
            res['loop'].append((r0, name, a))
        reg, imm = regmem_sites(ins)
        res['regmem'] += [(r0, name, a) for a in reg]
        res['immmem'] += [(r0, name, a) for a in imm]
        for s in andc_sites(ins):
            res['andc'].append((r0, name) + s)
        for s in ctest_sites(ins):
            res['ctest'].append((r0, name) + s)
        labels = frozenset(int(b.group(1), 16) for _, _, t in ins
                           for b in [re.search(r' 0x([0-9a-f]+)$', t)] if b)
        for s in onebit_sites(ins, labels):
            res['onebit'].append((r0, name) + s)
    return res


def first(sites, n=4):
    return ', '.join('0x%X' % s[2] for s in sites[:n])


def summary(res):
    one = res['one']
    print('one-member sets `clr; bset Dk,Dn\': %d' % len(one))
    for k, v in sorted(Counter((s[3], s[4]) for s in one).items()):
        print('  %s %-12s %3d  e.g. %s' % (k[0], k[1], v,
                                           first([s for s in one if (s[3], s[4]) == k])))
    memb = res['memb']
    print('byte-offset member operations (sets over a longword): %d' % len(memb))
    for k, v in sorted(Counter((s[3], s[4], s[5], s[6]) for s in memb).items()):
        sel = [s for s in memb if (s[3], s[4], s[5], s[6]) == k]
        print('  b%-3s bits=%-4d guarded=%-5s const=%-5s %3d  e.g. %s' % (k + (v, first(sel))))
    print('`move y; not; and x; beq/bne\': %d  e.g. %s' % (len(res['diff']), first(res['diff'])))
    print('dbf loops and/or-ing longwords through (An)+: %d  e.g. %s' % (
        len(res['loop']), first(res['loop'], 8)))
    print('constant through a register into memory (`#c,Dn; and/or Dn,<mem>\'): %d  (%s)' % (
        len(res['regmem']), first(res['regmem'], 8)))
    print('`andi/ori #c,<mem>\': %d' % len(res['immmem']))
    andc = res['andc']
    for k, v in sorted(Counter((s[3], s[4]) for s in andc).items()):
        print('  & of a held value: %-11s moveq-able=%-5s %3d  e.g. %s' % (
            k[0], k[1], v, first([s for s in andc if (s[3], s[4]) == k])))
    ctest_summary(res['ctest'])
    onebit_summary(res['onebit'])


def onebit_summary(sites):
    """The counts of the one-bit tests (codegen-patterns 7.1, "One bit")."""
    print('one-bit tests: %d' % len(sites))

    def cls(s):
        src = s[5]
        if s[3] == 'btst-reg' and src.startswith('op:'):
            src = 'op'
        return (s[3], src, s[6] + ('/' + s[7] if s[7] else ''))
    for k, v in sorted(Counter(cls(s) for s in sites).items()):
        sel = [s for s in sites if cls(s) == k]
        print('  %-12s %-16s %-10s %4d  e.g. %s' % (k + (v, first(sel))))
    reused = [s for s in sites if s[3] == 'btst-reg' and s[5].endswith('whole') and s[8]]
    print('  whole loads whose register is read again after the test: %d (%s)' % (
        len(reused), first(reused, 8)))
    sign = [s for s in sites if s[3].startswith('btst') and s[4] in (7, 15, 31)]
    print('  btst #7, #15 or #31: %d' % len(sign))


def ctest_summary(sites):
    """The counts of `btst #k' on memory and after a load."""
    print('constant `btst #k\' on memory or a register loaded from it: %d' % len(sites))
    for k, v in sorted(Counter((s[3], s[5], s[6], s[7]) for s in sites).items()):
        sel = [s for s in sites if (s[3], s[5], s[6], s[7]) == k]
        print('  %-4s %s %-5s %-5s %3d  e.g. %s' % (k + (v, first(sel))))
    sign = [s for s in sites if s[4] in (7, 15, 31)]
    print('  btst #7, #15 or #31: %d' % len(sign))
    # A field: (routine, base kind, the word holding the tested byte).
    forms = {}
    for s in sites:
        kind, d = operand_kind(s[8])
        if d is None or (s[3] == 'load' and s[7] != 'whole'):
            continue
        byte = d if s[3] == 'mem' else d + _SIZE[s[5]] - 1 - s[4] // 8
        forms.setdefault((s[0], kind, byte & ~1), set()).add(s[3])
    both = sorted('0x%X' % f[0] for f, v in forms.items() if len(v) == 2)
    print('  routine fields tested both in memory and by a whole load: %d (%s)' % (
        len(both), ', '.join(both)))


def main(argv):
    if not argv or argv[0] not in ('summary', 'list') or (
            argv[0] == 'list' and (len(argv) < 2 or argv[1] not in KINDS + ('immmem',))):
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
