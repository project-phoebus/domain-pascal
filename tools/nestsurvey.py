#!/usr/bin/env python3
"""nestsurvey.py - nested routines in the compiled code of the SR10.2 image.

Evidence tool for docs/codegen-patterns.md section 6 (bead
domain-pascal-5aw.3).  Routines are split as tools/a5survey.py does (map
symbols plus every `link.w A6' word, hand assembly excluded) and only the
instructions reachable from each start are looked at.

A routine is taken as *nested* when it is not map-named (nested routines
are local symbols) and reaches its enclosing frame in one of the two ways
the image shows (PMAP_$FLUSH 0xE1376C):

    A1     it reads A1 before writing it, on the straight-line entry path
           (0xE1359C `movea.l A1,A2'): the caller passed the static link
           in A1 (`movea.l A6,A1; bsr', `movea.l An,A1; bsr')
    dyn    it moves from its dynamic link `(A6)' (0xE13616 `movea.l (A6),A4'):
           the caller's frame pointer, which is the enclosing frame when
           only the parent calls it

Every `bsr' is classified by what the caller last wrote to A1 before it
(searching back over the call's argument pushes, not across another call
or a branch): `fp' (its own frame: the callee is nested in the caller),
`aN<-a1' / `aN<-(fp)' (a register holding the caller's own static or
dynamic link: the callee is a sibling, or the caller itself), `chain'
(`movea.l (d,An),A1': a walk up the static chain), `other', or `-' (A1
not written).

usage:
    tools/nestsurvey.py list        # one line per nested routine
    tools/nestsurvey.py summary     # counts quoted in codegen-patterns 6
    tools/nestsurvey.py entry       # per nested routine: the entry sequence
                                    # before the link copy | the copy |
                                    # parameter loads after it; counts; the
                                    # link's register by whether it calls
    tools/nestsurvey.py slots       # who addresses (-4..-1,A6): map-named,
                                    # unnamed, nested routines
    tools/nestsurvey.py frames      # the frame-size model: the same classes
                                    # split into functions (a caller reads
                                    # D0 after a call) and the rest, and
                                    # their smallest `link' sizes

Python 3 standard library + m68k-elf-objdump.  Read-only.  The per-routine
instructions are cached in tmp/nestsurvey.cache.
"""

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
                     'nestsurvey.cache')

_BSR = re.compile(r'^bsr[sw]?\s+0x([0-9a-f]+)')
_BRANCH = re.compile(r'^(b(?:ra|hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le)[sw]?|db\w+|jmp|rts)\b')


def split_ops(s):
    ops, depth, cur = [], 0, ''
    for ch in s:
        if ch == '(':
            depth += 1
        elif ch == ')':
            depth -= 1
        if ch == ',' and depth == 0:
            ops.append(cur.strip())
            cur = ''
        else:
            cur += ch
    if cur.strip():
        ops.append(cur.strip())
    return ops


def operands(t):
    parts = t.split(None, 1)
    return parts[0], (split_ops(parts[1]) if len(parts) > 1 else [])


def writes(t, reg):
    """True when instruction text t writes register `reg' (e.g. '%a1') as a
    whole (its destination operand is exactly that register)."""
    mn, ops = operands(t)
    if not ops or mn.startswith(('tst', 'cmp', 'btst', 'pea', 'jsr', 'bsr', 'jmp')):
        return False
    if mn.startswith('movem') and ops[-1].startswith('%'):
        return reg in re.split('[/-]', ops[-1])
    return ops[-1] == reg


def reads(t, reg):
    """True when t reads `reg': in a source operand, or as the base of the
    destination's address."""
    mn, ops = operands(t)
    if not ops:
        return False
    srcs = ops if mn.startswith(('tst', 'cmp', 'btst', 'pea', 'jsr', 'bsr')) else ops[:-1]
    if any(reg in o for o in srcs):
        return True
    d = ops[-1]
    return d != reg and reg in d


def a1_entry(insns):
    """Address of the first instruction that reads A1 before any write on
    the straight-line path from the start, or None.  Stops at a call or a
    branch (a call clobbers A1)."""
    for a, n, t in insns:
        if reads(t, '%a1'):
            return a
        if writes(t, '%a1') or _BRANCH.match(t) or t.startswith(('jsr', 'bsr')):
            return None
    return None


def dyn_reads(insns):
    """[(addr, text)] of the instructions with an `(A6)' operand (objdump
    `%fp@' without displacement)."""
    out = []
    for a, n, t in insns:
        _, ops = operands(t)
        if any(o == '%fp@' for o in ops):
            out.append((a, t))
    return out


