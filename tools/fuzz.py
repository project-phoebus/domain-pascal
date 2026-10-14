#!/usr/bin/env python3
"""Robustness fuzzer for dpc (bead domain-pascal-1dn).

dpc must never panic on user input: every failure is a positioned
diagnostic, exit status 1 (2 for usage errors).  A panic is caught by the
driver's backstop and reported as `internal error ... (please report)`,
exit status 3.  This script feeds dpc many generated inputs and flags any
run whose exit status is not 0, 1 or 2, or whose stderr says `internal
error`.

usage: fuzz.py {modules,mutate,soup} [--count N] [--seed S] [--timeout T]
                                     [--max-seconds M] [--dpc PATH] [--keep DIR]
       fuzz.py show SEED      (print the module `modules` generates from SEED)

Deterministic: input i of a run is generated from seed S+i alone, so a
failure is reproduced by `--seed S+i --count 1` (or `show`).  Each dpc run
is killed after --timeout seconds (default 5; a timeout is a failure), and
the run stops early, with a notice, after --max-seconds in all (default
300), so the fuzzer is safe to run in batches of a few hundred.

`modules` generates random, syntactically valid modules from a grammar of
the supported subset (declarations, records, packed records, pointers,
arrays, sets, enumerations; if/case/while/repeat/for/with/goto/exit/next/
return; calls, nested routines and functions) and runs each through the
whole pipeline and through `--check-only`.  Most are type-correct, so they
reach the code generator, where a refusal ("not supported yet") is a
valid outcome.  `mutate` makes one to three token mutations of the repository's own
Pascal sources (tests/golden, tests/conformance, tests/exec).  `soup`
feeds random token sequences (`--parse-only` and the whole pipeline).  Failing inputs are copied to --keep (default
tmp/fuzz-fail/).  Exit status 1 when anything failed.
"""

import argparse
import collections
import os
import random
import re
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_DPC = os.path.join(ROOT, "target", "debug", "dpc")

# Scalar kinds: i16, i32, bool, char, enum, sw (set of 0..15),
# sl (set of color: a 32-element set), sub (0..255), ptr (^rec).
INT = ("i16", "i32", "sub")


