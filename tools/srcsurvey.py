#!/usr/bin/env python3
"""srcsurvey.py - compile ../source's Domain Pascal with dpc, summarise
the diagnostics by kind, and check the routines that compile against the
image (bead domain-pascal-xdh.3).

    srcsurvey.py run [--rev REV] [--source DIR] [--work DIR] [--dpc DPC]
                     [--no-workaround NAME]... [--no-conform] [-v]
                     [MODULE.pas ...]
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
     use (status_$t keeps `all', 4 bytes at 0; uid_$t's two longwords and
     the word-aligned `timer' are where the packed or aligned declaration
     puts them), so a routine reported OK compiles to the image's bytes.
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
     Stubbed internal routines are named with `--module-routine'; dpc
     takes that only for a module with one code section, so in a module
     with more the callers of a stubbed internal routine are stubbed too.
  4. Conformance (unless --no-conform): every top-level routine that
     compiled, with its nested routines, is compiled alone (all the other
     routines stubbed, `dpc --fragment-origin' and `--module-routine' for
     the module routines it calls) and compared with the image by
     tools/conform.py.  A routine's address and size come from its header
     comment, "Original address: 0xADDR, N bytes" (../source's fragment
     format); nested routines lie between the previous top-level routine's
     end and the routine's address.

Diagnostics are grouped by kind, most frequent first, with the routines
each kind occurs in.  A kind is the message with identifiers in backquotes
(upper case or `$'), quoted strings and numbers replaced by placeholders.  Refusals ("not
supported yet") are listed apart from other errors and from warnings.
"""

import argparse
import collections
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)

DIAG_RE = re.compile(r'^(?P<file>[^\s:][^:]*):(?P<line>\d+):(?P<col>\d+): '
                     r'(?P<sev>error|warning): (?P<msg>.*)$')
ADDR_RE = re.compile(r'Original address:\s*0x([0-9A-Fa-f]+),\s*(\d+)\s*bytes')

# Workarounds for refusals in declarations (stage 2).  Each: (name, what
# dpc refuses, files it applies to, pattern, replacement).  They are the
# same rewrites as ../source's tools/pascal_check.sh gaps 5 and 6.
def _packed_overlay(m):
    """A packed variant record reduced to its overlay variants (one field
    each, e.g. status_$t's `2: (all: integer32)'), comments dropped."""
    body = strip_comments(m.group(3))
    keep = re.findall(r'[\w$]+(?:\s*,\s*[\w$]+)*\s*:\s*\(\s*[\w$]+\s*:\s*[\w$]+\s*\)', body)
    if not keep:
        return m.group(0)
    return '%srecord case %s of %s; end;' % (m.group(1), m.group(2), '; '.join(keep))


WORKAROUNDS = [
    ('packed-overlay', 'packed variant record (ref 3.10.7): only its one-field overlay variants '
     'kept, so that its size is right unpacked', '.ins.pas',
     re.compile(r'(=\s*)packed\s+record\s+case\s+([\w$]+)\s+of\b(.*?)\bend\s*;', re.I | re.S),
     _packed_overlay),
    ('packed-record', 'packed record (ref 3.10.7): `packed\' dropped', '.ins.pas',
     re.compile(r'\bpacked(\s+record)\b', re.I), r'\1'),
    ('field-aligned', 'attribute on a record field (ref 3.15): `[aligned(n)]\' dropped',
     '.ins.pas', re.compile(r'(:\s*)\[\s*aligned(?:\s*\(\s*\d+\s*\))?\s*\]\s*', re.I), r'\1'),
]


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


