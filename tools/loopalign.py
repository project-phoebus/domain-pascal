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