class Gen:
    """One generated module."""

    def __init__(self, seed):
        self.r = random.Random(seed)
        self.out = []
        self.labels = 0
        self.nroutines = 0

    def pick(self, xs):
        return self.r.choice(xs)

    def chance(self, p):
        return self.r.random() < p

    # ---- types and variables ------------------------------------------

    TYPE_OF = {
        "i16": "integer",
        "i32": "integer32",
        "bool": "boolean",
        "char": "char",
        "enum": "color",
        "sw": "wset",
        "sl": "cset",
        "sub": "byte_t",
        "ptr": "rec_p",
        "rec": "rec",
        "prec": "prec",
        "arr": "arr_t",
    }

    PRELUDE = """\
module fz;
type
  color = (red, green, blue, cyan, magenta, yellow, black, white);
  byte_t = 0..255;
  wset = set of 0..15;
  cset = set of color;
  rec = record
    a: integer;
    b: integer32;
    c: boolean;
    d: char;
    s: wset;
    e: color;
    nx: ^rec;
  end;
  rec_p = ^rec;
  prec = packed record
    f: 0..7;
    g: boolean;
    h: 0..31;
    k: byte_t;
  end;
  arr_t = array[1..10] of integer;
  barr_t = array[0..7] of byte_t;
  rarr_t = array[1..4] of rec;
  vrec = record
    case tag: integer of
      1: (x1: integer32);
      2: (y1, y2: integer);
  end;
"""

    EXTERNS = """\
procedure ext_$p(v: integer); options(val_param); extern;
procedure ext_$q(var x: integer32); extern;
procedure ext_$r(in a: integer; out b: integer32); extern;
procedure ext_$s(p: rec_p; c: char); options(val_param); extern;
function ext_$f(v: integer): integer; options(val_param); extern;
function ext_$g(var r: rec): boolean; extern;
function ext_$h: rec_p; extern;
function ext_$l(a, b: integer32): integer32; extern;
"""

    def var_names(self, scope):
        return [n for n, _ in scope]

    def vars_of(self, scope, kinds):
        return [(n, k) for n, k in scope if k in kinds]

    # ---- expressions --------------------------------------------------

    def lvalue(self, scope, kind, write=False, by_ref=False):
        """A designator of `kind`, or None; one that can be assigned
        (`write`: not an `in` parameter) or passed by reference (`by_ref`:
        not a field of a packed record)."""
        cands = []
        for n, k in scope:
            if write and n in self.readonly:
                continue
            if k == kind:
                cands.append(n)
            elif k == "rec" or k == "ptr":
                base = n if k == "rec" else n + "^"
                for f, fk in (("a", "i16"), ("b", "i32"), ("c", "bool"),
                              ("d", "char"), ("s", "sw"), ("e", "enum"),
                              ("nx", "ptr")):
                    if fk == kind:
                        cands.append(f"{base}.{f}")
            elif k == "prec" and not by_ref:
                for f, fk in (("f", "tiny"), ("g", "bool"), ("h", "tiny"),
                              ("k", "sub")):
                    if fk == kind:
                        cands.append(f"{n}.{f}")
            elif k == "arr" and kind in ("i16",):
                cands.append(f"{n}[{self.index(scope)}]")
            elif k == "barr" and kind == "sub":
                cands.append(f"{n}[{self.pick(['0', '7', '3'])}]")
            elif k == "rarr":
                for f, fk in (("a", "i16"), ("b", "i32"), ("c", "bool")):
                    if fk == kind:
                        cands.append(f"{n}[{self.pick(['1', '2', '4'])}].{f}")
            elif k == "vrec" and kind in ("i16", "i32"):
                cands.append(f"{n}." + ("y1" if kind == "i16" else "x1"))
        if not cands:
            return None
        return self.pick(cands)

    def index(self, scope):
        if self.chance(0.5):
            return str(self.r.randint(1, 10))
        plain = [n for n, k in scope if k == "i16"]
        return self.pick(plain) if plain else "1"

    def const(self, kind):
        if kind in INT:
            n = self.pick([0, 1, 2, 3, 7, 15, 16, 127, 255, -1, -2,
                           32767, -32768, 65535, 256, 70000, 16])
            if kind == "sub":
                n = abs(n) % 256
            elif kind == "i16" and not -32768 <= n <= 32767:
                n = n % 32768
            fmt = self.pick(["d", "d", "hex", "oct"])
            if n >= 0 and fmt == "hex":
                return f"16#{n:x}"
            if n >= 0 and fmt == "oct":
                return f"8#{n:o}"
            return f"({n})" if n < 0 else str(n)
        if kind == "tiny":
            return str(self.r.randint(0, 7))
        if kind == "bool":
            return self.pick(["true", "false"])
        if kind == "char":
            return self.pick(["'a'", "'Z'", "' '", "chr(0)", "chr(10)"])
        if kind == "enum":
            return self.pick(["red", "green", "blue", "white"])
        if kind == "sw":
            elems = self.r.sample(range(16), self.r.randint(0, 3))
            return "[" + ", ".join(map(str, elems)) + "]"
        if kind == "sl":
            elems = self.r.sample(["red", "green", "blue", "black"],
                                  self.r.randint(0, 2))
            if self.chance(0.2):
                elems = ["red..blue"]
            return "[" + ", ".join(elems) + "]"
        if kind == "ptr":
            return "nil"
        return "0"

    def var_expr(self, scope, kind, depth=0):
        """An expression of `kind` that is not a constant when the scope
        has a variable of that kind: a constant condition or `case`
        selector is refused before code generation, which then never
        sees the rest of the routine."""
        v = self.lvalue(scope, kind) if self.chance(0.7) else None
        if v:
            return v
        e = self.expr(scope, kind, depth)
        if kind == "bool" and e in ("true", "false"):
            k = self.pick(["i16", "i32", "sub", "enum"])
            v = self.lvalue(scope, k)
            if v:
                return f"({v} {self.pick(['=', '<>', '<', '>='])} {self.const(k)})"
        return e

    def expr(self, scope, kind, depth=0):
        if depth > 3 or self.chance(0.3):
            if self.chance(0.6):
                v = self.lvalue(scope, kind)
                if v:
                    return v
            return self.const(kind)
        r = self.r.random()
        if kind in INT:
            ops = ["+", "-", "*", "div", "mod", "&", "!"]
            if r < 0.45:
                k2 = self.pick(INT) if kind != "i16" else kind
                op = self.pick(ops)
                rhs = self.expr(scope, k2, depth + 1)
                if op in ("div", "mod") and rhs.strip("()").lstrip("-").split("#")[-1] in ("0", "00"):
                    rhs = "3"
                return f"({self.expr(scope, k2, depth + 1)} {op} {rhs})"
            if r < 0.55:
                return f"(-{self.expr(scope, kind, depth + 1)})"
            if r < 0.62:
                if self.chance(0.5):
                    return f"xor({self.expr(scope, kind, depth + 1)}, {self.expr(scope, kind, depth + 1)})"
                return f"(~{self.expr(scope, kind, depth + 1)})"
            if r < 0.7:
                f = self.pick(["lshft", "rshft", "arshft"])
                return f"{f}({self.expr(scope, kind, depth + 1)}, {self.r.randint(0, 9)})"
            if r < 0.75:
                return f"ext_$f({self.expr(scope, 'i16', depth + 1)})"
            if r < 0.8:
                return f"ord({self.expr(scope, self.pick(['enum', 'char', 'bool']), depth + 1)})"
            if r < 0.84:
                return self.pick(["sizeof(rec)", "sizeof(prec)", "firstof(byte_t)",
                                  "lastof(integer)"])
            if r < 0.88:
                fn = self.pick(self.functions(scope, kind)) if self.functions(scope, kind) else None
                if fn and self.call_args(scope, fn, depth):
                    return self.call_args(scope, fn, depth)
            if r < 0.97 and kind == "i32":
                return f"ext_$l({self.expr(scope, 'i32', depth + 1)}, {self.expr(scope, 'i32', depth + 1)})"
            if self.chance(0.9):
                return self.const(kind)
            if self.chance(0.3):
                return f"abs({self.expr(scope, kind, depth + 1)})"
            return f"{self.pick(['max', 'min'])}({self.expr(scope, kind, depth + 1)}, {self.expr(scope, kind, depth + 1)})"
        if kind == "bool":
            if r < 0.35:
                k = self.pick(["i16", "i32", "sub", "char", "enum", "i16"])
                return (f"({self.var_expr(scope, k, depth + 1)} "
                        f"{self.pick(['=', '<>', '<', '<=', '>', '>='])} "
                        f"{self.expr(scope, k, depth + 1)})")
            if r < 0.55:
                op = self.pick(["and", "or", "and then", "or else"])
                return f"({self.expr(scope, 'bool', depth + 1)} {op} {self.expr(scope, 'bool', depth + 1)})"
            if r < 0.63:
                return f"not {self.expr(scope, 'bool', depth + 1)}"
            if r < 0.72:
                if self.chance(0.5):
                    return f"({self.expr(scope, 'sub', depth + 1)} in {self.expr(scope, 'sw', depth + 1)})"
                return f"({self.expr(scope, 'enum', depth + 1)} in {self.expr(scope, 'sl', depth + 1)})"
            if r < 0.78:
                return f"odd({self.expr(scope, self.pick(['i16', 'i32']), depth + 1)})"
            if r < 0.84:
                p = self.lvalue(scope, "ptr")
                if p:
                    return f"({p} {self.pick(['=', '<>'])} nil)"
            if r < 0.88:
                rv = self.lvalue(scope, "rec")
                if rv and "^" not in rv:
                    return f"ext_$g({rv})"
            if r < 0.93:
                k = self.pick(["sw", "sl"])
                return f"({self.expr(scope, k, depth + 1)} {self.pick(['=', '<>', '<=', '>='])} {self.expr(scope, k, depth + 1)})"
            fn = self.functions(scope, "bool")
            c = self.call_args(scope, self.pick(fn), depth) if fn else None
            if c:
                return c
            return self.const("bool")
        if kind in ("sw", "sl"):
            if r < 0.6:
                return (f"({self.expr(scope, kind, depth + 1)} {self.pick(['+', '-', '*'])} "
                        f"{self.expr(scope, kind, depth + 1)})")
            if kind == "sw":
                v = self.lvalue(scope, "sub")
                if v:
                    return f"[{v}]"
            return self.const(kind)
        if kind == "char":
            if r < 0.5:
                return f"chr({self.expr(scope, 'sub', depth + 1)})"
        if kind == "enum":
            if r < 0.4:
                return f"{self.pick(['succ', 'pred'])}({self.expr(scope, 'enum', depth + 1)})"
        if kind == "ptr":
            if r < 0.3:
                return "ext_$h"
            if r < 0.6:
                rv = self.lvalue(scope, "rec")
                if rv:
                    return f"addr({rv})"
        v = self.lvalue(scope, kind)
        return v if v else self.const(kind)

    def functions(self, scope, kind):
        return [f for f in self.visible if f[1] == kind]

    def call_args(self, scope, routine, depth=0):
        name, _, params = routine
        if not params:
            return name
        args = []
        for mode, k in params:
            if mode in ("var", "out", "in out"):
                v = self.lvalue(scope, k, write=True, by_ref=True)
                if v is None:
                    return None
                args.append(v)
            elif k == "rec":
                v = self.lvalue(scope, k)
                if v is None:
                    return None
                args.append(v)
            else:
                args.append(self.expr(scope, k, depth + 1))
        return f"{name}({', '.join(args)})"

    # ---- statements ---------------------------------------------------

    def stmt(self, scope, ind, depth, ctx):
        pad = "  " * ind
        r = self.r.random()
        if depth > 3:
            r = r * 0.35
        if r < 0.25:
            kind = self.pick(["i16", "i32", "bool", "char", "enum", "sw", "sl",
                              "sub", "ptr", "i16", "i32", "tiny"])
            lv = self.lvalue(scope, kind, write=True)
            if lv:
                return [f"{pad}{lv} := {self.expr(scope, kind)};"]
            if self.chance(0.3):
                rs = [v for v in self.vars_of(scope, ("rec",)) if v[0] not in self.readonly]
                if len(rs) >= 1:
                    a, b = self.pick(rs)[0], self.pick(rs)[0]
                    return [f"{pad}{a} := {b};"]
            return [f"{pad}ext_$p({self.expr(scope, 'i16')});"]
        if r < 0.35:
            lines = [f"{pad}if {self.var_expr(scope, 'bool')} then begin"]
            lines += self.stmts(scope, ind + 1, depth + 1, ctx)
            if self.chance(0.4):
                lines += [f"{pad}end else begin"]
                lines += self.stmts(scope, ind + 1, depth + 1, ctx)
            return lines + [f"{pad}end;"]
        if r < 0.42:
            kind = self.pick(["i16", "enum", "char", "sub"])
            lines = [f"{pad}case {self.var_expr(scope, kind)} of"]
            pool = {"i16": ["0", "1", "2", "3", "5", "7", "100", "(-1)", "16#7fff"],
                    "sub": ["0", "1", "2", "3", "4", "8", "255"],
                    "enum": ["red", "green", "blue", "cyan", "white"],
                    "char": ["'a'", "'b'", "'Z'", "' '"]}[kind]
            labels = self.r.sample(pool, self.r.randint(1, min(5, len(pool))))
            if self.chance(0.3) and kind in ("i16", "sub"):
                labels = [", ".join(labels[:2])] + labels[2:]
            for c in labels:
                lines += [f"{pad}  {c}: begin"]
                lines += self.stmts(scope, ind + 2, depth + 1, ctx)
                lines += [f"{pad}  end;"]
            if self.chance(0.5):
                lines += [f"{pad}  otherwise"]
                lines += self.stmts(scope, ind + 2, depth + 1, ctx)
            return lines + [f"{pad}end;"]
        if r < 0.48:
            lines = [f"{pad}while {self.var_expr(scope, 'bool')} do begin"]
            lines += self.stmts(scope, ind + 1, depth + 1, ctx | {"loop"})
            return lines + [f"{pad}end;"]
        if r < 0.53:
            lines = [f"{pad}repeat"]
            lines += self.stmts(scope, ind + 1, depth + 1, ctx | {"loop"})
            return lines + [f"{pad}until {self.var_expr(scope, 'bool')};"]
        if r < 0.6:
            iv = self.vars_of(self.cur_locals, ("i16", "i32", "enum", "sub", "char"))
            iv = [v for v in iv if v[0] not in ctx]
            if iv:
                n, k = self.pick(iv)
                d = self.pick(["to", "downto"])
                lines = [f"{pad}for {n} := {self.expr(scope, k, 2)} {d} {self.expr(scope, k, 2)} do begin"]
                lines += self.stmts(scope, ind + 1, depth + 1, ctx | {"loop", n})
                return lines + [f"{pad}end;"]
        if r < 0.65:
            ws = self.vars_of(scope, ("rec", "ptr", "prec"))
            if ws:
                n, k = self.pick(ws)
                inner = [x for x in scope]
                fields = {"rec": [("a", "i16"), ("b", "i32"), ("c", "bool"), ("s", "sw")],
                          "ptr": [("a", "i16"), ("b", "i32"), ("e", "enum"), ("nx", "ptr")],
                          "prec": [("f", "sub"), ("g", "bool"), ("k", "sub")]}[k]
                inner += fields
                tgt = n + "^" if k == "ptr" else n
                lines = [f"{pad}with {tgt} do begin"]
                lines += self.stmts(inner, ind + 1, depth + 1, ctx)
                return lines + [f"{pad}end;"]
        if r < 0.7 and "loop" in ctx:
            return [f"{pad}if {self.var_expr(scope, 'bool', 2)} then {self.pick(['exit', 'next'])};"]
        if r < 0.73:
            return [f"{pad}if {self.var_expr(scope, 'bool', 2)} then return;"]
        if r < 0.77 and self.labels_here:
            return [f"{pad}if {self.var_expr(scope, 'bool', 2)} then goto {self.pick(self.labels_here)};"]
        if r < 0.9:
            procs = [p for p in self.visible if p[1] is None]
            c = self.call_args(scope, self.pick(procs)) if procs else None
            if c:
                return [f"{pad}{c};"]
        if r < 0.94:
            v = self.lvalue(scope, "i32")
            if v and "." not in v and "[" not in v:
                return [f"{pad}ext_$q({v});"]
            if v:
                return [f"{pad}ext_$r({self.expr(scope, 'i16')}, {v});"]
        if r < 0.96:
            p = self.lvalue(scope, "ptr")
            if p:
                return [f"{pad}ext_$s({p}, {self.expr(scope, 'char')});"]
        if r < 0.97:
            return [f"{pad}{self.pick(['disable', 'enable'])};"]
        return [f"{pad}ext_$p({self.expr(scope, 'i16')});"]

    def stmts(self, scope, ind, depth, ctx):
        lines = []
        for _ in range(self.r.randint(0 if self.chance(0.05) else 1, 3 if depth < 3 else 1)):
            lines += self.stmt(scope, ind, depth, ctx)
        return lines

    # ---- routines -----------------------------------------------------

    def local_vars(self, prefix, n):
        kinds = ["i16", "i32", "bool", "char", "enum", "sw", "sl", "sub",
                 "ptr", "rec", "prec", "arr", "i16", "i32", "barr", "rarr",
                 "vrec"]
        return [(f"{prefix}{k}", self.pick(kinds)) for k in range(n)]

    def type_name(self, k):
        return {"barr": "barr_t", "rarr": "rarr_t", "vrec": "vrec"}.get(k) or self.TYPE_OF[k]

    def routine(self, scope, ind, level):
        pad = "  " * ind
        self.nroutines += 1
        name = f"r{self.nroutines}" if level > 0 else f"fz_$r{self.nroutines}"
        is_fn = self.chance(0.35)
        rkind = self.pick(["i16", "i32", "bool", "ptr", "char"]) if is_fn else None
        params = []
        modes = ["", "", "var", "in", "out", "in out"]
        for k in range(self.r.randint(0, 3)):
            pk = self.pick(["i16", "i32", "bool", "char", "ptr", "sub", "rec", "sw"])
            params.append((self.pick(modes), pk, f"p{self.nroutines}_{k}"))
        opts = []
        if self.chance(0.4):
            opts.append("val_param")
        if level == 0 and self.chance(0.2):
            opts.append("internal")
        ps = "; ".join(f"{(m + ' ') if m else ''}{n}: {self.TYPE_OF[k]}" for m, k, n in params)
        head = f"{pad}{'function' if is_fn else 'procedure'} {name}"
        if params:
            head += f"({ps})"
        if is_fn:
            head += f": {self.TYPE_OF[rkind]}"
        head += ";"
        if opts:
            head += f" options({', '.join(opts)});"
        lines = [head]
        locs = self.local_vars(f"l{self.nroutines}_", self.r.randint(0, 5))
        labels = [str(10 + k) for k in range(self.r.randint(0, 1))]
        if labels:
            lines.append(f"{pad}label {', '.join(labels)};")
        if locs:
            lines.append(f"{pad}var")
            for n, k in locs:
                lines.append(f"{pad}  {n}: {self.type_name(k)};")
        inner = scope + [(n, k) for _, k, n in params] + locs
        self.readonly |= {n for m, _, n in params if m == "in"}
        sig = (name, rkind, [(m, k) for m, k, _ in params])
        # A recursive call is allowed.
        self.visible.append(sig)
        saved = list(self.visible)
        if level < 2 and self.chance(0.3 if level == 0 else 0.1):
            lines += self.routine(inner, ind + 1, level + 1)
        self.labels_here = labels
        self.cur_locals = locs
        lines.append(f"{pad}begin")
        groups = [self.stmt(inner, ind + 1, self.r.choice([0, 1, 2, 3]), frozenset())
                  for _ in range(self.r.randint(1, 3))]
        if labels:
            # A label on a statement of the routine's own body.
            groups.insert(self.r.randint(0, len(groups)), [f"{pad}  {labels[0]}:"])
        body = [line for g in groups for line in g]
        if is_fn and self.chance(0.9):
            body.append(f"{pad}  {name} := {self.expr(inner, rkind)};")
        lines += body
        lines.append(f"{pad}end;")
        self.labels_here = []
        self.visible = saved
        return lines

    def module(self):
        self.visible = [
            ("ext_$p", None, [("", "i16")]),
            ("ext_$f", "i16", [("", "i16")]),
        ]
        self.labels_here = []
        self.readonly = set()
        self.cur_locals = []
        globs = self.local_vars("g", self.r.randint(1, 6))
        lines = [self.PRELUDE, "var"]
        for n, k in globs:
            lines.append(f"  {n}: {self.type_name(k)};")
        lines.append(self.EXTERNS)
        for _ in range(self.r.choice([1, 1, 1, 2, 2, 3])):
            lines += self.routine(globs, 0, 0)
            lines.append("")
        return "\n".join(lines) + "\n"


