#!/usr/bin/env python3
"""localsurvey.py - which scalar locals the original compiler kept in
registers, from routines whose Domain Pascal source is known (bead
domain-pascal-iyd; docs/codegen-patterns.md 12.5).

    localsurvey.py run [--rev REV] [--source DIR] [--work DIR] [--dpc DPC]
                       [--module PATH] [-v]
    localsurvey.py c [--rev REV] [--source DIR] [--work DIR] [-v] [--misses]
                     [--dump FILE] [FILE.c ...]
    localsurvey.py onestmt [--rev REV] [--source DIR] [--work DIR] [FILE.c ...]

`run' exports ../source's domain_os/ at REV (default HEAD; as
tools/srcsurvey.py does, into WORK/export, default tmp/localsurvey; the
module file gets srcsurvey's incompleteness rewrites only) and runs
`dpc --dump-locals' on the module (default tty/tty.pas, the one Pascal
module of ../source).  dpc lowers every routine and prints, per routine,
how the body uses each scalar local: its assignments and reads, where
they are (routine-level statement or inside an `if'/loop/`case', inside
a call's argument, inside a condition), whether a call lies between an
assignment and a read, whether it is dereferenced or its address used
(`dpc_m68k::locals').

Each routine is found in the image from its header comment ("Original
address: 0xADDR, N bytes", ../source's fragment format; nested routines
have none and are left out) and disassembled (tools/a5survey.py
`reached').  A local is *in the frame* when some instruction addresses
its slot, `(d,A6)' with d inside the local; else *in a register* (its
slot stays allocated: the `link' size is the source's frame).  The
register the source's declaration comment names (`{ D1 }') is shown as
given; it is the fragment author's reading, not checked here.

The table lists every local that is assigned or read; the summary scores
the candidate rules of RULES (each a predicate "held in a register")
against the image, over the locals that are neither dereferenced nor
otherwise referenced (address taken, partial access) nor `for' indices,
with the misses of each.

`c' does the same for ../source's C (see "The C corpus" below): the
locals whose declarations carry a register or frame note, scored by the
same rules, split by data and pointer locals and by how many of D2-D7
the routine saves (register pressure).

`onestmt' (bead domain-pascal-s4a5; codegen-patterns 12.2, "One-
statement bodies") lists the C functions (test directories left out)
whose body is one `if', `switch' or loop statement reading a non-pointer
by-value parameter, finds each in the image (its `Original address'
header, else the map), and classifies each such parameter by its first
reference there: `entry' (loaded into a register before the first branch
or call), `frame' (tested or used in its slot), `pushed', `late', `none'.

Python 3 standard library, dpc, clang (for `c' and `onestmt'), m68k-elf-objdump.
Writes only under WORK (and --dump FILE).
"""

import argparse
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

_FP = re.compile(r'%fp@\((-?\d+)')
_LINK = re.compile(r'^linkw %fp,#(-?\d+)')
_NOTE_OFF = re.compile(r'\(-0x([0-9a-fA-F]+),A6\)')


_MOVEM = re.compile(r'^moveml (\S+),%sp@-')


def saved_regs(insns):
    """The registers the prologue saves (`movem.l', `move.l D2', `pea
    (An)' after the `link'), as a set of names like 'd2', 'a5'."""
    out = set()
    for _a, _n, t in insns[1:3]:
        m = _MOVEM.match(t)
        if m:
            for part in m.group(1).split('/'):
                part = part.replace('%', '')
                if '-' in part:
                    lo, hi = part.split('-')
                    out |= {lo[0] + str(k) for k in range(int(lo[1]), int(hi[1]) + 1)}
                else:
                    out.add(part)
            break
        m = re.match(r'^movel %(d\d),%sp@-$|^pea %(a\d)@$', t)
        if m:
            out.add(m.group(1) or m.group(2))
            break
    return out


def link_size(insns):
    """The frame size of the routine's `link.w A6,#-n' (n), or None."""
    for _a, _n, t in insns[:1]:
        m = _LINK.match(t)
        if m:
            return -int(m.group(1))
    return None


def slot_used(insns, offset, nbytes):
    """Whether some instruction of INSNS ([(addr, len, text)], objdump
    syntax) addresses a byte of offset..offset+nbytes-1 off A6."""
    for _a, _n, t in insns:
        for m in _FP.finditer(t):
            d = int(m.group(1))
            if offset <= d < offset + nbytes:
                return True
    return False


def parse_facts(text):
    """The routines of `dpc --dump-locals' output TEXT."""
    return [json.loads(line) for line in text.splitlines() if line.strip()]


def decl_notes(text):
    """{(routine name, local name): the comment after its declaration}
    for the `var' parts of TEXT (lower case names).  Only declarations of
    one name, `name: type; { comment }' on one line, are read."""
    out = {}
    routine = None
    for line in text.splitlines():
        m = re.match(r'\s*(?:procedure|function)\s+([\w$]+)', line, re.I)
        if m:
            routine = m.group(1).lower()
            continue
        m = re.match(r'\s*([\w$]+)\s*:(?!=)\s*[^;{]*;\s*\{\s*(.*?)\s*\}\s*$', line)
        if m and routine:
            out.setdefault((routine, m.group(1).lower()), m.group(2))
    return out


