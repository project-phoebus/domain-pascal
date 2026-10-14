#!/usr/bin/env python3
"""srcsurvey.py - compile ../source's Domain Pascal with dpc, summarise
the diagnostics by kind, and check the routines that compile against the
image (bead domain-pascal-xdh.3).

    srcsurvey.py run [--rev REV] [--source DIR] [--work DIR] [--dpc DPC]
                     [--no-workaround NAME]... [--no-conform]
                     [--no-build-settings] [--baseline [FILE] | --update]
                     [-v] [MODULE.pas ...]
    srcsurvey.py summarise DIAGNOSTICS.txt [--tree DIR] [-v]

`run' exports ../source's domain_os/ at REV (default HEAD) with `git
archive' into WORK/export (default tmp/srcsurvey; ../source itself is only
read) and works on that copy.  For each module file (default: every
`.pas' under domain_os/ whose first word after comments is `module';
paths relative to domain_os/), compiled with `dpc -I .' from domain_os/:

  0. Preparation.  Only the *incompleteness* handling of ../source's
     tools/pascal_check.sh is mirrored, never its workarounds for dpc
     gaps: an `%include' of a fragment that does not exist yet is dropped
     (with a comment it opens on the line), and a `forward' heading after
     a line `{ body: FRAGMENT }' becomes `extern' when FRAGMENT is missing.
  1. Stage "raw": `dpc --check-only' on the prepared copy, as is.
  2. Workarounds for refusals in declarations, which stop dpc before it
     looks at any routine (each one named, see WORKAROUNDS; --no-workaround
     NAME skips one).  Only declarations are rewritten, never text in a
     comment, and none changes a size or an offset the surviving routines
     use, so a routine reported OK compiles to the image's bytes.  One
     is needed at present, `packed-init' (KBD); it changes a data
     block's contents, so a routine whose code matches is reported
     CODE-OK, not OK, when only data differs.
  3. Stage "check" and "codegen", repeated: `dpc --check-only', then full
     code generation.  Every routine that a diagnostic falls in is
     *stubbed* (its body removed: a global routine is deleted, as its
     `extern' heading is in an insert file, and dropped from the `define'
     list; an `internal' one becomes an `internal; extern' heading, or,
     when the module file has its `forward' heading, that heading becomes
     `extern') and
     the stage is run again, until the module compiles.  So each routine
     is charged with the diagnostics of the first stage that rejects it
     (dpc reports the first refusal of a statement, not every one).
     Stubbed internal routines are named with `--module-routine', so their
     callers still compile.  dpc takes that with a fragment origin: the
     lowest routine address of the module, or, for a module with several
     code sections (TTY: WIRED_PROC, OS_INIT_PROC, .TEXT), one origin per
     section that still has a routine (`--fragment-origin=SECTION=0xADDR')
     and each stubbed routine named with its section
     (`--module-routine=SECTION:NAME=0xADDR'; SECTION from the map).
     A stubbed routine's `static' and `define' variables (its nested
     routines' too) stay, hoisted to unit level where its body was, in
     declaration order (`var (DATA_SECTION)' for a routine of a named
     data section, the static names prefixed with the routine's and its
     enclosing routines'; `define' names kept, they are exported): dpc
     allocates them in the same order, so every data area keeps the
     image's layout and offsets (TTY .DATA 0xE8242C and WIRED_DATA
     0xE2DDB4 byte-identical, tty_$i_set_dfl_funcs' `(0x24,A5)').  A
     static of a data section no compiled routine has is left out (dpc
     would take it for an overlay section).  So are a routine's statics
     when dpc rejects them at unit level (a type or constant declared in
     the routine, a refused declaration): said, and the data after them
     in their section moves.
  4. Conformance (unless --no-conform): every top-level routine that
     compiled, with its nested routines, is compiled alone (all the other
     routines stubbed, `dpc --fragment-origin' and `--module-routine' for
     the module routines it calls, in any of the module's code sections:
     `bsr' within its own, `jsr abs.l' from another) and compared with the
     image by tools/conform.py.  A routine's address and size come from its header
     comment, "Original address: 0xADDR, N bytes" (../source's fragment
     format); nested routines lie between the previous top-level routine's
     end and the routine's address.  Unless --no-build-settings, each is
     given the A5 mode and loop alignment it has in the image (step 5:
     `--a5-always', `--a5-always-from', `--a5-absolute' for the routine,
     `--loop-align=off' or `--loop-align-off-from').
  5. Build settings: for each code section of the module (each map code
     segment holding one of its routines), the A5 mode from
     tools/a5survey.py (`switch': always mode from the first exported
     routine that loads A5 without using it, when no exported routine
     after it goes without; else a per-routine `--a5-always' list; the
     module-local routines that load A5 listed too; exported routines
     that address their block absolutely as `--a5-absolute') and the
     loop-alignment switch from tools/loopalign.py (`switch': off from
     the first routine with a loop head at 2 mod 4 and no filler; or from
     the last routine when the segment ends in a zero word), as the dpc
     options a build of the whole module needs (codegen-patterns 4, 7),
     printed at the end ("build settings").  The `-from' routine is the
     first one the image proves; the interval printed with it is where
     the switch could be put as well.

The run ends with a summary of the top-level routines by status (OK,
CODE-OK, MISMATCH, SETUP, STUB: stubbed for a diagnostic, BLOCKED: the
module stops outside any routine).  `--update' writes them to the
baseline, tests/srcsurvey/baseline.txt, with the ../source commit;
`--baseline' compares with it, at that commit unless --rev is given, so
that only dpc changes the result (`make srcsurvey'), and exits 1 when a
routine's status is worse than the baseline's or the routine is gone.

Diagnostics are grouped by kind, most frequent first, with the routines
each kind occurs in.  A kind is the message with identifiers in backquotes
(upper case or `$'), quoted strings and numbers replaced by placeholders.  Refusals ("not
supported yet") are listed apart from other errors and from warnings.
"""

import argparse
import collections
import os
import re
import shlex
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import a5survey  # noqa: E402
import loopalign  # noqa: E402

DIAG_RE = re.compile(r'^(?P<file>[^\s:][^:]*):(?P<line>\d+):(?P<col>\d+): '
                     r'(?P<sev>error|warning): (?P<msg>.*)$')
ADDR_RE = re.compile(r'Original address:\s*0x([0-9A-Fa-f]+),\s*(\d+)\s*bytes')

# Workarounds for refusals in declarations (stage 2).  Each: (name, what
# dpc refuses, files it applies to, pattern, replacement, a string or a
# function of the match).  They are the same rewrites as ../source's
# tools/pascal_check.sh gaps.  Gap 5, `packed record' (status_$t, uid_$t
# in base/base.ins.pas), is no longer one: dpc lays packed records out by
# ref 3.10.7 (bead domain-pascal-xlm), and the `packed-overlay' and
# `packed-record' rewrites that reduced them to unpacked records were
# removed.  Nor is `field-aligned', which dropped `[aligned(n)]' from
# record fields (tty.ins.pas `timer: [aligned(1)] linteger'): dpc lays
# alignment attributes out by ref 3.15.6 (bead domain-pascal-l9p).
#
# `packed-init': dpc refuses an initial value for a packed record field
# that is not whole bytes (KBD's kbd_$state_table, an array of
# kbd_$state_entry_t, whose action and next_state are 4-bit fields).
# The initialiser is dropped: the array keeps its size and offset, only
# the data block's contents change, which conformance does not compare.
WORKAROUNDS = [
    ('packed-init',
     'initialiser of an array of kbd_$state_entry_t (packed, 4-bit fields) dropped',
     '.pas',
     re.compile(r'(\barray\s*\[[^\]]*\]\s*of\s+kbd_\$state_entry_t)\s*:=\s*'
                r'\[(?:[^\[\]]|\[[^\[\]]*\])*\]', re.I),
     r'\1'),
]
# workarounds that change only the contents of a data block, with the
# image range [lo, hi) of the bytes each one changes: a routine whose
# code matches and whose data differs from the image only in those
# bytes is reported `CODE-OK' (not `OK') when one of them was applied.
# packed-init: kbd_$state_table, 10 entries of 2 bytes (key, then the
# 4-bit action and next_state), at +0xC0 of KBD's block (kbd.pas), the
# block's last 0x14 bytes (map `D E2DDE4 KBD size = D4', end 0xE2DEB8).
DATA_WORKAROUNDS = {'packed-init': [(0xE2DEA4, 0xE2DEB8)]}

