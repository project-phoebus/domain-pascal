#!/usr/bin/env python3
"""datasurvey.py - how the compiled code of the SR10.2 image lays out
module data blocks (the static variables of a module, D segments).

Evidence tool for docs/codegen-patterns.md section 11, "Byte-sized module
variables" (bead domain-pascal-yvko); the frame counterpart is
tools/framesurvey.py.

Every compiled routine (tools/a5survey.py's split, hand assembly left out)
is followed from its start (a5survey.reached).  Its data operands are
mapped to image addresses:

- `(d,A5)' operands: A5 is the routine's own `lea <abs>.l,A5' target, or,
  for a routine that does not load A5, the target every loading routine
  of its code segment agrees on (else the routine is skipped);
- absolute operands `0xADDR' (not branch, bsr or jsr targets).

Only addresses inside a map D segment are kept.  Each is classed by the
accesses to it: `b', `w', `l' (operation size; Scc and the memory bit
instructions are byte operations), `a' (`pea'/`lea': its address is
taken), `x' (the displacement of an indexed operand: an array or table
starts there).  A *lone byte* is an address accessed only as a byte (and
perhaps by address) with no other accessed address in its word; it is
`near-x' (reported apart: it may be an element of an array) when the
nearest accessed address below it in its block is an indexed base.

Blocks are split by kind: `code' (a D segment named like a compiled code
segment: the module's own data) and `other' (any other D segment: hand
assembly or data-only modules).

usage:
    tools/datasurvey.py align        # lone bytes by block kind and offset
                                     # parity (in its block, and absolute)
    tools/datasurvey.py odd          # the lone bytes at odd offsets, with
                                     # their neighbours, for inspection
    tools/datasurvey.py pairs        # lone bytes in adjacent words
                                     # (byte variables not packed two to a
                                     # word), and words whose two bytes are
                                     # both accessed only as bytes
    tools/datasurvey.py names        # map symbols in D segments by
                                     # address parity, and the map-named
                                     # variables of compiled modules that
                                     # are accessed only as bytes, with
                                     # the distances to the next map
                                     # symbol and accessed offset
    tools/datasurvey.py block SEG    # every accessed offset of one D
                                     # segment (by name or hex address)

Python 3 standard library + m68k-elf-objdump.  Read-only.  The scan is
cached in tmp/datasurvey.cache (delete it after changing the scan).
"""

import bisect
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
                     'datasurvey.cache')

_A5 = re.compile(r'%a5@(?:\((-?\d+)(,%[ad]\d:[wl](?::\d)?)?\))?')
_ABS = re.compile(r'(?<![#\w])0x([0-9a-f]+)(?![\w@(])')
_SCC = re.compile(r's(ne|eq|mi|pl|gt|lt|ge|le|hi|ls|cc|cs|vc|vs|t|f)$')
_LEA_A5 = re.compile(r'^lea 0x([0-9a-f]+),%a5$')
_RESTORE = re.compile(r'^moveal %fp@\(-?\d+\),%a5$')


def access_size(mn):
    if mn in ('pea', 'lea'):
        return 'a'
    if mn in ('bset', 'bclr', 'btst', 'bchg') or _SCC.match(mn):
        return 'b'
    if mn[-1] in 'bwl':
        return mn[-1]
    return '?'


def operands(insns, a5):
    """[(insn addr, data addr, class)] for one routine's instructions."""
    out = []
    for a, n, t in insns:
        mn = t.split()[0]
        if mn.startswith(('movem', 'bsr', 'jsr', 'jmp', 'bra', 'db')) or \
                a5survey._BRANCH.match(t) or _LEA_A5.match(t) or _RESTORE.match(t):
            continue
        sz = access_size(mn)
        if a5 is not None:
            for m in _A5.finditer(t):
                if t == 'pea %a5@':
                    continue          # the A5 save, or a pointer to the block
                # objdump prints an indexed operand's displacement in hex
                # without `0x' (`%a5@(50,%d5:w)' is 0x50, bytes 3235 5050),
                # a plain one in decimal.
                d = int(m.group(1), 16 if m.group(2) else 10) if m.group(1) else 0
                out.append((a, a5 + d, 'x' if m.group(2) else sz))
        for m in _ABS.finditer(t):
            out.append((a, int(m.group(1), 16), sz))
    return out