# --- features and rules ----------------------------------------------------

def features(r, l):
    """The facts of local L of routine R (both from parse_facts) as flat
    features."""
    st, ld = l['stores'], l['loads']
    return {
        'stores': len(st),
        'loads': len(ld),
        'const': any(s['const'] for s in st),
        'all_const': bool(st) and all(s['const'] for s in st),
        'from_call': any(s['call'] for s in st),
        'nested_store': any(s['nested'] for s in st),
        'loop_store': any(s['loop'] is not None for s in st),
        'loop_load': any(x['loop'] is not None for x in ld),
        'arg': any(x['arg'] for x in ld),
        'pushed': any(x.get('whole') for x in ld),
        'only_pushed': bool(ld) and all(x.get('whole') for x in ld),
        'cond': any(x['cond'] for x in ld),
        'across_call': l['across_call'],
        'calls': len(r['calls']),
        'type': l['type'],
        'bytes': l['bytes'],
        # the narrow rule of codegen-patterns 12.5 before bead iyd
        'narrow': (len(st) == 1 and not st[0]['nested'] and not st[0]['const']
                   and all(x['top'] > st[0]['top'] for x in ld)
                   and not any(x['arg'] for x in ld)),
    }


def plain(l):
    """A local the rules decide: assigned or read, and only as a whole."""
    return (not l['deref'] and not l['other'] and not l['for_index']
            and (l['stores'] or l['loads']))


RULES = [
    ('narrow (12.5 before iyd)', lambda f: f['narrow']),
    ('never in a call argument', lambda f: not f['arg']),
    ('not in a call argument, not live across a call',
     lambda f: not f['arg'] and not f['across_call']),
    ('not in a call argument unless in a loop',
     lambda f: not f['arg'] or f['loop_load'] or f['loop_store']),
    ('every plain local', lambda f: True),
    ('unless every read pushes it', lambda f: not f['only_pushed']),
    ('unless some read pushes it', lambda f: not f['pushed']),
]


def score(rows):
    """[(rule name, hits, misses [(routine, local, image)])] over ROWS
    [(routine, local, features, image class)]."""
    out = []
    for name, pred in RULES:
        hits, misses = 0, []
        for rn, ln, f, img in rows:
            if (img == 'register') == bool(pred(f)):
                hits += 1
            else:
                misses.append((rn, ln, img))
        out.append((name, hits, misses))
    return out


# --- run -------------------------------------------------------------------

def routine_ranges(tree, path):
    """{routine name (lower case): (address, size)} of the module's
    routines with a header address (srcsurvey's fragment parse; the map
    extent for a routine whose "Image addresses:" row gives no size, bead
    domain-pascal-zlnu)."""
    import srcsurvey
    mod = srcsurvey.Module(tree, path)
    out = {}
    linkmap = None
    for name, (_inc, r) in mod.routines.items():
        if r.addr and r.size is None:
            if linkmap is None:
                import sau2map
                linkmap = sau2map.load()
            out[name.lower()] = (r.addr, srcsurvey.routine_size(r, mod.starts, linkmap))
        elif r.addr and r.size:
            out[name.lower()] = (r.addr, r.size)
    return out, mod


def short(f):
    st = ','.join(('n' if s['nested'] else 't') + ('L' if s['loop'] else '')
                  + ('k' if s['const'] else '') + ('c' if s['call'] else '')
                  for s in f['stores'])
    ld = ','.join(('n' if x['nested'] else 't') + ('L' if x['loop'] else '')
                  + ('p' if x.get('whole') else 'a' if x['arg'] else '')
                  + ('?' if x['cond'] else '')
                  for x in f['loads'])
    return st or '-', ld or '-'


