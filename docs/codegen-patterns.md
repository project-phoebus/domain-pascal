# Code-generation patterns of Apollo's Domain Pascal compiler (SR10.2 kernel)

This is the catalogue of what the original compiler emitted for m68k, as seen
in the SR10.2 SAU2 kernel image.  It is the specification of our backend.
CLAUDE.md has a summary.

Every entry carries a **status**:

- **Verified**: seen in the image at the addresses given.  Check them with
  `tools/imgdis.sh <VA> <LEN>`.
- **Reported**: recorded by the reverse-engineering work in `../source/`
  (agent notes, C comments, `docs/pascal-abi.md`) but not re-disassembled
  for this file.  Re-check before relying on it.
- **Uncertain**: a hypothesis, or a question that is still open.

Addresses are SR10.2 SAU2 virtual addresses.  Names come from
`~/src/domainos-archeology/sau2-maps/domain_os.10.2.map`.  A routine the map
does not name is called "module-local, unnamed".  The descriptive names that
`../source/domain_os` uses for those are given only as file pointers.

---

## 0. Compiler output vs. hand-written assembly: know which one you are looking at

The image holds both kinds of code.  **Only compiler output is our
specification.**  The hand-written routines are transcribed byte for byte in
`../source/domain_os/*/sau2/*.s`.  We link against them; we never imitate them.

Signs of hand-written assembly:

- **Location.** The routine sits in a map segment whose name ends in `_ASM`
  (`MMU_ASM`, `TIME_ASM`, `PROC1_ASM`, `MST_ASM`, `NETWORK_ASM`...).  Or it
  is code placed inside a **D** (data) segment, for example `STOP_WATCH`
  at 0xE81814 or `TIME_$READ_CAL` at 0xE2AF5E inside `D E2ADFC TIME_ASM`.
- **No frame.** There is no `link`/`unlk`; arguments are read straight off
  SP (`move.w (0x8,SP),SR` in ML_$SPIN_UNLOCK 0xE20BBE).
- **Register arguments.** Arguments come in registers (`lea x,A0; bsr`).
  There are routines with no `rts` that fall through into the next one (the
  cal delay helpers at 0xE2AF54/0xE2AF58 in TIME_ASM).  There are
  `movem`-all thunks and `rte`.
- **A5 loaded PC-relative.** A5 is set with `lea (-N,PC),A5` (STOP_$WATCH).
  Compiled code loads A5 with an **absolute** `lea`; see section 4.
- **Encodings.** Hand assembly often uses the CMP/SUB/ADD/OR/AND forms with
  an immediate effective address (`b0bc`, `94bc`, `d47c`, `807c`; see
  `../source/.claude/agent-memory/reverse-engineer/handwritten-asm-verification.md`).
  Compiled code nearly always uses CMPI/SUBI/ADDI/ANDI (section 13).

The M$ARITH runtime (map `I E0ABD4 M$ARITH size = 1DC`) is hand assembly
that compiled code *calls*.  The same holds for the `MMU_$*`, `ML_$*` and
`PROC1_$SET_LOCK` entry points.  `MST_$SET`/`MST_$SET_CLEAR` (0xE1A0FC,
0xE1A118, segment MST_ASM) are hand assembly too, even though their bodies
look like set operations.

Linear-sweep caveat: `m68k-elf-objdump -D` decodes data as instructions.
Jump tables and constant cells will look like `orib`, `bfexts` and so on.
Always find the routine boundaries from the map first.

---

## 1. Calling sequence

### 1.1 Argument order and cleanup (Verified)

- Arguments are pushed **right to left**, so argument 1 is at `(4,SP)` on
  entry and at `(8,A6)` after `link`.
- The **caller pops** them, with `addq.w #n,SP` or `lea (n,SP),SP`.
  - AST_$PAGE_ZERO 0xE00EFC: `addq.w #4,SP`
  - 0xE00EB0 (module-local, unnamed; `../source/domain_os/misc/zero_page.c`)
    at 0xE00EC8: `lea (0xc,SP),SP`
  - AREA_$FREE_FROM at 0xE07FF4: `addq.w #8,SP`
- **Skipped pop before return.** When a call is the last thing before
  `unlk`, the pop is omitted and `unlk` discards the arguments
  (AST_$PAGE_ZERO 0xE00F0C `jsr ML_$UNLOCK` then 0xE00F12 `unlk`).
- **Push order is not argument order.** A list of operands in the order the
  pushes appear is *reversed* relative to the Pascal parameter list.  Derive
  the order from the callee's frame offsets: `(8,A6)` is argument 1.

### 1.2 By reference vs. by value: the routine's options decide (Verified + reference)

The reference (section 5.2, Table 5-1; 5.5.7 `val_param`; 5.5.4 `internal`)
says this:

- **Default convention.** Every argument is passed **by address**, whatever
  its mode.  For value parameters the *callee* makes a local copy.
