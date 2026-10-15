#!/usr/bin/env python3
"""framesurvey.py - how the compiled code of the SR10.2 image lays out frames.

Evidence tool for docs/codegen-patterns.md section 2, "Frame layout"
(beads domain-pascal-ek7, domain-pascal-67c).  Routines are split as
tools/nestsurvey.py does (its cache, tmp/nestsurvey.cache); only plain
`(d,A6)' operands with a negative displacement inside the routine's `link'
area are looked at (indexed operands and the `movem' restore are not).

Each offset is classed by the accesses to it: `b', `w', `l' (the operation
size), `a' (`pea'/`lea': its address is taken).  A *lone* object of size
k is an offset accessed only at size k (and perhaps by address) with no
other accessed offset inside its word (bytes) or its own extent (words,
longwords).  An offset below which some `pea'/`lea' takes a frame address
may be a field of a record or array there ("lowpea"); the other lone
objects are almost surely variables of their own.

Not seen: accesses by nested routines through the static link (a byte
the parent touches may share its word with one only a child touches),
`btst'/`bset'/`bclr' and other `?'-sized operands (they make an offset
not lone), `movem' and indexed `(d,A6,Xn)' operands.  Bytes sharing a
word with another accessed offset are never lone, so byte packing would
show only as lone bytes at odd offsets.

usage:
    tools/framesurvey.py align        # lone objects by size, class and
                                      # offset mod 4 (the alignment rule)
    tools/framesurvey.py byte-pairs   # routines with lone byte objects in
                                      # two adjacent words (bytes not
                                      # packed two to a word)
    tools/framesurvey.py order        # routines with a lone byte or word
                                      # object above a lone longword, and
                                      # with every longword above them
                                      # (frames are not sorted by size)
    tools/framesurvey.py hidden [TREE]
                                      # frame bytes no instruction
                                      # addresses (bead domain-pascal-s9g0):
                                      # hypotheses for what the original
                                      # gave a slot though it kept it in a
                                      # register, scored over the image
                                      # (refutations: no room; untouched
                                      # frames of map-named procedures) and,
                                      # with TREE (../source's domain_os/),
                                      # against the bytes of its Pascal's
                                      # stand-in locals
    tools/framesurvey.py srclinks REPORT WORK_A WORK_B
                                      # two `tools/srcsurvey.py run' work
                                      # directories (one ../source tree,
                                      # two dpc builds; REPORT the stdout
                                      # of either): the routines whose
                                      # `link' size changed, against the
                                      # image's
    tools/framesurvey.py copies       # copies from a by-address slot into
                                      # the frame (value parameters passed
                                      # by address, bead
                                      # domain-pascal-5aw.12): the move
                                      # forms by size, the register rule of
                                      # the copies before the first branch
                                      # or call, where 8-byte copies sit

Python 3 standard library + m68k-elf-objdump (through nestsurvey).
Read-only.
"""

import os
import re
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

_FP = re.compile(r'%fp@\((-\d+)\)')
_SCC = re.compile(r's(ne|eq|mi|pl|gt|lt|ge|le|hi|ls|cc|cs|vc|vs|t|f)$')
SIZE = {'b': 1, 'w': 2, 'l': 4}


def access_size(mnemonic):
    """`b', `w', `l', `a' (address taken) or `?' for an instruction that
    has a (d,A6) operand."""
    if mnemonic in ('pea', 'lea'):
        return 'a'
    if _SCC.match(mnemonic):
        return 'b'
    if mnemonic[-1] in 'bwl' and not mnemonic.startswith(('bsr', 'bra', 'jsr')):
        return mnemonic[-1]
    return '?'


def accesses(insns, link):
    """[(address, offset, size)] for every plain (d,A6) operand inside the
    `link' area (offset >= link, offset < 0)."""
    out = []
    for a, n, t in insns:
        if t.startswith('moveml'):
            continue
        mn = t.split()[0]
        for m in _FP.finditer(t):
            d = int(m.group(1))
            if link <= d < 0:
                out.append((a, d, access_size(mn)))
    return out


def offsets(insns, link):
    """{offset: set of access sizes}."""
    o = defaultdict(set)
    for _, d, sz in accesses(insns, link):
        o[d].add(sz)
    return o