def cmd_run(a):
    import a5survey
    import srcsurvey
    work = os.path.abspath(a.work)
    tree, commit = srcsurvey.export_rev(os.path.abspath(a.source), a.rev, work)
    p = os.path.join(tree, a.module)
    with open(p) as fh:
        text, _d = srcsurvey.prepare_text(fh.read(),
                                          lambda q: os.path.exists(os.path.join(tree, q)))
    with open(p, 'w') as fh:
        fh.write(text)
    print('localsurvey: %s at %s (%s), %s' % (os.path.normpath(a.source), a.rev, commit[:12],
                                             a.module))
    # --strict: a fidelity survey; best-effort lowering paths are not the
    # image's (bead domain-pascal-5kgg)
    res = subprocess.run([a.dpc, '-I', '.', '--strict', '--dump-locals', a.module], cwd=tree,
                         capture_output=True, text=True)
    if res.returncode != 0:
        sys.stderr.write(res.stderr)
        return 1
    facts = parse_facts(res.stdout)
    ranges, mod = routine_ranges(tree, a.module)
    notes = {}
    for inc in mod.fragments:
        notes.update(decl_notes(mod.frag_text[inc]))
    rows = []
    print('%-30s %-14s %-8s %5s %-6s %-12s %-16s %-5s %s' %
          ('routine', 'local', 'type', 'off', 'image', 'stores', 'loads', 'xcall', 'note'))
    for r in facts:
        rng = ranges.get(r['routine'].lower())
        for l in r['locals']:
            if not (l['stores'] or l['loads']):
                continue
            note = notes.get((r['routine'].lower(), l['name'].lower()), '')
            off = l['offset']
            if rng is None:
                img = '?'
            else:
                insns = a5survey.reached(rng[0], rng[0] + rng[1])
                if link_size(insns) != (r['frame'] + 3) // 4 * 4:
                    # dpc lays the frame out otherwise than the image:
                    # the slot from the declaration's note, if it gives one
                    m = _NOTE_OFF.search(note)
                    off = -int(m.group(1), 16) if m else None
                if off is None:
                    img = '?'
                else:
                    img = 'frame' if slot_used(insns, off, l['bytes']) else 'register'
            kind = ('deref' if l['deref'] else 'other' if l['other']
                    else 'for' if l['for_index'] else l['type'])
            st, ld = short(l)
            print('%-30s %-14s %-8s %5s %-6s %-12s %-16s %-5s %s' %
                  (r['routine'], l['name'], kind, off if off is not None else '?', img, st, ld,
                   'yes' if l['across_call'] else '-', note[:40]))
            if img != '?' and plain(l):
                f = features(r, l)
                f['saved'] = sorted(saved_regs(insns))
                rows.append((r['routine'], l['name'], f, img))
    nreg = sum(1 for x in rows if x[3] == 'register')
    print()
    print('plain locals decided: %d (%d in a register, %d in the frame)'
          % (len(rows), nreg, len(rows) - nreg))
    for name, hits, misses in score(rows):
        print('  %-50s %3d/%d' % (name, hits, len(rows)))
        if a.verbose:
            for rn, ln, img in misses:
                print('      miss: %s %s (image: %s)' % (rn, ln, img))
    return 0


# --- the C corpus ------------------------------------------------------------
#
# ../source's C reconstructions annotate many locals with where the image
# keeps them: `uint16_t token;  /* D3 */', `clock_t abs_clock;  /* A6-0x14 */'.
# `c' parses each file with clang (-Xclang -ast-dump=json, the host build's
# flags: -std=gnu2x -DARCH_HOST -I.), takes the scalar locals of every
# function with such a note, and computes the same facts as dpc's
# --dump-locals from the C (c_facts).  A C local is not always a local of
# the original Pascal (a decompiler names register values too), so this
# corpus is weaker evidence than the Pascal one.

_SCALAR_C = re.compile(r'^(?:(?:const|volatile|unsigned|signed|short|long|int|char|_Bool|bool)\s*)+$'
                       r'|^u?int(?:8|16|32)_t$|^enum\b|\*$')
_C_REG = re.compile(r'^\s*([DA][0-7])\b')
_C_SLOT = re.compile(r'A6\s*-\s*0x([0-9a-fA-F]+)|\(-0x([0-9a-fA-F]+),\s*A6\)')
_C_NOTE = re.compile(r'/\*(.*?)\*/')


# C types that stand for Pascal records: status_$t is a packed variant
# record (base/base.ins.pas), always in memory.
C_RECORDS = {'status_$t'}


def c_scalar(ty):
    """Whether the clang type dict TY is a scalar the rules cover: an
    integer, a boolean, an enumeration, a pointer (bytes are not checked),
    not one of C_RECORDS."""
    if re.sub(r'\b(const|volatile)\b', '', ty.get('qualType', '')).strip() in C_RECORDS:
        return False
    t = (ty.get('desugaredQualType') or ty.get('qualType') or '').strip()
    if '[' in t or '(' in t:
        return False
    if t.endswith('*'):
        return True
    t = re.sub(r'\b(const|volatile)\b', '', t).strip()
    return bool(_SCALAR_C.match(t))


def c_note(text, offset):
    """The comment after a declaration at byte OFFSET of TEXT, on its line."""
    end = text.find('\n', offset)
    line = text[offset:end if end >= 0 else len(text)]
    m = _C_NOTE.search(line)
    return m.group(1).strip() if m else ''


def _strip(e):
    while e and e.get('kind') in ('ImplicitCastExpr', 'ParenExpr', 'CStyleCastExpr'):
        e = (e.get('inner') or [None])[0]
    return e


def _operator_call(callee):
    """Whether a C call stands for an operator of the original: M$ARITH
    (the compiler's 32-bit multiply/divide runtime, `M$MIS$LLL', ...) or
    ../source's ARCH_ helpers (arch/arch.h: `ARCH_DIVS_W', ...)."""
    c = _strip(callee)
    name = (c or {}).get('referencedDecl', {}).get('name', '') if c else ''
    return name.startswith(('M$', 'ARCH_', '__builtin'))


def _unslot(e):
    """An argument E without the `<< 16' of ARCH_PASCAL_WORD_SLOT (a word
    in the high half of a longword slot, ../source arch/arch.h)."""
    e = _strip(e)
    if (e and e.get('kind') == 'BinaryOperator' and e.get('opcode') == '<<'
            and _strip(e['inner'][1]).get('kind') == 'IntegerLiteral'
            and _strip(e['inner'][1]).get('value') == '16'):
        return _strip(e['inner'][0])
    return e


