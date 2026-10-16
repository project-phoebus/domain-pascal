#!/usr/bin/env python3
"""loopalign.py - survey loop-head / branch-target alignment in the SR10.2 image.

Evidence tool for docs/codegen-patterns.md section 7 (bead domain-pascal-895):
which labels did Apollo's compiler pad to a 4-byte boundary with the 0x2048
(`movea.l A0,A0') filler?

Each map `I' segment inside 0xE00800..0xE88834 is split into routines (map
symbols and `link.w A6' words) and each routine is disassembled on its own
with m68k-elf-objdump (linear sweep from the routine start).  Segments whose
name ends in `_ASM', and the segments in HAND_ASM (frameless code with
SP-relative or register arguments, or data), are hand assembly
(codegen-patterns section 0) and are skipped (`--asm' includes them).
The 4-byte `rts; 0x2048' stub segments (MT, CT, COLOR4, ...) are kept.
For every branch (bra, bcc, dbcc; not bsr/jsr) the target is classified:

    F  the instruction just before the target is 0x2048 (filler)
    A  target 4-aligned, no filler
    U  target at 2 mod 4, no filler

and grouped by direction (backward = loop head) and by kind.  A routine is
split at map symbols and at every `link.w A6' word; a routine is
`aligning' when one of its loop heads has the filler (class F or X in
`routines').

usage:
    tools/loopalign.py summary            # the counts quoted in section 7
    tools/loopalign.py segments           # per-segment F/A/U of backward targets
    tools/loopalign.py list KIND [CLASS]  # list sites, KIND in dbf,bwd,fwd
    tools/loopalign.py fillers            # every 0x2048 and what follows it
    tools/loopalign.py ends               # 0x2048 vs rts+0x0000 segment ends
    tools/loopalign.py routines [SEG...]  # per-routine class of loop heads:
                                          # F filler, u unaligned, X both,
                                          # a only 4-aligned, . no loop
    tools/loopalign.py switch SEG...      # where alignment switches off in
                                          # a segment (for dpc's
                                          # --loop-align-off-from and the
                                          # conformance manifest)
    tools/loopalign.py heads              # loop heads by back branch (dbf,
                                          # bcc, bra), entry (fall, bra-in:
                                          # a `bra' to the bottom test, i.e.
                                          # `while'; bra-out), routine class
                                          # (al aligning, na not, ? unknown,
                                          # from `switch') and F/A/U
                                          # (codegen-patterns 7.4)
    tools/loopalign.py notest [-v]        # loops closed only by `bra's (no
                                          # test anywhere: `repeat ... until
                                          # false', 7.2) by entry, routine
                                          # class and F/A/U; -v one line
                                          # per loop with its size
    tools/loopalign.py fors [-v]          # `dbf' loops: preheader order of
                                          # count and index, index in a
                                          # register or memory, index use
                                          # (byte index vs stepped pointer),
                                          # run-time guards (7.4)
    tools/loopalign.py guards [-v]        # `dbf' loops guarded before the
                                          # head: how the guard sets its
                                          # flags (`subq #k', `tst', `sub',
                                          # `bmi'/`beq'), the count in place
                                          # or copied, calls in the body,
                                          # the index's initial value (7.4,
                                          # "Run-time bounds"); -v one line
                                          # per loop
    tools/loopalign.py ptrs [-v] [ROOT...] # `dbf' loops: the stepped
                                          # pointers by how the preheader
                                          # forms them (root: `lea A5',
                                          # `movea A5', `lea A6', `movea
                                          # A6', `movea abs', `lea An', ...;
                                          # adjusted in place or not); -v
                                          # one line per pointer of the
                                          # ROOTs (default the A5 and A6
                                          # ones) with its displacements
                                          # and preheader (7.4, "Forming")

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import subprocess
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sau2map  # noqa: E402

IMG = os.environ.get('DOMAIN_OS_IMAGE', os.path.expanduser(
    '~/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2/sau2/domain_os'))
LO, HI = 0xE00800, 0xE88834
OFF = 0xDFFC00

_LINE = re.compile(r'^\s*([0-9a-f]+):\s+((?:[0-9a-f]{4} ?)+)\s*\t(.*)$')
_BR = re.compile(r'^(bra|b(?:hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le)|db\w+)[sw]?\s+(?:%d\d,)?0x([0-9a-f]+)')

_image = None


def image():
    global _image
    if _image is None:
        with open(IMG, 'rb') as f:
            _image = f.read()
    return _image


def word(a):
    o = a - OFF
    return (image()[o] << 8) | image()[o + 1]


def disasm(start, size):
    """[(addr, len, text)] for the bytes start..start+size."""
    data = image()[start - OFF:start - OFF + size]
    tmp = os.path.join(os.path.dirname(__file__), '..', 'tmp')
    os.makedirs(tmp, exist_ok=True)
    path = os.path.join(tmp, 'loopalign.%d.bin' % os.getpid())
    with open(path, 'wb') as f:
        f.write(data)
    try:
        out = subprocess.run(
            ['m68k-elf-objdump', '-D', '-b', 'binary', '-m', 'm68k:68010',
             '--adjust-vma=0x%x' % start, path],
            capture_output=True, text=True, check=True).stdout
    finally:
        os.unlink(path)
    insns = []
    for line in out.splitlines():
        mm = _LINE.match(line)
        if not mm:
            continue
        a = int(mm.group(1), 16)
        n = len(mm.group(2).split()) * 2
        if insns and insns[-1][0] == a:      # continuation line of an insn
            continue
        insns.append([a, n, mm.group(3).strip()])
    # fix lengths from successive addresses (objdump wraps long insns)
    for i in range(len(insns) - 1):
        insns[i][1] = insns[i + 1][0] - insns[i][0]
    return [tuple(x) for x in insns]


class Site:
    def __init__(self, seg, src, tgt, kind, text, prev):
        self.seg, self.src, self.tgt, self.kind, self.text = seg, src, tgt, kind, text
        self.prev = prev     # text of insn ending at tgt, or None
        if prev is not None and word(tgt - 2) == 0x2048 and prev.startswith('moveal %a0,%a0'):
            self.cls = 'F'
        elif tgt % 4 == 0:
            self.cls = 'A'
        else:
            self.cls = 'U'


# Segments without any `link.w A6' that are not compiled code: M$ARITH
# (section 0), PAS_$SET_BUILD 0xE11FA8 (SP args, `807c'), SMD_WIRED 0xE15B90
# (A0 argument, entry thunks), UID_$HASH 0xE17360, VFMT_$WRITEN 0xE6B0A4,
# UID_LIST 0xE1737C (a table).
HAND_ASM = {'M$ARITH', 'PAS_$SET_BUILD', 'SMD_WIRED', 'UID_$HASH',
            'VFMT_$WRITEN', 'UID_LIST'}


def is_asm(seg):
    """Hand assembly or data: an `_ASM' segment or one in HAND_ASM."""
    return seg.name.endswith('_ASM') or seg.name in HAND_ASM


def routine_starts(m, seg):
    """Start addresses of the routines in seg: map symbols inside it, plus
    every `link.w A6,#d' word (0x4E56, d <= 0) found by a raw word scan, so
    that module-local and nested routines are found too."""
    st = {s.addr for s in m.symbols if seg.contains(s.addr)}
    for p in range(seg.addr, min(seg.end, HI) - 2, 2):
        if word(p) == 0x4E56:
            d = word(p + 2)
            if d == 0 or (d >= 0x8000 and d % 2 == 0):
                st.add(p)
    st.add(seg.addr)
    return sorted(st)


# routine start -> [(addr, len, text)], filled by scan()
INSNS = {}


def scan(include_asm=False):
    """(map, sites, fillers, routines).  Each routine (see routine_starts)
    is disassembled on its own so that data between routines cannot
    desynchronise the sweep for the next one."""
    m = sau2map.load()
    sites, fillers, routines = [], [], []
    for seg in m.segments:
        if seg.kind != 'I' or not (LO <= seg.addr < HI) or seg.size == 0:
            continue
        asm = is_asm(seg)
        if asm and not include_asm:
            continue
        st = routine_starts(m, seg)
        bounds = list(zip(st, st[1:] + [min(seg.end, HI)]))
        for r0, r1 in bounds:
            if r1 <= r0:
                continue
            routines.append((seg, r0, r1))
            insns = disasm(r0, r1 - r0)
            INSNS[r0] = insns
            ends = {a + n: t for a, n, t in insns}
            starts = {a for a, n, t in insns}
            for i, (a, n, t) in enumerate(insns):
                if n == 2 and word(a) == 0x2048:
                    nxt = insns[i + 1] if i + 1 < len(insns) else None
                    fillers.append((seg, a, nxt))
                b = _BR.match(t)
                if not b:
                    continue
                tgt = int(b.group(2), 16)
                if not (r0 <= tgt < r1) or tgt not in starts:
                    continue
                mn = b.group(1)
                kind = 'dbf' if mn.startswith('db') else ('bwd' if tgt <= a else 'fwd')
                sites.append(Site(seg, a, tgt, kind, t, ends.get(tgt)))
    return m, sites, fillers, routines


def loops(sites):
    """{target: (class, [sites])} for targets reached by a backward branch
    (a dbf or a bra/bcc to a lower or equal address): the loop heads."""
    tg = defaultdict(list)
    for s in sites:
        tg[s.tgt].append(s)
    return {t: (ss[0].cls, ss) for t, ss in tg.items()
            if any(s.src >= t for s in ss)}


def per_routine(sites, routines):
    """[(seg, r0, r1, Counter of loop-head classes)] in address order."""
    lp = loops(sites)
    out = []
    heads = sorted(lp)
    import bisect
    for seg, r0, r1 in sorted(routines, key=lambda x: x[1]):
        c = Counter()
        for t in heads[bisect.bisect_left(heads, r0):bisect.bisect_left(heads, r1)]:
            c[lp[t][0]] += 1
        out.append((seg, r0, r1, c))
    return out


def rclass(c):
    """One letter for a routine: F aligned with filler somewhere and no
    unaligned head; u unaligned head(s) and no filler; X both; a only heads
    that happen to be 4-aligned; . no loops."""
    if c['F'] and c['U']:
        return 'X'
    return 'F' if c['F'] else 'u' if c['U'] else 'a' if c['A'] else '.'


def switch(pr, seg):
    """Where loop alignment switches off in map segment `seg'
    (codegen-patterns 7: on for a leading run of routines, off for the
    rest).  `pr' is per_routine()'s list.  Returns (last, first): the start
    of the last routine with a filler (class F or X) and of the first
    later routine with a head at 2 mod 4 and no filler (class u), each None
    if there is none.  The switch is after `last' and at or before
    `first'; routines between them (classes a and .) cannot be classed.
    Raises ValueError if a u routine comes before an F or X one (the
    switch would go back on, which the survey never saw)."""
    rs = [(r0, rclass(c)) for sg, r0, _, c in pr if sg is seg]
    last = None
    for r0, cl in rs:
        if cl in 'FX':
            last = r0
    first = next((r0 for r0, cl in rs if cl == 'u' and (last is None or r0 > last)), None)
    early = [r0 for r0, cl in rs if cl == 'u' and last is not None and r0 < last]
    if early:
        raise ValueError('%s: routine 0x%X does not align but 0x%X, later, does'
                         % (seg.name, early[0], last))
    return last, first


def align_classes(m, pr):
    """{routine start: 'al' | 'na' | '?'}: aligning (at or before the last
    routine of its segment with a filler), not aligning (at or after the
    first later one with a head at 2 mod 4 and none), or not known."""
    out = {}
    for seg in m.segments:
        rs = [(r0, rclass(c)) for sg, r0, _, c in pr if sg is seg]
        if not rs:
            continue
        try:
            last, first = switch(pr, seg)
        except ValueError:
            last, first = None, None
        for r0, cl in rs:
            if cl in 'FX' or (last is not None and r0 <= last):
                out[r0] = 'al'
            elif cl == 'u' or (first is not None and r0 >= first):
                out[r0] = 'na'
            else:
                out[r0] = '?'
    return out


def head_rows(m, sites, routines):
    """[(head, class F/A/U, back kind, entry, routine class, routine start,
    address of the last back branch)] for every loop head.  The entry is
    `fall' (the instruction before the head, a filler skipped, does not
    jump), `bra-in' (a `bra' to the loop's own bottom: a `while') or
    `bra-out'."""
    import bisect
    pr = per_routine(sites, routines)
    acls = align_classes(m, pr)
    starts = [r0 for _, r0, _, _ in pr]
    rows = []
    for t, (cls, ss) in sorted(loops(sites).items()):
        r0 = starts[bisect.bisect_right(starts, t) - 1]
        ins = INSNS[r0]
        idx = {a: i for i, (a, n, x) in enumerate(ins)}
        if t not in idx:
            continue
        j = idx[t] - 1
        if j >= 0 and ins[j][2].startswith('moveal %a0,%a0'):
            j -= 1
        prev = ins[j][2] if j >= 0 else ''
        last = max((x for x in ss if x.src >= t), key=lambda x: x.src)
        back = 'dbf' if last.kind == 'dbf' else 'bra' if last.text.startswith('bra') else 'bcc'
        entry = 'fall'
        if prev.startswith('bra'):
            mm = re.match(r'bra[sw]?\s+0x([0-9a-f]+)', prev)
            entry = 'bra-in' if mm and t < int(mm.group(1), 16) <= last.src else 'bra-out'
        rows.append((t, cls, back, entry, acls.get(r0, '?'), r0, last.src))
    return rows


def heads(m, sites, routines):
    """The table of codegen-patterns 7.4 (loop heads by shape and class)."""
    rows = head_rows(m, sites, routines)
    c = Counter((b, e, a, cl) for _, cl, b, e, a, _, _ in rows)
    for b in ('dbf', 'bcc', 'bra'):
        for e in ('fall', 'bra-in', 'bra-out'):
            for a in ('al', 'na', '?'):
                n = [c[b, e, a, k] for k in 'FAU']
                if sum(n):
                    print('%-4s %-8s %-3s F=%-4d A=%-4d U=%-4d' % (b, e, a, *n))
    big = [(t, src) for t, cl, b, e, a, _, src in rows
           if e == 'bra-in' and a == 'al' and cl == 'U']
    print('while heads at 2 mod 4 in aligning routines: %s' % ', '.join(
        '0x%X (loop 0x%X bytes)' % (t, src - t) for t, src in big))


def notest_rows(m, sites, routines):
    """[(head, F/A/U, entry, routine class, routine start, last back
    branch)] of the loops every backward branch of which is a `bra': no
    test anywhere (codegen-patterns 7.2, bead domain-pascal-pkle)."""
    lp = loops(sites)
    out = []
    for t, cl, back, entry, acls, r0, last in head_rows(m, sites, routines):
        if back == 'bra' and all(s.text.startswith('bra') for s in lp[t][1] if s.src >= t):
            out.append((t, cl, entry, acls, r0, last))
    return out


def notest(m, sites, routines, verbose=False):
    rows = notest_rows(m, sites, routines)
    c = Counter((e, a, cl) for _, cl, e, a, _, _ in rows)
    print('loops closed only by `bra\'s: %d' % len(rows))
    for e in ('fall', 'bra-in', 'bra-out'):
        for a in ('al', 'na', '?'):
            n = [c[e, a, k] for k in 'FAU']
            if sum(n):
                print('%-8s %-3s F=%-4d A=%-4d U=%-4d' % (e, a, *n))
    big = [(t, src) for t, cl, e, a, _, src in rows if a == 'al' and cl == 'U']
    print('heads at 2 mod 4 in aligning routines: %s' % ', '.join(
        '0x%X (loop 0x%X bytes)' % (t, src - t) for t, src in big))
    if verbose:
        for t, cl, e, a, r0, src in rows:
            print('0x%06X %s %-7s %-2s routine 0x%06X %-28s bra 0x%06X size 0x%X'
                  % (t, cl, e, a, r0, m.describe(r0), src, src - t))


_STEP = re.compile(r'(addq|subq)w #1,(%d\d)$')


def fors(m, sites, routines, verbose=False):
    """`dbf' loop statistics of codegen-patterns 7.4."""
    rows = [r for r in head_rows(m, sites, routines) if r[2] == 'dbf']
    order, where, use, guard = Counter(), Counter(), Counter(), Counter()
    for t, cls, back, entry, al, r0, src in rows:
        ins = INSNS[r0]
        idx = {a: i for i, (a, n, x) in enumerate(ins)}
        i, j = idx[t], idx[src]
        body = [x for _, _, x in ins[i:j]]
        tail = body[-6:]
        calls = any(x.startswith(('jsr', 'bsr')) for x in body)
        creg = re.search(r'(%d\d)', ins[j][2]).group(1)
        # preheader: back to a branch, call or label
        pre, k = [], i - 1
        while k >= 0 and len(pre) < 12:
            x = ins[k][2]
            if re.match(r'(b[a-z]{2}[sw]? |bra|db|jsr|bsr|rts|jmp)', x):
                break
            pre.insert(0, x)
            k -= 1
        # index in memory or in a register
        if any(re.match(r'addqw #1,%(fp|a\d)@\(-\d+\)$', x) for x in tail):
            where['memory'] += 1
        steps = [x for x in tail if _STEP.match(x)]
        if steps:
            where['register'] += 1
            ireg = _STEP.match(steps[-1]).group(2)
            cdef = [n for n, x in enumerate(pre) if x.endswith(',' + creg)
                    and re.match(r'(moveq|movew) #', x)]
            idef = [n for n, x in enumerate(pre)
                    if re.search(r'(,|clrw )' + re.escape(ireg) + '$', x)]
            if cdef and idef:
                order['count first' if cdef[-1] < idef[-1] else 'index first'] += 1
            used = [x for x in body if re.search(r'\(-?[0-9a-fx]*,' + re.escape(ireg) + r':[wl]\)', x)]
            copies = [x.split(',')[1] for x in body
                      if re.match(r'movew ' + re.escape(ireg) + r',%d\d$', x)]
            scaled = any(re.match(r'(addw (%d\d),\2|ls[lr][wl] #\d,(%d\d)|mul[us]w #\d+,(%d\d))$', y)
                         and y.split(',')[-1] in copies for y in body)
            ptr = any(re.match(r'(addql #\d,%a\d|lea %a(\d)@\(\d+\),%a\2)$', y)
                      for y in body[-5:])
            use[('direct ' if used else '') + ('scaled ' if scaled else '')
                + ('pointer' if ptr else '')] += 1
            if used:
                for x in used:
                    use['  direct op ' + x.split()[0]] += 1
        # run-time guard `subq.w #1,T; bmi' just before the preheader
        g = k
        if g >= 1 and ins[g][2].startswith('bmi') and ins[g - 1][2].startswith('subqw #1,%d'):
            tgt = int(re.search(r'0x([0-9a-f]+)', ins[g][2]).group(1), 16)
            if src < tgt <= src + 8:
                T = ins[g - 1][2].split(',')[1]
                moved = any(x == 'movew %s,%s' % (T, creg) for x in pre)
                kind = 'in place' if T == creg else 'copied' if moved else 'other'
                guard['%s, %s' % (kind, 'calls' if calls else 'no calls')] += 1
                if verbose:
                    print('guard %06X %s %s' % (t, kind, 'calls' if calls else ''))
    print('dbf loops:', len(rows))
    print('index:', dict(where))
    print('preheader (constant count, register index):', dict(order))
    print('index use:')
    for k, v in sorted(use.items()):
        print('  %4d %r' % (v, k))
    print('subq.w #1; bmi guards:')
    for k, v in sorted(guard.items()):
        print('  %4d %s' % (v, k))


