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
collapsed to one space, empty lines dropped.  A `<name>.args` file next
to a `.pas` holds extra `dpc` options, separated by white space.
`DPC_BLESS=1 cargo test --test golden` rewrites the expected files from
the compiler's output.

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
- `loops/align_off_from` - codegen-patterns 7: `dpc
  --loop-align-off-from` (options in `align_off_from.args`); fillers
  before the named routine, none from it on, the module end padded
  with zero words (VTOC_$MOUNT 0xE3871E / VTOC_$DISMOUNT 0xE387CE).
- `loops/fill` - codegen-patterns 7, 5.2: `for` loops over an array whose
  index is used for nothing else, as `movea.l #sym,A0`, count, filler,
  `clr.<s> (A0)+`, `dbf` (0xE00ECC..0xE00EDA).
- `calls/pop_branches` - codegen-patterns 1.1: the pop rule with
  branches (no pop when every path to the epilogue is free of pushes,
  calls and conditional branches; 0xE01166, 0xE00BCA).
- `calls/body_order` - codegen-patterns 6 ("Order"), 1.6: routines
  emitted in body order, a `forward` routine and an `extern` one defined
  later (ref 7.3.4) placed at their bodies and reached by forward `bsr`s
  (AUDIT_$SHUTDOWN 0xE70D2E); a routine left `extern` is called with
  `jsr`.
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
- `control/case_table` - codegen-patterns 7.3: `case` as a jump table
  (OSINFO_$GET_MMAP 0xE5C6C0..0xE5C6D6), the lower bound with `subq.w`
  (PROC1_$CREATE_P 0xE15178), `subi.w` and a widened `char` (ACL_$SERVER
  0xE495C8), `addq.w` (MST_$PRIV_SET_TOUCH_AHEAD_CNT 0xE44534); gaps,
  empty arms and the `otherwise` part (MAC_OS_$OPEN 0xE0B3AE).
