#!/usr/bin/env python3
"""modsurvey.py - which map modules are next for whole-module dpc units.

Planning tool for bead domain-pascal-xdh.2 (tests/link/README.md,
"Whole-module units").  A whole-module unit compiles every routine of one
map code segment (`I' line of the SAU2 map) and links in place of all of
the module's C objects in ../source.  For every compiled-code segment in
the image window 0xE00800..0xE88834 (hand assembly left out, as
tools/loopalign.is_asm does, and so are the `*_END_ADDR' segments whose
only names are section markers `*_$PROC_START'/`*_$PROC_END') up to a
size limit, it prints:

    NAME ADDR SIZE  the map segment
    n=  routines: map names in it, plus `link.w A6' words at no map name
        (module-local routines, loopalign.routine_starts), shown as `+Nloc'
    cov= map names a unit of tests/conformance/manifest.txt reproduces
        (marked `*'); `WHOLE' when one unit has every routine of the segment
        (names at one address are one routine)
    data a D segment of the same name exists (module data: the unit needs a
        C owner for it, tests/link/README.md "Module data")
    C:  ../source's objects defining the segment's map names (from the
        scratch build of `make linktest', tmp/linktest/); `(noC)' after a
        name no C object defines (the link would add it)
    ASM: names defined by hand-written assembly (never dropped: a blocker)
    OUT: map names outside the segment that those C objects also define
        (a blocker: their code would be lost)
    HELP: other C globals of those objects (no map name; tools/linktest.py
        drops them with a whole-module unit if nothing else uses them)

Rows are ranked: blockers first excluded (ASM, OUT), then by the number
of module-local routines and by size, smallest first.  The survey says
nothing about whether dpc can compile the code yet: try it.

usage:
    tools/modsurvey.py [--max SIZE] [--all]

--max SIZE (hex, default 200) limits the segment size; --all also lists
segments with blockers, after the others.  Needs `make linktest' to have
built tmp/linktest/source/domain_os/build/sau2/ once.
"""

import glob
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import linktest  # noqa: E402
import loopalign  # noqa: E402
import sau2map  # noqa: E402

BUILD = os.path.join(REPO, 'tmp', 'linktest', 'source', 'domain_os', 'build', 'sau2')
CONFORM_MANIFEST = os.path.join(REPO, 'tests', 'conformance', 'manifest.txt')
LO, HI = 0xE00800, 0xE88834


class Row:
    def __init__(self, seg):
        self.seg = seg
        self.names = []         # map names in the segment, map order
        self.locals = 0         # routines at no map name
        self.covered = set()
        self.whole = False
        self.data = False
        self.cobjs = []
        self.noc = set()
        self.asm = set()
        self.out = set()
        self.helpers = set()

    @property
    def blocked(self):
        return bool(self.asm or self.out)

    def key(self):
        return (self.blocked, self.locals, self.seg.size, self.seg.addr)

    def format(self):
        names = ' '.join(n + ('*' if n in self.covered else '') +
                         ('(noC)' if n in self.noc else '') for n in self.names)
        parts = ['%-16s %06X %4X n=%d%s cov=%d%s%s' % (
            self.seg.name, self.seg.addr, self.seg.size, len(self.names),
            '+%dloc' % self.locals if self.locals else '', len(self.covered),
            ' WHOLE' if self.whole else '', ' data' if self.data else ''),
            names, '| C:' + (' '.join(self.cobjs) or '-')]
        for tag, xs in (('ASM', self.asm), ('OUT', self.out), ('HELP', self.helpers)):
            if xs:
                parts.append('%s:%s' % (tag, ' '.join(sorted(xs))))
        return ' '.join(parts)


def manifest_units(path):
    """[[routine spec]] of each unit of a conformance manifest (options
    and `NAME=...' tokens left out; `NAME:LEN' -> NAME)."""
    units = []
    with open(path) as f:
        for line in f:
            toks = line.split('#', 1)[0].split()
            if not toks:
                continue
            specs, rest = [], iter(toks[1:])
            for t in rest:
                if t == '--section-addr':
                    next(rest, None)
                elif not t.startswith('--') and '=' not in t:
                    specs.append(t.split(':')[0])
            units.append(specs)
    return units