_GUARD_BR = re.compile(r'^(b(?:hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le))[sw]?\s+0x([0-9a-f]+)')


def exit_chain(ins, idx, j):
    """The addresses a `dbf' at ins[j] leaves to: the next instruction and
    the targets of the `bra's chained from there."""
    exits = set()
    k = j + 1
    while k < len(ins) and ins[k][0] not in exits:
        exits.add(ins[k][0])
        mm = re.match(r'^bra[sw]?\s+0x([0-9a-f]+)', ins[k][2])
        if not mm:
            break
        k = idx.get(int(mm.group(1), 16), len(ins))
        if k == len(ins):
            exits.add(int(mm.group(1), 16))
    if j + 1 >= len(ins):
        exits.add(ins[j][0] + ins[j][1])
    return exits


def guard_rows(m, sites, routines):
    """[(head, routine start, branch, before, after, calls, counter)] for
    every `dbf' loop guarded by a conditional branch to its exit (the
    instruction after the `dbf', or the target of a chain of `bra's
    starting there: a guard that jumps past the exit's `bra'), found by
    walking back from the head (fillers skipped) at most 14 instructions,
    not past another branch, call or `rts'.  `before': up to 5
    instructions before the guard;
    `after': the preheader between the guard and the head, fillers left
    out; `counter': the `dbf' register."""
    out = []
    for t, cls, back, entry, al, r0, src in head_rows(m, sites, routines):
        if back != 'dbf':
            continue
        ins = INSNS[r0]
        idx = {a: i for i, (a, n, x) in enumerate(ins)}
        i, j = idx[t], idx[src]
        exits = exit_chain(ins, idx, j)
        g = None
        for k in range(i - 1, max(-1, i - 15), -1):
            x = ins[k][2]
            b = _GUARD_BR.match(x)
            if b and int(b.group(2), 16) in exits:
                g = k
                break
            if b or re.match(r'(bra|jsr|bsr|rts|jmp|db)', x):
                break
        if g is None:
            continue
        before = [x for _, _, x in ins[max(0, g - 5):g]]
        after = [x for _, _, x in ins[g + 1:i] if x != 'moveal %a0,%a0']
        calls = any(x.startswith(('jsr', 'bsr')) for _, _, x in ins[i:j])
        creg = re.search(r'(%d\d)', ins[j][2]).group(1)
        out.append((t, r0, ins[g][2].split()[0].rstrip('sw'), before, after,
                    calls, creg))
    return out


