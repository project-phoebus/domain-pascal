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
- **An object reached through a pointer held in an address register**
  (a slot or pointer parameter loaded at entry, 12.2) is pushed with `pea
  (d,An)` / `pea (An)` (Verified: module-local 0xE32B76,
  `sio/init_dtte.c`, 0xE32B86 `pea (0xc,A2)`, 0xE32B92 `pea (0x18,A2)`,
  0xE32B9E `pea (A2)`; also `pea (A2)` at 0xE1BEB2, 0xE1D59E, 0xE3DB8C).
  Implemented; conformance unit `tests/conformance/sio_init_dtte.pas`.
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
  `size_branches`); gas never chooses.  The same holds for forward
  `bsr`s to a routine whose body comes later (section 6, "Order": 54
  `bsr.s` all in reach, 158 `bsr.w` none in reach).  A conformance
  fragment whose routine calls one of its module outside the fragment
  declares the callee `extern` and names it with `dpc
  --module-routine=NAME=0xADDR` (the manifest's `module-routines=` token
  looks the address up in the map): the call is a `bsr` sized from that
  address (DISK_$UNASSIGN_ALL 0xE6BE34 `bsr.w` to DISK_$UNASSIGN
  0xE6BDA8; bead domain-pascal-5aw.15).
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
    so).  If (b) is confirmed, those stand-ins go.  Evidence gathered for
    bead domain-pascal-nda in section 7.4 ("Hidden frame cost"): five
    one-loop routines have no frame bytes beyond their locals, and three
    routines of one loop shape have `link #-8`, `#-12` and `#-16`, so a
    `for` loop costs no hidden slot of 4 bytes or more; (a) is favoured.
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
- **Empty procedures: a bare `rts`, no `link` (Reported; strong image
  evidence; implemented).**  The image (0xE00800..0xE88834) has no
  `link; unlk; rts` sequence at all, nor `link; pea (A5); lea ...,A5;
  movea.l (-4,A6),A5; unlk; rts`, while some 30 map entries in non-`_ASM`
  `I` segments are a bare 2-byte `rts` (byte scan, bead
  domain-pascal-5aw.2 review; ad-hoc script): TIME_$Q_INIT 0xE16C5C
  (right before TIME_$Q_INIT_QUEUE), DMA_$FREE_ASID 0xE0A454,
  PROC2_$SET_STACK_TRUNC_SIZE 0xE414DC, SMD_$PUTC 0xE1D89C (between
  SMD_$COPY_FONT_TO_MD_HDM and SMD_$BUSY_WAIT), IIC_$INIT 0xE70A54
  (first in the IIC module of `link`-framed stubs), RING_$SEND_NULL
  0xE75914.  The one-routine modules MT 0xE70A4C, CT 0xE70AF4, COLOR5
  0xE752E0, COLOR7 0xE752E8 and SCSI_ 0xE88800 are `rts` followed by the
  0x2048 module-end filler of compiled code (section 7), whereas the
  hand-written LPR_ASM 0xE70A40 is `rts` padded with zeros.  So these are
  compiled empty procedures, not hand stubs.  Whether such a procedure
  had parameters is not visible in its code, but its callers show some
  did: MT_$RELEASE and CT_$RELEASE are called with three by-address
  arguments (unit, mode, status; `../source/domain_os/mt/release.c`,
  `ct/release.c`, PROC2's cleanup), SMD_$PUTC and DMA_$FREE_ASID with
  one (`smd/putc.c`, `dma/free_asid.c`).  An empty *function* is not seen (a
  constant result keeps its frame: IIC_$EXISTS 0xE70AB0 `link #-4; clr.b
  D0`).  The KBD_$PUT comment in `../source/domain_os/kbd/put.c` ("no
  link frame") is not re-checked here.
  - **Implemented** (`crates/dpc-m68k/src/isel.rs`, `select`;
    `dpc_ir::Function::is_empty_procedure`): a procedure whose body has
    no statements is a bare `rts`, with any parameters (by address, or
    by value under `val_param`; a value parameter the callee would copy
    is still refused, as everywhere), and never loads A5 (section 4).
    One with locals is refused: no `link; unlk; rts` is seen, and
    whether such a procedure keeps its frame is not known.  An empty
    *function* keeps its frame.  Byte-identical in `make conform`:
    TIME_$Q_INIT (`time_q_init_queue.pas`), IIC_$INIT (`iic.pas`, now
    the whole IIC module), SMD_$PUTC (`smd_busy_wait.pas`), and
    MT_$RELEASE as a whole one-routine module (`mt_release.pas`: `rts`
    and the 0x2048 module-end filler).

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
  is the last statement goes to the result register.  After other
  statements too, when they only store and nothing is loaded at entry
  (Verified at two sites: IO_$ALLOC 0xE1A9C6..0xE1A9CC, a status store
  then `clr.l D0`; IIC_$RECEIVE_CHECK 0xE70AE2..0xE70AEC, then `clr.b
  D0`; section 9).
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
- **Constant results (Verified at the sites):** `clr.<s> D0` for 0
  (PARITY_$CHK_IO 0xE0B1AA `clr.w D0`; IO_$ALLOC 0xE1A9CC `clr.l D0`),
  `moveq` for -128..127 (0xE0B19C `moveq #1,D0`, 0xE0B23A `moveq #-1,D0`,
  word results), `move.<s> #n` otherwise.  **Implemented**
  (`isel.rs`, `load_const`).
- **Comparisons as boolean results (Verified at four sites; the register
  rule is Uncertain):** the compare, then `s<cc>` into the data register
  the compare read last: AUDIT_$IS_PROCESS_AUDITED 0xE70DA8..0xE70DAC
  `tst.w (-2,A5,D0.w); seq D0` (the index register, D0, so no move);
  ACL_$IN_SUBSYS 0xE470B6..0xE470BA `tst.w (-0x3d5a,A1); sgt D0` (the
  index register of the `lea` before); module-local 0xE464AA..0xE464B2
  `cmpi.w #2,(-2,A0,D1.w); sne D1; move.b D1,D0`; PROC1_$TST_LOCK
  `btst D0,D1; sne D1` (the set's register).  A compare that reads no data
  register gets a new one.  **Implemented** (`isel.rs`, `test_value`) for
  comparisons only; `not`, `and then`, `or else` and boolean variables as
  values, and storing a comparison to memory, are rejected (not
  catalogued).
- **Function calls in expressions (Verified):** the call sequence of a
  procedure call (section 1), then the result read from D0: 0xE04700..
  0xE04704 `bsr.w; move.l D0,(A3)` (not popped: a `bra` to a join and the
  epilogue follow, section 1.1); 0xE0082C..0xE00830 `addq.w #8,SP; tst.b
  D0; bmi.s` (popped, a conditional branch follows); 0xE01BA2..0xE01BA4
  `addq.w #8,SP; move.w D0,D1; lsl.w #2,D1` (the result copied before
  arithmetic, 12.4 R2).  Survey of compiled code (calls followed by a use
  of D0 or A0, ad-hoc script): `move.w D0,...` after the pop ×300, `tst.b
  D0` ×190, `move.l D0,...` ×155; without a pop `move.w D0` ×35, `move.l
  D0` ×31.  **Implemented** (`isel.rs`, `expr_call`; the pop by the rule of
  1.1 for the statement holding the call).  Rejected: a call among another
  call's arguments (the pops are merged, 0xE69FF6, 0xE748E6), two calls in
  one statement, results in A0 (pointer results).  The tail wrapper above
  is the same code (`F := G(args)` with the move to D0 dropped).

---

## 4. Module static data and A5 (Verified; when A5 is set: survey Verified, cause Uncertain)

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
- **When a routine sets A5 (survey, Verified counts; cause Uncertain).**
  `tools/a5survey.py` (bead domain-pascal-acw) splits every compiled map
  `I` segment as `tools/loopalign.py` does (map symbols and `link`
  words, hand assembly excluded) and looks only at the instructions
  reachable from each routine's start, so constants after the code are
  not read as A5 operands.  "Uses" means an A5 operand other than the
  save, the `lea` and the restore, directly or through `bsr` callees
  that do not set A5 themselves (a `bsr` to a routine that sets A5 is
  not a use: 37 `n` routines, e.g. MST_$FIND 0xE0E11E, call such a
  routine and load nothing).  `case` jump tables are followed with the
  `cmpi #N` bound as entry count (targets can precede the table:
  ASKNODE_$INTERNET_INFO 0xE64FA6).  Letters: `S` sets and uses, `s` sets
  and never uses, `n` neither, `i` uses without setting.  (The older
  counts, 585/204/251, came from ad hoc scripts over a narrower set and
  a linear sweep; the numbers below replace them.)
  - 1682 routines in 199 compiled segments; 109 segments load A5.
    Map-named routines in those 109: 628 `S`, 213 `s`, 196 `n`, 10
    frameless (a bare `rts`: SMD_$PUTC 0xE1D89C, SMD_$COLD_INIT
    0xE34F00, PROC2_$SET_STACK_TRUNC_SIZE 0xE414DC, ...), and 1 `i`
    (RTWIRED_PROC_START 0xE87000: the label of the RTWIRED_CODE section's
    start, on a module-local routine that only RIP_$SEND 0xE871B6
    reaches, by `bsr`; it inherits A5 like any internal routine).  A few
    other map names are section labels too (OS_PROC_SHUTWIRED on FILE_
    0xE5D050, reached by `bsr`), so "map-named" slightly overcounts
    exported routines.  Of the 628 `S`, 486 use
    A5 directly, 117 only through a `bsr` callee (LOG_$READ 0xE17800),
    25 through deeper chains.  The 90 other segments' 184 named routines
    never load A5.
  - **What A5 points at.**  Every load in a segment is the same value,
    a `D` block named like the segment, in the data section paired with
    the code section (WIRED_PROC/WIRED_DATA, .TEXT/.DATA,
    OS_INIT_PROC/OS_INIT_DATA, PROC2_CREATE_PRO/PROC2_CREATE_DAT, ...;
    MST_UNWIRED and SMD in the PROC2_* code sections load blocks in
    DATA$).  A module with routines in several code sections has one
    block per data section (PROC2: 0xE7BE84 in .DATA, 0xE35030 in
    OS_INIT_DATA, 0xE86050, 0xE8605C), as the `section` routine
    attribute implies (ref 5.7.1, p. 5-24: "static data from every
    routine ... is assigned to .data", a routine with `[section(c,d)]`
    puts its static data in `d`).
    - 90 segments: A5 = the block's start (PROC1_ 0xE254E8).
    - 18 segments: the block is **4 zero bytes and A5 = its end**
      (OS_INIT_DATA AST_ 0xE35000 -> `lea 0xE35004`; PROC2_CREATE_DAT
      PROC2 0xE86050 -> 0xE86054; XPD .DATA 0xE81810 -> 0xE81814; ...).
      No code addresses those 4 bytes, and every routine of these
      segments that loads A5 is `s`.  The end of the block is the start
      of the next module's block (FIM_WIRED 0xE35004, ACL_ 0xE86054).
    - DIR .TEXT 0xE4AF28: A5 = 0xE7DC00 = block 0xE7DBF8 + 8.
      `(-8,A5)` and `(-4,A5)` hold the addresses of two constants in
      DIR's code (0xE4B33C: a zero long; 0xE4B230: 0x000E0025) and are
      pushed as by-address arguments (`move.l (-4,A5),-(SP)` for
      CRASH_SYSTEM at 0xE5365A).  DIR's code is 0x8FD0 bytes,
      more than `pea (d16,PC)` reaches, which may be why (Uncertain).
    - Why the 4-byte blocks exist and why A5 sits after them is
      Uncertain.  In DIR, too, A5 sits after a header reached at negative
      offsets; whether the 4-byte blocks are an empty such header is a
      lead only.  Three segments have such a block and never load A5
      (TAPE_BOOT 0xE32734, DMA_ 0xE0A328, PEB_UNWIRED 0xE5AD38).
  - **No block, no A5.**  No segment loads A5 without a `D` block of its
    name.  Of the 85 compiled segments with no such block in their
    paired data section, 81 never load A5; the other 4 load their
    module's block in another data section (MST_UNWIRED 0xE739F8 and
    0xE74A74, SMD 0xE75250: DATA$; PCHIST .TEXT 0xE5CA00: WIRED_DATA
    0xE2C204).
  - **`internal` and frameless routines.**  Unnamed routines do not set
    A5 (424 of 450), except 21 that are also called with `jsr abs.l`
    from the same-named module's code in *another code section*
    (0xE14B12 from PROC1_$INIT 0xE2F9EC in OS_INIT_PROC, besides `bsr`
    from PROC1_$BIND; 0xE4194C from PROC2_$CREATE 0xE7295A; 0xE0250C
    from AST_$ACTIVATE_ASTE_CANNED, whose A5 is 0xE35004), and 5 in RING
    that only `bsr` reaches, from routines with the same A5 (0xE76048,
    0xE7667C, 0xE7671C, 0xE767CC, 0xE76B7C; Uncertain).  14 more
    unnamed routines are called across sections and load nothing; all
    35 fit the two modes below like exported routines (counted as
    exported, the switch shapes become 70/21/18 and the 11
    counterexamples are unchanged).  Whether they are global routines
    the map omits or module-local routines that load A5 because a caller
    in another section has another A5 (or none) is Uncertain.  A
    bare-`rts` routine never loads A5.
  - **Two modes, one switch per segment (best supported rule).**  For
    the exported routines with a frame in a segment that loads A5:
    - *used mode:* A5 is loaded iff the routine uses the data (`S` or
      `n`);
    - *always mode:* A5 is loaded whatever the body (`S` or `s`).
    In each segment the routines start in used mode and may switch to
    always mode once; nothing switches back except in the three
    segments below.  (`tools/a5survey.py switch SEG` prints, per
    segment, the last `n` before the first `s` and the first `s`; the
    switch lies between them.  `S` fits both modes.)
    - 70 segments have no `s`: used mode throughout (PROC1_ 0xE1471C
      `nnnnnnnnnSnSSSSSSSSSnnnSSSnn`, MSG_UNWIRED 0xE5911C, TIME_).
    - 20 have `n` then `s`: SMD 0xE1D750 `n.ss` (SMD_$COPY_FONT_TO_MD_HDM
      `n`, then SMD_$BUSY_WAIT and SMD_$LITES `s`); AST_ 0xE00BE8
      (last `n` AST_$INVALIDATE_PAGE 0xE00F16, first `s` AST_$ASSOC
      0xE0444E; AST_$SET_TROUBLE 0xE071EA is after it); PROC2 0xE726EC
      (6 named `n` and a module-local, then PROC2_$WHO_AM_I `s`); FILE_ 0xE73F90; TTY 0xE671DC;
      DISK_ 0xE6B6DC; SMD 0xE6D700; RING 0xE75400; ...
    - 19 have an `s` and no `n` before it (MMAP_ 0xE0C514, first `s`
      MMAP_$ALLOC_CONTIG 0xE0D8F0; XNS_IDP 0xE17BF8, first `s`
      XNS_IDP_$GET_PORT_INFO 0xE18FB8; the 4-byte-block segments).
    - **Counterexamples:** 11 `n` routines after an `s`, in 3 segments:
      PKT 0xE1202C `nnSSsSnnSSSS` (PKT_$COPY_TO_PA 0xE1251C `s`, then
      PKT_$DUMP_DATA 0xE127E6, PKT_$DAT_COPY 0xE12834 `n`); XPD 0xE5AF38
      `nnnnnnnsssssnnnnssss` (XPD_$SET_DEBUGGER 0xE5BBD8 ..
      XPD_$CONTINUE_PROC 0xE5BED8); FILE_ 0xE5D050 (FILE_$SET_PROT_INT
      0xE5DD08 `s`, then FILE_$SET_PROT 0xE5DF3A .. FILE_$SET_DIRPTR
      0xE5E1B2 `n`).  So the mode can switch back; it is a property of
      runs of routines, not strictly once per segment.
    - In used mode, 3 routines address their own block absolutely
      instead of loading A5: AUDIT_$SUSPEND 0xE70DB6 and AUDIT_$RESUME
      0xE70DD6 (`lea 0xE854D8,A0`), REM_NAME_$REGISTER_SERVER 0xE4A4AE
      (`lea 0xE7DBB8,A0`).  Not explained; size is not it, as the
      neighbouring AUDIT_$IS_PROCESS_AUDITED 0xE70D94 reaches the same
      array (`(-2,A5,D0.w)`) after loading A5.  (DISK_$DIAG_IO 0xE6BC18,
      listed here before, reaches 0xE7ACCA in the *other* DISK_ unit's
      block 0xE7A1CC; its own segment loads 0xE826C4.  LOG_$INIT
      0xE30054 pushes the LOG_ address from the OS_INIT unit, which has
      no LOG_ block.  Both are the `extern` case.)
  - **Hypotheses refuted** (counts from `tools/a5survey.py summary`):
    - *Several compilation units per map segment, each "all or
      nothing".*  The routines on both sides of a switch share the same
      data offsets, directly off A5 (AST_: 1012, 1028, 1032, 1056, 1064
      before and after AST_$ASSOC; MMAP_: 0 and 28; XNS_IDP: 4, 8 and
      1336; PMAP_ 0xE12D38, ACL_ 0xE44C3C, DIR 0xE4AF28, SMD 0xE6D700
      likewise), and 267 `bsr` calls to
      unnamed routines cross the first `s` (AST_ 0xE04542 ->
      0xE00C08, ...); `bsr` never crosses an object module (section 7).
      The map does not merge same-named objects either: DISK_ 0xE3BB80
      and DISK_ 0xE3D9AC are adjacent in .TEXT and listed apart, each
      with its own block (0xE7A1CC, 0xE7AD5C).  Splitting at every
      change would also need one-routine units (PKT_$COPY_TO_PA).
    - *The loop-alignment switch (section 7) marks the same point.*  No.
      Of the 33 segments where both switches can be placed, the A5
      switch is strictly earlier in 17, i.e. always-mode routines still
      align loops (RING 0xE75400: A5 in
      (0xE762CA, 0xE76642], loops in (0xE76B7C, 0xE76E22]; DISK_
      0xE6B6DC; SMD 0xE6D700), strictly later in 4 (MAC_OS 0xE0B1BC: A5
      in (0xE0C0CE, 0xE0C350], loops in (0xE0B5A8, 0xE0BB12]; TTY
      0xE1AED0; XPD 0xE5AF38; FILE_ 0xE5D050), and could coincide in
      12.  The two are independent.
    - *Padding at unit ends.*  Only 6 zero words precede a routine
      inside the switch intervals (RING 0xE2FAE0, OLD_DIR 0xE54414,
      ...); nothing systematic.
    - *Data reached only through a callee* (hypothesis 3) accounts for
      142 of the setters (117 + 25 above), which the older survey
      already counted as users; it explains none of the 213 `s`.
      Following `jsr` to the unit's other exported routines adds
      nothing, since those set A5 themselves.
    - *A per-routine feature.*  `s` and `n` routines (named, with a
      frame, in segments that have a data block) do not differ in
      absolute data references (117 of 213 vs 130 of 285),
      or in making calls (171/213 vs 226/285).
      SMD_$BUSY_WAIT, XNS_IDP_$GET_PORT_INFO and TTY_$K_RESET 0xE672DE
      are `s` stubs and wrappers.  Only position predicts the mode.
  - **Reference:** no option or directive is documented for A5 or for
    loading the data base.  The routine options (5.5, pp. 5-15..5-21)
    concern the interface; `nosave` (5.5.8, p. 5-19) is about callee
    saves in assembly routines (its register names are garbled, below);
    `-opt` (6.4.24, pp. 6-26..6-30) lists no such optimisation.  A lead,
    not a finding: with `default_routine_options` (5.6.4, p. 5-22) "the
    options that you define in your list are used as the default routine
    options for every subsequent routine in that module that does not
    specify a routine option", which would give exactly this shape (runs, with
    single routines that keep the other behaviour, as in PKT and
    FILE_ 0xE5D050), but no documented option loads A5.  The cause is
    not recorded in the image.
  - **What `dpc` should do (recommendation, bead domain-pascal-acw).**
    Like loop alignment, make the mode a `dpc` option, since the source
    does not decide it:
    - `--a5=used|always` (default `used`) and
      `--a5-always-from=ROUTINE` (used before ROUTINE in emission order,
      always from it on), plus a per-routine `--a5-always=ROUTINE,...`
      for the three segments whose mode switches back (PKT, XPD, FILE_
      0xE5D050).  The conformance manifest records the switch per unit,
      as it does `--loop-align-off-from`.  Used mode plus the
      per-routine list alone reproduces every observed shape, including
      the switch-backs; `--a5-always-from` only saves listing the runs.
    - In both modes: `internal` routines inherit A5 and never load it
      (exceptions: the 26 unnamed routines above that load A5, which
      would need the per-routine list to apply to `internal` routines
      too);
      a routine compiled to a bare `rts` loads nothing; a unit without
      static data loads nothing in used mode.
    - In used mode an exported routine that does not use the data does
      **not** load A5 (PROC1_$TST_LOCK 0xE148CA, 196 `n`), instead of
      the former rejection.
    - For the open targets: SMD_$BUSY_WAIT needs always mode from
      SMD_$BUSY_WAIT (the routine before it is `n`);
      MMAP_$ALLOC_CONTIG and XNS_IDP_$GET_PORT_INFO need always mode
      for themselves (any switch point up to them reproduces the
      image, as every earlier routine of their segments is `S`).  All
      three blocks are ordinary (A5 = block start).
    - Not covered: the 4-byte blocks with A5 at their end (a unit with
      no static data in always mode); the DIR negative offsets; the 3
      used-mode routines that address their block absolutely instead of
      loading A5 (they would need the opposite override, e.g. a
      per-routine "absolute, no A5" list, or a source construct not yet
      identified).
  - **Implemented** (`crates/dpc-ir/src/lower.rs`, `set_loads_data_base`
    with `dpc_ir::DataBaseOptions`; `crates/dpc-m68k/src/isel.rs`,
    `load_data_base`, `static_base`; `dpc --a5-*`; bead
    domain-pascal-5aw.14), the recommendation above, without
    `--a5=used|always` (`--a5-always-from=` the first routine is
    always mode):
    - *Used mode* (the default): an exported routine loads A5 iff it
      addresses the unit's static data, directly or through `internal`
      routines of the unit it calls (transitively) that do not load A5
      themselves (LOG_$READ 0xE17806, whose worker 0xE17778 reads
      `(0x14,A5)`); a call to an exported routine or to one in the
      `--a5-always` list is not a use.  An exported routine that does not
      use the data loads nothing (PROC1_$TST_LOCK), instead of the old
      "not settled" rejection.  `internal` routines never load A5.  A5
      is set as the first instruction after the register save and saved
      like any callee-saved register (section 12.1): `pea (A5)` alone,
      in the `movem` otherwise.
    - `--a5-always-from=ROUTINE`: from ROUTINE on, in emission order (as
      `--loop-align-off-from`, section 7), every exported routine with a
      frame loads A5.  A bare-`rts` routine never does (section 2).
    - `--a5-always=ROUTINE,...`: these routines load A5 whatever their
      body, `internal` ones included (the 26 unnamed routines above; the
      switch-backs of PKT, XPD, FILE_ 0xE5D050).
    - `--a5-absolute=ROUTINE,...`: these exported routines do not load
      A5; the unit's data is addressed through `lea <block>.l,An`, placed
      where the first static operand needs it, the register (a virtual
      one, so the lowest free: A0 at both sites) kept for the rest of the
      body: REM_NAME_$REGISTER_SERVER 0xE4A4B2..0xE4A4C0 `lea
      0xE7DBB8,A0; move.l 0xE2B0D4,(0x28,A0); st (0x3c,A0)`, two
      statements, one `lea`.  AUDIT_$SUSPEND 0xE70DBA..0xE70DCE places
      the `lea` *between* the index load and its scaling (`move.w
      0xE20608,D0; lea 0xE854D8,A0; add.w D0,D0; move.w
      (-2,A0,D0.w),D1`); that scheduling is not modelled, so an array
      element of the unit's data is refused in such a routine
      (AUDIT_$SUSPEND and AUDIT_$RESUME also need an in-place element
      update, not supported yet).  Only bodies of assignments without
      calls or branches are accepted, the only shapes seen (where the
      base register would live across a call, or where a branch would
      put the `lea`, is not known).  No site addresses its own block with
      absolute operands `(sym+off).l`, so `dpc` has no such form.
    - Errors: a name that is not a routine of the unit; a routine in both
      `--a5-always` and `--a5-absolute`; an empty procedure in either;
      an `internal` routine in `--a5-absolute`; an absolute routine that
      does not address the data, or calls an `internal` routine that uses
      it through the inherited A5; `--a5-always` / `--a5-always-from` in
      a unit without static data (in the image such a unit's A5 points
      past a 4-byte block no code reads, which `dpc` does not lay out:
      not covered, nor are the DIR negative offsets).
    - The conformance manifest records the options per unit as tokens
      (`a5-always-from=`, `a5-always=`, `a5-absolute=`), as it does
      `loop-align-off-from=`.  Byte-identical in `make conform`:
      PROC1_$GET_LOADAV, PROC2_$GET_BOOT_FLAGS, XNS_IDP_$GET_STATS,
      AUDIT_$IS_PROCESS_AUDITED, BAT_$GET_BAT_STEP, FIM_$GET_FIM_ADDR
      (used mode); SMD_$PUTC, SMD_$BUSY_WAIT, SMD_$LITES
      (`smd_busy_wait.pas`, `a5-always-from=SMD_$BUSY_WAIT`);
      MMAP_$ALLOC_CONTIG (`a5-always=MMAP_$ALLOC_CONTIG`);
      XNS_IDP_$GET_PORT_INFO (`a5-always=XNS_IDP_$GET_PORT_INFO`);
      REM_NAME_$REGISTER_SERVER
      (`a5-absolute=REM_NAME_$REGISTER_SERVER`).  The always-mode units
      declare a stand-in variable so that the unit has a data area (each
      `.pas` header says so).
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
- **Constant member of a one-word or one-longword set (Verified;
  implemented):** the set's value loaded into a data register, then `btst
  #k` on it: REM_NAME_SERVER_LOCAL 0xE4A412..0xE4A41A `move.w (22,A0),D0;
  btst #13,D0; sne D0` (a boolean function, `sne` in the set's register);
  in conditions 0xE013BA..0xE013C2 `move.w (190,A2),D0; btst #4,D0;
  beq.w` and 0xE031B2 `btst #14,D5; bne.s`.  Survey of `btst #k` in
  compiled code (ad-hoc script): on a data register that a `move.w` from
  memory loaded, k = 8..15 ×192 and k < 8 ×58; after `move.l`, k >= 16 ×6;
  `btst #k,<memory>` ×422, all with k < 8 (the 68000 takes the bit number
  mod 8 there).  So a word set is loaded and tested whole whatever k is.
  The memory form is taken to be the test of a bit of a byte object (a
  packed field or a byte of a larger set): XPD_$INHERIT_PTRACE_OPTIONS
  0xE5B17C `btst #3,(13,A0); sne D0`, not catalogued.  `dpc`
  (`isel.rs`, `set_bit_test`) implements the register form for sets of
  2 and 4 bytes; a non-constant element on a word set is rejected (`btst
  Dn,Dm` takes the bit mod 32, not catalogued for words), and so is a
  constant element beyond the set's bits (`20 in s` for a word set would
  test a bit of the register above the loaded word; the value is false,
  ref 3.9.3, p. 3-16, but the original's code for it is not catalogued).
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
    6-byte objects are found instead.  But an 8-byte copy *from* a slot
    with only the source post-incremented exists: DIR_$ADD_MOUNT
    0xE534CA..0xE534D2 `movea.l (8,A6),A0; move.l (A0)+,(-0xb4,A6);
    move.l (A0)+,(-0xb0,A6)`, again at 0xE534DC (noticed in passing,
    bead domain-pascal-5aw.2; not counted by the survey above, which
    looked for `(Am)+,(An)+`; Uncertain).
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
- **1-based arrays (Verified):** the compiler folds the `-1` into the
  displacement, `(-2,A0,Dn.w)` (AUDIT_$SUSPEND 0xE70DC8;
  AUDIT_$IS_PROCESS_AUDITED 0xE70DA8 `tst.w (-2,A5,D0.w)`) or the `lea
  (2,An)` / `(-2,An)` pair in 0xE17778's word-copy loop
  (0xE177E0..0xE177F2): a loop pointer starts at `base + from × size`
  and the accesses fold `-lo × size` (section 7.4; PKT_$RECENTLY_MISSING
  0xE128D6 `lea (8,A5),A0`, `(-8,A0)`).
- **Index scaling (Verified):**
  - Multiply by 2 is done with `add.w Dn,Dn` (0xE70DA6); by 4 with `lsl.w
    #2` or `lsl.l #2` (FIM_$GET_FIM_ADDR 0xE0AA16, 0xE0B26C, 0xE13942).
  - Other sizes: the constant multiplication of section 8 (shifts and adds
    for 2^s (2^k ± 1), e.g. ×72 at DISK_$REVALIDATE 0xE6C072..0xE6C078;
    otherwise `mulu.w #size`, BAT_$GET_BAT_STEP 0xE3BB6A `mulu.w #0x234`,
    `proc2/suspend.c` 0xE412AE `mulu.w #0xE4`).  The `idx*0xF8` =
    `(idx<<8)-(idx<<3)` of `vtoc-disk-layouts.md` is the ×248 form of
    section 8 (`lsl #3; move; neg; lsl #5; add`, 0xE38994).
  - `mulu.w #c` and `muls.w #c` (survey, ad-hoc script: what reads the
    product): `mulu.w` feeding a `.w` index ×94 (0xE0B2D0), a `.l` index
    ×43 (0xE39364, after an unsigned compare of the index); `muls.w`
    feeding a `.l` index ×118 (0xE1D7A2); no `muls.w #c` feeds a `.w`
    index.  So the product's sign matters only for `.l` indices.
  - There are no 68020 scaled-index modes in compiled code.
- **Element operand (Verified at the sites; implemented in `isel.rs`,
  `index_ea`):**
  - displacement (array offset, field offset and the lower bound folded
    together) within -128..127: the indexed mode `(d,An,Dn.w)`
    (FIM_$GET_FIM_ADDR 0xE0AA18 `move.l (0x3c,A5,D0.w),D0`; 0xE70DA8;
    DISK_$REVALIDATE 0xE6C07A `pea (-0x48,A0,D0.w)`; FIM_$FREE_ASID
    0xE0AA8E `clr.l (0x3c,A5,D0.w)`);
  - beyond: `lea (0,An,Dn.w),Am`, then `(d,Am)` (BAT_$GET_BAT_STEP
    0xE3BB6E..0xE3BB72 `lea (0,A5,D1.w),A0; move.w (-0x220,A0),D0`;
    ACL_$IN_SUBSYS 0xE470B2; MMAP_$UNAVAIL_REMOV 0xE0CC4C `lea
    (0,A0,D0.l),A2; ...; pea (-0x2000,A2)`).  Two accesses to one element
    share the `lea` even with a small displacement (0xE71BCA..0xE71BD2,
    `(-26,A0)` and `(-25,A0)`): `dpc` has no such CSE.
  - the index register: an index read from memory (a variable of the
    unit or of another unit) is scaled in place (0xE0AA10..0xE0AA16
    `move.w PROC1_$AS_ID,D0; lsl.w #2,D0`; 0xE70DA0..0xE70DA6); a
    by-value parameter is loaded, **copied**, and the copy scaled, in a
    one-statement body as well (BAT_$GET_BAT_STEP 0xE3BB64..0xE3BB6A
    `move.w (8,A6),D0; move.w D0,D1; mulu.w #0x234,D1`; module-local
    0xE4649C..0xE464A8 `move.w (8,A6),D0; movea.l #abs,A0; move.w D0,D1;
    add.w D1,D1`) and when it was loaded at entry (FIM_$FREE_ASID
    0xE0AA8A `move.w D2,D0; lsl.w #2,D0`).  With element size 1 the index
    register is used as it is.
  - **Uncertain: `.w` or `.l`.**  Word indices appear as `.w`
    (all the sites above) and as `.l` after `ext.l` (0xE01D10..0xE01D16
    `move.w D4,D2; ext.l D2; lsl.l #3,D2; move.l (-56,A6,D2.l),-(SP)`;
    0xE074E8..0xE074EE; about 177 `ext.l` followed within three
    instructions by a `.l` index), both on small arrays.  In every `.l` case seen the
    index is a value held in a register (a register variable); in every
    `.w` case it was read from memory or is a parameter.  `dpc` holds
    few local variables in registers (section 12.5) and none as an index
    so far, so it uses `.w` for word indices and
    `.l` for longword ones; the rule is not confirmed.  A `.w` index is
    sign-extended by the 68000, and a word index is scaled at the word's
    width, so `dpc` takes a word index only when every index × element
    size of the array's bounds fits in -32768..32767 (an element of a
    `0..65535`-indexed array of 40001 chars, or of an `array[0..20000] of
    integer`, would be addressed wrongly); other word indices are
    rejected (review of bead domain-pascal-5aw.13).
  - **Another unit's array (Uncertain; rejected):** the base is loaded
    with `movea.l #imm,An`, but the immediate is not the array's address:
    ACL_$IN_SUBSYS 0xE470AA `movea.l #0xe97294,A0`, then `(-0x3d5a,A1)`;
    module-local 0xE464A0 `movea.l #0xe2622c,A0` (PROC1_$TYPE is
    0xE2612C), then `(-2,A0,D1.w)`; DISK_$REVALIDATE 0xE6C06A.  How the
    original split the address between the immediate and the displacement
    is not known.  `lea abs.l,An` occurs too (12 sites, AUDIT_$SUSPEND
    0xE70DC0) against 1731 `movea.l #abs,An`.  The base load is scheduled
    into the index computation (after the index's first instruction:
    0xE4703E..0xE47058, 0xE470A4..0xE470B2, 0xE4649C..0xE464A6,
    0xE6C066..0xE6C070), which fits the R11 rule of section 12.4.
  - **Bounds checks:** none in the image; the C of FIM_$FREE_ASID
    (`fim/free_asid.c`) notes "the original performs no bounds check".
    `dpc` emits none.
  - Arrays through a pointer are supported when the pointer is held in an
    address register since the entry; others are rejected (where the
    pointer load goes relative to the index is not catalogued).  A byte
    index (char, boolean) is rejected (its extension is not catalogued).
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
  AST_$PAGE_ZERO.  Given body order (next item), this is source order:
  the conformance source defines the worker first
  (`tests/conformance/ast_page_zero.pas`).  Calls *after* the callee are also seen
  (AST_$TOUCH_AREA 0xE03548 calls 0xE00EB0 with `bsr.w` at 0xE03636), so
  a worker does not have to precede every caller.