def a1_source(insns, i):
    """What the caller last wrote to A1 before the bsr at index i, as
    (kind, addr, text); kind is one of fp, reg, chain, other, -."""
    j = i - 1
    while j >= 0:
        a, n, t = insns[j]
        if t.startswith(('jsr', 'bsr')) or _BRANCH.match(t):
            break
        if writes(t, '%a1'):
            mn, ops = operands(t)
            src = ops[0] if len(ops) == 2 else ''
            if mn == 'moveal' and src == '%fp':
                return ('fp', a, t)
            if mn == 'moveal' and re.fullmatch(r'%a[0-5]', src):
                return ('reg', a, t)
            if mn == 'moveal' and re.fullmatch(r'%(a[0-5]|fp)@(\(-?\d+\))?', src):
                return ('chain', a, t)
            return ('other', a, t)
        j -= 1
    return ('-', None, None)


def reg_origin(insns, reg, before):
    """Where the caller's `reg' (e.g. '%a4') came from: the text of its last
    write before address `before' in address order ('a1' for `movea.l
    A1,reg', '(fp)' for `movea.l (A6),reg', else the text)."""
    last = None
    for a, n, t in insns:
        if a >= before:
            break
        if writes(t, reg):
            last = t
    if last is None:
        return '?'
    mn, ops = operands(last)
    if mn == 'moveal' and ops[0] == '%a1':
        return 'a1'
    if mn == 'moveal' and ops[0] == '%fp@':
        return '(fp)'
    return last


class R:
    def __init__(self, seg, r0, r1, name, insns, link):
        self.seg, self.r0, self.r1, self.name = seg, r0, r1, name
        self.insns, self.link = insns, link


def scan():
    rs, _ = a5survey.load()
    out = []
    for r in rs:
        if not r.has_link:
            continue
        ins = a5survey.reached(r.r0, r.r1)
        d = loopalign.word(r.r0 + 2)
        out.append(R(r.seg, r.r0, r.r1, r.name, ins, d - 0x10000 if d & 0x8000 else d))
    return out


def load(refresh=False):
    if not refresh and os.path.exists(CACHE):
        with open(CACHE, 'rb') as f:
            return [R(*x) for x in pickle.load(f)]
    rs = scan()
    os.makedirs(os.path.dirname(CACHE), exist_ok=True)
    with open(CACHE, 'wb') as f:
        pickle.dump([(r.seg, r.r0, r.r1, r.name, r.insns, r.link) for r in rs], f)
    return rs


class Call:
    def __init__(self, caller, addr, callee, kind, src, origin):
        self.caller, self.addr, self.callee = caller, addr, callee
        self.kind, self.src, self.origin = kind, src, origin


def analyse(rs):
    """(by_start, calls_to): calls_to maps a routine start to its [Call]s."""
    by_start = {r.r0: r for r in rs}
    calls_to = defaultdict(list)
    for r in rs:
        for i, (a, n, t) in enumerate(r.insns):
            m = _BSR.match(t)
            if not m:
                continue
            tgt = int(m.group(1), 16)
            if tgt not in by_start or (r.r0 < tgt < r.r1):
                continue
            kind, sa, st = a1_source(r.insns, i)
            origin = None
            if kind == 'reg':
                origin = reg_origin(r.insns, operands(st)[1][0], sa)
            calls_to[tgt].append(Call(r, a, tgt, kind, st, origin))
    for r in rs:
        r.a1 = a1_entry(r.insns)
        r.dyn = dyn_reads(r.insns)
        r.calls = calls_to.get(r.r0, [])
    return by_start, calls_to


def nested(rs):
    """The nested routines: not map-named (a nested routine is a local
    symbol, ref 5.5.4 p. 5-17) and reaching an enclosing frame through A1
    or through a move from `(A6)' (a `lea (A6)' is not a read of the
    link: 0xE4C02C, 0xE7649E)."""
    return [r for r in rs if not r.name and (r.a1 is not None or dyn_moves(r))]


def dyn_moves(r):
    return [(a, t) for a, t in r.dyn if re.match(r'^move[al]* %fp@,', t)]


def call_desc(c):
    k = c.kind
    if k == 'reg':
        k = 'reg<-%s' % c.origin if c.origin in ('a1', '(fp)') else 'reg<-?'
    return k


def kind(r):
    return 'A1' if r.a1 is not None and not dyn_moves(r) else 'dyn' if r.a1 is None else 'A1+dyn'


