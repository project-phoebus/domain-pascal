#!/usr/bin/env python3
"""framesurvey.py - how the compiled code of the SR10.2 image lays out frames.

Evidence tool for docs/codegen-patterns.md section 2, "Frame layout"
(beads domain-pascal-ek7, domain-pascal-67c).  Routines are split as
tools/nestsurvey.py does (its cache, tmp/nestsurvey.cache); only plain
`(d,A6)' operands with a negative displacement inside the routine's `link'
area are looked at (indexed operands and the `movem' restore are not).

Each offset is classed by the accesses to it: `b', `w', `l' (the operation
size), `a' (`pea'/`lea': its address is taken).  A *lone* object of size
k is an offset accessed only at size k (and perhaps by address) with no
other accessed offset inside its word (bytes) or its own extent (words,
longwords).  An offset below which some `pea'/`lea' takes a frame address
may be a field of a record or array there ("lowpea"); the other lone
objects are almost surely variables of their own.

Not seen: accesses by nested routines through the static link (a byte
the parent touches may share its word with one only a child touches),
`btst'/`bset'/`bclr' and other `?'-sized operands (they make an offset
not lone), `movem' and indexed `(d,A6,Xn)' operands.  Bytes sharing a
word with another accessed offset are never lone, so byte packing would
show only as lone bytes at odd offsets.

usage:
    tools/framesurvey.py align        # lone objects by size, class and
                                      # offset mod 4 (the alignment rule)
    tools/framesurvey.py byte-pairs   # routines with lone byte objects in
                                      # two adjacent words (bytes not
                                      # packed two to a word)
    tools/framesurvey.py order        # routines with a lone byte or word
                                      # object above a lone longword, and
                                      # with every longword above them
                                      # (frames are not sorted by size)

Python 3 standard library + m68k-elf-objdump (through nestsurvey).
Read-only.
"""

import os
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

_FP = re.compile(r'%fp@\((-\d+)\)')
_SCC = re.compile(r's(ne|eq|mi|pl|gt|lt|ge|le|hi|ls|cc|cs|vc|vs|t|f)$')
SIZE = {'b': 1, 'w': 2, 'l': 4}


def access_size(mnemonic):
    """`b', `w', `l', `a' (address taken) or `?' for an instruction that
    has a (d,A6) operand."""
    if mnemonic in ('pea', 'lea'):
        return 'a'
    if _SCC.match(mnemonic):
        return 'b'
    if mnemonic[-1] in 'bwl' and not mnemonic.startswith(('bsr', 'bra', 'jsr')):
        return mnemonic[-1]
    return '?'


def accesses(insns, link):
    """[(address, offset, size)] for every plain (d,A6) operand inside the
    `link' area (offset >= link, offset < 0)."""
    out = []
    for a, n, t in insns:
        if t.startswith('moveml'):
            continue
        mn = t.split()[0]
        for m in _FP.finditer(t):
            d = int(m.group(1))
            if link <= d < 0:
                out.append((a, d, access_size(mn)))
    return out


def offsets(insns, link):
    """{offset: set of access sizes}."""
    o = defaultdict(set)
    for _, d, sz in accesses(insns, link):
        o[d].add(sz)
    return o


def lone(offs, d, k):
    """Whether offset d holds a lone object of access size k."""
    s = offs.get(d, set())
    if k not in s or s - {k, 'a'}:
        return False
    lo, hi = (d - (d % 2), d - (d % 2) + 2) if k == 'b' else (d, d + SIZE[k])
    if any(o != d and lo <= o < hi for o in offs):
        return False
    # Nor inside another object's extent (the low byte of a longword).
    return not any(o < d < o + max(SIZE.get(z, 1) for z in s2)
                   for o, s2 in offs.items() if o != d)


def lowpea(offs, d):
    return any('a' in s for o, s in offs.items() if o < d)


def align_counts(routines):
    """Counter of (size, 'lowpea' | '-', d mod 4) over the lone objects."""
    c = Counter()
    for insns, link in routines:
        offs = offsets(insns, link)
        for d in offs:
            for k in 'bwl':
                if lone(offs, d, k):
                    c[(k, 'lowpea' if lowpea(offs, d) else '-', d % 4)] += 1
    return c


def byte_pairs(insns, link):
    """Offsets d of lone bytes with another lone byte at d-2, in a frame
    where no address is taken below them."""
    offs = offsets(insns, link)
    ds = [d for d in offs if lone(offs, d, 'b') and lone(offs, d - 2, 'b')]
    if ds and not lowpea(offs, min(ds) - 2):
        return sorted(ds, reverse=True)
    return []


def size_order(insns, link):
    """`small-above-long' when a lone byte or word object lies above
    (at a higher offset than) a lone longword, `longs-above' when every
    lone longword lies above them, None without both kinds."""
    offs = offsets(insns, link)
    small = [d for d in offs if lone(offs, d, 'w') or lone(offs, d, 'b')]
    longs = [d for d in offs if lone(offs, d, 'l')]
    if not small or not longs:
        return None
    return 'small-above-long' if max(small) > min(longs) else 'longs-above'


def main(argv):
    if not argv or argv[0] not in ('align', 'byte-pairs', 'order'):
        print(__doc__)
        return 2
    import nestsurvey
    rs = nestsurvey.load()
    if argv[0] == 'align':
        c = align_counts([(r.insns, r.link) for r in rs])
        for k in 'bwl':
            print('%s objects (offset mod 4):' % {'b': 'byte', 'w': 'word', 'l': 'longword'}[k])
            for cls in ('-', 'lowpea'):
                row = {m: c[(k, cls, m)] for m in range(4)}
                print('  %-7s %s' % (cls, '  '.join('%d:%d' % kv for kv in row.items())))
    elif argv[0] == 'order':
        c = Counter()
        for r in rs:
            k = size_order(r.insns, r.link)
            if k:
                c[(k, 'map-named' if r.name else 'unnamed')] += 1
        for k, v in sorted(c.items()):
            print('%-17s %-9s %d' % (k[0], k[1], v))
    else:
        n = 0
        for r in rs:
            ds = byte_pairs(r.insns, r.link)
            if ds:
                n += 1
                bs = sorted(set(ds) | {d - 2 for d in ds}, reverse=True)
                print('0x%06X %-28s link #%d  bytes at %s' % (
                    r.r0, r.name or '-', r.link, ', '.join(str(d) for d in bs)))
        print('%d routines' % n)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