def lone(offs, d, k):
    """Whether offset d holds a lone object of access size k."""
    s = offs.get(d, set())
    if k not in s or s - {k, 'a'}:
        return False
    lo, hi = (d - (d % 2), d - (d % 2) + 2) if k == 'b' else (d, d + SIZE[k])
    if any(o != d and lo <= o < hi for o in offs):
        return False
    # Nor inside another object's extent (the low byte of a longword).
    return not any(o < d < o + max(SIZE.get(z, 1) for z in s2)
                   for o, s2 in offs.items() if o != d)


def lowpea(offs, d):
    return any('a' in s for o, s in offs.items() if o < d)


def align_counts(routines):
    """Counter of (size, 'lowpea' | '-', d mod 4) over the lone objects."""
    c = Counter()
    for insns, link in routines:
        offs = offsets(insns, link)
        for d in offs:
            for k in 'bwl':
                if lone(offs, d, k):
                    c[(k, 'lowpea' if lowpea(offs, d) else '-', d % 4)] += 1
    return c


def byte_pairs(insns, link):
    """Offsets d of lone bytes with another lone byte at d-2, in a frame
    where no address is taken below them."""
    offs = offsets(insns, link)
    ds = [d for d in offs if lone(offs, d, 'b') and lone(offs, d - 2, 'b')]
    if ds and not lowpea(offs, min(ds) - 2):
        return sorted(ds, reverse=True)
    return []


def size_order(insns, link):
    """`small-above-long' when a lone byte or word object lies above
    (at a higher offset than) a lone longword, `longs-above' when every
    lone longword lies above them, None without both kinds."""
    offs = offsets(insns, link)
    small = [d for d in offs if lone(offs, d, 'w') or lone(offs, d, 'b')]
    longs = [d for d in offs if lone(offs, d, 'l')]
    if not small or not longs:
        return None
    return 'small-above-long' if max(small) > min(longs) else 'longs-above'


_SLOT_LOAD = re.compile(r'moveal %fp@\((\d+)\),%a(\d)$')
_CTL = re.compile(r'(bsr|jsr|bra|b(hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le)[sw]?|db\w+|jmp|rts)$')
_SAVE = re.compile(r'(moveml \S+,%sp@-|movel %d\d,%sp@-|pea %a\d@|lea 0x[0-9a-f]+,%a5)$')


def slot_copy(insns, i):
    """When insns[i] is `movea.l (d,A6),An' (d >= 8) followed by moves
    from (An) into the frame: (slot d, register n, [(size letter, source
    form '0' | '+' | 'd', frame offset)]) and the index after them; else
    None.  Source forms: `(An)', `(An)+', `(k,An)'."""
    m = _SLOT_LOAD.match(insns[i][2])
    if not m:
        return None
    a = m.group(2)
    mv = re.compile(r'move([bwl]) %%a%s@(\+|\((\d+)\))?,%%fp@\((-\d+)\)$' % a)
    seq = []
    j = i + 1
    while j < len(insns):
        mm = mv.match(insns[j][2])
        if not mm:
            break
        form = '+' if mm.group(2) == '+' else ('d' if mm.group(2) else '0')
        seq.append((mm.group(1), form, int(mm.group(4))))
        j += 1
    if not seq:
        return None
    return int(m.group(1)), int(a), seq, j


def copy_forms(rs):
    """Counter of (bytes, form) over every slot copy, and an example
    address per key; form e.g. `l+ l+' (`move.l (An)+,..' twice)."""
    c, ex = Counter(), {}
    for r in rs:
        for i in range(len(r.insns)):
            got = slot_copy(r.insns, i)
            if got:
                _, _, seq, _ = got
                k = (sum(SIZE[x[0]] for x in seq), ' '.join(x[0] + x[1] for x in seq))
                c[k] += 1
                ex.setdefault(k, r.insns[i][0])
    return c, ex


def entry_copies(insns):
    """The slot copies before a routine's first branch or call, after the
    register save, the A5 load and the entry loads: [(slot, register,
    seq, fits)], `fits' whether the register is the lowest address
    register that no entry load holds and no earlier copy left a value in
    (one used up by `(An)+' counts as free; codegen-patterns 12.4
    R4(b))."""
    out = []
    held, used = set(), set()
    i = 1
    while i < len(insns) and _SAVE.match(insns[i][2]):
        i += 1
    while i < len(insns):
        t = insns[i][2]
        if _CTL.match(t.split()[0]):
            break
        got = slot_copy(insns, i)
        if got:
            slot, a, seq, j = got
            free = [x for x in range(5) if x not in held and x not in used]
            out.append((slot, a, seq, bool(free) and a == free[0]))
            if seq[-1][1] == '+':
                used.discard(a)
            else:
                used.add(a)
            i = j
            continue
        m = _SLOT_LOAD.match(t)
        if m and not out:
            held.add(int(m.group(2)))
        elif not out and re.match(r'move[bwl] %fp@\(\d+\),%d\d$', t):
            pass
        else:
            break
        i += 1
    return out


