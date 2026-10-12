# Golden tests

Each test is a small Pascal file, `tests/golden/<area>/<name>.pas`, next to
the assembly the compiler is expected to produce, `<name>.s`.  The expected
`.s` is normalised: no comments, canonical spacing.

- There is one test per pattern in `docs/codegen-patterns.md`.  The `.pas`
  header comment names the pattern's section and the image address it
  mirrors.
- A change to an expected `.s` needs a reason in the commit message.

See CLAUDE.md section 6, item 2.

The runner is `crates/dpc/tests/golden.rs` (part of `cargo test` /
`make test`).  It compiles each `.pas` with the `dpc` binary and compares
the normalised output: `|` comments removed, lines trimmed, runs of blanks
collapsed to one space, empty lines dropped.  `DPC_BLESS=1 cargo test
--test golden` rewrites the expected files from the compiler's output.

Tests:

- `sets/in_long_set` - codegen-patterns 5.1, 3, 2: PROC1_$TST_LOCK
  0xE148CA.
- `stores/entry_loads` - codegen-patterns 12.2, 13, 1.3, 2:
  TIME_$Q_INIT_QUEUE 0xE16C5E.
- `stores/const_stores` - codegen-patterns 13 (clr, st, move #n, byte
  immediates 0..255), 12.2.
- `stores/store_from_memory` - codegen-patterns 13 (memory to memory in a
  one-statement body): PROC2_$GET_BOOT_FLAGS 0xE41B62.
- `data/static_data` - codegen-patterns 4, 13, 11: `pea (A5)` / `lea` /
  restore around a routine that uses the unit's data (PROC1_$GET_LOADAV
  0xE14BBE..0xE14BD2), `(A5)` / `(d,A5)` operands (XNS_IDP_$GET_STATS
  0xE18FE6, PROC2_$GET_BOOT_FLAGS 0xE41B66), an internal routine that
  inherits A5 (0xE17778), the `.bss.moddata.<UNIT>` block.
- `data/block_copy` - codegen-patterns 5.2, 7: block copies of 12 and 14
  bytes unrolled (0xE14BC6, 0xE419B0), of 144 and 22 bytes as `dbf`
  loops (0xE01494, 0xE19DA2).
- `calls/calls` - codegen-patterns 1.1, 1.3, 1.4, 1.5, 1.6, 12.2: pushes
  of constants (`clr`, `pea (n).w`, `move.l #n`, merged word pairs, `st`),
  values and addresses, pads, `addq.w`/`lea` pops, no pop when no push
  follows (0xE41050), `jsr` and `bsr.s`/`bsr.w` (0xE00EB0, AST_$PAGE_ZERO 0xE00EEC).
- `loops/fill` - codegen-patterns 7, 5.2: `for` loops over an array whose
  index is used for nothing else, as `movea.l #sym,A0`, count, filler,
  `clr.<s> (A0)+`, `dbf` (0xE00ECC..0xE00EDA).
- `calls/pop_branches` - codegen-patterns 1.1: the pop rule with
  branches (no pop when every path to the epilogue is free of pushes,
  calls and conditional branches; 0xE01166, 0xE00BCA).
- `control/if_else` - codegen-patterns 7, 7.1, 12.2: `if`/`else`,
  nested `if`s sharing a join, `return` as a `bra` to the epilogue,
  `.s`/`.w` branches (LOG_$READ's worker 0xE1778C..0xE177CC).
- `control/compares` - codegen-patterns 7.1, 5, 13: `tst` against 0 and
  `nil`, `cmpi` against constants, `cmp` with a held left operand,
  signed and unsigned branches, booleans with `tst.b` and `bpl`/`bmi`,
  `not`, `and then`, `or else`.
- `control/repeat` - codegen-patterns 7.2: `repeat ... until`, the head
  aligned, an `or else` test (0xE031A2..0xE031B6), a call in the body
  popped.
- `data/through_worker` - codegen-patterns 4, 1.5: LOG_$READ 0xE17800,
  A5 loaded for an internal callee, by-address parameters passed on.