def guard_class(br, before, after, creg):
    """(how the guard's flags are set, where the count goes, the index's
    initial value) for one guard_rows() row (codegen-patterns 7.4,
    "Run-time bounds").  How: `subq #k' / `subi #k' (a constant lower
    bound or a bound `e - c'), `tst' (from 0), `sub' (a run-time lower
    bound), with the branch (`bmi', `beq' for unsigned counts), else
    `other: <insn>'.  Where: `in place' (the instruction setting the flags
    writes the `dbf' register and no copy follows), `copied' (`move.w
    T,Dc' after the guard), `self-copy' (`move.w Dc,Dc'), `other'.  The
    index: `0' for `clr.w', `k' for `moveq #k', `?' when neither follows
    the guard."""
    last = before[-1] if before else ''
    m = re.match(r'(subqw|subiw) #(\d+),(%d\d)$', last)
    T = None
    if m:
        how, T = '%s #%s' % (m.group(1)[:4], m.group(2)), m.group(3)
    elif re.match(r'tstw %d\d$', last):
        how, T = 'tst', last.split()[1]
    elif re.match(r'subw [^,]+,%d\d$', last):
        how, T = 'sub', last.split(',')[-1]
    elif last.startswith('tstw '):
        # a bound in memory tested in place and loaded again (unsigned
        # counts: `tst.w (d,An); beq; move.w (d,An),Dc')
        mem = last[5:]
        return ('%s tst mem' % br,
                'reloaded' if 'movew %s,%s' % (mem, creg) in after else 'other',
                _guard_index(after, creg))
    else:
        how = 'other: ' + re.sub(r'0x[0-9a-f]+', 'ABS', last)
    copy = T is not None and 'movew %s,%s' % (T, creg) in after
    if T is None:
        where = 'other'
    elif T == creg:
        where = 'self-copy' if copy else 'in place'
    else:
        where = 'copied' if copy else 'other'
    return '%s %s' % (br, how), where, _guard_index(after, creg)