class _CWalk:
    """The facts of one C function body, in --dump-locals' form."""

    def __init__(self, locals_):
        self.loc = locals_            # decl id -> local dict
        self.calls = []
        self.seq = 0
        self.top = 0
        self.nested = False
        self.lp = None
        self.in_arg = False
        self.whole = False
        self.in_cond = False

    def site(self):
        return {'top': self.top, 'seq': self.seq, 'nested': self.nested,
                'loop': list(self.lp) if self.lp else None}

    def ref(self, e):
        e = _strip(e)
        if e and e.get('kind') == 'DeclRefExpr':
            return self.loc.get(e.get('referencedDecl', {}).get('id'))
        return None

    def store(self, l, value):
        v = _strip(value)
        l['stores'].append(dict(self.site(),
                                const=bool(v) and v.get('kind') in ('IntegerLiteral',
                                                                    'CharacterLiteral'),
                                call=bool(v) and v.get('kind') == 'CallExpr'))

    def load(self, l):
        l['loads'].append(dict(self.site(), arg=self.in_arg, whole=self.whole,
                               cond=self.in_cond))

    def expr(self, e):
        if not e:
            return
        k = e.get('kind')
        inner = e.get('inner') or []
        if k == 'DeclRefExpr':
            l = self.ref(e)
            if l is not None:
                self.load(l)
            return
        if k == 'BinaryOperator' and e.get('opcode') == '=':
            l = self.ref(inner[0])
            self.expr(inner[1])
            if l is not None:
                self.store(l, inner[1])
            else:
                self.lvalue(inner[0])
            return
        if k == 'CompoundAssignOperator' or (
                k == 'UnaryOperator' and e.get('opcode') in ('++', '--')):
            l = self.ref(inner[0])
            for x in inner[1:]:
                self.expr(x)
            if l is not None:
                self.load(l)
                self.store(l, None)
            else:
                self.lvalue(inner[0])
            return
        if k == 'UnaryOperator' and e.get('opcode') == '&':
            l = self.ref(inner[0])
            if l is not None:
                l['other'] = True
            else:
                self.lvalue(inner[0])
            return
        if k == 'UnaryOperator' and e.get('opcode') == '*':
            l = self.ref(inner[0])
            if l is not None:
                l['deref'] = True
            else:
                self.expr(inner[0])
            return
        if k == 'MemberExpr':
            l = self.ref(inner[0])
            if l is not None:
                l['deref' if e.get('isArrow') else 'other'] = True
            elif e.get('isArrow'):
                self.expr(inner[0])
            else:
                self.lvalue(inner[0])
            return
        if k == 'ArraySubscriptExpr':
            l = self.ref(inner[0])
            if l is not None:
                l['deref'] = True
            else:
                self.lvalue(inner[0])
            self.expr(inner[1])
            return
        if k == 'CallExpr' and _operator_call(inner[0]):
            for a in inner[1:]:
                self.expr(a)
            return
        if k == 'CallExpr':
            outer = self.in_arg
            self.in_arg = True
            for a in inner[1:]:
                self.whole = self.ref(_unslot(a)) is not None
                self.expr(a)
                self.whole = False
            self.in_arg = outer
            self.expr(inner[0]) if self.ref(inner[0]) else None
            self.calls.append(self.site())
            return
        if k == 'ConditionalOperator' or k == 'BinaryOperator' and e.get('opcode') in ('&&', '||'):
            outer = self.in_cond
            self.in_cond = True
            self.expr(inner[0])
            self.in_cond = outer
            for x in inner[1:]:
                self.expr(x)
            return
        for x in inner:
            if isinstance(x, dict) and x.get('kind', '').endswith(('Expr', 'Operator',
                                                                     'Literal')):
                self.expr(x)

    def lvalue(self, e):
        """An object written or addressed that is not a local itself."""
        self.expr(e)

    def cond(self, e):
        outer = self.in_cond
        self.in_cond = True
        self.expr(e)
        self.in_cond = outer

    def stmt(self, st, nested):
        if not st:
            return
        k = st.get('kind')
        inner = st.get('inner') or []
        if k == 'CompoundStmt':
            for x in inner:
                self.stmt(x, nested)
            return
        outer = (self.nested, self.lp)
        self.seq += 1
        self.nested = nested
        head = self.seq
        if k == 'DeclStmt':
            for d in inner:
                if d.get('kind') == 'VarDecl' and d.get('inner'):
                    l = self.loc.get(d.get('id'))
                    self.expr(d['inner'][0])
                    if l is not None:
                        self.store(l, d['inner'][0])
        elif k == 'IfStmt':
            self.cond(inner[0])
            for x in inner[1:]:
                self.stmt(x, True)
        elif k in ('WhileStmt', 'DoStmt', 'ForStmt'):
            last = head + _count_stmts(inner)
            self.lp = (head, last)
            if k == 'ForStmt':
                parts = inner + [{}] * (5 - len(inner))
                self.stmt(parts[0], nested)
                self.lp = (head, last)
                self.nested = True
                self.cond(parts[2])
                self.stmt(parts[4], True)
                self.expr(parts[3])
            elif k == 'WhileStmt':
                self.nested = True
                self.cond(inner[0])
                self.stmt(inner[1], True)
            else:
                self.stmt(inner[0], True)
                self.nested = True
                self.cond(inner[1])
        elif k == 'SwitchStmt':
            self.expr(inner[0])
            for x in inner[1:]:
                self.stmt(x, True)
        elif k in ('CaseStmt', 'DefaultStmt', 'LabelStmt'):
            self.seq -= 1
            for x in inner:
                if x.get('kind', '').endswith('Stmt') or x.get('kind') in (
                        'BinaryOperator', 'CallExpr', 'CompoundAssignOperator',
                        'UnaryOperator'):
                    self.stmt(x, nested)
        elif k == 'ReturnStmt':
            for x in inner:
                self.expr(x)
        elif k in ('BreakStmt', 'ContinueStmt', 'GotoStmt', 'NullStmt'):
            pass
        else:
            self.expr(st)
        self.nested, self.lp = outer

    def body(self, cs):
        for i, st in enumerate(cs.get('inner') or []):
            self.top = i
            self.stmt(st, False)