# ------------------------------------------------------------ hidden
#
# Frame bytes no instruction addresses (bead domain-pascal-s9g0; also
# -kwu, -nda): which register-held objects of the original kept a slot.

_FPX = re.compile(r'%fp@\((-\d+)(,%[ad]\d:[wl])?\)')
_ELEM = re.compile(r'^lea %a\d@\((-?\d+),%[ad]\d:[wl]\),%a\d$')
_SLOT_A = re.compile(r'^moveal %fp@\((\d+)\),(%a\d)$')


def used_bytes(insns, link):
    """The frame bytes inside the `link' area some instruction surely
    addresses: each plain or indexed `(d,A6)' operand covers d up to its
    access size (`b', `w', `l'; one byte for `pea'/`lea', other sizes and
    indexed operands, whose extent the code does not show).  A lower
    bound, so `link' minus it is an upper bound on the unaddressed bytes."""
    got = set()
    for _a, _n, t in insns:
        if t.startswith('moveml'):
            continue
        mn = t.split()[0]
        for m in _FPX.finditer(t):
            d = int(m.group(1))
            if not link <= d < 0:
                continue
            k = access_size(mn)
            n = 1 if m.group(2) or k not in SIZE else SIZE[k]
            got.update(range(d, min(d + n, 0)))
    return got


def elem_leas(insns):
    """`lea (d,An,Xn),Am': an element's address formed as a value."""
    return sum(1 for _a, _n, t in insns if _ELEM.match(t))


def dbf_loops(insns):
    return sum(1 for _a, _n, t in insns if t.startswith('dbf'))


def slot_reads(insns):
    """By-address parameter slots read in place, not copied into the
    frame: {slot d: access size letter} for `movea.l (d,A6),An' (d >= 8)
    followed, before An is written again, by a whole-object read `(An)'
    as the first operand of a move, `tst' or `cmp' (the value used in a
    register, pushed or tested; codegen-patterns 1.2, "Held in a register
    instead").  A slot also copied into the frame (`move (An),(k,A6)') is
    left out: its copy has a slot of its own."""
    out, copied = {}, set()
    for i, (_a, _n, t) in enumerate(insns):
        m = _SLOT_A.match(t)
        if not m:
            continue
        d, an = int(m.group(1)), m.group(2)
        for _a2, _n2, t2 in insns[i + 1:]:
            mn, ops = _ops(t2)
            if ops and ops[0] == an + '@' and mn[-1] in 'bwl' and \
                    mn.startswith(('move', 'tst', 'cmp')) and not mn.startswith('movem'):
                if len(ops) > 1 and _FP.search(ops[1]):
                    copied.add(d)
                else:
                    out.setdefault(d, mn[-1])
            if ops and ops[-1] == an and not mn.startswith(('tst', 'cmp', 'pea')):
                break
            if _CTL.match(mn):
                break
    return {d: k for d, k in out.items() if d not in copied}


_SAVED = re.compile(r'^(moveml (\S+),%sp@-|movel (%d\d),%sp@-|pea (%a\d)@)$')


def saved_regs(insns):
    """The callee-saved registers the prologue saves (D2-D7, A2-A4; A5
    left out, it holds the data area, section 4), as names like 'd2'."""
    out = set()
    for _a, _n, t in insns[1:3]:
        m = _SAVED.match(t)
        if not m:
            continue
        spec = m.group(2) or m.group(3) or m.group(4)
        for part in spec.split('/'):
            a, _, b = part.partition('-')
            a, b = a.lstrip('%'), (b or a).lstrip('%')
            for i in range(int(a[1]), int(b[1]) + 1):
                out.add(a[0] + str(i))
    return {x for x in out if x not in ('a5', 'a6', 'a7', 'd0', 'd1', 'a0', 'a1')}


_STEP = re.compile(r'^(addql #\d+,(%a\d)|lea (%a\d)@\(-?\d+\),(%a\d))$')


def stepped_regs(insns):
    """Address registers stepped in place (`addq.l #k,An', `lea
    (k,An),An'): strength-reduced pointers of loops (section 7.4)."""
    out = set()
    for _a, _n, t in insns:
        m = _STEP.match(t)
        if m and (m.group(2) or (m.group(3) == m.group(4))):
            out.add(m.group(2) or m.group(3))
    return out


