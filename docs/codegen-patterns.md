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
- **Pop size rule (Verified, survey of compiled code 0xE00800..0xE88834
  with ad-hoc scripts, not kept; re-check with `tools/imgdis.sh`):**
  `addq.w #4,SP` ×1961, `addq.w #6,SP` ×1 (PROC2_$UPGID_TO_UID 0xE4103C,
  an unpadded 6-byte list: section 1.4), `addq.w #8,SP` ×962; `lea
  (n,SP),SP` for n = 12..68 ×1850, never below 12.  No `addq.l` pops.  So
  `addq.w` up to 8 bytes, `lea` above.  The pad is always `subq.l #2,SP`
  (×2351).
- **The pop is skipped** when nothing is pushed again before the
  epilogue, whose `unlk` discards the arguments (Verified; survey of
  every statement-level call with arguments in compiled code,
  0xE00800..0xE88834, with a control-flow walk from the call to the
  epilogue; ad-hoc scripts, not kept; re-check sites with
  `tools/imgdis.sh`):
  - **Not popped** exactly when every path from the call to the epilogue
    is free of pushes, calls and *conditional* branches (a `dbf`
    counts as one); unconditional `bra`s to joins may occur.  1135
    calls, none popped: AST_$PAGE_ZERO 0xE00F0C `jsr ML_$UNLOCK` then
    `unlk`; PROC2_$UPGID_TO_UID 0xE41050, then two record copies and a
    status store; 0xE01166 `jsr ML_$UNLOCK; bra.s 0xE01176`, a join
    followed by stores and the epilogue; LOG_$READ, whose final `bsr` is
    followed by the A5 restore (codegen-patterns 12.1); 0xE01432 `jsr;
    bra.w` to the register restore (a `return`).
  - **Popped** whenever a push, a call or a conditional branch follows on
    some path: 2729 calls with both, 1742 with a push only, 240 with a
    conditional branch only (0xE00BCA `jsr; addq.w #8,SP`, then an `if`
    and the epilogue).  No call is popped with neither.
  - **Shared pop at a join (cross-jumping):** 56 unpopped calls branch
    to a pop at a join instead: when both parts of an `if` end in a call
    with the same argument bytes and a pop is needed, the `then` part's
    call is followed by `bra` to the `else` part's pop (0xE06156 `bsr.w;
    bra.s 0xE06194`, with `addq.w #4,SP` at 0xE06194 after the other
    call).  The 45 such pairs with *different* sizes pop separately
    (0xE01A38); none of the same size does.
  - Two calls inside the argument list of another call (0xE69FF6,
    0xE748E6) have their pops merged with the outer call's: not
    statement-level, outside this rule.
  - **Implemented** (bead domain-pascal-bzl; `crates/dpc-m68k/src/isel.rs`,
    `Selector::call` with `Tail` / `seq_clean`): the no-pop rule.  The
    shared pop is rejected ("not supported yet").  Goldens
    `tests/golden/calls/pop_branches.pas`, `calls/calls.pas`.
- **Implemented** (`crates/dpc-m68k/src/isel.rs`, `Selector::call`):
  pad, right-to-left pushes, `jsr`/`bsr`, `addq.w`/`lea` pop, the no-pop
  rule above.  Byte-identical in `make conform`:
  `tests/conformance/ast_page_zero.pas` (0xE00EB0 + AST_$PAGE_ZERO).

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
- **Exception (Uncertain):** PROC2_$UPGID_TO_UID 0xE41030..0xE4103C
  pushes 6 bytes with no pad: `move.w (-0xa,A6),-(SP); pea (-0x14,A6);
  bsr.w 0xE4232A; addq.w #6,SP`.  The module-local 0xE4232A stores 8
  bytes through its `(8,A6)` and the caller then copies them from
  `(-0x14,A6)`: perhaps a function returning a record through a hidden
  address, which would then not be padded.  `dpc` pads every list of
  2 mod 4 bytes and has no record-valued functions.

### 1.5 Constant arguments (Verified / Reported)

Counts from a survey of compiled code 0xE00800..0xE88834 (every
non-`_ASM` `I` segment split at map symbols and `link` words, as
`tools/loopalign.py` does; ad-hoc scripts, not kept; re-check with
`tools/imgdis.sh`).

- **Long constant, by value:** `pea (n).w` when n sign-extends from 16
  bits: 128 sites, values 1..0x7fff and -1 (0xE00EB4 `pea (0x16).w`
  pushes 0x00000016; `pea (-1).w` ×10, e.g. 0xE0F660; no other negative
  value is pushed by `pea`, and no `move.l #n,-(SP)` pushes one that
  sign-extends from 16 bits).  No `move.l #n,-(SP)` of a 16-bit value
  other than 0.  Others: `move.l #n,-(SP)`.  **Uncertain:** negative
  values other than -1 are taken to follow the same 16-bit rule.
- **Zero:** `clr.l -(SP)` (374 sites, 0xE135D0) and `clr.w -(SP)` (472,
  0xE135D2); no `move.w #0,-(SP)`.  19 `move.l #0,-(SP)` remain
  unexplained (0xE0F79E, 0xE13B32, ...; perhaps `nil`): Uncertain, `dpc`
  uses `clr.l` for the integer 0.
- **Bytes:** `st -(SP)` for true (196 sites, 0xE1365C).  `false` (a
  byte 0) clears the whole slot with `clr.w -(SP)`: RIP_$SEND_UPDATES
  pushes `st -(SP)` on one path (0xE6889E) and `clr.w -(SP)` on the other
  (0xE688BC) for the same byte parameter of RIP_$BROADCAST, which reads
  it with `move.b (8,A6),D2` (0xE872A6) (Verified at this site;
  implemented).  There is no `clr.b -(SP)` and no `move.b #n,-(SP)` at
  all, so other byte constants (characters) are pushed some other way,
  not yet catalogued; `dpc` rejects them.
