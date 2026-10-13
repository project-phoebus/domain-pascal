#!/usr/bin/env python3
"""pcsurvey.py - constants in the code section, reached with (d,PC), and
expressions passed by address through frame temporaries.

Evidence tool for docs/codegen-patterns.md sections 1.5 and 11 (bead
domain-pascal-5aw.9).  Compiled routines are split and followed the way
tools/a5survey.py does (map symbols and `link.w A6' words, hand assembly
excluded, only the instructions reachable from the routine's start).
Every reached instruction with a `(d,PC)' operand that is not a `case'
jump table index (`(T,PC,Dn.w)') is a site.

A site's target is classed by where it lies:

    own   after its own routine's code, before the next routine start
          (the routine's "pool")
    back  before its own routine (an earlier routine's pool)
    fwd   after its own routine's extent
    code  an instruction some routine reaches (a routine's address
          passed as a procedure parameter, DIR 0xE4C9E4)

A routine's pool is cut into cells at the targets that fall in it
(`cells'): a cell runs to the next target, less a 0x2048 filler before a
4-aligned target; the last runs to the routine's end, less trailing
0x2048 words.  (So a cell can include bytes nobody names, and the last
cell's size is a guess when the routine's extent runs on.)

usage:
    tools/pcsurvey.py list        # one line per site
    tools/pcsurvey.py summary     # sites by form and by class
    tools/pcsurvey.py order [-v]  # the layout rule of a routine's new
                                  # cells: 2 mod 4 sizes first, then the
                                  # others, each first-used first and the
                                  # rest in reverse order of first use
    tools/pcsurvey.py align       # the first cell of a multiple of 4
                                  # bytes: 4-aligned by itself, after a
                                  # 0x2048 filler, or at 2 mod 4, against
                                  # the routine's loop-alignment class
                                  # (tools/loopalign.py)
    tools/pcsurvey.py shared      # cells named from later routines, and
                                  # cells repeating an earlier cell's bytes
    tools/pcsurvey.py dups        # what follows a `pea (d,PC)': the same
                                  # cell again as `move.l (SP),-(SP)'
    tools/pcsurvey.py temps       # `move.<s> X,(d,A6); pea (d,A6)': an
                                  # expression stored and passed by
                                  # address; offsets per routine

Python 3 standard library + m68k-elf-objdump.  Read-only.  The scan is
cached in tmp/pcsurvey.cache (delete it after changing the scan).
"""

import os
import pickle
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import a5survey  # noqa: E402
import loopalign  # noqa: E402
import sau2map  # noqa: E402

CACHE = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'tmp',
                     'pcsurvey.cache')

FILLER = 0x2048

# `%pc@(0xADDR)' operand, not an indexed one
_PC = re.compile(r'%pc@\(0x([0-9a-f]+)\)')


def pc_operand(text):
    """(target, form) for an instruction text with one (d,PC) operand,
    the form being the text with the operand written `%pc@(T)'; None
    when there is none (indexed (d,PC,Xn) operands do not count)."""
    m = _PC.search(text)
    if not m:
        return None
    return int(m.group(1), 16), _PC.sub('%pc@(T)', text)


def site_class(r0, r1, rend, tgt, code):
    """'own', 'back', 'fwd' or 'code' (module docstring) for a target of
    the routine r0..r1 whose reached code ends at rend; `code' is the set
    of reached instruction addresses of the segment."""
    if tgt in code:
        return 'code'
    if rend <= tgt < r1:
        return 'own'
    if tgt < r0:
        return 'back'
    return 'fwd'


def layout(first_use, size):
    """The cell order the rule predicts for a routine's new cells:
    `first_use' lists them in order of first use, `size' gives each one's
    size.  Cells of 2 mod 4 bytes first, then the others; in each group
    the first used first, then the rest in reverse order of first use."""
    out = []
    for group in ([c for c in first_use if size[c] % 4],
                  [c for c in first_use if size[c] % 4 == 0]):
        if group:
            out += [group[0]] + group[1:][::-1]
    return out


