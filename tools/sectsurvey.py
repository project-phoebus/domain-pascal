#!/usr/bin/env python3
"""sectsurvey.py - modules spread over several sections of the SR10.2 image.

Evidence tool for docs/codegen-patterns.md section 11 and 1.6 (bead
domain-pascal-5aw.10).

    tools/sectsurvey.py modules     # per module name with code segments in
                                    # more than one output section: every
                                    # segment as KIND:SECTION@ADDR+SIZE
    tools/sectsurvey.py calls       # calls between routines of one module
                                    # name: counts by (instruction, same or
                                    # other segment, callee named or not,
                                    # callee's a5survey letter)
    tools/sectsurvey.py calls -v    # ... and every call into another
                                    # segment, and every `jsr' that stays
                                    # in its segment

A segment's output section is the map section (a map line with a section
number and a non-zero size) of its kind that contains its address.  The
routines and their reachable instructions are tools/a5survey.py's
(compiled code only, hand assembly excluded).

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import a5survey  # noqa: E402
import sau2map  # noqa: E402

_CALL = re.compile(r'^(jsr|bsr[sw]?)\s+0x([0-9a-f]+)$')


def output_section(m, seg):
    """The name of the map section of `seg''s kind containing it, or '?'."""
    for s in m.sections:
        if s.size and s.kind == seg.kind and s.addr <= seg.addr < s.end:
            return s.name
    return '?'


def modules(m):
    """{module name: [(kind, section, addr, size)]} for the names with code
    segments in more than one output section, in map order."""
    by = defaultdict(list)
    for g in m.segments:
        by[g.name].append((g.kind, output_section(m, g), g.addr, g.size))
    return {n: l for n, l in by.items()
            if len({sec for k, sec, _, _ in l if k == 'I'}) > 1}


def call_kind(insn):
    """'jsr' or 'bsr' for a direct call instruction text, else None."""
    c = _CALL.match(insn)
    return (c.group(1)[:3], int(c.group(2), 16)) if c else None


def calls(m, verbose=False, out=print):
    rs, asm = a5survey.load()
    by = a5survey.analyse(rs, asm, m)
    counts = Counter()
    for r in rs:
        for a, _, t in a5survey.reached(r.r0, r.r1):
            ck = call_kind(t)
            if ck is None:
                continue
            kind, tgt = ck
            c = by.get(tgt)
            if c is None or c.seg != r.seg:
                continue
            same = c.segaddr == r.segaddr
            counts[(kind, 'same-seg' if same else 'other-seg',
                    'named' if c.name else 'unnamed', a5survey.letter(c))] += 1
            if verbose and (not same or kind == 'jsr'):
                out('0x%X %s %s %s -> 0x%X %s %s'
                    % (a, r.seg, r.name or 'module-local@0x%X' % r.r0, kind, tgt,
                       c.name or 'module-local', a5survey.letter(c)))
    for k, v in sorted(counts.items()):
        out('%-4s %-9s %-7s %s  %d' % (k + (v,)))
    return counts


def main(argv):
    m = sau2map.load()
    if argv[:1] == ['modules']:
        for n, l in modules(m).items():
            print(n, ' '.join('%s:%s@%X+%X' % x for x in l))
        return 0
    if argv[:1] == ['calls']:
        calls(m, verbose='-v' in argv[1:])
        return 0
    print(__doc__.split('\n\n')[1], file=sys.stderr)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