DATA_DIFF_RE = re.compile(r'^  section \S+ differs first at ')


def code_ok_only(lines):
    """Whether conform.check's failure LINES report data-section
    differences and nothing else (the problem lines, before the
    side-by-side listing)."""
    probs = []
    for ln in lines[1:]:
        if not ln.startswith('  ') or ln.startswith('   '):
            break
        probs.append(ln)
    return bool(probs) and all(DATA_DIFF_RE.match(p) for p in probs)


def data_diffs(asm, sect, work):
    """Image addresses of every byte in which the data sections of ASM
    that have contents and an address in SECT ({section: address},
    data_sections) differ from the image; conform.check reports only the
    first difference of each."""
    import conform
    obj = os.path.join(work, 'data_diffs.o')
    conform.run([conform.AS, '-m68010', '-o', obj, os.path.abspath(asm)])
    out = []
    for s, _n, fl in conform.sections_of(obj):
        if s not in sect or 'CONTENTS' not in fl or 'CODE' in fl:
            continue
        b = os.path.join(work, 'data_diffs.bin')
        conform.run([conform.OBJCOPY, '-O', 'binary', '-j', s, obj, b])
        with open(b, 'rb') as fh:
            ours = fh.read()
        img = conform.read_image(sect[s], len(ours))
        out += [sect[s] + i for i, (x, y) in enumerate(zip(ours, img)) if x != y]
    return out


def within(addrs, names):
    """Whether every address of ADDRS lies in a range of DATA_WORKAROUNDS
    NAMES."""
    ranges = [rg for n in names for rg in DATA_WORKAROUNDS[n]]
    return all(any(lo <= a < hi for lo, hi in ranges) for a in addrs)


CODE_SKIP_RE = re.compile(r"""\{[^}]*\}|\(\*.*?\*\)|"[^"\n]*"|'(?:[^'\n]|'')*'""", re.S)


def apply_workaround(pat, rep, text):
    """PAT -> REP in TEXT, only at matches that start in code (not in a
    comment, ref 2.1.4, or a string), so that the place count is the
    declarations rewritten.  -> (new text, places)."""
    skip = [m.span() for m in CODE_SKIP_RE.finditer(text)]
    out, pos, n = [], 0, 0
    i = 0
    while True:
        m = pat.search(text, i)
        if not m:
            break
        if any(a <= m.start() < b for a, b in skip):
            i = m.start() + 1
            continue
        out.append(text[pos:m.start()])
        out.append(m.expand(rep) if isinstance(rep, str) else rep(m))
        pos = i = max(m.end(), m.start() + 1)
        n += 1
    out.append(text[pos:])
    return ''.join(out), n


# ---------------------------------------------------------------- export

def export_rev(source, rev, work):
    """`git archive' of SOURCE's domain_os/ at REV into WORK/export ->
    (the exported domain_os directory, the full commit id)."""
    commit = subprocess.run(['git', '-C', source, 'rev-parse', '--verify', rev + '^{commit}'],
                            check=True, capture_output=True, text=True).stdout.strip()
    exp = os.path.join(work, 'export')
    shutil.rmtree(exp, ignore_errors=True)
    os.makedirs(exp)
    arch = subprocess.run(['git', '-C', source, 'archive', '--format=tar', commit, 'domain_os'],
                          check=True, capture_output=True)
    subprocess.run(['tar', '-xf', '-', '-C', exp], input=arch.stdout, check=True)
    return os.path.join(exp, 'domain_os'), commit


def prepare_text(text, exists):
    """Apply the incompleteness rewrites to a module file's TEXT;
    EXISTS(path) says whether an include path exists.  -> (new text,
    dropped fragment paths)."""
    out, dropped = [], []
    skipping, fwd = False, False
    for line in text.splitlines(keepends=True):
        if fwd:
            if re.search(r'\bforward\b', line, re.I):
                line = re.sub(r'\bforward\b', 'extern', line, count=1, flags=re.I)
                fwd = False
            out.append(line)
            continue
        mb = re.match(r'\s*\{ body: (\S+) \}\s*$', line)
        if mb and not exists(mb.group(1)):
            fwd = True
            continue
        if skipping:
            if '}' in line:
                skipping = False
            continue
        mm = re.match(r"\s*%include\s+'([^']+)'", line, re.I)
        if mm and not exists(mm.group(1)):
            dropped.append(mm.group(1))
            skipping = '{' in line and '}' not in line
            continue
        out.append(line)
    return ''.join(out), dropped


def strip_comments(s):
    return re.sub(r'\{[^}]*\}|\(\*.*?\*\)', ' ', s, flags=re.S)


def find_modules(tree):
    """Module files under TREE (relative paths, sorted)."""
    mods = []
    for dirpath, _dirs, files in os.walk(tree):
        for f in files:
            if f.endswith('.pas') and not f.endswith('.ins.pas'):
                p = os.path.join(dirpath, f)
                with open(p, errors='replace') as fh:
                    if re.match(r'\s*module\b', strip_comments(fh.read()), re.I):
                        mods.append(os.path.relpath(p, tree))
    return sorted(mods)


def module_includes(text):
    """The `%include' paths of a module file's TEXT, in order."""
    return re.findall(r"(?m)^\s*%include\s+'([^']+)'", text)


# ------------------------------------------------------------- routines

TOKEN_RE = re.compile(r"""
    (?P<ws>\s+)
  | (?P<comment>\{[^}]*\}|\(\*.*?\*\)|"[^"\n]*")
  | (?P<string>'(?:[^'\n]|'')*')
  | (?P<word>[A-Za-z_$][\w$]*)
  | (?P<num>\d+\#[0-9A-Za-z]+|\d+)
  | (?P<op>:=|\.\.|<>|<=|>=|.)
""", re.S | re.X)

OPTION_WORDS = {'internal', 'extern', 'forward', 'val_param', 'nosave', 'noreturn',
                'abnormal', 'variable', 'd0_return', 'a0_return', 'c_param', 'options'}
DECL_WORDS = {'var', 'const', 'type', 'label', 'define'}


def tokenize(text):
    """-> [(kind, lowercased text, start, end)] without blanks/comments."""
    toks = []
    for m in TOKEN_RE.finditer(text):
        k = m.lastgroup
        if k in ('ws', 'comment'):
            continue
        toks.append((k, m.group().lower(), m.start(), m.end()))
    return toks


class Routine:
    def __init__(self, name):
        self.name = name
        self.start = None        # offset of the attribute list or the heading
        self.head = None         # offset of `procedure'/`function'
        self.head_end = None     # offset after the heading's options
        self.end = None          # offset after the body's `end;'
        self.body = None         # offset of the body's `begin'
        self.options = set()
        self.nested = []
        self.addr = None         # from the header comment
        self.size = None

    @property
    def internal(self):
        return 'internal' in self.options

    @property
    def has_body(self):
        return not self.options & {'extern', 'forward'}

    def __repr__(self):
        return 'Routine(%s)' % self.name


