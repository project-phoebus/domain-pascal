#!/usr/bin/env python3
"""arithsurvey.py - multiplication, division and mod in the compiled code of
the SR10.2 image: M$ARITH calls and inline mul<s>.w / div<s>.w.

Evidence tool for docs/codegen-patterns.md section 8 (beads
domain-pascal-5aw.5, -1x1).  Compiled routines are split and followed as
tools/casesurvey.py does (map symbols and `link.w A6' words, hand assembly
excluded, only the instructions reachable from the routine's start).

For every `jsr' to an M$ARITH entry (map `I E0ABD4 M$ARITH size = 1DC'):

    the entry; the pad (`subq.l #2,SP' since the previous call); the two
    pushes before the `jsr' (argument 2 = b first, then argument 1 = a);
    the pop after it (`addq.w #8,SP', or none); the first instruction
    after the pop that reads D0 (how the result is used); whether the
    instructions between the pad and the pushes compute something (the
    pad is "scheduled" into the computation of an operand, R9); and the
    registers D1/A1 live across the call (tools/regalloc.py's model).

For every inline `muls.w'/`mulu.w'/`divs.w'/`divu.w': the source kind
(immediate, register, memory) and how the result is used next (an index
`(d,An,Dn.l)' / `(d,An,Dn.w)', or the next instruction reading it).

For every product of a run-time value by a constant (bead
domain-pascal-r3yy; `scale_sites'): the factor, how it is formed (shifts
and adds; `mul<s>.w #c'; the constant first, `moveq #c,Dn; mul<s>.w
<mem>,Dn' or `mul<s>.w Dm,Dn'), where the value comes from (memory by
kind, a register held from before the block, a by-value parameter's
slot), and what reads the product first (an index `.w'/`.l' or another
instruction).  The table of codegen-patterns 5.2 and 8.

usage:
    tools/arithsurvey.py calls       # one line per M$ARITH call site
    tools/arithsurvey.py inline      # one line per inline mul/div
    tools/arithsurvey.py summary     # the counts quoted in codegen-patterns 8
    tools/arithsurvey.py scale       # products by a constant, by factor and form
    tools/arithsurvey.py scales      # one line per product by a constant
    tools/arithsurvey.py narrow      # size changes, codegen-patterns 8
                                     # ("Narrowing and widening")
    tools/arithsurvey.py narrows KIND  # the sites of one kind of `narrow'

For `narrow' (bead domain-pascal-8td), per routine and per basic block
(cut at branch targets, branches and calls):

    lowword   a frame slot (d,A6) accessed as a longword at d and as a
              word at d+2, or as a word at d and a byte at d+1: the low
              part of a variable read or written in place (or a record
              of two halves: re-check sites);
    narrowed  a word (byte) store or push `move.<s> Dn,<mem>' whose
              register was last written by a longword (word) instruction
              in the block: a value formed wider and its low part used,
              by the writer (`lsr.l', a load `move.l <mem>,Dn', ...);
    widened   a longword push `move.l Dn,-(SP)' of a register formed by
              `ext.l Dn' (sign) or `clr.l Dn; ...; move.w x,Dn' (zero),
              and a word push of `clr.w Dn; ...; move.b x,Dn';
    heldop    a word operation `<op>.w Dm,Dn' whose source register was
              last written by a longword instruction (a longword's low
              word used in word arithmetic).

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import a5survey  # noqa: E402
import loopalign  # noqa: E402
import sau2map  # noqa: E402

# The entries of M$ARITH, from the map (M$OIS$LLL and M$OIU$LLL share
# 0xE0AD7C).
ENTRIES = {
    0xE0ABD4: 'M$MIS$LLL', 0xE0AC02: 'M$MIU$LLW', 0xE0AC1C: 'M$MIU$LLL',
    0xE0AC44: 'M$MIS$LLW', 0xE0AC68: 'M$DIU$LLW', 0xE0AC84: 'M$DIS$LLW',
    0xE0ACC0: 'M$OIU$WLW', 0xE0ACD8: 'M$DIS$LLL', 0xE0AD0C: 'M$DIU$LLL',
    0xE0AD54: 'M$OIS$WLW', 0xE0AD68: 'M$OIS$WWL', 0xE0AD7C: 'M$OIS$LLL',
}

_JSR = re.compile(r'^jsr 0x([0-9a-f]+)$')
_CALL = re.compile(r'^(jsr|bsr)')
_PUSH = re.compile(r'^(?:move[bwl] .*,%sp@-|clr[bwl] %sp@-|pea .*)$')
_MUL = re.compile(r'^(mul|div)([su])w (.*),(%d\d)$')


def routines():
    """[(r0, r1, name)] of the compiled routines."""
    m = sau2map.load()
    out = []
    for seg in m.segments:
        if (seg.kind != 'I' or not (loopalign.LO <= seg.addr < loopalign.HI)
                or not seg.size or loopalign.is_asm(seg)):
            continue
        st = loopalign.routine_starts(m, seg)
        end = min(seg.end, loopalign.HI)
        for r0, r1 in zip(st, st[1:] + [end]):
            if r1 > r0:
                names = m.names_at(r0)
                out.append((r0, r1, names[0] if names else 'module-local'))
    return out


def call_site(ins, i):
    """The M$ARITH call at ins[i] as a dict, or None.  `ins' is [(addr,
    len, text)] in address order."""
    j = _JSR.match(ins[i][2])
    if not j or int(j.group(1), 16) not in ENTRIES:
        return None
    pushes, k = [], i - 1
    while k >= 0 and len(pushes) < 2 and _PUSH.match(ins[k][2]):
        pushes.append(ins[k][2])
        k -= 1
    # the pad: a `subq.l #2,SP' after the previous call (or its pop)
    pad, between = None, []
    q = k
    while q >= 0 and not _CALL.match(ins[q][2]) and i - q <= 16:
        if ins[q][2] == 'subql #2,%sp':
            pad = ins[q][0]
            break
        between.append(ins[q][2])
        q -= 1
    pop = ins[i + 1][2] if i + 1 < len(ins) else ''
    k = i + 2 if pop.startswith('addqw #8,%sp') else i + 1
    if not pop.startswith('addqw #8,%sp'):
        pop = None
    use = None
    while k < len(ins):
        if '%d0' in ins[k][2]:
            use = ins[k][2]
            break
        k += 1
    return {
        'addr': ins[i][0], 'entry': ENTRIES[int(j.group(1), 16)],
        'a': pushes[0] if pushes else None, 'b': pushes[1] if len(pushes) > 1 else None,
        'pad': pad, 'pad_inside': pad is not None and bool(between),
        'pop': pop, 'use': use,
    }


