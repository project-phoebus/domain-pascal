#!/usr/bin/env python3
"""sau2map.py - read the SR10.2 SAU2 kernel link map.

The map (~/src/domainos-archeology/sau2-maps/domain_os.10.2.map) is the
authoritative source of names and addresses (CLAUDE.md section 7).  Its
lines are, after a one-line build header:

    I 4  E00800  WIRED_PROC         loaded at 102000, size = 1D0F4
    D37       0  TRAP_PAGE          loaded at 0, size = 400
    I    E1471C  PROC1_             size = D88
         E148CA  PROC1_$TST_LOCK
         E151FE  EC_$INIT                         MARKED

- A line starting with `I' (code) or `D' (data) followed by a number and
  `loaded at' is an output SECTION of the link (WIRED_PROC, .TEXT, ...).
- A line starting with `I'/`D' without a number is a SEGMENT: one module's
  contribution to a section (`I E1471C PROC1_ size = D88').
- An indented line is a SYMBOL.  Some carry the flag `MARKED'.

All numbers are hex.  Symbols appear in address order.

Python 3 standard library only.  Usage as a module:

    m = sau2map.load()                     # default map path
    m.lookup('PROC1_$TST_LOCK').addr       # 0xE148CA
    m.routine_extent('PROC1_$TST_LOCK')    # (0xE148CA, 0x1C)
"""

import bisect
import os
import re
import sys

DEFAULT_MAP = os.environ.get(
    'SAU2_MAP',
    os.path.expanduser('~/src/domainos-archeology/sau2-maps/domain_os.10.2.map'))

_SEG_RE = re.compile(
    r'^(?P<kind>[ID])\s*(?P<num>\d+)?\s+(?P<addr>[0-9A-F]+)\s+(?P<name>\S+)\s+'
    r'(?:loaded at (?P<load>[0-9A-F]+),\s*)?size = (?P<size>[0-9A-F]+)\s*$')
_SYM_RE = re.compile(r'^\s+(?P<addr>[0-9A-F]+)\s+(?P<name>\S+)(?P<marked>\s+MARKED)?\s*$')


class Segment:
    """A section (number is not None) or a per-module segment of the map."""

    def __init__(self, kind, addr, name, size, number=None, load_addr=None):
        self.kind = kind            # 'I' (code) or 'D' (data)
        self.addr = addr
        self.name = name
        self.size = size
        self.number = number        # section number, None for a segment
        self.load_addr = load_addr  # `loaded at' (sections only)

    @property
    def end(self):
        return self.addr + self.size

    @property
    def is_section(self):
        return self.number is not None

    def contains(self, addr):
        return self.addr <= addr < self.end

    def __repr__(self):
        num = '' if self.number is None else str(self.number)
        return '%s%s %X %s size=%X' % (self.kind, num, self.addr, self.name, self.size)


class Symbol:
    def __init__(self, name, addr, marked=False):
        self.name = name
        self.addr = addr
        self.marked = marked

    def __repr__(self):
        return '%s=0x%X' % (self.name, self.addr)


class LinkMap:
    def __init__(self):
        self.header = ''
        self.sections = []      # Segment, is_section
        self.segments = []      # Segment, not is_section
        self.symbols = []       # Symbol, in map (address) order
        self.by_name = {}       # name -> Symbol
        self._addrs = []        # sorted distinct symbol addresses

    # ---- parsing -------------------------------------------------------
    @classmethod
    def parse(cls, text):
        m = cls()
        lines = text.splitlines()
        if lines and lines[0].startswith('Build ID'):
            m.header = lines.pop(0)
        for n, line in enumerate(lines, 2):
            if not line.strip():
                continue
            s = _SEG_RE.match(line)
            if s:
                seg = Segment(s.group('kind'), int(s.group('addr'), 16),
                              s.group('name'), int(s.group('size'), 16),
                              int(s.group('num')) if s.group('num') else None,
                              int(s.group('load'), 16) if s.group('load') else None)
                (m.sections if seg.is_section else m.segments).append(seg)
                continue
            y = _SYM_RE.match(line)
            if y:
                sym = Symbol(y.group('name'), int(y.group('addr'), 16),
                             bool(y.group('marked')))
                m.symbols.append(sym)
                m.by_name.setdefault(sym.name, sym)
                continue
            raise ValueError('map line %d not understood: %r' % (n, line))
        m._addrs = sorted({s.addr for s in m.symbols})
        return m

    # ---- lookups -------------------------------------------------------
    def lookup(self, name):
        """The Symbol called `name' (exact, then upper-cased), or None."""
        return self.by_name.get(name) or self.by_name.get(name.upper())

    def addr_of(self, name):
        s = self.lookup(name)
        return None if s is None else s.addr

    def names_at(self, addr):
        return [s.name for s in self.symbols if s.addr == addr]

    def segment(self, name, kind=None):
        """The first per-module segment called `name' (optionally of kind
        'I' or 'D'), or None."""
        for seg in self.segments:
            if seg.name == name and (kind is None or seg.kind == kind):
                return seg
        return None

    def segment_containing(self, addr):
        """The innermost segment (or, failing that, section) holding addr."""
        best = None
        for seg in self.segments + self.sections:
            if seg.size and seg.contains(addr):
                if best is None or seg.size < best.size:
                    best = seg
        return best

    def next_symbol_addr(self, addr):
        """The lowest symbol address strictly above addr, or None."""
        i = bisect.bisect_right(self._addrs, addr)
        return self._addrs[i] if i < len(self._addrs) else None

    def extent_at(self, addr):
        """(addr, length) of the code starting at addr: up to the next map
        symbol, or the end of the segment holding addr if that is closer."""
        seg = self.segment_containing(addr)
        nxt = self.next_symbol_addr(addr)
        ends = [e for e in (nxt, seg.end if seg else None) if e is not None]
        if not ends:
            raise ValueError('no symbol or segment end after 0x%X' % addr)
        return addr, min(ends) - addr

    def routine_extent(self, name):
        """(addr, length) of the routine the map calls `name'."""
        s = self.lookup(name)
        if s is None:
            raise KeyError(name)
        return self.extent_at(s.addr)

    def describe(self, addr):
        """A short name for an address: `SYM', `SYM+0xN' or `0xADDR'."""
        names = self.names_at(addr)
        if names:
            return names[0]
        i = bisect.bisect_right(self._addrs, addr) - 1
        if i >= 0:
            base = self._addrs[i]
            seg = self.segment_containing(addr)
            if seg is not None and seg.contains(base):
                return '%s+0x%X' % (self.names_at(base)[0], addr - base)
        return '0x%X' % addr


def load(path=None):
    with open(path or DEFAULT_MAP) as f:
        return LinkMap.parse(f.read())


def main(argv):
    """sau2map.py NAME|0xADDR ... : print address/extent/segment."""
    m = load()
    for arg in argv:
        if arg.lower().startswith('0x'):
            a = int(arg, 16)
            _, n = m.extent_at(a)
            print('0x%X %s len=0x%X seg=%r' % (a, m.describe(a), n, m.segment_containing(a)))
        else:
            s = m.lookup(arg)
            if s is None:
                print('%s: not in map' % arg)
                continue
            _, n = m.extent_at(s.addr)
            print('%s 0x%X len=0x%X seg=%r' % (s.name, s.addr, n, m.segment_containing(s.addr)))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