- **Top-level routines are emitted in body order (Verified).**  Pascal
  needs a routine declared before it is called, so a `bsr` to a *later*
  address in the same module means the callee's body follows its
  caller's while its declaration (`forward`, or an `extern` heading from
  an included `.ins.pas`, ref 5.5.2 p. 5-16, 7.3.4 p. 7-11) came earlier:
  the routine is emitted at its body, not at its declaration.  Survey
  (`tools/a5survey.py order`; compiled segments only, routines split as
  in section 4), `bsr`s to another routine of the same segment:

  | direction | caller | callee | sites | caller/callee pairs | segments |
  |---|---|---|---|---|---|
  | backward | map-named | map-named | 306 | 273 | 48 |
  | backward | map-named | module-local | 781 | 607 | 57 |
  | backward | module-local | map-named | 47 | 39 | 13 |
  | backward | module-local | module-local | 466 | 330 | 26 |
  | **forward** | map-named | map-named | 122 | 112 | 30 |
  | **forward** | map-named | module-local | 90 | 81 | 7 |
  | forward | module-local | map-named | 19 | 16 | 10 |
  | forward | module-local | module-local | 40 | 38 | 8 |

  The two bold rows are the evidence: a map-named caller is a top-level
  routine (the map lists global symbols only, and a nested routine is
  always a local symbol: ref 5.5.4 p. 5-17; "internal routine", p.
  9-26), and its callee cannot be a routine nested in it (those come
  before it), so the callee was declared ahead of the call and defined
  after it.  The map-named -> module-local row is the cleanest, since an
  unexported callee is necessarily in the same unit.  (The map-named ->
  map-named row could also hold calls between two units bound into one
  map segment, if the binder resolved a `bsr` to a global symbol: no
  `bsr` crosses a map segment, section 7, but whether one crosses units
  inside a segment is not checked.)  The map-named -> module-local row
  does not rest on the nested-before-parent rule either: 21 of its 53
  callees are called by two or more map-named routines, so are nested in
  none of them (0xE70D38 by AUDIT_$SHUTDOWN and AUDIT_$CONTROL; 0xE3EB8C
  by PROC2_$AWAKEN_GUARDIAN, PROC2_$SIGNAL, PROC2_$SIGNAL_OS and
  PROC2_$ACKNOWLEDGE; 0xE5E8E0 by 12 FILE_ routines, forward from
  FILE_$DELETE 0xE5DB6E `bsr.w`).  Sites: AUDIT_$SHUTDOWN 0xE70D1E
  `bsr.s 0xE70D38` at 0xE70D2E (the module-local routine follows directly, `link.w A6,#-12` at 0xE70D38);
  PROC2_$AWAKEN_GUARDIAN 0xE3E960 `bsr.w 0xE3EB8C` at 0xE3E9AC, while
  0xE3F0F2 and 0xE3F428 call the same module-local 0xE3EB8C backward;
  AREA_$TRANSFER 0xE08098 `bsr.w 0xE08816` at 0xE08164 and 0xE081D6
  (checked with `tools/imgdis.sh`).  Rows with a module-local caller are
  not counted as evidence: that caller may be a nested routine, which
  can call its enclosing routine (emitted after it) without any forward
  declaration.  Forward `bsr`s are sized like backward ones (section
  1.6): of the 212 from map-named callers, all 54 `bsr.s` are in reach
  and none of the 158 `bsr.w` is.  Nothing in the image could show
  emission at declaration position: a routine with no forward declaration
  has its declaration and body in one place, and a backward call fits
  either order.
  **Implemented:** `dpc-sema`'s `Program::top_routines` places a routine
  completed after a `forward`/`extern` heading at its body, and
  `dpc-ir`'s `Module::functions` (the emission order) follows it; a
  routine with no body in the unit is not emitted
  (`tests/golden/calls/body_order`).  Nested routines keep their heading
  order in `Routine::nested` (dpc refuses nested routines for now).

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
  all (0xE00EB0: only A0 and the `dbf` count D0).  Survey and the
  pointer's start (`base + from × size`, the `-lo × size` folded into the
  displacement): section 7.4 (wider elements stepped, 104 loops; byte
  arrays indexed with the index register, 84).
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
  `for` loops: section 7.4.  Byte-identical in
  `make conform` (`tests/conformance/ast_page_zero.pas`).
- **Run-time count:** `subq.w #1,Dn; bmi skip` before entering the `dbf`
  loop (0xE177DC..0xE177F2); the other guards, and when the count is
  copied into a counter of its own: section 7.4.
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
    code section, and the unit's routine order within one, which is the
    order of the routines' bodies: a routine declared `forward` (or
    `extern`) earlier is placed at its body (section 6, "Order",
    Verified).  A routine that does not align gets no
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
      routines, `while` heads (entered by a `bra` to the bottom test)
      split 29/27/2 (filler/aligned/2 mod 4), the two unaligned ones heads
      of loops over 0x100 bytes; `dbf` heads entered by fall-through
      229/188/15 (section 7.4, `tools/loopalign.py heads`).  (An earlier
      count, 3/2/3 against 312/158/25, took a head preceded by the filler
      for one entered by fall-through.)
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
        (XNS_IDP_$GET_STATS), the DISK_ segment 0xE3BB80 from 0xE3BC40 on
        (the module-local 0xE3CCCE).
      - Aligning: the DISK_ segment 0xE6B6DC up to DISK_$LV_MOUNT
        0xE6CA3A (DISK_$UNASSIGN_ALL).
- **`case` statements:** a jump table of word offsets or a compare
  chain; section 7.3.
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
  return end`), the original threaded the branch or laid the code out
  otherwise; which is not known.  dpc rejects these (`select_body`,
  `Selector::jump`).
- **`if c then return` (Verified; implemented):** one conditional branch
  on `c` itself to the epilogue, not a branch past a `bra`: TERM_$CONTROL
  0xE66B3A..0xE66B3C `tst.l (A3); bne.w 0xE66D78`, 0xE62E88, 0xE68C08
  (section 9).  No conditional branch over a `bra` exists in compiled
  code (survey of every routine `tools/regalloc.py` splits; ad-hoc
  script), against 888 conditional branches to an epilogue.  `dpc`
  (`Selector::if_`) branches to the epilogue on the condition, each
  operand of an `or else` on its own (`if a or else b then return`:
  `b<a> epi; b<b> epi`), and an `else` part follows the branch.  A `then
  return` that ends the routine stays refused (below).
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
  A computed left operand (an expression, a function result, an array
  element's value) is compared in its register with `cmpi #c,Dn` (0xE1216E
  `add.l D6,D7; cmpi.l #0x500,D7`) or `cmp.<s> x,Dn` (`compare_computed`);
  against 0 it is rejected (the flags of the operation, or a `tst`?).  A
  function's boolean result is tested with `tst.b D0` (section 3); `k in
  s` for a constant k with `btst #k` (section 5.1).  Not supported yet: a branch to a `bra` (above); comparing booleans
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
| `b<cc>` | a `bra` (to the test) | 213 | `while`: `bra test; head: body; test: <cond>; b<cc> head` (0xE01A8E..0xE01A98; section 7.4) |
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
  a `bra` to the test; the head after the `bra` is aligned like any loop
  head (in aligning routines 29 with the filler, 27 4-aligned, 2 at 2 mod
  4, both large; bead domain-pascal-ju5).  **Implemented**: section 7.4.


### 7.3 `case` statements (Verified at the sites given; the choice of form Verified by survey; implemented)

Survey: `tools/casesurvey.py` (bead domain-pascal-5aw.4) follows every
compiled routine as `tools/a5survey.py` does (jump tables through
`a5survey.table_entries`) and finds **50 jump tables** and **61 compare
chains** (`summary`; `list` gives one line per site with the number of
labels n, their range r = max - min + 1, and the arms).  A chain is two
or more `cmpi #k,Dn; beq` / `tst Dn; beq` on one register followed by a
`bra`, with at least two distinct targets (runs of `beq`s to one target
are conditions, `x in [a, b]`-like, and not counted).  For a table, n
counts the entries that do not go to the default target: a lower bound,
since an empty arm (or an arm merged with the `otherwise` code) shares
it.

**Jump table** (Verified, all 50):

