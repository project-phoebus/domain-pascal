#!/usr/bin/env python3
"""cachesurvey.py DIS [- NAMES] - test docs/register-model.md's cache
predictions over a per-routine disassembly: the image's (tools/alldis.py
SAU CPU OUT) or dpc's (tools/objdis.py OBJ...), restricted to the routine
names in the file NAMES when given (step 2 compares the two over the
routines both have).

Only routines whose first instruction is `linkw` (compiled code by
codegen-patterns 0's signs) and with no `jmp` through a table are scanned.

An *entry* says register R holds memory operand M at size S.  It is
created by a load (`move.S M,R`) or a store (`move.S R,M`) and killed
only by a write of R, a write of M (same text, or a write that may alias:
through a pointer register kills every pointer / unit / absolute entry, a
frame write kills the same text), or a write of a register M's address
uses.  Labels and calls do NOT kill entries: they set flags, so the
counts say what the original did across them.  (Calls kill the scratch
registers D0/D1/A0/A1 for real.)

Events, at each later read of M while some register holds (M, S):
  reread  the instruction reads M from memory again;
  reuse   (counted at reads of R) the register is read again for M: after
          a store-created entry any read; after a load-created one a read
          beyond the first.
Each is keyed by (kind of M, origin load/store, crossed label, crossed
call).  Destructive uses: `move M,Dk` / `move Dj,Dk` while Dj holds M,
followed by an in-place operation on Dk.
"""
import collections
import re
import sys

SCRATCH = {'d0', 'd1', 'a0', 'a1'}
REG = re.compile(r'%(d[0-7]|a[0-7]|fp|sp)')


def norm(r):
    return {'fp': 'a6', 'sp': 'a7'}.get(r, r)


def split_ops(s):
    out, depth, cur = [], 0, ''
    for ch in s:
        if ch == '(':
            depth += 1
        elif ch == ')':
            depth -= 1
        if ch == ',' and depth == 0:
            out.append(cur)
            cur = ''
        else:
            cur += ch
    if cur:
        out.append(cur)
    return [o.strip() for o in out]


def is_reg(o):
    return re.fullmatch(r'%(d[0-7]|a[0-7]|fp|sp)', o) is not None


def is_mem(o):
    if o.startswith('#') or is_reg(o) or o in ('%sr', '%ccr', '%usp'):
        return False
    return True


def cacheable(o):
    # Not auto-increment/decrement, not the stack, not PC-relative.
    return (is_mem(o) and not o.endswith('@+') and not o.endswith('@-')
            and '%sp' not in o and '%pc' not in o)


def kind(o):
    if re.match(r'%fp@\(-', o):
        return 'frame'
    if o.startswith('%fp@'):
        return 'param'
    if o.startswith('%a5@'):
        return 'unit'
    if '%' not in o:
        return 'abs'
    return 'pointer'


def regs_in(o):
    return {norm(m) for m in REG.findall(o)}


SIZE = {'b': 'b', 'w': 'w', 'l': 'l'}


def mnem_size(m):
    m2 = m[:-1] if m[-1] in 'bwl' and m not in ('bsr', 'bcl', 'bal') else m
    return m2, (m[-1] if m[-1] in 'bwl' else None)


WRITE_LAST = ('move', 'movea', 'moveq', 'clr', 'add', 'adda', 'addq', 'addi',
              'sub', 'suba', 'subq', 'subi', 'and', 'andi', 'or', 'ori', 'eor',
              'eori', 'lsl', 'lsr', 'asl', 'asr', 'rol', 'ror', 'roxl', 'roxr',
              'neg', 'negx', 'not', 'ext', 'extb', 'swap', 'mulu', 'muls',
              'divu', 'divs', 'lea', 'bset', 'bclr', 'bchg', 'addx', 'subx',
              'bfins', 'bfset', 'bfclr', 'bfchg', 'bfextu', 'bfexts')
PURE_DEST = ('move', 'movea', 'moveq', 'clr', 'lea')
INPLACE = ('add', 'adda', 'addq', 'addi', 'sub', 'suba', 'subq', 'subi', 'and',
           'andi', 'or', 'ori', 'eor', 'eori', 'lsl', 'lsr', 'asl', 'asr',
           'rol', 'ror', 'neg', 'not', 'ext', 'extb', 'swap', 'mulu', 'muls',
           'divu', 'divs')


def parse(path):
    routines = collections.OrderedDict()
    cur = None
    for line in open(path):
        if line.startswith('== '):
            cur = line.split()[1]
            routines[cur] = []
            continue
        m = re.match(r'\s*([0-9a-f]+):\s+[0-9a-f ]+?\s{2,}(\S+)\s*(.*)', line)
        if m and cur:
            routines[cur].append((int(m.group(1), 16), m.group(2), m.group(3)))
    return routines


