#!/usr/bin/env python3
"""Check the displacements gas assembled against their targets.

gas wraps a 16-bit displacement that is out of reach without a word
(`bra.w 1f` over 40000 bytes assembles to `6000 9C4A`), and it lengthens
an out-of-reach `dbf` unseen, so dpc's own layout (`emit.rs`,
`size_branches`) could drift from what gas emits.  This tool reads the
assembler listing (`as -aln=`) and, for every `b<cc>`/`bra`/`bsr` (`.s`
and `.w`), `db<cc>`, `(label,%pc)` operand and `case` table entry
`.short Nf-Mf` whose target is in the same file, checks that the
encoded displacement reaches the label (bead domain-pascal-pxov, from
the review's check).  Displacements the linker fills (a relocation at the
extension word: calls of other units' routines) are left to ld.

    usage: dispcheck.py file.s [as flags...]   (default -m68010)

Exit 0 when every displacement checked is right, 1 otherwise (each bad
one printed), 2 when the file does not assemble.  Library use:
`assemble_checked` (exectest, linktest) and `check_listing`.
"""

import os
import re
import subprocess
import sys
import tempfile

AS = 'm68k-elf-as'
OBJDUMP = 'm68k-elf-objdump'

_LINE = re.compile(r'^\s*(\d+)\s+([0-9a-f]{4,8}) ([0-9A-F ]+?)\s*\t(.*)$')
_CONT = re.compile(r'^\s*(\d+)\s+([0-9A-F ]+)\s*$')
_BARE = re.compile(r'^\s*(\d+)\s+\t(.*)$')


def parse_relocs(objdump_r):
    """{section: {offset: (type, symbol)}} from `objdump -r` output."""
    relocs, cur = {}, None
    for line in objdump_r.split('\n'):
        m = re.match(r'RELOCATION RECORDS FOR \[(.*)\]', line)
        if m:
            cur = m.group(1)
            relocs[cur] = {}
            continue
        m = re.match(r'([0-9a-f]{8}) (\S+)\s+(\S+)', line)
        if m and cur is not None:
            relocs[cur][int(m.group(1), 16)] = (m.group(2), m.group(3))
    return relocs


def _entries(listing):
    """[section, address or None, bytes, text] per listing line."""
    ent = []
    sect = '.text'
    for line in listing.split('\n'):
        m = _LINE.match(line)
        m2 = _CONT.match(line)
        if m:
            ent.append([sect, int(m.group(2), 16),
                        bytes.fromhex(m.group(3).replace(' ', '')), m.group(4).strip()])
        elif m2 and ent:
            ent[-1][2] += bytes.fromhex(m2.group(2).replace(' ', ''))
            continue
        else:
            m3 = _BARE.match(line)
            if not m3:
                continue
            ent.append([sect, None, b'', m3.group(2).strip()])
        t = ent[-1][3]
        ms = re.match(r'\.section\s+"?([^",\s]+)', t)
        if ms:
            sect = ms.group(1)
        elif re.match(r'\.(text|data|bss)\b', t):
            sect = '.' + re.match(r'\.(\w+)', t).group(1)
    # A label line takes the address of the next line with one in its
    # section, or the end of the last one.
    for i, e in enumerate(ent):
        if e[1] is not None:
            continue
        for j in range(i + 1, len(ent)):
            if ent[j][0] != e[0]:
                break
            if ent[j][1] is not None:
                e[1] = ent[j][1]
                break
        else:
            for j in range(i - 1, -1, -1):
                if ent[j][0] == e[0] and ent[j][1] is not None:
                    e[1] = ent[j][1] + len(ent[j][2])
                    break
    return ent


def _s16(b):
    v = int.from_bytes(b, 'big')
    return v - 0x10000 if v & 0x8000 else v


def _s8(v):
    return v - 0x100 if v & 0x80 else v