class _Parser:
    def __init__(self, text):
        self.text = text
        self.t = tokenize(text)
        self.i = 0

    def peek(self, k=0):
        j = self.i + k
        return self.t[j][1] if j < len(self.t) else ''

    def skip_balanced(self, open_, close):
        depth = 0
        while self.i < len(self.t):
            v = self.peek()
            self.i += 1
            if v == open_:
                depth += 1
            elif v == close:
                depth -= 1
                if depth == 0:
                    return

    def at_routine(self):
        """At `[attrs] procedure|function' (not a `^procedure' type)?"""
        v = self.peek()
        if v in ('procedure', 'function'):
            return self.i == 0 or self.t[self.i - 1][1] != '^'
        if v == '[':
            j = self.i
            while j < len(self.t) and self.t[j][1] != ']':
                j += 1
            return j + 1 < len(self.t) and self.t[j + 1][1] in ('procedure', 'function')
        return False

    def routine(self):
        start = self.t[self.i][2]
        if self.peek() == '[':
            self.skip_balanced('[', ']')
        head = self.t[self.i][2]
        self.i += 1                                  # procedure/function
        r = Routine(self.peek())
        r.start, r.head = start, head
        self.i += 1
        if self.peek() == '(':
            self.skip_balanced('(', ')')
        if self.peek() == ':':
            while self.i < len(self.t) and self.peek() != ';':
                self.i += 1
        self.i += 1                                  # ;
        while self.peek() in OPTION_WORDS:
            if self.peek() == 'options':
                self.i += 1
                j = self.i
                self.skip_balanced('(', ')')
                r.options |= {v for _k, v, _s, _e in self.t[j:self.i] if v not in '(),'}
            else:
                r.options.add(self.peek())
                self.i += 1
            if self.peek() == ';':
                self.i += 1
        r.head_end = self.t[self.i - 1][3]
        if not r.has_body:
            r.end = r.head_end
            return r
        while self.i < len(self.t):
            if self.at_routine():
                r.nested.append(self.routine())
            elif self.peek() == 'begin':
                r.body = self.t[self.i][2]
                self.block()
                if self.peek() == ';':
                    self.i += 1
                r.end = self.t[self.i - 1][3]
                return r
            else:
                self.decl_item()
        raise ValueError('%s: no body' % r.name)

    def decl_item(self):
        """Skip one token of a declaration part (records nest)."""
        if self.peek() == 'record':
            depth = 0
            while self.i < len(self.t):
                v = self.peek()
                self.i += 1
                if v in ('record', 'case'):
                    depth += v == 'record'
                elif v == 'end':
                    depth -= 1
                    if depth == 0:
                        return
        else:
            self.i += 1

    def block(self):
        depth = 0
        while self.i < len(self.t):
            v = self.peek()
            self.i += 1
            if v in ('begin', 'case', 'record'):
                depth += 1
            elif v == 'end':
                depth -= 1
                if depth == 0:
                    return

    def top(self):
        out = []
        while self.i < len(self.t):
            if self.at_routine():
                out.append(self.routine())
            else:
                self.i += 1
        return out


def parse_routines(text):
    """The top-level routines of a fragment's TEXT, each with its nested
    routines and the address and size of its header comment (the last
    "Original address: 0xADDR, N bytes" between the previous routine and
    its heading)."""
    rs = _Parser(text).top()
    prev = 0
    for r in rs:
        ms = list(ADDR_RE.finditer(text, prev, r.start))
        if ms:
            r.addr, r.size = int(ms[-1].group(1), 16), int(ms[-1].group(2))
        prev = r.end
    return rs


def routine_spans(text):
    """[(start line, end line, top-level name)] of TEXT's routines."""
    out = []
    for r in parse_routines(text):
        out.append((text.count('\n', 0, r.start) + 1, text.count('\n', 0, r.end) + 1, r.name))
    return out


SECTION_ATTR_RE = re.compile(r'\bsection\s*\(\s*([A-Za-z_$.][\w$.]*)\s*(?:,\s*([A-Za-z_$.][\w$.]*)\s*)?\)',
                             re.I)


def data_section_of(text, r, inherited=None):
    """The data section routine R of TEXT names in its attribute list
    `[section(code, data)]' (ref 5.7.1, p. 5-24), as written, or
    INHERITED (a nested routine's parent's; None: the module's)."""
    m = SECTION_ATTR_RE.search(strip_comments(text[r.start:r.head]))
    return m.group(2) if m and m.group(2) else inherited


class Static:
    """A `static' (or `define') variable item of a routine's `var' part:
    `NAMES: static TYPE [:= INIT];' (ref 7.2, p. 7-3), kept as text."""

    def __init__(self, routine, section, names, rest, kind):
        self.routine = routine     # the routine (nested one included) declaring it,
                                   # with its enclosing routines: `outer_inner'
        self.section = section     # its data section, None: the module's
        self.names = names         # the names as written
        self.rest = rest           # from the `:' to the `;', as written
        self.kind = kind           # 'static' or 'define'

    def __repr__(self):
        return 'Static(%s %s %s)' % (self.routine, self.names, self.section)


def routine_statics(text, r, section=None, outer=None):
    """The `static' and `define' variables of routine R of TEXT and of
    its nested routines, in declaration order (the order dpc allocates
    them in their data section, as the image's blocks are laid out:
    tty/tty.pas, "Data").  SECTION: the data section R inherits; OUTER:
    the name path of R's enclosing routines (`outer_inner')."""
    sect = data_section_of(text, r, section)
    path = '%s_%s' % (outer, r.name) if outer else r.name
    out = []
    if r.body is None:
        return out
    toks = [t for t in tokenize(text) if r.head_end <= t[2] < r.body]
    i = 0
    nested = list(r.nested)
    while i < len(toks):
        s = toks[i][2]
        n = next((x for x in nested if x.start <= s < x.end), None)
        if n is not None:
            out += routine_statics(text, n, sect, path)
            nested.remove(n)
            while i < len(toks) and toks[i][2] < n.end:
                i += 1
            continue
        if toks[i][1] != 'var':
            i += 1
            continue
        i += 1
        if i < len(toks) and toks[i][1] == '(':        # var (overlay): not hoisted
            while i < len(toks) and toks[i][1] != ')':
                i += 1
            i += 1
            continue
        # items: NAME {, NAME} : [alloc] TYPE [:= INIT] ;
        while (i + 1 < len(toks) and toks[i][0] == 'word'
               and toks[i][1] not in DECL_WORDS | {'begin', 'procedure', 'function'}
               and toks[i + 1][1] in (',', ':')):
            names = []
            while i < len(toks) and toks[i][1] != ':':
                if toks[i][0] == 'word':
                    names.append(text[toks[i][2]:toks[i][3]])
                i += 1
            colon = toks[i][2]
            kind = toks[i + 1][1] if i + 1 < len(toks) else ''
            depth = 0
            while i < len(toks):
                v = toks[i][1]
                if v in ('(', '[', 'record'):
                    depth += 1
                elif v in (')', ']', 'end'):
                    depth -= 1
                    if depth < 0:
                        # a `var' parameter of a procedure type in a
                        # `type' part (ref 3.13.3), not a `var' part
                        break
                elif v == ';' and depth == 0:
                    break
                i += 1
            if i >= len(toks) or depth < 0:
                break
            if kind in ('static', 'define'):
                out.append(Static(path, sect, names, text[colon:toks[i][3]], kind))
            i += 1
    return out


HOISTED_BEGIN = '{ srcsurvey: the static data of %s }'
HOISTED_BEGIN_RE = re.compile(r'^\{ srcsurvey: the static data of (\S+) \}')
HOISTED_END = '{ srcsurvey: end of the static data }'


def hoisted_text(statics, keep_sections=None, owner=None):
    """Unit-level `var' parts for STATICS of a stubbed routine, so that
    the data sections keep their layout (each variable at its offset):
    `var (SECTION)' for a routine's named data section (dpc puts a
    unit-level `var (name)' variable of a section a routine of the unit
    names in that section, codegen-patterns 11), a plain `var' for the
    module's.  A static of a named section that is not in KEEP_SECTIONS
    (lower case; None: all) is left out: no routine compiled has that
    data section, and dpc would take the name for an overlay section
    (ref 3.14, refused).  OWNER: the stubbed top-level routine, named
    in the comments that bracket the parts (hoisted_owner; None: no
    comments).  The names of `static' variables (local
    symbols) are prefixed with their routine's (and its enclosing
    routines'), so that they cannot clash with the unit's other names;
    `define' ones (exported) keep theirs."""
    out = []
    for st in statics:
        if st.section is not None and keep_sections is not None \
                and st.section.lower() not in keep_sections:
            continue
        names = st.names if st.kind == 'define' else [
            '%s_%s' % (st.routine, n) for n in st.names]
        out.append('\nvar%s\n  %s%s\n' % (' (%s)' % st.section if st.section else '',
                                           ', '.join(names), st.rest))
    if not out or owner is None:
        return ''.join(out)
    return '\n' + HOISTED_BEGIN % owner + ''.join(out) + HOISTED_END + '\n'


