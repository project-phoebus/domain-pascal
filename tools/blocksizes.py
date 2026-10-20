#!/usr/bin/env python3
"""blocksizes.py - dpc's module data blocks against the map's D segments.

Beads domain-pascal-kplu and domain-pascal-2u4o: ../source's gen_layout_ld
places each dpc data block (`.moddata.<MODULE>[.<SECTION>]',
`.bss.moddata.<MODULE>[.<SECTION>]'; no section name is the map's
`.DATA') at the address of its module's D segment in that section, so a
block of another size moves what follows it, and a block that is too
long overlaps the next module's.  This compares, for every dpc object
given, each block's size with the map segment's.

    blocksizes.py [-v] OBJ_OR_DIR ...

A directory is searched for `*.o' (linktest's
tmp/linktest/source/domain_os/build/sau2/pascal).  Each block is listed
as `=' (same size), `+N' / `-N' (dpc's block N bytes longer / shorter)
or `?' (no single map segment: none, or several of the module in the
section with none of the block's size, as DISK_'s three units).  -v
lists the `=' ones too.  The summary counts each kind; the exit status
is 0.

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sau2map  # noqa: E402

BLOCK_RE = re.compile(r'^(?:\.bss)?\.moddata\.(?P<module>[^.]+)(?:\.(?P<section>.+))?$')


def map_segments(path=None):
    """{(module, section): [(addr, size)]} for the map's D segments, each
    in the D section above it in the map (the map lists a section, then
    its segments)."""
    out = {}
    current = None
    with open(path or sau2map.DEFAULT_MAP) as f:
        for line in f:
            m = sau2map._SEG_RE.match(line.rstrip('\n'))
            if not m or m.group('kind') != 'D':
                continue
            addr, size = int(m.group('addr'), 16), int(m.group('size'), 16)
            if m.group('num'):
                if size:
                    current = m.group('name')
                continue
            if current is not None:
                out.setdefault((m.group('name'), current), []).append((addr, size))
    return out


def blocks(obj):
    """[(section name, size)] of the dpc data blocks of object `obj'."""
    hdr = subprocess.run(['m68k-elf-objdump', '-h', obj], check=True,
                         capture_output=True, text=True).stdout
    found = []
    for line in hdr.splitlines():
        f = line.split()
        if len(f) >= 7 and f[0].isdigit() and BLOCK_RE.match(f[1]):
            found.append((f[1], int(f[2], 16)))
    return found


def objects(args):
    for a in args:
        if os.path.isdir(a):
            for root, _, files in sorted(os.walk(a)):
                for n in sorted(files):
                    if n.endswith('.o'):
                        yield os.path.join(root, n)
        else:
            yield a


def compare(segs, sec, size):
    """(kind, map addr, map size) for block `sec' of `size' bytes."""
    m = BLOCK_RE.match(sec)
    msec = (m.group('section') or '.DATA').upper()[:16]
    cands = [c for (mod, s), cs in segs.items()
             if mod == m.group('module') and s.upper()[:16] == msec for c in cs]
    if len(cands) > 1:
        same = [c for c in cands if c[1] == size]
        cands = same if len(same) == 1 else cands
    if len(cands) != 1:
        return '?', None, None
    addr, msize = cands[0]
    d = size - msize
    return ('=' if d == 0 else '%+d' % d), addr, msize


def main(argv):
    verbose = '-v' in argv
    args = [a for a in argv if a != '-v']
    if not args:
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        return 2
    segs = map_segments()
    counts = {}
    for obj in objects(args):
        for sec, size in blocks(obj):
            kind, addr, msize = compare(segs, sec, size)
            key = kind if kind in ('=', '?') else ('longer' if kind[0] == '+' else 'shorter')
            counts[key] = counts.get(key, 0) + 1
            if kind != '=' or verbose:
                where = '' if addr is None else ' map 0x%06X size 0x%X' % (addr, msize)
                print('%-6s %s: %s 0x%X%s' % (kind, os.path.basename(obj), sec, size, where))
    print('blocks: %s' % ', '.join('%s %d' % (k, counts[k]) for k in sorted(counts)))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