```text
        move.w  <selector>,D0           ; OSINFO_$GET_MMAP 0xE5C6C0
        subq.w  #lo,D0                  ; when the labels start at lo != 0
        cmpi.w  #r,D0                   ; 0xE5C6C4: r = number of entries
        bcc     <otherwise or join>     ; unsigned: below lo is caught too
        add.w   D0,D0
        move.w  (T,PC,D0.w),D0          ; 303b 0006
        jmp     (T,PC,D0.w)             ; 4efb 0002
T:      .short  ARM0-T, ARM1-T, ...     ; 0xE5C6D6: 000a 0026 0032 00a4 006c
```

- **The bound** is always `cmpi #r` with r the number of entries and
  `bcc` (49 `cmpi.w`; one `cmpi.l` on a longword selector,
  ASKNODE_$INTERNET_INFO 0xE64F92, still `add.w`).  The index is a word.
- **The lower bound** is subtracted first: `subq.w #k` for 1..8
  (PROC1_$CREATE_P 0xE15178 `subq.w #3`; DMA_$CHECK 0xE0A3D2 `subq.w
  #1`), `subi.w #k` above (ACL_$SERVER 0xE495CE `subi.w #100`; DIR_$DO_OP
  0xE4C266 `subi.w #42`; UNMAP_CASE 0xE541E8 `subi.w #48`), `addq.w #4`
  for -4 (MST_$PRIV_SET_TOUCH_AHEAD_CNT 0xE44534).  Nothing lower than -4
  is seen.
- **The table** follows the `jmp` directly, not aligned (tables at 0 and
  at 2 mod 4: 0xE5C6D6, 0xE0A3E4), and the first arm follows the table
  (49 of 50; ASKNODE_$INTERNET_INFO 0xE64FA6's entries go back to the
  arms of an earlier chain, shared code).
- **Gaps:** a value with no label goes to the `otherwise` part, or to the
  join without one, like the `bcc` (MAC_OS_$OPEN 0xE0B3AE: labels 1 and
  2 to 0xE0B3D0; KBD_$RCV 0xE1CCFE: 0, 7, 8, 9 to the join 0xE1CE1E).

**Compare chain** (Verified, all 61):

```text
        move.w  (4,A0),D0               ; SYSBUS_$INIT 0xE0AB64
        beq.s   ARM0                    ; label 0 first: the load's flags
        cmpi.w  #1,D0
        beq.s   ARM1
        cmpi.w  #2,D0
        beq.s   ARM2
        bra.s   <otherwise or join>     ; 0xE0AB76
```

- **Order:** the labels are tested in source order, not sorted: the
  module-local 0xE1CC70 tests 0xC0, 0x91, 0x92, 0x95, 0x96, 0x99, 0x98,
  0x97 (the last three for one arm); TTY_$I_SIGNAL 0xE1B832 3, 2, 21, 1,
  26, 22.  In 59 of the 61 chains the arms lie in memory in the order of
  their first test; the 2 others share code between arms (0xE556E6,
  0xE657DA).
- **Label 0** is `tst.<s> Dn; beq` (TIME_$GET_EC 0xE16762, after a
  compare; 3 chains start so, all right after a `clr.w Dn; move.b x,Dn`
  zero-extension: 0xE556E6, DIR_$OLD_READ_LINKU 0xE57890,
  REM_FILE_$SERVER 0xE6429A), but first right after a word or longword
  move into Dn it is the `beq` alone, on the flags of the move (17
  chains: SYSBUS_$INIT 0xE0AB64, XPD_$PUT_REGISTERS 0xE5C3A0,
  TERM_$GET_EC 0xE723F6 `move.w (A2),D0; beq.s`; copies too, MSG_$WAITI
  0xE59C84 `move.w D0,D2; beq.s`, DISK_$WRITE_PROTECT 0xE3D980; 0xE01B58
  `move.l (A2),D0; beq`).  After a copy the compares go on in the copy
  (0xE59C84 `cmpi.w #1,D2`) or, twice, in its source (MSG_$SARI 0xE59EF6
  and module-local 0xE75656 `move.w D0,D1; beq.s; cmpi.w #1,D0`, both
  right after a call: a function result kept in D1 and dispatched from
  D0?  Not explained; cf. CAL_$WEEKDAY below).
- **Widths:** 58 chains `cmpi.w`, 3 `cmpi.l` (status values, 0xE5D2FE
  FILE_$SET_ATTRIBUTE).
- **An empty arm's labels** go to the join (ASKNODE_$INTERNET_INFO
  0xE65614 `cmpi.w #81,D0; beq.s 0xE6568A`, the target of the chain's
  final `bra`).

**Which form (Verified by survey, one exception).**

- **n <= 3: always a chain.**  All 49 chains of 2 or 3 labels, 32 of them
  dense (r = n: SYSBUS_$INIT 0..2, DISK_INIT 0xE19B24 3..5, RIP_$SERVER
  0xE68B62 1..3); no table has fewer than 4 labels (the 4 tables that
  count fewer have labels hidden in the default: SMD_$INIT 0xE34D5E
  dispatches a constant 0 over 4 entries; 0xE5197C, 0xE4DDB2, 0xE4DE7A
  subtract a lower bound whose label goes to the default).
- **n >= 4:** tables have r/n <= 2.71 (module-local 0xE63376 19/7,
  module-local 0xE6A766 59/22, 0xE1077A 15/6; most are near 1); chains
  have r/n >= 4.33 (TTY_$I_SIGNAL 26/6, REM_FILE_$SERVER 0xE6429A 139/29,
  0xE63A02 139/26, module-local 0xE1CC70 48/8, ASKNODE_$INTERNET_INFO
  0xE655D8 72/12, 0xE657DA 25/4, ...) **except one:** the 41-label chain
  of ASKNODE_$INTERNET_INFO 0xE6463E over 2..91 (r/n 2.20) where a table
  is predicted (ASKNODE_$SERVER 0xE65A5C, 44 labels over 92, and
  DIR_$DO_OP, 26 over 51, are tables).  Not explained: the size of the
  table (180 bytes) does not separate it from ASKNODE_$SERVER's (184);
  the selector is in D2 there (`move.w (A2),D2`, 0xE6463C), not D0.
  **Uncertain:** the threshold between 2.71 and 4.33; no site lies there.

**Arms and the join (Verified).**

- Arms follow the dispatch (the table, or the chain's `bra`) in source
  order as far as can be seen (the chains above; for tables the source is
  not known: OSINFO_$GET_MMAP's arm for label 4 precedes label 3's,
  0xE5C742/0xE5C77A, and the C reconstruction has them sorted).
- Each arm ends with a `bra` to the join, but **the last arm falls
  through** when there is no `otherwise` (CAL_$WEEKDAY 0xE6872E `moveq
  #6,D0` into the epilogue at 0xE68730; OSINFO_$GET_MMAP 0xE5C796;
  SYSBUS_$INIT 0xE0AB9C..0xE0ABAA).
- **The `otherwise` part comes after all the arms** and falls through to
  the join; the last arm branches over it (MAC_OS_$OPEN 0xE0B3C8 `bra.s
  0xE0B3DC` over 0xE0B3D0..0xE0B3DA; PROC1_$CREATE_P 0xE151AA `clr.w
  (-6,A6)` then the join 0xE151AE; module-local 0xE1CCBA `move.b D1,D0`
  after the arms, reached by the chain's `bra.s 0xE1CCBA`).  In all 21
  tables with an `otherwise` part it follows the last arm (at 0xE6A766
  one entry points past it: the jump arm of the next item).
- An arm that is only a jump goes to the jump's target directly: the
  table of module-local 0xE6A766 sends `' '` to 0xE6A852, the first
  instruction after the enclosing `dbf` loop (an `exit`).  `return`
  arms are not seen.
- A `return` at the end of an arm is its `bra` to the epilogue
  (TERM_$CONTROL 0xE66CEA..0xE66CF0, an `otherwise` part).
- **No match and no `otherwise`:** execution continues at the join
  (every table's `bcc` and every chain's `bra`): Domain Pascal "does not
  detect the lack of a case statement constant corresponding to a
  run-time case value" (ref Appendix D, ISO 6.8.3.5, p. D-4).
- **The join is not aligned**, except TERM_$CONTROL 0xE66D6E (a 0x2048
  before the join 0xE66D70, also the target of labels 9 and 24, empty
  arms); TERM_$CONTROL also has fillers at 0xE66C7E and 0xE66C86 before
  joins of its inner chains.  Not explained, not modelled.
- **Shared code (Verified that it happens, not modelled):** identical
  arm tails are merged ("Merges identical sequences of instructions",
  ref 6.4.24 p. 6-27): AS_$GET_ADDR 0xE5842A `bra.s 0xE58470` into
  another arm's tail; SMD_$GET_EC 0xE6FDF2..0xE6FE12 and ROUTE_$GET_EC
  0xE69C8E..0xE69CAE share the `jsr EC2_$REGISTER_EC1` of all arms;
  TERM_$CONTROL's two chains share one `otherwise` part (0xE66C72 and
  0xE66CDC both `bra.s 0xE66CEA`).

**The selector (Verified).**

- It is evaluated into a fresh data register, the lowest free (D0 at
  most sites; D1 at SUMA_$RCV 0xE1AD2A, where D0 holds a parameter
  loaded at entry, and at TTY_$I_RCV 0xE1B9AC), which the dispatch
  destroys.  A value already in a register is copied first, for tables
  (NETWORK_$SET_SERVICE 0xE0F48E `move.w D2,D0`; NETWORK_$PAGE_SERVER
  0xE116E2) and chains (TTY_$I_SIGNAL 0xE1B82C..0xE1B830 `move.w
  (0xc,A6),D0; move.w D0,D1`, the parameter loaded at entry and copied).
- A `char` (unsigned byte) selector is widened with `clr.w Dn; move.b
  x,Dn` (module-local 0xE1CC6C, ACL_$SERVER 0xE495C8, REM_FILE_$SERVER
  0xE6429A), then compared at word size.
- Computed selectors are seen (`move.w #0xf0,D0; and.b (1,A3),D0; lsr.w
  #4,D0` at KBD_$RCV 0xE1CCE2; `moveq #7,D0; and.b (A2),D0` at 0xE4F17C).

**Register use in the arms (Uncertain).**  The selector's register is
dead in the arms and is reused there in most chains (TERM_$CONTROL
0xE66C74, DTTY_$INIT 0xE34C16, XPD_$PUT_REGISTERS 0xE5C3B2; ad-hoc scan
of the arms of every site, not kept), but in the module-local 0xE1AAA8 (TERM_$STATUS_CONVERT in
`../source/domain_os/term/status_convert.c`) the first arm computes in
D0 and the two others in D1 (0xE1AAD2, 0xE1AADE, 0xE1AAEA), and in
TERM_$GET_EC the second arm uses D1/D3 (0xE72420) where the first used
D0/D1.  dpc takes the lowest free register (12.4 R1), D0 in every arm;
those two units do not compile to the image's bytes (see 12.4 R6).

**Single labels:** a `case` with one label is not told apart from an
`if` in the image.  A chain whose last test's arm is the first code after
the `bra` would be a conditional branch over a `bra`, which compiled code
does not have (7.1): not seen.

**Implemented** (bead domain-pascal-5aw.4; `crates/dpc-ir`
`StmtKind::Case`, lowering in `lower.rs` `Cx::case`;
`crates/dpc-m68k/src/isel.rs` `Selector::case`; `Insn::TableIndex`,
`TableJump`, `TableEntry` in `insn.rs`, the table's targets as the
dispatch's successors in `regs.rs`, entry reach checked in `emit.rs`
`size_branches`): a table when n >= 4 and 7r <= 19n (r/n <= 2.71), a
chain when n <= 3 or 3r >= 13n (r/n >= 4.33); the forms, bounds, label-0
tests, arm order, fall-through and `otherwise` placement above; `char`
selectors widened.  Refused ("not supported yet"): r/n between 2.71 and
4.33 with n >= 4; one label; a chain whose last label's arm comes first;
a chain whose arms are all empty (its `bra` would go to the `otherwise`
part right after it);
label 0 tested first on a computed selector (other than a `char`
widening); a constant selector (SMD_$INIT's, one site); a signed byte
selector (boolean); a function call in the selector (CAL_$WEEKDAY
0xE686F6 compares D0 after `move.w D0,D1`, not explained); a table on a
longword selector not starting at 0, or whose lowest label is below -8;
an empty `otherwise` part; parts ending in calls of the same argument
bytes when a pop is needed (shared pop, 1.1); arms that are only a jump
(`return`: refused by the branch-to-`bra` rule of 7.1, as is a `case` at
the end of an `if` part followed by a `bra`); label ranges
(`1..3:`, reference-clarifications Q7, open).  Goldens
`tests/golden/control/case_table.pas`, `control/case_chain.pas`; tests
`crates/dpc-m68k/tests/case.rs`.  **No conformance unit yet:** every
small compiled routine with a `case` needs something else first
(`tools/casesurvey.py list`, routines of up to 0xC0 bytes): shared arm
tails (AS_$GET_ADDR, SMD_$GET_EC, ROUTE_$GET_EC), pointer results in A0
(TIME_$GET_EC 0xE1670A: EC2_$REGISTER_EC1), the D1 arms of 0xE1AAA8
(above), function results assigned in arms (CAL_$WEEKDAY, module-local
0xE1CC64), nested routines (IO_ 0xE1A4DC, `movea.l (A6),A0`), loops
(module-local 0xE33778), an `in`-style chain and bit operations on memory
(DISK_$WRITE_PROTECT 0xE3D956 `bset #0,(165,A1)`).  OSINFO_$GET_MMAP
(0xE5C694, 0x36C bytes) is blocked at its first instructions: its seven
by-address slots are loaded at entry into D3, D4, A2, D5, D6 and moved to
A0/A1 for each use (`movea.l D6,A0; clr.l (A0)` at 0xE5C6B0), which dpc
does not do (12.2: a pointer kept in a D register); then `btst #5,(1,A1)`
on a byte in memory (5.1, not catalogued), `while` loops (7.2), local
variables in registers (12.4), and the `asid` selection repeated in
three arms with `cmpi.w #0x40; bls` joins.

### 7.4 Loops: `while`, `for`, `exit`/`next` (Verified shapes and counts; implemented in part)

Beads domain-pascal-5aw.15, -ju5, -96r.  Survey: `tools/loopalign.py
heads` and `tools/loopalign.py fors` (every compiled routine split as in
section 7; a loop head is the target of a `dbf` or of a backward branch;
a routine's class comes from `switch`: **al** at or before the last
routine of its segment with a filler, **na** at or after the first later
one with a head at 2 mod 4 and no filler, **?** in between).  Ad-hoc
scripts besides, not kept, are named as such below; re-check sites with
`tools/imgdis.sh`.

**Loop heads by shape** (`heads`; F filler, A 4-aligned, U 2 mod 4):

| Back branch | Entry | al F/A/U | na F/A/U | ? F/A/U |
|---|---|---|---|---|
| `dbf` | fall-through | 229/188/15 | 0/237/224 | 0/69/0 |
| `b<cc>` | fall-through (`repeat`) | 36/21/5 | 0/42/37 | 0/9/0 |
| `b<cc>` | a `bra` to the bottom test (`while`) | **29/27/2** | 0/58/68 | 0/8/0 |
| `b<cc>` | a `bra` elsewhere | 2/1/1 | 0/4/6 | |
| `bra` | fall-through | 18/19/5 | 0/30/21 | 0/4/0 |

- **`while` heads are aligned like every loop head (Verified; settles
  bead domain-pascal-ju5).**  The earlier "3/2/3" for heads entered by a
  branch counted only heads whose *preceding* instruction is the `bra`,
  so every head with a filler between the `bra` and the head was counted
  as entered by fall-through.  With the filler skipped, aligning routines
  have 29 `while` heads with the filler (MAC_OS_$CLOSE 0xE0B4CC `bra.s
  0xE0B4FC; movea.l A0,A0`, head 0xE0B4D0), 27 4-aligned and 2 at 2 mod 4,
  both heads of loops over 0x100 bytes (0xE6657E, 0x192 bytes;
  0xE6AB5A, 0x460), like the large-loop exceptions of section 7.  Non-
  aligning routines never have the filler (58 aligned, 68 not).
- **`while c do body`** (Verified: 213 such loops):

  ```text
          bra     TEST
          <filler>                ; aligning routines, head at 2 mod 4
  HEAD:   body
  TEST:   <branch to HEAD when c holds>
  EXIT:
  ```

  The DISK_ module-local 0xE3CCCE (`disk/write_multi.c`,
  `disk_$set_chain_status`): `movea.l D0,A1; bra.s 0xE3CCE4; move.l
  (A0),(0xc,A1); movea.l (A1),A1; cmpa.w #0,A1; bne.s 0xE3CCDE`.  The test
  is the condition's jumping code (7.1) with the last branch taken on
  success: `and then` leaves on its first operand's failure and branches
  back on the second's success (NAME_$VALIDATE's module-local
  0xE4A028..0xE4A034 `cmp.w (A1),D0; bcs.s <exit>; move.w (A1),D1;
  cmpi.b #47,(-1,A0,D1.w); beq.s <head>`).
- **`while p <> nil` on a pointer variable** (ad-hoc script over the
  bottom tests): 45 loops test an address register (`cmpa.w #0,An`), 12 a
  frame slot (`tst.l (d,A6)`), a few a D register or the flags of a
  `move.l`.  So a list walk's pointer local is usually in an address
  register (12.5).
- **`repeat`**: fall-through heads, aligned (section 7.2).  The `bra`
  rows (a test at the top, a `bra` back; 7.2): source construct not
  identified; `dpc` never emits it.

**`for` loops** (`fors`; 963 `dbf` loops):

- **A counted loop is a `dbf` loop with a count of N-1** (section 7).
  With **constant bounds** the count is `moveq #N-1` (558 loops) or
  `move.w #N-1` above 127 (47; ad-hoc script: the last definition of the
  `dbf` register before the head).
- **The index lives in a register when the body uses it** (Verified):
  307 `dbf` loops step an index register at the bottom (`addq.w #1,Dn`;
  `subq.w #1,Dn` for `downto`: 0xE3C99A `subq.w #1,D1; subq.l #4,A1`,
  0xE6CAAC, 0xE6CF56), 13 an index in memory (`addq.w #1,(d,A6)`, or
  through a static link in nested routines: 0xE721F8 `move.w
  #1,(-2,A2)` ... `addq.w #1,(-2,A2)`; SOCK_$INIT 0xE2FE54 in its own
  frame, not explained).  The register is separate from the `dbf`
  counter: MST_$ALLOC_ASID 0xE42D50 `moveq #57,D0; movea.l #abs,A1; clr.w
  D2`, the body, `addq.w #1,D2; dbf D0`.  The index's frame slot is not
  written in the register case.
- **Preheader order: the count first, then the index** (Verified: 116 of
  118 loops with a constant count and the index's initial value in the
  preheader; `clr.w Di` for 0, `moveq #k,Di` otherwise).  Invariant
  address loads may come between (0xE42D52; PROC2's 0xE42238
  `moveq #69,D0; movea.l #abs,A0; moveq #1,D1; addq.l #8,A0`; MAC_$NET_
  TO_PORT_NUM 0xE0C372 `moveq #7,D0; clr.w D1; movea.l #abs,A2`): their
  place is not settled.  An index whose initial value equals the count is
  *copied* from the count's register: 0xE6A8BC `moveq #19,D2; move.w
  D2,D1` (the loop at 0xE6A8BC, 20 iterations, the index stepped down);
  FLP_$SHUTDOWN 0xE3E24A copies a register-held 0 likewise (`move.w
  D0,D2`; constant CSE, 12.4 R7).
- **The step**: the index first, then the pointers (below), then `dbf`
  (PKT_$DUMP_DATA 0xE12822 `addq.w #1,D3; addq.l #4,A2; dbf D2`; 0xE0C388;
  0xE4225E; 0xE75298).