def hoisted_owner(text, line):
    """The routine whose statics hoisted_text put on 1-based LINE of
    TEXT (between its HOISTED_BEGIN and HOISTED_END), or None."""
    lines = text.splitlines()
    for k in range(min(line, len(lines)) - 1, -1, -1):
        if lines[k].startswith(HOISTED_END):
            return None
        m = HOISTED_BEGIN_RE.match(lines[k])
        if m:
            return m.group(1)
    return None


def stub_text(text, names, forwards=(), keep_sections=None, no_hoist=()):
    """TEXT with the top-level routines NAMES stubbed: a global one
    deleted, an internal one reduced to `<heading> extern;' (without its
    attribute list), or deleted too when it is in FORWARDS (its `forward'
    heading elsewhere becomes the `extern' one, see extern_forwards).
    A stubbed routine's static variables stay, hoisted to unit level in
    its place (hoisted_text; KEEP_SECTIONS: the data sections to keep),
    except those of the routines in NO_HOIST (dropped: their declarations
    do not compile at unit level, see survey_stage).
    -> (new text, the global names deleted)."""
    out, pos, deleted = [], 0, []
    for r in parse_routines(text):
        if r.name not in names or not r.has_body:
            continue
        out.append(text[pos:r.start])
        if r.internal and r.name not in forwards:
            out.append(text[r.head:r.head_end] + ' extern;')
        elif not r.internal:
            deleted.append(r.name)
        if r.name not in no_hoist:
            out.append(hoisted_text(routine_statics(text, r), keep_sections, r.name))
        pos = r.end
    out.append(text[pos:])
    return ''.join(out), deleted


def extern_forwards(text, names):
    """TEXT with the `forward' headings of NAMES made `extern' (the
    routine's body is stubbed: a second heading would redeclare it)."""
    out, pos = [], 0
    for r in parse_routines(text):
        if r.name in names and 'forward' in r.options:
            out.append(text[pos:r.head])
            out.append(re.sub(r'\bforward\b', 'extern', text[r.head:r.head_end],
                              count=1, flags=re.I))
            pos = r.head_end
    out.append(text[pos:])
    return ''.join(out)


def drop_defines(text, names):
    """Remove NAMES from the `define' part of a module's TEXT (one name
    per item, `define a; b;'); the word `define' goes when none is left."""
    m = re.search(r'(?im)^define\b', text)
    if not m or not names:
        return text
    lower = {n.lower() for n in names}
    i = m.end()
    items = []
    tok = re.compile(r'\s*(\{[^}]*\})|\s*([A-Za-z_$][\w$]*)\s*;')
    while True:
        t = tok.match(text, i)
        if not t:
            break
        items.append((t.start(), t.end(), t.group(2)))
        i = t.end()
    kept = [text[s:e] for s, e, n in items if n is None or n.lower() not in lower]
    if not any(n and n.lower() not in lower for _s, _e, n in items):
        return text[:m.start()] + ''.join(kept).lstrip('\n') + text[i:]
    return text[:m.end()] + ''.join(kept) + text[i:]


# ----------------------------------------------------------- diagnostics

def parse_diags(text):
    """dpc's stderr -> list of dicts (file, line, col, sev, msg)."""
    diags = []
    for line in text.splitlines():
        m = DIAG_RE.match(line)
        if m:
            d = m.groupdict()
            d['line'] = int(d['line'])
            d['col'] = int(d['col'])
            diags.append(d)
    return diags


def kind_of(msg):
    """Normalise a message to its kind."""
    # names: backquoted text with a capital or a `$' (dpc prints the
    # program's identifiers upper-cased); lower-case words such as
    # `aligned' or `in' name the construct and are kept
    k = re.sub(r"`[^`']*[A-Z$][^`']*'", "`X'", msg)
    k = re.sub(r'`[^`]*[A-Z$][^`]*`', '`X`', k)
    k = re.sub(r'"[^"]*"', '"X"', k)
    # numbers, but not reference citations (ref 3.15, p. 5-19, Table 5-1)
    k = re.sub(r'(?<![\w.$#-])(?<!ref )(?<!p\. )(?<!Table )(?:0x[0-9a-fA-F]+|\d+#[0-9a-zA-Z]+|\d+)'
               r'(?![\w.#-])', 'N', k)
    return k


def is_refusal(msg):
    return 'not supported yet' in msg


def hoisted_owner_at(tree, path, line):
    """hoisted_owner for LINE of file PATH of TREE."""
    try:
        with open(os.path.join(tree, path), errors='replace') as fh:
            return hoisted_owner(fh.read(), line)
    except OSError:
        return None


class Locator:
    """Diagnostic position -> top-level routine name (or the file name)."""

    def __init__(self, tree):
        self.tree = tree
        self.cache = {}

    def __call__(self, path, line):
        if path not in self.cache:
            try:
                with open(os.path.join(self.tree, path), errors='replace') as fh:
                    self.cache[path] = routine_spans(fh.read())
            except (OSError, ValueError):
                self.cache[path] = []
        for a, b, n in self.cache[path]:
            if a <= line <= b:
                return n
        return None


def classify(d):
    if d['sev'] == 'warning':
        return 'warning'
    return 'refusal' if is_refusal(d['msg']) else 'error'


def summarise(entries):
    """ENTRIES: [(diag, routine-or-None)] -> (refusals, errors, warnings),
    each [(kind, count, [(routine, n)], example diag)], most frequent
    first."""
    groups = {'refusal': {}, 'error': {}, 'warning': {}}
    for d, where in entries:
        g = groups[classify(d)].setdefault(kind_of(d['msg']),
                                           {'n': 0, 'at': collections.Counter(), 'ex': d})
        g['n'] += 1
        g['at'][where or os.path.basename(d['file'])] += 1
    res = []
    for cls in ('refusal', 'error', 'warning'):
        items = sorted(groups[cls].items(), key=lambda kv: (-kv[1]['n'], kv[0]))
        res.append([(k, g['n'], sorted(g['at'].items()), g['ex']) for k, g in items])
    return tuple(res)


def report(title, summary, out=sys.stdout, verbose=False):
    names = ('Refusals ("not supported yet")', 'Errors', 'Warnings')
    print('== %s' % title, file=out)
    for name, items in zip(names, summary):
        if not items:
            continue
        total = sum(n for _k, n, _r, _e in items)
        print('-- %s: %d diagnostic(s), %d kind(s)' % (name, total, len(items)), file=out)
        for k, n, at, ex in items:
            print('  %4d  %s' % (n, k), file=out)
            print('        in: %s' % ', '.join('%s(%d)' % x if x[1] > 1 else x[0] for x in at),
                  file=out)
            if verbose:
                print('        e.g. %s:%d:%d: %s' % (ex['file'], ex['line'], ex['col'], ex['msg']),
                      file=out)


# --------------------------------------------------------------- driver

def run_dpc(dpc, tree, src, check_only=False, out=None, opts=()):
    cmd = [dpc, '-I', '.'] + list(opts)
    if check_only:
        cmd.append('--check-only')
    if out:
        cmd += ['-o', out]
    cmd.append(src)
    p = subprocess.run(cmd, cwd=tree, capture_output=True, text=True)
    return p.returncode, p.stderr