def _guard_index(after, creg):
    """The index's initial value in a guarded preheader: the first `clr.w'
    or `moveq' into a data register other than the counter."""
    for x in after:
        mm = re.match(r'(clrw |moveq #(-?\d+),)(%d\d)$', x)
        if mm and mm.group(3) != creg:
            return '0' if mm.group(1) == 'clrw ' else mm.group(2)
    return '?'


def guards(m, sites, routines, verbose=False):
    """`dbf' loop guards by how they test, where the count goes, calls in
    the body and the index's initial value (codegen-patterns 7.4)."""
    c = Counter()
    for t, r0, br, before, after, calls, creg in guard_rows(m, sites, routines):
        how, where, lo = guard_class(br, before, after, creg)
        k = (how, where, 'calls' if calls else 'no calls', lo)
        c[k] += 1
        if verbose:
            print('%06X %-28s %-26s %-9s %-8s lo=%-2s | %s || %s' % (
                t, m.describe(t), how, where, 'calls' if calls else '-', lo,
                '; '.join(before[-3:]), '; '.join(after)))
    for k, v in sorted(c.items(), key=lambda kv: (kv[0][0], -kv[1])):
        print('%4d  %-26s %-9s %-8s from %s' % ((v,) + k))


# A pointer step at a loop's bottom: `addq.l/subq.l #s,An' or `lea (s,An),An'.
_PSTEP = re.compile(r'(addql|subql) #(\d),(%a[0-6])$')
_PLEA = re.compile(r'lea (%a[0-6])@\((-?\d+)\),(%a[0-6])$')

