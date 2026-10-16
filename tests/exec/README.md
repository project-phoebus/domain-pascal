# Execution tests

CLAUDE.md section 6 item 5 asks for tests that run compiled programs and
check them against Pascal semantics.  These tests do that on the m68k
itself, not on aarch64: dpc output runs on the Musashi 68010 emulator
(bead domain-pascal-f03).  Golden and conformance tests check that the
code *looks like* Apollo's.  These tests check that it *computes* what the
Pascal source says.  A failure here is a miscompile.

Run them with `make exectest`, which `make test` also runs, or with
`python3 tools/exectest.py [-v] [--trace] [tests/exec/NAME.pas ...]`.
They take under a second.

## Prerequisites

- **Musashi** (Karl Stenerud's 68000-family emulator, version 3.3).  It is
  **not** in this repository: its licence allows non-commercial use only
  and does not grant redistribution.  The driver compiles it from
  `MUSASHI_DIR`, which defaults to
  `~/src/domainos-archeology/otterlater/musashi`, into
  `target/exectest/`.  That directory is only read.  Its `m68kconf.h`
  enables address-error emulation, which the tests need.
- `m68k-elf-as`, `-ld`, `-objcopy`, `-nm`, and a host `cc`.
- **M$ARITH**: dpc's run-time library, the archive `dpc --print-runtime`
  names (`target/runtime/m68k/libm_arith.a`, built by `make build` from
  `runtime/m68k/m_arith.s`, the transcription of the image's arithmetic
  runtime; `runtime/README.md`).  It is linked as is, like libgcc: only
  tests that call an M$ entry pull it in.  Override the path with
  `EXECTEST_RUNTIME`.

If any of these is missing, the driver prints `exectest: skipped: ...`
and exits 0.

## How a test runs

1. `target/debug/dpc -I tests/exec` compiles `NAME.pas`, and any `link:`
   modules.
2. `m68k-elf-as -m68010` assembles the output.
3. The objects are linked with `tools/exectest/stubs.s` and the M$ARITH
   archive by
   `tools/exectest/link.ld`: code at 0x10000, then data, then bss.
4. `objcopy -O binary` produces a flat image.
5. `tools/exectest/runner` runs each `call:` in turn, in one memory image,
   so static data persists from one call to the next.

The work files (`.s`, `.o`, `.elf`, `.bin`) are kept in
`tmp/exectest/NAME/`.  The `.s` is the one to quote in reports.

The runner (`tools/exectest/runner.c`) does the following:

- It single-steps a 68010 in supervisor mode (so privileged
  instructions, `disable`/`enable`, do not trap).  It refuses to build
  unless Musashi's `m68kconf.h` enables the 68010 and address errors.
- For each call, it pushes the arguments right to left with the word-slot
  ABI (codegen-patterns 1.3):
  - a 2-byte pad first if the slots do not total a multiple of 4;
  - a byte goes in the high half of its slot.
- It loads canaries into D2-D7 and A2-A6, and junk into D0/D1/A0/A1.
- It fills the stack with 0xA5.
- It returns through a sentinel address.

On return it checks:

- the callee-saved registers;
- SP: just above the return address, since the caller pops the arguments;
- a guard area above the arguments;
- the interrupt mask: it must be back to 0.

The run fails on any of the following:

- an exception, especially an **address error** (a word or long access at
  an odd address, which the 68010 traps);
- an **illegal instruction**, which includes 68020-only encodings;
- a zero divide;
- a **wild access**: memory outside the image, the stack and the argument
  cells.  Low memory (the vectors) is outside, so a nil pointer is
  caught; a write to the code section (below `_etext`) is wild too, and
  so is a stub argument that points outside, or a word or longword one at
  an odd address;
- a stack overflow (SP within 256 bytes of the bottom of the stack);
- more than 2,000,000 instructions.

The runner prints `!! ...` lines into the compared output.  `--trace`
prints each instruction and the scratch registers on stderr.

## Stubs

`exec.ins.pas` declares the stub routines; `%include 'exec.ins.pas'`.  The
runner implements them in C.  Like a real call, each one:

- clobbers D0, D1, A0 and A1 with junk (except the register that holds its
  result);
- clobbers the condition codes.

A word or byte result has junk in the bits above it.

| Routine | Prints |
|---|---|
| `exec_$log_l` / `_w` / `_uw` / `_c` / `_bool` / `_x (in v)` | the value, by address: decimal, unsigned, `true` / `false` / `bool?0xNN` (not 0 or 0xFF), hex |
| `exec_$log_mem(in v: univ ...; in n)` | `mem` and n bytes in hex |
| `exec_$logv_l` / `_w` / `_c (v)` | a value parameter (`val_param`) |
| `exec_$mark(v)` | `mark N` (`val_param` word) |
| `exec_$id_l` / `_w` / `_bool (in v)` | nothing; returns v in D0 |
| `exec_$id_p(in v: univ_ptr)` | nothing; returns v in A0 (also a way to get `addr(x)` into a pointer) |
| `exec_$clobber` | nothing |

`exec_$counter` is a word that acts as a device register: each word read
returns the next value, and a store sets it (see `volatile.pas`).

## Writing a test

A test is a module, `tests/exec/NAME.pas`.  Its first comment holds the
directives:

```
{ What it tests, with ref and codegen-patterns citations.

  call: NAME_$ENTRY w:5 @l:-7! b:200 -> w
  link: other.mod.pas
  dpc: --loop-align=off
  xfail: why (a known miscompile)
  expect:
    one expected output line per line
    => 3
}
```

The directives:

- `call: SYMBOL ARGS [-> KIND]`: a call of an exported routine.  There can
  be any number.
  - Value slots are `l:N`, `w:N` and `b:N`.
  - By-address arguments are `@l:N`, `@w:N` and `@b:N`: N goes in a fresh
    cell.
  - A trailing `!` prints `out V`, the cell's value after the call.
  - `-> l|w|uw|c|b|a0` prints `=> V`, the result in D0 (signed long,
    signed word, unsigned word, unsigned byte, boolean) or A0 (hex).
  - Use the parameter conventions of ref 5.2 / Table 5-1.  With
    `val_param`, an `in integer32` is a value `l:`, not `@l:`.
- `link:`: more modules in `tests/exec/` named `*.mod.pas`, which are not
  tests themselves.  An example is a unit whose arrays the test reaches as
  another unit's.
- `xfail:`: the test documents a known miscompile.  It must build, run
  and fail: `XPASS` is an error, so remove the line once the compiler is
  fixed, and so is dpc (or a tool) refusing the test.  The expected output
  stays the **correct** one.
- `expect:` must come last.  The rest of the comment is the expected
  output.

Expected values come from the Pascal semantics, never from dpc's output:

- Cite the reference for anything subtle, e.g. `div` truncates toward zero
  and `mod` takes the dividend's sign (p. 4-65, 4-115).
- When a test fails, decide from the reference which side is wrong before
  touching either.
- dpc refuses many forms it has not catalogued yet ("not supported yet").
  Those are not failures.  Rewrite the test around them, and say in its
  header what it could not express.

## Tests

| Test | Covers |
|---|---|
| `smoke` | the harness: every stub kind, value arguments, a result, an out parameter |
| `regheld` | register-held locals across calls, branches, loops; M$ARITH results held |
| `regloop` | a loop calling M$ARITH and a stub; a variable changed in the loop |
| `cse` | reused values invalidated by stores through another index, a var call, a field, a pointer |
| `forward` | stored-local forwarding; a var call on the local |
| `ptr_dregs` | bead domain-pascal-pze: a by-address parameter whose address is also data, held in a data register; its copies reused across `if`s, made again after a call; passed on from the data register |
| `ptr_dregs_flow` | review of bead domain-pascal-pze: copies of pointers in data registers not reused across a call in a loop or inside `with`, reused after a join both parts copied on; a by-address parameter compared and stepped around a call; a pointer only stored, held in an address register |
| `stale_param` | bead domain-pascal-pe9: a value parameter re-read after its address is passed (also from a nested routine) |
| `late_slot_pass` | bead domain-pascal-i24w: by-address parameters loaded for one use after a branch into a register past dead slot loads (the out parameter after a status test in A3, a third slot after an `if` in A2), on both sides of the test |
| `one_stmt_params` | bead domain-pascal-s4a5: bodies of one `if`, `case`, `for`, `while` or `repeat` reading by-value parameters loaded at entry; a boolean tested on its load's flags |
| `entry_load_flags` | review of bead domain-pascal-s4a5: a parameter tested right after the entry loads on its load's flags (bytes, words, longs; zero, negative, unsigned bytes, low byte or word zero), also past `movea.l` entry loads; `tst` of a parameter loaded before the last |
| `args_held` | bead domain-pascal-9j1z: parameters loaded at entry whose last use is a call's argument in bodies with branches (in an `if` part, in each part, used on the other path, after an inner join, before the first branch, in a `case` arm, before a `return`, with others live across the call), held in callee-saved registers |
| `divmod` | `div`/`mod` signs, words and longwords, by powers of two, word operands widened |
| `udiv` | unsigned-word `div`/`mod`; longword by a word constant |
| `arith` | multiplication by constants, `muls.w` products, M$ARITH, wraparound |
| `wide_compare` | **xfail**: a word sum compared, 32-bit per ref p. 4-5 and 4-198 |
| `unsigned` | `0..65535` compares (`bhi`/`bcs`), zero-extension, bytes |
| `ulong` | bead domain-pascal-5j9: `0..2147483647` values of 2^31 and above compared unsigned, `div`/`mod` unsigned (M$ARITH, `lsr.l`); against an `integer32` or `integer` signed |
| `ulong_ops` | review of bead domain-pascal-5j9: `0..2147483647` decremented in place past 0, wrapped, `*` (M$MIU), mixed with `integer32` (signed), `case`, function result, uplevel, `while`, index |
| `narrow` | bead domain-pascal-8td: longwords narrowed to words and words to bytes (stored, passed by value, returned; from a register, the frame, the unit's data, a field by address read at the higher address, `rshft` and longword sums computed in 32 bits); words and bytes widened for value parameters; low words 0x5678, 0xFFFF, 0x8000, 0x7FFF, 0 and bytes 0xFF, 0x80, 0; `chr` of a volatile word read whole |
| `bits` | `&`, `!`, `~`, `xor`, `lshft`, `rshft`, `arshft` |
| `bytes` | char and boolean fields at odd offsets, char-indexed arrays, byte value slots |
| `byte_statics` | bead domain-pascal-yvko: char and boolean module variables and a routine's `static` char one to a word (SIO_IO 0xE2DEB8+0x48 `70 00 60 00`), initialised and not; stores leave the neighbours and pad bytes alone |
| `byte_ints` | `[byte]` subranges (ref 3.15.5): unsigned and signed bytes at odd offsets, widened for arithmetic and comparisons, transfers between them, by value and by address, a byte result, `&`/`lshft`/`rshft`, byte arrays; boundary values (0, 127, 128, 255; -128, -1) through `-`, `div`, `mod`, `~`, `!`, comparisons, `case`, and `lshft(b, 8) > 5` as a signed `integer` |
| `sets` | `+`, `-`, `*`, `in`, `<> []` on a word set; `in` on a 128-bit set |
| `const_sets` | bead domain-pascal-bitu: `x in [..]` with a run-time element, an integer constructor as a longword constant (`integer` and zero-extended `0..65535` elements, 0..31), a one-word enumeration set by `moveq`; in a condition and as a value |
| `packed_fields` | bit-field stores keep neighbours; a field across a byte boundary |
| `cases` | jump table with a gap and out-of-range values, compare chain, negative labels, chars |
| `gotos` | forward and backward `goto`, out of loops, into a `case` arm, from a function |
| `loops` | `for` over empty ranges, `downto`, negative bounds, `exit`, `next`, `while` |
| `forever` | bead domain-pascal-pkle: `repeat ... until false` left by `exit` in the middle, by a final `if ... then exit` (the negated branch back), by a final `exit` (no `bra` back), from an `if` part and `case` arms, by `return` and `goto`; `next` to the head; nested; functions left by `return`, `return` from a `case` arm, `goto` a label that ends the routine, an inner loop left by `exit` and the outer by `return` |
| `elem_bases` | bead domain-pascal-rlnm: elements of other units' arrays (1-based words, ×12 and ×56 records, `char` indices) read, stored and passed by address, indices from parameters, another unit's variable and a local held across a call; byte elements through a pointer in another unit; a negative lower bound, a longword index, `i + 1`, an element in a condition, a byte element by a word index, one array in two statements around a call, a computed value stored |
| `elem_addrs` | bead domain-pascal-o1kl: `with` records of other units' arrays (×12 and ×56 elements, word, longword and computed indices) read twice in the base's own register, and across a call in a callee-saved register, written through and passed by address; review: a negative lower bound, the index's variable changed in the body and by a call in it, a longword index by address with a 56-byte element, reads in both arms of an `if`, `exec_$clobber` between two reads |
| `index_mul` | bead domain-pascal-r3yy: index scalings and products by constants done with a multiply (×44, ×46, ×84, ×92, ×100 with the constant first; ×268 `mulu.w #c`): indices at both bounds, a negative lower bound, from a by-value or by-address parameter, a unit variable, a register-held loop index; another unit's array; word and longword products of negative words and beyond 16 bits (at the ends of `integer`); a `.w` index of -13200..13200 by `mulu.w`; a zero-extended byte index multiplied in place |
| `stepped_data` | bead domain-pascal-xm5w: `for` loops stepping a pointer through arrays of the unit's data, of the frame (`lea (k,A5)`, `lea (k,A6)` near, `movea.l` in place far) and of another unit (`movea.l #sym`); `with a[i]` through a copy of the pointer; `exit`/`return` from the loop |
| `stepped_data_more` | review of bead domain-pascal-xm5w: stepped pointers from a negative lower bound, run-time counts run zero times, part of the range, `downto` in place, 6- and 10-byte elements, nested `with`, `with` inside and around inner loops, two `with`s of one element, `exit`/`next`, nested loops, a frame array in place from 3, an element passed by address, one pass |
| `fill` | `for` clear loops (`clr (A0)+`, `dbf`) over another unit's arrays |
| `overlays` | bead domain-pascal-raxl: two units sharing overlay sections, `var (name)` (ref 3.14, 7.2.4 Method 4), under other names: one unit's initial values read by the other, each unit's stores seen by the other (common symbols, and a definition for the initialised one) |
| `shortcircuit` | `and then` / `or else` skip their right operand |
| `bools` | plain `and` / `or` in conditions, under `not`, stored; `odd` of words and longwords, negative ones |
| `and_or_call` | bead domain-pascal-i1by: a plain `or` / `and` whose right operand compares a function's result (the call on either side, a word through a pointer, 0, a constant, a register-held longword, `0..65535` unsigned) at -32768, -128, -1, 0, 1, 127, 128, 255, 32767 and the longword extremes |
| `withs` | `with` over a variable, a pointer changed in the body, an element (index changed), an element through a pointer across a call, nested and alias forms; `addr` stored and passed by value |
| `addr_values` | bead domain-pascal-j28d: `integer32(addr(x))` in differences (another unit's field offsets, `lea abs.l` and `sub.l #sym`; frame elements, both `lea`d), plus and minus a value (`add.l #sym`/`sub.l #sym`), stored as integers and read back through a pointer, passed by value |
| `nested` | uplevel reads and writes, siblings, recursion through the static link |
| `results` | results with junk above them, booleans, A0 pointers, `f := g(...)`, a result held across a call |
| `records` | 12-, 14- and 144-byte copies, through var parameters and pointers |
| `pointers` | routine pointers (`jsr (An)`), a linked list |
| `pointer_targets` | bead domain-pascal-s9j: objects reached through a pointer passed by address (`p.f^` with the record held, a field through a slot, the enclosing routine's slot, a pointer local, a pointer in the unit's data) |
| `pointer_targets_more` | review of bead domain-pascal-s9j: two pointer locals and two slots in one argument list, a call after a push through a held slot, a nested routine called by a sibling (static link in A1), `p^` of a pointer local, `p.f^` with a field of the same held record |
| `initdata` | initialised data, repeat counts, variants, strings, a `static` local across calls |
| `packed_init` | bead domain-pascal-c0j8: initial values of packed records' bit fields (KBD's kbd_$state_table bytes), read back |
| `packed_bools` | bead domain-pascal-51e: packed arrays of booleans (ref 3.11.4.2, 3.15.5 p. 3-62): every element of a 64-element array set and cleared alone (each bit of each byte), read back by conditions and a function result; indices by address, by value, a `for` index, a frame local, a module variable; an initialised 16-element array from 2 |
| `packed_bools_all` | review of bead domain-pascal-51e: each of the 64 elements set alone and cleared alone, every byte and element checked, then set and cleared one by one; 256-element arrays from 1 and from 8 (the test's low-byte arithmetic wrapping); initial values of 1-, 2-, 3- (padded to 4) and 4-bit elements; signed and unsigned byte elements; elements as `while`, `repeat` and `and then`/`or else` conditions |
| `volatile` | a volatile variable re-read in a loop and in one expression |
| `ptr_a0` | bead domain-pascal-78n2: pointer locals assigned a call that no call follows, held in A0 itself, around out and var parameters, other pointers, `with`, `case`, `while`, sets, block copies, nested calls, `and then`, results |
| `park_int_if` | bead domain-pascal-1j61: an integer local held in D0 moved to an address register in the one `if` part that refers to it (`then`; `else` after a `then` ending in `return`) and first in an `if` whose condition dereferences it; both sides of each test |
| `park_int_if_edge` | bead domain-pascal-1j61: the same, with D0 held over a compound condition, statements, a loop and a nested `if` before the move, an out parameter written after the `if`, and a condition reading the local with arithmetic |