- `control/case_chain` - codegen-patterns 7.3: `case` as a compare chain
  in source label order (SYSBUS_$INIT 0xE0AB64, label 0 on the load's
  flags; TIME_$GET_EC 0xE16762 `tst.w`; module-local 0xE1CC64; a
  longword chain; TTY_$I_SIGNAL 0xE1B82C's copied selector).
- `data/through_worker` - codegen-patterns 4, 1.5: LOG_$READ 0xE17800,
  A5 loaded for an internal callee, by-address parameters passed on.
- `regalloc/live_ranges` - codegen-patterns 12.4 R1-R3, 12.2, 1.5: the
  allocator (bead domain-pascal-g5q.5): parameters held across calls in
  A2/D2 with `pea (d,A2)` pushes (module-local 0xE32B76), a third data
  value in D2 without a call (0xE177A0), registers reused after a
  parameter's last use, a value dead before a call in a scratch register,
  a parameter live around a `repeat` with a call in it, and (12.4 R4(c))
  parameters whose last use is a call's argument held through the call in
  D2 / A2 (PROC1_$SET_ASID 0xE148FE; module-local 0xE1D592).
- `expr/arith` - codegen-patterns 8, 13, 3: `addq`/`subq`, constant-first
  `moveq; add`, `addi`/`subi`, `c - x`, constant multiplication with
  shifts and adds (x6 0xE6886C, x12, x28 0xE5AEF6) or `mulu.w` (x564
  0xE3BB6A), `ext.l; divs.w` and `swap`, `neg`, `not`, widening, updates
  in place (`addq`/`subi`/`andi`/`ori` to memory).
- `expr/marith` - codegen-patterns 8, 12.4 R3, 1.1, 1.4: M$ARITH calls
  (pad, argument 2 first, `addq.w #8`, the last one unpopped:
  TIME_$GET_ADJUST 0xE16ABE..0xE16AE0), a result passed on to another
  call (0xE1695C), D1/A1 held across them, `muls.w`/`mulu.w` products of
  words used as longwords (0xE14C14, 0xE67F2A), `divs.w` for a longword
  quotient used as a word (0xE10CAE), `M$OIS$WLW` (0xE30D3E), `bpl; addi;
  asr` for a signed `div` by a power of two (0xE14BEE, 0xE68698), `divu.w`
  on unsigned words (0xE4CE1A).
- `expr/bits` - codegen-patterns 8: constant-first `&` (0xE177A0),
  `andi`/`ori`/`eori` on computed values, shifts split above 8 (0xE02AA6),
  a shift by a register (0xE02D52).
- `expr/index` - codegen-patterns 5.2: array elements in the unit's data
  and the frame, `(d,An,Dn.w)` / `lea (0,An,Dn.w)` (0xE70DA8, 0xE0AA18,
  0xE3BB6E), scaling, the parameter-index copy (0xE3BB68, 0xE0AA8A), a
  store to an element, an element by address (0xE6C07A), `.l` indices.
- `expr/calls_sets` - codegen-patterns 3, 1.1, 5.1, 7.1: function calls in
  expressions (0xE04700, 0xE01BA4, 0xE0082C), the tail wrapper, `btst #k`
  on a one-word set (0xE4A412, 0xE013BA), comparisons as results (`seq`,
  `sgt`, `sne`).
- `stores/status` - codegen-patterns 9, 12.4 R4(b), 7.1, 3, 1.5: status
  out-parameters: `status.all := ...` through the slot (SMD_$BUSY_WAIT
  0xE1D8AA, MMAP_$ALLOC_CONTIG 0xE0D8FC, without their A5 loads, which
  need the always-mode options of `data/a5_modes`), slots
  used once in registers no earlier statement left a value in
  (IIC_$RECEIVE 0xE70AC0, XNS_IDP_$GET_STATS 0xE18FF4), registers used up
  by `(An)+` counting as free (DIR_$ADD_MOUNT 0xE534DC), error stores and
  `return`, a held status passed on with `pea (A2)` and tested with
  `tst.l`, `if status.all <> status_$ok then return` as one `bne` to the
  epilogue (TERM_$CONTROL 0xE66B3A), a status store then a constant
  result (IIC_$RECEIVE_CHECK 0xE70ADE).
- `data/a5_modes` - codegen-patterns 4, 2: which routines load A5
  (options in `a5_modes.args`): used mode (PROC1_$GET_LOADAV 0xE14BBE;
  PROC1_$TST_LOCK 0xE148CA loads nothing), `--a5-always` for an
  `internal` routine (0xE14B12), so its caller loads nothing, and for a
  used-mode routine (the PKT/XPD switch-backs), `--a5-always-from`
  (SMD_$BUSY_WAIT 0xE1D8A2), `--a5-absolute` (`lea <area>.l,A0`,
  REM_NAME_$REGISTER_SERVER 0xE4A4B2), bare `rts` routines that never
  load A5 (SMD_$PUTC 0xE1D89C).
- `frame/empty` - codegen-patterns 2: empty procedures as a bare `rts`,
  with and without parameters (TIME_$Q_INIT 0xE16C5C, MT_$RELEASE
  0xE70A4C), in a unit with data, the module end padded with 0x2048.
- `loops/while` - codegen-patterns 7.4, 12.5: `while` loops as `bra` to
  the bottom test, the head aligned like any loop head (MAC_OS_$CLOSE
  0xE0B4CC), an `and then` test (0xE4A028), a walk pointer held in an
  address register and tested with `cmpa.w #0,An` (0xE3CCCE), `exit` and
  `next` as branches past or to the test.
- `loops/for` - codegen-patterns 7.4: `for` loops with an index the body
  uses: the `dbf` count, then the index (`clr.w` / `moveq`), the step
  before `dbf` (`subq.w` for `downto`), a byte array indexed with the
  index register (0xE1CA70), count and index callee-saved across calls
  (DISK_$UNASSIGN_ALL 0xE6BE24), a run-time upper bound with its `subq.w
  #1; bmi` guard and the count copied (DTTY_$WRITE_STRING 0xE1D6AA), a
  count above 127 as `move.w #n`, `exit` and `next`.
- `sysprog/interrupts` - codegen-patterns 10, 12.5, 1.1: `disable` /
  `enable` as `ori.w #0x700,SR` / `andi.w #0xF8FF,SR` (TIME_$GET_ADJUST
  0xE16AB0, 0xE16ABA), `set_sr` as `move.w SR,Dn; move.w newsr,SR`
  (PROC1_$SET_VT 0xE149B6), its result stored, held in a local's register
  or discarded (0xE149F2), `discard` of a function call.
- `expr/builtins` - codegen-patterns 10.1, 13, 8: `sizeof`, `firstof`,
  `lastof`, `succ`, `pred`, `max`, `min`, `in_range` folded to constants;
  `succ`/`pred` of a variable as `addq`/`subq` in place (0xE00B3E).
- `nested/static_link` - codegen-patterns 6, 12.5: nested routines in the
  shape of PMAP_$FLUSH 0xE1376C: emitted before their enclosing routine;
  the static link passed in A1 to a routine a sibling also calls
  (0xE13928, 0xE13718, 0xE135A4), the dynamic link `(A6)` used by one
  only its parent calls (0xE13616); uplevel slots, objects and locals
  through the link register (0xE135B8, 0xE135DA, 0xE1362E); the 4-byte
  link slot of nested frames; the parent's uplevel local in its frame.
- `nested/recursion` - codegen-patterns 6, 4: a recursive nested routine
  passing its own link (0xE397D0), a nested function reading the
  enclosing local through `(A6)`, A5 inherited by nested routines.