def parents(r):
    """The enclosing routine(s) the calls show: the callers that pass their
    own frame in A1 (an A1 routine), or every caller (a dyn routine)."""
    if kind(r) == 'A1':
        return {c.caller.r0 for c in r.calls if c.kind == 'fp'}
    return {c.caller.r0 for c in r.calls}


def level(r, by_start, ns, seen=()):
    """1 for a routine nested in a top-level one, 2 below that, ...;
    None when no parent is seen."""
    ps = parents(r)
    if len(ps) != 1:
        return None
    p = next(iter(ps))
    if p not in ns or p in seen:
        return 1
    up = level(by_start[p], by_start, ns, seen + (r.r0,))
    return None if up is None else up + 1


def describe(r, by_start):
    callers = Counter((c.caller.r0, call_desc(c)) for c in r.calls)
    cs = ', '.join('%06X%s %s x%d' % (a, '' if not by_start[a].name else ' ' + by_start[a].name,
                                       k, n) for (a, k), n in sorted(callers.items()))
    return '%06X %-12s %-6s link %4d  callers: %s' % (
        r.r0, r.seg, kind(r), r.link, cs or 'none (by bsr)')


def summary(rs, by_start):
    ns = nested(rs)
    nss = {r.r0 for r in ns}
    print('routines with a link: %d; nested (not map-named; A1 read at entry, or a move '
          'from (A6)): %d' % (len(rs), len(ns)))
    print('by kind:', dict(Counter(kind(r) for r in ns)))
    # The rule: A1 iff some call comes from a routine other than the parent.
    rows = Counter()
    for r in ns:
        callers = {c.caller.r0 for c in r.calls}
        if not callers:
            rows[(kind(r), 'no bsr caller')] += 1
            continue
        ps = parents(r)
        if kind(r) == 'A1':
            other = callers - ps
            why = ('self' if r.r0 in other else '') + (
                'sibling/other' if other - {r.r0} else '')
            rows[(kind(r), 'non-parent caller: ' + (why or 'NONE'))] += 1
        else:
            rows[(kind(r), 'one caller' if len(callers) == 1 else
                  '%d callers' % len(callers))] += 1
    for k in sorted(rows):
        print('  %-4s %-40s %d' % (k + (rows[k],)))
    lv = Counter(level(r, by_start, nss) for r in ns)
    print('nesting levels (None: parent not seen):', dict(lv))
    before = after = 0
    for r in ns:
        for p in parents(r):
            if r.r0 < p:
                before += 1
            else:
                after += 1
    print('nested routine before its parent: %d, after: %d' % (before, after))
    sib = Counter()
    for r in ns:
        for c in r.calls:
            if c.caller.r0 != r.r0 and c.caller.r0 in nss and c.caller.r0 not in parents(r):
                sib['callee before caller' if r.r0 < c.caller.r0 else 'callee after caller'] += 1
    print('calls between nested routines other than parent->child:', dict(sib))


_LINK = re.compile(r'^move(a?)l (%a1|%fp@),(%[ad]\d|%fp@\(-\d+\))$')
_PARAM = re.compile(r'^move[bwl]?a?l? %fp@\((\d+)\),%[ad]\d$')


def entry(r):
    """(instructions before the link is copied, the link copy, parameter
    loads after it on the straight-line entry path): the save and the A5
    load left out."""
    before, link, after = [], None, []
    for a, n, t in r.insns[1:16]:
        if (t.startswith('moveml') and t.endswith('%sp@-')) or t in (
                'pea %a2@', 'pea %a5@', 'movel %d2,%sp@-') or t.startswith('lea 0x'):
            if link is None and not before:
                continue
        if _BRANCH.match(t) or t.startswith(('jsr', 'bsr')):
            break
        if link is None and _LINK.match(t):
            link = t
        elif link is None:
            before.append(t)
        elif _PARAM.match(t):
            after.append(t)
    return before, link, after


def entries(rs):
    ns = nested(rs)
    c = Counter()
    for r in ns:
        before, link, after = entry(r)
        params = [t for t in before if _PARAM.match(t)]
        c[('params before the link' if params else 'no parameter load before it') +
          (', a parameter load after it' if after else '')] += 1
        print('%06X %-6s %s | %s | %s' % (r.r0, kind(r), '; '.join(before), link,
                                          '; '.join(after)))
    print(dict(c))
    regs = Counter()
    for r in ns:
        _, link, _ = entry(r)
        calls = any(t.startswith(('jsr', 'bsr')) for _, _, t in r.insns)
        regs[(_LINK.match(link).group(3) if link else None, 'calls' if calls else 'no calls')] += 1
    print('link register:', dict(regs))