SOUP = (
    "module program procedure function begin end var type const label "
    "if then else case of otherwise while do repeat until for to downto "
    "with goto exit next return record packed array set ^ nil and or not "
    "div mod in := = <> < <= > >= + - * / ( ) [ ] , ; : . .. & ! ~ "
    "integer integer32 boolean char options val_param internal extern "
    "define static univ_ptr addr sizeof lshft rshft x y z 0 1 16#ff 'a' "
    "true false %include %if %then %else %endif %var %enable %config "
    "{ } (* *) \" ' 99999999999 -32768 8#777 2#101 36#zz"
).split()


def soup(seed):
    r = random.Random(seed)
    head = "module s;\n" if r.random() < 0.7 else ""
    toks = [r.choice(SOUP) for _ in range(r.randint(1, 80))]
    return head + " ".join(toks) + "\n"


# `mutate`: token-level mutations of the repository's own Pascal sources
# (goldens, conformance units, execution tests), which reach far more of
# the code generator than generated modules, most of which are refused
# somewhere.
CORPUS_DIRS = ("tests/golden", "tests/conformance", "tests/exec")
TOKEN = re.compile(r"\{[^}]*\}|\(\*.*?\*\)|'(?:[^']|'')*'|\d+#[0-9a-zA-Z]+|\d+"
                   r"|[A-Za-z_$][A-Za-z0-9_$]*|:=|\.\.|<=|>=|<>|\S", re.S)
