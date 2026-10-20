# domain-pascal

This project is a compiler for **Domain Pascal**, the Pascal dialect that
Apollo Computer documented in the *Domain Pascal Language Reference*
(000792-A01, December 1990).  It generates m68k code, and that code is meant
to **follow the same patterns as Apollo's own compiler**.  The reference
point is the compiled Pascal in the SR10.2 Domain/OS kernel.

It belongs to the Domain/OS archaeology work.  The sibling repository
`../source` reconstructs the SR10.2 SAU2 kernel in C.  This compiler is meant
to let that kernel be translated back into Pascal, its original language,
and built with the original ABI.  The output links against the kernel's
hand-written assembly and against the `M$ARITH` runtime.

The compiler is written in Rust; the test and conformance harness will be
in Python.  So far only the skeleton exists: every phase is a stub, and the
driver writes an assembly file that holds just a comment header.

- `crates/dpc-syntax`: source positions, diagnostics, `%` preprocessor,
  lexer, parser, AST.
- `crates/dpc-sema`: symbols, types, data layout, parameter conventions.
- `crates/dpc-ir`: the target-neutral, per-routine IR.
- `crates/dpc-target`: the target description trait.
- `crates/dpc-m68k`: the m68k backend and gas-syntax emitter.
- `crates/dpc`: the driver, `dpc [-I dir]... [-o out.s] file.pas`; it
  also takes the reference's compiler options (`dpc file.pas -config
  color mono -cond -idir dir ...`; `docs/compiler-options.md`).
- `runtime/`: the run-time library dpc ships, as gcc ships libgcc: for
  m68k, Apollo's `M$ARITH` (`runtime/m68k/m_arith.s`) and
  `PAS_$SET_BUILD` (`runtime/m68k/pas_set_build.s`, which dpc never
  calls), built by `make build` into `target/runtime/m68k/libm_arith.a`;
  `dpc --print-runtime` prints its path (`runtime/README.md`).
- `tests/golden/`, `tests/conformance/`: golden and image-conformance tests.
- `tests/corpus/baseline.txt`: the expected `dpc --check-only` outcome of
  each file of Apollo's SR10.2 Pascal inserts and examples
  (`tools/corpus.py`, `make corpus`; skipped when the install is absent).

Dependencies run one way: syntax, then sema, then ir, then m68k (with
target), then dpc.  Build with `make build` (or `cargo build`).  Run the
tests with `make test`, which runs `cargo test` and, once it exists, the
Python harness in `tools/tests`.

To find your way around:

- `CLAUDE.md`: project rules, language summary, ABI summary, architecture,
  test strategy, milestones.
- `docs/codegen-patterns.md`: what Apollo's compiler emitted, with kernel
  image addresses as evidence.
- `docs/000792-A01_Domain_Pascal_Language_Reference_Dec90.txt`: the
  language reference, as OCR text.
- `docs/reference-clarifications.md`: the owner's rulings on passages that
  are unclear or badly OCR'd.
- `tools/imgdis.sh`: disassembles a range of the SR10.2 SAU2 kernel image.

**Strict and best-effort compilation** (owner 2026-10-17, bead
domain-pascal-5kgg).  dpc refuses, with an "... is not supported yet"
error, what it cannot yet compile as Apollo's compiler did.  Some of those
refusals guard only the image's exact form (a register choice, an
instruction order, a hold or a reload, which of two forms seen in the
image) at a place where dpc already has a correct code path.  By default
dpc takes that path and prints a warning `best effort (not
byte-identical): ...`: the code computes the right values but is not
checked against the image (status "Best effort" in
`docs/codegen-patterns.md`).  `dpc --strict` makes every refusal an
error again.  The golden tests, `make conform` (`tools/conform.py`) and
`tools/srcsurvey.py` use `--strict`, so their expected outputs are the
image's forms; `tools/linktest.py` and ../source's kernel build use the
default.  Refusals of constructs dpc does not implement are errors in
both modes.

Only m68k is targeted for now; aarch64 may follow later for native test
runs.  LLVM is not used.