# The definition that starts a stepped pointer's chain, by its source
# (bead domain-pascal-xm5w): the unit's data (A5), the frame (A6), an
# absolute address (another unit's variable), another address register
# (a pointer), a load from memory, a data register.
_ROOTS = [
    (r'lea %a5@(\((-?\d+)\))?,', 'lea A5'),
    (r'moveal %a5,', 'movea A5'),
    (r'lea %fp@(\((-?\d+)\))?,', 'lea A6'),
    (r'moveal %fp,', 'movea A6'),
    (r'moveal #', 'movea abs'),
    (r'lea 0x', 'lea abs'),
    (r'lea %a[0-4]@', 'lea An'),
    (r'moveal %a[0-4],', 'movea An'),
    (r'moveal %fp@', 'load frame'),
    (r'moveal %a5@', 'load A5'),
    (r'moveal %a[0-4]@', 'load An'),
    (r'moveal %d\d,', 'from Dn'),
    (r'moveal 0x', 'load abs'),
]


def root_class(text):
    """The class of a stepped pointer's first definition (see _ROOTS)."""
    for pat, name in _ROOTS:
        if re.match(pat, text):
            return name
    return 'other ' + text.split()[0]


def _writes(text, reg):
    """Whether instruction `text' writes address register `reg' (as its
    last operand; `(An)+'/`-(An)' side effects are not counted)."""
    return text.endswith(',' + reg) or text in ('clrl ' + reg,) or \
        re.match(r'\w+ ' + re.escape(reg) + '$', text) is not None