def _ops(t):
    import nestsurvey
    return nestsurvey.operands(t)


def hidden_features(r, functions=(), nested=()):
    """The frame facts of routine R (a tools/nestsurvey.py routine)."""
    link = r.link
    used = used_bytes(r.insns, link)
    reserved = 0 if r.name else 4
    return {
        'L': -link,
        'free': -link - len(used),
        'untouched': not used,
        'reserved': reserved,
        'func': r.r0 in functions,
        'nested': r.r0 in nested,
        'elem': elem_leas(r.insns),
        'dbf': dbf_loops(r.insns),
        'slots': slot_reads(r.insns),
        'saved': saved_regs(r.insns),
        'stepped': stepped_regs(r.insns),
    }


def round4(n):
    return (n + 3) & ~3


# Each hypothesis: the bytes it says the routine's frame holds for
# objects the code keeps out of memory, from hidden_features.
HYPOTHESES = (
    ('none: no slot for anything held in a register', lambda f: 0),
    ('elem: 4 per element address formed as a value', lambda f: 4 * f['elem']),
    ('elem1: 4 when some element address is formed', lambda f: 4 * (f['elem'] > 0)),
    ('dbf: 4 per dbf loop', lambda f: 4 * f['dbf']),
    ('dbf2: 2 per dbf loop', lambda f: 2 * f['dbf']),
    ('slots: a copy per by-address slot read in place',
     lambda f: sum(max(SIZE[k], 2) for k in f['slots'].values())),
    ('slots4: 4 per by-address slot read in place', lambda f: 4 * len(f['slots'])),
    ('regs: 4 per saved register (D2-D7, A2-A4)', lambda f: 4 * len(f['saved'])),
    ('aregs: 4 per saved address register (A2-A4)',
     lambda f: 4 * sum(1 for x in f['saved'] if x[0] == 'a')),
    ('stepped: 4 per address register stepped in place', lambda f: 4 * len(f['stepped'])),
)


def image_fit(fs):
    """Over hidden_features FS, per hypothesis: the routines where it
    predicts something (P > 0); those whose frame has no room for it (P
    more than the unaddressed bytes less the reserved slot: a
    refutation, the unaddressed count being an upper bound); and, among
    the untouched frames (link > 0) of map-named procedures, where the
    whole frame is objects the code keeps out of memory, those where
    round4(P) == link."""
    rows = []
    unt = [f for f in fs if f['untouched'] and not f['reserved'] and not f['func'] and f['L']]
    for name, p in HYPOTHESES:
        some = sum(1 for f in fs if p(f) > 0)
        bad = sum(1 for f in fs if p(f) > f['free'] - f['reserved'])
        exact = sum(1 for f in unt if round4(p(f)) == f['L'])
        rows.append((name, some, bad, exact, len(unt)))
    return rows


# The ../source Pascal (bead domain-pascal-s9g0): routines whose header
# names an image address, and the bytes of the stand-in locals its
# translators declared for frame bytes nothing explains (`unused_8:
# integer32;  { (-0x8,A6): stand-in ... }').

_STANDIN = re.compile(r'^\s+([a-z_0-9]+)\s*:\s*([^;:=]*?)\s*(?::=[^;]*)?;\s*\{([^}]*)',
                      re.M | re.I)
_TSIZE = {'integer32': 4, 'linteger': 4, 'integer': 2, 'integer16': 2, 'pinteger': 2,
          'char': 1, 'boolean': 1}


def type_bytes(t):
    """Bytes of a stand-in's type text: a scalar of _TSIZE or `array[1..n]
    of' one (bytes word-aligned in the frame as a whole); else None."""
    t = re.sub(r'\s+', ' ', t.strip().lower())
    m = re.match(r'array ?\[ ?1 ?\.\. ?(\d+) ?\] ?of (\w+)$', t)
    if m and m.group(2) in _TSIZE:
        return int(m.group(1)) * _TSIZE[m.group(2)]
    return _TSIZE.get(t)


def standin_bytes(decls):
    """Sum of the stand-in locals' sizes in declaration text DECLS (each
    rounded to a word, as frame objects are), and the names whose type is
    not understood."""
    n, unknown = 0, []
    for m in _STANDIN.finditer(decls):
        if not re.search(r'stand-?\s*in', m.group(3), re.I):
            continue
        b = type_bytes(m.group(2))
        if b is None:
            unknown.append(m.group(1))
        else:
            n += b + (b & 1)
    return n, unknown