class Module:
    """A module file of the exported tree and its fragments.  The texts
    are read once (`pristine', after the preparation); stubbing rewrites
    the tree's files from them, with every routine stubbed so far."""

    def __init__(self, tree, path):
        self.tree, self.path = tree, path
        with open(os.path.join(tree, path)) as fh:
            self.text = fh.read()
        self.fragments = []                 # include paths holding routine bodies
        self.frag_text = {}                 # include path -> its text
        # routines with a `forward' heading in the module file
        self.forwards = {r.name for r in parse_routines(self.text) if 'forward' in r.options}
        self.routines = {}                  # name -> (fragment, Routine)
        self.dsect = {}                     # name -> data section (lower case) or None
        self.stubbed = set()
        self.no_hoist = set()               # stubbed routines whose statics are dropped
        for inc in module_includes(self.text):
            if inc.endswith('.ins.pas'):
                continue
            with open(os.path.join(tree, inc)) as fh:
                self.frag_text[inc] = fh.read()
            rs = parse_routines(self.frag_text[inc])
            self.fragments.append(inc)
            for r in rs:
                self.routines[r.name] = (inc, r)
                d = data_section_of(self.frag_text[inc], r)
                self.dsect[r.name] = d.lower() if d else None

    def keep_sections(self, bodies):
        """The named data sections of the routines BODIES (lower case)."""
        return {self.dsect[n] for n in bodies if self.dsect.get(n)}

    def stubbed_texts(self, names):
        """{path: text} of the module file and its fragments with NAMES
        stubbed (stub_text; the statics of a stubbed routine are kept
        for the data sections of the routines left)."""
        bodies = {n for n, (_i, r) in self.routines.items() if r.has_body} - set(names)
        keep = self.keep_sections(bodies)
        out, deleted = {}, []
        for inc in self.fragments:
            out[inc], d = stub_text(self.frag_text[inc], names, self.forwards, keep,
                                    self.no_hoist)
            deleted += d
        out[self.path] = extern_forwards(drop_defines(self.text, deleted), names)
        return out, deleted

    def stub(self, names):
        """Stub NAMES too in the tree's files (the survey's copy)."""
        self.stubbed |= set(names)
        texts, _d = self.stubbed_texts(self.stubbed)
        for p, s in texts.items():
            with open(os.path.join(self.tree, p), 'w') as fh:
                fh.write(s)

    def unit_text(self, keep):
        """The module with every fragment inlined and every top-level
        routine except KEEP stubbed."""
        texts, deleted = self.stubbed_texts(set(self.routines) - {keep})
        frag = '|'.join(re.escape(f) for f in self.fragments) or '(?!)'
        return re.sub(r"(?m)^\s*%include\s+'(" + frag + r")'\s*;",
                      lambda m: texts[m.group(1)], texts[self.path])


def code_section(addr, linkmap):
    """The map's output code section at ADDR, or None (no map, or none)."""
    if linkmap is None:
        return None
    import conform
    try:
        return conform.code_section_of(addr, linkmap)
    except conform.SetupError:
        return None


def module_routine_opts(mod, stubbed, linkmap):
    """dpc options naming MOD's STUBBED internal routines as module
    routines, with the fragment origins dpc wants alongside (compile
    only: no byte is compared).  A module with one code section: the
    lowest routine address as the origin.  Several (by the map's output
    section of each routine's header address): one origin per section that
    still has a routine with a body, its lowest address, and each stubbed
    routine with its section."""
    opts = []
    sects = {n: code_section(r.addr, linkmap) for n, (_i, r) in mod.routines.items()
             if r.addr is not None}
    several = len(set(sects.values()) - {None}) > 1
    for n in stubbed:
        _inc, r = mod.routines[n]
        if r.internal and r.addr is not None:
            sect = sects[n] + ':' if several and sects[n] else ''
            opts.append('--module-routine=%s%s=0x%X' % (sect, n.upper(), r.addr))
    if not opts:
        return opts
    if not several:
        lo = min(r.addr for _i, r in mod.routines.values() if r.addr is not None)
        return ['--fragment-origin', '0x%X' % lo] + opts
    los = {}
    for n, (_i, r) in mod.routines.items():
        if n in stubbed or not r.has_body or sects.get(n) is None:
            continue
        los[sects[n]] = min(los.get(sects[n], r.addr), r.addr)
    return ['--fragment-origin=%s=0x%X' % kv for kv in sorted(los.items())] + opts


def survey_stage(dpc, mod, check_only, stubbed_before, log, linkmap=None):
    """Run one stage until the module passes it, stubbing routines.
    The internal routines stubbed so far (STUBBED_BEFORE and this stage's)
    are named to dpc with `--module-routine' at their header address, so
    that their callers still compile (module_routine_opts; LINKMAP gives
    each its code section).  -> ([(diag, routine)], stubbed names, ok)."""
    entries, stubbed = [], []
    fragment_ok = True
    for _round in range(50):
        opts = (module_routine_opts(mod, list(stubbed_before) + stubbed, linkmap)
                if fragment_ok else [])
        rc, err = run_dpc(dpc, mod.tree, mod.path, check_only=check_only,
                          out=None if check_only else os.devnull, opts=opts)
        log.write(err)
        if rc and opts and re.search(r'^dpc: error: --(fragment-origin|module-routine)', err,
                                     re.M):
            # dpc refused the fragment options (a section's origin cannot
            # be given, say): the stubbed internal routines' callers get
            # charged with their calls instead
            print('srcsurvey: %s: %s' % (mod.path, err.strip().splitlines()[-1]))
            print('srcsurvey: %s: so no --module-routine: the callers of stubbed internal '
                  'routines are stubbed too' % mod.path)
            fragment_ok = False
            continue
        diags = parse_diags(err)
        loc = Locator(mod.tree)
        names, unhoist = [], set()
        for d in diags:
            r = loc(d['file'], d['line'])
            if d['sev'] == 'error' and r is None:
                # in the statics hoisted from a stubbed routine (a type or
                # constant of the routine's, a refused declaration): drop
                # them, the data after them in their section moves
                h = hoisted_owner_at(mod.tree, d['file'], d['line'])
                if h and h not in mod.no_hoist:
                    unhoist.add(h)
                    continue
            entries.append((d, r))
            if d['sev'] == 'error' and r and r not in names and r not in stubbed:
                names.append(r)
        if rc == 0:
            return entries, stubbed, True
        if unhoist:
            print('srcsurvey: %s: the static data of %s does not compile at unit level; '
                  'left out (the data after it moves)' % (mod.path, ', '.join(sorted(unhoist))))
            mod.no_hoist |= unhoist
        if not names and not unhoist:
            return entries, stubbed, False
        mod.stub(set(names))
        stubbed += names
    return entries, stubbed, False


def unit_specs(mod, name, linkmap, table=None):
    """conform.py specs for top-level routine NAME with its nested
    routines, or None when its address is unknown."""
    inc, r = mod.routines[name]
    if r.addr is None:
        return None
    ends = [o.addr + o.size for _i, o in (table or mod.routines).values()
            if o.addr is not None and o.addr < r.addr]
    sym = linkmap.lookup(name)
    specs = []
    lo = max(ends) if r.nested and ends else r.addr
    if lo < r.addr:
        specs.append('0x%X:%X' % (lo, r.addr - lo))
    specs.append('%s:%X' % (sym.name if sym and sym.addr == r.addr else '0x%X' % r.addr,
                            r.size))
    return specs


def data_sections(asm_text, linkmap):
    """`--section-addr' pairs for the module data sections of an .s."""
    out = {}
    # dpc: `.bss.moddata.<UNIT>[.<SECT>]', or `.moddata.<UNIT>[.<SECT>]'
    # for a block with initial values (emit.rs, data_section_name).
    # <UNIT> is the module's name upper-cased, which is the map's
    # segment name (`module log_' is the map's LOG_, the file log.pas)
    for sec in re.findall(r'\.section\s+"?((?:\.bss)?\.moddata\.[^",\s]+)', asm_text):
        unit, dot, name = sec.split('.moddata.', 1)[1].partition('.')
        want = name if dot else '.DATA'
        for s in linkmap.sections:
            if s.kind == 'D' and s.name == want:
                for g in linkmap.segments:
                    if g.kind == 'D' and g.name == unit.upper() and s.contains(g.addr):
                        out[sec] = g.addr
    return out


