"""dpcflags.py - the extra dpc options ../source's build passes.

../source's domain_os/Makefile compiles each module file with

    $(DPC) $(DPC_FLAGS) -o $@ $<

where DPC_FLAGS holds the compiler options of the build, such as
`-config sau2' (ref 6.4.8, p. 6-12; docs/compiler-options.md).
tools/linktest.py and tools/srcsurvey.py compile ../source's modules as
that Makefile does, so they must pass the same options: without them a
module would compile its non-SAU2 conditional branches.

dpc_flags(TREE) reads DPC_FLAGS from TREE/Makefile:

  1. by asking make: `make -s -f Makefile -f - <query>', the query rule
     (on stdin) printing $(DPC_FLAGS) with $(info), so every form of
     assignment, override and `VAR=X' on the command line counts;
  2. if make is missing or fails: by reading the Makefile's own
     assignments of DPC_FLAGS (`=', `:=', `::=', `?=', `+=', backslash
     continuations, comments); a value with a `$' reference cannot be
     read so, and gives up;
  3. else none: no DPC_FLAGS, no extra options.

The flags go right after dpc's name and before the tools' own options,
as in the Makefile's rule: a list option such as `-config a b' then ends
at the next option (`-o', `-I'), never at the module's file name.
"""

import os
import re
import shlex
import subprocess

VAR = 'DPC_FLAGS'
MARK = '@@dpcflags@@'
QUERY = '__dpcflags_print__'
MAKE = 'make'
TIMEOUT = 120


def from_make(tree, make_vars=(), make=MAKE, makefile='Makefile'):
    """DPC_FLAGS as make sees it in TREE -> list of options, or None when
    make cannot say (no make, no Makefile, make failed)."""
    if not os.path.exists(os.path.join(tree, makefile)):
        return None
    rule = '%s:\n\t@:$(info %s$(%s))\n' % (QUERY, MARK, VAR)
    try:
        p = subprocess.run([make, '-s', '-f', makefile, '-f', '-'] + [v for v in make_vars if '=' in v] + [QUERY],
                           cwd=tree, input=rule, capture_output=True, text=True,
                           timeout=TIMEOUT)
    except (OSError, subprocess.TimeoutExpired):
        return None
    if p.returncode:
        return None
    for ln in p.stdout.splitlines():
        if ln.startswith(MARK):
            return shlex.split(ln[len(MARK):])
    return None


_ASSIGN = re.compile(r'^\s*(?:override\s+)?(?:export\s+)?' + VAR +
                     r'\s*(::=|:=|\?=|\+=|=)\s*(.*?)\s*$')


def from_text(text):
    """DPC_FLAGS from a Makefile's text -> list of options; [] when it is
    not assigned; None when a value refers to a variable (`$')."""
    lines, cur = [], ''
    for ln in text.splitlines():
        if ln.endswith('\\'):
            cur += ln[:-1] + ' '
            continue
        lines.append(cur + ln)
        cur = ''
    if cur:
        lines.append(cur)
    value = None
    for ln in lines:
        if ln.startswith('\t'):
            continue                    # a recipe line
        ln = ln.split('#', 1)[0]
        m = _ASSIGN.match(ln)
        if not m:
            continue
        op, v = m.groups()
        if '$' in v:
            return None
        if op == '?=':
            if value is None:
                value = v
        elif op == '+=':
            value = v if value is None else (value + ' ' + v).strip()
        else:
            value = v
    return shlex.split(value) if value else []


def dpc_flags(tree, make_vars=(), make=MAKE):
    """-> (options, how): the DPC_FLAGS of TREE's Makefile and where they
    came from (`make', `Makefile text', `none')."""
    for mv in make_vars:
        name, _eq, val = mv.partition('=')
        if name.strip() == VAR:
            return shlex.split(val), 'command line'
    flags = from_make(tree, make_vars, make)
    if flags is not None:
        return flags, 'make'
    path = os.path.join(tree, 'Makefile')
    if os.path.exists(path):
        with open(path) as fh:
            flags = from_text(fh.read())
        if flags is not None:
            return flags, 'Makefile text'
    return [], 'none'
