#!/usr/bin/env python3
"""a5survey.py - which compiled routines of the SR10.2 image load A5.

Evidence tool for docs/codegen-patterns.md section 4 (bead domain-pascal-acw).
Every map `I' segment in 0xE00800..0xE88834 that is not hand assembly
(loopalign.is_asm) is split into routines the way tools/loopalign.py does
(map symbols plus every `link.w A6' word), and each routine is
disassembled on its own.  Per routine it records:

    sets   the routine loads A5 with `lea <abs>.l,A5'
    uses   an A5 operand other than the save (`pea (A5)' directly followed
           by the `lea', or A5 in a `movem' list), the load and the restore
           (`movea.l (d,A6),A5'); i.e. the routine addresses the data off A5
    abs    an absolute operand inside the segment's own D segment (the D
           segment of the same name, or the one its A5 loads point into)
    bsr    bsr targets (module-local calls)
    jsr    absolute call or address operands naming the routine, from any
           code (hand assembly included): the routine is reachable from
           outside its segment

`uses*' is `uses' closed over bsr: a routine that does not set A5 itself
inherits it, so a bsr to it from a routine that sets A5 counts as a use
when the callee (transitively) uses A5.

Routines are split at `link' words, so a routine's extent can include
the constants after its code; only the instructions reachable from its
start are looked at (`reached').

usage:
    tools/a5survey.py summary          # counts quoted in codegen-patterns 4
    tools/a5survey.py runs [SEG...]    # per-segment class string, one
                                       # letter per routine (see `letter';
                                       # unnamed ones in parentheses)
    tools/a5survey.py switch [SEG...]  # where a segment switches from
                                       # `load iff used' to `always load'
                                       # (for dpc's --a5-always-from)
    tools/a5survey.py list CLASS       # routines of a class: set-unused,
                                       # none, unnamed-set, abs-only
    tools/a5survey.py order [-v]       # forward vs backward bsr to routines
                                       # of the same segment, by caller and
                                       # callee kind (codegen-patterns 6,
                                       # "Order"); -v lists forward pairs

Python 3 standard library + m68k-elf-objdump.  Read-only.  The scan is
cached in tmp/a5survey.cache (delete it after changing the scan).
"""

import os
import pickle
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import loopalign  # noqa: E402
import sau2map  # noqa: E402

CACHE = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'tmp',
                     'a5survey.cache')

_LEA_A5 = re.compile(r'^lea 0x([0-9a-f]+),%a5$')
_RESTORE = re.compile(r'^moveal %fp@\(-?\d+\),%a5$')
_HEX = re.compile(r'0x([0-9a-f]+)')
_BSR = re.compile(r'^bsr[sw]?\s+0x([0-9a-f]+)')
_JSR = re.compile(r'^jsr 0x([0-9a-f]+)')


def classify_insns(insns):
    """(sets, uses, lea_targets, hexes, bsrs, save) for one routine's
    [(addr, len, text)].  `save' is 'pea', 'movem' or None."""
    sets, uses, save = False, False, None
    leas, hexes, bsrs = [], [], []
    for i, (a, n, t) in enumerate(insns):
        m = _LEA_A5.match(t)
        if m:
            sets = True
            leas.append(int(m.group(1), 16))
            continue
        b = _BSR.match(t)
        if b:
            bsrs.append(int(b.group(1), 16))
        for h in _HEX.findall(t):
            hexes.append(int(h, 16))
        if '%a5' not in t:
            continue
        if t.startswith('moveml') or t.startswith('movem'):
            if save is None:
                save = 'movem'
            continue
        if _RESTORE.match(t):
            continue
        if t == 'pea %a5@' and i + 1 < len(insns) and _LEA_A5.match(insns[i + 1][2]):
            save = 'pea'
            continue
        uses = True
    return sets, uses, leas, hexes, bsrs, save


_BRANCH = re.compile(r'^(bra|b(?:hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le)|db\w+)[sw]?\s+(?:%d\d,)?0x([0-9a-f]+)')
_TABLE = re.compile(r'^jmp %pc@\(0x([0-9a-f]+),%d\d:w\)')