def inline_site(ins, i):
    """The inline multiply or divide at ins[i] as a dict, or None."""
    m = _MUL.match(ins[i][2])
    if not m:
        return None
    src, dst = m.group(3), m.group(4)
    kind = 'imm' if src.startswith('#') else 'reg' if re.fullmatch(r'%d\d', src) else 'mem'
    use = None
    for _, _, t in ins[i + 1:i + 8]:
        if dst in t:
            use = t
            break
    if use is None:
        cls = '?'
    elif re.search(dst + r':l\)', use):
        cls = 'index.l'
    elif re.search(dst + r':w\)', use):
        cls = 'index.w'
    else:
        cls = use.split()[0]
    return {'addr': ins[i][0], 'op': m.group(1) + m.group(2) + 'w', 'src': kind,
            'text': ins[i][2], 'use': use, 'cls': cls}


def live_across(r0, r1, addrs):
    """{call addr: sorted D1/A1 live after it} from tools/regalloc.py's
    model, or None when the model skips the routine."""
    import regalloc
    if loopalign.word(r0) != 0x4E56:
        return None
    try:
        rt = regalloc.Routine(r0, r1)
    except Exception:       # regalloc.Unknown and model limits
        return None
    out = {}
    for a in addrs:
        i = rt.at.get(a)
        if i is not None and i in rt.lout:
            out[a] = sorted(r for r in rt.lout[i] if r in ('d1', 'a1'))
    return out