NUMBERS = ["0", "1", "-1", "2", "7", "8", "15", "16", "31", "32", "255", "256",
           "32767", "32768", "-32768", "65535", "65536", "2147483647",
           "16#7fffffff", "16#ffffffff", "16#80000000", "4294967296"]


def corpus():
    """The sources `mutate` starts from, sorted (so a seed always means
    the same input while the corpus is unchanged; a failure is kept, in
    --keep, for when it changes)."""
    out = []
    for d in CORPUS_DIRS:
        for dirpath, _, files in os.walk(os.path.join(ROOT, d)):
            out += [os.path.join(dirpath, f) for f in files if f.endswith(".pas")]
    return sorted(out)


OPERATORS = ["+", "-", "*", "/", "div", "mod", "&", "!", "=", "<>", "<", "<=",
             ">", ">=", "and", "or", "in"]


def token_class(t):
    if t[:1].isdigit():
        return "num"
    if t.lower() in OPERATORS:
        return "op"
    if t[:1].isalpha() or t[:1] in "_$":
        return "id"
    return "punct"


def mutate(seed, files):
    """(source, its directory): one to three token mutations of a corpus
    source chosen by `seed`.  Most keep the syntax (a name replaced by
    another of the file's names, a number by an edge value, an operator
    by another), so the result reaches the checker and the code
    generator; a few delete, duplicate or swap a token."""
    r = random.Random(seed)
    path = r.choice(files)
    with open(path) as f:
        text = f.read()
    spans = [m.span() for m in TOKEN.finditer(text)
             if not m.group().startswith(("{", "(*"))]
    toks = [text[a:b] for a, b in spans]
    if not toks:
        return text, os.path.dirname(path)
    by_class = collections.defaultdict(list)
    for t in toks:
        by_class[token_class(t)].append(t)
    for _ in range(r.randint(1, 3)):
        i = r.randrange(len(toks))
        c = token_class(toks[i])
        op = r.random()
        if op < 0.06:
            toks[i] = ""
        elif op < 0.1:
            toks[i] = toks[i] + " " + toks[i]
        elif op < 0.14 and i + 1 < len(toks):
            toks[i], toks[i + 1] = toks[i + 1], toks[i]
        elif c == "num":
            toks[i] = r.choice(NUMBERS)
        elif c == "op":
            toks[i] = r.choice(OPERATORS)
        else:
            toks[i] = r.choice(by_class[c])
    # Keep the text between tokens (comments, line breaks).
    out, last = [], 0
    for (a, b), t in zip(spans, toks):
        out.append(text[last:a])
        out.append(t)
        last = b
    out.append(text[last:])
    return "".join(out), os.path.dirname(path)