def scan(routines, listkey=None):
    tally = collections.Counter()
    examples = collections.defaultdict(list)

    def ev(key, name, addr):
        tally[key] += 1
        if len(examples[key]) < 400:
            examples[key].append('%s 0x%X' % (name, addr))

    for name, insns in routines.items():
        if not insns or insns[0][1] != 'linkw':
            continue
        if any(i[1] == 'jmp' for i in insns):
            continue
        targets = collections.defaultdict(list)
        for a0, m, ops in insns:
            if (m.startswith('b') and m not in ('bset', 'bclr', 'bchg', 'btst')
                    and not m.startswith('bf')) or m.startswith('db'):
                t = ops.split(',')[-1].strip().split(' ')[0]
                if re.fullmatch(r'(0x)?[0-9a-f]+', t):
                    targets[int(t, 16)].append(a0)
        # entries: reg -> dict(op, size, origin, label, call, reads)
        ent = {}
        prev_uncond = False
        for k, (addr, m, ops) in enumerate(insns):
            if prev_uncond:
                for e in ent.values():
                    e['label'] = True
            elif addr in targets:
                # A join kills an entry only if some branch into it comes
                # from before the entry was created, or from after the
                # label (a back edge): R7's rule.
                for e in ent.values():
                    e['join'] = True
                    if any(src < e['at'] or src >= addr for src in targets[addr]):
                        e['label'] = True
            prev_uncond = m in ('bras', 'braw', 'bral', 'rts', 'jmp', 'rte')
            base, sz = mnem_size(m)
            o = split_ops(ops) if ops else []
            if m in ('jsr', 'bsrw', 'bsrs', 'bsrl'):
                for r in list(ent):
                    if r in SCRATCH:
                        del ent[r]
                    else:
                        ent[r]['call'] = True
                continue
            # Which operands are read / written.
            writes_last = base in WRITE_LAST or m.startswith('s') and base not in ('sub', 'suba', 'subq', 'subi', 'subx', 'swap')
            if base in ('cmp', 'cmpa', 'cmpi', 'tst', 'btst', 'cmpm', 'chk', 'bftst') or base.startswith('b') and base not in ('bset', 'bclr', 'bchg') and not base.startswith('bf'):
                writes_last = False
            pure = base in PURE_DEST or (m.startswith('s') and base not in ('sub', 'suba', 'subq', 'subi', 'subx', 'swap'))
            reads = o if not (writes_last and pure) else o[:-1]
            if base in ('lea', 'pea'):
                reads_mem = []
            else:
                reads_mem = [x for x in reads if cacheable(x)]
            # Register reads (for reuse).
            reg_reads = set()
            for x in reads:
                if is_reg(x):
                    reg_reads.add(norm(x[1:]))
            if writes_last and not pure and o and is_reg(o[-1]):
                reg_reads.add(norm(o[-1][1:]))
            # Rereads of a held memory operand.
            held = {}
            for r, e in ent.items():
                held.setdefault((e['op'], e['size']), r)
            def use_class():
                if base in ('cmp', 'cmpa', 'cmpi', 'tst'):
                    return 'test'
                if base in ('btst', 'bftst', 'bfextu', 'bfexts'):
                    return 'bit'
                if o and o[-1].endswith('@-') and base in ('move', 'movea', 'pea'):
                    return 'push'
                if base in ('move', 'movea') and o and is_reg(o[-1]):
                    pm = insns[k - 1] if k else None
                    if pm and pm[1].startswith('clr') and pm[2] == o[-1]:
                        return 'load-zx'
                    nx = insns[k + 1] if k + 1 < len(insns) else None
                    if nx and mnem_size(nx[1])[0] in INPLACE and split_ops(nx[2])[-1:] == [o[-1]]:
                        return 'load-inpl'
                    return 'load'
                if base in ('move', 'movea'):
                    return 'copy-mem'
                if writes_last and o and not is_reg(o[-1]):
                    return 'rmw'
                return 'operand'
            uc = use_class()
            for x in reads_mem:
                for (op, s), r in held.items():
                    if op == x and (sz is None or s == sz):
                        e = ent[r]
                        if not e['label']:
                            ev(('rr-class', uc + '/' + e['origin'], kind(x), e['call']), name, addr)
                            if not e['call']:
                                ev(('holder', 'reread', e['origin'], r[0], o[-1][1] if uc == 'load' else '-', uc), name, addr)
                                ev(('forward', 'reread', e['origin'], r[0], e['join'], uc), name, addr)
                        ev(('reread', kind(x), e['origin'], e['label'], e['call']), name, addr)
            # Reuses of a held register.
            for r in reg_reads:
                e = ent.get(r)
                if not e:
                    continue
                e['reads'] += 1
                if e['origin'] == 'store' or e['reads'] > 1:
                    if not e['label']:
                        ev(('re-class', use_class() + '/' + e['origin'], kind(e['op']), e['call']), name, addr)
                        if not e['call']:
                            ev(('holder', 'reuse', e['origin'], r[0], o[-1][1] if use_class() == 'load' else '-', use_class()), name, addr)
                            ev(('forward', 'reuse', e['origin'], r[0], e['join'], use_class()), name, addr)
                    ev(('reuse', kind(e['op']), e['origin'], e['label'], e['call']), name, addr)
            # Destructive: an in-place op on a register just loaded /
            # copied while another register holds the operand.
            if base in INPLACE and o and is_reg(o[-1]) and k > 0:
                dk = norm(o[-1][1:])
                pm, po = insns[k - 1][1], split_ops(insns[k - 1][2]) if insns[k - 1][2] else []
                if mnem_size(pm)[0] in ('move', 'movea') and len(po) == 2 and is_reg(po[1]) and norm(po[1][1:]) == dk:
                    src = po[0]
                    if cacheable(src):
                        if src in [e['op'] for r, e in pre_ent.items() if r != dk]:
                            ev(('destructive', 'reload', kind(src)), name, addr)
                    elif is_reg(src):
                        sr = norm(src[1:])
                        e = pre_ent.get(sr)
                        if e:
                            ev(('destructive', 'copy', kind(e['op'])), name, addr)
            pre_ent = {r: dict(e) for r, e in ent.items()}
            # Writes.
            if not writes_last or not o:
                continue
            d = o[-1]
            if is_reg(d):
                r = norm(d[1:])
                ent.pop(r, None)
                for r2 in list(ent):
                    if r in regs_in(ent[r2]['op']):
                        del ent[r2]
                if base in ('move', 'movea') and len(o) == 2 and cacheable(o[0]):
                    ent[r] = dict(op=o[0], size=sz, origin='load', label=False, call=False, reads=0, at=addr, join=False)
                if d.endswith('@+') or d.endswith('@-'):
                    pass
            else:
                # A memory write.
                if d.endswith('@-') or d.endswith('@+'):
                    for r in regs_in(d):
                        for r2 in list(ent):
                            if r in regs_in(ent[r2]['op']):
                                del ent[r2]
                    continue
                kd = kind(d)
                for r2 in list(ent):
                    e = ent[r2]
                    if e['op'] == d or (kd == 'pointer' and kind(e['op']) in ('pointer', 'unit', 'abs')) \
                            or (kd in ('unit', 'abs') and kind(e['op']) == 'pointer') \
                            or (kd == kind(e['op']) and ('%d' in d or '%d' in e['op'])):
                        del ent[r2]
                if base in ('move', 'movea') and len(o) == 2 and is_reg(o[0]) and cacheable(d):
                    r = norm(o[0][1:])
                    ent[r] = dict(op=d, size=sz, origin='store', label=False, call=False, reads=0, at=addr, join=False)
    return tally, examples