def _count_stmts(nodes):
    n = 0
    for x in nodes:
        if not isinstance(x, dict):
            continue
        k = x.get('kind', '')
        if k.endswith('Stmt') and k not in ('CompoundStmt', 'CaseStmt', 'DefaultStmt',
                                            'LabelStmt'):
            n += 1
        n += _count_stmts(x.get('inner') or [])
    return n


def c_functions(ast, text):
    """[(function name, {routine facts as --dump-locals})] of the functions
    of the clang JSON AST defined in TEXT (the main file), with the notes
    of their scalar locals ('note' in each local)."""
    out = []
    for fd in ast.get('inner') or []:
        if fd.get('kind') != 'FunctionDecl':
            continue
        body = [x for x in fd.get('inner') or [] if x.get('kind') == 'CompoundStmt']
        if not body:
            continue
        name = fd.get('name', '')
        off = fd.get('loc', {}).get('offset')
        if off is None or text[off:off + len(name)] != name:
            continue
        locs = {}
        _collect_locals(body[0], text, locs)
        if not locs:
            continue
        w = _CWalk(locs)
        w.body(body[0])
        for l in locs.values():
            l['across_call'] = _across(w.calls, l)
        out.append((name, {'routine': name, 'calls': w.calls,
                           'locals': list(locs.values())}))
    return out


def _collect_locals(node, text, out):
    for x in node.get('inner') or []:
        if not isinstance(x, dict):
            continue
        if (x.get('kind') == 'VarDecl' and x.get('storageClass') not in ('static', 'extern')
                and c_scalar(x.get('type', {}))):
            off = x.get('loc', {}).get('offset')
            note = c_note(text, off) if off is not None else ''
            t = x['type'].get('desugaredQualType') or x['type'].get('qualType', '')
            out[x['id']] = {'name': x.get('name', '?'), 'type': x['type'].get('qualType', ''),
                            'pointer': t.strip().endswith('*'),
                            'note': note, 'stores': [], 'loads': [], 'deref': False,
                            'other': False, 'for_index': False, 'bytes': None}
        _collect_locals(x, text, out)


def _across(calls, l):
    for r in l['loads']:
        for s in l['stores']:
            if any(s['seq'] < c['seq'] < r['seq'] for c in calls):
                return True
        if r['loop'] and any(r['loop'][0] <= c['seq'] <= r['loop'][1] for c in calls):
            return True
    return False


def c_class(note, insns):
    """The image class of a C local from its NOTE: `register' for a note
    naming a register, `frame' for a note giving an A6 slot that INSNS
    (the function's code, or None when not in the map) address; else
    None."""
    m = _C_SLOT.search(note)
    reg = _C_REG.match(note)
    if m:
        off = -int(m.group(1) or m.group(2), 16)
        names_reg = re.search(r'\b(D[0-7]|A[0-4])\b', note)
        if insns is None:
            return None if names_reg else 'frame'
        if slot_used(insns, off, 1):
            return 'frame'
        return 'register' if names_reg else None
    if reg and reg.group(1) not in ('A5', 'A6', 'A7'):
        return 'register'
    return None