def cut(lo, hi, targets, word):
    """[(target, size)] for the pool lo..hi of one routine cut at
    `targets' (sorted, inside lo..hi); `word(a)' reads the image."""
    out = []
    for i, t in enumerate(targets):
        if i + 1 < len(targets):
            e = targets[i + 1]
            if e % 4 == 0 and e - t > 2 and word(e - 2) == FILLER:
                e -= 2
        else:
            e = hi
            while e - 2 > t and word(e - 2) == FILLER:
                e -= 2
        out.append((t, e - t))
    return out


class Seg:
    def __init__(self, name, addr, end):
        self.name, self.addr, self.end = name, addr, end
        self.routines = []      # (r0, r1, reached-end)
        self.sites = []         # (routine r0, addr, target, form, next text)
        self.code = set()       # reached instruction addresses
        self.insns = {}         # r0 -> [(addr, len, text)] reached


def scan():
    m = sau2map.load()
    segs = []
    for seg in m.segments:
        if seg.kind != 'I' or not (loopalign.LO <= seg.addr < loopalign.HI) or not seg.size:
            continue
        if loopalign.is_asm(seg):
            continue
        s = Seg(seg.name, seg.addr, min(seg.end, loopalign.HI))
        st = loopalign.routine_starts(m, seg)
        for r0, r1 in zip(st, st[1:] + [s.end]):
            if r1 <= r0:
                continue
            ins = a5survey.reached(r0, r1)
            rend = max((a + n for a, n, _ in ins), default=r0)
            s.routines.append((r0, r1, rend))
            s.insns[r0] = ins
            for i, (a, n, t) in enumerate(ins):
                s.code.add(a)
                p = pc_operand(t)
                if p:
                    nxt = ins[i + 1][2] if i + 1 < len(ins) else None
                    s.sites.append((r0, a, p[0], p[1], nxt))
        segs.append(s)
    return m, segs


def load(refresh=False):
    """(map, [Seg]); the segments are cached as plain dicts so that the
    cache does not depend on the module this runs as."""
    if not refresh and os.path.exists(CACHE):
        with open(CACHE, 'rb') as f:
            dicts = pickle.load(f)
        segs = []
        for d in dicts:
            s = Seg(d['name'], d['addr'], d['end'])
            s.__dict__.update(d)
            segs.append(s)
        return sau2map.load(), segs
    m, segs = scan()
    os.makedirs(os.path.dirname(CACHE), exist_ok=True)
    with open(CACHE, 'wb') as f:
        pickle.dump([s.__dict__ for s in segs], f)
    return m, segs


def classed(s):
    """[(routine, addr, target, form, next, class)] for segment s."""
    rs = {r[0]: r for r in s.routines}
    out = []
    for r0, a, t, form, nxt in s.sites:
        r = rs[r0]
        out.append((r, a, t, form, nxt, site_class(r0, r[1], r[2], t, s.code)))
    return out


def pools(s):
    """{routine: ([new cells in order of first use], {cell: size})} for
    the routines of s whose own pool some site names."""
    first = defaultdict(list)
    for r, a, t, _, _, k in classed(s):
        if k == 'own' and t not in first[r]:
            first[r].append(t)
    out = {}
    for r, fu in first.items():
        targets = sorted({t for _, _, t, _, _ in s.sites if r[2] <= t < r[1]})
        size = dict(cut(r[2], r[1], targets, loopalign.word))
        out[r] = (fu, size)
    return out


def hexbytes(a, n):
    return ' '.join('%04x' % loopalign.word(a + i) for i in range(0, n, 2))


def cmd_list(m, segs, args):
    for s in segs:
        for r, a, t, form, nxt, k in classed(s):
            print('%-16s %-30s %06X %06X %-4s %-26s next=%s' % (
                s.name, m.describe(r[0]), a, t, k, form, nxt))


def cmd_summary(m, segs, args):
    forms, kinds = Counter(), Counter()
    for s in segs:
        for r, a, t, form, nxt, k in classed(s):
            forms[form] += 1
            kinds[k] += 1
    print('sites by class:', dict(kinds))
    for f, n in forms.most_common():
        print('  %5d  %s' % (n, f))