- **Calls in the body**: the count and the index are values live across
  the call, so callee-saved (12.4 R3): DISK_$UNASSIGN_ALL 0xE6BE24
  `moveq #9,D2; moveq #1,D3`, the body `move.w D3,(-6,A6); pea (-4,A6);
  pea (-6,A6); bsr.w DISK_$UNASSIGN; addq.w #8,SP`, then `addq.w #1,D3;
  dbf D2` (Verified; reproduced, `tests/conformance/disk_unassign_all.pas`).
  SMD_$FREE_ASID 0xE75278 and MSG_$FREE_ASID 0xE74E5C store their index
  register to a frame word the same way before passing it by address.
  Whether that word is the index's own slot or another local (`vol :=
  i`) cannot be seen; `dpc` takes the second reading (the C of
  DISK_$UNASSIGN_ALL has a separate `vol_idx`).
- **Array indexing by the index** (`fors`, index use of the register-index
  loops): byte arrays are indexed with the index register itself, `(d,
  An,Di.w)` (84 loops whose direct uses are all byte operations or `lea`:
  `move.b` 73, `lea` 34, `cmpi.b` 9, ...; KBD's module-local 0xE1CA70
  `cmp.b (0,A5,D1.w),D2`; KBD_$INQ_KBD_TYPE 0xE72590 `move.b
  (40,A0,D0.w),(0,A2,D0.w)`; DTTY_$WRITE_STRING 0xE1D6B4 `pea
  (-1,A2,D2.w)`).  Wider elements are reached through **a pointer
  stepped by the element size** (104 loops with a stepped pointer and no
  direct use; only 9 scale a copy of the index without a pointer).  The
  pointer is the address of element `i` *plus `lo × size`*: it starts at
  `base + from × size` and the accesses fold `-lo × size` (and the field
  offset) into the displacement, so a 1-based array is reached with a
  pointer "one element in": LOG_$READ's worker 0xE177E0 `lea (2,A1),A0;
  lea (2,A2),A1`, then `move.w (-2,A0),(-2,A1)`; PKT_$RECENTLY_MISSING
  0xE128D6 `lea (8,A5),A0`, `cmp.l (-8,A0),D1`; PKT_$DUMP_DATA 0xE12800
  `lea (4,A0),A2`, `(-4,A2)`; HINT_$INIT_CACHE 0xE313E2 `lea (12,A0),A0`
  then `(-4,A0)`, `(-8,A0)`, `(-12,A0)`; PROC2's 0xE42240, MSG_$FREE_ASID
  0xE74E44 `addq.l #8`.  With `lo = from = 0` the pointer is the array's
  address (DIR_$OLD_INIT 0xE31502 `movea.l #OLD_DIR,A0; clr.l
  (0x2b8,A0)`; NET_IO_$FREE_ASID 0xE74E8E).  An access at displacement 0
  that is the pointer's last use steps with `(An)+` (0xE00ED8 `clr.l
  (A0)+`; AST_ 0xE0287A `bclr #7,(A2)+; addq.l #3,A2`).  Steps: `addq.l
  #s` up to 8, `lea (s,An),An` above (0xE3150C `addq.l #8`; HINT 0xE313F4
  `lea (12,A0),A0`).  **Several pointers** (ad-hoc script over the `dbf`
  loops with two or more pointer steps and bodies of at most eight
  instructions, 30 loops): in the copies looked at they are formed in the
  order of their first use, the source of `a[i] := b[i]` first
  (0xE177E0, 0xE0A790, 0xE0E384, 0xE4F010, 0xE5C98C; not 0xE7487C), and
  29 of the 30 step them in the reverse order of their forming (the last
  definition before the loop, a pre-advance `addq` aside): 0xE177EE
  `addq.l #2,A1`, the destination, before `addq.l #2,A0`; 0xE2F3F8 forms
  A2, A1, A0 and steps A0, A1, A2.  The exception, 0xE17730, steps in the
  reverse order of first use instead.  A `with a[i] do` body copies the
  pointer at the head (`movea.l A0,A1`: DISK_$REGISTER 0xE3D9EC,
  RING_$KICK_DRIVER 0xE768BC, 0xE42244).  `dpc` does not implement the
  stepped pointer yet and refuses a `for` index used to index elements
  wider than a byte.