def scan():
    m = sau2map.load()
    dsegs = sorted((s for s in m.segments if s.kind == 'D' and s.size),
                   key=lambda s: s.addr)
    starts = [s.addr for s in dsegs]
    code_names = {s.name for s in m.segments if s.kind == 'I'
                  and not loopalign.is_asm(s)}
    per_seg = defaultdict(list)
    for seg in m.segments:
        if seg.kind != 'I' or not (loopalign.LO <= seg.addr < loopalign.HI) \
                or not seg.size or loopalign.is_asm(seg):
            continue
        st = loopalign.routine_starts(m, seg)
        end = min(seg.end, loopalign.HI)
        for r0, r1 in zip(st, st[1:] + [end]):
            if r1 > r0:
                ins = a5survey.reached(r0, r1)
                leas = [int(mm.group(1), 16) for _, _, t in ins
                        for mm in [_LEA_A5.match(t)] if mm]
                per_seg[seg.addr].append((r0, ins, leas))
    acc = []      # (dseg addr, offset, class, insn addr)
    for segaddr, rs in per_seg.items():
        seg_leas = {x for _, _, leas in rs for x in leas}
        for r0, ins, leas in rs:
            if len(set(leas)) == 1:
                a5 = leas[0]
            elif not leas and len(seg_leas) == 1:
                a5 = next(iter(seg_leas))
            else:
                a5 = None
            for ia, da, c in operands(ins, a5):
                i = bisect.bisect_right(starts, da) - 1
                if i >= 0 and dsegs[i].contains(da):
                    acc.append((dsegs[i].addr, da - dsegs[i].addr, c, ia))
    blocks = {s.addr: (s.name, s.size, 'code' if s.name in code_names else 'other')
              for s in dsegs}
    return blocks, acc


def load():
    if os.path.exists(CACHE):
        with open(CACHE, 'rb') as f:
            return pickle.load(f)
    data = scan()
    os.makedirs(os.path.dirname(CACHE), exist_ok=True)
    with open(CACHE, 'wb') as f:
        pickle.dump(data, f)
    return data


def by_block(acc):
    b = defaultdict(lambda: defaultdict(set))
    sites = defaultdict(list)
    for blk, off, c, ia in acc:
        b[blk][off].add(c)
        sites[(blk, off)].append(ia)
    return b, sites


def lone_bytes(offs):
    """{offset: 'lone'|'near-x'} for the lone bytes of one block."""
    keys = sorted(offs)
    out = {}
    for o, cs in offs.items():
        if not cs <= {'b', 'a'} or 'b' not in cs:
            continue
        if (o ^ 1) in offs:
            continue
        i = bisect.bisect_left(keys, o) - 1
        out[o] = 'near-x' if i >= 0 and 'x' in offs[keys[i]] else 'lone'
    return out


def cmd_align(blocks, acc):
    b, _ = by_block(acc)
    cnt = Counter()
    for blk, offs in b.items():
        kind = blocks[blk][2]
        for o, k in lone_bytes(offs).items():
            cnt[(kind, k, 'even' if o % 2 == 0 else 'odd',
                 'abs-even' if (blk + o) % 2 == 0 else 'abs-odd')] += 1
    for k in sorted(cnt):
        print('%-6s %-7s %-5s %-9s %5d' % (k + (cnt[k],)))
    odd_blocks = Counter(blocks[blk][2] for blk in b if blk % 2)
    print('blocks with accesses: %d (%d at odd addresses: %s)' % (
        len(b), sum(odd_blocks.values()), dict(odd_blocks)))


def neighbours(offs, o):
    return ' '.join('%+d:%s' % (d, ''.join(sorted(offs[o + d])))
                    for d in (-2, -1, 1, 2) if o + d in offs)


def cmd_odd(blocks, acc):
    b, sites = by_block(acc)
    for blk in sorted(b):
        offs = b[blk]
        for o, k in sorted(lone_bytes(offs).items()):
            if o % 2:
                print('%-6s %-16s %06X+0x%-4X %-6s at %s  [%s]' % (
                    blocks[blk][2], blocks[blk][0], blk, o, k,
                    ','.join('%X' % x for x in sorted(set(sites[(blk, o)]))[:4]),
                    neighbours(offs, o)))