- **Adjacent word constants merged:** two adjacent word constants become
  one longword push.
  - 0xE00EB4 is the MMU_$INSTALL asid.w/prot.w pair, `pea (0x16).w`
    (the hand-written MMU_$INSTALL reads asid at (0x31,SP) and prot at
    (0x33,SP), `../source/domain_os/mmu/sau2/install.s`).
  - ACL/FILE call sites push the rights/option pair as one `move.l #imm`
    (Reported: `file-subsystem.md`).
  - The HIGH half is the *first* (lower-address) parameter.
  - Survey: no `move.w #n,-(SP)` / `clr.w -(SP)` is directly followed by
    another one, except 0xE38B42/0xE38B44, where the second is a branch
    target (two paths, not one argument list).  So every pair was merged.
    **Uncertain:** which pair merges in a run of three word constants
    (`dpc` rejects it), and whether a byte next to a word merges.
- **Address of another unit's variable** (by address under the default
  convention, or `addr(x)` by value): `move.l #sym,-(SP)`, 374 sites, 362
  of them a map symbol's own address (UID_$NIL ×36 e.g. 0xE01824,
  TIME_$CLOCKH_EC ×24, DIR_$MUTEX ×22; 0xE00EB8 `move.l #AST_$ZERO_BUFF`).
  An address *inside* another unit's object is pushed both ways: 12
  `move.l #` (0xE32398 DIR_$MUTEX+0x14) and all 24 `pea abs.l` (0xE0F5DC
  ROUTE_$PORT+0x30, 0xE0A36C DMA+0xC0): Uncertain, rejected by `dpc`.
  The same address into a register is `movea.l #sym,An` (0xE00ECC;
  DISK_$GET_ERROR_INFO 0xE6C090).
- **A by-address parameter passed on by address** is its slot, pushed
  with `move.l (d,A6),-(SP)` (Verified: LOG_$READ 0xE1780C `actual_len`,
  0xE17818 `buffer`, both passed on to the worker 0xE17778).
- **Implemented** (`isel.rs`, `Selector::push`, `push_long_const`): all
  the forms above that are Verified; `addr(x)` (ref "Addr", p. 4-13) in
  `dpc-sema`.  Golden `tests/golden/data/through_worker.pas` (LOG_$READ's
  instruction sequence).
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
- **Variable of another size by address:** an `integer32` variable for a
  value or `in` `integer` parameter (assignment compatible, ref 5.3.2)
  cannot be passed as its own address (the callee would read its high
  word); presumably a converted copy is passed like an expression, not
  catalogued.  `dpc-sema` marks such an argument `needs_temp` and
  `dpc-ir` rejects it ("not supported yet").

### 1.6 Call instructions (Verified)

- **Same module and section:** `bsr.s` or `bsr.w` (0xE00F02, 0xE1781C).
  `.s` exactly when the target is in reach: in compiled code all 121
  `bsr.s` are in reach and none of the 1800 `bsr.w` is (survey as in
  1.5).  **Implemented:** the emitter lays the section out with the
  encoded length of every instruction (`Insn::len_at`, the 0x2048 filler
  included) and picks the size (`crates/dpc-m68k/src/emit.rs`,
  `size_branches`); gas never chooses.
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
  - **Untouched frames are ordinary (Verified counts; cause Uncertain).**
    Of the compiled routines that end in `rts` (survey as in 1.5), 306
    without any loop have a `link` area they never address (143 of
    4 bytes, 104 of 8, 27 of 12, ...), against 469 that use theirs: locals
    the compiler kept in registers, or never needed, still got frame
    space.  Routines with `dbf` loops and an untouched frame most often
    have 12 bytes (35 of 132; 29 have 8, 24 have 16, 13 have 4): the
    module-local 0xE00EB0 (`link #-12`, one clearing loop),
    NET_IO_$FREE_ASID 0xE74E84, FLP_$SHUTDOWN 0xE3E228, the KBD
    module-local 0xE1CA62, and two-loop routines too (MEM_$PARITY_LOG
    0xE0ADB0, OS_DISK_PROC 0xE3824C, ACL_$SET_PROJ_LIST 0xE480F4).  But
    PKT_$DUMP_DATA 0xE127E6 has `link #-4` for a loop whose index it
    keeps in D3, and MMAP_$SET_WS_INDEX 0xE0D1C8 `#-4`.  Two hypotheses
    remain: (a) the source declared locals that never reached memory
    (any routine's frame can be unused, above); (b) a `for` loop costs
    hidden frame slots (a saved limit or count?).  (b) does not fit the
    two `#-4` routines unless their loops are not `for` loops; both leave
    their loop early (PKT_$DUMP_DATA 0xE12814 `ble`, MMAP_$SET_WS_INDEX
    0xE0D212 `bra`), but so does the KBD module-local 0xE1CA62 (0xE1CA78
    `bra`), which has 12.  The C reconstructions in `../source`
    (`net_io/free_asid.c`, `flp/shutdown.c`, `pkt/dump_data.c`,
    `mmap/set_ws_index.c`) declare only register-held locals and cannot
    tell.  Not settled; `dpc` gives a `for` loop no hidden slot (as
    (a)), and `tests/conformance/ast_page_zero.pas` declares stand-in
    locals for 0xE00EB0's 12 bytes (its header and the declaration say
    so).  If (b) is confirmed, those stand-ins go.
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
  - **Implemented** (`crates/dpc-ir/src/lower.rs`, `set_loads_data_base`;
    `crates/dpc-m68k/src/isel.rs`, `load_data_base`): an exported routine
    that addresses the unit's static data, directly or through `internal`
    routines of the unit it calls (transitively; LOG_$READ 0xE17806,
    whose worker 0xE17778 reads `(0x14,A5)`), sets A5, as the first
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
  - **Loop-head alignment: the copy loop is an ordinary loop head
    (Verified, see section 7).**  Whether the filler appears depends on
    where the routine sits (section 7), not on the loop being a copy.  In
    aligning routines, 63 copy loops have the filler, 32 are 4-aligned
    already and 2 are at 2 mod 4 (0xE6CBBE, 0xE74766).  In the other
    routines, none of the 140 copy loops has a filler: 93 are 4-aligned
    and 47 are at 2 mod 4.  The cases in question all sit in
    non-aligning code: 0xE1155E (NETWORK_$PAGE_SERVER), 0xE496C2 (ACL_),
    and AST_$LOAD_AOTE.  In AST_$LOAD_AOTE, 0xE023D4 is at 0 mod 4 by
    accident and 0xE02482 is at 2 mod 4.  `dpc` currently always aligns
    (the AST_ 0xE0149E form).  That is right only for aligning routines.
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
  compiler rule.  `dpc` emits top-level routines in source order, so the
  conformance source declares the worker first
  (`tests/conformance/ast_page_zero.pas`).  Calls *after* the callee are
  also seen (AST_$TOUCH_AREA 0xE03548 calls 0xE00EB0 with `bsr.w` at
  0xE03636), so a worker does not have to precede every caller.