def reached(r0, r1):
    """The instructions of r0..r1 reachable from r0 by fall-through and
    branches ([(addr, len, text)] in address order).  A linear sweep
    would decode constants and strings after the code as instructions
    (with bogus A5 operands); this follows the flow instead.  `case' jump
    tables (codegen-patterns 7: `cmpi #N,Dn; bcc; add.w Dn,Dn;
    move.w (tbl,PC,Dn.w),Dn; jmp (tbl,PC,Dn.w)') are followed: N word
    offsets from the table start (table_entries)."""
    lin = {a: (a, n, t) for a, n, t in loopalign.disasm(r0, r1 - r0)}
    seen, work, out = set(), [r0], {}
    while work:
        a = work.pop()
        prev = []      # texts of the fall-through run so far
        while r0 <= a < r1 and a not in seen:
            if a not in lin:
                lin.update({x: (x, n, t) for x, n, t in
                            loopalign.disasm(a, min(r1 - a, 0x400))})
                if a not in lin:
                    break
            seen.add(a)
            _, n, t = lin[a]
            if t.startswith('.short') or t.startswith('.long'):
                break
            out[a] = lin[a]
            b = _BRANCH.match(t)
            if b:
                work.append(int(b.group(2), 16))
                if b.group(1) == 'bra':
                    break
            tb = _TABLE.match(t)
            if tb:
                work.extend(table_entries(int(tb.group(1), 16), prev, r0, r1))
                break
            if (t.startswith('rts') or t.startswith('rte') or t.startswith('rtd')
                    or t.startswith('jmp')):
                break
            prev.append(t)
            a += n
    return [out[a] for a in sorted(out)]


_CMPI = re.compile(r'^cmpi[wl] #(\d+),%d\d$')


def table_entries(tbl, prev, r0, r1):
    """Targets of the `case' jump table at tbl.  The entry count is the
    bound N of the `cmpi #N,Dn' a few instructions before the `jmp'
    (every table in the image has one).  Targets may lie before the
    table (ASKNODE_$INTERNET_INFO 0xE64FA6: entries 2 and 3 go back to
    0xE64A24).  Without a bound: entries up to the lowest target after
    the table."""
    n = None
    for t in reversed(prev[-4:]):
        m = _CMPI.match(t)
        if m:
            n = int(m.group(1))
            break
    out, lo, p = [], r1, tbl
    while (p < tbl + 2 * n) if n is not None else (p < lo):
        if p + 2 > r1:
            break
        off = loopalign.word(p)
        off = off - 0x10000 if off & 0x8000 else off
        tgt = tbl + off
        if not (r0 <= tgt < r1) or tgt % 2 or (n is None and tgt <= tbl):
            break
        out.append(tgt)
        if tgt > tbl:
            lo = min(lo, tgt)
        p += 2
    return out


class Routine:
    def __init__(self, seg, r0, r1, name):
        self.seg, self.r0, self.r1, self.name = seg, r0, r1, name
        self.sets = self.uses = False
        self.leas, self.hexes, self.bsrs = [], [], []
        self.save = None
        self.has_link = False
        self.callers_bsr = set()     # routine starts that bsr here
        self.ext_refs = set()        # addresses outside the segment naming r0
        self.uses_t = False          # uses, closed over bsr callees
        self.abs_own = False         # absolute operand into own D segment


def scan(include_asm_refs=True):
    """[Routine] in address order for all compiled code segments."""
    m = sau2map.load()
    rs = []
    other_code = []   # (seg, start, end) for hand-asm segments (refs only)
    for seg in m.segments:
        if seg.kind != 'I' or not (loopalign.LO <= seg.addr < loopalign.HI) or not seg.size:
            continue
        if loopalign.is_asm(seg):
            other_code.append(seg)
            continue
        st = loopalign.routine_starts(m, seg)
        end = min(seg.end, loopalign.HI)
        for r0, r1 in zip(st, st[1:] + [end]):
            if r1 <= r0:
                continue
            names = m.names_at(r0)
            r = Routine(seg.name, r0, r1, names[0] if names else None)
            r.segaddr = seg.addr
            ins = reached(r0, r1)
            (r.sets, r.uses, r.leas, r.hexes, r.bsrs, r.save) = classify_insns(ins)
            r.has_link = loopalign.word(r0) == 0x4E56
            rs.append(r)
    asm_hexes = []
    if include_asm_refs:
        for seg in other_code:
            end = min(seg.end, loopalign.HI)
            ins = loopalign.disasm(seg.addr, end - seg.addr)
            for a, n, t in ins:
                for h in _HEX.findall(t):
                    asm_hexes.append((seg.addr, a, int(h, 16)))
    return m, rs, asm_hexes


def load(refresh=False):
    if not refresh and os.path.exists(CACHE):
        with open(CACHE, 'rb') as f:
            return pickle.load(f)
    m, rs, asm_hexes = scan()
    data = (rs, asm_hexes)
    os.makedirs(os.path.dirname(CACHE), exist_ok=True)
    with open(CACHE, 'wb') as f:
        pickle.dump(data, f)
    return data