def scan(with_live=False):
    calls, inline = [], []
    for r0, r1, name in routines():
        ins = a5survey.reached(r0, r1)
        here = []
        for i in range(len(ins)):
            c = call_site(ins, i)
            if c:
                c['routine'] = (r0, name)
                here.append(c)
            s = inline_site(ins, i)
            if s:
                s['routine'] = (r0, name)
                inline.append(s)
        if here and with_live:
            live = live_across(r0, r1, [c['addr'] for c in here])
            for c in here:
                c['live'] = None if live is None else live.get(c['addr'])
        calls.extend(here)
    return calls, inline


def push_kind(t):
    if t is None:
        return '-'
    if t.startswith('clr'):
        return t.split()[0] + ' 0'
    if re.match(r'move[bwl] #', t):
        return t.split()[0] + ' #'
    if re.match(r'move[bwl] %d\d,', t):
        return t.split()[0] + ' Dn'
    return t.split()[0] + ' mem'


def summary(calls, inline):
    print('M$ARITH calls: %d' % len(calls))
    for e, n in sorted(Counter(c['entry'] for c in calls).items()):
        print('  %-10s %d' % (e, n))
    word_b = [c for c in calls if c['entry'].endswith('W') or c['entry'].endswith('WWL')]
    print('pad: %d of %d calls with a word argument; %d of %d without' % (
        sum(1 for c in word_b if c['pad']), len(word_b),
        sum(1 for c in calls if c not in word_b and c['pad']), len(calls) - len(word_b)))
    print('pad not directly before the pushes: %d' % sum(1 for c in calls if c['pad_inside']))
    print('argument b pushed as: %s' % dict(Counter(push_kind(c['b']) for c in calls)))
    print('argument a pushed as: %s' % dict(Counter(push_kind(c['a']) for c in calls)))
    print('popped with addq.w #8: %d; not popped: %d (%s)' % (
        sum(1 for c in calls if c['pop']), sum(1 for c in calls if not c['pop']),
        ', '.join('0x%X' % c['addr'] for c in calls if not c['pop'])))
    live = [c for c in calls if c.get('live') is not None]
    print('D1/A1 live across: %d of %d modelled calls (%d register/call pairs)' % (
        sum(1 for c in live if c['live']), len(live), sum(len(c['live']) for c in live)))
    print('inline: %s' % dict(Counter((s['op'], s['src']) for s in inline)))
    print('inline uses: %s' % dict(Counter((s['op'], s['cls']) for s in inline)))



# ---------------------------------------------------------------------------
# `scale': products by a constant, index scalings among them (bead
# domain-pascal-r3yy).

def operands(t):
    """(mnemonic, [operands]) of an objdump instruction text, the operands
    split at the commas outside parentheses."""
    if ' ' not in t:
        return t, []
    op, rest = t.split(' ', 1)
    out, depth, cur = [], 0, ''
    for ch in rest:
        if ch == ',' and depth == 0:
            out.append(cur)
            cur = ''
            continue
        depth += ch == '('
        depth -= ch == ')'
        cur += ch
    out.append(cur)
    return op, out


def mem_kind(ea):
    """Where a memory operand is: `param' (a by-value slot, (d,A6) d > 0),
    `local' ((d,A6) d < 0), `unit' ((d,A5)), `abs', `elem' (indexed),
    `ptr' (through A0-A4: a by-address parameter or a pointer)."""
    if ea.startswith('0x') or re.fullmatch(r'\d+', ea):
        return 'abs'
    if re.search(r',%[ad]\d:[wl]\)', ea):
        return 'elem'
    m = re.match(r'^%fp@\((-?\d+)\)$', ea)
    if m:
        return 'param' if int(m.group(1)) > 0 else 'local'
    if ea.startswith('%a5@'):
        return 'unit'
    if re.match(r'^%a\d@', ea):
        return 'ptr'
    return 'other'


_ONE_WRITES = re.compile(r'^(clr|neg|not|ext|swap|s(?:t|f|hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|'
                         r'gt|le))[bwl]?$')
