#!/usr/bin/env python3
"""alldis.py SAU CPU OUT: disassemble every map routine of a SAU image
(read-only) into OUT, one `== NAME ADDR' header per routine, for
tools/cachesurvey.py.  CPU is objdump's m68k variant (68000 for the
MC68010 SAUs, 68020 for SAU6-9).  Reads the install's image and map
through ../source's domain_os/tools/image_file.py (read-only)."""
import sys, re, subprocess, tempfile, os
sys.path.insert(0, '/Users/toshok/src/domainos-archeology/source/domain_os/tools')
from image_file import Image
sau, cpu, out = sys.argv[1], sys.argv[2], sys.argv[3]
d = f'/Users/toshok/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2/sau{sau}'
mp = d + '/domain_os.map'
img = Image(mp, d + '/domain_os')
addrs = []
for l in open(mp, errors='replace'):
    m = re.match(r'^\s+([0-9A-F]{6,8})\s+(\S+)\s*$', l)
    if m: addrs.append((int(m.group(1), 16), m.group(2)))
addrs = sorted(set(addrs))
starts = sorted({a for a, _ in addrs})
with open(out, 'w') as o:
    for i, (a, n) in enumerate(addrs):
        nxt = [s for s in starts if s > a]
        if not nxt: continue
        ln = min(nxt[0] - a, 0x4000)
        b = img.read(a, ln)
        if not b: continue
        with tempfile.NamedTemporaryFile(delete=False) as f: f.write(b)
        r = subprocess.run(['m68k-elf-objdump', '-D', '-b', 'binary', '-m', 'm68k:' + cpu,
                            '--adjust-vma=' + hex(a), f.name], capture_output=True, text=True).stdout
        os.unlink(f.name)
        o.write('== %s %x\n' % (n, a))
        for l in r.splitlines():
            w = l.split()
            if w and w[0].endswith(':') and all(c in '0123456789abcdef' for c in w[0][:-1]): o.write(l + '\n')
