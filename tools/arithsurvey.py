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

usage:
    tools/arithsurvey.py calls       # one line per M$ARITH call site
    tools/arithsurvey.py inline      # one line per inline mul/div
    tools/arithsurvey.py summary     # the counts quoted in codegen-patterns 8

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


def main(argv):
    if not argv or argv[0] not in ('calls', 'inline', 'summary'):
        print(__doc__)
        return 2
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