_IDXOP = re.compile(r'%(a\d|pc)@\((-?[0-9a-fx]+),%(d\d):([wl])\)')
def _pow2(v):
    return v > 0 and v & (v - 1) == 0


def scale_sites(ins, labels=frozenset()):
    """[dict] of the products of a run-time value by a constant formed in
    a block of `ins' (cut at branch targets and after branches, calls and
    jumps): each product's first reading other than an instruction that
    goes on forming it (an indexed operand `(d,An,Dx.<s>)', a store, an
    add of another value, a compare...).  Only products whose factor is
    not 1 are kept; a factor of 2^s is kept for indexed operands only.

    The registers are simulated: a constant (`moveq', `move #'), a linear
    function `coef * X + off' of a value X first read in the block (a load
    from memory, or a register's value at the block's start, `held'), or
    unknown.  Each dict: addr (the reading), coef, form (`mulu #imm',
    `muls #imm', `moveq*mem' for `moveq #c,Dn; mul<s>.w <mem>,Dn',
    `movew#*mem' for `move.w #c,Dn; ...', `const*reg', `reg*const',
    `shifts' when no multiply instruction is used), mul (`mulu', `muls' or
    None), x (the kind of X: mem_kind, `held', `areg', `call', `computed'
    for a mask), zext / ext (X zero- or sign-extended before), idx (`w',
    `l' or None: the reading is an indexed operand with the product as
    its index), use (the reading's text), chain (the texts that formed
    the product)."""
    regs, xs, hist = {}, [], {}
    out, done = [], set()

    def fresh(kind, k):
        xs.append({'kind': kind, 'at': k, 'zext': False, 'ext': False})
        return ('x', len(xs) - 1, 1, 0)

    def reset(k):
        for r in range(8):
            regs['d%d' % r] = fresh('held', k)
            hist['d%d' % r] = ()

    def record(k, r, idx):
        v = regs.get(r)
        if not v or v[0] != 'x' or v[2] in (0, 1):
            return
        if idx is None and (_pow2(abs(v[2])) or v[2] == -1):
            return
        if (r, v, hist[r]) in done:
            return
        done.add((r, v, hist[r]))
        chain = [ins[j][2] for j in sorted(set(hist[r]))]
        muls = [c for c in chain if c.startswith(('mulu', 'muls'))]
        if muls:
            mop, mops = operands(muls[-1])
            src = mops[0]
            before = [c for c in chain if operands(c)[1][-1:] == mops[-1:]
                      and chain.index(c) < chain.index(muls[-1])]
            if src.startswith('#'):
                form = mop[:4] + ' #imm'
            elif re.fullmatch(r'%d\d', src):
                form = 'const*reg' if before and re.match(r'^(moveq|movew) #', before[-1]) \
                    else 'reg*const'
            else:
                cl = before[-1] if before else ''
                form = ('moveq*mem' if cl.startswith('moveq') else
                        'movew#*mem' if cl.startswith('movew #') else 'movel#*mem'
                        if cl.startswith('movel #') else '?*mem')
            mul = mop[:4]
        else:
            form, mul = 'shifts', None
        x = xs[v[1]]
        out.append({'addr': ins[k][0], 'coef': v[2], 'off': v[3], 'form': form, 'mul': mul,
                    'x': x['kind'], 'zext': x['zext'], 'ext': x['ext'], 'idx': idx,
                    'use': ins[k][2], 'chain': chain})

    reset(0)
    seen = set()
    for k, (a, _, t) in enumerate(ins):
        if a in labels:
            reset(k)
        op, ops = operands(t)
        dst = ops[-1] if ops else ''
        dreg = dst if re.fullmatch(r'%d\d', dst) else None
        dreg = dreg[1:] if dreg else None
        base = re.sub(r'[bwl]$', '', op) if op not in ('moveq', 'mulsw', 'muluw') else op
        sz = op[-1] if op[-1] in 'bwl' and op not in ('moveq',) else 'l'
        # readings by indexed operands
        for m in _IDXOP.finditer(t):
            r = m.group(3)
            key = (regs.get(r), r)
            if key[0] and key[0][0] == 'x' and (k, r) not in seen:
                seen.add((k, r))
                record(k, r, m.group(4))
        new = None
        cont = set()        # registers whose value this instruction goes on forming
        src = ops[0] if len(ops) == 2 else None
        sv = regs.get(src[1:]) if src and re.fullmatch(r'%d\d', src) else None
        dv = regs.get(dreg) if dreg else None
        h = hist.get(dreg, ()) if dreg else ()
        if dreg and len(ops) == 2:
            imm = int(src[1:], 0) if re.fullmatch(r'#-?\d+', src) else None
            if op == 'moveq' or (base == 'move' and imm is not None):
                new, h = ('c', imm), (k,)
            elif base == 'move' and sv is not None and sz in 'wl':
                new, h = sv, hist[src[1:]] + (k,)
                cont.add(src[1:])
            elif base == 'move' and re.fullmatch(r'%a\d', src):
                new, h = fresh('areg', k), ()
            elif base == 'move' and src:
                new, h = fresh(mem_kind(src) if src != '%sp@+' else 'pop', k), (k,)
                xs[new[1]]['size'] = sz
                if sz == 'w' and dv == ('c', 0):
                    xs[new[1]]['zext'] = True
                    h = hist[dreg] + (k,)
            elif base in ('add', 'sub', 'addq', 'subq', 'addi', 'subi') and dv:
                sgn = -1 if base.startswith('sub') else 1
                if imm is not None and dv[0] == 'x':
                    new, h = ('x', dv[1], dv[2], dv[3] + sgn * imm), h + (k,)
                elif imm is not None and dv[0] == 'c':
                    new = ('c', dv[1] + sgn * imm)
                elif sv and sv[0] == 'x' and dv[0] == 'x' and sv[1] == dv[1]:
                    new = ('x', dv[1], dv[2] + sgn * sv[2], dv[3] + sgn * sv[3])
                    h = h + hist[src[1:]] + (k,)
                    cont.add(src[1:])
                elif sv and sv[0] == 'c' and dv[0] == 'x':
                    new, h = ('x', dv[1], dv[2], dv[3] + sgn * sv[1]), h + (k,)
                elif sv and sv[0] == 'c' and dv[0] == 'c':
                    new = ('c', dv[1] + sgn * sv[1])
            elif base in ('and', 'andi') and sz in 'wl':
                # a mask (a hash's bucket: HINT_$GET_HINTS 0xE4998E..0xE4999A
                # `moveq #63,D3; moveq #84,D5; and.w D0,D3; ...; muls.w D3,D5'):
                # a new value X, computed
                new, h = fresh('computed', k), h + (k,)
            elif base in ('lsl', 'asl') and dv and imm is not None:
                new = (('x', dv[1], dv[2] << imm, dv[3] << imm) if dv[0] == 'x'
                       else ('c', dv[1] << imm))
                h = h + (k,)
            elif op in ('mulsw', 'muluw') and dv:
                if imm is not None and dv[0] == 'x':
                    new, h = ('x', dv[1], dv[2] * imm, dv[3] * imm), h + (k,)
                elif sv and sv[0] == 'c' and dv[0] == 'x':
                    new, h = ('x', dv[1], dv[2] * sv[1], dv[3] * sv[1]), h + hist[src[1:]] + (k,)
                elif sv and sv[0] == 'x' and dv[0] == 'c':
                    new = ('x', sv[1], sv[2] * dv[1], sv[3] * dv[1])
                    h = h + hist[src[1:]] + (k,)
                    cont.add(src[1:])
                elif dv[0] == 'c' and imm is None and not re.fullmatch(r'%[ad]\d', src):
                    x = fresh(mem_kind(src), k)
                    new, h = ('x', x[1], dv[1], 0), h + (k,)
        elif dreg and len(ops) == 1:
            if base == 'clr':
                new, h = ('c', 0), (k,)
            elif base == 'neg' and dv:
                new = ('x', dv[1], -dv[2], -dv[3]) if dv[0] == 'x' else ('c', -dv[1])
                h = h + (k,)
            elif base == 'ext' and dv:
                new, h = dv, h + (k,)
                if dv[0] == 'x':
                    xs[dv[1]]['ext'] = True
        if dreg and new and new[0] == 'x' and dv and dv[0] == 'x' and dv[1] == new[1]:
            cont.add(dreg)
        # other readings of a product
        rd = set(re.findall(r'%(d\d)', t)) - {m.group(3) for m in _IDXOP.finditer(t)}
        if dreg and (base in ('move', 'moveq', 'clr') or op == 'exg'
                     or (len(ops) == 1 and _ONE_WRITES.match(op) and base != 'neg'
                         and not base.startswith(('ext', 'not', 'swap')))):
            rd.discard(dreg)        # written, not read
        if op.startswith('movem') and '%d' in dst:
            rd = set()
        for r in sorted(rd - cont):
            if (k, r) not in seen:
                seen.add((k, r))
                record(k, r, None)
        # writes
        if op.startswith(('jsr', 'bsr')):
            for r in ('d0', 'd1'):
                regs[r], hist[r] = fresh('call', k), ()
        elif dreg and (len(ops) == 2 or _ONE_WRITES.match(op)):
            if op == 'exg':
                a2 = ops[0][1:]
                regs[a2], regs[dreg] = regs.get(dreg), regs.get(a2)
                hist[a2], hist[dreg] = hist.get(dreg, ()), hist.get(a2, ())
            elif op.startswith(('cmp', 'tst', 'btst')):
                pass
            else:
                regs[dreg], hist[dreg] = new, (h if new else ())
        elif op.startswith('movem') and dst.startswith('%d'):
            for r in re.findall(r'%(d\d)', dst):
                regs[r], hist[r] = fresh('held', k), ()
        elif op.startswith('db') and len(ops) == 2:
            r = ops[0][1:]
            regs[r], hist[r] = None, ()
        if _BLOCK_END.match(t):
            reset(k + 1)
    return out