def stepped_pointers(ins, i, j, filler=lambda a: False):
    """The pointers a `dbf' loop steps, its head ins[i], its `dbf' ins[j]
    (codegen-patterns 7.4, "Array indexing by the index"; bead
    domain-pascal-xm5w).  [dict] per address register stepped in the body
    (`addq.l'/`subq.l'/`lea (s,An),An'; `(An)+' only: kind `postinc'):

      reg, step (the step's size, 0 for `(An)+'), kind ('step'|'postinc'),
      chain [(addr, text)]: the preheader's definitions of the register,
          the first a root (`lea (d,A5),An', `movea.l A5,An', ...) and the
          rest in-place adjustments (`addq.l', `lea (d,An),An'),
      root (root_class of the first), offset (the constant the chain adds
          to its source: the root's displacement plus the adjustments),
      disps: the displacements of the body's accesses through the
          register (`(d,An)', 0 for `(An)'; the step itself not counted),
      count ('const' or 'run': the `dbf' counter's last preheader
          definition is an immediate `moveq'/`move.w', or not),
      order: the preheader instructions from the root on, as text.

    `filler(addr)' says whether the word at addr is the 0x2048 filler (it
    is skipped before the head)."""
    body = [x for _, _, x in ins[i:j]]
    stepped = {}
    for x in body:
        mm = _PSTEP.match(x)
        if mm:
            stepped.setdefault(mm.group(3), []).append(int(mm.group(2)))
        mm = _PLEA.match(x)
        if mm and mm.group(1) == mm.group(3):
            stepped.setdefault(mm.group(1), []).append(abs(int(mm.group(2))))
    for r in re.findall(r'(%a[0-6])@\+', ' '.join(body)):
        stepped.setdefault(r, [])
    creg = re.search(r'(%d\d)', ins[j][2]).group(1)
    count = 'run'
    for k in range(i - 1, max(-1, i - 40), -1):
        x = ins[k][2]
        if x.endswith(',' + creg) or x.endswith(' ' + creg):
            count = 'const' if re.match(r'(moveq|movew) #', x) else 'run'
            break
    out = []
    for r, steps in sorted(stepped.items()):
        k = i - 1
        if k >= 0 and ins[k][1] == 2 and filler(ins[k][0]):
            k -= 1
        chain = []
        while k >= 0 and i - k < 40:
            x = ins[k][2]
            if _writes(x, r):
                chain.insert(0, (ins[k][0], x))
                mm = _PLEA.match(x)
                if not (_PSTEP.match(x) or (mm and mm.group(1) == r)):
                    break
            k -= 1
        root = root_class(chain[0][1]) if chain else 'none'
        offset = 0
        for n, (_, x) in enumerate(chain):
            mm = re.match(r'lea (?:%a\d|%fp)@\((-?\d+)\),', x)
            if mm:
                offset += int(mm.group(1))
            mm = _PSTEP.match(x)
            if mm:
                offset += int(mm.group(2)) * (1 if mm.group(1) == 'addql' else -1)
        disps = set()
        for x in body:
            mm = _PLEA.match(x)
            if (mm and mm.group(1) == r == mm.group(3)) or _PSTEP.match(x):
                continue                      # a step (`lea (d,Ap),Aw' is an access)
            for d in re.findall(re.escape(r) + r'@\((-?\d+)\)', x):
                disps.add(int(d))
            if re.search(re.escape(r) + r'@(?![(+])', x):
                disps.add(0)
        start = idx_of(ins, chain[0][0]) if chain else i
        out.append(dict(reg=r, step=steps[-1] if steps else 0,
                        kind='step' if steps else 'postinc', chain=chain,
                        root=root, offset=offset, disps=sorted(disps),
                        count=count, order=[x for _, _, x in ins[start:i]]))
    return out