---

## 7. Control flow

- **`for` loops with a known count** use `dbf`.  The count register is
  loaded with N-1 (`moveq #N-1` / `move.w #N-1`), and `dbf` runs the body N
  times (Verified):
  - 0xE00ED2 `move.w #0xFF,D0; ...; clr.l (A0)+; dbf` clears 1 KB.
  - 0xE42D50 `moveq #57,D0`: an index register counts up separately
    (`clr.w D2` ... `addq.w #1,D2`).
- **Array indexing in loops is strength-reduced (Verified at the sites
  given; the general rule is Uncertain).**  The element address lives in
  an address register that steps by the element size, with the index's
  offset folded into a displacement: PKT_$DUMP_DATA 0xE12800 `lea
  (4,A0),A2`, then `move.l (-4,A2),-(SP)` and `addq.l #4,A2`
  (0xE12816, 0xE12824), with the index itself in D3 because the body also
  compares it.  An offset-0 access steps with `(An)+`: 0xE00ED8 `clr.l
  (A0)+`; AST_ 0xE0287A `bclr #7,(A2)+; addq.l #3,A2` (4-byte elements,
  byte accessed).  An index used for nothing else gets no register at
  all (0xE00EB0: only A0 and the `dbf` count D0).
- **Clearing loops `clr.<s> (An)+; dbf` (Verified; 21 in compiled
  code):** the address and the count are loaded in either order, with no
  rule found.  Address first: 0xE00ECC `movea.l #AST_$ZERO_BUFF,A0;
  move.w #0xFF,D0`; 0xE40A0E `lea (0xc,A2),A0; moveq #5,D0`.  Count
  first: 0xE0277A `moveq #31,D0; lea (A3),A0`; 0xE09C98; 0xE06B36;
  ROUTE_ 0xE69D12 `move.w #128,D0; movea.l #abs,A0`; 0xE6200E.  Whether
  these are source `for` loops or something else (a record or array
  assignment of zeros?) cannot be told from the image.
