#!/usr/bin/env python3
"""indcallsurvey.py - calls through routine pointers in the SR10.2 image.

Evidence tool for docs/codegen-patterns.md section 1.6 (bead
domain-pascal-5aw.8).  Every map `I' segment in 0xE00800..0xE88834 is split
into routines as tools/loopalign.py does (map symbols and `link.w A6'
words); the instructions reachable from each routine's start
(a5survey.reached) are searched for `jsr (An)' and `jsr (d,An)'.  Per site:

    reg     the address register the call goes through
    where   where the last definition of that register lies relative to
            the argument pushes of the call (`after-pushes': between the
            last push and the `jsr'; `before'/`between'; `outside': not in
            the straight-line run before the call, i.e. before a label or
            branch)
    load    the instructions after the last push (the pointer's load)
    args    address registers defined among the pushes
    pop     what follows the call: `addq.w #n', `lea (n,SP)', or none
    reads   D0 / A0 read after the call before being written (the result)
    a5      an A5 write after the call other than the epilogue restore

The run before a call is cut at labels and at branches, calls, `movem',
`link' and returns (only the straight-line code that holds the pushes).

usage:
    tools/indcallsurvey.py summary     # counts quoted in codegen-patterns 1.6
    tools/indcallsurvey.py list        # one line per site
    tools/indcallsurvey.py addr        # how routine addresses are produced:
                                       # absolute immediates and PC-relative
                                       # operands that name a routine start

Python 3 standard library + m68k-elf-objdump.  Read-only.  The scan is
cached in tmp/indcallsurvey.cache (delete it after changing the scan).
"""

import os
import pickle
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import a5survey  # noqa: E402
import loopalign  # noqa: E402
import sau2map  # noqa: E402

CACHE = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'tmp',
                     'indcallsurvey.cache')

_JSR = re.compile(r'^jsr %a(\d)@(\(-?\d+\))?$')
_STOP = re.compile(r'^(b[a-z]+[sw]?|db[a-z]+|jsr|bsr[sw]?|rts|rte|jmp|link|movem|unlk)\b')
_DEST = re.compile(r'^\S+ .*,(%[ad]\d)$')
_IMM = re.compile(r'#?(0x[0-9a-f]+)|#(-?\d+)')


def dest(t):
    m = _DEST.match(t)
    if m:
        return m.group(1)
    m = re.match(r'^(?:clr[bwl]|s[a-z]{1,2}|ext[wl]|neg[bwl]|not[bwl]) (%[ad]\d)$', t)
    return m.group(1) if m else None


def is_push(t):
    return '%sp@-' in t or t.startswith('pea ') or t == 'subql #2,%sp'


def scan():
    """(sites, routine-address operands)."""
    m = sau2map.load()
    starts = {}
    segs = []
    for seg in m.segments:
        if seg.kind != 'I' or not (loopalign.LO <= seg.addr < loopalign.HI) or not seg.size:
            continue
        segs.append(seg)
        for s in loopalign.routine_starts(m, seg):
            starts[s] = seg.name
    sites, addrs = [], []
    for seg in segs:
        asm = loopalign.is_asm(seg)
        st = loopalign.routine_starts(m, seg)
        end = min(seg.end, loopalign.HI)
        for r0, r1 in zip(st, st[1:] + [end]):
            if r1 <= r0:
                continue
            names = m.names_at(r0)
            name = names[0] if names else None
            ins = a5survey.reached(r0, r1)
            labels = set()
            for a, n, t in ins:
                b = re.match(r'^(?:b[a-z]+[sw]?|db[a-z]+ %d\d,) ?(0x[0-9a-f]+)$', t)
                if b:
                    labels.add(int(b.group(1), 16))
            for i, (a, n, t) in enumerate(ins):
                if not asm and not t.startswith(('jsr', 'bsr', 'b', 'db', 'jmp')):
                    for mm in _IMM.finditer(t):
                        v = int(mm.group(1), 16) if mm.group(1) else int(mm.group(2))
                        v &= 0xffffffff
                        # a routine: a `link.w A6' (or a bare `rts') at v;
                        # code-section data such as UID_LIST is not
                        if (v in starts and v != r0 and (mm.group(0).startswith('#')
                                                         or '%pc@' in t)
                                and loopalign.word(v) in (0x4E56, 0x4E75)):
                            nm = m.names_at(v)
                            addrs.append((seg.name, a, t, v,
                                          nm[0] if nm else 'module-local %x' % v))
                j = _JSR.match(t)
                if not j:
                    continue
                sites.append(site(seg.name, asm, name, r0, ins, i, labels, j))
    return sites, addrs