def idx_of(ins, addr):
    for n, (a, _, _) in enumerate(ins):
        if a == addr:
            return n
    raise KeyError(addr)


def ptrs(m, sites, routines, verbose=False, roots=None):
    """Stepped pointers of `dbf' loops by how they are formed (bead
    domain-pascal-xm5w; codegen-patterns 7.4): counts by kind, root and
    in place (a chain longer than the root) or not, then with -v one line
    per pointer of the roots given (default the unit's data and the
    frame).  Loops whose body is one instruction (block copies and fills,
    codegen-patterns 5.2) are counted apart as `blk'."""
    rows = [r for r in head_rows(m, sites, routines) if r[2] == 'dbf']
    c = Counter()
    lines = []
    roots = roots or ('lea A5', 'movea A5', 'lea A6', 'movea A6')
    for t, cls, back, entry, al, r0, src in rows:
        ins = INSNS[r0]
        idx = {a: n for n, (a, _, _) in enumerate(ins)}
        i, j = idx[t], idx[src]
        for p in stepped_pointers(ins, i, j, lambda a: word(a) == 0x2048):
            kind = 'blk' if j - i == 1 else p['kind']
            inplace = len(p['chain']) > 1
            c[(kind, p['root'], 'in place' if inplace else '')] += 1
            if verbose and kind == 'step' and p['root'] in roots:
                lines.append('%06X %-3s %-5s %-9s %-24s %s step=%-3d off=%-5d disps=%s  | %s' % (
                    t, al, p['count'], p['root'] + ('+' if inplace else ''),
                    m.describe(r0)[:24], p['reg'], p['step'], p['offset'],
                    ','.join(map(str, p['disps'])), ' ; '.join(p['order'])))
    for k, v in sorted(c.items()):
        print('%4d %-8s %-12s %s' % (v, *k))
    for x in lines:
        print(x)


def summary(m, sites, fillers, routines):
    """The counts quoted in codegen-patterns section 7."""
    import bisect
    fa = Counter((a + 2) % 4 for _, a, _ in fillers)
    print('0x2048 words: %d; next insn 4-aligned: %d' % (len(fillers), fa[0]))
    lp = loops(sites)
    pr = per_routine(sites, routines)
    print('routines by class:', dict(Counter(rclass(c) for _, _, _, c in pr)))
    starts = [r0 for _, r0, _, _ in pr]
    cls_of = [rclass(c) for _, _, _, c in pr]

    def grp(a):
        return 'aligning' if cls_of[bisect.bisect_right(starts, a) - 1] in 'FX' else 'other'
    heads, copies, fwd = Counter(), Counter(), Counter()
    tg = defaultdict(list)
    for s in sites:
        tg[s.tgt].append(s)
    for t, (cls, ss) in lp.items():
        heads[grp(t), cls] += 1
        d = disasm(t, 6)
        if (len(d) >= 2 and re.match(r'movel %a\d@\+,%a\d@\+', d[0][2])
                and d[1][2].startswith('db')):
            copies[grp(t), cls] += 1
    for t, ss in tg.items():
        if t not in lp:
            fwd[grp(t), ss[0].cls] += 1
    for name, c in (('loop heads', heads), ('one-move.l copy loops', copies),
                    ('forward-only targets', fwd)):
        for g in ('aligning', 'other'):
            print('%-22s %-9s F=%-4d A=%-5d U=%-5d' % (
                name, g, c[g, 'F'], c[g, 'A'], c[g, 'U']))