def conform_routine(mod, name, dpc, linkmap, work, out, table=None, settings=None,
                    data_changed=()):
    """Compile top-level routine NAME alone and compare it with the
    image.  MOD's texts are the module's before any routine was stubbed;
    TABLE (default MOD.routines) gives the addresses of the routines NAME
    calls.  SETTINGS (module_settings) give the routine the A5 mode and
    loop alignment it has in the image (unit_build_opts).  DATA_CHANGED
    names the DATA_WORKAROUNDS applied to the module.  -> 'OK',
    'CODE-OK' (only data differs, all of it in DATA_CHANGED's ranges),
    'MISMATCH', or 'SETUP: ...'."""
    import conform
    table = table or mod.routines
    specs = unit_specs(mod, name, linkmap, table)
    if specs is None:
        return 'SETUP: no "Original address" in the header'
    unit = os.path.join(mod.tree, '_unit_%s.pas' % name.replace('$', '_'))
    with open(unit, 'w') as fh:
        fh.write(mod.unit_text(name))
    _inc, r = mod.routines[name]
    try:
        routines = conform.unit_routines(specs, linkmap)
        opts = conform.fragment_args(routines, linkmap)
        body = mod.frag_text[_inc][r.start:r.end]
        called = set(re.findall(r'[A-Za-z_$][\w$]*', strip_comments(body).lower()))
        items = []
        # the module: the code segments of the routine's segment's name,
        # in any output section (conform.check_in_module)
        mine = linkmap.segment_containing(r.addr)
        for other, (_i, o) in table.items():
            if other == name or other not in called or o.addr is None:
                continue
            sym = linkmap.lookup(other)
            seg = linkmap.segment_containing(o.addr)
            if seg is not mine and (seg is None or mine is None or seg.is_section
                                    or seg.kind != 'I' or seg.name != mine.name):
                continue
            items.append(sym.name if sym else '%s=0x%X' % (other, o.addr))
        mopts, local = conform.module_routine_args(
            ['module-routines=' + ','.join(items)] if items else [], routines, linkmap)
        asm = os.path.join(work, os.path.basename(unit)[:-4] + '.s')
        lo = routines[0].addr
        bopts = unit_build_opts(settings, r, lo) if settings else []
        rc, err = run_dpc(dpc, mod.tree, os.path.basename(unit), out=asm,
                          opts=opts + mopts + bopts)
        if rc:
            return 'SETUP: dpc failed alone: ' + (err.strip().splitlines() or ['?'])[0]
        with open(asm) as fh:
            sect = data_sections(fh.read(), linkmap)
        lines = []

        def keep(line):
            lines.append(line)
            if out:
                out(line)
        ok = conform.check(asm, specs, linkmap, sect, work=work, name=name,
                           local_syms=local, quiet=not out, out=keep)
        if ok:
            return 'OK'
        # code-only: no problem but data differences, and every
        # differing data byte one a workaround in DATA_CHANGED changed
        if data_changed and code_ok_only(lines):
            diffs = data_diffs(asm, sect, work)
            if diffs and within(diffs, data_changed):
                return 'CODE-OK'
        return 'MISMATCH'
    except conform.SetupError as e:
        return 'SETUP: %s' % e


# ------------------------------------------------------ build settings

def a5_settings(rows):
    """The A5 options for one code section (map segment) from
    tools/a5survey.py's classification (codegen-patterns 4).  ROWS:
    [(addr, name, letter, exported, abs_own)] in address order, LETTER
    from a5survey.letter, EXPORTED: map-named, NAME the routine's name
    (None if not known).  -> dict:

      always_from  (addr, name) of the routine from which the section is
                   in always mode (a5survey.switch: the first `s'
                   routine), when no `n' routine follows it; else None
      interval     (last n, first s) addresses: the switch may be put at
                   any exported routine after the first and up to the
                   second
      always       [(addr, name)] that load A5 by themselves: the module-
                   local routines that load it, and, when an `n' routine
                   follows the first `s' (the mode switches back: PKT,
                   XPD, FILE_), every exported `s' routine instead of
                   always_from
      absolute     [(addr, name)]: exported routines that load no A5 and
                   address their own block absolutely (`n' with an
                   absolute operand into it, REM_NAME_$REGISTER_SERVER)"""
    ex = [(a, l) for a, _n, l, e, _b in rows if e and l != '.']
    last, first, late = a5survey.switch(ex)
    names = {a: n for a, n, _l, _e, _b in rows}
    out = {'always_from': None, 'interval': (last, first), 'always': [], 'absolute': []}
    if first is not None and not late:
        out['always_from'] = (first, names[first])
    elif first is not None:
        out['always'] += [(a, names[a]) for a, _n, l, e, _b in rows if e and l == 's']
    out['always'] += [(a, names[a]) for a, _n, l, e, _b in rows if not e and l in 'sS']
    out['always'].sort()
    out['absolute'] = [(a, names[a]) for a, _n, l, e, b in rows if e and l == 'n' and b]
    return out


def loop_settings(pr, seg, names, end_aligns=None):
    """The loop-alignment option for code section SEG (a map segment)
    from tools/loopalign.py's classes (codegen-patterns 7).  PR:
    loopalign.per_routine's list; NAMES: {routine start: name or None};
    END_ALIGNS: whether the segment ends in the 0x2048 filler (True),
    in a zero word after `rts' (False: the last routine does not align),
    or not known (None).  -> dict:

      off_from   (addr, name) of the routine from which loops are not
                 aligned: the first routine with a head at 2 mod 4 and no
                 filler (loopalign.switch); with none, the last routine
                 when the segment ends in a zero word; else None (on)
      interval   (last aligning, first non-aligning) addresses
      error      loopalign.switch's complaint when the section aligns
                 again after a routine that does not (no option says it)"""
    try:
        last, first = loopalign.switch(pr, seg)
    except ValueError as e:
        return {'off_from': None, 'interval': (None, None), 'error': str(e)}
    starts = [r0 for sg, r0, _r1, _c in pr if sg is seg]
    out = {'off_from': None, 'interval': (last, first), 'error': None}
    if first is not None:
        out['off_from'] = (first, names.get(first))
    elif end_aligns is False and starts:
        if last is not None and starts[-1] == last:
            out['error'] = ('the last routine 0x%X aligns, but the segment ends in a zero word'
                            % last)
        else:
            out['off_from'] = (starts[-1], names.get(starts[-1]))
            out['interval'] = (last, starts[-1])
    return out


def segment_end_aligns(seg):
    """True when map segment SEG ends in the 0x2048 filler, False in a
    zero word after `rts' (loopalign.py `ends'), else None."""
    w = loopalign.word(seg.end - 2)
    if w == 0x2048:
        return True
    if w == 0 and loopalign.word(seg.end - 4) == 0x4E75:
        return False
    return None


_SURVEYS = {}


def park_d0_starts():
    """The image routines that hold a call's pointer result in D0 where a
    call follows (resultsurvey.park_d0_routines; `dpc --park-d0'),
    computed once."""
    if 'park' not in _SURVEYS:
        import nestsurvey
        import resultsurvey
        rs = nestsurvey.load()
        by_start, _ = nestsurvey.analyse(rs)
        _SURVEYS['park'] = resultsurvey.park_d0_routines(rs, by_start)
    return _SURVEYS['park']


def image_surveys():
    """(map, a5survey routines by start, loopalign per_routine list),
    computed once."""
    if 'map' not in _SURVEYS:
        m, _sites, _f, routines = loopalign.scan()
        _SURVEYS['pr'] = loopalign.per_routine(_sites, routines)
        rs, ah = a5survey.load()
        _SURVEYS['a5'] = a5survey.analyse(rs, ah, m)
        _SURVEYS['map'] = m
    return _SURVEYS['map'], _SURVEYS['a5'], _SURVEYS['pr']