def cmd_c(a):
    import a5survey
    import sau2map
    import srcsurvey
    work = os.path.abspath(a.work)
    tree, commit = srcsurvey.export_rev(os.path.abspath(a.source), a.rev, work)
    print('localsurvey c: %s at %s (%s)' % (os.path.normpath(a.source), a.rev, commit[:12]))
    m = sau2map.load()
    files = a.files or sorted(
        os.path.relpath(os.path.join(d, f), tree)
        for d, _s, fs in os.walk(tree) for f in fs if f.endswith('.c'))
    rows = []
    nfun = 0
    for path in files:
        with open(os.path.join(tree, path), errors='replace') as fh:
            text = fh.read()
        if not re.search(r';\s*/\*\s*([DA][0-7]\b|A6\s*-\s*0x)', text):
            continue
        res = subprocess.run(['clang', '-std=gnu2x', '-DARCH_HOST', '-I.', '-I' + os.path.dirname(path),
                              '-fsyntax-only', '-w', '-Xclang', '-ast-dump=json', path],
                             cwd=tree, capture_output=True, text=True)
        if not res.stdout:
            continue
        try:
            ast = json.loads(res.stdout)
        except ValueError:
            continue
        for name, r in c_functions(ast, text):
            try:
                ext = m.routine_extent(name)
            except Exception:
                ext = None
            insns = a5survey.reached(ext[0], ext[0] + ext[1]) if ext else None
            nfun += 1
            for l in r['locals']:
                img = c_class(l['note'], insns)
                if img is None or not plain(l):
                    continue
                f = features(r, dict(l, bytes=0, type=l['type']))
                f['saved'] = sorted(saved_regs(insns)) if insns else None
                f['dsaved'] = (sum(1 for x in f['saved'] if x[0] == 'd')
                               if insns else None)
                f['pointer'] = l['pointer']
                f['note'] = l['note']
                rows.append(('%s:%s' % (path, name), l['name'], f, img))
                if a.verbose:
                    st, ld = short(l)
                    print('%-44s %-16s %-8s %-14s %-20s %-4s %s' %
                          (path + ':' + name, l['name'], img, st[:14], ld[:20],
                           'x' if l['across_call'] else '-', l['note'][:30]))
    if a.dump:
        with open(a.dump, 'w') as fh:
            json.dump(rows, fh)
    print('functions with scalar locals: %d' % nfun)
    pressure_table(rows)
    for title, sel in (('data locals, D2-D7 not all used (fewer than 5 saved)',
                        lambda f: not f['pointer'] and f['dsaved'] is not None
                        and f['dsaved'] < 5),
                       ('data locals, all', lambda f: not f['pointer']),
                       ('pointer locals, all', lambda f: f['pointer'])):
        sub = [x for x in rows if sel(x[2])]
        nreg = sum(1 for x in sub if x[3] == 'register')
        print('%s: %d (%d in a register, %d in the frame)'
              % (title, len(sub), nreg, len(sub) - nreg))
        for name, hits, misses in score(sub):
            print('  %-50s %4d/%d' % (name, hits, len(sub)))
            if a.misses:
                for rn, ln, img in misses:
                    print('      miss: %s %s (image: %s)' % (rn, ln, img))
    return 0


def pressure_table(rows):
    """Data locals by the number of D2-D7 their routine saves and by
    whether every read pushes them: in a register / in the frame."""
    from collections import Counter
    c = Counter()
    for _r, _l, f, img in rows:
        if f['pointer'] or f['dsaved'] is None:
            continue
        c[(f['dsaved'], f['only_pushed'], img)] += 1
    print('data locals by D2-D7 saved: register/frame, for "not every read '
          'pushes it" | "every read pushes it"')
    for k in range(7):
        print('  %d saved: %3d/%-3d | %3d/%-3d' % (
            k, c[(k, False, 'register')], c[(k, False, 'frame')],
            c[(k, True, 'register')], c[(k, True, 'frame')]))


# --- one-statement bodies (bead domain-pascal-s4a5) ---------------------------

ONE_KINDS = {'IfStmt': 'if', 'SwitchStmt': 'case', 'ForStmt': 'for', 'WhileStmt': 'while',
             'DoStmt': 'repeat'}


def c_param_bytes(ty):
    """The size of a C parameter type TY (a clang type dict) as the image
    passes it by value: 1, 2 or 4; None for a type this survey does not
    lay out (records, arrays, C_RECORDS)."""
    q = re.sub(r'\b(const|volatile)\b', '', ty.get('qualType', '')).strip()
    if q in C_RECORDS:
        return None
    t = re.sub(r'\b(const|volatile)\b', '',
               (ty.get('desugaredQualType') or ty.get('qualType') or '')).strip()
    t = re.sub(r'\s+', ' ', t)
    if t.endswith('*'):
        return 4
    if '[' in t or '(' in t or t.startswith(('struct', 'union')):
        return None
    if t in ('char', 'signed char', 'unsigned char', '_Bool', 'bool', 'int8_t', 'uint8_t'):
        return 1
    if t in ('short', 'unsigned short', 'short int', 'unsigned short int', 'int16_t',
             'uint16_t'):
        return 2
    if t in ('int', 'unsigned int', 'unsigned', 'long', 'unsigned long', 'long int',
             'unsigned long int', 'int32_t', 'uint32_t'):
        return 4
    return None


def param_slots(sizes):
    """The (d,A6) offsets of parameters of SIZES (bytes each, pushed right
    to left: the first at 8; a byte or a word takes a 2-byte slot, the
    byte in its high half, so at the slot's own offset; codegen-patterns
    1.1); None where a size is None (and for every later parameter)."""
    out, off = [], 8
    for n in sizes:
        if n is None or off is None:
            out.append(None)
            off = None
            continue
        out.append(off)
        off += 2 if n <= 2 else 4
    return out


def _stmts(compound):
    """The statements of a C compound statement as Pascal counts them: a
    declaration without an initialiser, a null statement and a final
    `return;' are not statements."""
    out = []
    for x in compound.get('inner') or []:
        k = x.get('kind')
        if k == 'NullStmt':
            continue
        if k == 'DeclStmt' and not any(d.get('init') for d in x.get('inner') or []):
            continue
        out.append(x)
    if out and out[-1].get('kind') == 'ReturnStmt' and not out[-1].get('inner'):
        out.pop()
    return out