def stub_text(text, names, forwards=()):
    """TEXT with the top-level routines NAMES stubbed: a global one
    deleted, an internal one reduced to `<heading> extern;' (without its
    attribute list), or deleted too when it is in FORWARDS (its `forward'
    heading elsewhere becomes the `extern' one, see extern_forwards).
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
    """A module file of the exported tree and its fragments."""

    def __init__(self, tree, path):
        self.tree, self.path = tree, path
        with open(os.path.join(tree, path)) as fh:
            self.text = fh.read()
        self.fragments = []                 # include paths holding routine bodies
        # routines with a `forward' heading in the module file
        self.forwards = {r.name for r in parse_routines(self.text) if 'forward' in r.options}
        self.routines = {}                  # name -> (fragment, Routine)
        for inc in module_includes(self.text):
            if inc.endswith('.ins.pas'):
                continue
            with open(os.path.join(tree, inc)) as fh:
                rs = parse_routines(fh.read())
            self.fragments.append(inc)
            for r in rs:
                self.routines[r.name] = (inc, r)

    def stub(self, names):
        """Stub NAMES in the tree's files (the survey's copy)."""
        deleted = []
        for inc in self.fragments:
            p = os.path.join(self.tree, inc)
            with open(p) as fh:
                s, d = stub_text(fh.read(), names, self.forwards)
            deleted += d
            with open(p, 'w') as fh:
                fh.write(s)
        with open(os.path.join(self.tree, self.path)) as fh:
            s = extern_forwards(drop_defines(fh.read(), deleted), names)
        with open(os.path.join(self.tree, self.path), 'w') as fh:
            fh.write(s)

    def unit_text(self, keep):
        """The module with every fragment inlined and every top-level
        routine except KEEP stubbed."""
        others = set(self.routines) - {keep}
        deleted = []

        def inline(m):
            with open(os.path.join(self.tree, m.group(1))) as fh:
                s, d = stub_text(fh.read(), others, self.forwards)
            deleted.extend(d)
            return s

        frag = '|'.join(re.escape(f) for f in self.fragments) or '(?!)'
        s = re.sub(r"(?m)^\s*%include\s+'(" + frag + r")'\s*;", inline,
                   extern_forwards(self.text, others))
        return drop_defines(s, deleted)


def survey_stage(dpc, mod, check_only, stubbed_before, log):
    """Run one stage until the module passes it, stubbing routines.
    The internal routines stubbed so far (STUBBED_BEFORE and this stage's)
    are named to dpc with `--module-routine' at their header address, so
    that their callers still compile.  -> ([(diag, routine)], stubbed
    names, ok)."""
    entries, stubbed = [], []
    fragment_ok = True
    for _round in range(50):
        opts = []
        for n in (list(stubbed_before) + stubbed) if fragment_ok else ():
            _inc, r = mod.routines[n]
            if r.internal and r.addr is not None:
                opts.append('--module-routine=%s=0x%X' % (n.upper(), r.addr))
        if opts:
            # dpc wants a fragment origin with --module-routine; the
            # lowest routine address of the module serves (compile only)
            lo = min(r.addr for _i, r in mod.routines.values() if r.addr is not None)
            opts[:0] = ['--fragment-origin', '0x%X' % lo]
        rc, err = run_dpc(dpc, mod.tree, mod.path, check_only=check_only,
                          out=None if check_only else os.devnull, opts=opts)
        log.write(err)
        if rc and opts and 'a fragment may have only one' in err:
            # dpc takes --module-routine only with --fragment-origin, and
            # that only for one code section: the stubbed internal
            # routines' callers get charged with their calls instead
            print('srcsurvey: %s: %s' % (mod.path, err.strip().splitlines()[-1]))
            print('srcsurvey: %s: so no --module-routine: the callers of stubbed internal '
                  'routines are stubbed too' % mod.path)
            fragment_ok = False
            continue
        diags = parse_diags(err)
        loc = Locator(mod.tree)
        names = []
        for d in diags:
            r = loc(d['file'], d['line'])
            entries.append((d, r))
            if d['sev'] == 'error' and r and r not in names and r not in stubbed:
                names.append(r)
        if rc == 0:
            return entries, stubbed, True
        if not names:
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


def data_sections(asm_text, linkmap, module):
    """`--section-addr' pairs for the module data sections of an .s."""
    out = {}
    for sec in re.findall(r'\.section\s+"?(\.(?:bss|data)\.moddata\.[^",\s]+)', asm_text):
        parts = sec.split('.')
        want = parts[-1] if len(parts) > 4 else '.DATA'
        for s in linkmap.sections:
            if s.kind == 'D' and s.name == want:
                for g in linkmap.segments:
                    if g.kind == 'D' and g.name == module.upper() and s.contains(g.addr):
                        out[sec] = g.addr
    return out


def conform_routine(mod, name, dpc, linkmap, work, out, table=None):
    """Compile top-level routine NAME alone and compare it with the
    image.  MOD reads the files as they are now; TABLE (default
    MOD.routines) is the module's routines before any was stubbed, for
    the addresses of the routines NAME calls.  -> 'OK', 'MISMATCH', or
    'SETUP: ...'."""
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
        with open(os.path.join(mod.tree, _inc)) as fh:
            body = fh.read()[r.start:r.end]
        called = set(re.findall(r'[A-Za-z_$][\w$]*', strip_comments(body).lower()))
        items = []
        for other, (_i, o) in table.items():
            if other == name or other not in called or o.addr is None:
                continue
            sym = linkmap.lookup(other)
            if not (linkmap.segment_containing(o.addr) is linkmap.segment_containing(r.addr)):
                continue
            items.append(sym.name if sym else '%s=0x%X' % (other, o.addr))
        mopts, local = conform.module_routine_args(
            ['module-routines=' + ','.join(items)] if items else [], routines, linkmap)
        asm = os.path.join(work, os.path.basename(unit)[:-4] + '.s')
        rc, err = run_dpc(dpc, mod.tree, os.path.basename(unit), out=asm, opts=opts + mopts)
        if rc:
            return 'SETUP: dpc failed alone: ' + (err.strip().splitlines() or ['?'])[0]
        with open(asm) as fh:
            sect = data_sections(fh.read(), linkmap, os.path.basename(mod.path)[:-4])
        ok = conform.check(asm, specs, linkmap, sect, work=work, name=name,
                           local_syms=local, quiet=not out, out=out or (lambda *a: None))
        return 'OK' if ok else 'MISMATCH'
    except conform.SetupError as e:
        return 'SETUP: %s' % e


def cmd_run(a):
    work = os.path.abspath(a.work)
    tree, commit = export_rev(os.path.abspath(a.source), a.rev, work)
    print('srcsurvey: %s at %s (%s)' % (os.path.normpath(a.source), a.rev, commit[:12]))
    mods = a.modules or find_modules(tree)
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
        mod = Module(tree, path)
        logp = os.path.join(work, path.replace('/', '_') + '.log')
        with open(logp, 'w') as log:
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
            ok = True
            all_stubbed = []
            for stage, co in (('check', True), ('codegen', False)):
                entries, stubbed, ok = survey_stage(a.dpc, mod, co, all_stubbed, log)
                all_stubbed += stubbed
                report('%s: %s, routines stubbed: %s' % (path, stage, ', '.join(stubbed) or 'none'),
                       summarise(entries), verbose=a.verbose)
                if not ok:
                    print('srcsurvey: %s: %s stage fails outside any routine; stopping'
                          % (path, stage))
                    break
        print('srcsurvey: log %s' % os.path.relpath(logp, ROOT))
        if not ok or a.no_conform:
            continue
        import sau2map
        linkmap = sau2map.load()
        compiled = [n for n, (_i, r) in Module(tree, path).routines.items() if r.has_body]
        print('== %s: conformance of the %d routine(s) that compile' % (path, len(compiled)))
        cwork = os.path.join(work, 'conform')
        os.makedirs(cwork, exist_ok=True)
        fresh = Module(tree, path)
        for name in compiled:
            res = conform_routine(fresh, name, a.dpc, linkmap, cwork,
                                  print if a.verbose else None, mod.routines)
            _i, r = fresh.routines[name]
            print('  %-9s %-36s %s' % (res.split(':')[0], name,
                                       ('0x%X' % r.addr) if r.addr else '?')
                  + (('  ' + res[7:]) if res.startswith('SETUP') else ''))
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
    r.add_argument('--rev', default='HEAD')
    r.add_argument('--source', default=os.path.join(ROOT, '..', 'source'))
    r.add_argument('--work', default=os.path.join(ROOT, 'tmp', 'srcsurvey'))
    r.add_argument('--dpc', default=os.path.join(ROOT, 'target', 'debug', 'dpc'))
    r.add_argument('--no-workaround', action='append', default=[],
                   choices=[w[0] for w in WORKAROUNDS])
    r.add_argument('--no-conform', action='store_true')
    r.add_argument('-v', '--verbose', action='store_true')
    r.add_argument('modules', nargs='*')
    s = sub.add_parser('summarise')
    s.add_argument('diagnostics')
    s.add_argument('--tree', default='.')
    s.add_argument('-v', '--verbose', action='store_true')
    a = ap.parse_args(argv)
    return cmd_run(a) if a.cmd == 'run' else cmd_summarise(a)


if __name__ == '__main__':
    sys.exit(main())
