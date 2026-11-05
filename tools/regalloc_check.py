#!/usr/bin/env python3
"""regalloc_check.py - score dpc's register-held locals against the image.

    tools/regalloc_check.py TSV OBJDIR [-v] [--misses N]

TSV: one of tests/regalloc/sau<N>_high.tsv, ../source's high-confidence
table of locals (its register-allocation survey, 2026-11-04, of the SAU2
and SAU9 images: routine, fragment, local, kind, dpc frame offset, bytes,
type, image treatment `frame'/`register', image register or `-'),
restricted by ../source to routines whose frames agree with dpc's
(the offsets are dpc's layout at ffd368d with ../source 096ca5c7).
OBJDIR: the objects of ../source's Pascal for that SAU (build/sau<N>/pascal
of a ../source worktree after `make TARGET_SAU=sau<N> pascal').

For each row whose routine is a symbol of an object, dpc's treatment is
`frame' when the routine's code addresses any byte of the local's slot
(an `(-n,%fp)' operand overlapping it), else `register'.  Prints the
agreement by kind and treatment; with -v the misses.  Codegen-patterns
12.5.
"""
import argparse
import collections
import glob
import os
import re
import subprocess
import sys


def routines(obj):
    """{symbol: [instruction text]} of an object, via m68k-elf-objdump."""
    out = subprocess.run(['m68k-elf-objdump', '-d', obj], capture_output=True,
                         text=True).stdout
    cur = None
    res = {}
    for line in out.splitlines():
        m = re.match(r'[0-9a-f]+ <([^>]+)>:', line)
        if m:
            cur = m.group(1)
            res[cur] = []
            continue
        if cur and '\t' in line:
            parts = line.split('\t')
            if len(parts) >= 3:
                res[cur].append('\t'.join(parts[2:]))
    return res


def frame_offsets(insns):
    """The offset of every `%fp@(n)' operand."""
    offs = []
    for i in insns:
        for m in re.finditer(r'%fp@\((-?\d+)\)', i):
            offs.append(int(m.group(1)))
    return offs


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('tsv')
    ap.add_argument('objdir')
    ap.add_argument('-v', action='store_true')
    ap.add_argument('--misses', type=int, default=40)
    a = ap.parse_args()
    code = {}
    for obj in glob.glob(os.path.join(a.objdir, '**', '*.o'), recursive=True):
        for k, v in routines(obj).items():
            code.setdefault(k, v)
    tally = collections.Counter()
    misses = []
    absent = 0
    for line in open(a.tsv):
        if line.startswith('#'):
            continue
        f = line.rstrip('\n').split('\t')
        routine, frag, local, kind, off, size, ty, img, reg = f
        if routine not in code:
            absent += 1
            continue
        off, size = int(off), int(size)
        # An operand starting inside the slot (a wider access starting
        # below it is not counted).
        touched = any(off <= o < off + size for o in frame_offsets(code[routine]))
        dpc = 'frame' if touched else 'register'
        cls = kind
        tally[(cls, img, dpc == img)] += 1
        if dpc != img:
            misses.append((routine, local, ty, off, img, reg, dpc))
    total = sum(tally.values())
    hits = sum(n for (c, i, ok), n in tally.items() if ok)
    print(f'{os.path.basename(a.tsv)}: {hits}/{total} agree ({100.0 * hits / max(total, 1):.1f}%), '
          f'{absent} rows of routines not compiled')
    for c in sorted({c for c, _, _ in tally}):
        for i in ('register', 'frame'):
            ok, bad = tally[(c, i, True)], tally[(c, i, False)]
            if ok + bad:
                print(f'  {c:11} image {i:8}: {ok}/{ok + bad}')
    if a.v:
        for m in misses[:a.misses]:
            print('  miss %s %s (%s, %d): image %s %s, dpc %s' % m)


if __name__ == '__main__':
    main()