def _cond_children(st):
    """Indices of ST's children that are its controlling expression."""
    k = st.get('kind')
    inner = st.get('inner') or []
    if k == 'DoStmt':
        return {len(inner) - 1}
    if k == 'ForStmt':
        return {0, 1, 2, 3}  # init, condition variable, condition, increment
    if k == 'IfStmt':
        n = (1 if st.get('hasInit') else 0) + (1 if st.get('hasVar') else 0)
        return set(range(n + 1))
    return {0}


def param_reads(st, pids):
    """The reads of the parameters PIDS (clang ids) in statement ST:
    [(id, where, how)], where 'cond' (in the controlling expression of ST)
    or 'body', how 'addr' (`&p'), 'arg' (a whole call argument) or
    'value'; and whether ST has a call."""
    reads, calls = [], [False]

    def walk(n, where, parent):
        if not isinstance(n, dict):
            return
        k = n.get('kind')
        if k == 'CallExpr':
            calls[0] = True
        if k == 'DeclRefExpr':
            d = n.get('referencedDecl') or {}
            if d.get('id') in pids:
                how = 'value'
                if parent and parent.get('kind') == 'UnaryOperator' and parent.get('opcode') == '&':
                    how = 'addr'
                elif parent and parent.get('kind') == 'CallExpr':
                    how = 'arg'
                reads.append((d['id'], where, how))
            return
        for x in n.get('inner') or []:
            # the casts and parentheses around a reference keep its parent
            p = parent if k in ('ImplicitCastExpr', 'ParenExpr') else n
            walk(x, where, p)

    conds = _cond_children(st)
    for i, x in enumerate(st.get('inner') or []):
        walk(x, 'cond' if i in conds else 'body', st)
    return reads, calls[0]


def c_one_statement(ast, text):
    """The functions of the clang JSON AST (TEXT the main file) whose body
    is one `if', `switch', loop statement: [{name, kind, addr (from the
    nearest `Original address' header before it, else None), params:
    [{name, bytes, slot, reads: [(where, how)]}], call}]."""
    out = []
    prev_end = 0
    for fd in ast.get('inner') or []:
        if fd.get('kind') != 'FunctionDecl':
            continue
        rng = fd.get('range', {})
        body = [x for x in fd.get('inner') or [] if x.get('kind') == 'CompoundStmt']
        name = fd.get('name', '')
        off = fd.get('loc', {}).get('offset')
        if not body or off is None or text[off:off + len(name)] != name:
            continue
        hdr = re.findall(r'Original address:?\s*0x([0-9a-fA-F]+)', text[prev_end:off])
        prev_end = rng.get('end', {}).get('offset', off)
        st = _stmts(body[0])
        if len(st) != 1 or st[0].get('kind') not in ONE_KINDS:
            continue
        parms = [x for x in fd.get('inner') or [] if x.get('kind') == 'ParmVarDecl']
        sizes = [c_param_bytes(p.get('type', {})) for p in parms]
        slots = param_slots(sizes)
        pids = {p['id']: i for i, p in enumerate(parms)}
        reads, call = param_reads(st[0], pids)
        params = []
        for i, p in enumerate(parms):
            t = (p.get('type', {}).get('desugaredQualType')
                 or p.get('type', {}).get('qualType', ''))
            params.append({'name': p.get('name', '?'), 'bytes': sizes[i], 'slot': slots[i],
                           'pointer': t.strip().endswith('*'),
                           'reads': [(w, h) for pid, w, h in reads if pids[pid] == i]})
        out.append({'name': name, 'kind': ONE_KINDS[st[0]['kind']],
                    'addr': int(hdr[-1], 16) if hdr else None, 'params': params,
                    'call': call})
    return out


_CT = re.compile(r'^(b(?:ra|hi|ls|cc|cs|ne|eq|vc|vs|pl|mi|ge|lt|gt|le|sr)[sw]?|db\w+|jmp|jsr|bsr\w*|rts)\b')


def slot_refs(insns, slot, nbytes):
    """How INSNS (a routine, [(addr, len, text)]) reach a by-value
    parameter of NBYTES at (SLOT,A6): [(index, before, kind, insn text)],
    `before' whether no branch, call or return comes earlier, kind 'load'
    (a move into a register), 'push' (a move to -(SP) or `pea'), 'inplace'
    (another instruction with the slot as an operand)."""
    out = []
    first_ct = next((i for i, (_, _, t) in enumerate(insns) if _CT.match(t)), len(insns))
    for i, (_, _, t) in enumerate(insns):
        hit = False
        for m in _FP.finditer(t):
            d = int(m.group(1))
            if slot <= d < slot + (2 if nbytes <= 2 else 4):
                hit = True
        if not hit:
            continue
        mn, _, ops = t.partition(' ')
        if mn.startswith('pea') or ops.endswith('%sp@-'):
            kind = 'push'
        elif mn.startswith('move') and re.search(r',%[ad][0-7]$', ops) and ops.startswith('%fp@'):
            kind = 'load'
        else:
            kind = 'inplace'
        out.append((i, i < first_ct, kind, t))
    return out


def classify_param(refs):
    """One by-value parameter's image class from slot_refs: 'entry' (its
    first reference loads it into a register before the first branch or
    call), 'frame' (it is tested or used in place, `cmpi #k,(d,A6)'),
    'pushed', 'late' (loaded after a branch or call), 'none'."""
    if not refs:
        return 'none'
    _, before, kind, _ = refs[0]
    if kind == 'load':
        return 'entry' if before else 'late'
    return 'frame' if kind == 'inplace' else 'pushed'