- **Run-time bounds.**  The count `hi - lo` is computed into a register
  and guarded before the loop: `subq.w #1,T; bmi EXIT` for `lo = 1`
  (LOG_$READ's worker 0xE177DC; DTTY_$WRITE_STRING 0xE1D6AC), `sub.w
  lo,T; bmi` for a run-time `lo` (0xE010B6, 0xE02C04), `tst.w T; bmi`
  for `lo = 0` (0xE17EBE and 0xE19024, the count then copied *to itself*:
  0xE17EC2 `move.w D2,D2`, 0xE19028 `move.w D0,D0`).  An unsigned
  (`0..65535`) count is guarded with `tst.w; beq` *before* `subq.w #1`
  (module-local 0xE54B6E..0xE54B72; 0xE0D01A..0xE0D028 and
  0xE0D942..0xE0D952, where the `subq.w` comes after the copy into the
  counter), since `bmi` would be wrong above 32768.  Whether the count
  stays in `T` or is copied into a counter of its own (`fors`, guards):
  with a call in the body it is copied 45 times and kept in place once
  (DTTY_$WRITE_STRING 0xE1D6B0 `move.w D0,D3`, the counter callee-saved,
  the scratch `T` not); without one, copied 45 times and kept in place 64
  times, whether `T` was loaded from memory or copied from a register,
  with no rule found (LOG_$READ's worker and PKT_$RECENTLY_MISSING keep
  it; 0xE0B1D8 `move.w D0,D1; subq.w #1,D1; bmi; move.w D1,D0` copies it
  back).
- **Hidden frame cost of a `for` loop** (bead domain-pascal-nda; evidence
  only, nothing implemented): five routines with one `for` loop have no
  frame bytes beyond their locals: DTTY_$WRITE_STRING `link #-4` (the
  index), PKT_$DUMP_DATA `link #-4` (the index), HINT_$INIT_CACHE and
  DIR_$OLD_INIT `link #-4`, DISK_$UNASSIGN_ALL `link #-8` (a 4-byte status
  at -4, a word at -6, the index).  So a loop costs no hidden slot of 4
  bytes or more; a 2-byte one would still round away in all five except
  DISK_$UNASSIGN_ALL read with a separate `vol` local.  Routines of the
  same loop shape differ: DISK_$UNASSIGN_ALL `#-8`, MSG_$FREE_ASID
  0xE74E2C `#-12`, SMD_$FREE_ASID 0xE75250 `#-16`, each with a status at
  -4 and the index stored to -6.  And loops are not needed for unused
  frame space: the DISK_ module-local 0xE3CCCE (`while`, one 4-byte
  local held in A1) has `link #-8`.  This favours hypothesis (a) of
  section 2 (declared locals the compiler kept out of memory) over (b).
- **`exit` and `next`** (ref pp. 4-77, 4-123): a jump to the loop's exit
  (after `dbf` or after the bottom test) or to its continuation (the
  `for` step, the `while`/`repeat` test).  The image cannot tell them
  from `goto`/`return`/`if` structure, but the shapes are the usual ones:
  KBD's module-local 0xE1CA74 `bne.s` to the step `addq.w #1,D1`
  (0xE1CA7A) over a `then` part that leaves with `bra.s` to the
  epilogue; PKT_$DUMP_DATA 0xE12814 `ble.s` out of the loop.  `if c then
  exit` is one conditional branch on `c` (no conditional branch over a
  `bra` in compiled code, 7.1).

**Implemented** (`crates/dpc-ir/src/lower.rs`, `for_loop`,
`check_for_indices`; `crates/dpc-m68k/src/isel.rs`, `while_`, `for_`,
`loop_pointer_locals`, `CmpaZero`):

- `while` as above, with the 0x2048 filler in aligning routines;
  `cmpa.w #0,An` for `=`/`<>` `nil` on a pointer held in an address
  register.
- `for` with a word index variable of the routine itself, never read
  outside loops over it nor assigned or addressed in them (ref p. 4-89;
  `dpc` keeps it in a register for the loop only):
  - constant bounds, N in 1..=65536: count first, index (only when the
    body reads it) `clr.w`/`moveq` (initial value in -128..127 and not
    the count's), step `addq.w`/`subq.w #1`, `dbf`;
  - a run-time upper bound of an ascending loop from 1 with a signed
    index, in a body that calls: `move.w hi,T; subq.w #1,T; bmi EXIT;
    move.w T,Dc; moveq #1,Di`;
  - a `for` loop of the `Fill` shape stays as in section 7.
- `exit`, `next`, and `if c then exit|next` as one conditional branch.
- A slot used inside a loop body is loaded at entry like one used more
  than once (12.2: 0xE3CCD6; NAME_$VALIDATE's module-local 0xE4A004 loads
  the slots its loops use at entry, 0xE4A018/0xE4A01C, and the one used
  once outside them where it is needed, 0xE4A03C).
- A pointer local assigned in a loop is held in an address register
  (12.5, `loop_pointer_locals`).

**Not supported yet** (positioned errors): run-time bounds other than
the case above (no rule for the copy without calls; `tst.w; beq` for
unsigned counts; run-time `lo`; `downto`); a `for` index indexing
elements wider than a byte (stepped pointers); an index initial value
outside -128..127 or equal to the count; byte or longword indices; an
index of another scope (0xE721F8) or read after its loop (MST_$ALLOC_ASID
0xE42D7C pushes D2 after leaving the loop with a `goto`); a loop that
never runs; an empty loop body; an `exit`/`next` that would branch to the
next instruction.

Goldens `tests/golden/loops/while.pas`, `loops/for.pas`; tests
`crates/dpc-m68k/src/isel.rs` (`for_with_index_and_call`, `for_forms`,
`unsettled_for_loops`, `while_shape`, `loop_pointer_local_rule`),
`crates/dpc-ir/src/tests.rs` (`for_loops`, `while_exit_next`).
Conformance (`make conform`): `tests/conformance/disk_unassign_all.pas`
(DISK_$UNASSIGN_ALL 0xE6BE1C; its `bsr.w` to DISK_$UNASSIGN, outside the
unit, through the manifest's `module-routines=` token and `dpc
--module-routine`), `tests/conformance/disk_set_chain_status.pas` (the
DISK_ module-local 0xE3CCCE).

---

## 8. Arithmetic

Survey method for this section: every compiled routine of 0xE00800..
0xE88834 (split as `tools/loopalign.py` does, cut at the first `rts`),
disassembled once and searched with ad-hoc scripts (not kept); counts are
of instruction sequences, re-check sites with `tools/imgdis.sh`.  Beads
domain-pascal-5aw.13 and -d7i.  **Implemented** in `crates/dpc-m68k/src/
isel.rs` (`eval`, `binary`, `mul_const`, `shift`, `convert`,
`update_in_place`), lowering in `crates/dpc-ir/src/lower.rs` (`binary`,
`builtin_value`, `narrow`); goldens `tests/golden/expr/*`, tests
`crates/dpc-m68k/tests/expr.rs`; conformance units
`rip_packet_length.pas`, `bat_get_bat_step.pas`, `fim_get_fim_addr.pas`,
`audit_is_process_audited.pas`.

- **Operands (Verified):** a variable is used as a memory operand or
  loaded into a new data register (`move.<s> ea,Dn`); a value held in a
  register (a parameter loaded at entry, 12.2; a function result in D0,
  section 3) is never modified: arithmetic works on a copy
  (BAT_$GET_BAT_STEP 0xE3BB68 `move.w D0,D1`; 0xE01BA4; FIM_$FREE_ASID
  0xE0AA8A), which takes another register (12.4 R2).  A computed value is
  modified in place.
- **`x + c`, `x - c` (Verified counts):**
  - 1..8: `addq`/`subq` on the value in a register (after a load: `addq`
    ×203, `subq` ×223; 0xE00C10, 0xE010D0, RIP_$PACKET_LENGTH 0xE68874
    `addq.w #2,D0`);
  - `x + c` for c in 9..127 with `x` in memory: the constant first,
    `moveq #c,Dn; add.<s> x,Dn` (×23: 0xE008A8 `moveq #30,D0; add.l
    (-64,A6),D0`; 0xE30BB0 `moveq #64,D2; add.w abs.l,D2`), against 4 loads
    followed by `addi` of such a constant (0xE30A80; the three at 0xE00920
    follow a zero-extension, a computed value);
  - otherwise the value in a register and `addi`/`subi` (×26 `addi` of a
    word or long constant after a load, 0xE08840; `subi` of 9..127 after a
    load ×13, 0xE0C462; 0xE0B6E2 `subi.w #0x400,D2`);
  - `c - x`: the constant first, `sub.<s> x,Dn` (about 20: 0xE08EE4 `moveq
    #31,D2; sub.w (-26,A6),D2`; 0xE10114 `move.l #0x400,D2; sub.l
    (-372,A6),D2`; LOG_$READ's worker 0xE177C4);
  - not catalogued (rejected): a negative constant; `x + c` for 9..127
    with `x` held in a register (11 `moveq; add.l Dx,Dn`, 11 copies and
    `addi.l`).
- **`x + y`, `x - y` (Uncertain operand order):** one operand in a
  register, the other added to it (0xE177B4 `add.l D1,D2`; `add.w mem,Dn`
  after a load ×38).  Which source operand the original put in the
  register cannot be seen; `dpc` takes the left one, or the computed one
  for `+`.  12.4 R7: the sum goes into the operand that is not reused
  (0xE177B4), which `dpc` does not model.
- **`x := x op c` (Verified counts):** `addq`/`subq` to memory ×1076
  (review recount: 731 `addq`/`subq` with a memory destination in the
  whole range 0xE00800..0xE88834, hand-written assembly included, so the
  1076 is an overcount; the order of magnitude stands)
  (0xE00B3E `addq.w #1,abs.l`; ACL_$ENTER_SUPER 0xE46FA8 `addq.w
  #1,(0xb76,A0)`), `addi`/`subi` to memory ×20 (0xE010C4 `addi.l
  #0xc0,(0x3f4,A5)`), `andi`/`ori` to memory ×560 (0xE0086E `andi.l
  #-0x7f8001,(0x26,A2)`), against 6 plain operands loaded, changed and
  stored back (0xE025B0).  For an array element both forms occur: 14
  `addq`/`subq` to an indexed operand (0xE13154 `addq.l
  #1,(-4,A0,D0.w)`) against 10 load, change, store sequences
  (AUDIT_$SUSPEND 0xE70DC8..0xE70DCE): rejected.
- **`&` (ref p. 4-33; Verified counts):** with a constant and a variable
  (in memory or held in a register): the constant first, then `and.<s>
  x,Dn` (about 300 with the constant loaded by the instruction just
  before: 0xE177A0 `moveq #-2,D2; and.w D1,D2`, D1 a parameter held since
  the entry; 0xE60F4A `move.l #0xfffff,D0; and.l (-28,A6),D0`; 0xE02D44
  `moveq #15,D1; and.b (157,A3),D1`; 0xE00A14), against about 60 loads
  followed directly by `andi`, some of them conversions (0xE00868 `andi.l
  #0xff`, a zero-extension) and some not (0xE05810 `move.l (-188,A6),D1;
  andi.l #0x7fffffff,D1` before a compare; 0xE03494): Uncertain as a
  rule, `dpc` uses the majority form.  On a computed value: `andi` in
  place (0xE11024 after `lsr.w`).
- **`!`, `xor` (Uncertain):** on a computed value, `ori`/`eori` in place
  (0xE03498 `andi.w #-0x4001,D1; ori.w #0x2000,D1`); with a variable both
  orders occur (4 constant-first `or`s, 5 loads and `ori`): rejected.
  `eor` is rare (7 `eor`, 1 `eori` in compiled code).
- **`-x`, `~x`:** `neg.<s>`, `not.<s>` on the value in a register (`neg.w`
  ×97, many from the multiplications below; `not.w` ×13, `not.l` ×46,
  `not.b` ×43).
- **Shifts (ref pp. 4-109, 4-171, 4-29; Verified):** `lsl`/`lsr`/`asr` on
  the value in a register; a constant count above 8 is split into shifts
  of 8 and the rest (×1024 0xE02AA6 `lsl.l #8,D0; lsl.l #2,D0`, 96 sites;
  ×32768 0xE00876 `lsl.l #8; lsl.l #7`); a count in a register: `lsl.w
  D1,D3` (0xE02D52).  Ref p. 4-171: `rshft` of an operand smaller than 32
  bits expands it to 32 bits first (`rshft(-9, 3)` is -2 in 16 bits); its
  code for a signed 16-bit operand is not catalogued and `dpc` rejects
  it.  `arshft` gives the same low word either way (`asr.w`).
- **Multiplication by a constant (ref p. 6-28; Verified):** with c = 2^s
  × m, m odd:

  | m | Code (after the value is in Dn) | Constants seen (sites) |
  |---|---|---|
  | 1 | `add.w Dn,Dn` for s = 1, else `lsl #s` (split above 8) | 2, 4, 8, 16, 128, 256, 1024 (×96), 4096, 32768 |
  | 2^k + 1 | the 2^s part; `move Dn,Dm`; `Dm` shifted by k (`add Dm,Dm` for k = 1); `add Dm,Dn` | 3, 5, 6 (RIP_$PACKET_LENGTH 0xE6886C), 9, 10, 12 (×109, 0xE073D4), 17, 18, 20, 24, 36, 48, 72 (DISK_$REVALIDATE 0xE6C072), 96, 144, 192, 257 |
  | 2^k - 1, k >= 3 | the 2^s part; `move Dn,Dm`; `neg Dn`; `Dm` shifted by k; `add Dm,Dn` | 28 (PEB_$GET_FP 0xE5AEF6), 56, 60, 120, 248 |
  | other | `mulu.w #c,Dn` (word results) | 92, 150, 153, 204, 228, 244, 250, 268, 300, 336, 363, 564 (BAT_$GET_BAT_STEP 0xE3BB6A), 580, 1083, 1244, 1624, 5240, 12345 |

  The one exception is ×26 at 0xE71BBA (odd part 13, done bit by bit:
  `add.w D1,D1; move.w D1,D0; lsl.w #2,D0; add.w D0,D1; add.w D0,D0;
  add.w D0,D1`), while other odd parts of three bits (21, 67, 145) use
  `mulu.w`: `dpc` rejects odd parts of three bits other than 2^k ± 1.  A
  68000 cycle count does not separate the two groups (268 and 336 would
  be cheaper with shifts).  `muls.w #c` occurs where the product is a
  `.l` index (section 5.2); for a word result `dpc` uses `mulu.w`, for a
  longword product of a word see "Products of words used as longwords"
  below.  The `move` that
  starts the second chain takes a new register (D1 at 0xE68870, 12.4 R8);
  when the multiplicand is held, it is copied first (0xE073D4 `move.w
  D1,D3; lsl.w #2,D3; ...`).  **Uncertain:** ×17 at 0xE87974 keeps the
  product in the copy (`move.l D0,D6; lsl.l #4,D6; add.l D0,D6`), D0
  being used later; `dpc` copies a held multiplicand and then uses the
  forms above.
- **Multiplication, division and mod: the survey (beads
  domain-pascal-5aw.5, -1x1).**  `tools/arithsurvey.py` (`summary`,
  `calls`, `inline`; routines split and followed as in
  `tools/casesurvey.py`) lists every `jsr` to an M$ARITH entry and every
  inline `mul<s>.w`/`div<s>.w` of compiled code.  **Implemented** in
  `crates/dpc-m68k/src/isel.rs` (`mul`, `mul_words`, `div`, `div_pow2`,
  `marith`, `marith_op`, `word_part`; the narrowing hook in `convert`;
  `arith_calls` for the pop rule) and `crates/dpc-ir/src/lower.rs`
  (`binary`: the sign of `div`/`mod`, the widened dividend; `narrow`);
  golden `tests/golden/expr/marith`, tests `crates/dpc-m68k/tests/expr.rs`
  (`marith_calls`, `word_products_used_as_longwords`, `division_forms`,
  `values_live_in_d1_across_marith`), `crates/dpc-ir/src/tests.rs`.
  Semantics were checked in a scratch 68000 evaluator (not kept): the
  M$ARITH entries of `../source/domain_os/math/sau2/arith.s` and the
  generated sequences against ref "Div" p. 4-66 (truncation toward zero)
  and "Mod" pp. 4-115..4-116 (the dividend's sign), over random and edge
  operands; the entries preserve D1-D3/A1 (12.4 R3).
- **The M$ARITH entries (Verified: frames in arith.s, call sites).**
  Name: MI = `*`, DI = `div`, OI = `mod`; S/U = signed/unsigned; then the
  sizes of the result, argument 1 (`a`, at `(4,SP)` in the callee) and
  argument 2 (`b`, after it): `M$MIS$LLW` is a longword `a` times a word
  `b`, longword result; `M$OIU$WLW` is an unsigned longword `a` mod a word
  `b`, word result; `M$OIS$WWL` a word `a` mod a longword `b`.  The
  results: D0, sized by the name (an `M$OI*$WLW` remainder in D0.w).
  `M$OIS$LLL` and `M$OIU$LLL` are one entry (0xE0AD7C).  Calls in compiled
  code (92):

  | Entry | Address | Calls | Example |
  |---|---|---|---|
  | M$MIS$LLL | E0ABD4 | 9 | 0xE14C02 (`x * 60296`), 0xE6D606 |
  | M$MIU$LLW | E0AC02 | 10 | 0xE1C2B8 (`#25000`), 0xE30BC8 |
  | M$MIU$LLL | E0AC1C | 2 | 0xE5D008 |
  | M$MIS$LLW | E0AC44 | 12 | 0xE16970, 0xE3B1E2 |
  | M$DIU$LLW | E0AC68 | 14 | 0xE1405E (`#12`), CAL_$DECODE_TIME 0xE68794 (`#60`) |
  | M$DIS$LLW | E0AC84 | 1 | 0xE16962 (by a word variable) |
  | M$OIU$WLW | E0ACC0 | 11 | AREA_$FREE_FROM 0xE07FEE (`#11`), 0xE3CB94 |
  | M$DIS$LLL | E0ACD8 | 10 | TIME_$GET_ADJUST 0xE16AC6 (`#250000`), 0xE3AE7C |
  | M$DIU$LLL | E0AD0C | 2 | AST_$GET_SEG_MAP 0xE06E16 |
  | M$OIS$WLW | E0AD54 | 16 | 0xE30D3E (`mod 32`), CAL_$WEEKDAY 0xE686C0 |
  | M$OIS$WWL | E0AD68 | 0 | |
  | M$OIS$LLL = M$OIU$LLL | E0AD7C | 5 | TIME_$GET_ADJUST 0xE16AD8 |

- **The call sequence (Verified, counts of the 92 calls):**
  - the pad `subq.l #2,SP` when `b` is a word: 64 of the 64 calls with a
    word argument, none of the 28 others (section 1.4: 6 bytes);
  - `b` pushed first, then `a` (section 1.1): a word `b` by `move.w
    #c,-(SP)` (37) or `move.w ea,-(SP)` (27); a longword `b` by `move.l
    #c,-(SP)` (8, all constants beyond a word: 60296, 64453, 65173,
    250000) or `move.l ea,-(SP)` (20); `a` by `move.l` (91) or `clr.l
    -(SP)` (0xE10A74); the same variable as both, once and then `move.l
    (SP),-(SP)` (OS_$SHUTDOWN 0xE6D600..0xE6D606, `x * x`; one site,
    implemented);
  - `jsr (M$...).l`, then `addq.w #8,SP` (89), not popped where the
    procedure-call rule of section 1.1 skips the pop (0xE16A92, 0xE16AD8,
    0xE19BA6: only the result move and the epilogue follow);
  - the operands are computed before the pushes, like any operator's
    (0xE14BEE..0xE14C00: `x div 256` into D0, then `move.l #60296,-(SP);
    move.l D0,-(SP)`).  **Uncertain:** with a pad, a computed operand has
    the pad scheduled into its computation (22 calls, e.g. CAL_$WEEKDAY
    0xE686AE..0xE686BE `move.w (A1),D0; subq.l #2,SP; ext.l D0; addi.l
    #9,D0`) or after it (0xE0920C, 0xE094A2, 0xE09F86): `dpc` refuses a
    computed operand with a pad, except an M$ARITH result as `a`, whose
    call and pop come first (0xE1695C..0xE16970: `M$DIS$LLW`, `addq.w`,
    then the pad and `move.l D0,-(SP)` for `M$MIS$LLW`).  Two computed
    operands are refused: the order differs (0xE10A86..0xE10A98 `b` first,
    0xE69560..0xE69572 `a` first);
  - the result is used in D0 and modified there (0xE39C3E `add.l
    (14,A6),D0`; 0xE14C0A `tst.l D0; bpl; addi.l #255,D0`; CAL_$WEEKDAY
    0xE686C8 `muls.w #153,D0`); an unsigned `M$OIU$WLW` remainder used as a
    longword is zero-extended into a new register (0xE07FF6 `clr.l D1;
    move.w D0,D1`; 0xE07A3A, 0xE07DD2, 0xE5AA4C); the 16 `M$OIS$WLW`
    results are all used as words (`dpc` refuses one used as a longword);
  - **registers (Verified; 12.4 R3):** D1 or A1 is live across 31 of the
    83 calls `tools/regalloc.py` models (42 register/call pairs; the
    entries write only D0/A0, and preserve D1-D3 with `movem`).  A value
    whose last use is in the pushes stays in a scratch register at
    TIME_$GET_ADJUST 0xE16AD6 (`move.l D1,-(SP)`, D1 dead after; a local
    variable).  **Uncertain:** whether a parameter loaded at entry would be
    held (12.4 R4(c)); `dpc` does not hold for M$ARITH calls.
- **Which form (rule, counts; implemented):**
  - **`*`:** a constant factor whose odd part is 1 or 2^k ± 1: shifts
    and adds at the product's width (above).  **A word times a word, or a
    word times a constant that fits one, used as a longword: one
    `muls.w`/`mulu.w`, its 32-bit product** (bead domain-pascal-1x1;
    `muls` for signed words, `mulu` for `0..65535` ones): 15 sites, `muls`
    0xE10A80 (`sub.l` after), PROC1_$LOADAV_CALLBACK 0xE14C18 and
    0xE14C4C (`move.w D1,D2; muls.w #5240,D2; add.l D2,D0`), TPAD_$DATA
    0xE69232, 0xE692B4, 0xE69396, 0xE693A6 (the product divided, `divs.w
    #2200`); `mulu` 0xE13EE0, 0xE3995A, 0xE3C1BE, 0xE3C1D0, 0xE554AC,
    0xE5552E (`mulu.w #150,D0; move.l D0,D3`), SIO_$K_TIMED_BREAK
    0xE67F2A (`mulu.w #250`, a `uint16_t` in the C), 0xE6C752.  No M$ARITH
    call multiplies two words of one sign (the one call with two word
    operands, DISK_INIT 0xE19B8A..0xE19B94, zero-extends a `0..65535` word
    for `M$MIS$LLW` by a signed word: refused by `dpc` as a computed
    operand with a pad).  Otherwise M$ARITH: `M$MI<s>$LLW` when one operand
    is a word (or a constant that fits a word of the operation's sign),
    passed as `b` whichever side it is on; `M$MI<s>$LLL` otherwise, a
    constant as `b` (all 5 constant sites).  A word times a word for a word:
    `mul<s>.w` by the sign of the type (0xE30A9A `muls.w D3,D0; move.w
    D0,(A0)+`; VTOC_$GET_UID 0xE3925A `mulu.w D4,D6`; Uncertain: either
    gives the low word).  **Uncertain:** which of two variables goes to the
    register (`dpc`: the left one or the computed one, as for `+`;
    0xE10A7E copies a held D2 and multiplies by memory); which operand of
    `M$MI*$LLL` is `a` (`dpc`: the left one).
  - **Word `div`/`mod` (both operands words of one sign):** signed,
    `ext.l; divs.w` and `swap` (0xE31154..0xE3115C; CAL_$DECODE_TIME
    0xE68800..0xE68808 `ext.l D0; divs.w #4,D0; swap D0` for `mod 4`;
    TPAD_$INIT 0xE335F4..0xE335FA `ext.l D0; divs.w (-38,A3),D0` by a
    variable); unsigned, `clr.l Dn; move.w x,Dn` or `andi.l #0xffff` on a
    computed value, then `divu.w` (0xE382B4..0xE382CA, 0xE4CE1A..0xE4CE22,
    0xE54B8C..0xE54B92 by a register, 0xE39942..0xE39948 by memory,
    0xE11816..0xE11822).  `0..65535` words have an integer32 host, so
    their quotient or remainder is a longword operation on two words in
    the IR: divided as words; used as a longword, the result is
    zero-extended into a new register (0xE382BE/0xE382CC `clr.l D2; ...;
    move.w D0,D2` before `lsl.l #6,D2`).
  - **Signed `div` by 2^k, either width (Verified; 63 sequences):**
    `bpl.s L; addq/addi #2^k-1,Dn; L: asr #k,Dn` on the flags of the
    instruction that loaded or computed Dn, or after `tst` (0xE14C0A
    after a call): CAL_$WEEKDAY 0xE68698..0xE6869E `move.w D1,D3; bpl.s;
    addq.w #3,D3; asr.w #2,D3`; PROC1_$LOADAV_CALLBACK 0xE14BEE `addi.l
    #255; asr.l #8`; `div 1024` as `addi.l #1023; asr.l #8; asr.l #2`
    (0xE06D5C, 0xE0896E).  No `div` by a power of two uses `divs.w` (the
    one `divs.w #4`, CAL_$DECODE_TIME 0xE68804, is a `mod`).  Also
    when the quotient is used as a word (0xE30D26..0xE30D32 `asr.l #5`,
    then `move.w`).  Unsigned longword by 2^k: `lsr.l` (0xE14086, 0xE06E2E).
    `mod` by 2^k is a division like any other (0xE68804 above; 0xE30D3E
    `M$OIS$WLW` for `mod 32`).  The join is not aligned (section 7:
    forward targets; MST_$INIT 0xE30D2E has a filler, one of the
    exceptions listed there).  Refused: unsigned word `div`/`mod` and
    unsigned `mod` by 2^k (no site: shift, mask or division?).
  - **A longword `div` whose quotient is used as a word, by a word (or a
    constant that fits one): `div<s>.w` in place on the longword, no
    call** (about 30 sites: of the 70 inline `div<s>.w`, 23 `divs.w` and
    7 `divu.w` divide a longword the instruction before computed or
    loaded, the others an extended word; ad-hoc classification of the
    `inline` listing): 0xE10CAE..0xE114D8 ×8 `move.l (-786,A2),D0; sub.l
    (-794,A2),D0; divs.w #250,D0; move.w D0,-(SP)`; REM_FILE_$SERVER
    0xE644FC, 0xE6450A; PMAP_$PURIFIER_L 0xE13AFC `move.l abs,D2; divu.w
    #50,D2; move.w D2,(1932,A5)`, 0xE13B0A; 0xE3CB84 `divu.w (160,A1),D5`
    (by a word variable); RIP_$SERVER 0xE68AE8; TPAD_$DATA 0xE6939A,
    0xE693AA; 0xE6929C, 0xE69328 (an `M$MIS$LLW` result divided);
    0xE2F052..0xE2F058 `ext.l D1; lsl.l #8,D1; lsl.l #2,D1; divs.w
    #192,D1` (a word times 1024, divided).  Every one whose use was
    checked uses the quotient as a word (`move.w`, a word push, `tst.w`,
    `cmpi.w`), also as an operand of word arithmetic: module-local
    0xE077DA at 0xE078CC..0xE078D6 `move.l A3,D0; sub.l #0xd94c00,D0;
    divu.w #48,D0; addq.w #1,D0`; PROC2_$GET_CR_REC 0xE40186..0xE40190
    (`divs.w #24`, `addq.w #1`); TPAD_$DATA 0xE6954A (an `M$MIS$LLW`
    product, `divs.w (-34,A2)`, `sub.w`).  `dpc-ir` narrows the operands
    of a narrowed sum the same way (`narrow`).  The
    quotient is right only when it fits in 16 bits (otherwise DIVS/DIVU
    set V and leave the dividend): the original accepts that, `dpc` does
    the same.  The same quotient used as a longword calls M$ARITH
    (0xE14076 `M$DIU$LLW` of the same 0xE23C98 by 50, then `add.l`).
  - **Other longword `div`:** `M$DI<s>$LLW` when the divisor is a word or a
    constant that fits one, `M$DI<s>$LLL` otherwise (also when the
    quotient is used as a word: 0xE39C26, 0xE3AE7C `M$DIS$LLL`, then
    `move.w D0,...`).
  - **Longword `mod`:** `M$OI<s>$WLW` by a word or a word constant (27
    calls), whatever the use of the result (0xE3CB94 calls `M$OIU$WLW` for
    the remainder right after `divu.w` gave the quotient of the same
    operands at 0xE3CB84: a word remainder needs no overflow check, but
    `divu.w` would lose it when the quotient overflows); `M$OI<s>$LLL`
    otherwise.  `M$OIS$WWL` (a word mod a longword) is never called:
    refused.
  - **The sign:** ref 4.3.3, pp. 4-6..4-7: `div` and `mod` (and the
    relational operators, `max`) are unsigned when both operands are
    unsigned run-time values, or one is and the other a constant in
    0..2147483647.  The image's unsigned entries (`M$DIU`, `M$OIU`,
    `M$MIU`) take zero-extended `0..65535` words, their sums, and
    longwords whose type the image does not show (0xE1C2B4); `dpc` gives
    `*` the same rule (Uncertain, above).  **Open:** ref 3.4 (p. 3-9) calls the
    subranges 0..2^30 to 0..2^31-1 "unsigned ranges" and 4.3.3's example
    divides a `0..2147483647` value unsigned, but `dpc-ir`'s `sign_of`
    treats such 4-byte subranges as signed (only 2-byte ones with an
    integer32 host are unsigned): the unsigned longword forms are not
    reachable from such types yet.
  - **A word sum, difference or product as the dividend is divided in 32
    bits** (ref 4.3.1, p. 4-5): CAL_$WEEKDAY 0xE686AE..0xE686C0 `move.w
    (A1),D0; ext.l D0; addi.l #9,D0` then `M$OIS$WLW` for `(month + 9) mod
    12`; 0xE686D0..0xE686D4 `ext.l D0; addq.l #2,D0; divs.w #5,D0`;
    0xE686DA..0xE686EE (a sum of two words + 1, `mod 7`); RIP_$SERVER
    0xE68AE2..0xE68AE8 `move.w D2,D5; ext.l D5; subq.l #2,D5; divs.w
    #6,D5`; TPAD_$DATA 0xE69392..0xE6939A (a `muls.w` product).  `dpc-ir`
    lowers such a `div`/`mod` as a longword operation narrowed back to
    the word.  **Exception (Uncertain):** ACL_$INIT 0xE31150..0xE3116E
    computes `(i + 1) mod 31` and `(31 + i - 1) mod 31` in 16 bits, `i`
    the index of a `for` loop over 0..30 (`acl/init.c`), whose range the
    compiler may have used; `dpc` has no such loops.  Such a narrowed
    word quotient used as a longword again is refused.
  - **Refused (not catalogued):** a computed divisor of `div<s>.w`, or
    one whose address takes an instruction; a division by a negative
    constant inline; byte operands; an M$ARITH call among another call's
    arguments; a second call while an M$ARITH or function result in D0 is
    still needed (e.g. `l * l + l * l`).
- **Conformance:** TIME_$GET_ADJUST 0xE16AA8 (0x40 bytes: `disable`/
  `enable`, section 10; a local held in D1 across both calls, `link #-4`,
  section 12.5) is byte-identical (`tests/conformance/time_get_adjust.pas`,
  beads domain-pascal-5aw.7, -0wc): `M$DIS$LLL`/`M$OIS$LLL` by a
  longword constant, the second call not popped, `lsl.l #2,D0`.  The
  other small M$ARITH callers need what `dpc` lacks: TERM_$ENQUEUE_TPAD
  0xE72472 (a `while` loop), module-local 0xE5AA28 (record copies and a
  computed operand with a pad), module-local 0xE5D134 and 0xE54B58
  (`divu.w` after register-held computations and a `for` loop).
  `tests/golden/expr/marith` reproduces TIME_$GET_ADJUST's call
  sequence with the local replaced by a variable of another unit.
- **Widening and narrowing (Verified at the sites):** a signed word
  widened with `ext.l` after the load (0xE02C28..0xE02C2C `move.w D2,D0;
  ext.l D0; move.l D0,(-32,A6)`, an assignment; 610 `ext.l` in compiled
  code); an unsigned `0..65535` word with `clr.l Dn; move.w x,Dn`
  (0xE177A6..0xE177AC), or `andi.l #0xffff` on a value already in a
  register (0xE11818); a `char` widened to a word with `clr.w Dn; move.b
  x,Dn` (0xE0E316..0xE0E318).  A byte widened to a longword occurs as
  `move.b; andi.l #0xff` (0xE00864..0xE00868): rejected.
  **A word sum, difference, product or negation used as a longword is
  computed in 32 bits, its operands widened first** (ref 4.3.1, p. 4-5:
  "The compiler computes operands smaller than 32 bits with 32 bits of
  precision when necessary to achieve correct arithmetic"): 0xE0863C..
  0xE08642 `move.w D3,D2; ext.l D3; subq.l #2,D3; move.l D3,(-176,A6)`;
  0xE69568..0xE6956E `ext.l D5; addq.l #1,D5` for a longword argument;
  0xE07442..0xE0744A `ext.l D6; lsl.l #2,D6; move.l D6,D7; add.l D7,D7`
  (×12).  Review survey of 0xE00800..0xE88834: the instruction before an
  `ext.l Dn` that writes Dn is a `move.w` ×355, `andi.w` ×2, `asr.w`,
  `subq.w` (before a `divs.w`) and `divs.w` once each, never a word
  `add`/`sub`/`mul`; `ext.l` followed by a 32-bit `lsl.l`, `add.l`,
  `addq.l` or `addi.l` and a `move.l` store ×79.  **Implemented**
  (`lower.rs`, `convert`: the widening is pushed into the operands of
  `+`, `-`, `*` and unary `-`; the bit operators, `div`, `mod`, `rshft`
  and `arshft` give the same value either way and are widened after).  A
  word `lshft` used as a longword is rejected (p. 4-109 does not say
  whether it is expanded first, as pp. 4-29 and 4-171 say of `arshft` and
  `rshft`).  A word product whose constant is not done with shifts, or of
  two words, is one `muls.w`/`mulu.w` instead (32-bit product, see the
  rule for `*` below).  Compares are
  unchanged: a word sum is compared at the word's width (review survey:
  `move.w; add.w/sub.w; cmp.w/cmpi.w/tst.w` ×41 against `move.w; ext.l;
  add.l/sub.l; cmp.l/cmpi.l` ×7, e.g. 0xE0F9D0..0xE0F9D8; the rule
  between them is not known).  An integer32
  expression stored into a word object (or returned as a word result) is
  computed at word width when the operation allows it (`+`, `-`, `*`,
  `&`, `!`, `xor`, `lshft`, `-`, `~`): 0xE177C4 `move.w #0x400,D3; sub.w
  D0,D3; move.w D3,(A0)` stores `16#400 - offset`, `offset` a `0..65535`
  word (an integer32 expression).  Ref 4.3.1, p. 4-5: "the compiler tries
  to avoid the extra code produced by data expansion".
- **Signedness of results (Uncertain, one site):** the sum of two
  unsigned run-time values (or of one and a non-negative constant) is
  compared unsigned: 0xE177B4..0xE177B8 `add.l D1,D2; cmp.l D3,D2; bls`
  for `offset + actual_len^ > 16#400` with two `0..65535` words.  `dpc`
  makes `+`, `*`, `&`, `!` of such operands unsigned (the analogue of ref
  4.3.3 for `div`/`mod`); `-` stays signed.
- **No libgcc-style helpers.** M$ARITH is the whole integer runtime seen so
  far.
- **Floating point:** not used in the kernel as far as is known.  The
  reference documents FPU/compiler variants (ref 6.2, Appendix F).  Out of
  scope until needed.

---

## 9. Status returns and error exits (Verified at the sites given; implemented in part)

Bead domain-pascal-5aw.2.  Counts come from surveys of the routines
`tools/regalloc.py` splits (0xE00800..0xE88834, compiled code), with
ad-hoc scripts that were not kept; re-check sites with `tools/imgdis.sh`.
A "status slot" there is a parameter slot whose address is loaded with
`movea.l (d,A6),An` and through which a status-shaped constant (a
subsystem number in the high word, a small code in the low word) is
stored.

- **The type (reference; layout partly Uncertain).**  Ref 9.1.1-9.1.2
  (pp. 9-1..9-3): `status_$t` is declared in `/sys/ins/base.ins.pas`,
  which we do not have.  It is not itself an integer32: its `all` field
  is (`OPEN(f, pathname1, 'NEW', err_stat.all)`; the error "Improper
  status argument", p. 9-21: "a data type other than integer32 ...
  `OPEN(f1, 'anger1', 'NEW', st); {wrong}` / `OPEN(f2, 'anger2', 'NEW',
  st.all); {right}`").  It also has a `code` field that "contains a
  number that corresponds to a particular error" (p. 9-3), at an offset
  the reference does not give.  `status_$ok` is 0 (9.1, p. 9-1: a successful call returns 0;
  "Open", pp. 4-132..4-133, tests `statrec.all = 0` and `statrec.all =
  status_$ok` alike).
  `../source` types it `int32_t` (`base/base.h`) and sets bit 31 on
  failure paths (`proc2/resume.c`, 0xE41444 `bset.b #7`), which fits a
  fail flag in the top bit of `all` but does not place `code`.  The
  conformance sources therefore declare `status_$t = record all:
  integer32 end` (only `all`, at offset 0, 4 bytes);
  `tests/conformance/name_convert_acl_status.pas`, older, uses a plain
  integer32, which compiles to the same code.  Status codes in sources
  carry `../source`'s names or are written in hex with the database text
  in a comment.  Never invent a `status_$` name.
- **Clear on entry** (`status.all := status_$ok` as the first
  statement): `movea.l (d,A6),An; clr.l (An)`, an ordinary store of 0
  through a slot (section 13): SMD_$BUSY_WAIT 0xE1D8AA; IO_$ALLOC
  0xE1A9C6; MAC_OS_$OPEN 0xE0B25C..0xE0B260, after the entry loads of
  `(8)` and `(0xc)`, in parameter order (12.2); module-local 0xE1A4A4
  0xE1A4AC..0xE1A4B0.  Not every clear through a slot is a status:
  MMAP_$ALLOC_CONTIG 0xE0D8FC clears `pages_alloced` and stores its
  status afterwards (0xE0D906 `move.l #0x6000e,(A1)`,
  `mmap/alloc_contig.c`), and XNS_IDP_$GET_STATS clears its status last
  (0xE18FF8 `clr.l (A1)`).
- **Error stores:** `move.l #imm,(An)` through the status register
  (0xE0B28C `move.l #0x3a0001,(A0)`; XNS_IDP_$GET_PORT_INFO 0xE18FC8;
  the IIC stubs 0xE70A56..0xE70AF2, `#0x2c000a`), never `moveq` +
  `move` (section 13).
- **Where the status pointer lives.**  A status slot used more than once
  is loaded once, at entry, in parameter order (12.2), into the
  register the allocator gives it; live across calls it is callee-saved
  (12.4 R3) and held for the whole body: 251 routines load their status
  slot into A2-A4 (239 of them exactly once), e.g. module-local 0xE01950
  (`movea.l (0x10,A6),A2` at 0xE01960, error store `move.l
  #0x30004,(A2)` at 0xE019A6), TERM_$CONTROL (A3, 0xE66B3A).  115 others
  keep it in scratch registers: 60 with one load (no call while it is
  live, e.g. MMAP_$GET_WS_INDEX 0xE0CA4C), 55 reloading it into A0 after
  calls (MAC_OS_$OPEN 0xE0B25C, then 0xE0B2AE, 0xE0B2EA, 0xE0B320,
  0xE0B3D0, 0xE0B3F6; that routine saves all of D2-D7/A2-A5, so register
  pressure is a candidate cause).  **Uncertain:** when the original
  reloads instead of holding.  `dpc` does the majority form (entry load,
  allocator register).
- **Passing the status on:** held in An, it is pushed with `pea (An)`
  (survey of every slot held in A2-A4, not only status slots: `pea (An)`
  1447 times, `move.l (d,A6),-(SP)` 80 times while the register holds the
  same slot; 0xE019FE `pea (A2)`); not held, the slot itself, `move.l
  (d,A6),-(SP)` (MAC_OS_$OPEN 0xE0B3DE; LOG_$READ 0xE1780C).  Both are
  section 1.5, implemented.
- **Testing it:** `tst.l (An)` (section 7.1, against 0), then `bne` (499
  times after `tst.l (An)`, 173 of them straight to the epilogue) or
  `beq` (125).  TERM_$CONTROL 0xE66B3A `tst.l (A3)` right after the pop
  of the call that set it.
- **Early exits.**  `return` is a `bra` to the register restore that
  starts the epilogue (section 7.1): MAC_OS_$OPEN 0xE0B292 `bra.w
  0xE0B452`; 0xE019AC `bra.w 0xE01AC8`.  **`if c then return`** is one
  conditional branch on `c` to the epilogue, not a branch past a `bra`:
  TERM_$CONTROL 0xE66B3A..0xE66B3C `tst.l (A3); bne.w 0xE66D78` (`if
  status.all <> status_$ok then return`, `term/control.c`; 0xE66D78 is
  the `movem.l` restore), 0xE62E88 `bne.w`, 0xE68C08 `beq.w`.  Compiled
  code has no conditional branch over a `bra` at all, against 888
  conditional branches to an epilogue (Verified by survey).
- **Slots used once** take a register no earlier statement left a value
  in (12.4 R4(b); IIC_$RECEIVE 0xE70AC0..0xE70AD0 A0, A1, A2;
  XNS_IDP_$GET_STATS 0xE18FF4 A1; MMAP_$ALLOC_CONTIG 0xE0D902 A1).
- **A status store, then a constant result:** IO_$ALLOC 0xE1A9C6..
  0xE1A9CC `clr.l (A0); clr.l D0`; IIC_$RECEIVE_CHECK 0xE70AE6..0xE70AEC
  `move.l #0x2c000a,(A0); clr.b D0`.  **Uncertain:** module-local
  0xE1A4A4 returns `status.all = status_$ok` as a boolean, and its error
  path branches past the `tst.l (A0)` straight to the `seq D0`
  (0xE1A4BE `bra.s 0xE1A4D6`), using the flags of the store `move.l
  #0x100008,(A0)`: not supported.
- **Implemented** (`crates/dpc-m68k/src/isel.rs`): the stores, tests and
  pushes above through the existing patterns; `if c then return` as one
  branch to the epilogue (`Selector::if_`; an `else` part follows the
  branch; a `then return` that ends the routine is still refused);
  slots used once in bodies without branches and calls
  (`fresh_slot_loads`, `regs::allocate_apart`); function bodies of
  several statements when nothing is loaded at entry and the statements
  before the result only store, without calls or branches.  Golden
  `tests/golden/stores/status.pas`; tests in
  `crates/dpc-m68k/tests/control.rs`; byte-identical in `make conform`:
  `tests/conformance/iic.pas` (the IIC module),
  `xns_idp_get_stats.pas`, `io_alloc.pas`, and, with the A5 options of
  section 4 (always mode), `smd_busy_wait.pas` (SMD_$BUSY_WAIT
  0xE1D89E), `mmap_alloc_contig.pas` (MMAP_$ALLOC_CONTIG 0xE0D8F0),
  `xns_idp_get_port_info.pas` (XNS_IDP_$GET_PORT_INFO 0xE18FB8).
- **Not supported yet:**
  - A status whose last use is a call argument in a body with branches
    (12.4 R4(c)), e.g. `status.all := status_$ok; if ... then begin
    status.all := ...; return end; p(..., status)`.
  - A slot used once in a body with branches or calls (12.4 R4(b)).
- **Status code names** come from the real text tables
  (`~/src/domainos-archeology/stcodes`: `node stcode.js stcode.db.10.2`
  prints the whole 10.2 table, subsystem headings and `(hex)` codes;
  find a code by its `(hex)`, e.g. `(2c000a)` "device not in system"
  under "OS / internet interface controller").  The table gives texts,
  not symbols: the `status_$...` symbols are `../source`'s.
- **`abnormal`, `noreturn`, cleanup handlers:** the kernel has cleanup
  handlers (`../source/domain_os/proc2/cleanup_handlers_internal.c`,
  FIM_$CLEANUP).  How the compiler treats `options(abnormal)` around them
  is **Uncertain**.  The reference (5.5.6, 5.5.9) only says that
  optimisation is suppressed and that the return sequence after a
  `noreturn` call is dropped (compare CRASH_SYSTEM call sites; the
  AREA_$DEACTIVATE_ASTE quirk in `../source/docs/original-quirks.md`).

---

## 10. Interrupt priority in compiled code (Verified; implemented)

**Survey (Verified; beads domain-pascal-5aw.7, -0wc).**  Every compiled
routine, split and disassembled as `tools/loopalign.py` does (map `I`
segments, `_ASM` and HAND_ASM segments left out, codegen-patterns 0), was
searched for instructions naming SR.  There are only these 27, all listed
here (re-check with `tools/imgdis.sh`):

| Instruction | Count | Sites |
|---|---|---|
| `ori.w #0x700,SR` (`007c 0700`) | 10 | PROC1_$RESUME 0xE147AC; PROC1_$SUSPEND 0xE14850; PROC1_$UNBIND 0xE14E80, 0xE14F00; PROC1_$SET_PRIORITY 0xE15294; TIME_$SET_TIME_OF_DAY 0xE167EC; TIME_$ADJUST_TIME_OF_DAY 0xE16992; TIME_$GET_ADJUST 0xE16AB0; PROC1_$INIT 0xE2F9A4; module-local 0xE70954 (PEB_WIRED) 0xE70968 |
| `andi.w #0xF8FF,SR` (`027c f8ff`) | 7 | 0xE147EE; 0xE152C8; 0xE1686A; 0xE169AE; 0xE16ABA; 0xE70998, 0xE709B8 |
| `move.w SR,Dn` (`40c0+n`) | 5 | PROC1_$SET_VT 0xE149B6 (D3), 0xE149F2 (D1); PROC1_$TS_END_CALLBACK 0xE14A90 (D3), 0xE14AF8 (D0); WIN_$INT 0xE19C36 (D1) |
| `move.w <ea>,SR` (`46c0+ea`) | 5 | 0xE149B8 `#0x2700`, 0xE149F4 `D3`; 0xE14A92 `#0x2700`, 0xE14AFA `D3`; 0xE19C38 `#0x2500` |

No `move SR,-(SP)`, `move (SP)+,SR`, `move SR,<memory>` or other SR form
occurs in compiled code; those are hand assembly (ML_$SPIN_UNLOCK
0xE20BBE `move.w (0x8,SP),SR`, section 0).

- **Source spelling** (ref Appendix E, Table E-1, p. E-1; Table 4-7,
  p. 4-10).  E.2 (p. E-2): the routines "generate privileged instructions
  and may only be executed from supervisor mode"; that is a run-time
  matter, `dpc` puts no compile-time restriction on them.
  - `disable;` (p. E-3) is `ori.w #0x700,SR`; `enable;` (p. E-4) is
    `andi.w #0xF8FF,SR`, which **forces IPL 0** and does not restore the
    previous level.  The pairing is the image's: every `andi` follows an
    `ori` of the same routine; four `ori`s have no `andi` in their routine
    (PROC1_$SUSPEND 0xE14850 and PROC1_$UNBIND 0xE14E80 call
    PROC1_$TRY_TO_SUSPEND 0xE1471C with `bsr` right after them;
    PROC1_$UNBIND 0xE14F00 and PROC1_$INIT 0xE2F9A4 do not).  Encodings checked
    against `m68k-elf-as -m68010` (`ori.w #0x700,%sr`, `andi.w
    #0xf8ff,%sr`): identical.
  - `oldsr := set_sr(newsr);` (p. E-5, checked in the PDF, p. 568) is
    expanded "something like" `move.w SR,d0; move.w newsr,SR; move.w
    d0,oldsr`.  The image matches the first two instructions at all
    five sites: the old word is read into a data register, then the new
    one written, a constant (`#0x2700`, `#0x2500`) or a register (`D3`,
    the word an earlier `set_sr` saved).  At 0xE149B6 and 0xE14A90 the old
    word goes straight into the register of a local variable (D3, held
    across calls: section 12.5).  At 0xE149F2, 0xE14AF8 and 0xE19C36 it
    is never read: `discard(set_sr(old))` (ref "Discard", p. 4-62), or an
    assignment to a dead variable; the register is D1, D0, D1 although D0
    and D1 are both free at all three (Uncertain: `dpc` takes the lowest
    free register, which fits 0xE14AF8 only).  No site stores the old
    word to memory; `dpc` does it with `move.w Dn,oldsr` as the reference
    shows (reference only).
  - **Types (TODO(ref-question), reference-clarifications Q16):** the
    entry gives none.  `dpc` takes a 16-bit integer argument (or a
    constant in -32768..65535) and gives an `integer16` result, the
    `move.w`s of the expansion.
- **Implemented** (`dpc-ir` `StmtKind::Disable`/`Enable`/`Discard`,
  `Expr::SetSr`; `dpc-m68k` `Insn::SrImm`/`MoveFromSr`/`MoveToSr`).  A
  `set_sr` argument that must be computed, or read through an address
  register, is refused: whether that code comes before or after the
  `move.w SR` is not seen.  Golden `tests/golden/sysprog/interrupts`;
  conformance `tests/conformance/time_get_adjust.pas` (TIME_$GET_ADJUST
  0xE16AA8, byte-identical).
- **Barriers.**  Instructions of neighbouring statements are not moved
  across `ori`/`andi` at any site: the entry load 0xE16AAC stays before
  0xE16AB0; at TIME_$SET_TIME_OF_DAY the last store 0xE16864, a branch
  target, directly precedes the `andi` at 0xE1686A, and the pushes of the
  next call follow it.  Register *values* do cross: PROC1_$SET_PRIORITY
  stores D0 at 0xE15290 and compares it again after the `ori` at
  0xE15298 (section 12.4 R7).  `dpc` selects statements in order and has
  no scheduler, so it does the same; the register allocator treats the
  SR instructions as using no register.

### 10.1 The other predeclared routines (bead domain-pascal-0wc)

- **Folded in `dpc-sema`** (no code; the constant is then stored or used
  like any other, sections 1.5, 13):
  - `sizeof(x)` ("Sizeof", p. 4-181): x a type name, a variable, a
    constant or a string; the bytes of its type, a string constant's
    length ("SIZEOF('wildebeest')" is 10, p. 4-182).  An integer constant
    (`integer16` when it fits).  The second form with variant tags
    (p. 4-182) is not supported: how one variant's size is rounded is not
    stated.
  - `firstof(x)`/`lastof(x)` ("Firstof", p. 4-87; "Lastof", p. 4-106;
    both checked in the PDF, pp. 220, 239): the tables' rows for
    integer32/integer16 (`lastof` only), char, boolean, enumerations,
    arrays (the index type's bound) and varying arrays; a subrange
    (not in the tables) gives its bound, after "returns the first [final]
    possible value of x".  **`firstof` of integer, integer16 or integer32
    is refused**: p. 4-87 gives -32767 and -2147483647, one above the
    types' least values (TODO(ref-question), reference-clarifications
    Q17).
  - `succ`, `pred`, `max`, `min`, `in_range` of constants.
- **Code:**
  - `succ(x)`/`pred(x)` ("Succ", p. 4-191; "Pred", p. 4-147) of integers,
    chars and enumerations are `x + 1`/`x - 1` at x's type, with no range
    check (Appendix D, p. D-5), so they get the patterns of section 8
    (`addq.w #1` in place for `w := succ(w)`).  Of a boolean the result is
    `true`/`false` whatever x is; a non-constant boolean argument is
    refused (whether x is still evaluated is not known).  The image
    cannot tell `succ(x)` from `x + 1`.
  - `discard(f(...))`: a call statement, the result register unread
    (ref 5.2; section 3).  `discard(set_sr(x))`: above.  `discard` of any
    other expression is refused.
  - `max`, `min`, `in_range` of run-time values: **refused**.  The only
    candidates in the image are the five `x := min(x, c)`-shaped sites of
    12.4 R8 (`move.w #c,Dn; cmp.w Dn,Dx; ble.s; move.w Dn,Dx`: 0xE19ED0,
    0xE2F08A, 0xE66008, 0xE66372, 0xE874D6), which an `if x > c then x
    := c` would give too (0xE66372 even skips the store to memory when
    the value is in range).  No site that must be one of these built-ins
    is known.
- `addr` is section 1.5 and 5.2.

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
  that order when they are free; since bead domain-pascal-g5q.5 every
  value gets the lowest register of its class free over its live range,
  D2/A2 and up included, a callee-saved one when it lives across a call
  (`crates/dpc-m68k/src/regs.rs`; the model is 12.4 R1-R3).
- **Re-evaluation:** the same expression is reloaded rather than kept in a
  register (0xE135DA/0xE135E2 reload `(8,A2)`, an uplevel variable;
  which values are kept is in 12.4, R7/R8).
- **The order of evaluation of arguments** follows the push order (right to
  left), with address temporaries computed in between (0xE135D0..0xE135EE).
- The kernel's `.TEXT` shows strength reduction (shifts for constant
  multiplies), constant folding and common-address reuse.  The compiler
  *did* optimise a little.  Reproduce what is seen; invent nothing.
  The model of register choice, CSE and scheduling is in 12.4.

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
look before coding).**  Section 12.4 now models these points: R1 and R3
for the first two bullets, R4(a) for the third.  The bullets are kept
for their sites.

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
  scheduler; Verified that it happens, rule unknown; 12.4 R9–R12): `movea.l #imm,A0`
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
    In bodies without branches and calls the rule is "the lowest
    register no earlier statement left a value in, unless an `(An)+`
    used that value up" (12.4 R4(b), Verified there by survey and
    implemented); elsewhere it is not settled.
  - The same register is also kept for a slot used more than once when
    calls intervene: the status pointer, 9.
- *A held value is copied before arithmetic:* BAT_$GET_BAT_STEP 0xE3BB68
  `move.w D0,D1; mulu.w #0x234,D1`; module-local 0xE46498 `move.w D0,D1;
  add.w D1,D1`.  **Implemented** (section 8, operands): a value held in a
  register is copied into a new one before an operation changes it; in a
  one-statement body a by-value parameter used as an array index is
  loaded and copied the same way (both sites above are one-statement
  functions).  But RIP_$PACKET_LENGTH 0xE68868..0xE6886C, also a
  one-statement function, loads its parameter into D0 and doubles it in
  place (`add.w D0,D0`): an operand of arithmetic is not copied.
  **Uncertain:** why an index is and an operand is not.

**Implemented** (`crates/dpc-m68k/src/isel.rs`, `select_body`,
`Selector::entry_loads`): a *procedure* body of two or more statements,
or of one `if`/`repeat`, loads every by-value parameter it reads and
every by-address slot it uses more than once, in parameter order (D for
data, A for a dereferenced pointer or a slot).  Since bead
domain-pascal-g5q.5 these loads are ordinary values for the allocator
(12.4 R1-R3, `crates/dpc-m68k/src/regs.rs`): each takes the lowest
register of its class free over its live range, so a third value of a
class takes D2/A2, a value live across a call a callee-saved register
(module-local 0xE32B76: A2, D2 around three `jsr`; PROC1_$SET_TYPE
0xE152F2: D2, D3), and a parameter's register is free after its last
use.  The statements of such a body take registers of their own the same
way.  A body of one `if`/`repeat` (or `while`/`for`) that reads a by-value
parameter is rejected (no site seen).  A slot used inside a loop body
counts as used more than once and is loaded at entry (bead
domain-pascal-5aw.15; the DISK_ module-local 0xE3CCD6 `movea.l
(0xc,A6),A0`, the status slot of its `while` loop's one store;
NAME_$VALIDATE's module-local 0xE4A004 loads the slots its loops use at
0xE4A018/0xE4A01C and the one used once outside them where it is
needed, 0xE4A03C; section 7.4).  A slot used once in a body of several
statements takes a register no earlier statement left a value in (12.4
R4(b), bead domain-pascal-5aw.2; IIC_$RECEIVE 0xE70AC0..0xE70AD0) when
the body has no branches and no calls, and is rejected otherwise, except
in a simple statement right after a call in a body that loaded nothing
at entry (0xE00ECC..0xE00ED2: A0, D0 after the `jsr` at 0xE00EC2).  A parameter that is only pushed as a whole argument is not
"read" for this rule: it is pushed from the frame at each call (0xE00EBE,
0xE00EDE; AST_$SET_ATTRIBUTE 0xE053DA).  A one-statement body keeps its
own evaluation order (PROC1_$TST_LOCK, PROC1_$GET_LOCKS,
TIME_$Q_FLUSH_QUEUE stay byte-identical).  A function body of several
statements is compiled only when it loads nothing at entry and its
statements before the result assignment only store, without calls or
branches (IO_$ALLOC 0xE1A9C2, IIC_$RECEIVE_CHECK 0xE70ADE; section 9).
Reported as not supported yet: other function bodies of several
statements (R5); a pointer parameter also
used as data; storing
a parameter that was not loaded at entry (one-statement bodies);
assigning to a parameter; in a body with branches, a parameter whose
last use is an argument of a call (12.4 R4(c)).  In a body without
branches such a parameter is held through the call, in a callee-saved
register (12.4 R4(c): PROC1_$SET_ASID 0xE148FE D2, a conformance unit).

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
   while D0/D1 hold parameters.  **Done** (bead domain-pascal-d7i;
   section 8, `&`): `dpc` produces 0xE17778..0xE177A4 byte for byte in a
   reconstruction cut after the `if` that ends at 0xE177A4.
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
   Section 7.4 (bead domain-pascal-96r) now describes the shape in
   general: pointers start at `base + from × size`, are formed in the
   order of first use and stepped in the reverse order (29 of 30 loops).
   `dpc` compiles `for` loops since bead domain-pascal-5aw.15, but not
   this one: it refuses (i) a `for` index indexing elements wider than a
   byte (the stepped pointers are not implemented; this is the worker's
   first diagnostic once item 5's type transfer is lowered) and (ii) a
   run-time count in a loop without calls, where the count is copied into
   a counter of its own 45 times and, as here, kept in place 64 times,
   with no rule found.
7. The frame: `link #-0x10` with nothing in it addressed (section 2,
   untouched frames); the reconstruction declares stand-ins.

Items 2–5 are taken up by the model in 12.4.

### 12.4 Register allocation and optimisation model (beads domain-pascal-hy0, -6tn, -5cx)

**What the reference says** (6.4.24 "-opt: Optimized Code"; we do not
know which level the kernel was compiled at, and none of this is
evidence about the image):

- `-dba` and up (p. 6-27): "Rearranges expressions to minimize the
  number of registers needed to compute them"; "Computes multiple
  occurrences of the same expression within a statement only one time".
- `-opt 0` (p. 6-27): "Permits values to remain in registers across
  statements"; "Merges identical sequences of instructions".
- `-opt 1` (pp. 6-27, 6-28): "Eliminates some global common
  subexpressions"; "Transforms integer multiplication by a constant into
  shift and add instructions"; "Merges assignment statements where
  possible".
- `-opt 3`, the default (pp. 6-28..6-32): "Global register allocation
  allows local variables to have their values placed in machine
  registers" (p. 6-29); "Instruction reordering" (p. 6-29); "Removal of
  invariant expressions from loops" (p. 6-29); "Strength reduction"
  (pp. 6-30, 6-31, with the array-pointer example of section 12.3 item
  6);
  "Live analysis on local variables" ("allocate a local variable to a
  machine register for exactly as long as the variable's value is
  needed", p. 6-32); "Extensive searches ... for global common
  subexpressions" (p. 6-32).

The image shows global register allocation, global CSE, strength
reduction and instruction reordering (sites below).  In the reference
that combination needs `-opt 3` or higher.  The level actually used is
not recorded (Uncertain).

**Method.** `tools/regalloc.py` (docstring) splits every compiled routine
as `tools/loopalign.py` does, builds a def/use model of each instruction,
and forms *values* (webs: the definitions of one register that reach
common uses; a two-address update extends its value).  For each value it
asks whether a rule, given the registers of all other values, picks the
register the compiler chose.  A rule also fails when the chosen register
would conflict under it.  `summary` gives the rates; `misses RULE` lists
sites; `routine ADDR` shows one routine; `corpus` prints the table below.
Survey: 1099 routines of 0x30–0x150 bytes (14 skipped for `case` jump
tables), 4978 data and 5158 address values; also the whole image (1588
routines).

*What a "hit" means.*  The lifetimes come from the machine code, and each
value is tested against the registers the original gave all the *other*
values (leave one out).  Any allocator that colours values one at a
time, each with the lowest register its already-coloured neighbours
leave free, scores 100% on this test whatever order it colours in.  So
the `col`/`colU` rates measure how often the original's choice is
*locally minimal*.  They do not say in which order the compiler
allocated.  `colQU` counts only values defined earlier in address
order, i.e. an allocator that works through the routine from the top;
it is the rate a one-pass allocator in `dpc` could expect (R1, below).

*Review fixes (2026-10-12).*  Two model bugs were fixed and the numbers
below re-run.  (1) Every instruction that read and wrote one register
was treated as a two-address update.  That merged a load through a dying
base into the same register (`movea.l (4,A0),A0`) with the base's value,
which hid R2's counterexamples.  Such loads are now new values; `lea
(d,An),An` (an `adda`) is still an update.  (2) The dead last `(A0)+` of
a block copy, or a dead `dbf` counter, made A0/D0 a "function result",
live up to every `rts`, in 170 of the 1099 routines.

**Corpus** (studied against the C; `tools/regalloc.py corpus`): 44
routines, 329 values, 268 predicted by rule R1 below (`colU`), 61 not.

| Routine | Size | C in `../source/domain_os` | Values | R1 misses |
|---|---|---|---|---|
| module-local 0xE17778 | 0x88 | `log/read_internal.c` | 14 | 0xE177C4 D3, 0xE177D0 D3 (R1: D2) |
| LOG_$READ 0xE17800 | 0x28 | `log/read.c` | 1 | |
| MST_$VA_TO_SEGNO 0xE0DCC8 | 0x78 | `mst/va_to_segno.c` | 10 | 0xE0DCD6 D2, 0xE0DCE6 D3 |
| AST_$LOCATE_ASTE 0xE07050 | 0x9C | `ast/locate_aste.c` | 13 | 0xE0707E, 0xE0708E, 0xE07098, 0xE070B8 (D), 0xE070B6, 0xE070CA (A) |
| AST_$INVALIDATE_PAGE 0xE00F16 | 0x66 | `ast/invalidate_page.c` | 9 | 0xE00F34, 0xE00F4E A1 |
| FIM_$ACKNOWLEDGE 0xE0A96C | 0x56 | `fim/acknowledge.c` | 8 | 0xE0A972 D2, 0xE0A9A0, 0xE0A9AA A1 |
| FIM_$INSTALL 0xE0A9C2 | 0x42 | `fim/install.c` | 6 | 0xE0A9EA D2, 0xE0A9F0 A1 |
| FIM_$INIT_ASID 0xE0AA24 | 0x48 | `fim/init_asid.c` | 8 | 0xE0AA5A A1 |
| AST_$SET_TROUBLE 0xE071EA | 0x36 | `ast/set_trouble.c` | 2 | 0xE071FC A2 |
| module-local 0xE0B1BC | 0x46 | `mac_os/check_range_overlap.c` | 10 | 0xE0B1E2 D2 |
| module-local 0xE0B202 | 0x44 | `mac_os/find_packet_type.c` | 9 | |
| MMAP_$GET_WS_INDEX 0xE0CA3A | 0x40 | `mmap/get_ws_index.c` | 5 | 0xE0CA62 A2, 0xE0CA68 A1 |
| MMAP_$SET_WS_MAX 0xE0CA7A | 0x48 | `mmap/set_ws_max.c` | 5 | 0xE0CAAE D3 |
| module-local 0xE01BEE | 0x64 | `vtoc/search_volumes.c` | 6 | 0xE01C10 D1 |
| module-local 0xE0283C | 0x5C | `ast/clear_transition_bits.c` | 6 | 0xE02852 D1, 0xE0285A D3, 0xE0286A D1 |
| AREA_$DELETE 0xE07C22 | 0xE4 | `area/delete.c` | 10 | 0xE07C86, 0xE07CA0 D1 |
| PROC2_$SET_SERVER 0xE41468 | 0x74 | `proc2/set_server.c` | 7 | 0xE414A0 D3 |
| module-local 0xE15830 | 0x58 | `rip/update_int.c` | 10 | 0xE1584A, 0xE15854, 0xE1585E, 0xE15868 D2, 0xE1586E D3 |
| PROC1_$SUSPENDP 0xE14876 | 0x54 | `proc1/suspendp.c` | 7 | |
| PKT_$DUMP_DATA 0xE127E6 | 0x4E | `pkt/dump_data.c` | 7 | 0xE12808 D1 |
| NETWORK_$REMOVE_NET 0xE0F27C | 0x50 | `network/network.h` (declaration only) | 4 | 0xE0F2A4 D2 |
| TIME_$Q_INIT_QUEUE 0xE16C5E | 0x22 | `time/q_init_queue.c` | 3 | |
| PROC1_$TST_LOCK 0xE148CA | 0x1C | `proc1/tst_lock.c` | 5 | |
| PROC1_$SET_ASID 0xE148F8 | 0x26 | `proc1/set_asid.c` | 2 | 0xE148FE D2 |
| AREA_$FREE_ASID 0xE07E80 | 0x8E | `area/free.c` | 5 | |
| PMAP_$PURGE_WS 0xE146B4 | 0x68 | `pmap/purge_ws.c` | 4 | |
| PARITY_$CHK_IO 0xE0B174 | 0x48 | `parity/chk_io.c` | 6 | |
| IIC_$RECEIVE 0xE70ABA | 0x24 | `iic/receive.c` | 3 | 0xE70ACA A1, 0xE70AD0 A2 |
| module-local 0xE1359C (nested, PMAP_$FLUSH) | 0x70 | `pmap/update_seg_map.c` | 5 | 0xE135E2 A1, 0xE135E6 A3 |
| FIND_VOLX 0xE6B0BC | 0x5C | `volx/find_volx.c` | 10 | |
| module-local 0xE7630C | 0x46 | `ring/find_pkt_type.c` | 9 | |
| module-local 0xE1C9FC | 0x66 | `kbd/state_lookup.c` | 12 | 0xE1CA04 D4, 0xE1CA20 D5, 0xE1CA32 D5, 0xE1CA4A D2 |
| module-local 0xE54E10 | 0x52 | `dir/old_alloc_block.c` | 10 | |
| ACL_$GET_RE_SIDS 0xE488B6 | 0x56 | `acl/get_re_sids.c` | 10 | 0xE488F0 A2, 0xE488F4 D1, 0xE488FC A1 |
| MSG_$TEST_FOR_MESSAGE 0xE59FB2 | 0x7A | `msg/test_for_message.c` | 12 | 0xE59FFE D1 |
| module-local 0xE3EF38 | 0x68 | `proc2/deliver_pending_internal.c` | 10 | 0xE3EF90 D3 |
| module-local 0xE190BC | 0x6C | `win/wait_for_controller.c` | 11 | 0xE190D6, 0xE19106 D3; 0xE190DC A0 (the next value, `movea.l (4,A0),A0` at 0xE190E0, reuses it: against R2) |
| TERM_$P2_CLEANUP 0xE751F0 | 0x60 | `term/p2_cleanup.c` | 12 | |
| module-local 0xE65904 | 0x76 | `route/validate_port.c` | 11 | |
| ACL_$INHERIT_SUBSYS 0xE49138 | 0x5E | `acl/inherit_subsys.c` | 10 | 0xE49166 D2, 0xE49174 D3, 0xE49186 A2 |
| module-local 0xE0C6E2 | 0x7E | `mmap/internal.c` | 14 | 0xE0C714 D3, 0xE0C742 D2 |
| XNS_IDP_$GET_STATS 0xE18FD6 | 0x2C | `xns/idp_misc.c` | 2 | 0xE18FF4 A1 |
| PROC1_$SET_TYPE 0xE152E4 | 0x40 | `proc1/set_type.c` | 4 | |
| MMAP_$ALLOC_CONTIG 0xE0D8F0 | 0x24 | `mmap/alloc_contig.c` | 2 | 0xE0D902 A1 |

#### Register choice

**R1. One register per value, the lowest-numbered one that does not
conflict (Verified as a property of about 81% of values; the order in
which values are given registers is Uncertain).**  The order is D0, D1,
..., D7 and A0, ..., A4.  The register must not hold another value at any
point of this value's live range (Chaitin interference: one is defined
where the other is live).  There is no separate "scratch first" policy:
D2 and A2 are simply the next registers in the order.  In 0xE17778 there
is no call, yet `moveq #-2,D2` (0xE177A0) takes D2 because D0 and D1 hold
parameters.
- Parameters loaded at entry are ordinary values.  Their registers are
  reused after their last use: D1 (`max_len`, last read 0xE177A2) gets
  the zero-extended `offset` at 0xE177A6; D0 (last read 0xE177C8) gets
  the word count at 0xE177D2; A0 (last read 0xE177D2) gets the source
  pointer at 0xE177E0.
- Rates of rule `colU` (R1 + R2 + R3), and of `colQU` (the same, with
  values coloured in address order):

  | Values | Data `colU` | Address `colU` | Data `colQU` | Address `colQU` |
  |---|---|---|---|---|
  | all, size 0x30–0x150 | 81.1% of 4978 | 81.5% of 5158 | 72.8% | 73.1% |
  | live in more than one block | 93.2% of 2292 | 91.0% of 2253 | 77.1% | 76.5% |
  | parameter loads | 95.1% of 698 | 83.2% of 1891 | 69.8% | 71.0% |
  | whole image | 75.0% of 11517 | 76.5% of 11112 | 65.1% | 68.8% |

- **The order is not settled.**  `colU` is order-free (see "What a hit
  means"); colouring in address order loses 8–9 points.  The typical
  loss: parameters loaded at entry and held across calls take D3/A3 or
  higher while later temporaries take D2/A2 (module-local 0xE01C52,
  `ast/lookup_with_hints.c`, 0xE01C5A..0xE01C66: A3, D5, D6, A4;
  AST_$ASSOC 0xE0445C..0xE04464: D3, D4, A3).
  Also PARITY_$CHK_IO 0xE0B180 (`(8)`→D1, R5) and PROC1_$TST_LOCK
  0xE148DE (`sne D1` while D0 is free; the result is then copied to D0).
  Other orders scored no better (by survey, data / address): reverse
  address order 58.6% / 65.0%; shortest live range first 59.3% / 65.5%;
  single-block values first 67.1% / 70.1%; longest first 71.9% / 72.8%.
- Linear scans that look only at the definition point do worse: "lowest
  free at the definition in address order" (`lo`) scores 60.1% / 57.0%;
  "most recently freed" (`lifo`) 58.6% / 50.2%.  Unlike `colQU`, they
  ignore later conflicts and calls inside the range.
- **The misses go mostly one way.**  In 893 of the 939 data misses and
  909 of the 955 address misses, the original took a *higher* register
  than `colU`.  The original keeps some registers busy *longer* than the
  value's liveness (R4).  Most of the 46 + 46 others are loads that
  reuse their dying base register (R2) or call results.

**R2. A value does not take a register read by an instruction that
defines it while the other value dies there (Verified for copies and
operator results as an improvement: R1 alone scores 78.2% / 78.0%,
R1+R2 81.1% / 81.5%; it gains 386 values and loses 61).**  In other
words, no copy coalescing:
- 0xE177D0 `move.l D1,D3; ...; add.l (0x14,A5),D3`: D1's last use, but
  the sum is not computed in D1.
- 0xE177DA `movea.l D3,A1`: A0 is free, but the next A0 value is defined
  from this one (`lea (2,A1),A0`, 0xE177E0).
- PROC1_$SUSPENDP 0xE1489A A2: A1 is free, but A1 is defined from it
  (`movea.l (A2,D2.w),A1`).
- 12.2's "held value copied before arithmetic": BAT_$GET_BAT_STEP
  0xE3BB68; 0xE46498.
- **Not for loads through a dying register (Uncertain).**  Routines of
  0x30–0x150 bytes, loads whose base or index register dies at the load:
  `movea.l` into the same A register 35 times (0xE01BBC `movea.l
  (A0),A0`; 0xE190E0 `movea.l (4,A0),A0`), into another one 144 times;
  `move.w`/`move.l` into the index register 28 times (0xE176C4
  `move.w (2,A0,D1.w),D1`), into another 41 times.  `lea (d,An),An` is
  counted as an update (`adda`), not a new value: 337 such, against 378
  `lea` into another register while the base dies.  When the compiler
  reuses the base is not known.

**R3. Calls (Verified).**
- A value live across a call is in D2–D7/A2–A4.  1834 such values in
  the survey; `colU` places 99.2% of them right (lowest non-conflicting
  callee-saved register), `colQU` 80.2%.  That they are callee-saved is
  built into the model (a call writes the scratch registers); the
  evidence is that, in the whole image, no D1/A1 read is reached by a
  non-M$ARITH call except on one path at 0xE47442 and 0xE47624.
- No value lives in D0/D1/A0/A1 across a call, **except across M$ARITH
  calls**, which the compiler treats as writing only D0 and A0.  Example:
  A1 is read before `jsr M$OIU$WLW` (0xE3CB8E, call at 0xE3CB94) and
  after it (0xE3CB9E).  In the 1588 analysed routines, D1 or A1 is live
  across 31 of the 83 M$ARITH calls (42 register/call pairs).  This is
  what M$ARITH does: every one of its twelve entry points writes only D0
  and A0 (and the flags); those that use D1–D3 save them with `movem`
  (`../source/domain_os/math/sau2/arith.s`; M$DIS$LLW also writes its
  divisor argument slot).  Re-counted by `tools/arithsurvey.py summary` (bead
  domain-pascal-5aw.5): 31 of 83, 42 pairs; the preservation of D1-D3/A1
  checked by running the entries in a scratch evaluator.  Whether R4(c)
  below applies to M$ARITH calls is not settled: `dpc` does not hold for
  them; TIME_$GET_ADJUST 0xE16AD6 pushes D1 for its last use and D1 stays
  a scratch register, but D1 holds a local variable there, not a
  parameter loaded at entry (section 8; reproduced, section 12.5).

**R4. Registers are held longer than liveness (Uncertain; this is most
of the remaining misses).**  The corpus misses fall into these groups:

- **(a) Held to the end of the enclosing statement.**  A value whose
  last use lies in the condition or in an arm of a structured statement
  (`if`/`else if` chain, an `and then` condition, a `for`/`while` with
  its body) keeps its register until the end of that statement:
  - 0xE15830 (`rip/update_int.c`): the flag parameter in D1, tested at
    0xE15846, is not reused anywhere in the `if`; D2 is used three times
    (0xE1584A, 0xE15854, 0xE1585E), then D2 and D3.
  - AST_$LOCATE_ASTE: `hint_index` in D0 (last read 0xE0707A); the
    nested `if` inside the outer `if` uses D1 (0xE0708E, 0xE07098,
    0xE070B8).
  - AREA_$DELETE 0xE07C86, 0xE07CA0.
  - 0xE0B1E2: the D1 temporary of the first `and then` term is not
    reused by the second.
  - 0xE0283C 0xE02852, 0xE0285A, 0xE0286A (the count stays in D0
    through the loop).
  - MMAP_$SET_WS_MAX 0xE0CAAE.
  - MMAP_$GET_WS_INDEX 0xE0CA62/0xE0CA68: the status pointer A0 is held
    through the `if`.
  - FIM_$INSTALL 0xE0A9EA (`old_addr` in D1 is held through the `if` it
    is tested in).
  - 0xE01C10, 0xE0F2A4, 0xE12808.
  - Inside one argument list: 0xE1359C (A0, A1, A3; section 12.1).
  - Against it, registers *are* reused inside one statement:
    `movea.l (8,A2),A0` twice in `p^.f := p^.f - n` (0xE03FEE,
    0xE03FF8).  FIM_$ACKNOWLEDGE reuses A1 for each statement
    (0xE0A98E, 0xE0A9A0, 0xE0A9AA).
- **(b) By-address parameter slots read once take a register no earlier
  slot load used** (12.2):
  - IIC_$RECEIVE 0xE70AC0..0xE70AD0 A0, A1, A2; XNS_IDP_$GET_STATS
    0xE18FF4 A1; MMAP_$ALLOC_CONTIG 0xE0D902 A1; AST_$INVALIDATE_PAGE
    0xE00F34/0xE00F4E A1, the same register again after the call;
    0xE177CC A2.
  - Against it: DIR_$ADD_MOUNT loads three slots into A0 (0xE534CA,
    0xE534DC, 0xE53508; the first two feed `move.l (A0)+` block copies);
    PROC2_$GET_PID 0xE40D06; 0xE19158.
  - **Survey (bead domain-pascal-5aw.2; ad-hoc script on top of
    `tools/regalloc.py`'s model, not kept).**  For every `movea.l
    (d,A6),An` of a slot that starts a value, two rules were compared:
    `colU` (lowest register free over the range) and "fresh": the lowest
    of those whose content at the load is not a value an earlier
    instruction left there, a value used up by `(An)+`/`-(An)` and a
    register a call wrote counting as free, a reload of the same slot
    into the register that already holds it allowed, `colU` when no
    register qualifies.  In routines **without any branch or call**: 105
    loads, both rules agree on 86, "fresh" alone is right 17 times
    (all the IIC_$RECEIVE, PROC2_$ALIGN_CTL, XNS_IDP_$GET_STATS,
    MMAP_$ALLOC_CONTIG sites; module-local 0xE32BB8 `(0x14,A6)`→A2 with
    A0 and A1 dead; TERM_$PCHIST_ENABLE 0xE7245C A1 after the status
    cleared through A0), `colU`
    alone once (0xE41C2E: A0 reused for a slot right after its last use
    as a base in the same statement) and neither once (0xE3353A).  With
    branches before the load, "fresh" alone 85, `colU` alone 28,
    neither 14; with branches and calls, 102 / 81 / 30 (e.g. module-local
    0xE030C0, 0xE030E0 `movea.l (8,A6),A0` right after `clr.l (A0)` of the
    status in A0).  So the rule holds in straight-line, call-free code
    as "a register no earlier *statement* left a value in" (the 0xE41C2E
    reuse is within a statement); elsewhere something else decides.
  - **Implemented** for bodies without branches and calls
    (`crates/dpc-m68k/src/isel.rs`, `fresh_slot_loads`; the allocator
    keeps each such slot apart from the address values of earlier
    statements that no `(An)+` used up, `regs::allocate_apart`).  Bodies
    with branches or calls are refused.  The slot loads of block copies
    (`block_address`) still take the lowest free register (ACL_
    0xE47944 suggests the same rule applies there: A2 after an `(A2)+`
    copy while the dead A0 of an earlier statement is passed over; not
    implemented).  Conformance: `tests/conformance/iic.pas`,
    `xns_idp_get_stats.pas`.
- **(c) A call in the statement.**  A value that does not live across a
  call, but belongs to a statement that ends in one, gets a callee-saved
  register:
  - FIM_$ACKNOWLEDGE 0xE0A972 `asid*12` → D2; its last use is the `pea`
    before the `jsr`.
  - PROC1_$SET_ASID 0xE148FE D2.
  - AST_$SET_TROUBLE 0xE071FC A2.
  - Parameters loaded at entry whose last use is an argument of the
    routine's last call, seen while looking for step-1 units (bead
    domain-pascal-g5q.5; not counted separately): TTY_$I_FLUSH_OUTPUT
    0xE1B80C A2 (`pea (A2); bsr.w` at 0xE1B816); module-local 0xE1D592
    A2 (0xE1D59E); SIO_$I_INIT 0xE67E5E (`sio/i_init.c`) A2
    (0xE67E74 `pea (0x68,A2)`).  `dpc` now reproduces these in bodies
    without branches (below).
  - Survey: 433 values of this kind (callee-saved register, no call
    crossed, a call later in the block of the last use).  Against: 1365
    values in scratch registers whose block also continues to a call.
  - **Hypothesis tested (review of bead domain-pascal-g5q.5, 2026-10-12):
    a value defined before a call statement whose last use is in that
    call's arguments is live through the call.**  `tools/regalloc.py
    callargs` (whole image, no size bound): the argument sequence of a
    call is the run before it in its block back to the previous store
    that is not a push, or the previous call; M$ARITH calls do not count.
    Values not crossing the call, whose every last use is there, and for
    which liveness alone leaves a scratch register free:
    - parameter loads, routines **without branches: 5 callee-saved, 0
      scratch**: PROC1_$SET_ASID 0xE148FE D2; TTY_$I_OUTPUT_BUFFER_DRAINED
      0xE1B39A A2; TTY_$I_FLUSH_OUTPUT 0xE1B80C A2; module-local 0xE1D592
      at 0xE1D598 A2; SIO_$I_INIT 0xE67E64 A2.  Other values: 6
      callee-saved, 2 scratch (TIME_$RELEASE 0xE58B9C; 0xE5AA84 in the
      routine at 0xE5AA28, a value reused after its store);
    - parameter loads, routines **with branches: 26 callee-saved, 49
      scratch** (the proxy also counts some values last used in a
      register-variable assignment just before a call, e.g. AREA_$DELETE
      0xE07C34).  Checked scratch sites: EC2_$ADVANCE 0xE42CB8 D0
      (`move.l D0,-(SP)` at 0xE42CDA); DIR_$FIND_NET 0xE4FE36 A0 (`pea
      (A0)` at 0xE4FE76); DIR_$DO_OP 0xE4D00E A0 (0xE4D040); TTY_$I_INIT
      0xE3325A A0 (0xE332DC); AST_$UPDATE 0xE01B6E A0 (0xE01B82);
      MAC_$SEND 0xE0BB88 A0 (0xE0BB9C); the routine at 0xE55220 (map:
      DIR_$OLD_CLEANUP+0x6F6), 0xE5523C A0, `clr.l (A0)` then `pea (A0)`
      at 0xE5524C, the shape of 0xE1D592;
    - temporaries computed inside the argument list: 1198 scratch, 276
      callee-saved.

    So the hypothesis holds without exception only in routines without
    branches (why is Uncertain).  `dpc` holds such a parameter through
    the call in a body without branches (`held_through_calls` in
    `crates/dpc-m68k/src/isel.rs`; conformance unit PROC1_$SET_ASID) and
    refuses it in a body with branches.
- **Mechanical proxies tested and refuted as general rules** (each makes
  the original's own choices conflict elsewhere):
  - free only after the next store or call (`colUS`): 66.5% / 69.0%;
  - free only at the post-dominator of a conditional block (`colUC`):
    52.4% / 55.1%;
  - parameter values live from the entry (`colEU`): 80.9% / 70.6%;
  - parameter values live from the start of their block (`colLU`):
    81.1% / 80.6%.

  The rule needs the source's statement structure.  `dpc` has it, so (a)
  must be implemented and tested in `dpc` against the sites above.

**R5. Function results (Uncertain).**
- MST_$VA_TO_SEGNO loads the parameter it returns straight into D0
  (0xE0DCE0), and the first parameter goes to D2 (0xE0DCD6), not D0/D1.
- PARITY_$CHK_IO loads `(8)` into D1 and `(0xc)` into D0 (0xE0B180,
  0xE0B18A; section 12.2).
- The rule that ties a parameter or temporary to the result register is
  not known.

**R6. Not explained (Uncertain).**
- 0xE177C4 `move.w #0x400,D3` and 0xE177D0 `move.l D1,D3`: D3 although
  D2 is free under R1–R3.  The (a) rule does not help:
  - at 0xE177C4 it would also hold D3 (the `0x400` of the condition),
    giving D4;
  - at 0xE177D0 nothing holds D2 any more.
- FIM_$ACKNOWLEDGE 0xE0A9A0/0xE0A9AA, FIM_$INIT_ASID 0xE0AA5A and
  FIM_$INSTALL 0xE0A9F0 load an array base into A1 while A0 is free.
- Candidate explanations, none verified:
  - **LIFO free list** (last freed first; ties go to the value allocated
    later).  It gets 0xE177C4 (D2 and D3 die together at 0xE177B6, D3
    was allocated later), 0xE177D0 (D3 freed last, 0xE177CA) and the
    FIM_$ACKNOWLEDGE/FIM_$INIT_ASID A1s.  But it fails FIM_$INSTALL,
    and over the survey it explains only 352 of 939 data misses and 216
    of 955 address misses while breaking 450 / 529 hits (`lifoU`:
    79.2% / 75.4%).
  - 0xE177C4 only: the register of the `0x400` CSE (R8) is reused for
    `0x400 - offset`, and the constant is reloaded into it.
  - 0xE177D0 only: D2 belonged to the `for` variable `i`, which
    strength reduction removed (ref p. 6-30: "the optimizer may eliminate
    all assignments to i"), and `ord4(ptr) + offset` is a loop invariant
    hoisted into the preheader while `i` is allocated (ref p. 6-29).

#### Common subexpressions

**R7. Operator results are reused across statements, across an `if` and
its join, and past stores to other objects (Verified at the sites).**
- 0xE177A6 `clr.l D1; move.w D0,D1`, the zero-extended `offset`, is
  reused at 0xE177D0 after the `if` (0xE177A6..0xE177CA) and after the
  stores through `actual_len`.
- FIM_$ACKNOWLEDGE: `PROC1_$AS_ID * 12` (0xE0A972..0xE0A97E, D2) is used
  at 0xE0A994 and again at 0xE0A9B0, past the stores at 0xE0A994 and
  0xE0A9A6 (other static arrays).
- Two-address arithmetic goes into the operand that is not reused:
  0xE177B4 `add.l D1,D2` keeps D1 (the CSE).

**R8. Plain variable reads are not kept across statements (Verified at
the sites).**
- PROC1_$AS_ID is re-read for each use: FIM_$ACKNOWLEDGE 0xE0A972,
  0xE0A980, 0xE0A99A; FIM_$INSTALL 0xE0A9D0, 0xE0A9EA (the C comments
  this).
- Within one statement it varies:
  - 0xE0283C reads `(2,A2)` once for the two compares of one condition
    (0xE0285A..0xE02868), then again in the next statement (0xE0286C).
  - FIM_$ACKNOWLEDGE reads PROC1_$AS_ID twice for its first statement
    (0xE0A972 for ×12, 0xE0A980 for ×4).
- Uplevel variables (through the static link) are re-read even within
  one statement: 0xE03FEE/0xE03FF8; 0xE135DA/0xE135E2.
- Uncertain against: AREA_$DELETE reads the flags word once
  (0xE07C78) for the `while` test and for the next `if` (0xE07C80).
- **Constants in registers** (Verified, rare).  In the whole image there
  are only 11 `move #imm,Dn` followed within three instructions by
  `cmp Dn,..`; `.TEXT` alone has 996 `cmpi.w` and 355 `cmpi.l`
  (section 13).
  - Five have the `x := min(x, c)` shape, where the constant is used
    twice: `move #c,Dn; cmp Dn,Dx; b..; move Dn,x` at 0xE19ED0,
    0xE2F08A, 0xE66008, 0xE66372, 0xE874D6.
  - Two are the same shape for `c - y`: `move.l #c,Dn; sub.l Dy,Dn`
    (0xE0E964, 0xE637B6).
  - Two are a constant assigned to a variable's register: 0xE6DAF2 in
    one arm of an `if`; 0xE43228 after `cmpi.l`.
  - 0xE60F52 (and 0xE60F4A) load the mask 16#FFFFF and AND memory into
    it, constant first.  0xE177A0 `moveq #-2,D2; and.w D1,D2` has the
    same order.
  - 0xE177AE is the eleventh: `move.l #0x400,D3; ...; cmp.l D3,D2`.
    - Its only other long use would be `0x400 - offset` at 0xE177C4,
      done as `move.w #0x400,D3` in the same register.
    - The word compares 0xE1779A and 0xE177BA use `cmpi.w`.
    - Why this compare holds its constant in a register is Uncertain.
- Multiplication by a constant becomes shifts and adds (ref p. 6-28;
  Verified):
  - ×12: 0xE0A972..0xE0A97E;
  - ×20: AST_$LOCATE_ASTE 0xE0707A..0xE07082;
  - ×36: MMAP_$SET_WS_MAX 0xE0CAAA..0xE0CAB2;
  - ×48: 0xE07C52..0xE07C5C.

  The product stays in the first register, and the partial product takes
  another free one (D0 at 0xE0A97A, D2 at 0xE0707E, D3 at 0xE0CAAE).

#### Instruction order (scheduling)

**R9. Instructions are reordered inside a basic block (Verified that it
happens).**
- 0xE177A6..0xE177B8: both `clr.l` first, then both word moves, the
  constant load between the second move and the `add` that uses it.
- 0xE177CC..0xE177DE: three chains interleaved, `buffer` (A2), the
  source pointer (D3 → A1) and the word count (D0).
- FIM_$ACKNOWLEDGE 0xE0A980..0xE0A994: the two `movea.l #` bases between
  the index load, its shift and the move.
- MST_$VA_TO_SEGNO 0xE0DCD6..0xE0DCEA: the third parameter load
  between the copy and the first shift, and `clr.l D3` between the two
  shifts.
- `movea.l #imm` or a pad between two parameter loads: PARITY_$CHK_IO
  0xE0B184; module-local 0xE758C4 at 0xE758CC; PMAP_$PURGE_WS
  0xE146C6.
- The pad before the parameter load: AREA_$FREE_ASID 0xE07E8E.

**R10. Barriers (Verified at the sites).**
- Nothing moves across a call.
- A call's pad is not hoisted above the previous statement's store
  (0xE00C12/0xE00C16; 0xE0774C/0xE07750), although nothing in the
  registers stops it.
- Shift-and-add multiplications and `(An)+` block copies are never
  interleaved with other code:
  - ×12: 0xE0A972..0xE0A97E (an independent `movea.l #` was ready);
  - 0xE190D2..0xE190DC;
  - 0xE14A2E..0xE14A3A;
  - 0xE04ADA..0xE04AE2.

  They are probably expanded after scheduling (Uncertain).

**R11. The ordering policy (Uncertain).**
- In the survey (`sched --strict`), 367 of 2752 load→use adjacent pairs
  and 881 of 4737 other def→use pairs could have been separated by an
  independent instruction of the same block.  Many of these are in the
  R10 units.  So there is no strict "never use the previous result
  next" rule.
- One candidate list rule reproduces the blocks 0xE177A6..0xE177B8 and
  0xE177CC..0xE177DE exactly: top-down, take the first instruction in
  generation order whose inputs are ready and that does not read a
  register written by the instruction just placed; otherwise the first
  ready one.
  - For 0xE177CC it needs `movea.l (8,A6),A2` first in generation
    order.  This is plausible if loop invariants are hoisted to the head
    of the block before the loop (ref p. 6-29), but Uncertain.
  - MST_$VA_TO_SEGNO needs the load of `(0x10,A6)` after the whole
    shift chain in generation order.
- A second candidate, alternating memory loads and other instructions,
  fits PARITY_$CHK_IO, module-local 0xE758C4 and PMAP_$PURGE_WS, but
  not AREA_$FREE_ASID.

**R12. Registers are chosen before scheduling (Uncertain; supported).**
The final order sometimes shows a register taken while a lower one is
free.  It is free only because the scheduler later moved its last use
earlier:
- 0xE177CC: `buffer` is in A2 while A1 is free.  In generation order the
  source pointer (A1) comes first.
- MMAP_$GET_WS_INDEX: the destination slot (A2, 0xE0CA62) is loaded
  before the source temporary (A1, 0xE0CA68).  With R4(a) holding A0,
  this fits only if A1 was allocated first.

#### What dpc must implement to reproduce 0xE17778, in order

| # | Step | Image sites | Support |
|---|---|---|---|
| 1 | Value-based allocation (R1–R3) in place of pinned parameters: the entry loads of 12.2 stay; a parameter's register is released after its last use; each new value takes the lowest register free over its whole live range; no coalescing of copies and operator results | 0xE17780..0xE17788, 0xE177A0, 0xE177A6..0xE177AE, 0xE177D2, 0xE177DA..0xE177E4 | R1–R3 Verified as properties of the output; the order of allocation is not (`colQU` 73%, R1).  **Implemented** (bead domain-pascal-g5q.5, `crates/dpc-m68k/src/regs.rs`): address order of first definition; R2 for register operands and `lea` bases, not for loads through a dying base; M$ARITH calls write only D0/A0; R4(c) for parameters a call's arguments read last, in bodies without branches.  New units: module-local 0xE32B76, PROC1_$SET_ASID |
| 2 | Global CSE of operator results (here the zero-extension), with the two-address result in the non-CSE operand | 0xE177A6, 0xE177B4, 0xE177D0 | Verified at the sites; when a store kills a CSE is open |
| 3 | `max_len & 16#FFFE` as `moveq #-2,D2; and.w D1,D2` (constant into the result register first) | 0xE177A0 | Same order at 0xE60F4A, 0xE60F52 (R8).  **Implemented** (section 8, `&`; bead domain-pascal-d7i) |
| 4 | The 32-bit compare against a constant held in a register | 0xE177AE | Speculative (R8: 11 sites) |
| 5 | D3 at 0xE177C4 and 0xE177D0 | 0xE177C4, 0xE177D0 | Speculative (R6) |
| 6 | The `for` loop as strength-reduced pointers (12.3 item 6) | 0xE177DC..0xE177F2 | Shape Verified; separate bead |
| 7 | A block scheduler run after register choice, with the barriers of R10 | 0xE177A6..0xE177B8, 0xE177CC..0xE177DE | Exists: Verified; policy Uncertain (R11) |

**Consistency with the units `dpc` already reproduces** (review,
2026-10-12; `tests/conformance/manifest.txt`).  Every value in
PROC1_$GET_LOCKS, TIME_$Q_INIT_QUEUE, TIME_$Q_FLUSH_QUEUE, module-local
0xE00EB0, AST_$PAGE_ZERO, PROC1_$GET_LOADAV, PROC2_$GET_BOOT_FLAGS and
NAME_CONVERT_ACL_STATUS is predicted by both `colU` and `colQU`.  Two
points need care:

- PROC1_$TST_LOCK 0xE148DE `sne D1`: D0 (the element, dead after the
  `btst`) is free.  Address-order allocation of a fresh value gives D0.
  `dpc` puts the `sne` in the tested operand's register
  (`set_member_long`), which is what the image shows; keep that as an
  in-place result, not a new allocation.  The order of 0xE148CE..
  0xE148D8 (set address, element, set value) is the generation order
  that `dpc` already uses.  With it, the candidate scheduler of R11
  changes nothing.  Generating the set's load before the element and
  allocating in that order would give the set D0 and the element D1,
  which is wrong.
- TIME_$Q_INIT_QUEUE 0xE16C62..0xE16C6A: R1 gives D0, D1, A0, in either
  order.  The model does *not* explain why the parameters are loaded at
  entry, in parameter order, with the pointer (needed first) last.  Loaded
  where used, they would be memory-to-memory moves with no register at
  all.  The entry-load rule of 12.2 is still needed; R1–R3 replace only
  the pinning.

What must stay green for step 1: `make test` (the `regs.rs` unit tests
and the golden tests, in particular `tests/golden/stores/entry_loads`,
`control/*`, `calls/*`, `data/through_worker`) and `make conform` (all
units of the manifest, the handasm fixtures included).  Step 1 is done
(bead domain-pascal-g5q.5): no golden file and no conformance unit
changed; the units above are reproduced by the allocator as they were by
the pinned registers.  PROC1_$TST_LOCK's `sne` is in place because the
set value and its boolean are one virtual register; the move of a value
into D0 for a function result is not subject to R2 (PROC1_$GET_LOCKS
0xE148F0 loads the result straight into D0).

### 12.5 Local variables held in registers (Verified at the sites; rule Uncertain; implemented narrowly)

Ref 6.4.24, p. 6-29 (`-opt 3`): "Global register allocation allows local
variables to have their values placed in machine registers"; p. 6-32,
"allocate a local variable to a machine register for exactly as long as
the variable's value is needed".  The image shows both kinds of local:

- **In a register, frame slot still allocated:** TIME_$GET_ADJUST
  0xE16AA8, `link.w A6,#-4` and the frame never addressed; the C's one
  local (`time/get_adjust.c`, `current_delta`) is loaded into D1 at
  0xE16AB4 and read from D1 at 0xE16AC4 and 0xE16AD6, across two M$ARITH
  calls (12.4 R3).  PROC1_$SET_VT 0xE149B6 and PROC1_$TS_END_CALLBACK
  0xE14A90: the status word `set_sr` saves is in D3 across calls
  (section 10), and PROC1_$SET_VT's word at 0xE149A4/0xE149AA (`move.w
  #-1,D2` in one arm of an `if`, `move.w (4,A0),D2` in the other) in D2.
  Section 2's untouched frames (306 routines) fit this too.
- **In the frame:** PROC1_$TS_END_CALLBACK's `timeslice`
  (`proc1/ts_end_callback.c`), assigned in both arms of an `if`
  (0xE14AA0 `move.w #-1,(-8,A6)`, 0xE14AF2) and pushed by value from
  memory (0xE14AFE `move.w (-8,A6),-(SP)`), although its address is never
  taken; PROC1_$SET_VT's record at `(-8,A6)` (0xE149BC `clr.l`, 0xE149CC
  `move.w D1,(-4,A6)`), whose address is passed (0xE149D0 `pea`).

Which locals the compiler kept in registers is therefore not just "those
whose address is not taken" (TS_END_CALLBACK's `timeslice` is not, and
D2 at PROC1_$SET_VT is assigned in an `if` too).  Not settled.

**Survey (review of beads domain-pascal-5aw.7, -0wc).**  Every compiled
routine with a `link` (1635, split as `tools/loopalign.py` does) was
scanned for frame slots inside the `link` area with exactly one store,
from a non-constant, outside every backward-branch range, read only after
the store, never `lea`/`pea`'d: 409 slots.  Many are value-parameter
copies, expression temporaries or record fields, so sites were checked
against the C in `../source`:

- **Frame: assigned once at the top level, read as a by-value call
  argument** (each checked in the C): MMAP_$FREE 0xE0CAEC (the spin-lock
  token from ML_$SPIN_LOCK, pushed at 0xE0CB0A), TTY_$K_FLUSH_INPUT
  0xE1C0BE and SIO2681_$SET_BREAK 0xE1D134 (tokens),
  PROC2_$DELIVER_PENDING 0xE3F53E (`current_idx`, a memory load, pushed
  at 0xE3F564), LOG_$SHUTDN 0xE175AE (`saved_ptr`, a memory load, pushed
  at 0xE175CE).  At the instruction level 44 frame slots hold a call
  result (D0/A0 stored right after a `jsr`/`bsr`) and are later pushed
  (MMAP_$FREE_LIST, MMAP_$FREE_REMOVE, MMAP_$ALLOC_FREE, NETBUF_$GETVA,
  NETBUF_$RTNVA, TIME_$Q_ENTER_ELEM, TIME_$Q_REMOVE_ELEM,
  TTY_$K_FLUSH_OUTPUT, ...; not all checked in the C, some may be
  assigned inside an `if`).  NETBUF_$GETVA reuses the value from D0 after
  the store (0xE0EC9A, 0xE0ECAC), as for any frame variable (12.4).
- **Register:** TIME_$GET_ADJUST (D1, read only as M$ARITH operands);
  PROC1_$SET_VT D2/D3 (read by `move.w D2,(72,A2)`, `move.w D3,SR`).
  UID_$GEN 0xE1A018 keeps the C's `token` in D3 (0xE1A040, 0xE1A0B2: two
  assignments, one inside a loop) and pushes it by value (0xE1A07A,
  0xE1A0D6), so "passed by value means frame" is not absolute either (a
  loop-weighted priority would fit; not settled).  Pointer locals are
  often in address registers (MMAP_$FREE's `page` in A2 from 0xE0CADE;
  KBD_$PUT's descriptor in A2 from 0xE1CEC6), which `dpc` does not model:
  a pointer local that is dereferenced stays in the frame.
- Expression temporaries live across a call are spilled to the frame
  (SMD_$BLT_U 0xE6FB1E; module-local 0xE5AA28 at 0xE5AA6C, across an
  M$ARITH call); not this rule's business.

So `dpc` also requires that the local is **never read inside a call
argument**: every checked local assigned once at the top level and
passed by value is in the frame, and that covers PROC1_$TS_END_CALLBACK's
`timeslice` too.  No checked site contradicts the narrowed rule, but it
is still fitted to two routines (Uncertain).  Locals outside it stay in
the frame, which UID_$GEN shows is not always the original's choice.

**Implemented** (`dpc-m68k` `isel.rs`, `register_locals`; bead
domain-pascal-0wc): the narrowest rule that fits TIME_$GET_ADJUST.  A
scalar local (1, 2 or 4 bytes) whose only references are whole reads and
assignments of the variable, assigned exactly once, by a statement of
the routine's own statement part (not inside an `if`, loop or `case`),
with a value that is not a constant, read only by later statements, and
never read inside an argument of a call (survey above),
is a data value: the assignment computes or loads it into a new virtual
register (`move.l (abs).l,Dn`, or the `move.w SR,Dn` of `set_sr`), its
reads use that register, and the allocator gives it a register like any
other value (12.4 R1-R3; D1 at 0xE16AB4).  Its frame slot stays
allocated (`dpc-sema` frame size, `link #-4`).  Every other local stays
in its frame slot, as before, and so does every local of a routine with
nested routines (they reach its locals through the frame, which the
per-body analysis does not see; `dpc-ir` `scalar_locals`).  A value held
across a call goes to a callee-saved register (12.4).  A constant assignment is excluded because
the one register-variable constant seen, 0xE149A4, is `move.w #-1,D2`,
not the `moveq` `dpc` uses for constants in registers (section 13).
Unit test `register_local_rule`; golden `tests/golden/sysprog/interrupts`
(`held_sr`); conformance `tests/conformance/time_get_adjust.pas`.

**Pointer locals of loops in address registers (implemented narrowly;
bead domain-pascal-5aw.15).**  A list walk keeps its pointer in an
address register: the DISK_ module-local 0xE3CCCE `movea.l D0,A1` (the
pointer from the parameter loaded at entry), then in its `while` loop
`movea.l (A1),A1` (`r := r^.next`) and the test `cmpa.w #0,A1`; its
`link #-8` area is never addressed.  Of the `while` loops that test a
pointer against `nil` at the bottom, 45 test an address register and 12
a frame slot (section 7.4).  `dpc` (`isel.rs`, `loop_pointer_locals`)
holds a 4-byte pointer local in one virtual address register for the
whole body when its only references are whole reads, dereferences and
whole assignments, the first of them an assignment of a non-constant
value by a statement of the routine's own statement part, another
assignment inside a loop, and the body makes no call (fitted to
0xE3CCCE; **Uncertain**).  Every assignment writes that register, a load
straight into it.  Other pointer locals stay in the frame.  Unit test
`loop_pointer_local_rule`; golden `tests/golden/loops/while` (`whl_$walk`);
conformance `tests/conformance/disk_set_chain_status.pas`.

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
- `addi`/`subi`/`andi`/`ori`/`eori` (`0640`, `0480`, `0240`, `0040`,
  `0a40` for words) are written with the explicit `i` mnemonics, which gas
  encodes as written (no `addq` substitution).  Byte immediates are 0..255
  (`andi.b #0x80`, the high byte of the extension word zero, as for
  `move.b`).
- Indexed operands `(d,An,Dn.w)` / `(d,An,Dn.l)` use the brief extension
  word (one word: index register, size, 8-bit displacement; no scale on
  the 68010): 0xE70DA8 `4a75 00fe` `tst.w (-2,A5,D0.w)`.  `lea
  (0,An,Dn.w),Am` (0xE3BB6E `41f5 1000`) likewise.
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