def source_routines(tree):
    """[(path, name, addr, stand-in bytes, `with' count)] for the
    top-level routines of the `.pas' fragments under TREE with a header
    address (tools/srcsurvey.py's parser)."""
    import srcsurvey
    out = []
    for dp, _ds, fl in os.walk(tree):
        for fn in sorted(fl):
            if not fn.endswith('.pas') or fn.endswith('.ins.pas'):
                continue
            p = os.path.join(dp, fn)
            with open(p, errors='replace') as fh:
                text = fh.read()
            try:
                routines = srcsurvey.parse_routines(text)
            except (ValueError, IndexError):
                continue
            for r in routines:
                if r.addr is None or not r.has_body:
                    continue
                dend = min([n.start for n in r.nested] + [r.body])
                s, _unk = standin_bytes(text[r.head_end:dend])
                body = srcsurvey.TOKEN_COMMENT_RE.sub(' ', text[r.body:r.end]).lower()
                withs = len(re.findall(r'\bwith\b', body))
                out.append((os.path.relpath(p, tree), r.name, r.addr, s, withs))
    return sorted(out, key=lambda x: x[2])


def source_fit(rows, feats):
    """ROWS from source_routines, FEATS {addr: hidden_features}: per
    hypothesis, how often round4 of its prediction equals the stand-in
    bytes, over the routines with stand-ins and over those without."""
    have = [(x, feats[x[2]]) for x in rows if x[2] in feats]
    out = []
    for name, p in HYPOTHESES:
        c = Counter()
        for x, f in have:
            c[('standin' if x[3] else 'none', round4(p(f)) == round4(x[3]))] += 1
        out.append((name, c))
    return len(have), out


# Two `tools/srcsurvey.py run' work directories (the same ../source tree,
# two dpc builds): each compiled routine's `link' size in dpc's unit
# against the image's.

_REPORT_ROW = re.compile(r'\s+(OK|MISMATCH|CODE-OK)\s+(\S+)\s+0x([0-9A-F]+)')
_DPC_LINK = re.compile(r'link\.w\s+%fp,#(-?(?:0x[0-9a-f]+|\d+))')


def report_routines(text):
    """{routine: address} of the compiled routines of a srcsurvey report."""
    return {m.group(2): int(m.group(3), 16)
            for m in map(_REPORT_ROW.match, text.splitlines()) if m}


def unit_link(text, name):
    """The `link' displacement of routine NAME in a dpc unit's TEXT, or
    None (no such label, or no `link')."""
    i = text.find('\n%s:' % name.upper())
    if i < 0:
        return None
    m = _DPC_LINK.search(text, i)
    j = text.find('\trts', i)
    if not m or (0 <= j < m.start()):
        return None
    v = m.group(1)
    return -int(v[3:], 16) if v.startswith('-0x') else int(v, 0)


def work_link(work, name):
    p = os.path.join(work, 'conform', '_unit_%s.s' % name.replace('$', '_'))
    if not os.path.exists(p):
        return None
    with open(p) as fh:
        return unit_link(fh.read(), name)


def image_link(addr):
    import loopalign
    if loopalign.word(addr) != 0x4E56:
        return None
    d = loopalign.word(addr + 2)
    return d - 0x10000 if d & 0x8000 else d


def compare_links(names, before, after, image):
    """NAMES {routine: address}; BEFORE/AFTER(name) and IMAGE(addr) give
    `link' displacements: (fixed, broke, closer, farther, matches before,
    matches after), the first four lists of (name, image, before,
    after) for the routines whose link changed."""
    fixed, broke, closer, farther = [], [], [], []
    n0 = n1 = 0
    for name, addr in sorted(names.items(), key=lambda kv: kv[1]):
        il, a, b = image(addr), before(name), after(name)
        n0 += a == il
        n1 += b == il
        if a == b or il is None or a is None or b is None:
            continue
        row = (name, il, a, b)
        if b == il:
            fixed.append(row)
        elif a == il:
            broke.append(row)
        elif abs(b - il) < abs(a - il):
            closer.append(row)
        else:
            farther.append(row)
    return fixed, broke, closer, farther, n0, n1