def run(dpc, path, extra, incdir=None, timeout=5.0):
    """Run dpc on `path` (with `-I incdir`); (failure or None, exit
    status, stderr).  A failure is a timeout, an exit status other than
    0, 1 or 2, or an `internal error` report."""
    inc = ["-I", incdir] if incdir else []
    try:
        p = subprocess.run([dpc, *inc, *extra, "-o", path + ".s", path],
                           capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return "timeout", None, ""
    if p.returncode not in (0, 1, 2) or "internal error" in p.stderr:
        return f"exit {p.returncode}", p.returncode, p.stderr
    return None, p.returncode, p.stderr


def first_error(stderr):
    """The first `error:` message of `stderr`, with quoted names and
    numbers blanked so that like messages count together."""
    for line in stderr.splitlines():
        if ": error: " in line:
            msg = line.split(": error: ", 1)[1]
            return re.sub(r"`[^`]*`|\b\d+\b|0x[0-9A-Fa-f]+", "_", msg)[:100]
    return None


def fuzz(mode, n, seed, dpc, keep, top=0, timeout=5.0, max_seconds=300.0):
    """Run `n` inputs from `seed` on (fewer if `max_seconds` runs out);
    print each failure and a summary (exit statuses; with `top`, the most
    frequent first errors).  The number of failures."""
    bad = 0
    done = 0
    deadline = time.monotonic() + max_seconds
    codes = collections.Counter()
    errors = collections.Counter()
    files = corpus() if mode == "mutate" else []
    with tempfile.TemporaryDirectory(dir=os.path.join(ROOT, "tmp")) as d:
        for i in range(n):
            if time.monotonic() > deadline:
                print(f"stopped after {max_seconds:g} s (--max-seconds)")
                break
            done += 1
            s = seed + i
            incdir = None
            if mode == "modules":
                src = Gen(s).module()
            elif mode == "mutate":
                src, incdir = mutate(s, files)
            else:
                src = soup(s)
            path = os.path.join(d, f"f{s}.pas")
            with open(path, "w") as f:
                f.write(src)
            runs = ([], ["--parse-only"]) if mode == "soup" else ([], ["--check-only"])
            for extra in runs:
                why, code, err = run(dpc, path, extra, incdir, timeout)
                codes[(" ".join(extra) or "full", code)] += 1
                if not extra and code == 1:
                    errors[first_error(err)] += 1
                if why is None:
                    continue
                bad += 1
                os.makedirs(keep, exist_ok=True)
                kept = os.path.join(keep, f"{mode}-{s}.pas")
                with open(kept, "w") as f:
                    f.write(src)
                first = err.strip().splitlines()[:1]
                print(f"seed {s} {' '.join(extra)}: {why}: {first} -> {kept}")
                break
    for (run_, code), k in sorted(codes.items(), key=lambda x: str(x)):
        print(f"  {run_}: exit {code}: {k}")
    for msg, k in errors.most_common(top):
        print(f"  {k:6d}  {msg}")
    print(f"{mode}: seeds {seed}..{seed + done - 1}: {done} inputs, {bad} failures")
    return bad


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("mode", choices=["modules", "mutate", "soup", "show"])
    ap.add_argument("seed_arg", nargs="?", type=int)
    ap.add_argument("-n", "--count", dest="n", type=int, default=500,
                    help="number of inputs (default 500)")
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--timeout", type=float, default=5.0,
                    help="seconds allowed per dpc run (default 5)")
    ap.add_argument("--max-seconds", type=float, default=300.0,
                    help="stop after this many seconds in all (default 300)")
    ap.add_argument("--dpc", default=DEFAULT_DPC)
    ap.add_argument("--keep", default=os.path.join(ROOT, "tmp", "fuzz-fail"))
    ap.add_argument("--top", type=int, default=0,
                    help="also list the TOP most frequent first errors")
    a = ap.parse_args(argv)
    if a.mode == "show":
        sys.stdout.write(Gen(a.seed_arg or 0).module())
        return 0
    os.makedirs(os.path.join(ROOT, "tmp"), exist_ok=True)
    if not os.path.exists(a.dpc):
        sys.exit(f"fuzz.py: no dpc binary at {a.dpc} (run `cargo build`)")
    bad = fuzz(a.mode, a.n, a.seed, a.dpc, a.keep, a.top, a.timeout,
               a.max_seconds)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