def cmd_pairs(blocks, acc):
    b, sites = by_block(acc)
    adj = Counter()
    shared = []
    for blk in sorted(b):
        offs = b[blk]
        lb = lone_bytes(offs)
        ev = sorted(o for o, k in lb.items() if k == 'lone' and o % 2 == 0)
        runs = [o for o in ev if o + 2 in lb and lb[o + 2] == 'lone']
        if runs:
            adj[blocks[blk][2]] += len(runs)
            print('adjacent %-6s %-16s %06X: %s' % (
                blocks[blk][2], blocks[blk][0], blk,
                ' '.join('+0x%X' % o for o in ev if o in runs or o - 2 in runs)))
        for o in sorted(offs):
            if o % 2 == 0 and o + 1 in offs and offs[o] <= {'b', 'a'} \
                    and offs[o + 1] <= {'b', 'a'} and 'b' in offs[o] and 'b' in offs[o + 1]:
                shared.append((blocks[blk][2], blocks[blk][0], blk, o))
    print('adjacent-word lone byte pairs:', dict(adj))
    for k, n, blk, o in shared:
        print('shared   %-6s %-16s %06X+0x%X/+0x%X' % (k, n, blk, o, o + 1))
    print('words with both bytes accessed only as bytes:',
          dict(Counter(k for k, *_ in shared)))


def cmd_block(blocks, acc, key):
    b, sites = by_block(acc)
    for blk, (name, size, kind) in sorted(blocks.items()):
        if key.upper() in (name, '%X' % blk, '0X%X' % blk):
            print('%s %06X size %X %s' % (name, blk, size, kind))
            for o in sorted(b.get(blk, {})):
                print('  +0x%-5X %-4s %s' % (o, ''.join(sorted(b[blk][o])),
                      ' '.join('%X' % x for x in sorted(set(sites[(blk, o)]))[:6])))


def cmd_names(blocks, acc):
    m = sau2map.load()
    b, _ = by_block(acc)
    starts = sorted(blocks)
    syms = sorted({s.addr: s for s in m.symbols}.values(), key=lambda s: s.addr)
    par = Counter()
    rows = []
    for i, s in enumerate(syms):
        j = bisect.bisect_right(starts, s.addr) - 1
        if j < 0:
            continue
        blk = starts[j]
        name, size, kind = blocks[blk]
        if not blk <= s.addr < blk + size:
            continue
        o = s.addr - blk
        par[(kind, 'odd' if s.addr % 2 else 'even')] += 1
        if s.addr % 2:
            print('odd symbol: %s 0x%X (%s %06X)' % (s.name, s.addr, name, blk))
        offs = b.get(blk, {})
        cs = offs.get(o, set())
        if kind != 'code' or 'b' not in cs or not cs <= {'b', 'a'} or o + 1 in offs:
            continue
        nsym = [t.addr - s.addr for t in syms[i + 1:i + 2] if t.addr < blk + size]
        nacc = [k - o for k in offs if k > o]
        rows.append((s.name, s.addr, name, o, nsym[0] if nsym else None,
                     min(nacc) if nacc else None))
    print('symbols in D segments by block kind and parity:', dict(par))
    fmt = lambda d: '+%d' % d if d is not None else '-'  # noqa: E731
    for n, a, seg, o, ds, da in rows:
        print('%-28s 0x%X %-13s +0x%-5X next symbol %-4s next access %s' % (
            n, a, seg, o, fmt(ds), fmt(da)))
    print('map-named byte variables: %d, at odd addresses: %d, '
          'next map symbol at +2: %d' % (
              len(rows), sum(1 for r in rows if r[1] % 2),
              sum(1 for r in rows if r[4] == 2)))


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    blocks, acc = load()
    if argv[1] == 'align':
        cmd_align(blocks, acc)
    elif argv[1] == 'odd':
        cmd_odd(blocks, acc)
    elif argv[1] == 'pairs':
        cmd_pairs(blocks, acc)
    elif argv[1] == 'names':
        cmd_names(blocks, acc)
    elif argv[1] == 'block' and len(argv) > 2:
        cmd_block(blocks, acc, argv[2])
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
