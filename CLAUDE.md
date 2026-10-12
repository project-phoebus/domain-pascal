# Agent Instructions: domain-pascal

## 1. Purpose and scope

This repository is a **Domain Pascal compiler**.  Domain Pascal is Apollo
Computer's Pascal dialect, as described in the December 1990 *Domain Pascal
Language Reference* (000792-A01).  The compiler targets the **m68k**, and
its output must **follow the same code patterns as Apollo's own compiler**,
as seen in the SR10.2 Domain/OS kernel image that the sibling repository
`../source` reverse engineers.

**Why it exists.**  `../source/domain_os` is a C reconstruction of the
SR10.2 SAU2 kernel.  The original kernel was written mostly in Domain Pascal.
The long-term plan is to translate that C tree back into Domain Pascal and
build it with this compiler.  The objects we produce must therefore:

- use the **same ABI** as the kernel's compiled Pascal;
- link with the hand-written assembly in `../source/domain_os/*/sau2/*.s`,
  and with the compiler's arithmetic runtime M$ARITH in
  `../source/domain_os/math/sau2/arith.s`;
- assemble with the toolchain `../source` already uses (`m68k-elf-as`,
  `m68k-elf-ld`).

**Mindset: we are archivists.**  The goal is to **reproduce the original
compiler's output patterns**, not to produce good code.

- Do not invent optimisations.
- Do not "clean up" the original's redundancies.  Examples: a `movea.l A0,A0`
  filler, reloading a value that is already in a register, a `link` that
  reserves unused space.
- When you do not know what the original did, find out from the image.  If
  the image does not settle it, leave a TODO and file an issue (section 7).
  Do not guess.

**Out of scope for now:**

- Any target other than m68k.  aarch64 is a "later, would be cool" target,
  mainly so the test suite can run compiled programs natively (section 4.5).
- LLVM or any other large optimising backend.  These are **forbidden**.
- Pascal I/O runtime (`writeln` etc.), reals and floating point, unless a
  kernel routine needs them.

## 2. The language

### 2.1 The reference

- **Text:** `docs/000792-A01_Domain_Pascal_Language_Reference_Dec90.txt`.
  This is OCR output (884 KB, form feeds between pages).
- **Searchable PDF**, needed for tables, syntax diagrams and code samples:
  `~/src/domainos-archeology/docs/apollo-docs/ocr/000792-A01_Domain_Pascal_Language_Reference_Dec90.ocr.pdf`.
- **OCR is imperfect.**
  - The OCR log flags 22 PDF pages as possibly poor and 2 as empty.  The
    list is in `docs/reference-clarifications.md`.
  - Tables come out scrambled (e.g. Table 5-1, Argument Passing
    Conventions), as do syntax diagrams.
  - Some words are garbled: "integerl6" means `integer16`, "readin" means
    `readln`.
- **Rule: never guess at an unclear passage.**  This covers garbled syntax,
  tables, code samples and ambiguous semantics.
  1. Read the same page in the PDF.
  2. If it is still unclear, **ask the owner**.  Quote the passage and give
     its section and printed page number (e.g. "5.2, p. 5-4").
  3. Record the owner's answer in `docs/reference-clarifications.md`.
     Always read that file before working on a language area.
- **Cite your sources.**  When code implements a language rule, cite the
  section and page in a comment, e.g. `/* ref 5.5.7 val_param, p. 5-19 */`.

### 2.2 Domain Pascal beyond ISO Pascal

Each item below is checked against the reference text; the section numbers
point into it.  Items marked **(kernel)** are directly visible in the image's
code or data.

**Lexical**

- `_` and `$` are allowed in identifiers (1.3.1, 2.1.1).  Every kernel
  symbol relies on `$`, e.g. `AST_$PAGE_ZERO`.
- The language is case-insensitive (2.1.6).
- Integers can be written in any base from 2 to 16: `16#1c6`, `8#723` (2.1.2).
- Three comment forms (2.1.4).
- Escape sequences inside strings (2.1.5.1).

**Program structure**

- `module name [, codesect [, datasect]];` (7.1, 7.1.1).
  - A module is a compilation unit with no main body.
  - Declaration parts can come in any order.
- Variable allocation clauses (7.2): `define` (allocate and export),
  `extern` (allocate elsewhere), and `static`.
- `var (section_name)` for named overlay sections (3.14).
- Routine attribute list `[section(code, data)]` (5.7.1).  The kernel is
  linked into named sections such as WIRED_PROC and WIRED_DATA
  **(kernel)**; see `docs/codegen-patterns.md` section 11.