- **Implemented** (`crates/dpc-ir/src/lower.rs`, `fill`;
  `crates/dpc-m68k/src/isel.rs`, `Selector::fill`): exactly the 0xE00EB0
  shape: `for i := c1 to c2 do a[i] := 0` with `i` a frame local and
  `a` another unit's array starting at element `c1`; `i` itself is never
  stored (0xE00EB0 does not), which is sound because using the index
  after a loop that ends normally is forbidden (warning "Potential
  illegal use of FOR index variable", ref 9, p. 9-31);
  `movea.l #sym,A0`, the count, the filler, `clr.<s> (A0)+`, `dbf`.  Other
  `for` loops are rejected ("not supported yet").  Byte-identical in
  `make conform` (`tests/conformance/ast_page_zero.pas`).
- **Run-time count:** `subq.w #1,Dn; bmi skip` before entering the `dbf`
  loop (0xE177DC..0xE177F2).
- **Loop-head alignment.**  Some code pads with the 2-byte no-op
  `movea.l A0,A0` (0x2048) so that a loop head starts on a 4-byte
  boundary.  Other code never does.  The survey tool is
  `tools/loopalign.py`:
  - `summary` gives the counts below;
  - `routines [SEG]` gives the per-routine classes;
  - `ends` gives the segment-end padding.

  The survey disassembles every `I` segment in 0xE00800..0xE88834 that
  is not hand assembly (`_ASM` segments, plus M$ARITH, PAS_$SET_BUILD,
  SMD_WIRED, UID_$HASH, UID_LIST and VFMT_$WRITEN), one routine at a time.  It splits routines at map
  symbols and at `link.w A6` words.  A loop head is any target of a
  `dbf` or of a backward `bra`/`bcc`.
  - **The filler (Verified).**  There are 482 `0x2048` words, and 480 of
    them are followed by a 4-aligned instruction.  The other two are
    taken to be real degenerate register copies, not fillers:
    - 0xE332FC is followed by a second `0x2048`, the actual filler at
      0xE332FE, before the loop head 0xE33300 (TTY_$I_INIT);
    - 0xE4DC90 (DIR_$READ_LINKU, non-aligning code) is followed by
      `moveq #18,D1` at 2 mod 4.
  - **What the filler precedes:**
    - 315 loop heads.  Examples: 0xE00ED6, 0xE0149E, 0xE128DA, and
      0xE1CE42 before KBD_$RCV's loop at 0xE1CE44.
    - The end of a segment's code, 61 times: 0xE1A9D2 before OS_TERM
      0xE1A9D4, 0xE1AD16 before BITPAD 0xE1AD18.
    - 34 4-aligned constant cells after a routine's code
      (MMAP_$AVAIL 0xE0CCB6, MMAP_$WIRE 0xE0CD16, PKT 0xE12D32).
    - 64 forward-only branch targets (see below).
    - 6 other instructions.  Some of them are labels reached only
      through a jump table (ASKNODE_$INTERNET_INFO 0xE649E6 before
      `bra.w`), which the survey cannot see as branch targets.
  - **The rule: alignment is on for a leading run of routines in a map
    segment and off for the rest (Verified as a split; its cause is
    Uncertain).**  Each routine was classed by its loop heads:
    - 206 routines have a filler and no head at 2 mod 4;
    - 219 have heads at 2 mod 4 and no filler at all;
    - 20 have both;
    - 237 have only heads that happen to be 4-aligned;
    - 1000 have no loop.

    Loop heads split as follows:

    | Routines | With filler | 4-aligned, no filler | 2 mod 4 |
    |---|---|---|---|
    | 226 that align at least one head | 315 | 160 | 28 |
    | All others | 0 (by definition) | 560 | 358 |

    The classes form **runs of adjacent routines** inside a map segment,
    and the switch goes one way only.  In all 24 segments that hold both
    classes, the aligning routines come first and the non-aligning ones
    after; none switches back.  (MMAP_ counts if the 2 mod 4 head of
    MMAP_$WS_SCAN, 0xE0D3C2, a 0x1AA-byte loop, is read as one of the
    large loops below; the switch is then at MMAP_$ALLOC_PURE 0xE0D78E.)
    - AST_: the last filler is in the module-local routine at 0xE01502
      (in AST_$GET_DISM_SEQN's extent).  The first head at 2 mod 4 is in
      the routine at 0xE01872.  AST_$UPDATE 0xE016D0 lies between and
      has only 4-aligned heads, so it cannot be classed.
    - .TEXT VTOC_: VTOC_$MOUNT has a filler (0xE3871E).  The next
      routine, VTOC_$DISMOUNT 0xE38764, has a copy loop at 2 mod 4 with
      none (0xE387CE).
    - RING: aligning up to RING_$IOCTL; RING_$SVC_CLOSE 0xE76E22 is not
      (head 0xE76EAE).
    - XNS_IDP and NETWORK switch the same way.
    - PMAP_ and PROC1_ never align.
    - OS_INIT_PROC is almost entirely aligning.

    **Both sides of a switch are one object module.**  Non-aligning
    routines and aligning ones call each other's unnamed (module-local)
    routines with `bsr`: 62 such calls in 16 segments.
    For example, AST_$TOUCH_AREA 0xE03548 has dbf heads at 2 mod 4
    without a filler (0xE037EA, 0xE0386E).  It calls the module-local
    routine at 0xE00EB0 (`bsr.w` at 0xE03636), whose filler is 0xE00ED6.
    XNS_IDP_$OS_OPEN calls the module-local 0xE17D12 (`bsr.w` at
    0xE17F36).  `bsr` never crosses a segment in the image (the 10
    apparent cases are data decoded as `bsr.s`).  So a
    single compilation can contain both kinds of code; the switch is not
    "one source file per class".  (This assumes the binder cannot
    resolve calls to routines that have no global symbol, which is not
    checked.)

    The **end padding of a segment** agrees with the class of its last
    routines.  `ends` lists the 61 segments that end in `0x2048` and the
    21 that end `rts; 0x0000`.  (Most other segments end in an `unlk;
    rts` or a constant that is already at 0 mod 4.)  Taking the last
    routine that has a filler or a 2 mod 4 head:
    - 24 `0x2048` segments end in aligning code and none in non-aligning
      code;
    - 15 `0x0000` segments end in non-aligning code (AST_ 0xE0736E,
      RING, ACL_, OLD_DIR, VTOC_ .TEXT, ...);
    - 2 `0x0000` segments end in aligning code (NET_IO, HINT_), but their
      last few routines have no loop to tell.
    - 4 of the 21 final `0x0000` words are not padding but a pooled
      word constant that the segment's code passes by address with
      `pea (d,PC)` (section 1.5): NET_IO 0xE0E8C2 (from
      0xE0E848), OS_TERM 0xE1AB26 (0xE1AB0C), WIN_ 0xE303D6 (0xE30358,
      0xE303BE), DIR_SERVER 0xE5839E (0xE58374).  So NET_IO is not an
      exception; HINT_ still is (Verified: a scan of every routine of the
      21 segments for a PC-relative operand naming the last word).

    So non-aligning code does not pad the segment end; whether the
    compiler or the binder wrote the `0x0000` is Uncertain.  The cause of
    the switch is not recorded in the image.  Because it happens inside
    one object module and only from aligning to non-aligning, a
    per-file compiler option or compiler version does not explain it
    alone.  Something in the compilation that turns alignment off for
    the rest of the unit fits better; no candidate has been confirmed.
    The reference documents no option or directive for code or loop
    alignment: the `-opt` and `-cpu` descriptions (6.4.24, p. 6-26;
    6.4.9, p. 6-14) do not mention it.  The only directives that stay in
    force for the rest of a unit and concern alignment are
    `%natural_alignment`, `%word_alignment` and
    `%push_alignment`/`%pop_alignment` (4.7, Table 4-11, p. 4-44).  The
    reference ties them to data layout only.  Whether one of them also
    turns loop alignment off is untested (a lead, not a finding).
  - **Implemented** (bead domain-pascal-vp7): since the cause is not
    known, the switch is a `dpc` option, not a source directive:
    `--loop-align=on|off` (default on) and
    `--loop-align-off-from=ROUTINE` (on before ROUTINE in emission
    order, off from it on).  dpc's emission order is code section by
    code section, and the unit's routine order within one; a routine
    declared `forward` is placed at its declaration, not at its body
    (whether the original compiler did the same is unverified).  A routine that does not align gets no
    filler (`crates/dpc-m68k/src/emit.rs` drops its `Align4`s before the
    branches are sized); the module end, or a module-tail fragment's, is
    padded with `0x2048` after an aligning last routine and with zero
    words otherwise.  The conformance manifest passes the option per unit
    (`tests/conformance/README.md`); `tools/loopalign.py switch SEG`
    shows the interval in which a segment's switch lies.
  - **Hypotheses refuted:**
    - *Another alignment base.*  All 210 `I` segments start at 0 mod 4
      and have a size that is a multiple of 4.  Aligning code aligns to
      absolute addresses: PKT_$RECENTLY_MISSING starts at 0xE128BA (2
      mod 4), and its filler at 0xE128DA puts the head on 0xE128DC.
      Non-aligning code has no filler at all, so no other base would
      account for it.
    - *Copy loops vs. source loops.*  See section 5.2: copy loops behave
      like any other head.
    - *Entry by fall-through vs. by `bra` to the test.*  In aligning
      routines, heads entered by fall-through split 312/158/25
      (filler/aligned/2 mod 4); heads entered by a branch split 3/2/3.
    - *Calls in the body.*  Heads of loops with a call split 95/44/18;
      loops without one split 220/116/10.
    - *Section.*  The split runs inside sections.  `dpc
      --fragment-origin` reproduces the absolute base for conformance
      units that start mid-module (tests/conformance/README.md).
  - **Inside aligning routines, which heads (Uncertain).**  Nearly all
    small loops are aligned.  The size here is the head-to-branch
    distance:

    | Body size | With filler | 4-aligned | 2 mod 4 |
    |---|---|---|---|
    | 0x40 bytes or less | 235 | 126 | 6 |
    | Up to 0x100 | 74 | 29 | 4 |
    | Over 0x100 | 6 | 5 | 18 |

    Of the 28 unaligned heads in aligning routines, 15 are outer loops,
    mostly large: 0xE0124A, 0xE0738A, 0xE0FDE2, 0xE34426, 0xE423C6,
    0xE4240A, 0xE44F02, 0xE4B072, 0xE64F86, 0xE6657E, 0xE6AB5A,
    0xE6CAE2, 0xE6DAEE, 0xE71F66, 0xE75A22.  The other 13 are innermost
    loops, with no rule found: 0xE2F45E, 0xE319DE, 0xE33FE6, 0xE3443E,
    0xE427DE, 0xE5515A, 0xE666EE, 0xE6A73A, 0xE6CBBE, 0xE74556,
    0xE7469E, 0xE74766, 0xE76CB2.  Large loops
    above 0x100 bytes are aligned by the filler only 6 times: 0xE0FDA0,
    0xE12C28, 0xE1CF18, 0xE61130, 0xE611E0, 0xE71FE8.
  - **Other branch targets (Verified: not aligned as a rule).**
    Forward-only targets in aligning routines: 47 have the filler, 991
    are 4-aligned and 971 are at 2 mod 4.  In the other routines the
    split is 17/3367/3452.  So `if`/`else` joins are not aligned.  The
    few that are cluster in a handful of routines, for reasons not yet
    understood:
    - MST_$INIT 0xE30C9C..0xE30D30, at the joins of `max`/`min`-like
      and shift-for-divide sequences;
    - ACL_ 0xE4519C..0xE45850;
    - ASKNODE_$INTERNET_INFO;
    - NETWORK_$READ_AHEAD.
  - **What `dpc` should do:** an alignment switch that can change
    between routines, not just once per unit.  The conformance manifest
    records, for each unit, whether it starts aligning and the first
    routine that does not align.
    - **Aligning routines:** put the filler before every loop head (the
      `dbf` target, or the target of the backward `bra`/`bcc`) that
      would start at 2 mod 4.  If the last routine aligns, pad the end
      of the code section with `0x2048`.  Do not align forward labels.
    - **Non-aligning routines:** no filler; if the last routine does not
      align, the end padding is `0x0000`.
    - Large outer loops in aligning routines, which the original often left
      unaligned, need an issue before a conformance target hits one.
    - Classes of current and next targets:
      - Aligning: AST_ up to AST_$GET_DISM_SEQN (0xE00EB0,
        AST_$PAGE_ZERO), LOG_ (LOG_$ADD 0xE1763E has two fillers), PKT,
        OSINFO.
      - Non-aligning: PMAP_ (PMAP_$FLUSH's two heads are 4-aligned by
        accident), PROC1_, XNS_IDP from XNS_IDP_$OS_OPEN on
        (XNS_IDP_$GET_STATS).
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
- **Branch sizes (Verified):** `.s` when the target is in reach,
  otherwise `.w`; no `.l` branches (68010).  Compiled code has 11515 `.s`
  branches, all in reach, and 2793 `.w`, all out of reach but two
  (0xE7065C `bra.w`, 0xE706DE `bne.w`).  Both are in SMD_UTILS
  (0xE702F4), which the survey counts as compiled code but which is hand
  assembly: `movem.l` before `link` and A5 taken from A0 (0xE706C0..
  0xE706C4), the `movem.l` restore after `unlk` (0xE7075A), a frameless
  routine addressing its arguments off SP (0xE70760), `bne.w` on the
  flags of a store (0xE706DA..0xE706DE).  **Implemented:**
  the emitter sizes every `b<cc>` and `bsr` from its layout
  (`crates/dpc-m68k/src/emit.rs`, `size_branches`) and writes the size
  explicitly; gas refuses an out-of-range `.s` rather than relaxing it.
- **Boolean tests:** `tst.b` then `bmi`/`bpl` (signed byte: 0xFF is
  negative); `sne`/`seq` to materialise a value.  After `tst.b` the
  compiled code branches with `bpl` 919 times, `bmi` 425, `beq`/`bne` 37
  (Verified counts).

### 7.1 Conditions and `if` (Verified at the sites given; implemented)

Counts from the survey of compiled code (every non-`_ASM` `I` segment,
split as `tools/loopalign.py` does; ad-hoc scripts, not kept).

- **Against 0 or `nil`: `tst.<s>`**; there is no `cmpi #0` in compiled
  code.  `=`/`<>` branch with `beq`/`bne`.  For a signed value `<` and
  `>=` use `bmi`/`bpl` (after `tst.w`: `bmi`/`bpl` 144 times, `blt`/`bge`
  4 times, 0xE0EB86, 0xE38A94, 0xE38D5A, 0xE6BA12, source construct not
  identified; after `tst.l`: `bmi`/`bpl` 11 times, `blt`/`bge` never);
  `<=`/`>` use `ble`/`bgt` (42 and 17
  times after `tst.w`).  0xE1778C `tst.l (0x14,A5); bne.s` is `if ptr =
  nil then`.  A pointer in an address register is tested with `cmpa.w
  #0,An` (151 sites, 0xE0243C; `tst An` is not a 68010 instruction).
- **Against another constant: `cmpi.<s> #n,<ea>`**, on a register
  (0xE1779A `cmpi.w #0x400,D1`, a parameter loaded at entry) or on memory
  (0xE4FE90 `cmpi.w #4,(0x12,A6)`; 0xE58624 `cmpi.l #0x230001,(A0)`;
  0xE4BAE0 `cmpi.l #0x400,(0x10,A3)`).  The compare has the width of the
  operand's storage, not of its host type: the `0..65535` parameter at
  0xE1779A is compared with `cmpi.w`.
- **Signed or unsigned:** `blt`/`ble`/`bgt`/`bge` for signed values,
  `bcs`/`bls`/`bhi`/`bcc` for unsigned ones (0xE1779E `bcc` skips the
  `then` part of an unsigned `<`; 0xE177BE `bls` of an unsigned `>`).
- **Two variables:** `cmp.<s> <ea>,Dn` with one of them in a register
  (0xE6A9B4 `move.w (A1),D5; cmp.w (0x14,A6),D5`).  Which operand of the
  source went to the register cannot be seen in the image.
- **A long constant held in a register (Uncertain):** 0xE177AE..0xE177B6
  `move.l #0x400,D3; add.l D1,D2; cmp.l D3,D2`, where 15 of the 28 `add.l
  Dm,Dn` directly followed by a compare use `cmpi.l` instead, among them
  the same zero-extended word sum (0xE12164..0xE12174 `clr.l D7; clr.l
  D6; move.w D2,D7; move.w D3,D6; add.l D6,D7; cmpi.l #0x500,D7; bls`).
  The other register-held constants found are the left operand of a
  subtraction (0xE10002 `move.l #0x400,D1; ...; sub.l D0,D1`; 0xE0E964).
  Not explained.
- **Flags of a load (Uncertain):** the `tst` is sometimes left out after
  a move into the tested register (RIP_$SEND_UPDATES 0xE6887E `move.b
  (8,A6),D0; bpl`; 0xE014DC `move.l (A3),D0; beq`) and sometimes not
  (0xE0B640 `move.l (A1),D0; tst.l D0; bmi`; 0xE0BEAE).  After a store
  to memory the memory is tested again (0xE02806 `move.w D0,(0x412,A0);
  tst.w (0x412,A0)`; 53 such pairs).
- **`if c then T else E`:** a branch on the negated condition to `E`
  (or past `T`), `T`, a `bra` to the join, `E`.  0xE177BA..0xE177CC
  `cmpi.w #0x400,D0; bls.s 0xE177C4; clr.w (A0); bra.s 0xE177CC; <E>`.
  `else if` chains branch to the end of the chain from every `then`
  part, and the joins coincide (NAME_CONVERT_ACL_STATUS
  0xE5861C..0xE58652).  Joins are not aligned (section 7, other branch
  targets).
- **`return`** (ref "Return", p. 4-166) is a `bra` to the epilogue's
  first instruction, the register restore: 0xE17794 `bra.s 0xE177F6`.
  Compiled code never has two `bra`s in a row, so a `then` part ending in
  `return` has no `bra` to the join after it (Verified by absence; dpc
  leaves it out).
- **No branch to a `bra` (Verified by absence):** of the 3324 forward
  `bra`s in compiled code none targets a `bra`, and no `b<cc>` does
  (0xE34BC8, the one hit, is data).  So where the plain layout above
  would branch to a `bra` (`if a then begin if b then ... else ... end
  else ...`: the inner join is followed by the outer `bra`; `if a then
  begin if b then ... end else ...`; `if a then begin if b then ...;
  return end`; `if a or else b then return`), the original threaded the
  branch or laid the code out otherwise; which is not known.  dpc rejects
  these (`select_body`, `Selector::jump`).
- **Identical ends of the two parts (Uncertain):** the pop is shared at
  the join (section 1.1), and RIP_$SEND_UPDATES shares a `jsr`: one part
  is `subq.l #2,SP; st -(SP); bra.s 0xE688BE`, the other `subq.l #2,SP;
  clr.w -(SP)` falling into the `jsr RIP_$BROADCAST` at 0xE688BE.  But
  0xE381A8 and 0xE381D2 are two `jsr 0xE825E4` ending the two parts of
  one `if`, not shared, and 0xE1A9F0/0xE1AA00 two identical `move.w
  0xE2E00E,D0` (of the 3324 forward `bra`s to a join reached by falling
  through, 4 have the same instruction before the `bra` and before the
  join).  dpc never shares anything but rejects the shared-pop case.
- **`and then` / `or else`** (ref pp. 4-18..4-19, 4-134): jumping code,
  each operand with its own branch: 0xE031A2..0xE031B6 `cmp.w D4,D2;
  bcc.s <exit>; ...; btst #13,D5; bne.s <exit>; tst.w D5; bmi.s <exit>;
  btst #14,D5; bne.s <head>` (a loop test `c1 or else ... or else c4`).
- **Implemented** (`crates/dpc-ir`, `Cond`, `StmtKind::If`/`Return`;
  `crates/dpc-m68k/src/isel.rs`, `Selector::if_`, `branch`, `compare`):
  all of the above that is Verified, with `not`, `and then`, `or else`.
  Not supported yet: a branch to a `bra` (above); comparing booleans
  (`b = true`: compiled code tests booleans with `tst.b`, and has two
  `cmpi.b #0xFF` only, 0xE1B96C, 0xE53FDE, not shown to be booleans);
  a constant on the left of a comparison; comparing
  two variables unless the left one is held in a data register (a
  parameter loaded at entry); operands of different widths or signs
  (implicit widening); unsigned ordering against 0; `cmpa.w #0,An` (no
  pattern yet produces a held pointer that is compared); a test right
  after the load of its register (above); `and`/`or` without
  `then`/`else`; an `if` with an empty `then` part; a `return` at the
  end of a nested part that ends the routine (a `bra` to the next
  instruction, or nothing?); the shared pop (section 1.1).  Goldens
  `tests/golden/control/if_else.pas`, `control/compares.pas`;
  byte-identical in `make conform`:
  `tests/conformance/name_convert_acl_status.pas`.

### 7.2 Loops with a test (Verified shapes; alignment partly Uncertain)

Backward branches other than `dbf` in compiled code (same survey):

| Branch back | Head entered by | Count | Shape |
|---|---|---|---|
| `b<cc>` | a `bra` (to the test) | 213 | `while`: `bra test; head: body; test: <cond>; b<cc> head` (0xE01A8E..0xE01A98) |
| `b<cc>` | falling through | 190 | test at the bottom, entered from the top: `repeat ... until` (0xE03192..0xE031B6, 0xE03208..0xE03224) |
| `bra` | falling through | 131 | test at the top, exits forward (0xE00E68); source construct not identified |
| `bra` | a `bra` | 7 | |

- **`repeat body until c`:** the head is entered by falling through and
  aligned like every such head in aligning code (section 7: 312 with
  the filler, 158 4-aligned, 25 at 2 mod 4); the test branches back on
  the negated condition.  **Implemented** (`Selector::repeat`, the
  0x2048 filler as for every loop head dpc emits); golden
  `tests/golden/control/repeat.pas`.
- **`while c do body`:** compiled with the test at the bottom, entered by
  a `bra` to the test.  Whether the head after that `bra` gets the
  filler is not settled (in aligning routines: 3 with the filler, 2
  4-aligned, 3 at 2 mod 4).  Not supported yet.

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
  order.  The per-module **code** contribution is 4-byte aligned.  It is
  padded with `0x2048` when its last routines align loops, and with
  `0x0000` when they do not (section 7).
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
| module-local 0xE00EB0 (`misc/zero_page.c`) | `link #-0xc` (frame never touched in the body; section 2) | none | No register holds a value across the two `jsr`s; the argument `(8,A6)` is re-read for each call (0xE00EBE, 0xE00EDE). Loop: A0 pointer (`movea.l #imm,A0`), D0 `dbf` counter, the lowest free scratch registers right after the first `jsr`. Byte-identical in `make conform` (`tests/conformance/ast_page_zero.pas`). |
| AST_$PAGE_ZERO 0xE00EEC (`ast/page_zero.c`) | `link #0` | none | No registers at all: constants pushed, `(8,A6)` pushed, three calls. Byte-identical in `make conform` (same unit). |
| module-local 0xE17778 (`log/read_internal.c`) | `link #-0x10` | `movem.l D2-D3/A2,-(SP)` (`48e7 3020`) 0xE1777C / `movem.l (-0x1c,A6),D2-D3/A2` (`4cee 040c ffe4`) 0xE177F6 | No calls, yet callee-saved registers are used. Parameters at entry: word `(0xc,A6)`→D0, word `(0xe,A6)`→D1, pointer `(0x10,A6)`→A0, held in these *scratch* registers for most of the routine. Temporaries then take D2, D3 (0xE177A0 `moveq #-2,D2`; 0xE177A8..0xE177AE). The pointer `(8,A6)` is loaded into **A2** only at 0xE177CC, although A1 is free there; A1 is taken next (0xE177DA) and the copy loop uses A0/A1. Registers are reused after the last use of what they held: D1 (`max_len`, last read 0xE177A2) holds the zero-extended `offset` from 0xE177A6; D0 (`offset`, last read 0xE177C8) the word count from 0xE177D2; A0 (`actual_len`, last used 0xE177D2) the source pointer from 0xE177E0. The zero-extended `offset` in D1 is reused by a later statement (0xE177D0 `move.l D1,D3`, CSE across an `if`). 0xE177CC..0xE177DC interleaves two independent computations (`movea.l (8,A6),A2; move.l D1,D3; move.w (A0),D0; add.l (0x14,A5),D3; lsr.w #1,D0; movea.l D3,A1`): an instruction scheduler. D3 is taken at 0xE177C4 although D2 is free. See section 12.3. |
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
- *By-address (`var`) slots* (bead domain-pascal-b1v):
  - **used more than once: loaded at entry**, in parameter order with
    the by-value parameters, into an address register that holds the
    address for the whole body (Verified at four sites): LOG_$READ's
    worker 0xE17780..0xE17788 `move.w (0xc,A6),D0; move.w (0xe,A6),D1;
    movea.l (0x10,A6),A0` (`actual_len`, nine uses); MMAP_$GET_WS_INDEX
    0xE0CA48..0xE0CA4C `move.w (8,A6),D0; movea.l (0xe,A6),A0` (two
    uses); NAME_CONVERT_ACL_STATUS 0xE58620 (six uses, a body of one
    `if ... else if` chain); PEB_$TEST_PARITY_ERR 0xE12000.  The one
    exception found, module-local 0xE0A328 (DMA_$INIT_M68450_CHANNEL in
    `dma/init_m68450_channel.c`), reloads `movea.l (8,A6),A0` before each
    of five stores, which are device-register stores (`device`, ref
    3.15.3; rejected by `dpc-sema`).
  - **used once: loaded where needed**, into a register no earlier load
    of the body used: IIC_$RECEIVE 0xE70AC0..0xE70AD0 A0, A1, A2 (the
    last one callee-saved, although A0 and A1 are free again);
    PROC2_$ALIGN_CTL 0xE41D82..0xE41D8E likewise; XNS_IDP_$GET_STATS
    0xE18FF4 A1; LOG_$READ's worker 0xE177CC `movea.l (8,A6),A2` while
    A1 is free (A1 is taken next, for another pointer);
    MMAP_$GET_WS_INDEX 0xE0CA62 `movea.l (0xa,A6),A2`, A1 taken next.
    The allocation rule is not settled.
- *A held value is copied before arithmetic:* BAT_$GET_BAT_STEP 0xE3BB68
  `move.w D0,D1; mulu.w #0x234,D1`; module-local 0xE46498 `move.w D0,D1;
  add.w D1,D1`.

**Implemented** (`crates/dpc-m68k/src/isel.rs`, `select_body`,
`Selector::entry_loads`): a *procedure* body of two or more statements,
or of one `if`/`repeat`, loads every by-value parameter it reads and
every by-address slot it uses more than once, in parameter order, into
the lowest free scratch register (D for data, A for a dereferenced
pointer or a slot); its statements then use only those registers.  A
body of one `if`/`repeat` that reads a by-value parameter is rejected
(no site seen); a slot used once in such a body is rejected (the
fresh-register rule above).  A parameter that is only
pushed as a whole argument is not "read" for this rule: it is pushed from
the frame at each call (0xE00EBE, 0xE00EDE; AST_$SET_ATTRIBUTE 0xE053DA).
A statement right after a call may take scratch registers of its own when
nothing was loaded at entry (0xE00ECC..0xE00ED2: A0, D0 after the `jsr`
at 0xE00EC2; a call leaves no value in a scratch register).  A body that
loads parameters at entry and also calls is rejected: a scratch register
does not survive the call, and when the original moved such a value to a
callee-saved register is not catalogued (PROC1_$SET_TYPE 0xE152F2).  A one-statement body keeps
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


### 12.3 What LOG_$READ's worker 0xE17778 still needs (bead domain-pascal-g5q.4)

`tests/conformance/pending/log_read.pas` is a reconstruction of the
worker and LOG_$READ (0xE17778..0xE17828); it is not in the manifest
because `dpc` cannot compile the worker yet.  What dpc now produces
byte for byte in its shape: the entry loads (0xE17780..0xE17788, with
the by-address `actual_len` held in A0, section 12.2), the `if ... nil
... return` (0xE1778C..0xE17794), the constant store and the unsigned
compares and branches (0xE17796..0xE1779E, 0xE177BA..0xE177C2), and
LOG_$READ itself (golden `tests/golden/data/through_worker.pas`).  Not
settled, each from this one routine, in image order:

1. `moveq #-2,D2; and.w D1,D2; move.w D2,(A0)` (0xE177A0): `max_len &
   16#FFFE` with the constant loaded first and the variable ANDed into
   it, in D2: the first callee-saved register, taken without any call
   while D0/D1 hold parameters.  Bit operators are not implemented; the
   operand order and the register choice need a survey.
2. `clr.l D1; clr.l D2; move.w D0,D1; move.w (A0),D2` (0xE177A6): the
   zero-extension of two `0..65535` words for a 32-bit sum (implicit
   widening, section 5), both `clr.l` first.  D1, which held `max_len`
   (last read at 0xE177A2), is reused: registers are freed after the
   last use of a parameter, which dpc's pinned registers never are.
   0xE0102C..0xE01034 orders a zero- and a sign-extension the same way
   (`clr.l D0; move.w D2,D1; move.w (0x46e,A5),D0; ext.l D1`).
3. `move.l #0x400,D3; add.l D1,D2; cmp.l D3,D2; bls` (0xE177AE): the
   register-held constant (section 7.1, Uncertain).
4. `move.w #0x400,D3; sub.w D0,D3; move.w D3,(A0)` (0xE177C4): `0x400 -
   offset` in D3 although D2 is free.
5. `move.l D1,D3; ...; add.l (0x14,A5),D3; ...; movea.l D3,A1`
   (0xE177D0..0xE177DA): `univ_ptr(integer32(ptr) + offset)` (ref "Type
   Transfer Functions", p. 4-197, the example `UNIV_PTR(INTEGER32(p) +
   2)`), reusing the zero-extended `offset` from item 2 across the `if`
   (common subexpression), interleaved with the word count `move.w
   (A0),D0; lsr.w #1,D0` (an instruction scheduler), and `buffer` loaded
   into A2 first (0xE177CC) while A1 is free (section 12.2).
6. `subq.w #1,D0; bmi.s <end>; lea (2,A1),A0; lea (2,A2),A1; <head:>
   move.w (-2,A0),(-2,A1); addq.l #2,A1; addq.l #2,A0; dbf D0,<head>`
   (0xE177DC..0xE177F2): a `for` loop over 1-based arrays with a run-time
   count, the index strength-reduced into two pointers that start one
   element in (the `-1` of the lower bound folded into the
   displacement, section 5.2), the destination stepped first.  The head
   0xE177E8 is 4-aligned with no filler (LOG_ aligns, section 7).
7. The frame: `link #-0x10` with nothing in it addressed (section 2,
   untouched frames); the reconstruction declares stand-ins.

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
  `.balignw 4,0x2048` at the end of each code section.  That is right
  only when the last routines align; otherwise the end is `0x0000`
  (section 7).  Loop labels are
  gas numeric local labels (`1:`, `dbf %d0,1b`).
- **68010 only.** No 68020-only instruction (bit-field ops, `extb.l`,
  32-bit `mul`/`div`, scaled index, `bra.l`) has been found in compiled
  code.  The linear-sweep hits are all inside data.  The kernel runs on
  68010 machines (DN300/DN320), so this is required, not a style.  68881
  code appears only in hand-written FP save/restore (`fim/sau2`), not in
  compiled output.  **Uncertain:** confirm with a per-routine scan once the
  harness exists.