- **`val_param`**, and also **`internal`** routines ("Currently, however,
  internal routines are treated the same as val_param routines", p. 5-4).
  Value and `in` parameters of 4 bytes or less, except character arrays,
  are passed **by value**.  Everything else is still passed by address.
- **`univ`** cancels the effect of `val_param` and `internal` (Table 5-1).
- **`c_param`** = `val_param` + `d0_return`, and records are also passed
  by value.

What the image shows:

- **By address.** User-visible entry points take everything by address,
  which is the default convention.
  - OSINFO_$GET_MMAP 0xE5C694: seven pointer arguments.
  - LOG_$READ 0xE17800: it receives `max_len` by address and dereferences
    it with `movea.l (0xc,A6),A0; move.w (A0),-(SP)` before passing it **by
    value** to its module-local worker at 0xE17778.
- **By value.** Kernel-internal entry points take small scalars by value,
  so they must have been declared `val_param` or with an equivalent default.
  - PROC1_$TST_LOCK 0xE148CA reads a word value: `move.w (0x8,A6),D0w`.
  - ML_$LOCK and ML_$UNLOCK are called with `move.w #imm,-(SP)`
    (AST_$PAGE_ZERO 0xE00EF2, 0xE00F08).
  - TIME_$Q_INIT_QUEUE 0xE16C5E reads a byte at `(8,A6)` and a word at
    `(0xA,A6)`.
  - Module-local workers are called with values (0xE00F02 `bsr` to
    0xE00EB0 with `move.l (8,A6),-(SP)`).
- **Uncertain:** how the kernel source spelled this.  It could be
  per-routine `options(val_param)` or `internal`, a `routine_option`
  declaration part, or `%config`/`default_routine_options` (reference
  section 5.6).  We do not have the kernel's Pascal source.  Our compiler
  must support all of these and get the per-routine convention right.  The
  C tree's prototypes (`../source/domain_os/*/*.h`) tell you, routine by
  routine, which convention the image uses.

### 1.3 Stack slots for small values: the "word-slot" ABI (Verified)

From `../source/docs/pascal-abi.md`:

- A 32-bit value, a pointer or an address is pushed as 4 bytes (`move.l`,
  `pea`).
- A **16-bit value** is pushed as **2 bytes** (`move.w x,-(SP)`).
- An **8-bit value** (char, boolean, 8-bit subrange) is pushed with
  `move.b x,-(SP)`.  The 68k keeps SP even, so this takes 2 bytes and the
  **byte lands in the HIGH (even-address) half** of its word.  Callees
  therefore read a byte parameter at the slot's first byte:
  - TIME_$Q_INIT_QUEUE 0xE16C62: `move.b (0x8,A6),D0b`
  - KBD_$RCV pushes a key with `move.b D0b,-(SP)` at 0xE1CE56.
- Booleans are pushed by value with `st -(SP)` (0xE1365C).  See
  `../source/.claude/agent-memory/reverse-engineer/` (SOCK_$PUT,
  RIP_$FIND_NEXTHOP notes).
- Adjacent parameters are **not** padded to 4 bytes.  A word followed by a
  longword leaves the longword at an address that is 2 mod 4 relative to
  the frame (IO_$TRAP: `(4)` vector.w, `(6)` handler.l).

### 1.4 Alignment pad before the pushes (Verified)

When the argument bytes do not add up to a multiple of 4, the caller first
does `subq.l #2,SP`, then the pushes.  The pad therefore lies **above** the
last parameter, and the caller's pop includes it.

- word + long + long = 10 bytes, then pad.  MAC_OS_$OPEN 0xE0B3DC
  `subq.l #2,SP; move.l (0x10,A6),-(SP); move.l D7,-(SP); move.w (0x30,A3),-(SP); ...; jsr (A1)`.
- One word.  AST_$PAGE_ZERO 0xE00EF0..0xE00EFC
  `subq.l #2,SP; move.w #0x14,-(SP); jsr ML_$LOCK; addq.w #4,SP`.
- Nested calls each get their own pad.  KBD_$RCV 0xE1CE48 and 0xE1CE4A have
  two `subq.l #2,SP` in a row: the outer call's pad (byte + long) and the
  inner call's pad (one byte).
- **Uncertain / caution:** several comments in `../source/domain_os` call
  this `subq.l #2,SP` a "Pascal function result slot"
  (`kbd/kbd_internal.h`, `rip/init.c`, `proc2/debug_unlink_from_list.c`,
  `.claude/agent-memory/reverse-engineer/mac-channel-flags-high-byte.md`).
  Every case re-checked for this file is the alignment pad.  For example,
  the MAC_OS driver "open" call that note describes pushes word + long +
  long.  Function results come back in registers (section 3).  Before
  trusting any "result slot" claim, add up the bytes that are pushed.

### 1.5 Constant arguments (Verified / Reported)

- **Long constant, by value:** `pea (n).w`.  0xE00EB4 `pea (0x16).w`
  pushes 0x00000016.  `clr.l -(SP)` and `clr.w -(SP)` push zeros
  (0xE135D0, 0xE135D2).
- **Adjacent word constants merged:** two adjacent word constants become
  one longword push.
  - 0xE00EB4 is the MMU_$INSTALL asid.w/prot.w pair, `pea (0x16).w`.
  - ACL/FILE call sites push the rights/option pair as one `move.l #imm`
    (Reported: `file-subsystem.md`).
  - The HIGH half is the *first* (lower-address) parameter.
- **Constant by address:** this is the default convention for a constant
  actual.  The compiler puts the literal in the **code section** and passes
  `pea (d,PC)`.
  - TERM_$SEND_KBD_STRING 0xE1AB0C: `pea (0x18,PC)` → 0xE1AB26.
  - The next instruction, `move.l (SP),-(SP)`, passes the **same** address
    again for a second parameter.
  - Pooled constants are shared between unrelated uses (Reported: NAME_$
    0xE5472E word 0; see `name-dir-subsystems.md`, and the
    `domain-pascal-codegen-conventions` owner memory note, OS_$INIT).
- **Expression by address:** a non-variable actual passed by address is
  first stored in a frame temporary, and that temporary's address is
  pushed.  TONE_$ENABLE 0xE1ACFC: `lea (0x1268,A5),A0; move.l A0,(-8,A6); pea (-8,A6)`.

### 1.6 Call instructions (Verified)

- **Same module and section:** `bsr.s` or `bsr.w` (0xE00F02, 0xE1781C).
- **Other modules:** `jsr abs.l` (0xE00EF6 `jsr 0x00E20B12`).
- **Procedure variables:** `movea.l ...,An; jsr (An)` (0xE0B3F0, 0xE1CE5E).
- All kernel addresses are absolute.  This kernel image has no
  PC-relative inter-module calls.

---

## 2. Frame, prologue and epilogue (Verified)

- **Prologue:** `link A6,#-locals`.
  - `link.w` is always used.  No `link.l` appears in the kernel image.
  - The size is sometimes larger than the locals that are used.  IO_$ALLOC
    0xE1A9C2 and PROC1_$TST_LOCK 0xE148CA have `link #-4` and never touch
    the slot.  **Uncertain:** whether that is a function-result variable
    (both are functions).
  - The displacement is always a multiple of 4: all 1171 `link.w A6,#d`
    found at routine addresses named in the map have `d mod 4 = 0` (a scan
    of every `4e56` word in 0xE00800..0xE88834 finds one exception, not at
    a routine start, possibly data).  **Uncertain:** whether the total is
    rounded or each frame object gets a 4-byte-aligned slot.  `dpc-sema`
    records the raw size (`Routine::frame_size`) and leaves this to the
    backend.
  - **Empty frame (Verified):** `link.w A6,#0` when there are no locals:
    TIME_$Q_INIT_QUEUE 0xE16C5E, LOG_$READ 0xE17800.  The `link` is not
    omitted.
  - **Implemented:** `dpc-m68k` rounds the *total* up to a multiple of 4
    (`M68k::link_size`): 1 byte (TST_LOCK's boolean result variable) gives
    `#-4`, 0 gives `#0`.  This fits every site above, but it is the
    Uncertain choice just described: a routine with several small locals
    will tell the two rules apart.
- **Register save:** straight after `link`, only the callee-saved
  registers the routine uses.  The instruction depends on how many there
  are (Verified, sweep of every `link` at 0xE00800..0xE88834; details and
  counts in section 12.1):
  - two or more: `movem.l <regs>,-(SP)` (0xE5C698 `movem.l D2-D7/A2-A4,-(SP)`);
  - exactly one: `move.l D2,-(SP)` (PROC1_$SET_ASID 0xE148FC) or
    `pea (An)` (0xE14922 A2, 0xE14BBE A5).  No `movem` with a one-register
    mask follows a `link`;
  - none: nothing (0xE16C5E).
  - **Implemented:** `crates/dpc-m68k/src/frame.rs`.
- **Callee-saved registers:** D2–D7, A2–A4 (reference 5.5.8 `nosave`), and
  A5 and A6.
- **Scratch registers:** D0, D1, A0, A1.  A1 also carries the static link
  (section 6).
- **Register restore** is done from the frame, not by popping:
  `movem.l (-N,A6),<regs>` (0xE13602, 0xE177F6), or for one register
  `move.l (-N,A6),Dn` / `movea.l (-N,A6),An` (0xE14916, 0xE14BD2), then
  `unlk A6; rts`.  **N = link displacement + 4 × (number of saved
  registers)**: the save area sits directly below the locals (Verified:
  0xE17778 `link #-0x10` + 3 registers → `(-0x1c,A6)`; 0xE1359C `link #-4`
  + 3 → `(-0x10,A6)`; OSINFO_$GET_MMAP `link #-0x18` + 9 → `(-0x3c,A6)`
  at 0xE5C9F6; MAC_OS_$OPEN `link #-0x28` + 10 → `(-0x50,A6)` at
  0xE0B452; and every one of the 1162 `movem` and 309 single-register
  restores paired with a `link` + save in the sweep of section 12.1).
- **Procedures without callee-saved registers:** just `unlk A6; rts`
  (0xE16C7C).
- **Uncertain:** whether the compiler ever omits `link`.  Every compiled
  routine sampled has one, including empty ones.  The KBD_$PUT comment in
  `../source/domain_os/kbd/put.c` says "no link frame".  That needs
  checking: it may be hand assembly, or a degenerate case.

---

## 3. Function results (Verified / reference)

- **Scalars** come back in **D0**, at the size of the type:
  - boolean, char: D0.b
  - integer/integer16, enumerations: D0.w
  - integer32: D0.l
- Pascal boolean results: PROC1_$TST_LOCK 0xE148DE
  `btst D0,D1; sne D1; move.b D1,D0`.  The upper bytes of D0 are left as
  garbage.
- `clr.l D0` for a zero result: IO_$ALLOC 0xE1A9CC.
- **Pointer results** come back in **A0** by default (reference 5.5.10
  `d0_return`, 5.5.11 `a0_return`).  The image does this:
  - MST_$MAPS
  - NET_$FIND_HANDLER
  - TIME_$RTE_INT / TIME_$VT_INT end with `movea.l #0,A0`
  - See `ARCH_RESULT_A0` in `../source/domain_os/arch/m68k/arch.h`.
- **Procedures** leave D0/A0 as whatever the body left.  Callers never read
  them.
- **Result assignment as the last statement (Verified, one site):** in
  PROC1_$TST_LOCK the body is the single statement `f := x in s`.  The
  value is computed into a scratch register and moved straight into D0
  (0xE148E0 `move.b D1,D0`), followed by the epilogue.  The frame has
  room for a result variable (`link #-4`), but the slot is never written
  or read.  `dpc` implements exactly this case: a result assignment that
  is the last statement goes to the result register.
  **Uncertain (rejected by `dpc` for now):** what the compiler does when
  the result is assigned before the last statement, assigned on several
  paths, or read back (stored to the frame slot and reloaded before the
  return?).  Find a function with an early or conditional assignment in
  the image before implementing it.
- **Tail wrapper:** a wrapper `F := G(args)` compiles to the call followed
  directly by the epilogue, with no move to D0 (ACL_$IS_SUSER,
  PROC2_$GET_ASID).  So from the wrapper alone you cannot tell a procedure
  from a function.  See `svc-d0-result-trap.md`.
- **Uncertain:** how results larger than 4 bytes (records, sets, reals) are
  returned.  This is not yet observed in the kernel.

---

## 4. Module static data and A5 (Verified; when A5 is set: partly Uncertain)

Static data includes module-level `var`, `static` variables and `define`d
variables.  Each module's static data is a data segment, addressed off
**A5**.

- **Exported entry points** load A5 with the **absolute** address of their
  module's data segment, and save and restore the caller's A5:
  - **`pea (A5)` form** (only A5 to save): PROC1_$GET_LOADAV 0xE14BBE
    `pea (A5); lea 0x00E254E8,A5`, where 0xE254E8 is `D E254E8 PROC1_`.
    The restore is `movea.l (-4,A6),A5` before `unlk` (0xE14BD2).  Also
    LOG_$READ 0xE17804 (A5 = 0xE2B280 `D LOG_`), XNS_IDP_$GET_STATS
    0xE18FDA (0xE2B314 `D XNS_IDP`) and SMD_$BUSY_WAIT 0xE1D8A2
    (0xE2E060 `D SMD`).
  - **`movem` form** (other registers saved too): MAC_OS_$OPEN 0xE0B24A
    `movem.l D2-D7/A2-A5,-(SP); lea 0x00E22990,A5`, where 0xE22990 is
    `D E22990 MAC_OS`.
  - With locals, the saved A5 sits just below them: TONE_$ENABLE
    `link #-8`, restore from `(-0xC,A6)` (0xE1AD0E).
- **Module-local workers and nested routines** do **not** set A5.  They
  inherit it from their caller (0xE17778, LOG_$READ's worker, reads
  `(0x14,A5)`).  Reported: a nested procedure's A5 accesses use the
  enclosing routine's A5 (`reverse-engineer/` notes, "find the lea in the
  caller").
- **Other modules' data** is reached with absolute addresses
  (`movea.l 0x00E1EAC8,A0` in PROC1_$TST_LOCK; `move.l 0x00E23C98,D2` in
  PMAP_$PURIFIER_L 0xE13AF6).
- **When a routine sets A5 (survey, Verified counts; rule partly
  Uncertain).**  Every `link` in the map's `I` segments other than
  `*_ASM`, split at `link`s, in modules that have a `D` segment of the
  same name; "uses" means an `(..,A5..)` operand or A5 passed as an
  argument, directly or through a module-local callee that does not set
  A5 itself.  The scripts were ad hoc and not kept (re-check sites with
  `tools/imgdis.sh`).
  - 585 map-named routines set A5 and use it.  No map-named routine was
    found using A5 without setting it (the 18 apparent cases are
    artefacts of the split: FILE_$MK_IMMUTABLE 0xE5DBC0 and PROC1_$SET_VT
    0xE1495C have no A5 operand of their own).
  - 204 map-named routines set A5 and never use it: XNS_IDP_$GET_PORT_INFO
    0xE18FB8 (one status store), SMD_$BUSY_WAIT 0xE1D89E, SMD_$LITES
    0xE1D8B8, AST_$SET_TROUBLE 0xE071EA.  251 neither set nor use it:
    TIME_$WAIT 0xE1650A, PROC1_$TST_LOCK 0xE148CA.  So the rule is not
    "only routines that use the data", and not "every exported routine of
    a map module with a data segment" either.
  - Within one map segment the routines that set A5 come in runs
    (PROC1_ 0xE1471C: ten that do not, then ten that do, then two that
    do not, ...), and every segment loads one A5 value only.  This fits "every exported
    routine of a *compilation unit* that has static data sets A5", if a
    map segment merges several compilation units that share one data
    block; the unit boundaries are not known, so this stays Uncertain.
    Supporting: the routines that do not set A5 never address their
    segment's data off A5; four address it absolutely
    (AUDIT_$SUSPEND 0xE70DB6, AUDIT_$RESUME 0xE70DD6, REM_NAME_$REGISTER_SERVER
    0xE4A4AE, DISK_$DIAG_IO 0xE6BC18), and LOG_$INIT 0xE30054 pushes the
    LOG_ data address as `move.l #0xE2B280,-(SP)`, as other units do
    with an `extern`.
  - Routines the map does not name: 363 do not set A5 (183 of them use
    it, inherited, like 0xE17778); 23 set it, but nearly all of those are
    also called with `jsr` from other modules (0xE14B12 from 0xE2F9EC,
    0xE4194C from 0xE7295A, ...), i.e. exported routines the map omits.
    A few set it although only `bsr` reaches them (RING 0xE76048,
    0xE7667C): Uncertain.
  - **Implemented** (`crates/dpc-ir/src/lower.rs`, `loads_data_base`;
    `crates/dpc-m68k/src/isel.rs`, `load_data_base`): an exported routine
    that addresses the unit's static data sets A5, as the first
    instruction after the register save; an `internal` routine never
    does (it inherits A5).  An exported routine that does not address the
    data, in a unit that has some, is rejected ("not settled").  A5 is
    saved like any callee-saved register (section 12.1): `pea (A5)` alone,
    in the `movem` otherwise.  Byte-identical in `make conform`:
    PROC1_$GET_LOADAV (`tests/conformance/proc1_get_loadav.pas`) and
    PROC2_$GET_BOOT_FLAGS 0xE41B56 (`proc2_get_boot_flags.pas`).
- **Reference oddity:** the OCR of section 5.5.8 says A5 "holds the pointer
  to the current stack area" and A6 "the address of the current data area".
  In the image, A6 is the frame pointer and A5 the data base.  Treat the
  manual text as garbled or wrong there, and log it in
  `docs/reference-clarifications.md` if the owner rules on it.

---

## 5. Data representation (reference chapter 3 + image)

| Type | Size | Notes |
|---|---|---|
| `integer`, `integer16` | 2 | signed; **`integer` is 16 bits** (ref 3.2.1) |
| `integer32` | 4 | |
| subrange | size of base type (ref 3.8.1) | 2 bytes up to `0..65535` (ref 5.3.4.1 p. 5-13: `0..65535` and integer are "both 16-bit integers"; Table 7-2 p. 7-30: `unsigned short`); 4 bytes once more than 16 bits are needed (3.4 p. 3-10); packed records differ |
| `boolean` | 1 | true = 0xFF, false = 0x00 (ref 3.5.3); tested with `tst.b`/`bmi`/`bpl`, i.e. **signed** byte |
| `char` | 1 | |
| enumeration | 2 | ordinal in a word (ref 3.7.1) |
| pointer, `univ_ptr` | 4 | |
| set | ⌈(max ordinal + 1)/16⌉ words | ref 3.9.3; see 5.1.  The high end of the base type decides, whatever the low end: `set of 80..170` is 11 words, `set of 'a'..'z'` 8 words (p. 3-17); checked by `crates/dpc-sema/src/tests/layout.rs` |
| record | ref 3.10.5 (alignment rules: "natural" vs default word alignment; `aligned`/`unaligned` records) | **Reported: default word alignment** (ref 3.10.5.5: fields longer than a byte on word boundaries, record size even).  `route_$port_stats_t` (`../source/domain_os/route/route.h`) has longwords at 0x2 and 0x6, which natural alignment cannot produce: ROUTE_RTWIRED code `addq.l #1,(0x6,A2)` 0xE87660, `addq.l #1,(0xA,A2,D1.l)` 0xE8765A (seen; that A2 is the record start is the C's claim).  `proc1_t` (`proc1/proc1.h`) also matches word rules.  Field offsets in `_Static_assert`s remain ground truth |
| arrays | 1-based is common in the kernel; element N at base + (N-lo)*size | |

**Signedness (implemented in `dpc-ir`, `lower.rs` `sign_of`).**  Every IR
value carries a width and a sign (`dpc_ir::Scalar`).  Verified for the
2-byte `0..65535`-style subrange: 0xE17778 (C `uint16_t` parameters)
zero-extends with `clr.l D1; move.w D0,D1` (0xE177A6..0xE177AA), compares
with `cmpi.w #0x400,D1; bcc` (0xE1779A) and `bls` (0xE177BA), and halves
with `lsr.w #1` (0xE177D8).  `integer`/`integer32` signed (ref 3.2.1);
boolean signed (the `tst.b`/`bmi` tests above); char, pointers and sets
unsigned.  **Uncertain:** enumerations (taken as unsigned) and
non-negative `integer16` subranges such as `0..100` (taken as signed, like
their host); no compare or widening of either has been catalogued.

### 5.1 Sets (Verified)

- A set is a **big-endian multi-word integer**.  Member *k* is bit *k* of
  that integer, so member 0 is bit 0 of the **last** byte (ref 3.9.3,
  Figure 3-6).
- **Membership test, inline:** MST_$ALLOC_ASID 0xE42D5C..0xE42D64
  `moveq #63,D1; sub.w D2,D1; lsr.w #3,D1; btst D2,(0,A2,D1.w)` on a
  64-element set.  The byte offset is `(size_bits-1 - k) >> 3`, and the bit
  is `k mod 8` (`btst` with a byte destination uses only the low 3 bits).
- **Set that fits in a register:** `btst Dn,Dm` on a longword
  (PROC1_$TST_LOCK 0xE148DC, `resource_locks_held`).
- **Evaluation order of `k in p^.f` (Verified, one site):** PROC1_$TST_LOCK
  0xE148CE..0xE148DE.
  1. The set operand's *address* first: `movea.l PROC1_$CURRENT_PCB.l,A0`
     (0xE148CE).
  2. Then the element into a data register: `move.w (8,A6),D0` (0xE148D4).
  3. Then the set's value into the next data register: `move.l (0x40,A0),D1`
     (0xE148D8).
  4. `btst D0,D1` (0xE148DC), and `sne D1` (0xE148DE): the boolean is
     materialised in the **set's** register, not in the element's (D0 is
     free again at that point, but is not used).
  5. As a function result, `move.b D1,D0` (0xE148E0; section 3).

  `dpc` reproduces this order for any `in` on a one-longword set variable
  (`crates/dpc-m68k/src/isel.rs`, `set_member_long`).  **Uncertain:**
  whether "address of the right operand before the value of the left
  one" is a general rule of the compiler's binary operators or specific
  to `in`; and the code for one-word sets held in a register (not seen
  yet; `dpc` rejects them).
- **Set constants** sit in data, for example the two 32-byte `set of char`
  tables at the start of the OLD_DIR data segment 0xE7FD24 (Reported:
  `name-dir-subsystems.md`).
- **Uncertain:** the code for set union, intersection and difference, and
  for large `+ [x]` / `- [x]` on sets in memory.

### 5.2 Records and arrays (Verified / Reported)

- **Record assignment (survey of `(Am)+,(An)+` copies in compiled code,
  0xE00800..0xE88834; Verified counts).**  The source address goes into
  one address register, then the destination address into another (`lea
  (d,An)`, `lea (d,A6)`, `lea (A5)` for offset 0 of the data area rather
  than `movea.l A5,An`; a by-address parameter's slot with `movea.l
  (d,A6),An`), then:
  - **10 to 18 bytes: unrolled.**  One `move.l (A0)+,(A1)+` per longword,
    a `move.w` for a final word.  Runs of 2, 3 and 4 `move.l`: 15, 59 and
    42 sites; every run of 2 is followed by a `move.w` (10 bytes,
    0xE307F8..0xE30802).  12 bytes: PROC1_$GET_LOADAV 0xE14BC6..0xE14BD0
    `lea (A5),A0; movea.l (8,A6),A1; move.l (A0)+,(A1)+` ×3; 14 bytes:
    0xE419B0..0xE419BE.
  - **20 bytes and more: a `dbf` loop.**  237 loops of one `move.l
    (Am)+,(An)+`; the smallest constant count is `moveq #4` (5
    longwords).  AST_ 0xE01494..0xE014A2: `lea (0xc,A2),A0; lea
    (-0x98,A6),A1; moveq #35,D0; movea.l A0,A0; move.l (A0)+,(A1)+;
    dbf` (36 longwords).  The count is loaded after both addresses in 118
    loops (26 load it earlier, mostly run-time counts); a count above 127 is
    `move.w #n,Dn` (0xE3B97A `move.w #130,D0`); a final word follows the
    loop as `move.w (Am)+,(An)+` (37 loops, e.g. 0xE19DA2..0xE19DAA, 22
    bytes).
  - **Loop-head alignment: Uncertain.**  Of the 240 one-`move.l` `dbf`
    copy loops in a linear sweep of 0xE00800..0xE88834, 63 have the 0x2048
    filler (all then 4-aligned), 127 are 4-aligned without it and 50 start
    at 2 mod 4 with no filler, in compiled code too: 0xE1155E (routine
    0xE11548, `link`, `movem`, `lea 0xE248FC,A5`; `moveq #6,D0` at
    0xE1155C), 0xE496C2, AST_ 0xE02482 (the same routine's loop
    at 0xE023D4 is 4-aligned).  Same modules, same register shapes; no
    rule found.
    `dpc` always aligns (the AST_ 0xE0149E form).
  - **6 and 8 bytes: displacement moves.**  No `(An)+` copy of exactly 6
    or 8 bytes exists; pairs such as `move.l (0x30,A5),(-8,A6); move.l
    (0x34,A5),(-4,A6)` (0xE4AD5C) and many `move.l`/`move.w` pairs of
    6-byte objects are found instead.
  - **Displacement moves of 12 and 16 bytes** are rare: XNS_IDP_$GET_STATS
    0xE18FE6 `move.l (A5),(A0); move.l (4,A5),(4,A0); move.l
    (8,A5),(8,A0)` and 0xE1ABEA (16 bytes) against 59 + 42 `(An)+`
    copies.  Taken to be field-by-field assignments in the source
    (Uncertain: the image cannot tell; the C of XNS_IDP_$GET_STATS
    assigns three named fields).
  - **Implemented** (`crates/dpc-m68k/src/isel.rs`, `copy`): records and
    arrays that are not 1, 2 or 4 bytes, 10 bytes and more, with A0/A1;
    the source or destination in the data area, in the frame, or the
    object a pointer in the frame points to.  Rejected: 6 and 8 bytes
    (which operand the original formed first is not catalogued), another
    unit's object (`movea.l #abs,A1` at 0xE336E2, one site), a field
    through a pointer, statements of bodies with several statements.
    Records and arrays of 1, 2 or 4 bytes are moved as scalars
    (Uncertain: not catalogued).
- **1-based arrays:** the compiler folds the `-1` into the displacement,
  `(-2,A0,Dn.w)` (Reported: AUDIT_$SUSPEND 0xE70DC8) or the `lea (2,An)` /
  `(-2,An)` pair in 0xE17778's word-copy loop (0xE177E0..0xE177F2).
- **Index scaling:**
  - Multiply by 2 is done with `add.w Dn,Dn`; by 4 with `lsl.w #2` or
    `lsl.l #2` (0xE0B26C, 0xE13942).
  - Other sizes use `mulu.w #size` (`proc2/suspend.c` 0xE412AE
    `mulu.w #0xE4`).
  - Or shifts and subtract: `idx*0xF8` = `(idx<<8)-(idx<<3)` (Reported:
    `vtoc-disk-layouts.md`).
  - There are no 68020 scaled-index modes in compiled code.
- **Packed records / bit fields:** Uncertain; nothing has been catalogued
  yet.

---

## 6. Nested routines (Verified at PMAP_$FLUSH)

PMAP_$FLUSH is at 0xE1376C.  Its nested routines are at 0xE1359C and
0xE1360C, and they come **before** their parent in the code.

- **Static link in A1.** The caller loads the enclosing routine's frame
  pointer into **A1** before the `bsr`:
  - From the parent: `movea.l A6,A1; bsr.w 0xE1359C` (0xE13928, 0xE13978).
  - From a sibling nested routine: `movea.l A4,A1; bsr.w 0xE1359C`
    (0xE13718), where A4 = that sibling's `(A6)` = the parent's frame.
  - The callee copies it: 0xE135A4 `movea.l A1,A2`, then reaches the
    parent's locals as `(d,A2)`.
- **Dynamic link used as the static link.** 0xE1360C is called only from its
  parent, and the parent does not load A1 (0xE13856, 0xE13950, 0xE13992).
  The callee does `movea.l (A6),A4` (0xE13616): the saved caller frame
  pointer, which here *is* the parent frame.  Also seen in
  PROC2_$SUSPEND's nested routine at 0xE41214 and in VFMT_$MAIN, RIP_$SEND
  and KBD_$RCV (C comments in `../source/domain_os`).
- **Uncertain:** the compiler's rule for choosing A1 or `(A6)`.  The
  hypothesis consistent with PMAP_$FLUSH: A1 is passed when the routine is
  reachable from anything other than its direct parent, for example a
  sibling.  The deeper-nesting chain walk has not been observed.
- **Arguments:** nested routines also take ordinary stack arguments
  (0xE1359C reads `(8,A6)`, `(0xC,A6)`, `(0x10,A6)`), with the value
  conventions of `internal` routines.
  Verified at the call site too: PMAP_$FLUSH 0xE13920..0xE1392A pushes
  `subq.l #2,SP; move.w D6,-(SP); move.l D5,-(SP); pea (A2)` (a word and a
  longword by value, one address, pad first) before `movea.l A6,A1; bsr.w
  0xE1359C`.  The reference does not say that nested routines get the
  `internal` convention; `dpc-sema` applies it on this evidence.
- **A5:** nested routines inherit A5 (section 4).
- **Order:** nested routines are emitted before their enclosing routine
  (PMAP_$FLUSH).  The same holds for module-local workers placed before
  their first user: 0xE17778 comes before LOG_$READ, and 0xE00EB0 before
  AST_$PAGE_ZERO.  **Uncertain** whether this is source order or a
  compiler rule.

---

## 7. Control flow

- **`for` loops with a known count** use `dbf`.  The count register is
  loaded with N-1 (`moveq #N-1` / `move.w #N-1`), and `dbf` runs the body N
  times (Verified):
  - 0xE00ED2 `move.w #0xFF,D0; ...; clr.l (A0)+; dbf` clears 1 KB.
  - 0xE42D50 `moveq #57,D0`: an index register counts up separately
    (`clr.w D2` ... `addq.w #1,D2`).
- **Run-time count:** `subq.w #1,Dn; bmi skip` before entering the `dbf`
  loop (0xE177DC..0xE177F2).
- **Loop-head alignment (Verified where the filler is; which loops: Uncertain):** the compiler pads with the 2-byte
  no-op `movea.l A0,A0` (0x2048) so that a loop head starts on a 4-byte
  boundary.  479 of the 481 `0x2048` words in the image's kernel text are
  followed by a 4-aligned instruction.
  - Before a `dbf` loop: 0xE00ED6, 0xE0149E.
  - The boundary is **absolute** (relative to the 4-aligned module code
    base), not relative to the routine: PKT_$RECENTLY_MISSING starts at
    0xE128BA (2 mod 4) in module PKT (code 0xE1202C), and its filler at
    0xE128DA puts the loop head on 0xE128DC.  `dpc --fragment-origin`
    reproduces this for conformance units that start mid-module
    (tests/conformance/README.md).
  - At a branch target: 0xE1CE42.
  - At the end of a module's code, so the next module starts 4-aligned:
    0xE1A9D2 before OS_TERM 0xE1A9D4, and 0xE1AD16 before BITPAD 0xE1AD18.
  - **Uncertain:** exactly which labels get aligned.  Not every loop
    head: of the `dbf` loops a linear sweep finds (hand assembly
    included), 196 have the filler, 423 are 4-aligned without it and 198
    start at 2 mod 4 with none, compiled code among them (0xE1155E,
    section 5.2).  The other branch targets are not yet characterised.
- **`case` statements** become a bounds check plus a jump table of word
  offsets (Verified: OSINFO_$GET_MMAP 0xE5C6C0..0xE5C6D6):
  `cmpi.w #n,D0; bcc default; add.w D0,D0; move.w (tbl,PC,D0.w),D0; jmp (tbl,PC,D0.w)`
  (encoded `303b 0006` / `4efb 0002`), with the table of word offsets
  (relative to the table start) directly after the `jmp`.  More examples:
  - MAC_OS_$OPEN 0xE0B3AE
  - KBD_$RCV 0xE1CCFE
  - TTY_$I_RCV (19 entries)
  
  **Uncertain:** the threshold below which the compiler emits a compare
  chain instead.  Verified: the module-local key translator at 0xE1CC64
  (sparse labels 0x91..0xC0) is a `cmpi.w #k,D0; beq` chain, with labels
  tested in what looks like source order, not sorted order.
- **Branch sizes:** `.s` when the target is in reach, otherwise `.w`.  No
  `.l` branches appear (68010).
- **Boolean tests:** `tst.b` then `bmi`/`bpl` (signed byte: 0xFF is
  negative); `sne`/`seq` to materialise a value.

---

## 8. Arithmetic

- **16x16 multiply:** `mulu.w`/`muls.w` inline.
- **Word divide or mod** (32/16 → 16) is inline: `divu.w`/`divs.w`, with
  `swap` for the remainder (Verified: PMAP_$PURIFIER_L 0xE13AFC
  `divu.w #50,D2`).  On overflow the register keeps the dividend.
- **32-bit multiply, divide and mod** call **M$ARITH** (Verified: 0xE07FEE
  `jsr M$OIU$WLW` for `d2 mod 11`, with a word-slot divisor and a pad).
  The routines (map, `../source/domain_os/math/sau2/arith.s`):

  | Routine | Address |
  |---|---|
  | M$MIS$LLL | E0ABD4 |
  | M$MIU$LLW | E0AC02 |
  | M$MIU$LLL | E0AC1C |
  | M$MIS$LLW | E0AC44 |
  | M$DIU$LLW | E0AC68 |
  | M$DIS$LLW | E0AC84 |
  | M$OIU$WLW | E0ACC0 |
  | M$DIS$LLL | E0ACD8 |
  | M$DIU$LLL | E0AD0C |
  | M$OIS$WLW | E0AD54 |
  | M$OIS$WWL | E0AD68 |
  | M$OIS$LLL = M$OIU$LLL | E0AD7C |

  Name decoding (inferred from the frames in arith.s, **not documented**):
  MI = multiply, DI = div, OI = mod; S/U = signed/unsigned; then result,
  left operand and right operand sizes (L/W).  Arguments use the word-slot
  ABI; the result is in D0; the caller pops.
- **Uncertain:** the rule that picks inline `divu.w` or an M$ call.  It
  probably depends on the static operand types, for example
  integer32 div integer16 constant → `divu.w`.  Catalogue more call sites
  before coding it.
- **No libgcc-style helpers.** M$ARITH is the whole integer runtime seen so
  far.
- **Floating point:** not used in the kernel as far as is known.  The
  reference documents FPU/compiler variants (ref 6.2, Appendix F).  Out of
  scope until needed.

---

## 9. Status returns and error exits (Reported)

- Kernel routines take a `status_$t` (32-bit) `out` parameter by address
  and clear it on entry:
  - IO_$ALLOC 0xE1A9C6 `movea.l (0x14,A6),A0; clr.l (A0)`
  - SMD_$BUSY_WAIT 0xE1D8AA
  - MMAP_$ALLOC_CONTIG 0xE0D8FC
- Error codes are stored with `move.l #imm,(An)`, for example
  `move.l #0x003A0001,(A0)` at 0xE0B28C.  Error paths branch to a shared
  epilogue (`bra.w`).
- Status code names come from the real text tables
  (`~/src/domainos-archeology/stcodes`, `node stcode.js stcode.db.10.2 <hex>`).
  Never invent a `status_$` name.
- **`abnormal`, `noreturn`, cleanup handlers:** the kernel has cleanup
  handlers (`../source/domain_os/proc2/cleanup_handlers_internal.c`,
  FIM_$CLEANUP).  How the compiler treats `options(abnormal)` around them
  is **Uncertain**.  The reference (5.5.6, 5.5.9) only says that
  optimisation is suppressed and that the return sequence after a
  `noreturn` call is dropped (compare CRASH_SYSTEM call sites; the
  AREA_$DEACTIVATE_ASTE quirk in `../source/docs/original-quirks.md`).

---

## 10. Interrupt priority in compiled code (Verified; mapping to built-ins from the reference)

Compiled Pascal routines contain `ori.w #0x700,SR` … `andi.w #0xF8FF,SR`.
For example TIME_$SET_TIME_OF_DAY, 0xE167EC → 0xE1686A, and PROC1_$RESUME,
PROC1_$SUSPEND, PROC1_$SET_PRIORITY, SMD_$LOCK_DISPLAY.

- **Source spelling:** the reference's systems-programming built-ins
  (Appendix E, Table E-1; also Table 4-7), all privileged:
  - `disable;` turns off the interrupt enable: `ori.w #0x700,SR`.
  - `enable;` turns it back on: `andi.w #0xF8FF,SR`.  This **forces IPL 0**;
    it does not restore the previous level, exactly as the image shows.
  - `oldsr := set_sr(newsr);` reads SR and installs a new value.  The
    reference gives the expansion as `move.w SR,D0; move.w newsr,SR;
    move.w D0,oldsr`.  Kernel `move SR,-(SP)` / `move (SP)+,SR` pairs
    are the real save/restore; check each against set_sr before assuming
    it is compiled (some are hand assembly).
- The mapping of `disable`/`enable` to these exact instructions is inferred
  from the image plus the reference's description; confirm the encodings
  against more sites when the codegen agent implements them.

---

## 11. Sections and layout (Verified from the map; mechanism partly Uncertain)

- **Reference defaults:** code goes to `.text` and data to `.data`
  (ref 5.7.1, 2.2.1).  A module heading can name both:
  `module name, codesect, datasect;` (ref 7.1.1).  A routine can override
  them with `[section(c,d)]` (ref 5.7.1), and variables with
  `var (sect)` (ref 3.14).
- **The SR10.2 kernel link** (map) has output sections:
  - `WIRED_PROC` (I, 0xE00800) and `WIRED_DATA` (D, 0xE1DC00)
  - `OS_INIT_PROC` / `OS_INIT_DATA`
  - `.TEXT` (I, 0xE38000) and `.DATA` (D, 0xE78400)
  - `PROCEDURE$` (I, 0xE71D7C) and `DATA$` (D, 0xE85708)
  - `RING_PROC` / `RING_DATA`, `ETHER_PROC` ...

  Inside each, there is one contribution per module, in link order, named
  after the module (`I E00BE8 AST_`, `D E1DC80 AST_`).  So a module's code
  and its data land in **different** output sections, each in module
  order.  The per-module **code** contribution is 4-byte aligned and padded
  with `0x2048` (section 7).
- **Uncertain:** which source construct produced `WIRED_PROC`/`WIRED_DATA`
  (module heading section names are the likely candidate), and why
  `PROCEDURE$`/`DATA$` exist besides `.TEXT`/`.DATA`.  They may be another
  compiler's defaults or older defaults.
- **Constants:** literals passed by address sit in the code section, next
  to the routine (section 1.5).  Constant UIDs are 8-byte cells in the code
  section (`UID_$NIL` 0xE1737C, … in `I E1737C UID_LIST`).
- **External names** keep `$` (`AST_$PAGE_ZERO`, `M$OIU$WLW`) and are
  upper-case in the map.  Domain Pascal is case-insensitive (ref 2.1.6), so
  the emitter must upper-case global symbols.  **Uncertain:** whether the
  object format folded case or the compiler did.
- `internal` routines and `static` variables are local symbols (ref 5.5.4,
  7.2).  The map does not list them.  That is why 0xE00EB0 and 0xE17778
  have no map name.
- **Module data blocks (Verified from the map):** all 161 `D` segments
  start at a multiple of 4 and have a size that is a multiple of 4
  (PROC1_ 0xE254E8, size 0xCC4).  Whether the compiler or the binder
  rounded is not known.
- **What `dpc` emits for a unit's static data (our convention):** one
  zero-filled block in the NOBITS section `.bss.moddata.<UNIT>` (unit
  name upper-cased), `.balign 4`, size rounded up to 4, starting at the
  local label `.Ldata`; `lea (.Ldata).l,%a5` is relocated against that
  section.  For a kernel unit the upper-cased unit name is the map's `D`
  segment name (`PROC1_`, `PROC2`).  Only the `.bss.moddata.` prefix is
  `../source`'s: there `<name>` is the C object of a `MODULE_DATA_DEFINE`
  site (`.bss.moddata.PROC1_$DATA`; PROC2's `D` segment 0xE7BE84 is
  `PROC2_$UNWIRED_DATA`, with initial contents, so `.moddata.`), and
  `tools/gen_layout_ld.py` keys a block by that C site's address
  literal, not by section or segment name.  A dpc unit's section would
  get no key in `../source`'s link; how a Pascal unit's block replaces
  (or becomes) the C block in a link test is not designed yet.  `define`d
  variables are global symbols inside the block; other statics have no
  symbol.  `tools/conform.py` places the section with the manifest's
  `--section-addr .bss.moddata.<UNIT>=0x<D segment>`.  Not supported
  yet: initial values (a progbits block), data sections named in the
  module heading or in `var (name)`.

---

## 12. Register allocation and other "unoptimised" artefacts (Verified)

These are not optimisations.  They are part of the output we must
reproduce.

- **Parameters loaded into registers at entry.** Pointer and value
  parameters are loaded into callee-saved registers right after the
  `movem` (OSINFO_$GET_MMAP 0xE5C69C..0xE5C6AC: `move.l (8,A6),D3`, ...,
  pointers kept in **D** registers and moved to an A register only when
  used, `movea.l D6,A0`).  In routines without calls, into scratch
  registers (TIME_$Q_INIT_QUEUE 0xE16C62..0xE16C6A).  When this happens
  and when it does not: section 12.2.
- **Redundant moves** are kept: `lea (A5),A0` (0xE14BC6); `movea.l A0,A0`
  as filler (section 7).
- **Indirect operand with no displacement (Verified):** `(An)`, not
  `(0,An)`: LOG_$READ 0xE17814 `move.w (A0),-(SP)` (`3f10`).
- **Scratch register order (Verified, small sample):** data values take
  D0 then D1, pointers A0 (PROC1_$TST_LOCK 0xE148CE..0xE148D8;
  TIME_$Q_INIT_QUEUE 0xE16C62..0xE16C6A `move.b (8,A6),D0; move.w
  (0xa,A6),D1; movea.l (0xc,A6),A0`).  `dpc` allocates D0, D1, A0, A1 in
  that order and rejects expressions that need more
  (`crates/dpc-m68k/src/regs.rs`).  Callee-saved registers are handed out
  only on explicit request (`Regs::alloc_held`, lowest free first), for
  values that must survive a call; see 12.1 for what is and is not known
  about when the original used them.
- **Re-evaluation:** the same expression is reloaded rather than kept in a
  register (0xE135DA/0xE135E2 reload `(8,A2)`).
- **The order of evaluation of arguments** follows the push order (right to
  left), with address temporaries computed in between (0xE135D0..0xE135EE).
- The kernel's `.TEXT` shows strength reduction (shifts for constant
  multiplies), constant folding and common-address reuse.  The compiler
  *did* optimise a little.  Reproduce what is seen; invent nothing.

### 12.1 Register usage in the next conformance targets (Verified at the addresses given)

Disassembled with `tools/imgdis.sh`; semantics from the C in
`../source/domain_os`.  "Save" is what follows `link`; "restore" what
precedes `unlk`.

| Routine | Frame | Save / restore | Registers |
|---|---|---|---|
| TIME_$Q_INIT_QUEUE 0xE16C5E (`time/q_init_queue.c`) | `link #0` | none | All three parameters loaded at entry, before any store: byte `(8,A6)`→D0, word `(0xa,A6)`→D1, pointer `(0xc,A6)`→A0 (0xE16C62..0xE16C6A); stores through A0 follow. Scratch only. Byte-identical in `make conform` (`tests/conformance/time_q_init_queue.pas`, with TIME_$Q_FLUSH_QUEUE; rule in 12.2). |
| PROC1_$GET_LOADAV 0xE14BBA (`proc1/get_loadav.c`) | `link #0` | `pea (A5)` 0xE14BBE / `movea.l (-4,A6),A5` 0xE14BD2 | `lea 0xE254E8,A5` right after the save; `lea (A5),A0` (source, offset 0 of the data), `movea.l (8,A6),A1` (destination), three `move.l (A0)+,(A1)+`. Byte-identical in `make conform` (`tests/conformance/proc1_get_loadav.pas`). |
| module-local 0xE00EB0 (`misc/zero_page.c`) | `link #-0xc` (frame never touched in the body) | none | No register holds a value across the two `jsr`s; the argument `(8,A6)` is re-read for each call (0xE00EBE, 0xE00EDE). Loop: A0 pointer (`movea.l #imm,A0`), D0 `dbf` counter. |
| AST_$PAGE_ZERO 0xE00EEC (`ast/page_zero.c`) | `link #0` | none | No registers at all: constants pushed, `(8,A6)` pushed, three calls. |
| module-local 0xE17778 (`log/read_internal.c`) | `link #-0x10` | `movem.l D2-D3/A2,-(SP)` (`48e7 3020`) 0xE1777C / `movem.l (-0x1c,A6),D2-D3/A2` (`4cee 040c ffe4`) 0xE177F6 | No calls, yet callee-saved registers are used. Parameters at entry: word `(0xc,A6)`→D0, word `(0xe,A6)`→D1, pointer `(0x10,A6)`→A0, held in these *scratch* registers for most of the routine. Temporaries then take D2, D3 (0xE177A0 `moveq #-2,D2`; 0xE177A8..0xE177AE). The pointer `(8,A6)` is loaded into **A2** only at 0xE177CC, although A1 is free there; A1 is taken next (0xE177DA) and the copy loop uses A0/A1. |
| LOG_$READ 0xE17800 (`log/read.c`) | `link #0` | `pea (A5)` 0xE17804 / `movea.l (-4,A6),A5` 0xE17820 | `lea 0xE2B280,A5`; A0 scratch to dereference the by-address `max_len` (0xE17810). The final `bsr` is followed by the A5 restore, no pop. |
| nested 0xE1359C of PMAP_$FLUSH | `link #-4` (frame unused) | `movem.l D2/A2-A3,-(SP)` (`48e7 2030`) 0xE135A0 / `movem.l (-0x10,A6),D2/A2-A3` (`4cee 0c04 fff0`) 0xE13602 | Static link `movea.l A1,A2` at entry (0xE135A4); parameter `(0xc,A6)`→D2 at entry (0xE135A6), **held across** `jsr` 0xE135BC and used after it (0xE135D4, 0xE135FA). Inside one argument list the pointer `(8,A2)` is reloaded into A0 (0xE135DA), then into **A1** (0xE135E2), and a pointer read through it goes to **A3** (0xE135E6): registers are not reused within the statement (A2 is busy). |
| PROC1_$SET_ASID 0xE148F8 | `link #0` | `move.l D2,-(SP)` 0xE148FC / `move.l (-4,A6),D2` 0xE14916 | Parameter `(8,A6)` → D2 at entry, used twice (a store and a push) before the one `jsr`, not after it. |
| OSINFO_$GET_MMAP 0xE5C694 | `link #-0x18` | `movem.l D2-D7/A2-A4,-(SP)` (`48e7 3f38`) 0xE5C698 / `movem.l (-0x3c,A6),D2-D7/A2-A4` (`4cee 1cfc ffc4`) 0xE5C9F6 | Parameters at entry: `(8)`→D3, `(0xc)`→D4, `(0x10)`→A2, `(0x1c)`→D5, `(0x20)`→D6 (0xE5C69C..0xE5C6AC); pointers kept in D registers and moved to A0/A1 when used (`movea.l D6,A0`, `movea.l D3,A1`). D2 is a temporary (0xE5C76C, 0xE5C77A, 0xE5C7BC). |
| MAC_OS_$OPEN 0xE0B246 | `link #-0x28` | `movem.l D2-D7/A2-A5,-(SP)` (`48e7 3f3c`) 0xE0B24A / `movem.l (-0x50,A6),D2-D7/A2-A5` (`4cee 3cfc ffb0`) 0xE0B452 | A5 saved inside the `movem`, then `lea 0xE22990,A5`; parameters `(8)`→D6, `(0xc)`→D7. |
| TONE_$ENABLE 0xE1ACE8 | `link #-8` | `pea (A5)` / `movea.l (-0xc,A6),A5` 0xE1AD0E | Saved A5 directly below the 8 bytes of locals. |

**Sweep (Verified, a script over 0xE00800..0xE88834, every `4e56`
`link.w` followed by a save):**

- 1170 `link` + `movem.l <list>,-(SP)`; masks have 2 to 10 registers, never
  one.  For 1162 of them the next `movem.l (d16,A6)` restore has the
  bit-reversed mask and `d = -(link + 4n)`.  The 8 others pair with a
  restore whose mask differs, i.e. belong to a routine with no restore of
  its own (not counterexamples).
- 685 of the `movem` masks include A5 (A5 saved inside the `movem`).
- Single-register saves: `move.l D2,-(SP)` ×34 (only D2 seen), `pea (A2)`
  ×92, `pea (A5)` ×183.  Every one is restored from `(-(link+4),A6)` with
  `move.l`/`movea.l`.  `pea (A5)` is never followed by a `movem`.
- Independent re-check (review of the change that added this section),
  over map-named routine starts in `I` segments other than `*_ASM` only:
  1315 starts; 826 `link` + `movem` (2 to 10 registers, none with one),
  819 of them with the first restore before `unlk` at `-(link + 4n)` and
  the bit-reversed mask, the other 7 having no restore of their own
  (e.g. DXM_$HELPER_COMMON 0xE171E4 falls into the next routine's
  `unlk`); 241 single-register saves (`move.l D2` ×16, `pea (A2)` ×50,
  `pea (A5)` ×175), every one restored from `(-(link+4),A6)`; 651 masks
  with A5; no `pea (A5)` followed by `movem`.  Scanning every `4e56` in
  those segments gives 1169 / 1161 / 684 and the same single-register
  counts as above.

**What this settles (implemented in `crates/dpc-m68k/src/frame.rs`):**
the save instruction by register count, the save area directly below the
`link` area, restore offsets, and A5 as one more register of the save set.

**What it does not settle (Uncertain; the "calls" and "loops" beads must
look before coding):**

- *When* the compiler puts a value in a callee-saved register (more
  data in section 12.2).  It is not
  "only across calls": 0xE17778 has no call and uses D2/D3/A2.  It is
  not "scratch first, then callee-saved" either: at 0xE177CC A2 is chosen
  while A1 is free.  Parameters go to registers at entry in some routines
  (0xE16C5E, 0xE17778, 0xE1359C, OSINFO_$GET_MMAP) and are re-read from the
  frame in others (0xE00EB0).  Pointers may be kept in D registers
  (OSINFO_$GET_MMAP).
- Which callee-saved register first: lowest-free fits 0xE1359C,
  PROC1_$SET_ASID and 0xE17778, but OSINFO_$GET_MMAP's first parameter
  goes to D3 with D2 used as a temporary.
- Register reuse within a statement: 0xE1359C takes A0, A1, A3 for three
  successive loads in one argument list (no reuse); PROC1_$TST_LOCK
  frees the address register of `(0x40,A0)` before allocating the next
  data register, but that is consistent with either rule.

### 12.2 When by-value parameters are loaded at entry (survey; partly Verified)

Method: every `link` in the map's `I` segments other than `*_ASM`
(0xE00800..0xE88834, map-named and module-local routines), disassembled to
the first `rts`; the parameter reads `(d,A6)`, d >= 8, listed with their
position.  The survey used ad-hoc scripts that were not kept; re-check sites with `tools/imgdis.sh`.  Semantics and parameter
kinds from the C in `../source/domain_os`.

**What the image shows.**

- *By-value parameters read by a body of several statements are loaded
  at entry, in parameter order, even when each is read only once and
  only by a late statement* (Verified):
  - TIME_$Q_INIT_QUEUE 0xE16C62..0xE16C6A: byte `(8)`→D0, word
    `(0xa)`→D1, pointer `(0xc)`→A0; four stores follow, the byte and the
    word are each read once, by the third and fourth statements.
  - module-local 0xE0A328 (DMA_, `dma/init_m68450_channel.c`): word
    `(0xc)`→D0 at 0xE0A32C, read once, by the last of five statements.
  - module-local 0xE758C4 (RING, `ring/helpers.c`): longwords `(8)`→D0,
    `(0xc)`→D1 at 0xE758C8/0xE758D2, each read once.
  - MMAP_$SET_WS_MAX 0xE0CA88..0xE0CA90: `(8)`→D0, `(0xa)`→D1 (read once,
    by the last statement), `(0xe)`→A0; SIO6509_$INQ_LINE
    0xE7266C..0xE72674: `(0xc)`→A0, `(0x10)`→D0, `(0x14)`→A1, each read
    once (both bodies have an `if`).
- *Register choice:* the lowest free scratch register of the class, in
  parameter order: data in D0, D1, pointers that are dereferenced in A0,
  A1 (the five sites above).  Under register pressure, or when the value
  must survive a call, callee-saved registers are used, also without
  calls (MST_$VA_TO_SEGNO 0xE0DCD6 D2; ROUTE_$FIND_PORT 0xE15B06 D3;
  PROC1_$SET_TYPE 0xE152F2 D2, D3 around a `jsr`).  A pointer that is
  also used as data is kept in a D register and copied to an A register
  for each dereference (EC2_$INIT 0xE42C64, `movea.l D0,A0` at
  0xE42C70; ACL_$ADD_PROJ 0xE47EC0, `movea.l D4,A0` at 0xE47ED4).
- *Functions differ:* PARITY_$CHK_IO 0xE0B180..0xE0B18A loads `(8)` into
  **D1** and `(0xc)` into D0.
- *One-statement bodies:* PROC1_$TST_LOCK reads its parameter inside the
  expression, after the set's address (0xE148CE..0xE148D4, section 5.1);
  TIME_$Q_FLUSH_QUEUE 0xE16C84 loads its pointer right before the store.
- *Entry loads are interleaved with other code* (an instruction
  scheduler; Verified that it happens, rule unknown): `movea.l #imm,A0`
  between two parameter loads (PARITY_$CHK_IO 0xE0B184; NET_IO_$BOOT_DEVICE
  0xE31C1E; 0xE758CC); the call pad `subq.l #2,SP` before a parameter
  load (AREA_$FREE_ASID 0xE07E8E; PMAP_$PURGE_WS 0xE146C6); stores to
  one record in non-source order (XPD_$RESET_PTRACE_OPTS 0xE5B15E..
  0xE5B16C: offsets 12, 0, 13, 4, 8).
- *Not loaded at entry:* a by-value parameter that is only pushed as an
  argument is pushed from the frame (AST_$SET_ATTRIBUTE 0xE053DA `move.l
  (0xe,A6),-(SP)`); a byte read once after a call is read there
  (TTY_$I_ERR 0xE1BE28).
- *By-address (`var`) slots behave differently* and are not settled:
  reloaded for every statement (module-local 0xE0A328: `movea.l (8,A6),A0`
  five times, if its first parameter is the device block by address, as
  its C suggests); loaded where first needed, each into a fresh register
  with no reuse within the body (IIC_$RECEIVE 0xE70AC0..0xE70AD0: A0, A1,
  A2, the last one callee-saved; PROC2_$ALIGN_CTL 0xE41D82..0xE41D8E;
  XNS_IDP_$GET_STATS 0xE18FF4 A1); or loaded at entry when read more than
  once (PEB_$TEST_PARITY_ERR 0xE12000; MMAP_$GET_WS_INDEX 0xE0CA4C).
- *A held value is copied before arithmetic:* BAT_$GET_BAT_STEP 0xE3BB68
  `move.w D0,D1; mulu.w #0x234,D1`; module-local 0xE46498 `move.w D0,D1;
  add.w D1,D1`.

**Implemented** (`crates/dpc-m68k/src/isel.rs`, `select_body`,
`Selector::entry_loads`): a *procedure* body of two or more statements
loads every by-value parameter it reads, in parameter order, into the
lowest free scratch register (D for data, A for a dereferenced pointer);
its statements then use only those registers.  A one-statement body keeps
its own evaluation order (PROC1_$TST_LOCK, PROC1_$GET_LOCKS,
TIME_$Q_FLUSH_QUEUE stay byte-identical).  Reported as not supported yet:
a function body of several statements; a third parameter of a class; a
pointer parameter also used as data; a statement in a several-statement
body that needs a register of its own (scheduling and the no-reuse
pattern above); computing with a held parameter (the copy rule); storing
a parameter that was not loaded at entry (one-statement bodies);
assigning to a parameter.

**Uncertain:**

- Whether PROC1_$TST_LOCK's parameter is "not loaded at entry" or "loaded
  at entry, then moved after the address load by the scheduler".  The
  scheduler's rule would decide it; TIME_$Q_INIT_QUEUE shows no such
  move (`movea.l (0xc,A6),A0` stays next to its first use).  Against a
  simple "one-statement bodies are not preloaded" rule: module-local
  0xE46498 (ACL_), a one-statement boolean function, reads `(8,A6)` into
  D0 first (0xE4649C), before `movea.l #imm,A0` and the copy `move.w
  D0,D1`.
- The rule for by-address slots (above), and when the original chose
  callee-saved registers without calls.

---

## 13. Instruction encodings (Verified, matters for byte comparison)

Counts below are from a sweep of the `.TEXT` range 0xE38000..0xE71D7C.

- Immediate compares and arithmetic use the **CMPI/ADDI/SUBI/ANDI** forms
  (`0c40` cmpi.w ×996, `0c80` cmpi.l ×355, `0240` andi.w, `0680` addi.l, …),
  which is what gas emits by default.
- **Exceptions:**
  - `sub.l #imm,Dn` with the immediate-EA encoding `90bc`, seen 6 times
    (e.g. 0xE40188).
  - `cmpa.w #0,An` (`b6fc`, e.g. 0xE38EC0).
  
  Where gas would pick a different encoding, the emitter must write the
  original one.  The tree's hand-asm files do it with `.short`.
- `addq`/`subq` are used for 1..8; `moveq` for −128..127 into a D register.
- **Stores of constants** (Verified; implemented in `isel.rs`,
  `Selector::store`):
  - zero: `clr.<s> <ea>` (TIME_$Q_INIT_QUEUE 0xE16C6E `clr.l (A0)`;
    `clr.w` IIC_$RECEIVE 0xE70ACE; `clr.b` XPD_$RESET_PTRACE_OPTS
    0xE5B15E);
  - boolean true: `st <ea>` (SMD_$ENABLE_TRACKING 0xE6E468 `st (0xe0,A5)`,
    a boolean per `smd/smd_internal.h`; about 150 `st` to memory in the
    image);
  - others: `move.<s> #n,<ea>`, also for small longwords (`move.l #1`,
    `move.l #7,(A0)` 0xE311C8; never `moveq` + `move` seen for a store):
    module-local 0xE0A328 at 0xE0A334 `move.b #0x10,(7,A0)`,
    MMAP_$ALLOC_CONTIG 0xE0D906 `move.l #0x6000e,(A1)`, 0xE758D8
    `move.w #0x200,(0xa,A0)`;
  - **a byte immediate's extension word has a zero high byte**:
    0xE0A33E `10bc 00ff` for `move.b #0xff,(A0)`; gas encodes `#-1` as
    `ffff`, so the emitter writes byte immediates as 0..255 (`Ea::Imm`).
    Of the 235 `move/and/or/cmp.b` with a negative immediate in the
    survey of section 12.2, 229 have a `00` high byte and none `ff` (the
    other 6 are linear-sweep decodes of data).
  - **device registers**: `move.b #0` instead of `clr` (WIN_ module
    0xE19112, a controller register per `win/wait_for_controller.c`;
    RING 0xE758E2; a few dozen `move.<s> #0` in the survey, pushes
    included).  Ref 3.15.3 (p. 3-57): the
    `device` attribute stops "gratuitous read-modify-write references";
    `clr` reads its operand on the 68000.  How the objects were declared
    is not known.  `dpc-sema` rejects `device`, `volatile` and `atomic`
    ("not supported yet") so that such stores are never compiled to `clr`.
- **Store merging (reference; Uncertain in the image):** ref 3.15.3,
  p. 3-57: "By default, the compiler optimizes certain adjacent references
  by merging them into one large reference" (`a := 0; b := 0` on two
  `integer16`s becomes one 32-bit store).  Not yet found in the image;
  the two adjacent `clr.b` of XPD_$RESET_PTRACE_OPTS (0xE5B15E, 0xE5B164)
  are not merged.  `dpc` rejects two consecutive stores to adjacent
  objects of one base that fit in a longword.
- `link.w`, `bsr.s`/`bsr.w`, `bra.s`/`bra.w`: the length must match the
  original, so emit explicit sizes and never rely on gas relaxation
  (`jbra`).
- **Emitter conventions (`dpc-m68k`):** gas syntax as in the hand fixtures
  (`tests/conformance/handasm/`): `%` register names, `%fp` for A6,
  `(d,%an)` operands, absolute operands as `(SYM).l` so gas cannot pick
  abs.w.  The module-end `0x2048` padding (section 7) is emitted as
  `.balignw 4,0x2048` at the end of each code section.  Loop labels are
  gas numeric local labels (`1:`, `dbf %d0,1b`).
- **68010 only.** No 68020-only instruction (bit-field ops, `extb.l`,
  32-bit `mul`/`div`, scaled index, `bra.l`) has been found in compiled
  code.  The linear-sweep hits are all inside data.  The kernel runs on
  68010 machines (DN300/DN320), so this is required, not a style.  68881
  code appears only in hand-written FP save/restore (`fim/sau2`), not in
  compiled output.  **Uncertain:** confirm with a per-routine scan once the
  harness exists.