- Compiler directives (4.7 "Compiler Directives", p. 4-43):
  - `%include`
  - `%if`/`%ifdef`/`%else`/`%elseif`/`%elseifdef`/`%endif`, `%config`,
    `%enable`, `%error`, `%exit`
  - `%natural_alignment`, `%push_alignment`, `%pop_alignment`
  - `%begin_inline`/`%end_inline`, `%begin_noinline`/`%end_noinline`
  - `%debug`, `%list`/`%nolist`, `%eject`
- Apollo system interfaces are `%include`d `.ins.pas` files.  We do not
  have the kernel's own include files.  Declarations are reconstructed from
  `../source/domain_os/**/*.h`.

**Types (chapter 3)**

- `integer` = `integer16` (16-bit, signed); `integer32` (32-bit) (3.2.1).
  **`integer` is 16 bits.**
- No unsigned types.  Unsigned values are written as subranges such as
  `0..65535` (3.4).
- `boolean` is 1 byte: 0xFF is true, 0 is false (3.5.3) **(kernel)**.
- Enumerations are 1 word (3.7.1).
- Sets are up to 1024 elements, sized in whole words, and stored as a
  big-endian bit string (3.9.3) **(kernel)**.
- Records:
  - variant records **(kernel)**, e.g. the DIR_$DO_OP reply;
  - packed records;
  - `aligned record` / `unaligned record` (3.10.6);
  - alignment rules (3.10.5; 3.15.6).
- Arrays:
  - 1-based arrays are common in the kernel **(kernel)**;
  - variable-length arrays (3.11.2);
  - array initialisers with repeat counts (3.11.1).
- Variable-length strings: `varying[n] of char` (3.11.2; operations
  p. 4-204; C.2.5).
- Pointers:
  - `^T`;
  - `univ_ptr` (3.13.2);
  - procedure and function pointer types (3.13.3) **(kernel: driver
    tables, `jsr (A1)`)**.
- Variable and type attributes (3.15):
  - `volatile`, `atomic`, `device`;
  - `address`, `size`;
  - `aligned`, `natural`.
- An attribute declaration part (3.16).
- Static variables can be initialised in `var` (3.2.2 etc.).

**Statements and expressions (chapter 4)**

- Bit operators and functions:
  - `&` (and), `!` (or), `~` (not);
  - `xor`;
  - `lshft`, `rshft`, `arshft` (4-33, 4-109, 4-171, 4-29, 4-221).
- `and then` / `or else` short-circuit operators (4-18, 4-134).
- `exit`, `next` and `return` statements (4-77, 4-123, 4-166).
- `otherwise` in `case` (4-36).
- `addr`, `sizeof`, `firstof`/`lastof`, `in_range`, `max`/`min`, `discard`,
  `align`.
- Type-transfer functions (4-197), and extensions to `with` (4-211, C.3.18).

**Routines (chapter 5)**

- Parameter modes:
  - value (no mode);
  - `var`;
  - `in`, `out`, `in out` (5.3.2);
  - `univ` to turn off type checking (5.3.3).
- Routine options (5.5): `forward`, `extern`, `internal`, `variable`,
  `abnormal`, `val_param`, `nosave`, `noreturn`, `d0_return`, `a0_return`,
  `c_param`.
- `routine_option` declarations and `default_routine_options` (5.6).
- **Argument passing (5.2, Table 5-1).**  By default *everything* is passed
  by address, and the callee copies value parameters.  `val_param` and
  `internal` pass value and `in` scalars of 4 bytes or less by value.  The
  kernel uses both conventions **(kernel)**: see
  `docs/codegen-patterns.md` section 1.2.
- Nested routines (2.4) **(kernel)**.
- Recursion (5.8).

**Do not implement** I/O (chapter 8), files or reals until something needs
them.

## 3. Target ABI and code patterns: summary

The full catalogue, with image addresses and a status for every claim
(Verified / Reported / Uncertain), is **`docs/codegen-patterns.md`**.  Read
it before touching the backend.  The most important points:

1. **Compiler output vs. hand assembly.**  Only compiled code is our model.
   - Hand assembly lives in `*_ASM` map segments or in code inside D
     segments.
   - Its signs: no `link`, register arguments, `rte`, `lea (d,PC),A5`.
   - It is already transcribed in `../source/domain_os/*/sau2/*.s`.
   - Do not copy its idioms.  This includes the `.short`-encoded
     CMP/SUB immediates and the PC-relative A5.