def analyse(rs, asm_hexes, m):
    """Fill in callers, external references, uses_t, abs_own."""
    by_start = {r.r0: r for r in rs}
    dseg = {}
    for s in m.segments:
        if s.kind == 'D' and s.size:
            dseg.setdefault(s.name, s)
    for r in rs:
        for t in r.bsrs:
            c = by_start.get(t)
            if c is not None:
                c.callers_bsr.add(r.r0)
        for h in r.hexes:
            c = by_start.get(h)
            if c is not None and c.segaddr != r.segaddr:
                c.ext_refs.add(r.r0)
    for sa, a, h in asm_hexes:
        c = by_start.get(h)
        if c is not None:
            c.ext_refs.add(a)
    # the segment's data: the D segment its A5 loads point into, else the
    # D segment of the same name
    seg_data = {}
    for r in rs:
        for l in r.leas:
            d = m.segment_containing(l)
            if d is not None and d.kind == 'D':
                seg_data.setdefault(r.segaddr, set()).add((d.addr, d.end, d.name))
    for r in rs:
        ds = seg_data.get(r.segaddr)
        if not ds and r.seg in dseg:
            ds = {(dseg[r.seg].addr, dseg[r.seg].end, r.seg)}
        r.data = ds or set()
        r.abs_own = any(lo <= h < hi for h in r.hexes for lo, hi, _ in r.data
                        if h not in r.leas)
    # uses closed over bsr to callees that do not set A5
    memo = {}

    def ut(r, stack):
        if r.r0 in memo:
            return memo[r.r0]
        if r.r0 in stack:
            return r.uses
        stack.add(r.r0)
        v = r.uses
        for t in r.bsrs:
            c = by_start.get(t)
            if c is not None and not c.sets and ut(c, stack):
                v = True
        stack.discard(r.r0)
        memo[r.r0] = v
        return v
    for r in rs:
        r.uses_t = ut(r, set())
    return by_start


def letter(r):
    """One letter per routine: S sets A5 and uses it (uses*), s sets it and
    never uses it, i uses it without setting it (inherits), n neither;
    `.' a routine with no `link' (a bare `rts' stub, for instance)."""
    if not r.has_link:
        return '.'
    if r.sets:
        return 'S' if r.uses_t else 's'
    return 'i' if r.uses_t else 'n'


def switch(letters):
    """Where A5 loading switches from `used' to `always' in one segment
    (codegen-patterns 4).  `letters' is [(start, letter)] for the
    segment's exported routines in address order (letter() values).

    Returns (last, first, late): `first' is the start of the first s
    routine (sets A5 without using it), `last' the start of the last n
    routine before it (no A5 although exported), each None if there is
    none; the switch lies after `last' and at or before `first'.  `late'
    lists the n routines after `first': routines the one-switch model does
    not explain.  S routines fit either mode and are not used."""
    first = next((a for a, l in letters if l == 's'), None)
    last = None
    for a, l in letters:
        if l == 'n' and (first is None or a < first):
            last = a
    late = [a for a, l in letters if l == 'n' and first is not None and a > first]
    return last, first, late


def segments(rs):
    """{segment start: [Routine]} in address order."""
    out = defaultdict(list)
    for r in rs:
        out[r.segaddr].append(r)
    return out


def exported(rr):
    """The routines of a segment that count as exported: map-named ones."""
    return [r for r in rr if r.name]


def summary(m, rs, by_start):
    setseg = {r.segaddr for r in rs if r.sets}
    print('routines: %d in %d compiled segments; %d segments load A5'
          % (len(rs), len(segments(rs)), len(setseg)))
    c = Counter(letter(r) for r in rs if r.name and r.segaddr in setseg)
    print('map-named routines in segments that load A5:', dict(c))
    d = Counter()
    for r in rs:
        if r.name and r.sets and r.uses_t:
            one = r.uses or any(by_start.get(t) is not None and not by_start[t].sets
                                and by_start[t].uses for t in r.bsrs)
            d['direct' if r.uses else 'one bsr level' if one else 'deeper'] += 1
    print('S routines by how they use A5:', dict(d))
    c = Counter(letter(r) for r in rs if r.name and r.segaddr not in setseg)
    print('map-named routines in segments that never load A5:', dict(c))
    c = Counter((letter(r), bool(r.ext_refs)) for r in rs if not r.name)
    print('unnamed routines (letter, referenced from outside):', dict(c))
    # the switch model
    kinds, late, cross = Counter(), [], 0
    for sa, rr in segments(rs).items():
        if sa not in setseg:
            continue
        ls = [(r.r0, letter(r)) for r in exported(rr) if letter(r) != '.']
        last, first, lt = switch(ls)
        kinds['no s (load iff used)' if first is None else
              'no n before first s' if last is None else 'n..s switch'] += 1
        late += [(rr[0].seg, a) for a in lt]
        if first is not None:
            for r in rr:
                for t in r.bsrs:
                    cb = by_start.get(t)
                    if cb is not None and not cb.name and (r.r0 >= first) != (t >= first):
                        cross += 1
    print('segments by switch shape:', dict(kinds))
    print('n routines after the first s: %d: %s' % (len(late), ', '.join(
        '%s 0x%X' % x for x in late)))
    print('bsr calls to unnamed routines across the first s: %d' % cross)