def main():
    rs = parse(sys.argv[1])
    if len(sys.argv) > 3:
        keep = set(open(sys.argv[3]).read().split())
        rs = collections.OrderedDict((k, v) for k, v in rs.items() if k in keep)
    print('routines', sum(1 for v in rs.values() if v and v[0][1] == 'linkw'))
    tally, ex = scan(rs)
    listkey = sys.argv[2] if len(sys.argv) > 2 and sys.argv[2] != '-' else None
    rows = collections.defaultdict(lambda: collections.Counter())
    for key, n in tally.items():
        if key[0] in ('destructive', 'rr-class', 're-class', 'holder', 'forward'):
            continue
        what, k, origin, label, call = key
        rows[(k, origin, label, call)][what] += n
    print('%-8s %-6s %-6s %-5s %8s %8s' % ('kind', 'origin', 'label', 'call', 'reuse', 'reread'))
    for key in sorted(rows):
        r = rows[key]
        print('%-8s %-6s %-6s %-5s %8d %8d' % (key[0], key[1], key[2], key[3], r['reuse'], r['reread']))
    cls = collections.Counter()
    for key, n in tally.items():
        if key[0] in ('rr-class', 're-class') and not key[3]:
            cls[(key[1], key[2], key[0])] += n
    print('by use, no killing label, no call (use, kind: reuse / reread):')
    for (u, k) in sorted({(a, b) for a, b, _ in cls}):
        print('  %-9s %-8s %5d / %5d' % (u, k, cls[(u, k, 're-class')], cls[(u, k, 'rr-class')]))
    hold = collections.Counter()
    for key, n in tally.items():
        if key[0] == 'holder':
            hold[key[1:]] += n
    print('by holder (origin, holder a/d, wanted a/d, use: reuse / reread):')
    for k in sorted({x[1:] for x in hold}):
        print('  %-6s %s %s %-10s %5d / %5d' % (k + (hold[('reuse',) + k], hold[('reread',) + k])))
    fw = collections.Counter()
    for key, n in tally.items():
        if key[0] == 'forward':
            fw[key[1:]] += n
    print('after a forward join (origin, holder, crossed, use: reuse / reread):')
    for k in sorted({x[1:] for x in fw}, key=str):
        print('  %-6s %s %-5s %-10s %5d / %5d' % (k + (fw[('reuse',) + k], fw[('reread',) + k])))
    print('destructive:')
    for key, n in sorted(tally.items()):
        if key[0] == 'destructive':
            print('  ', key[1:], n, ex[key][:4])
    if listkey:
        for key in tally:
            if listkey in ' '.join(map(str, key)):
                print(key, ex[key])


if __name__ == '__main__':
    main()