_BLOCK_END = re.compile(r'^(b[a-z]{2}[sw]?|bra[sw]?|jmp|rts|db[a-z]+|bsr[sw]?|jsr) ')


def odd_bits(c):
    c = abs(c)
    while c and not c & 1:
        c >>= 1
    return bin(c).count('1')


def scale_summary(sites):
    """The table of codegen-patterns 5.2/8: per factor, the forms."""
    by = {}
    for s in sites:
        by.setdefault(abs(s['coef']), []).append(s)
    print('products by a constant (factor: odd part bits; form x count [idx w/l] e.g.)')
    for c in sorted(by):
        ss = by[c]
        forms = Counter((s['form'], s['idx'] or '-') for s in ss)
        print('%6d %d  %s' % (c, odd_bits(c), '; '.join(
            '%s %s x%d e.g. 0x%X' % (f, i, n, next(s['addr'] for s in ss
                                                  if (s['form'], s['idx'] or '-') == (f, i)))
            for (f, i), n in sorted(forms.items()))))


def scale_scan():
    import boolsurvey
    out = []
    for r0, r1, name in routines():
        ins = a5survey.reached(r0, r1)
        for s in scale_sites(ins, boolsurvey.branch_targets(ins)):
            s['routine'] = (r0, name)
            out.append(s)
    return out