def module_settings(mod, linkmap):
    """[(code section name, map segment, a5 dict, loop dict, park list)]
    for MOD's code sections (the map code segments holding its routines'
    header addresses), in address order.  Names are the source's (upper
    case) where it has a routine at the address, else the map's.  PARK:
    [(addr, name)], the source's routines of the section that the image
    compiles as `dpc --park-d0' does (park_d0_starts)."""
    m, by_start, pr = image_surveys()
    src = {r.addr: n.upper() for n, (_i, r) in mod.routines.items() if r.addr is not None}
    segs = {}
    for _n, (_i, r) in mod.routines.items():
        if r.addr is None:
            continue
        s = m.segment_containing(r.addr)
        if s is not None and s.kind == 'I' and not s.is_section:
            segs[s.addr] = s
    out = []
    for sa in sorted(segs):
        seg = segs[sa]
        rows = []
        for r in sorted((r for r in by_start.values() if r.segaddr == sa), key=lambda r: r.r0):
            nm = src.get(r.r0) or (r.name.upper() if r.name else None)
            rows.append((r.r0, nm, a5survey.letter(r), bool(r.name), r.abs_own))
        names = {r0: src.get(r0) or ((m.names_at(r0) or [None])[0])
                 for sg, r0, _r1, _c in pr if sg is seg}
        a5 = a5_settings(rows)
        # routines that load A5 and use it only in their `bsr' callees
        # (LOG_$READ 0xE17800 for its worker 0xE17778): used mode loads it
        # in a whole-module build, a fragment with the callee stubbed
        # needs it said (unit_build_opts)
        a5['for_callees'] = [(r.r0, src.get(r.r0) or (r.name.upper() if r.name else None))
                             for r in sorted(by_start.values(), key=lambda r: r.r0)
                             if r.segaddr == sa and r.has_link and r.sets
                             and not r.uses and r.uses_t]
        park = [(a, src[a]) for a in sorted(park_d0_starts()) if seg.contains(a) and a in src]
        out.append((code_section(sa, linkmap) or seg.name, seg, a5,
                    loop_settings(pr, seg, names, segment_end_aligns(seg)), park))
    return out


def _nm(an):
    a, n = an
    return n if n else 'module-local at 0x%X' % a


def settings_opts(settings):
    """The dpc options for MODULE_SETTINGS' list (comma lists, one
    `-from' routine per code section), and what no option can say (a
    routine without a name).  -> (options, problems)."""
    lists = {'--a5-always-from': [], '--a5-always': [], '--a5-absolute': [],
             '--loop-align-off-from': [], '--park-d0': []}
    problems = []

    def add(flag, sect, an):
        if an[1]:
            lists[flag].append(an[1])
        else:
            problems.append('%s: %s needs %s, which has no name' % (sect, flag, _nm(an)))
    for sect, _seg, a5, lp, park in settings:
        if a5['always_from']:
            add('--a5-always-from', sect, a5['always_from'])
        for an in a5['always']:
            add('--a5-always', sect, an)
        for an in a5['absolute']:
            add('--a5-absolute', sect, an)
        if lp['error']:
            problems.append('%s: loop alignment: %s' % (sect, lp['error']))
        elif lp['off_from']:
            add('--loop-align-off-from', sect, lp['off_from'])
        for an in park:
            add('--park-d0', sect, an)
    return ['%s=%s' % (f, ','.join(v)) for f, v in lists.items() if v], problems


def report_settings(path, settings, out=sys.stdout):
    """The `build settings' section for one module."""
    opts, problems = settings_opts(settings)
    print('== %s: build settings (A5 mode, codegen-patterns 4; loop alignment, 7; '
          'D0 parking, 3)' % path, file=out)

    def iv(lo, hi):
        return '(%s, %s]' % ('0x%X' % lo if lo is not None else 'start',
                             '0x%X' % hi if hi is not None else 'end')
    for sect, seg, a5, lp, park in settings:
        a = a5['always_from']
        print('  %-14s 0x%X..0x%X  A5: %s%s%s;  loops: %s' % (
            sect, seg.addr, seg.end,
            'always from %s, switch in %s' % (_nm(a), iv(*a5['interval'])) if a
            else 'used mode' if not a5['always'] or a5['interval'][1] is None
            else 'used mode, always-mode routines listed (the mode switches back)',
            (', always: ' + ', '.join(map(_nm, a5['always']))) if a5['always'] else '',
            (', absolute: ' + ', '.join(map(_nm, a5['absolute']))) if a5['absolute'] else '',
            ('not expressible: ' + lp['error']) if lp['error']
            else 'off from %s, switch in %s' % (_nm(lp['off_from']), iv(*lp['interval']))
            if lp['off_from'] else 'aligned (default)'), file=out)
        if park:
            print('  %-14s D0 parking where a call follows: %s'
                  % ('', ', '.join(map(_nm, park))), file=out)
    # quoted for a shell: the names have `$'
    print('  dpc %s' % (' '.join(map(shlex.quote, opts)) or '(no options)'), file=out)
    for p in problems:
        print('  not expressible: %s' % p, file=out)


def unit_build_opts(settings, r, lo):
    """dpc options that give top-level routine R (with nested routines
    from address LO) the A5 mode and loop alignment it has in a build of
    the whole module with SETTINGS' options.  A routine that loads A5
    only for its `bsr' callees (used mode, which the fragment cannot see
    with the callees stubbed) is given `--a5-always'.  A routine the
    image compiles as `--park-d0' does is given it."""
    name = r.name.upper()
    for _sect, seg, a5, lp, park in settings:
        if not seg.contains(r.addr):
            continue
        opts = []
        if any(n == name for _a, n in a5['always']):
            opts.append('--a5-always=' + name)
        elif any(n == name for _a, n in a5['absolute']):
            opts.append('--a5-absolute=' + name)
        elif a5['always_from'] and not r.internal and r.addr >= a5['always_from'][0]:
            opts.append('--a5-always-from=' + name)
        elif any(n == name for _a, n in a5.get('for_callees', ())):
            # loads A5 for module-local callees, stubbed in the fragment
            opts.append('--a5-always=' + name)
        off = lp['off_from']
        if off and not lp['error']:
            last = lp['interval'][0]
            if off[0] <= lo or (off[0] < r.addr and (last is None or last < lo)):
                # off from the first routine, or from a nested routine
                # (no name to give) with none before it that aligns
                opts.append('--loop-align=off')
            elif off[0] <= r.addr:
                opts.append('--loop-align-off-from=' + name)
        if any(n == name for _a, n in park):
            opts.append('--park-d0=' + name)
        return opts
    return []


# ------------------------------------------------------------ baseline

BASELINE = os.path.join(ROOT, 'tests', 'srcsurvey', 'baseline.txt')

# per top-level routine, best first: compiled and byte-identical; code
# identical, data changed by a DATA_WORKAROUNDS rewrite; compiled, not
# identical; conformance not set up; stubbed (a diagnostic in it); not
# reached (the module stops outside any routine); compiled, conformance
# not run (--no-conform)
STATUS_RANK = {'OK': 5, 'CODE-OK': 4, 'COMPILED': 3, 'MISMATCH': 3, 'SETUP': 2,
               'STUB': 1, 'BLOCKED': 0}


def format_baseline(commit, results):
    """The baseline file's text: the ../source commit, then one line per
    (module, routine, status), in RESULTS' order."""
    lines = ['# tools/srcsurvey.py baseline: status, module, routine (top level).',
             '# Regenerate with `python3 tools/srcsurvey.py run --update` and say why',
             '# in the commit.  `make srcsurvey` compares at the commit below.',
             '# source: %s' % commit]
    lines += ['%s\t%s\t%s' % (st, m, n) for m, n, st in results]
    return '\n'.join(lines) + '\n'


def read_baseline(path):
    """-> (commit or None, {(module, routine): status})."""
    commit, out = None, {}
    with open(path) as fh:
        for ln in fh:
            ln = ln.rstrip('\n')
            if ln.startswith('# source: '):
                commit = ln[len('# source: '):].strip()
            elif ln and not ln.startswith('#'):
                st, m, n = ln.split('\t')
                if st not in STATUS_RANK:
                    raise ValueError('%s: unknown status %r' % (path, st))
                out[(m, n)] = st
    return commit, out


def compare_baseline(base, results, modules=None):
    """Differences between BASE ({(module, routine): status}) and
    RESULTS [(module, routine, status)], restricted to MODULES when
    given.  -> (lines, regressions): a regression is a routine whose
    status ranks lower than in the baseline, or that is gone."""
    now = {(m, n): st for m, n, st in results}
    lines, regressions = [], 0
    for key in sorted(set(base) | set(now)):
        if modules is not None and key[0] not in modules:
            continue
        b, c = base.get(key), now.get(key)
        if b == c:
            continue
        worse = c is None or (b is not None and STATUS_RANK[c] < STATUS_RANK[b])
        regressions += worse
        lines.append('  %s %-12s %s: %s -> %s' % ('WORSE' if worse else 'diff ', key[0],
                                                  key[1], b or 'new', c or 'gone'))
    return lines, regressions


