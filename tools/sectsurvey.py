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
    tools/sectsurvey.py otherdata   # operands naming a D segment of the
                                    # routine's module name: counts by
                                    # (own or other data section, code
                                    # section, data section, form), then
                                    # when a `movea.l #block,An' of another
                                    # section's block is loaded again
    tools/sectsurvey.py otherdata -v  # ... and the sites

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


# The data section paired with each code section (codegen-patterns 11).
PAIR = {'WIRED_PROC': 'WIRED_DATA', 'OS_INIT_PROC': 'OS_INIT_DATA',
        '.TEXT': '.DATA', 'PROCEDURE$': 'DATA$',
        'PROC2_CREATE_PRO': 'PROC2_CREATE_DAT',
        'PROC2_DELETE_PRO': 'PROC2_DELETE_DAT', 'RING_PROC': 'RING_DATA',
        'RTWIRED_CODE': 'RTWIRED_DATA', 'PBU_WIRED_PROC': 'PBU_WIRED_DATA',
        'SCSI_WIRED_PROC': 'SCSI_WIRED_DATA'}
_NUM = re.compile(r'#(\d+)|0x([0-9a-f]+)')
_MOVEA_IMM = re.compile(r'^moveal #(\d+),%(a\d)$')
_BR = re.compile(r'^(?:bra|b(?:hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le)|db\w+)[sw]?\s+'
                 r'(?:%d\d,)?0x([0-9a-f]+)')


def operand_form(t, imm):
    """The form of an operand naming a data block in instruction text T
    (IMM: as an immediate)."""
    if imm and re.match(r'moveal #\d+,%a\d$', t):
        return 'movea.l #blk,An'
    if imm and re.match(r'movel #\d+,%sp@-$', t):
        return 'move.l #addr,-(SP)'
    if imm:
        return 'immediate: ' + t.split()[0]
    if re.match(r'lea 0x[0-9a-f]+,%a5$', t):
        return 'lea blk,A5'
    if re.match(r'lea 0x[0-9a-f]+,%a\d$', t):
        return 'lea abs,An'
    if t.startswith('pea 0x'):
        return 'pea abs'
    return 'absolute: ' + t.split()[0]


def otherdata(m, verbose=False, out=print):
    """Operands (absolute or immediate) naming a D segment of the
    routine's own module name, in compiled code, and the reloads of
    another data section's block."""
    rs, asm = a5survey.load()
    a5survey.analyse(rs, asm, m)
    dsegs = defaultdict(list)
    for g in m.segments:
        if g.kind == 'D' and g.size:
            dsegs[g.name].append((g, output_section(m, g)))
    csec = {g.addr: output_section(m, g) for g in m.segments if g.kind == 'I'}
    counts, reloads = Counter(), Counter()
    for r in rs:
        own = PAIR.get(csec.get(r.segaddr))
        name = r.name or 'module-local@0x%X' % r.r0
        ins = a5survey.reached(r.r0, r.r1)
        blocks = set()
        for a, _, t in ins:
            if t.startswith(('jsr', 'bsr', 'bra', 'db')) or _BR.match(t):
                continue
            for mm in _NUM.finditer(t):
                v = int(mm.group(1)) if mm.group(1) else int(mm.group(2), 16)
                for g, dsec in dsegs.get(r.seg, []):
                    if not g.addr <= v < g.end + (1 if g.size == 4 else 0):
                        continue
                    rel = 'own' if dsec == own else 'other'
                    form = operand_form(t, bool(mm.group(1)))
                    counts[(rel, csec.get(r.segaddr), dsec, form)] += 1
                    if rel == 'other' and form == 'movea.l #blk,An':
                        blocks.add(v)
                    if verbose and rel == 'other':
                        out('0x%X %s %s %s +0x%X  %s' % (a, r.seg, name, dsec, v - g.addr, t))
        if not blocks:
            continue
        # Walk in address order: a block's register is ended by a call (or
        # an overwrite) and marked when a branch target intervenes.
        targets = {int(b.group(1), 16) for _, _, t in ins for b in [_BR.match(t)] if b}
        held, seen = {}, set()
        for a, _, t in ins:
            if a in targets:
                held = {k: (blk, 'label') for k, (blk, _) in held.items()}
            mv = _MOVEA_IMM.match(t)
            if mv and int(mv.group(1)) in blocks:
                blk, reg = int(mv.group(1)), mv.group(2)
                prev = [st for _, (b, st) in held.items() if b == blk]
                kind = ('first' if blk not in seen else
                        'after a call or overwrite' if not prev else
                        'while held, past a branch target' if 'label' in prev else
                        'while held')
                seen.add(blk)
                reloads[kind] += 1
                if verbose and kind.startswith('while'):
                    out('  reload 0x%X %s: %s' % (a, name, kind))
                held[reg] = (blk, 'ok')
                continue
            if t.startswith(('jsr', 'bsr', 'trap')):
                held = {}
                continue
            d = re.search(r',%(a\d)$', t)
            if d and d.group(1) in held and not t.startswith(('pea', 'cmp', 'tst')):
                del held[d.group(1)]
            for k, (blk, st) in held.items():
                if '%' + k + '@' in t:
                    reloads['use past a branch target' if st == 'label' else 'use'] += 1
    for k, v in sorted(counts.items(), key=lambda kv: tuple(map(str, kv[0]))):
        out('%-5s %-16s %-16s %-24s %d' % (k + (v,)))
    for k, v in sorted(reloads.items()):
        out('block register: %-34s %d' % (k, v))
    return counts, reloads


def main(argv):
    m = sau2map.load()
    if argv[:1] == ['modules']:
        for n, l in modules(m).items():
            print(n, ' '.join('%s:%s@%X+%X' % x for x in l))
        return 0
    if argv[:1] == ['calls']:
        calls(m, verbose='-v' in argv[1:])
        return 0
    if argv[:1] == ['otherdata']:
        otherdata(m, verbose='-v' in argv[1:])
        return 0
    print(__doc__.split('\n\n')[1], file=sys.stderr)
    return 2


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