def main(argv):
    if not argv or argv[0] not in ('align', 'byte-pairs', 'order', 'copies', 'hidden',
                                   'srclinks'):
        print(__doc__)
        return 2
    if argv[0] == 'srclinks':
        if len(argv) != 4:
            print(__doc__)
            return 2
        with open(argv[1]) as fh:
            names = report_routines(fh.read())
        res = compare_links(names, lambda n: work_link(argv[2], n),
                            lambda n: work_link(argv[3], n), image_link)
        for title, rows in zip(('fixed', 'broke', 'closer', 'farther'), res[:4]):
            print('%s: %d' % (title, len(rows)))
            for name, il, a, b in rows:
                print('  %-34s image #%d: %d -> %d' % (name, il, a, b))
        print('link as the image: %d before, %d after, of %d compiled' % (res[4], res[5], len(names)))
        return 0
    import nestsurvey
    rs = nestsurvey.load()
    if argv[0] == 'hidden':
        by_start, _ = nestsurvey.analyse(rs)
        fns = nestsurvey.functions(rs, by_start)
        nss = {r.r0 for r in nestsurvey.nested(rs)}
        feats = {r.r0: hidden_features(r, fns, nss) for r in rs}
        fs = list(feats.values())
        print('%d routines with a link; %d untouched frames (link > 0) of map-named procedures' % (
            len(fs), sum(1 for f in fs if f['untouched'] and not f['reserved'] and not f['func'] and f['L'])))
        print('image: hypothesis | P > 0 | no room (refuted) | untouched map-named '
              'procedures, round4(P) == link')
        for name, some, bad, exact, n in image_fit(fs):
            print('  %-52s %5d %5d   %4d of %d' % (name, some, bad, exact, n))
        tight = sum(1 for f in fs if not f['untouched'] and f['free'] - f['reserved'] < 4)
        print('%d frames with fewer than 4 unaddressed bytes beyond the reserved slot' % tight)
        if len(argv) > 1:
            rows = source_routines(argv[1])
            n, out = source_fit(rows, feats)
            ns = sum(1 for x in rows if x[2] in feats and x[3])
            print('source: %d routines with an address and a link, %d with stand-ins' % (n, ns))
            print('  hypothesis | fits with stand-ins | fits without')
            for name, c in out:
                print('  %-52s %4d of %-4d %4d of %d' % (
                    name, c[('standin', True)], c[('standin', True)] + c[('standin', False)],
                    c[('none', True)], c[('none', True)] + c[('none', False)]))
        return 0
    if argv[0] == 'align':
        c = align_counts([(r.insns, r.link) for r in rs])
        for k in 'bwl':
            print('%s objects (offset mod 4):' % {'b': 'byte', 'w': 'word', 'l': 'longword'}[k])
            for cls in ('-', 'lowpea'):
                row = {m: c[(k, cls, m)] for m in range(4)}
                print('  %-7s %s' % (cls, '  '.join('%d:%d' % kv for kv in row.items())))
    elif argv[0] == 'copies':
        c, ex = copy_forms(rs)
        print('slot copies by bytes and form (count, first site):')
        for k in sorted(c):
            print('  %2d %-12s %4d  0x%06X' % (k[0], k[1], c[k], ex[k]))
        fit, pos = Counter(), Counter()
        for r in rs:
            for slot, a, seq, ok in entry_copies(r.insns):
                fit[ok] += 1
                if not ok:
                    print('  not R4(b): 0x%06X %s slot %d A%d' % (r.r0, r.name or '-', slot, a))
                n = sum(SIZE[x[0]] for x in seq)
                if n == 8 and r.name:
                    lo = min(x[2] for x in seq)
                    above = [o for o in offsets(r.insns, r.link) if o >= lo + 8]
                    pos[('top' if lo == -8 else 'deep',
                         'objects above' if above else 'nothing above')] += 1
        print('entry copies, register by 12.4 R4(b): %d fit, %d not' % (fit[True], fit[False]))
        print('8-byte entry copies of map-named routines:')
        for k, v in sorted(pos.items()):
            print('  %-4s %-14s %d' % (k[0], k[1], v))
    elif argv[0] == 'order':
        c = Counter()
        for r in rs:
            k = size_order(r.insns, r.link)
            if k:
                c[(k, 'map-named' if r.name else 'unnamed')] += 1
        for k, v in sorted(c.items()):
            print('%-17s %-9s %d' % (k[0], k[1], v))
    else:
        n = 0
        for r in rs:
            ds = byte_pairs(r.insns, r.link)
            if ds:
                n += 1
                bs = sorted(set(ds) | {d - 2 for d in ds}, reverse=True)
                print('0x%06X %-28s link #%d  bytes at %s' % (
                    r.r0, r.name or '-', r.link, ', '.join(str(d) for d in bs)))
        print('%d routines' % n)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