def bsr_kind(caller, target, callee):
    """Classify one bsr from Routine `caller' to `target' (`callee' the
    Routine starting there, or None): None when it is not a call to
    another routine of the same segment, else (direction, caller kind,
    callee kind) with direction 'fwd' (the callee's body is after the
    caller's) or 'back', kinds 'named' (map symbol) or 'local'."""
    if callee is None or callee.segaddr != caller.segaddr:
        return None
    if caller.r0 <= target < caller.r1:
        return None
    return ('fwd' if target > caller.r0 else 'back',
            'named' if caller.name else 'local',
            'named' if callee.name else 'local')


def order(rs, by_start, verbose):
    sites, pairs = Counter(), defaultdict(set)
    for r in rs:
        for t in r.bsrs:
            k = bsr_kind(r, t, by_start.get(t))
            if k is not None:
                sites[k] += 1
                pairs[k].add((r.r0, t))
    print('direction caller callee     sites pairs segments')
    for k in sorted(sites):
        segs = {by_start[a].seg for a, _ in pairs[k]}
        print('%-9s %-6s %-10s %5d %5d %4d' % (k + (sites[k], len(pairs[k]), len(segs))))
    if verbose:
        for k in sorted(pairs):
            if k[0] != 'fwd':
                continue
            for a, t in sorted(pairs[k]):
                print('%s %s->%s  %06X %-30s -> %06X %s' % (
                    k[0], k[1], k[2], a, by_start[a].name or 'module-local',
                    t, by_start[t].name or 'module-local'))


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    m = sau2map.load()
    rs, asm_hexes = load('--refresh' in argv)
    argv = [a for a in argv if a != '--refresh']
    by_start = analyse(rs, asm_hexes, m)
    cmd = argv[0]
    if cmd == 'summary':
        summary(m, rs, by_start)
    elif cmd == 'runs':
        for sa, rr in sorted(segments(rs).items()):
            if argv[1:] and rr[0].seg not in argv[1:]:
                continue
            line = ''.join(letter(r) if r.name else '(%s)' % letter(r) for r in rr)
            print('%06X %-16s %s' % (sa, rr[0].seg, line))
    elif cmd == 'switch':
        for sa, rr in sorted(segments(rs).items()):
            if (argv[1:] and rr[0].seg not in argv[1:]) or not any(r.sets for r in rr):
                continue
            ex = [r for r in exported(rr) if letter(r) != '.']
            last, first, late = switch([(r.r0, letter(r)) for r in ex])
            print('%06X %-16s %s' % (sa, rr[0].seg, ''.join(letter(r) for r in ex)))
            if first is None:
                print('  no s routine: load A5 iff used')
                continue
            print('  last n before the switch: %s' % (
                '0x%X %s' % (last, by_start[last].name) if last else 'none'))
            print('  first s (always from here): 0x%X %s' % (first, by_start[first].name))
            if late:
                print('  n after it (unexplained): %s' % ', '.join(
                    '0x%X %s' % (a, by_start[a].name) for a in late))
    elif cmd == 'list':
        want = argv[1] if len(argv) > 1 else ''
        for r in rs:
            l = letter(r)
            ok = {'set-unused': r.name and l == 's',
                  'none': r.name and l == 'n' and r.data,
                  'unnamed-set': not r.name and r.sets,
                  'abs-only': not r.sets and r.abs_own}.get(want)
            if ok:
                print('%06X %-14s %-32s %s save=%s ext=%d bsr-callers=%d' % (
                    r.r0, r.seg, r.name or 'module-local', l, r.save,
                    len(r.ext_refs), len(r.callers_bsr)))
    elif cmd == 'order':
        order(rs, by_start, '-v' in argv[1:])
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == '__main__':
    # run through the importable module so that the pickled cache names
    # a5survey.Routine, not __main__.Routine
    import a5survey
    sys.exit(a5survey.main(sys.argv[1:]))