def site(seg, asm, name, r0, ins, i, labels, j):
    a = ins[i][0]
    reg = '%a' + j.group(1)
    w = []
    k = i - 1
    while k >= 0 and not _STOP.match(ins[k][2]):
        w.append(ins[k])
        if ins[k][0] in labels:
            break
        k -= 1
    w.reverse()
    pushes = [x for x, (_, _, t) in enumerate(w) if is_push(t)]
    last = pushes[-1] if pushes else -1
    first = pushes[0] if pushes else len(w)
    defs = [x for x, (_, _, t) in enumerate(w) if dest(t) == reg]
    if not defs:
        where = 'outside'
    elif defs[-1] > last:
        where = 'after-pushes'
    elif defs[-1] < first:
        where = 'before'
    else:
        where = 'between'
    args = sorted({dest(t) for _, _, t in w[:last + 1]
                   if dest(t) and dest(t).startswith('%a')})
    load = [t for _, _, t in w[last + 1:]]
    after = ins[i + 1:i + 8]
    pop = 'none'
    if after:
        t = after[0][2]
        if re.match(r'^addqw #\d,%sp$', t) or re.match(r'^lea %sp@\(\d+\),%sp$', t):
            pop = t
    reads = set()
    for r in ('%d0', '%a0'):
        for _, _, t in after:
            ops = t.split(' ', 1)[1] if ' ' in t else ''
            if r in ops and dest(t) != r:
                reads.add(r)
                break
            if dest(t) == r or _STOP.match(t):
                break
    a5 = any(dest(t) == '%a5' and not t.startswith('moveal %fp@')
             for _, _, t in after)
    return dict(seg=seg, asm=asm, name=name, r0=r0, addr=a, reg=reg,
                disp=j.group(2), where=where, load=load, args=args, pop=pop,
                reads=sorted(reads), a5=a5)


def load_cache(refresh=False):
    if not refresh and os.path.exists(CACHE):
        with open(CACHE, 'rb') as f:
            return pickle.load(f)
    data = scan()
    os.makedirs(os.path.dirname(CACHE), exist_ok=True)
    with open(CACHE, 'wb') as f:
        pickle.dump(data, f)
    return data


def load_form(s):
    """The form of the instruction that defines the call register."""
    if not s['load']:
        return 'none'
    t = s['load'][-1]
    t = re.sub(r'%a(\d)', r'An', t)
    t = re.sub(r'%d(\d)', r'Dn', t)
    t = re.sub(r'-?\d+', 'd', t)
    return t


def summary(sites, addrs):
    comp = [s for s in sites if not s['asm']]
    print('jsr (An)/(d,An) sites: %d (%d in compiled code, %d in hand assembly)'
          % (len(sites), len(comp), len(sites) - len(comp)))
    print('  with a displacement (jsr (d,An)):', sum(1 for s in comp if s['disp']))
    print('  routines:', len({s['r0'] for s in comp}))
    print('register:', dict(Counter(s['reg'] for s in comp).most_common()))
    print('load placement:', dict(Counter(s['where'] for s in comp).most_common()))
    print('load form:', dict(Counter(load_form(s) for s in comp).most_common()))
    print('pop:', dict(Counter(s['pop'] for s in comp).most_common()))
    print('result read after the call:',
          dict(Counter(','.join(s['reads']) or '-' for s in comp).most_common()))
    print('A5 written after the call (not the epilogue restore):',
          sum(1 for s in comp if s['a5']))
    print('routine addresses as operands (compiled code): %d' % len(addrs))
    print('  forms:', dict(Counter(re.sub(r'0x[0-9a-f]+|-?\d+', 'N', t)
                                   for _, _, t, _, _ in addrs).most_common()))


def listing(sites):
    for s in sites:
        if s['asm']:
            continue
        print('%06x %-28s %-4s %-12s args=%-16s pop=%-18s reads=%-8s %s'
              % (s['addr'], s['name'] or 'module-local %x' % s['r0'], s['reg'],
                 s['where'], ','.join(s['args']), s['pop'],
                 ','.join(s['reads']) or '-', ' ; '.join(s['load'])))


def addr_listing(addrs):
    for seg, a, t, v, nm in addrs:
        print('%06x %-16s %-36s %s' % (a, seg, t, nm))


def main(argv):
    if len(argv) < 2 or argv[1] not in ('summary', 'list', 'addr'):
        print(__doc__)
        return 2
    sites, addrs = load_cache()
    if argv[1] == 'summary':
        summary(sites, addrs)
    elif argv[1] == 'list':
        listing(sites)
    else:
        addr_listing(addrs)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