def check_listing(listing, relocs=None):
    """(number checked, [bad descriptions]) for a gas `-aln` listing;
    `relocs` as `parse_relocs` gives (displacements the linker fills are
    skipped)."""
    relocs = relocs or {}
    ent = _entries(listing)
    syms, numdefs = {}, {}
    for i, e in enumerate(ent):
        for m in re.finditer(r'^([A-Za-z_.$][\w.$]*|\d+):', e[3]):
            name = m.group(1)
            if name.isdigit():
                numdefs.setdefault((e[0], name), []).append((i, e[1]))
            else:
                syms[name] = (e[0], e[1])

    def resolve(sect, i, ref):
        m = re.fullmatch(r'(\d+)([fb])', ref)
        if m:
            defs = numdefs.get((sect, m.group(1)), [])
            if m.group(2) == 'f':
                c = [a for (j, a) in defs if j > i]
                return c[0] if c else None
            c = [a for (j, a) in defs if j < i]
            return c[-1] if c else None
        s = syms.get(ref)
        return s[1] if s and s[0] == sect else None

    bad, checked = [], 0
    for i, (sect, addr, by, text) in enumerate(ent):
        t = text.split('|')[0].strip()
        if not by or addr is None:
            continue
        linked = (addr + 2) in relocs.get(sect, {})
        m = re.match(r'(b[a-z]{1,3})\.([sw])\s+(\S+)$', t)
        if m and m.group(1) not in ('btst', 'bset', 'bclr', 'bchg'):
            if m.group(2) == 'w' and linked:
                continue
            tgt = resolve(sect, i, m.group(3))
            got = addr + 2 + (_s8(by[1]) if m.group(2) == 's' else _s16(by[2:4]))
            checked += 1
            if tgt is None or got != tgt:
                bad.append('%s 0x%x %s: reaches %s, not %s' % (
                    sect, addr, t, hex(got), tgt is not None and hex(tgt)))
            continue
        m = re.match(r'db[a-z]{1,2}\s+%d\d,(\S+)$', t)
        if m:
            tgt = resolve(sect, i, m.group(1))
            got = addr + 2 + _s16(by[2:4])
            checked += 1
            if len(by) != 4 or got != tgt:
                bad.append('%s 0x%x %s: %d bytes, reaches %s, not %s' % (
                    sect, addr, t, len(by), hex(got), tgt is not None and hex(tgt)))
            continue
        m = re.search(r'\(([^,()]+),%pc(,%[ad]\d\.[wl])?\)', t)
        if m:
            if not m.group(2) and linked:
                continue
            tgt = resolve(sect, i, m.group(1))
            if tgt is None:
                continue
            got = addr + 2 + (_s8(by[3]) if m.group(2) else _s16(by[2:4]))
            checked += 1
            if got != tgt:
                bad.append('%s 0x%x %s: reaches %s, not %s' % (sect, addr, t, hex(got), hex(tgt)))
            continue
        m = re.match(r'\.short\s+(\d+[fb])-(\d+[fb])$', t)
        if m:
            a, b = resolve(sect, i, m.group(1)), resolve(sect, i, m.group(2))
            if a is None or b is None:
                continue
            checked += 1
            if _s16(by[0:2]) != a - b:
                bad.append('%s 0x%x %s: %d, not %d' % (sect, addr, t, _s16(by[0:2]), a - b))
    return checked, bad


def assemble_checked(asm, obj, flags=('-m68010',), cwd=None, as_cmd=AS, objdump=OBJDUMP):
    """Assemble `asm` into `obj` and check its displacements: (as's
    CompletedProcess, [bad descriptions]).  The check is skipped (no bad
    ones) when the assembly fails."""
    fd, lst = tempfile.mkstemp(suffix='.lst')
    os.close(fd)
    try:
        p = subprocess.run([as_cmd] + list(flags) + ['-aln=' + lst, '-o', obj, asm],
                           cwd=cwd, capture_output=True, text=True)
        if p.returncode:
            return p, []
        with open(lst, errors='replace') as fh:
            listing = fh.read()
    finally:
        os.unlink(lst)
    objpath = obj if os.path.isabs(obj) or cwd is None else os.path.join(cwd, obj)
    r = subprocess.run([objdump, '-r', objpath], capture_output=True, text=True)
    _, bad = check_listing(listing, parse_relocs(r.stdout))
    return p, bad


def main(argv):
    if len(argv) < 2:
        print(__doc__.strip().split('\n\n')[2], file=sys.stderr)
        return 2
    asm = argv[1]
    flags = argv[2:] or ['-m68010']
    fd, obj = tempfile.mkstemp(suffix='.o')
    os.close(fd)
    try:
        p, bad = assemble_checked(asm, obj, flags)
    finally:
        os.unlink(obj)
    if p.returncode:
        sys.stderr.write(p.stderr)
        return 2
    for b in bad:
        print('BAD', b)
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