# ---------------------------------------------------------------------------
# `narrow': size changes of integer values (bead domain-pascal-8td).

_FPSLOT = re.compile(r'%fp@\((-?\d+)\)')
_SIZED = re.compile(r'^[a-z]+([bwl])$')


def op_size(op):
    """`b', `w' or `l' of a sized objdump mnemonic (`movew'), else None
    (`moveq', `lea', `pea', `movem' have none here)."""
    if op in ('moveq',) or op.startswith(('lea', 'pea', 'movem', 'jsr', 'bsr', 'b')):
        return None
    m = _SIZED.match(op)
    return m.group(1) if m else None


def lowword_sites(ins):
    """[(kind, d, addr)] of the frame slots of one routine accessed at two
    sizes: kind `l@d,w@d+2' (a longword at d, a word at d+2) or `w@d,b@d+1';
    addr is the narrower access's first instruction."""
    acc = {}
    for a, _, t in ins:
        s = op_size(t.split()[0])
        if not s:
            continue
        for m in _FPSLOT.finditer(t):
            acc.setdefault(int(m.group(1)), []).append((s, a))
    out = []
    for d in sorted(acc):
        if d >= 0:
            continue
        sizes = {s for s, _ in acc[d]}
        for wide, narrow, k in (('l', 'w', 2), ('w', 'b', 1)):
            if wide in sizes:
                hits = [a for s, a in acc.get(d + k, ()) if s == narrow]
                if hits:
                    out.append(('%s@d,%s@d+%d' % (wide, narrow, k), d, hits[0]))
    return out