2. **Calls.**
   - Arguments are pushed right to left, and the caller pops.  The pop is
     skipped when `unlk` follows.
   - `jsr abs.l` across modules; `bsr.s`/`bsr.w` within a module.
   - By address with `pea`, or by value per the routine's options (ref 5.2).
3. **Word slots.**
   - 16-bit values are pushed as 2 bytes.
   - 8-bit values are pushed with `move.b` and occupy the **high** byte of
     a 2-byte slot.
   - If the total is not a multiple of 4, `subq.l #2,SP` is done **before**
     the pushes.  It is a pad, **not** a result slot: re-check any "result
     slot" comment in `../source`.
   - Reference: `../source/docs/pascal-abi.md`.
4. **Constants.**
   - `pea (n).w` pushes a long constant.
   - Adjacent word constants are merged into one long push.
   - Constants passed by address go to the code section and are passed
     with `pea (d,PC)`.  Pooled, sometimes pushed twice
     (`move.l (SP),-(SP)`).
5. **Frame.**
   - `link.w A6,#-n`, then `movem.l` of the callee-saved registers used
     (D2–D7, A2–A5).
   - Restore with `movem.l (-k,A6)`, then `unlk`, `rts`.
6. **Results.**
   - Scalars in D0, sized to the type.  Booleans are produced with `sne`
     and come back in D0.b.
   - Pointers in **A0** by default (ref 5.5.10/11).
   - A function wrapper `F := G(...)` is a bare call followed by the
     epilogue.
7. **Module data (A5).**
   - Exported entry points `lea <module data seg>.l,A5`.  The caller's A5
     is saved with `pea (A5)` or inside the `movem`, and restored from the
     frame.
   - Module-local workers and nested routines inherit A5.
   - Other modules' data is reached with absolute addresses.
8. **Nested routines.**
   - The enclosing frame is passed in **A1** (`movea.l A6,A1; bsr`), or
     the callee uses `(A6)`, the dynamic link, when only its parent calls
     it.
   - Nested routines are emitted before their parent.
   - Example: PMAP_$FLUSH 0xE1376C, nested routines at 0xE1359C and
     0xE1360C.
9. **Loops.**
   - Counted loops use `dbf`, with the counter loaded with N−1.  A count
     only known at run time is guarded with `subq.w #1; bmi`.
   - **Loop heads may be 4-byte aligned with the no-op `movea.l A0,A0`
     (0x2048), but only in "aligning" code.**  Within a map segment,
     alignment is on for a leading run of routines and off for the rest
     (cause unknown).  Aligning code also pads the segment end with
     0x2048; non-aligning code ends `rts; 0x0000`.  See
     codegen-patterns section 7 and `tools/loopalign.py`.
10. **`case`.**
    - Dense label sets use a word-offset jump table (`add.w D0,D0;
      move.w (tbl,PC,D0.w),D0; jmp (tbl,PC,D0.w)`).
    - Sparse ones use a `cmpi.w/beq` chain.
11. **Arithmetic.**
    - `mulu.w`/`muls.w`/`divu.w`/`divs.w` inline for 16-bit operands.
    - 32-bit `*`, `div` and `mod` call **M$ARITH** (`M$MIS$LLL`,
      `M$DIU$LLL`, `M$OIU$WLW`, …), with word-slot arguments and the result
      in D0.
    - No other runtime is used.
12. **Sets.**
    - A set is a big-endian multi-word bit string.
    - `in` is computed inline: byte = `(bits-1-k)>>3`, then `btst`.
    - A set that fits in a register uses `btst Dn,Dm`.
13. **Status.**
    - A `status_$t` out-parameter is cleared on entry (`clr.l (An)`).
    - Error codes are stored with `move.l #imm,(An)`.
    - Names come from `~/src/domainos-archeology/stcodes`.
14. **Interrupt level.**  The reference's systems-programming built-ins
    (Appendix E; Table 4-7): `disable` and `enable` (procedures) and
    `oldsr := set_sr(newsr)` (function).  Compiled kernel routines contain
    `ori.w #0x700,SR` … `andi.w #0xF8FF,SR`, i.e. `disable` … `enable`;
    `enable` forces IPL 0 rather than restoring.  `set_sr` is the
    save/replace form.  See codegen-patterns section 10.
