# The aarch64 target (Mach-O, macOS user mode)

The owner started this backend on 2026-11-12 so that Domain Pascal can be
compiled to run in user mode on an Apple-silicon Mac (CLAUDE.md 1 and
4.5; epic bead domain-pascal-gamq).  It is the first target besides the
m68k, and the first step towards retargeting Domain/OS.

Unlike the m68k backend it reproduces nothing of Apollo's: it aims at
correct code.  The exec tests are its gate ("Tests" below).

## Use

```
dpc --target=aarch64-apple-darwin -o m.s m.pas
clang -arch arm64 -c m.s -o m.o
clang -arch arm64 m.o $(dpc --print-runtime=aarch64-apple-darwin) -o m
```

`--target` takes an LLVM-style triple (owner, 2026-11-12).  Supported:
`m68k-apollo-domainos` (the default) and `aarch64-apple-darwin`, also
spelt `arm64-apple-darwin`, `aarch64-apple-macos` or `arm64-apple-macos`,
with an optional OS version (`arm64-apple-macos14.0`).  Other triples
(`aarch64-unknown-linux-gnu`, ...) are refused until their backends exist.
A `program` gets a `_main` that calls its body and returns 0.

## The owner's decisions (2026-11-12)

- **Pointers are 8 bytes**, native addresses.  macOS keeps the low 4 GB of
  a 64-bit process unmapped, so 4-byte pointers were not an option
  without offsets from a base.
- **Data is little-endian.**
- **Calls follow Apple's arm64 ABI**, so Pascal and C call each other
  directly.

## Target description (`dpc_aarch64::Aarch64`)

| Question | Answer |
|---|---|
| pointer size | 8 |
| argument slot | 8 bytes each; slot k is register xk for k < 8, the stack above |
| first argument | x29+16 (the callee stores its register arguments there) |
| `val_param` limit | 4 bytes, as on the 680x0 (ref 5.5.7, p. 5-19); a pointer still goes by value (`dpc-sema` `param_conv`). **Uncertain**: the reference gives no figure for another target. |
| function results | as on the m68k (`dpc-sema` `result_reg`): ordinals, sets of 1 word or 2, records of 4 bytes and pointers in x0/w0; larger records through a hidden first argument (not x8: Pascal-to-Pascal only) |
| predeclared `_ISP__M68K` etc. (ref 4.7, p. 4-46) | all false |
| byte order | little-endian |
| a main program's external variables | by symbol, not through cells below a data base |

## Data layout

The front end's layout rules are unchanged (sizes, alignment, record
offsets), except that a pointer takes 8 bytes.  Byte order:

- **Integers, pointers, reals** are little-endian.
- **Register-sized sets** (1, 2 or 4 bytes) are integers whose bit k is
  member k, so they are little-endian too: a `set of 0..15` holding [0]
  is the bytes `01 00`.
- **Larger sets** keep the language's byte string (ref 3.9.3, Figure
  3-6): member k is bit k mod 8 of byte `(bits-1-k) div 8`.
- **Packed records' bit fields** keep the reference's byte image (ref
  3.10.7, "left to right within bytes and then by increasing byte
  address"): the backend reads a field's unit, reverses its bytes (`rev16`,
  `rev`), extracts or inserts, and reverses back.  A unit read as a
  native little-endian integer would not do: a 12-bit field read through
  a word unit would then overlap a 4-bit neighbour read through a byte
  unit.  A whole-byte or whole-word field of a packed record is an
  ordinary little-endian integer.
- **Strings and arrays of char** are bytes, as anywhere.

Consequence (accepted with the choice of little-endian): a variant record
that overlays an integer with smaller fields reads differently from the
m68k.  `status_$t`'s `all` against `fail`/`subsys`/`modc`/`code` is the
important case: on aarch64 `code` is the high half of `all`, not the low
one.  tests/exec/record_results.pas shows it.

## integer64 and PointerInt

Owner, 2026-11-12: a portable way to keep an address in an integer.

- **`integer64`** is a predeclared type of dpc's, not in the reference: a
  64-bit signed integer, 8 bytes, word aligned as a field (as `double`).
  It is checked on every target; the m68k backend refuses code that
  computes with one (`dpc_m68k::emit`, "not supported on m68k"), though a
  variable of it and its initial value are laid out.
  - An integer constant beyond 32 bits is an `integer64` (the reference's
    3.2.3 and 3.4 make it invalid); a based constant of up to 32 bits keeps
    the reference's reading as a 32-bit pattern (`16#FFFFFFFF` is -1), a
    wider one is a 64-bit pattern.  Constant folding beyond 64 bits is an
    error.
  - An operation with an `integer64` operand is 64-bit (as ref 4.3.1
    widens 16-bit operands to 32); `lshft`, `rshft`, `arshft`, `xor`,
    `&`, `!` and `~` work on all 64 bits; `firstof`/`lastof` are -2**63
    and 2**63 - 1.
  - It goes by value under `val_param` where pointers are 8 bytes, and a
    function returns it in x0.
  - `write` prints it (format kind 11, dpc's own, read by the aarch64 run
    time only).