_BLOCK_CUT = re.compile(r'^(b[a-z]{2}[sw]?|bra[sw]?|jmp|rts|db[a-z]+) ')


def last_writer(ins, i, r, labels=frozenset()):
    """The index of the instruction before ins[i], in its block, that last
    wrote data register `r' (`d0'..`d7'), or None.  A call ends the search
    with None (it clobbers D0/D1), so does a branch or a branch target."""
    k = i - 1
    while k >= 0:
        a, _, t = ins[k]
        if _BLOCK_CUT.match(t) or t.startswith(('jsr', 'bsr')):
            return None
        op, ops = operands(t)
        if ops and ops[-1] == '%' + r and not op.startswith(('cmp', 'tst', 'btst')):
            return k
        if op == 'exg' and '%' + r in ops:
            return k
        if a in labels:
            return None
        k -= 1
    return None


def writer_class(t):
    """How an instruction writing a data register forms it: `load.<s>'
    (from memory), `copy.<s>' (from a register), `moveq', or the
    mnemonic without its size and `.<s>' (`lsr.l', `add.w')."""
    op, ops = operands(t)
    if op == 'moveq':
        return 'moveq'
    s = op_size(op) or '?'
    if op.startswith('move') and len(ops) == 2:
        if re.fullmatch(r'%[ad]\d', ops[0]):
            return 'copy.' + s
        if not ops[0].startswith('#'):
            return 'load.' + s
    return re.sub(r'[bwl]$', '', op) + '.' + s


def narrowed_sites(ins, labels=frozenset()):
    """[(size, writer class, addr, writer text, text)] of the word and
    byte stores and pushes `move.<s> Dn,<mem>' whose register was formed
    by a wider instruction in the block."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = re.match(r'^move([wb]) %(d\d),(.*)$', t)
        if not m or re.fullmatch(r'%[ad]\d', m.group(3)):
            continue
        k = last_writer(ins, i, m.group(2), labels)
        if k is None:
            continue
        cls = writer_class(ins[k][2])
        wide = {'w': ('l',), 'b': ('w', 'l')}[m.group(1)]
        if cls.endswith(tuple('.' + w for w in wide)) and not cls.startswith(('ext', 'clr')):
            out.append((m.group(1), cls, a, ins[k][2], t))
    return out


def widened_sites(ins, labels=frozenset()):
    """[(kind, addr)] of the pushes of a widened register: `ext.l',
    `clr.l;move.w' for `move.l Dn,-(SP)', `clr.w;move.b' for `move.w
    Dn,-(SP)'."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = re.match(r'^move([lw]) %(d\d),%sp@-$', t)
        if not m:
            continue
        r = m.group(2)
        k = last_writer(ins, i, r, labels)
        if k is None:
            continue
        w = ins[k][2]
        if m.group(1) == 'l' and w == 'extl %' + r:
            out.append(('ext.l', a))
        elif re.match(r'^move([wb]) ', w) and (m.group(1), w[4]) in (('l', 'w'), ('w', 'b')):
            k2 = last_writer(ins, k, r, labels)
            clr = 'clr' + {'l': 'l', 'w': 'w'}[m.group(1)] + ' %' + r
            if k2 is not None and ins[k2][2] == clr:
                out.append(('clr.l;move.w' if m.group(1) == 'l' else 'clr.w;move.b', a))
    return out