def survey_row(seg, linkmap, defs, units, local_starts=(), asm_objs=()):
    """One Row for map segment `seg': `defs' as linktest.parse_nm gives
    it, `units' as manifest_units, `local_starts' the routine starts at no
    map name, `asm_objs' the hand-written objects."""
    r = Row(seg)
    r.names = [s.name for s in linkmap.symbols if seg.contains(s.addr)]
    r.locals = len(set(local_starts))
    r.data = linkmap.segment(seg.name, 'D') is not None
    names = set(r.names)
    # routines by address: names sharing one (CT_$PROC_START, CT_$RELEASE)
    # are one routine
    at = {}
    for s in linkmap.symbols:
        if seg.contains(s.addr):
            at.setdefault(s.addr, set()).add(s.name)
    for specs in units:
        hit = names & set(specs)
        r.covered |= hit
        if hit and not r.locals and all(ns & hit for ns in at.values()):
            r.whole = True
    definers = {}
    for o, d in defs.items():
        for n in names & set(d):
            definers.setdefault(n, set()).add(o)
    r.noc = names - set(definers)
    objs = set()
    for n, os_ in definers.items():
        for o in os_:
            if o in asm_objs:
                r.asm.add(n)
            else:
                objs.add(o)
    r.cobjs = sorted(objs)
    for o in objs:
        for g in set(defs[o]) - names:
            m = linkmap.lookup(g)
            (r.out if m is not None and m.name == g else r.helpers).add(g)
    return r


def is_marker(name):
    """A map name that marks a section boundary (CT_$PROC_START,
    MT_$PROC_END), not a routine."""
    return name.endswith(('_$PROC_START', '_$PROC_END'))


def survey(linkmap, defs, units, asm_objs=(), max_size=0x200, starts=None):
    """Rows for every compiled-code segment of the image window up to
    `max_size' bytes, ranked (Row.key).  `starts(seg)' gives the routine
    starts of a segment (default loopalign.routine_starts)."""
    starts = starts or (lambda seg: loopalign.routine_starts(linkmap, seg))
    rows = []
    for seg in linkmap.segments:
        if (seg.kind != 'I' or not seg.size or seg.size > max_size or seg.addr < LO
                or seg.end > HI or loopalign.is_asm(seg)):
            continue
        syms = [s for s in linkmap.symbols if seg.contains(s.addr)]
        if all(is_marker(s.name) for s in syms):
            # no names, or only section markers: the `*_END_ADDR' segments
            # (SM_END_ADDR 0xE6D6F8, CT_END_ADDR 0xE70AF8) hold a zero
            # longword, no routine
            continue
        named = {s.addr for s in syms}
        local = [a for a in starts(seg) if a not in named]
        rows.append(survey_row(seg, linkmap, defs, units, local, asm_objs))
    return sorted(rows, key=Row.key)


def build_defs():
    # the scratch build only (the dpc objects live outside build/)
    objs = sorted(glob.glob(os.path.join(BUILD, '**', '*.o'), recursive=True))
    if not objs:
        raise SystemExit('modsurvey: no objects under %s (run `make linktest` once)' % BUILD)
    defs = {}
    for i in range(0, len(objs), 300):
        out = subprocess.run([linktest.NM, '-A', '--defined-only'] + objs[i:i + 300],
                             capture_output=True, text=True, check=True).stdout
        for o, d in linktest.parse_nm(out).items():
            defs[os.path.relpath(o, BUILD)] = d
    asm = {o for o in defs if os.sep + 'sau2' + os.sep in os.sep + o}
    return defs, asm


def main(argv):
    max_size, show_all = 0x200, False
    args = iter(argv)
    for a in args:
        if a == '--max':
            max_size = int(next(args, '200'), 16)
        elif a == '--all':
            show_all = True
        else:
            print(__doc__)
            return 2
    linkmap = sau2map.load()
    defs, asm = build_defs()
    rows = survey(linkmap, defs, manifest_units(CONFORM_MANIFEST), asm, max_size)
    for r in rows:
        if show_all or not r.blocked:
            print(r.format())
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