- **`PointerInt`** is declared in dpc's include file
  `include/pointerint.ins.pas`: `integer32` under `%if _ISP__M68K`,
  `integer64` otherwise.  A type transfer between it and a pointer is
  always of equal sizes.  Use it with `-I include` and `%include
  'pointerint.ins.pas'`.

Test: `tests/exec-a64/integer64.pas` (aarch64 only).

## Code generation (`crates/dpc-aarch64/src/emit.rs`)

A tree-walking code generator over the IR, with no optimisation:

- Expression values live on an evaluation stack of the callee-saved
  registers x19-x28, so they survive calls; a routine saves those it
  uses.  An expression needing more than 10 is refused (not seen yet).
- A value of 1, 2 or 4 bytes is held in the 32-bit view of its register,
  sign- or zero-extended to 32 bits by its type; a pointer in the 64-bit
  view.  A word or byte operation wraps at its width, except a word `*`,
  whose product is 32 bits (the m68k's `muls.w`; the IR relies on it,
  tests/exec/abs_more.pas).
- Shifts take their count mod 64 on the operand extended to 64 bits, as
  the m68k's register shifts do (a longword shifted by 32 is 0).
- Every frame object stays in memory at `dpc-sema`'s offset from x29.

Frame, from high addresses down:

```
caller's frame:  stack arguments 9.. (at x29+16+8*8 ..)
callee:          register arguments 1..8, stored at entry (x29+16 ..)
                 saved x29, x30                       <- x29
                 dpc-sema's locals (negative offsets)
                 `with` temporaries, the static-link home
                 locals beyond the frame (`big_locals`)
                 saved x19..x28                        <- sp
```

- **Calls**: arguments are evaluated right to left (where the m68k pushes
  them), then moved to x0-x7; more than 8 go on the stack, 8 bytes each
  (generic AAPCS64, not Apple's packed stack arguments: a C callee with
  more than 8 arguments of less than 8 bytes would not match).  A value
  argument of less than 32 bits is extended to 32, as Apple's ABI asks.
- **Nested routines**: the static link travels in x15 and is kept in a
  frame slot; deeper frames are reached through each routine's link slot
  at x29-8 (`dpc-ir`'s `link_slot`).
- **Module data**: each data area is a local label in `__DATA,__data`
  reached with `adrp`/`add`; other units' objects through the GOT.
  Overlay sections (ref 3.14) are common symbols, or defined by the unit
  that initialises them.
- **Constants by address** go to `__DATA,__const`.
- **`for` loops** compute their bounds as 64-bit integers (an unsigned
  word bound unsigned, ref 4.3.3 and reference-clarifications question
  32) and count their passes, as the m68k's `dbf` does; the index is
  stepped after each pass and is one past `to` after a normal end.
- **Copies** go forward a longword at a time, like `move.l (An)+,(Am)+`,
  so an overlapping copy gives what it gives on the m68k.  A string
  constant assigned to an array of char is blank-padded (ref p. 4-204).

Refused on aarch64: `disable`, `enable`, `set_sr` (supervisor
instructions, ref Appendix E); real arithmetic (the IR names the m68k's
floating point interpreter; bead domain-pascal-s6tc); a call of an
`internal` routine of another unit.

## Run time (`runtime/aarch64/dpcrt.c`)

C, archived in `target/runtime/aarch64/libdpcrt.a` by `make build` on an
arm64 macOS host: `PAS_$WRITE`, `PAS_$WRITELN`, `PAS_$OUTPUT`,
`PAS_$NEW`, `PAS_$DISPOSE` (runtime/README.md).

## Tests

`make exectest-a64` (`tools/exectest_a64.py`) runs tests/exec natively:
dpc, `clang -c`, a C driver generated from each test's `call:` lines,
the C stubs `tools/exectest/a64_stubs.c`, and the run time.
`tests/exec/aarch64.txt` lists:

- `skip`: the tests that do not apply (the m68k runner's device register
  `exec_$counter`, `disable`/`enable`, type transfers between a pointer
  and `integer32`);
- `xfail`: reals;
- `expect`: the tests whose output differs by design, with the aarch64
  output in `tests/exec/aarch64/NAME.expect`.  Most are memory dumps of
  words; each was checked to hold the m68k expectation's bytes on every
  line, only reordered, or (type transfers, overlays) against the layout
  above by hand.

`tests/exec-a64/` holds tests of aarch64 only, in the same form
(`integer64`, which the m68k refuses), with dpc's `include/` on their
include path.

`warns:` lines are not checked: they name the m68k backend's best-effort
paths.

## Open questions

- **Pointers and `integer32`.**  Kernel and Apollo code moves addresses
  through `integer32` (`integer32(p)`, `ptr_t(i)`); the type transfer
  needs equal sizes (ref p. 4-197), so dpc refuses it on aarch64.  Such
  code needs `PointerInt` instead (above).
- `read`/`readln` (bead domain-pascal-54tf).
- Reals (bead domain-pascal-s6tc).