def routine_results(path, mod, ok, conformed):
    """[(module path, routine, status)] for MOD's top-level routines with
    bodies, in source order; CONFORMED: {name: conformance result}."""
    out = []
    for n, (_i, r) in mod.routines.items():
        if not r.has_body:
            continue
        if not ok:
            st = 'BLOCKED'
        elif n in mod.stubbed:
            st = 'STUB'
        else:
            st = conformed.get(n, 'COMPILED').split(':')[0]
        out.append((path, n, st))
    return out


def cmd_run(a):
    work = os.path.abspath(a.work)
    # dpc runs with the exported tree as its directory
    a.dpc = os.path.abspath(a.dpc)
    base_commit, base = None, None
    if a.baseline and not a.update:
        base_commit, base = read_baseline(a.baseline)
    if a.rev is None:
        # compared with a baseline: at its commit, so that only dpc moves
        a.rev = base_commit or 'HEAD'
    tree, commit = export_rev(os.path.abspath(a.source), a.rev, work)
    print('srcsurvey: %s at %s (%s)' % (os.path.normpath(a.source), a.rev, commit[:12]))
    mods = a.modules or find_modules(tree)
    settings_out = []
    results = []
    for k, path in enumerate(mods):
        if k:
            # a fresh copy: the previous module's workarounds and stubs
            # must not reach this one's raw stage
            tree, _c = export_rev(os.path.abspath(a.source), commit, work)
        p = os.path.join(tree, path)
        with open(p) as fh:
            text, dropped = prepare_text(fh.read(), lambda q: os.path.exists(os.path.join(tree, q)))
        with open(p, 'w') as fh:
            fh.write(text)
        print('srcsurvey: %s: %d missing fragment(s) left out%s' %
              (path, len(dropped), (': ' + ' '.join(dropped)) if dropped and a.verbose else ''))
        logp = os.path.join(work, path.replace('/', '_') + '.log')
        with open(logp, 'w') as log:
            data_changed = []
            rc, err = run_dpc(a.dpc, tree, path, check_only=True)
            log.write(err)
            loc = Locator(tree)
            report('%s: raw --check-only (exit %d)' % (path, rc),
                   summarise([(d, loc(d['file'], d['line'])) for d in parse_diags(err)]),
                   verbose=a.verbose)
            for name, what, suffix, pat, rep in WORKAROUNDS:
                if name in a.no_workaround:
                    continue
                n = 0
                for dirpath, _d, files in os.walk(tree):
                    for f in files:
                        if f.endswith(suffix):
                            q = os.path.join(dirpath, f)
                            with open(q) as fh:
                                s = fh.read()
                            s2, k = apply_workaround(pat, rep, s)
                            if k:
                                n += k
                                with open(q, 'w') as fh:
                                    fh.write(s2)
                print('srcsurvey: workaround %s: %s (%d place(s))' % (name, what, n))
                if n and name in DATA_WORKAROUNDS:
                    data_changed.append(name)
            mod = Module(tree, path)
            ok = True
            all_stubbed = []
            import sau2map
            linkmap = sau2map.load()
            for stage, co in (('check', True), ('codegen', False)):
                entries, stubbed, ok = survey_stage(a.dpc, mod, co, all_stubbed, log, linkmap)
                all_stubbed += stubbed
                report('%s: %s, routines stubbed: %s' % (path, stage, ', '.join(stubbed) or 'none'),
                       summarise(entries), verbose=a.verbose)
                if not ok:
                    print('srcsurvey: %s: %s stage fails outside any routine; stopping'
                          % (path, stage))
                    break
        print('srcsurvey: log %s' % os.path.relpath(logp, ROOT))
        settings = module_settings(mod, linkmap)
        settings_out.append((path, settings))
        conformed = {}
        if not ok or a.no_conform:
            results += routine_results(path, mod, ok, conformed)
            continue
        compiled = [n for n, (_i, r) in mod.routines.items()
                    if r.has_body and n not in mod.stubbed]
        print('== %s: conformance of the %d routine(s) that compile' % (path, len(compiled)))
        cwork = os.path.join(work, 'conform')
        os.makedirs(cwork, exist_ok=True)
        for name in compiled:
            res = conform_routine(mod, name, a.dpc, linkmap, cwork,
                                  print if a.verbose else None, mod.routines,
                                  None if a.no_build_settings else settings,
                                  data_changed)
            conformed[name] = res
            _i, r = mod.routines[name]
            print('  %-9s %-36s %s' % (res.split(':')[0], name,
                                       ('0x%X' % r.addr) if r.addr else '?')
                  + (('  ' + res[7:]) if res.startswith('SETUP') else '')
                  + (('  (data differs: workaround %s)' % ', '.join(data_changed))
                     if res == 'CODE-OK' else ''))
        results += routine_results(path, mod, ok, conformed)
    for path, settings in settings_out:
        report_settings(path, settings)
    counts = collections.Counter(st for _m, _n, st in results)
    print('== summary: %d routine(s): %s' % (
        len(results), ', '.join('%s %d' % (st, counts[st])
                                for st in sorted(counts, key=lambda x: -STATUS_RANK[x]))))
    if a.update:
        os.makedirs(os.path.dirname(a.baseline), exist_ok=True)
        with open(a.baseline, 'w') as fh:
            fh.write(format_baseline(commit, results))
        print('srcsurvey: baseline written to %s' % os.path.relpath(a.baseline, ROOT))
    elif base is not None:
        lines, worse = compare_baseline(base, results, set(mods) if a.modules else None)
        print('== baseline %s (source %s): %d difference(s), %d regression(s)'
              % (os.path.relpath(a.baseline, ROOT), (base_commit or '?')[:12],
                 len(lines), worse))
        for ln in lines:
            print(ln)
        if worse:
            return 1
    return 0


def cmd_summarise(a):
    with open(a.diagnostics) as fh:
        diags = parse_diags(fh.read())
    loc = Locator(a.tree)
    report(a.diagnostics, summarise([(d, loc(d['file'], d['line'])) for d in diags]),
           verbose=a.verbose)
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    sub = ap.add_subparsers(dest='cmd', required=True)
    r = sub.add_parser('run')
    r.add_argument('--rev', default=None,
                   help="../source revision (default HEAD, or the baseline's commit "
                        'with --baseline)')
    r.add_argument('--source', default=os.path.join(ROOT, '..', 'source'))
    r.add_argument('--work', default=os.path.join(ROOT, 'tmp', 'srcsurvey'))
    r.add_argument('--dpc', default=os.path.join(ROOT, 'target', 'debug', 'dpc'))
    r.add_argument('--no-workaround', action='append', default=[],
                   choices=[w[0] for w in WORKAROUNDS])
    r.add_argument('--no-conform', action='store_true')
    r.add_argument('--no-build-settings', action='store_true')
    r.add_argument('--baseline', nargs='?', const=BASELINE, default=None,
                   help='compare per-routine statuses with this baseline (default '
                        'tests/srcsurvey/baseline.txt); exit 1 on a regression')
    r.add_argument('--update', action='store_true',
                   help='write the baseline (--baseline FILE, or the default)')
    r.add_argument('-v', '--verbose', action='store_true')
    r.add_argument('modules', nargs='*')
    s = sub.add_parser('summarise')
    s.add_argument('diagnostics')
    s.add_argument('--tree', default='.')
    s.add_argument('-v', '--verbose', action='store_true')
    a = ap.parse_args(argv)
    if a.cmd == 'run' and a.update:
        if a.modules:
            ap.error('--update writes the whole baseline: give no MODULE')
        if a.no_conform:
            ap.error('--update records conformance results: not with --no-conform')
        a.baseline = a.baseline or BASELINE
    return cmd_run(a) if a.cmd == 'run' else cmd_summarise(a)


if __name__ == '__main__':
    sys.exit(main())