def own_offsets(r):
    """The negative frame offsets r addresses inside its `link' area."""
    o = set()
    for a, n, t in r.insns:
        if t.startswith('moveml %fp@('):
            continue
        for m in re.finditer(r'%fp@\((-\d+)(?:,|\))', t):
            d = int(m.group(1))
            if d >= r.link:
                o.add(d)
    return o


def slots(rs):
    """The 4 bytes below the frame pointer: map-named routines use them for
    locals, unnamed ones (internal and nested) leave them alone."""
    nss = {r.r0 for r in nested(rs)}
    for label, sel in (('map-named', lambda r: r.name),
                       ('unnamed, not nested', lambda r: not r.name and r.r0 not in nss),
                       ('nested', lambda r: r.r0 in nss)):
        c = Counter()
        odd = []
        for r in rs:
            if not sel(r):
                continue
            o = own_offsets(r)
            if r.link == 0:
                c['link #0'] += 1
            elif any(d >= -4 for d in o):
                c['(-4..-1,A6) addressed'] += 1
                odd.append(r.r0)
            elif o:
                c['frame used below -4 only'] += 1
            else:
                c['frame never addressed'] += 1
        print('%-20s %s' % (label, dict(c)))
        if label != 'map-named':
            print('    -4 slot addressed: %s' % ', '.join('0x%06X' % a for a in odd))


_JSR = re.compile(r'^jsr\s+0x([0-9a-f]+)')
_POP = re.compile(r'^(addq[wl]\s+#\d+,%sp|lea %sp@\(\d+\),%sp)$')


def reads_d0_after(insns, i):
    """Whether the instruction after the call at insns[i] (after its pop,
    if any) reads D0: a test or compare of it, or D0 as a source operand
    (codegen-patterns 3, "Function calls in expressions")."""
    j = i + 1
    if j < len(insns) and _POP.match(insns[j][2]):
        j += 1
    if j >= len(insns):
        return False
    op, args = operands(insns[j][2])
    if not args:
        return False
    if op.startswith(('tst', 'cmp')):
        return any(re.search(r'%d0\b', a) for a in args)
    return bool(re.search(r'%d0\b', args[0])) and not op.startswith(('moveq', 'clr', 'lea'))


def functions(rs, by_start):
    """The starts of the routines some caller reads D0 after calling."""
    fs = set()
    for r in rs:
        for i, (a, n, t) in enumerate(r.insns):
            m = _BSR.match(t) or _JSR.match(t)
            if m and int(m.group(1), 16) in by_start and reads_d0_after(r.insns, i):
                fs.add(int(m.group(1), 16))
    return fs


def frames(rs, by_start):
    """Frame sizes by class: a routine that is not a global symbol
    (unnamed: internal or nested) reserves a 4-byte slot at the top of its
    frame, a function a slot for its result variable."""
    nss = {r.r0 for r in nested(rs)}
    fs = functions(rs, by_start)
    for label, sel in (('map-named', lambda r: r.name),
                       ('unnamed, not nested', lambda r: not r.name and r.r0 not in nss),
                       ('nested', lambda r: r.r0 in nss)):
        for fl, fsel in (('functions', lambda r: r.r0 in fs), ('others', lambda r: r.r0 not in fs)):
            sel_rs = [r for r in rs if sel(r) and fsel(r)]
            sizes = Counter(-r.link for r in sel_rs)
            print('%-20s %-9s %4d  link sizes: %s' % (
                label, fl, len(sel_rs),
                ', '.join('%d:%d' % kv for kv in sorted(sizes.items())[:5])))
            small = [r for r in sel_rs if -r.link < (4 if label == 'map-named' else 8 if fl == 'functions' else 4)
                     and not (label == 'map-named' and fl == 'others')]
            if small:
                print('    below the model: %s' % ', '.join(
                    '0x%06X%s link #%d' % (r.r0, ' ' + r.name if r.name else '', r.link) for r in small))


def main(argv):
    if not argv:
        print(__doc__)
        return 2
    rs = load('--refresh' in argv)
    argv = [a for a in argv if a != '--refresh']
    by_start, _ = analyse(rs)
    cmd = argv[0]
    if cmd == 'list':
        for r in nested(rs):
            print(describe(r, by_start))
    elif cmd == 'summary':
        summary(rs, by_start)
    elif cmd == 'entry':
        entries(rs)
    elif cmd == 'slots':
        slots(rs)
    elif cmd == 'frames':
        frames(rs, by_start)
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