def cmd_order(m, segs, args):
    c = Counter()
    for s in segs:
        for r, (fu, size) in sorted(pools(s).items()):
            if len(fu) < 2:
                c['one cell'] += 1
                continue
            laid = sorted(fu)
            ok = layout(fu, size) == laid
            if not ok:
                # the last cell's size is a guess (module docstring)
                alt = dict(size)
                alt[laid[-1]] = 2 if size[laid[-1]] % 4 == 0 else 4
                ok = 'last cell resized' if layout(fu, alt) == laid else False
            c[ok] += 1
            if not ok or '-v' in args:
                print('%-6s %-14s %-30s %s' % (
                    'fits' if ok else 'NO', s.name, m.describe(r[0]),
                    ' '.join('+%X:%d#%d' % (t - r[2], size[t], fu.index(t)) for t in laid)))
    print(dict(c))


def cmd_align(m, segs, args):
    m2, sites, fillers, routines = loopalign.scan()
    ac = loopalign.align_classes(m2, loopalign.per_routine(sites, routines))
    c = Counter()
    for s in segs:
        for r, (fu, size) in sorted(pools(s).items()):
            longs = sorted(t for t in fu if size[t] % 4 == 0)
            if not longs:
                continue
            t = longs[0]
            if t - 2 >= r[2] and loopalign.word(t - 2) == FILLER:
                k = 'filler'
            else:
                k = '4-aligned' if t % 4 == 0 else '2 mod 4'
            cls = ac.get(r[0], '?')
            c[(k, cls)] += 1
            if k == '2 mod 4':
                print('%-14s %-30s cell %06X  routine class %s' % (
                    s.name, m.describe(r[0]), t, cls))
    for k, n in sorted(c.items()):
        print('%4d  %-10s in %s routines' % (n, k[0], k[1]))


def cmd_shared(m, segs, args):
    img = loopalign.image()
    back, dup = Counter(), 0
    for s in segs:
        for r, a, t, form, nxt, k in classed(s):
            if k == 'back':
                back[s.name] += 1
        seen = {}
        for r, (fu, size) in sorted(pools(s).items()):
            for t in sorted(size):
                v = img[t - loopalign.OFF:t - loopalign.OFF + size[t]]
                if v in seen:
                    dup += 1
                    print('%-14s cell %06X %s repeats %06X (%s)' % (
                        s.name, t, v.hex(), seen[v], m.describe(r[0])))
                else:
                    seen[v] = t
    print('sites naming an earlier routine\'s cell: %d' % sum(back.values()))
    print('cells repeating an earlier cell\'s bytes: %d' % dup)


def cmd_dups(m, segs, args):
    c = Counter()
    for s in segs:
        for r0, ins in s.insns.items():
            for i in range(1, len(ins)):
                p, t = ins[i - 1][2], ins[i][2]
                if not _PC.search(p) or not p.startswith('pea'):
                    continue
                if t == 'movel %sp@,%sp@-':
                    c['move.l (SP),-(SP)'] += 1
                elif t == p:
                    c['the same pea again'] += 1
                    print('same pea twice at %06X' % ins[i][0])
    print(dict(c))


_STORE = re.compile(r'^move([bwl]) (.*),%fp@\((-\d+)\)$')
_PEA_FP = re.compile(r'^pea %fp@\((-\d+)\)$')


def cmd_temps(m, segs, args):
    rows = []
    for s in segs:
        for r0, ins in s.insns.items():
            offs = []
            for i in range(1, len(ins)):
                st, pe = _STORE.match(ins[i - 1][2]), _PEA_FP.match(ins[i][2])
                if st and pe and st.group(3) == pe.group(1):
                    offs.append((int(pe.group(1)), st.group(1), ins[i][0]))
            if offs:
                rows.append((m.describe(r0), offs))
    reused = sum(1 for _, o in rows if len({x for x, _, _ in o}) < len(o))
    for name, offs in rows:
        print('%-32s %s' % (name, ' '.join('%d.%s@%06X' % o for o in offs)))
    print('routines: %d, with an offset used by two or more calls: %d' % (len(rows), reused))


COMMANDS = {'list': cmd_list, 'summary': cmd_summary, 'order': cmd_order,
            'align': cmd_align, 'shared': cmd_shared, 'dups': cmd_dups,
            'temps': cmd_temps}


def main(argv):
    if len(argv) < 2 or argv[1] not in COMMANDS:
        print(__doc__)
        return 2
    m, segs = load()
    COMMANDS[argv[1]](m, segs, argv[2:])
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