def heldop_sites(ins, labels=frozenset()):
    """[(op, writer class, addr, writer text, text)] of the word
    operations `<op>.w Dm,Dn' whose source register was last written by a
    longword instruction other than a widening (`ext.l', `clr.l')."""
    out = []
    for i, (a, _, t) in enumerate(ins):
        m = re.match(r'^(add|sub|and|or|eor|cmp)w %(d\d),%d\d$', t)
        if not m:
            continue
        k = last_writer(ins, i, m.group(2), labels)
        if k is None:
            continue
        cls = writer_class(ins[k][2])
        if cls.endswith('.l') and not cls.startswith(('ext', 'clr')):
            out.append((m.group(1), cls, a, ins[k][2], t))
    return out


def narrow_scan():
    """{kind: [(key, addr, routine, detail)]} over the compiled code."""
    import boolsurvey
    out = {'lowword': [], 'narrowed': [], 'widened': [], 'heldop': []}
    for r0, r1, name in routines():
        ins = a5survey.reached(r0, r1)
        labels = boolsurvey.branch_targets(ins)
        rt = '%06X %s' % (r0, name)
        for kind, d, a in lowword_sites(ins):
            out['lowword'].append((kind, a, rt, 'd=%d' % d))
        for s, cls, a, w, t in narrowed_sites(ins, labels):
            out['narrowed'].append(('%s<-%s' % (s, cls), a, rt, '%s; %s' % (w, t)))
        for kind, a in widened_sites(ins, labels):
            out['widened'].append((kind, a, rt, ''))
        for op, cls, a, w, t in heldop_sites(ins, labels):
            out['heldop'].append(('%s.w<-%s' % (op, cls), a, rt, '%s; %s' % (w, t)))
    return out


def narrow_summary(res):
    for kind in ('lowword', 'narrowed', 'widened', 'heldop'):
        print('%s:' % kind)
        c = Counter(k for k, *_ in res[kind])
        for key, n in c.most_common():
            ex = [a for k, a, *_ in res[kind] if k == key][:3]
            print('  %-24s %4d  e.g. %s' % (key, n, ', '.join('0x%X' % a for a in ex)))


def main(argv):
    if not argv or argv[0] not in ('calls', 'inline', 'summary', 'scale', 'scales',
                                   'narrow', 'narrows'):
        print(__doc__)
        return 2
    if argv[0] == 'narrow':
        narrow_summary(narrow_scan())
        return 0
    if argv[0] == 'narrows':
        kind = argv[1] if len(argv) > 1 else 'narrowed'
        for key, a, rt, detail in narrow_scan().get(kind, []):
            print('0x%06X %-22s %-30s %s' % (a, key, rt, detail))
        return 0
    if argv[0] in ('scale', 'scales'):
        sites = scale_scan()
        if argv[0] == 'scale':
            scale_summary(sites)
            return 0
        for s in sites:
            print('0x%06X %6d %-10s %-5s %-6s %-5s %-26s %-34s | %s' % (
                s['addr'], s['coef'], s['form'], s['idx'] or '-', s['x'],
                'zext' if s['zext'] else 'ext' if s['ext'] else '-',
                '%06X %s' % s['routine'], s['use'], '; '.join(s['chain'])))
        return 0
    calls, inline = scan(with_live=argv[0] != 'inline')
    if argv[0] == 'calls':
        for c in calls:
            print('0x%06X %-10s %-26s pad=%-3s b=%-24s a=%-24s pop=%-3s live=%-9s use=%s' % (
                c['addr'], c['entry'], '%06X %s' % c['routine'],
                'in' if c['pad_inside'] else 'yes' if c['pad'] else 'no', c['b'], c['a'],
                'yes' if c['pop'] else 'no', ','.join(c['live'] or []) if c.get('live') is not None
                else '?', c['use']))
    elif argv[0] == 'inline':
        for s in inline:
            print('0x%06X %-6s %-4s %-8s %-30s %-26s %s' % (
                s['addr'], s['op'], s['src'], s['cls'], s['text'],
                '%06X %s' % s['routine'], s['use']))
    else:
        summary(calls, inline)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