15. **Sections.**
    - The reference defaults are `.text`/`.data`.
    - The kernel uses module or section names (WIRED_PROC/WIRED_DATA,
      .TEXT/.DATA, …).  Each module contributes one code chunk and one data
      chunk, in link order.
    - External names keep `$` and are upper-cased.
    - `internal` routines and `static` variables are local symbols.
16. **Encodings.**
    - 68010 instructions only.
    - Explicit branch sizes; do not rely on gas relaxation.
    - Mostly CMPI/ADDI/SUBI encodings, with a few exceptions (`90bc`
      sub.l #imm, `b?fc` cmpa.w #imm).  Where gas would choose differently,
      emit `.short`.

Where to look for more evidence:

- `../source/docs/pascal-abi.md`, `../source/docs/original-quirks.md`
- `../source/.claude/agent-memory/reverse-engineer/*.md` (e.g.
  `svc-d0-result-trap.md`, `no-libgcc-divides.md`, `ec-wait-abi.md`,
  `review-pass-lessons.md`, `handwritten-asm-verification.md`)
- the owner's memory notes
  `~/.claude/projects/-Users-toshok-src-domainos-archeology-source/memory/domain-pascal-*.md`
- the C comments in `../source/domain_os`.  Grep for `Pascal`, `nested`,
  `ARCH_PASCAL_WORD_SLOT`, `ARCH_RESULT_A0`, `A5 =`, `dbf`, `M$`.

The C comments were written while decompiling.  Treat them as leads to
verify, not as truth.  Known example: the "result slot" wording.

### Looking at the image

- `tools/imgdis.sh <VA-hex> <LEN-hex>` disassembles the SR10.2 SAU2 image
  (`~/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2/sau2/domain_os`;
  file offset = VA − 0xDFFC00, valid for 0xE00800..0xE88834).
- Find routine boundaries in the link map
  `~/src/domainos-archeology/sau2-maps/domain_os.10.2.map`.  Its names are
  authoritative.
- `gsk analyze <addr>` (Ghidra) may be used **read-only** if it is
  reachable.  Never rename anything, never switch the Ghidra program, never
  write to Ghidra from this repo.  That is `../source`'s job.

## 4. Architecture (proposal; keep it simple)

```
source (.pas + %include)  ->  preprocessor  ->  lexer  ->  parser -> AST
  ->  semantic analysis (symbols, types, layout, parameter conventions)
  ->  IR (simple tree or three-address, per routine; frames and slots explicit)
  ->  m68k code generator (pattern-directed, no global optimiser)
  ->  assembly text (gas syntax, explicit sizes)  ->  m68k-elf-as / m68k-elf-ld
```

### 4.1 Front end

A hand-written lexer and a recursive-descent parser.  The preprocessor
handles the `%` directives, with `%include` search paths.  Every token keeps
its source position, so diagnostics can point at the source.

### 4.2 Semantic analysis

This phase owns every decision that is part of the **ABI**, so that the
backend only executes them:

- symbol tables with nested scopes;
- the type system;
- **data layout**: sizes, alignment, record offsets, set sizes, the
  enumeration word;
- **parameter-passing decisions**: per parameter, by address or by value,
  slot size, local copy, from the mode and the routine's options (Table 5-1);
- routine linkage: global or local, section;
- uplevel references for nested routines.

### 4.3 IR

The IR is deliberately low-level and close to the m68k.  Frame slots,
argument slots, pads and A5-relative statics are explicit.  Keep it simple
enough that a reviewer can line it up with the image's instruction order.

### 4.4 Code generator

- Tree-pattern selection with a fixed, simple register policy chosen to
  reproduce the original.  Parameters are loaded into callee-saved D/A
  registers at entry, and so on (codegen-patterns section 12).
- Each pattern cites the image address it reproduces, in a comment.
- **Output: gas syntax for `m68k-elf-as`** with `-m68010` semantics.
  Branch and displacement sizes are always explicit.  Where gas cannot
  produce the original encoding, emit `.short` with the mnemonic in a
  comment (the convention of `../source/domain_os/*/sau2/*.s`).
- One output section per module code section and one per data section,
  named after the Pascal section names.  This fits the
  `.text.<name>`-style sections that `../source`'s linker scripts place in
  map order.

### 4.5 Target seam

- The backend sits behind an interface: a target description (sizes,
  alignment, register sets, calling convention) plus an instruction
  selector and an emitter.
- Layout and parameter-passing decisions that come from the *language*
  stay in the front end.  Decisions that come from the *ABI* (word slots,
  A5, result registers) live in the target description.
- aarch64 later: same front end and IR, a second selector and emitter.  Its
  purpose is to run the test suite natively, not ABI compatibility with
  Apollo.  Do not build it now.  Just do not hard-code m68k register names
  in the front end or the IR.

## 5. Implementation language: DECIDED

**The owner chose (2026-10-11): Rust for the compiler, Python for the test
and conformance harness.**  The comparison below is kept for the record.

| Option | For | Against |
|---|---|---|
| **Rust** (installed: cargo 1.83) | Strong types; exhaustive `match` catches unhandled AST/IR cases when several agents add variants in parallel. `cargo test` is built in. One fast binary for big golden and conformance runs. | More ceremony; slower to sketch in; borrow-checker friction in tree-rewriting passes. |
| **Python 3** (installed: 3.14) | Fastest to write. Same language as `../source`'s tools (`asm_compare.py`, `rfc_decode.py`, `gen_layout_ld.py`), so the conformance harness can reuse them directly. | No static exhaustiveness (mitigate with mypy + dataclasses + match); slower; easier for parallel agents to break each other's assumptions silently. |
| **OCaml** (not installed) | The classic compiler language: algebraic data types and pattern matching fit AST/IR/instruction selection well. | Not installed (the owner handles installs); less familiar; smaller ecosystem for the tooling around it. |

**Recommendation: Rust for the compiler, Python for the test and
conformance harness.**  The harness compares bytes, parses maps and drives
`m68k-elf-objdump`, which is where `../source` already has Python to borrow.
The owner may prefer all-Python for speed of iteration.  That is a reasonable
choice too: the compiler is not performance-critical, and the main risk
(silent breakage between parallel agents) can be contained with strict
typing and the golden tests.

## 6. Test strategy

1. **Unit tests per phase.**
   - Lexer: tokens, bases, comments, `$` identifiers, case folding.
   - Parser: one test per construct, plus the module and `%include`
     structure.
   - Semantics: layout tests that assert sizes and offsets from the
     reference's examples (3.9.3 sets, 3.10.5 records) and from
     `../source`'s `_Static_assert`s.  Parameter-convention tests that
     encode Table 5-1 row by row.
2. **Golden tests.**  Under `tests/golden/<area>/<name>.pas`, with an
   expected `.s` (normalised: no comments, canonical spacing).
   - One test per pattern in `docs/codegen-patterns.md`.  Its header comment
     names the pattern section and the image address it mirrors.
   - Updating a golden file needs a reason in the commit message.
3. **Image conformance (the main gate).**
   - Write Pascal versions of real kernel routines, compile and assemble
     them, link standalone at the routine's image address, and compare
     bytes with the image.
   - Relocations are matched symbolically, as `asm_compare.py` does.
     Absolute operands that refer to other modules are expected to differ
     only through relocations.
   - Start with these small, compiler-generated routines (all Verified in
     `docs/codegen-patterns.md`; the C versions in `../source/domain_os`
     give the semantics):

| Routine | Address, size | C in `../source` | Exercises |
|---|---|---|---|
| PROC1_$TST_LOCK | 0xE148CA, 0x1C | `proc1/tst_lock.c` | val_param word; boolean function result (`btst/sne/move.b` into D0); `link #-4` |
| TIME_$Q_INIT_QUEUE | 0xE16C5E, 0x22 | `time/q_init_queue.c` | byte and word value slots (byte in the high half); record field stores through a pointer |
| PROC1_$GET_LOADAV | 0xE14BBA, 0x20 | `proc1/get_loadav.c` | A5 save/`lea`/restore with `pea (A5)`; 12-byte record copy `move.l (A0)+,(A1)+` ×3 |
| AST_$PAGE_ZERO + module-local 0xE00EB0 | 0xE00EEC, 0x2A; 0xE00EB0, 0x3C | `ast/page_zero.c`, `misc/zero_page.c` | pad + word-slot calls to ML_$LOCK/UNLOCK; omitted final pop; `pea (0x16).w` merged word pair; `dbf` clear loop with the 0x2048 alignment filler; internal routine by value |
| LOG_$READ + module-local 0xE17778 | 0xE17800, 0x28; 0xE17778, 0x88 | `log/read.c`, `log/read_internal.c` | default by-address entry vs. by-value worker; `clr.w -(SP)` constant; A5 inherited by the worker; word arithmetic; run-time-count `dbf` loop over 1-based arrays |

   The harness must keep a table of the external symbols and their image
   addresses (ML_$LOCK 0xE20B12, MMU_$INSTALL 0xE24048, …), taken from the
   map.  Later candidates: XNS_IDP_$GET_STATS 0xE18FD6, MMAP_$ALLOC_CONTIG
   0xE0D8F0, SMD_$BUSY_WAIT 0xE1D89E, then PMAP_$FLUSH (nested routines)
   and OSINFO_$GET_MMAP (jump table).
4. **Link tests against `../source`.**  Compile a Pascal translation of a
   kernel routine, swap it for the C object in a `../source` build in a
   scratch copy, and run `../source`'s `make check` and host tests where
   they apply.  Never modify `../source`: work in `tmp/`.
5. **Later: aarch64 execution tests.**  Compile Pascal test programs for the
   host and run them, checking semantics (sets, variant records, nested
   routines, `div`/`mod` signs against the M$ARITH models in
   `../source/domain_os/math/{mult,div,mod}.c`).

## 7. Working rules

- **Issues.**  This repo uses **bd (beads)**; the owner ran `bd init`
  (2026-10-11).  Use it as `../source` does: `bd ready`,
  `bd update <id> --status in_progress`, `bd close <id>`; low-priority
  issues for TODOs.  Create an issue before citing its id; never write an
  id you have not seen `bd create` print.
- **Commits.**  Commit locally only.  **Never `git push`.**  Commit only
  when asked or at the end of a work unit, per the owner's instructions,
  and finish with a clean `git status`.
- **Scratch.**  Scratch files go in `tmp/` (git-ignored).  Never write
  outside this repo: `../source`, the image, the map, the docs and Ghidra
  are read-only from here.
- **Never invent identifiers.**  No made-up symbol names, addresses, status
  names or issue ids.
  - Look names up in the SAU2 map and status names in the stcode database.
  - Create the issue first, then cite its id.
  - If something is unknown, say so in words.
  - A routine the map does not name is "module-local at 0xADDR".
- **Cite evidence.**
  - Language behaviour: cite the reference section and printed page.
  - Codegen behaviour: cite the image address(es).
  - Mark anything unverified as such.
  - When you verify or refute an entry in `docs/codegen-patterns.md`,
    update its status in the same change.
- **Unclear reference passages.**  Check the PDF page first.  If it is
  still unclear, ask the owner, quoting the passage with its section and
  page, and record the answer in `docs/reference-clarifications.md`.  Never
  guess.
- **Fidelity over polish.**  Reproduce the original's oddities.  If you
  think the original had a bug, the place to record it is
  `../source/docs/original-quirks.md`.  That file is not ours to edit: tell
  the owner.
- **Parallel agents own separate areas.**  Coordinate through issues, not
  through edits to another agent's files.  Suggested split:
  1. **lexer/parser + preprocessor** (AST, `%` directives)
  2. **semantics/types/layout** (symbol tables, Table 5-1 conventions,
     record/set layout)
  3. **m68k codegen** (IR → instructions, frames, calls, emitter)
  4. **runtime/ABI conformance** (keeping `docs/codegen-patterns.md`
     current: verifying "Uncertain" entries against the image, adding
     addresses)
  5. **test harness** (golden runner, image-conformance comparator, link
     tests)

  Interfaces between areas (AST types, the IR, the target description)
  change only through an issue that the affected owners can see.
- **Small steps.**  Each change comes with its tests.  Do not run long
  silent commands.  Keep tool output short.

## 8. First milestones

1. **Owner decisions** (done 2026-10-11): Rust + Python (section 5);
   `bd init` run; interrupt control uses the reference's `disable` /
   `enable` / `set_sr` built-ins (Appendix E).
2. **Skeleton.**
   - Build system, an empty compiler driver, test runner.
   - `tools/` harness that links an `.s` at an image address and compares
     it with `tools/imgdis.sh` output and bytes.
3. **Front end for the kernel subset.**  Modules, declarations, integer and
   boolean types, records, pointers, procedures and functions with options,
   and `%include`.
4. **Codegen for straight-line code.**  First conformance pass:
   PROC1_$TST_LOCK, TIME_$Q_INIT_QUEUE, PROC1_$GET_LOADAV.
5. **Calls, constants, loops and module-local routines.**  AST_$PAGE_ZERO +
   0xE00EB0, LOG_$READ + 0xE17778.
6. **Nested routines, `case`, sets, M$ARITH calls.**  Then PMAP_$FLUSH and
   OSINFO_$GET_MMAP.
7. **First link test** inside a scratch copy of `../source`.