def segments(m, sites, fillers):
    per = defaultdict(Counter)
    for s in sites:
        if s.kind in ('dbf', 'bwd'):
            per[s.seg][s.cls] += 1
    for seg, c in sorted(per.items(), key=lambda x: x[0].addr):
        sec = [x.name for x in m.sections if x.kind == 'I' and x.size and x.contains(seg.addr)]
        print('%06X %-18s %-14s F=%-3d A=%-3d U=%-3d' % (
            seg.addr, seg.name, sec[0] if sec else '?', c['F'], c['A'], c['U']))


def listing(m, sites, kind, cls=None):
    for s in sites:
        if s.kind == kind and (cls is None or s.cls == cls):
            print('%s %06X <- %06X %-14s %-28s | %s' % (
                s.cls, s.tgt, s.src, s.seg.name, m.describe(s.tgt), s.text))


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    asm = '--asm' in argv
    argv = [a for a in argv if a != '--asm']
    if argv[0] == 'list' and (len(argv) < 2 or argv[1] not in ('dbf', 'bwd', 'fwd')):
        print(__doc__)
        return 2
    m, sites, fillers, routines = scan(asm)
    if argv[0] == 'summary':
        summary(m, sites, fillers, routines)
    elif argv[0] == 'segments':
        segments(m, sites, fillers)
    elif argv[0] == 'list':
        listing(m, sites, argv[1], argv[2] if len(argv) > 2 else None)
    elif argv[0] == 'routines':
        pr = per_routine(sites, routines)
        tot = Counter(rclass(c) for _, _, _, c in pr)
        print('routines by class:', dict(tot))
        if len(argv) > 1:
            for seg, r0, r1, c in pr:
                if seg.name in argv[1:]:
                    print('%06X %s %-34s %s' % (r0, rclass(c), m.describe(r0), dict(c)))
        else:
            cur, line = None, ''
            for seg, r0, r1, c in pr:
                if seg is not cur:
                    if cur is not None and line.strip('.'):
                        print('%06X %-16s %s' % (cur.addr, cur.name, line))
                    cur, line = seg, ''
                line += rclass(c)
            if line.strip('.'):
                print('%06X %-16s %s' % (cur.addr, cur.name, line))
    elif argv[0] == 'switch':
        if len(argv) < 2:
            print(__doc__)
            return 2
        pr = per_routine(sites, routines)
        # the map name, which dpc --loop-align-off-from takes; a routine
        # the map does not name is module-local (CLAUDE.md 7)
        names = {r0: (m.names_at(r0) or ['module-local'])[0] for _, r0, _, _ in pr}
        rc = 0
        for seg in m.segments:
            if seg.name not in argv[1:] or not any(sg is seg for sg, _, _, _ in pr):
                continue
            line = ''.join(rclass(c) for sg, _, _, c in pr if sg is seg)
            print('%06X..%06X %s  %s' % (seg.addr, seg.end, seg.name, line))
            try:
                last, first = switch(pr, seg)
            except ValueError as e:
                print('  error: %s' % e)
                rc = 1
                continue
            print('  last aligning:      %s' % ('0x%X %s' % (last, names[last])
                                               if last is not None else 'none seen'))
            print('  first non-aligning: %s' % ('0x%X %s' % (first, names[first])
                                                if first is not None else 'none seen'))
            # routines that cannot be classed lie between the two
            lo = last if last is not None else seg.addr - 1
            hi = first if first is not None else seg.end
            open_ = [r0 for sg, r0, _, _ in pr if sg is seg and lo < r0 < hi]
            if open_:
                print('  unclassed between: %s' % ', '.join(
                    '0x%X %s' % (r0, names[r0]) for r0 in open_[:6])
                    + (', ... (%d)' % len(open_) if len(open_) > 6 else ''))
        return rc
    elif argv[0] == 'heads':
        heads(m, sites, routines)
    elif argv[0] == 'notest':
        notest(m, sites, routines, '-v' in argv[1:])
    elif argv[0] == 'fors':
        fors(m, sites, routines, '-v' in argv[1:])
    elif argv[0] == 'guards':
        guards(m, sites, routines, '-v' in argv[1:])
    elif argv[0] == 'ptrs':
        roots = [a for a in argv[1:] if a != '-v']
        ptrs(m, sites, routines, '-v' in argv[1:], roots or None)
    elif argv[0] == 'ends':
        # last word of each code segment that ends after an rts, with the
        # classes of the segment's routines (as in `routines')
        pr = per_routine(sites, routines)
        for seg in m.segments:
            if (seg.kind != 'I' or not (LO <= seg.addr < HI) or not seg.size
                    or seg.end > HI or is_asm(seg)):
                continue
            w = word(seg.end - 2)
            if not (w == 0x2048 or (w == 0 and word(seg.end - 4) == 0x4E75)):
                continue
            line = ''.join(rclass(c) for sg, _, _, c in pr if sg is seg)
            print('%04X %06X %-16s %s' % (w, seg.addr, seg.name, line[-16:]))
    elif argv[0] == 'fillers':
        for seg, a, nxt in fillers:
            print('%06X %-14s %-28s next=%s' % (a, seg.name, m.describe(a),
                                                 nxt[2] if nxt else None))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