def cmd_onestmt(a):
    import concurrent.futures
    import a5survey
    import regalloc
    import srcsurvey
    work = os.path.abspath(a.work)
    tree, commit = srcsurvey.export_rev(os.path.abspath(a.source), a.rev, work)
    print('localsurvey onestmt: %s at %s (%s)' % (os.path.normpath(a.source), a.rev, commit[:12]))
    m, rs = regalloc.routines()
    ends = {r0: r1 for _seg, r0, r1 in rs}
    files = a.files or sorted(
        os.path.relpath(os.path.join(d, f), tree)
        for d, _s, fs in os.walk(tree) for f in fs
        if f.endswith('.c') and '/test' not in os.path.relpath(d, tree) + '/'
        and not os.path.relpath(d, tree).startswith('test'))

    def ast_of(path):
        with open(os.path.join(tree, path), errors='replace') as fh:
            text = fh.read()
        if not re.search(r'\b(if|switch|for|while|do)\b', text):
            return path, text, None
        res = subprocess.run(['clang', '-std=gnu2x', '-DARCH_HOST', '-I.',
                              '-I' + os.path.dirname(path), '-fsyntax-only', '-w', '-Xclang',
                              '-ast-dump=json', path],
                             cwd=tree, capture_output=True, text=True)
        try:
            return path, text, json.loads(res.stdout) if res.stdout else None
        except ValueError:
            return path, text, None

    rows = []
    seen = set()
    with concurrent.futures.ThreadPoolExecutor(8) as ex:
        for path, text, ast in ex.map(ast_of, files):
            if ast is None:
                continue
            for f in c_one_statement(ast, text):
                addr = f['addr']
                if addr is None:
                    s = m.lookup(f['name'].upper())
                    addr = s.addr if s else None
                if addr is None or addr not in ends or addr in seen:
                    continue
                seen.add(addr)
                insns = a5survey.reached(addr, ends[addr])
                vals = [p for p in f['params']
                        if p['slot'] is not None and not p['pointer'] and p['reads']
                        and any(h == 'value' for _w, h in p['reads'])]
                if not vals:
                    continue
                for p in vals:
                    refs = slot_refs(insns, p['slot'], p['bytes'])
                    p['class'] = classify_param(refs)
                    p['refs'] = len(refs)
                    p['first'] = refs[0][3] if refs else ''
                rows.append((path, f, addr, insns))
    from collections import Counter
    by = Counter()
    for path, f, addr, insns in rows:
        for p in [p for p in f['params'] if 'class' in p]:
            where = ('cond' if all(w == 'cond' for w, _h in p['reads'])
                     else 'body' if all(w == 'body' for w, _h in p['reads']) else 'both')
            nr = len(p['reads'])
            key = (f['kind'], where, 'call' if f['call'] else 'nocall', p['class'])
            by[key] += 1
            print('%-8s %-40s %-28s %-6s %-7s %-6s reads=%d refs=%d %-6s %s' % (
                '%06X' % addr, path, f['name'][:28], f['kind'], where,
                'call' if f['call'] else '-', nr, p['refs'], p['class'], p['first']))
    print('== by statement, where read, call, image class')
    for k, n in sorted(by.items()):
        print('  %-8s %-5s %-7s %-7s %d' % (k + (n,)))
    tot = Counter(k[3] for k in by.elements())
    print('== totals: %s' % ', '.join('%s %d' % kv for kv in sorted(tot.items())))
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    sub = ap.add_subparsers(dest='cmd', required=True)
    r = sub.add_parser('run')
    r.add_argument('--rev', default='HEAD')
    r.add_argument('--source', default=os.path.join(ROOT, '..', 'source'))
    r.add_argument('--work', default=os.path.join(ROOT, 'tmp', 'localsurvey'))
    r.add_argument('--dpc', default=os.path.join(ROOT, 'target', 'debug', 'dpc'))
    r.add_argument('--module', default='tty/tty.pas')
    r.add_argument('-v', '--verbose', action='store_true')
    c = sub.add_parser('c')
    c.add_argument('--rev', default='HEAD')
    c.add_argument('--source', default=os.path.join(ROOT, '..', 'source'))
    c.add_argument('--work', default=os.path.join(ROOT, 'tmp', 'localsurvey'))
    c.add_argument('-v', '--verbose', action='store_true')
    c.add_argument('--misses', action='store_true')
    c.add_argument('--dump', help='write the rows as JSON to this file')
    c.add_argument('files', nargs='*')
    o = sub.add_parser('onestmt')
    o.add_argument('--rev', default='HEAD')
    o.add_argument('--source', default=os.path.join(ROOT, '..', 'source'))
    o.add_argument('--work', default=os.path.join(ROOT, 'tmp', 'localsurvey'))
    o.add_argument('files', nargs='*')
    a = ap.parse_args(argv)
    return {'run': cmd_run, 'c': cmd_c, 'onestmt': cmd_onestmt}[a.cmd](a)


if __name__ == '__main__':
    sys.exit(main())
