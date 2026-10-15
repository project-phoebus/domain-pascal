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

### 0.1 Apollo insert files (primary evidence for declarations, not for codegen)

The SR10.2 install tree has Apollo's **customer** insert files:
`~/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2/sys/ins/*.ins.pas`
(56 Pascal inserts; read-only, like the image).  They are genuine
Apollo files: dated change histories (`base.ins.pas`: "BASE.INS.PAS,
/sys/ins, ceb-jrw, 01/02/89", changes from 06/30/81 to 01/02/89;
`prsvr.ins.pas`: "THE FOLLOWING PROGRAMS ARE THE SOLE PROPERTY OF APOLLO
COMPUTER INC.").

- **Use them for declarations:** type layouts, field names, constant
  values and routine headings of the public interfaces, e.g. in
  `base.ins.pas` `status_$t` (section 9), `uid_$t = PACKED RECORD high:
  integer32; low: integer32; END`, `binteger = 0..255`, `pinteger =
  0..65535`, `linteger = 0..2147483647`, `time_$clock_t` (a `[aligned(1)]
  PACKED RECORD` with variants `high`/`low` and `high16`/`low32`, under
  `%PUSH_ALIGNMENT; %WORD_ALIGNMENT;` for SR10), and `ec2_$eventcount_t`
  (`[volatile] RECORD value: [aligned(1)] integer32; awaiters:
  pinteger; END` for SR10 68k).  They also show Apollo's own use of the
  language: routine options on routine pointer types
  (`prsvr.ins.pas`, `^FUNCTION (...) : db_ptr ;options(d0_return,val_param,extern);`),
  the conditional variables `isp_$m68k` / `obj_$coff` (predeclared by
  the m68k target, `dpc_m68k::M68k`'s `Target::predeclared_cond_vars`),
  include guards
  (`docs/reference-clarifications.md` Q2, Q4, Q19, Q21).
- **They are not the kernel's own inserts.**  The kernel was compiled
  against Apollo's internal `.ins.pas` files, which may differ, and the
  customer files declare only the public interfaces.  An image
  observation overrides an insert file.
- **They say nothing about code generation.**  Codegen claims still need
  image addresses.
- `../source`'s reconstructed inserts (`domain_os/**/*.ins.pas`) follow
  the customer files where they say so in their comments; they are
  agent-written, not Apollo files.

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

#### Value parameters passed by address: the callee's copy (survey Verified; frame position Uncertain; implemented; bead domain-pascal-5aw.12)

Ref 5.2 Table 5-1 (p. 5-5), "no mode", default convention: the caller
"Passes address", the callee "Creates local copies of arguments and makes
direct references to them"; `in` parameters are not copied ("Makes
indirect references", and the callee may not assign them, 5.3.2 p. 5-7).

Survey (`tools/framesurvey.py copies`: every compiled routine of
`tools/nestsurvey.py`'s split; a "slot copy" is `movea.l (d,A6),An`, d >=
8, followed by moves from `(An)` into the frame):

- **The copy sequences, by size** (all slot copies anywhere in a
  routine, Verified counts):
  - 1, 2, 4 bytes: one `move.<s> (An),(k,A6)` (3, 40 and 30 sites;
    FILE_$INVALIDATE 0xE75186 `move.b`, SMD_$SET_TP_CURSOR 0xE6E97E
    `move.w`, 0xE6E976 `move.l`).  A few take a field, `(d,An)` (1, 5,
    3 sites): statements reading a record field, not copies.
  - 6 bytes: `move.l (An),(k,A6); move.w (4,An),(k+4,A6)` (all 6 sites:
    TONE_$TIME 0xE17304, FILE_$SET_DTM_F 0xE5E208, FILE_$SET_DTU_F,
    0xE05220, 0xE1652E, 0xE5AA2C).
  - 8 bytes: `move.l (An)+,(k,A6); move.l (An)+,(k+4,A6)` (all 124 sites,
    none with displacements; DIR_$GET_ENTRYU 0xE4D512..0xE4D51A, the
    PROC2_, XPD_, FILE_, ACL_ UID parameters).
  - 10 bytes and more: the block copy of 5.2 from the slot, `movea.l
    (d,A6),A0; lea (k,A6),A1; move.l (A0)+,(A1)+ ...` (XPD_$SET_PTRACE_OPTS
    0xE5AFB0..0xE5AFBE, 14 bytes), a `dbf` loop from 20 bytes (OS_$INIT
    0xE33808, 36 bytes).
- **When (Verified at the sites):** at entry, after the register save,
  the A5 load and the entry loads of the other parameters (12.2):
  DIR_$GET_ENTRYU 0xE4D508..0xE4D51A `lea abs.l,A5; movea.l (0x18,A6),A2;
  movea.l (8,A6),A0; move.l (A0)+,(-8,A6); move.l (A0)+,(-4,A6)`;
  AST_$SET_DTS 0xE05540 (`move.w (8,A6),D3; movea.l (0x12,A6),A3`, then
  the copy of `(0xa,A6)`, the second parameter, after the third);
  DIR_$OLD_GET_ENTRYU 0xE58064..0xE5807C (four entry loads, then the
  copy of the first parameter).  Then **in parameter order**
  (FILE_$INVALIDATE 0xE75166..0xE75186, four copies; SMD_$SET_TP_CURSOR).
- **Registers (Verified by survey):** each copy's slot goes to the lowest
  address register that no entry load holds and no earlier copy left a
  value in; one used up by `(An)+` counts as free (12.4 R4(b)):
  FILE_$INVALIDATE A0, A0 (after the 8-byte `(A0)+`), A1, A2 (saved for
  this alone).  133 of the 135 copies before a routine's first branch or
  call fit; the other two (DISK_$SORT 0xE3C3F0, PROC2_$SIGNAL_OS
  0xE3F0B8) copy through a register loaded at entry, the slot used again
  later.  The next statement's slot loads continue the rule:
  SMD_$SET_TP_CURSOR 0xE6E98C `movea.l (8,A6),A2` after copies through
  A0 and A1, in a one-statement body; DIR_$GET_ENTRYU 0xE4D51E `movea.l
  (0x10,A6),A0` after the copy's `(A0)+`.
- **Unmodified parameters are copied** (Verified, as far as the source
  can be inferred): the UID parameters above are never assigned (the C
  reconstructions only read them), yet copied and then passed on as the
  copy (DIR_$GET_ENTRYU 0xE4D55E `pea (-8,A6)` to DIR_$OLD_GET_ENTRYU;
  TONE_$TIME 0xE1732C `pea (-8,A6)` to TIME_$WAIT), or pushed by value
  from the copy (SMD_$SET_TP_CURSOR 0xE6E984, 0xE6E988; FILE_$INVALIDATE
  0xE751B4..0xE751BC).  A parameter read through its slot without a copy
  (SMD_$SET_TP_CURSOR's first, 0xE6E98C; DIR_$GET_ENTRYU's name and
  length, passed on with `move.l (d,A6),-(SP)`) is taken to be `in`.
  Whether an *unused* value parameter is copied cannot be seen (an empty
  procedure is a bare `rts`, section 2).
- **Held in a register instead (Uncertain):** 189 by-address slots are
  dereferenced into a data register before the first branch or call
  (`movea.l (8,A6),A0; move.w (A0),D2`: FIM_$FREE_ASID 0xE0AA6C,
  VOLX_$MOUNT 0xE6B118 four words into D4..D7), with no frame copy.
  These may be `in` parameters read once at entry, or value parameters
  whose copy lives in a register (as a local can, 12.5); the image cannot
  tell.  `dpc` copies every value parameter into the frame.
- **Frame position (Uncertain):** of the 8-byte entry copies in
  map-named routines, 29 sit at the top of the frame (`(-8,A6)`, nothing
  above: DIR_$GET_ENTRYU `link #-8`; PROC2_$DEBUG, XPD_$READ_PROC with
  their status local at -12 below), 34 below other frame objects
  (FILE_$LOCATE 0xE60620 at -40, FILE_$READ_LOCK_ENTRY 0xE608EC at the
  bottom, -48, below a 26-byte local), 1 below nothing it addresses
  (MST_$UNMAP_PRIVI).  Several copies are not in parameter order in the
  frame: FILE_$INVALIDATE has the UID at -8, its status local at -12,
  then the third parameter at -16, the second at -20, the fourth (a
  byte) at -22; MST_$GET_UID_ASID 0xE44010 copies its word to -4 with
  locals at -2, -6 and -8; XPD_$SET_DEBUGGER 0xE5BBD8 puts its first
  parameter's copy (-16) below its second's (-8).  No order fits every
  site (as for the result variable, section 2).  `dpc` allocates the
  copies first, from the top, in parameter order, before the locals;
  this fits the top sites and SMD_$SET_TP_CURSOR (-4 then -6), not the
  others.

**Implemented** (`dpc-sema` `Param::copy_offset`; `dpc-ir`
`Function::param_copies`, `var_addr`; `dpc-m68k` `isel.rs`,
`Selector::param_copies`): the body reads, writes and passes on the copy
(an assignment to a value parameter never reaches the caller's object;
`addr` and passing it by address name the copy); the copies are emitted
after the entry loads in the forms above, each slot register kept apart
from the earlier ones by 12.4 R4(b), and a body after copies takes fresh
registers for its slot loads as a body of several statements does.
Refused: a value parameter the body never uses (nor any nested routine),
in an empty procedure too; sizes other than 1, 2, 4, 6, 8 or an even 10
and more (a 3-byte character array); a block copy whose registers come
out other than A0 to A1, i.e. after a copy that left its register or with
an entry load in A0 or A1 (the 31 slot block copies seen are A0 to A1
where nothing holds them, XPD_$SET_PTRACE_OPTS 0xE5AFB0, OS_$INIT
0xE33808; the three others are in nested routines or after a branch,
0xE4626E, 0xE5B192, FILE_$OLD_AP 0xE5E132); copies in a nested routine
(none catalogued).  Golden `tests/golden/params/value_copies`; driver tests
`crates/dpc-m68k/tests/params.rs`; byte-identical in `make conform`:
TONE_$TIME (`tone_time.pas`, the whole TONE module, a 6-byte copy) and
SMD_$SET_TP_CURSOR (`smd_set_tp_cursor.pas`, two copies and A2 for the
`in` slot), PROC2_$PGUID_TO_UPGID (`proc2_pguid_to_upgid.pas`, an 8-byte
copy; its slots after ML_$UNLOCK in A0 and A1, 12.4 R4(b)).  Blocked: DIR_$GET_ENTRYU (section 6), MST_$GET_UID_ASID (its
frame order), SET_LITES_LOC 0xE0C4DC (its copy reloaded into A1 for a `nil` test and
reused, `movea.l (-4,A6),A1; cmpa.w #0,A1; ...; move.l A1,(A5)`: a
pointer local's register and reuse not modelled), TERM_$WRITE 0xE668D8
(a constant cell of an earlier routine, `pea (-0x5e,PC)` to 0xE66898).

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
  0xE135D2); no `move.w #0,-(SP)`.  The 19 `move.l #0,-(SP)` are all
  arguments of EC_$WAIT 0xE20610 (`tools/nilsurvey.py push`: 0xE0F79E,
  0xE13B32, ...), its unused eventcount pointers, and **not** a zero:
  every one of their immediates is a cell of the binder's RELOC table
  (`tools/nilsurvey.py` reads it; ../source docs/rfc-cold-start.md step
  8), so each is the address of a symbol at 0, pushed as `move.l
  #sym,-(SP)` like any other unit's variable ("Address of another unit's
  variable" below).  The map has two symbols at 0, EC_$NIL_EC (segment
  ABSOLUTE, MARKED) and TRAP_PAGE; an eventcount argument makes it
  EC_$NIL_EC, passed as TIME_$CLOCKH_EC is beside it (FLP's EXCS
  0xE3E2A0 `clr.l` for the integer 0, 0xE3E2B0 `move.l #0` [RELOC],
  0xE3E2B6 `move.l #0xe2b0d4`).  The ../source translations write `nil`
  there (dxm/helpers.pas, dbuf/get_block.pas, flp/excs.pas,
  pkt/ping_server.pas); that is not the original's source.  Likewise 15
  of the 17 `movea.l #0,An` are RELOC cells (`tools/nilsurvey.py
  areg`), among them DXM_$HELPER_COMMON 0xE171F2 and PKT_$PING_SERVER
  0xE12C14, which load the address into A2 before a loop and push it
  twice with `pea (A2)` (DBUF_$GET_BLOCK 0xE3A88E pushes it twice with
  `move.l #0,-(SP); move.l (SP),-(SP)`).  `nil` itself pushed is `clr.l
  -(SP)`: the port pointer of the module-local NET_IO_$PUT_IN_SOCK_COMMON
  at NET_IO_$PUT_IN_SOCK 0xE0E4CC and NET_IO_$PUT_IN_SOCK_INT 0xE0E506
  (the C passes NULL), the only `nil` push known; `dpc` pushes `nil`
  so, as any longword 0 (bead domain-pascal-73sx; golden `calls/nil_push`,
  exec test `nil_push`; Verified: ../source net_io/put_in_sock.pas's two
  routines compile byte-identical, srcsurvey OK), and refuses a `nil` next to another constant
  argument (no site: at both sites its neighbours are parameters).
  `dpc` uses `clr.l` for the integer 0 (Verified at FLP's EXCS).
- **Bytes:** `st -(SP)` for true (196 sites, 0xE1365C).  `false` (a
  byte 0) clears the whole slot with `clr.w -(SP)`: RIP_$SEND_UPDATES
  pushes `st -(SP)` on one path (0xE6889E) and `clr.w -(SP)` on the other
  (0xE688BC) for the same byte parameter of RIP_$BROADCAST, which reads
  it with `move.b (8,A6),D2` (0xE872A6) (Verified at this site;
  implemented).  There is no `clr.b -(SP)`, no `sf -(SP)` and no
  `move.b #n,-(SP)` at all.  **Other byte constants are word pushes
  with the byte in the high half, `move.w #n<<8,-(SP)` (Verified;
  implemented, bead domain-pascal-g5xv).**  Survey (ad-hoc script; each
  push's call followed to its callee, the slot's first read checked):
  1422 `move.w #n,-(SP)` in compiled code, 54 of them with a zero low
  byte.  None of the other 1368 (1292 of them followed) is read as a byte
  (`move.b (d,A6)`, or `pea (d,A6)` of the slot).  Of the 54, 26 go to
  byte parameters, all characters:
  - TTY's tty_$i_buf_put (module-local 0xE1AF42, reads `move.b
    (8,A6),D2` at 0xE1AF4A): 0xE1AFC0 `move.w #-0x200` (chr(254), the
    delay escape; `../source/domain_os/tty/i_put_chars.pas`), 0xE1B0C4
    #0x0800, 0xE1B10C/0xE1B1AE #0x0a00, 0xE1B130/0xE1B17A #0x0d00,
    0xE1B20A #0x2000;
  - module-local 0xE1B3B4 (takes `pea (12,A6)` of its byte): 0xE1B608,
    0xE1B624, 0xE1BB18, 0xE1BB26, 0xE1BB5E, 0xE1BB6C #0x0800, 0xE1B616
    #0x2000, 0xE1B646 #0x5c00, 0xE1B660 #0x5e00, 0xE1BC92 #0x2f00, and
    0xE1B67E #0x3f00, which joins a path pushing `move.b D2,-(SP)`
    (0xE1B686) for the same call;
  - TTY_$I_RCV 0xE1B92A (`move.b (12,A6),D3`): 0xE1BBDE #0x0a00
    (0xE1BBBC #0x0d00 reaches the same call by a branch);
  - SIO2681_$XMIT 0xE1D4FC (`move.b (12,A6),(7,A2)`): SIO2681_$INT
    0xE1CFDA, 0xE1D036 #0x2000;
  - TTY_$I_ENABLE_CRASH_FUNC 0xE67292 (`move.b (12,A6),D0`): SIO_$INIT 0xE32E62,
    TERM_$INIT 0xE331F4 #0x1b00;
  - SIO_$I_TSTART 0xE1C7D6 #0x1300 and 0xE1C7F8 #0x1100 (XOFF / XON
    through the driver's routine pointer, `jsr (A1)`;
    `../source/domain_os/sio/i_tstart.c`).

  The other 28 zero-low-byte word pushes go to word parameters (M$OIS$WLW
  0xE0AD54 #0x100 ×7 and #0x400 ×1; module-local 0xE47B78 `move.w (12,A6),D2` #0x400;
  0xE15D8C, 0xE15E62, 0xE15F14, 0xE17828, 0xE4A4C8, 0xE60FD8, 0xE1501A).
  No word push whose low byte is nonzero is read as a byte.  chr(0) is
  `clr.w` like `false` (0xE1AFD4, to tty_$i_buf_put); 111 of the 472
  `clr.w -(SP)` (391 followed) reach a byte slot.  **Not settled (bead domain-pascal-3f6h),
  `dpc` refuses:** a byte with all bits set other than `true` (chr(255),
  a signed -1): `move.w #-0x100` is not in compiled code and `st` sites
  cannot be told from `true` without the callee's type; and a nonzero
  byte constant next to a word or byte constant: none of the 26 has one
  (a pad, a pointer, `st` or a non-constant is next to each; SIO_$INIT
  0xE32E60 `st -(SP)` then 0xE32E62 `move.w #0x1b00` stand unmerged, as
  `true` does next to `clr.w`), so whether it merges like `false` is
  unknown.  Constants passed by address (default convention) go to the
  code-section pool (below) and are unaffected.
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
  - `false` (a byte 0, pushed as `clr.w`, below) next to a word constant
    merges like a word 0 (Verified at these sites): FILE_$CHECK_PROT's
    boolean `ignore_super` (passed on with `pea (0x12,A6)` at 0xE5D226,
    read by ACL_$RIGHTS as a byte, `move.b (A3),-(SP)` at 0xE46A54) and
    the word after it are one `clr.l -(SP)` at both call sites (0xE73FFC
    FILE_$TRUNCATE_D, 0xE43DAC).  Also (bead domain-pascal-g5xv
    survey; callee reads the low half's slot as a byte): `move.l
    #0x10000,-(SP)` at AST_$PMAP_ASSOC 0xE04404 and AST_$ASSOC_AREA
    0xE0466C (callee MMAP_$INSTALL_LIST 0xE0CD80 `move.b (14,A6),D0`),
    MMAP_$IMPURE_TRANSFER 0xE0CC16 (0xE0C514 `move.b (18,A6),D2`),
    DISK_$FORMAT_WHOLE 0xE3D2FA and DISK_$FORMAT 0xE3D416 (0xE3BE8A
    `move.b (10,A6),D3`); RIP_$HALT_ROUTER 0xE873C8 `move.l #0x80000`
    (RIP_$SEND 0xE871B6); ACL_$CONVERT_FROM_9ACL 0xE48FB6 `move.l #0x4000000`
    (0xE47B78 `move.b (14,A6),D3`); PROC2_$CREATE 0xE7281A `move.l
    #0x70000` (MST_$MAP_INITIAL_AREA 0xE42E9E `move.b (24,A6),D4`);
    PMAP_$WS_SCAN_CALLBACK 0xE145C6 `move.l #0x50000` (MMAP_$WS_SCAN
    0xE0D364 `tst.b (10,A6)`); `move.l #0x40000` at DIR_$SET_PROTECTION
    0xE5232A, DIR_$SET_ACL 0xE52D10, DIR_$OLD_FIX_DIR 0xE55D04,
    DIR_$OLD_DROP_HARD_LINKU 0xE56D30, PACCT_$START 0xE5A846 and
    AUDIT_$INHERIT_AUDIT 0xE71790 (FILE_$PRIV_LOCK 0xE5F0EE `tst.b
    (18,A6)`): a word constant and a byte 0 after it, one longword.  The
    other order as `pea (n).w`: ROUTE_$SEND_USER_PORT 0xE87CE8 `pea
    (0x2).w`, a byte 0 then the word 2 (SOCK_$PUT 0xE1614E `move.b
    (14,A6),-(SP)`, `move.w (16,A6),-(SP)`).  No merged longword
    (`move.l #n` or `pea (n).w`) has a nonzero byte half (re-surveyed in
    review: every constant longword push followed to a callee reading
    one of its halves only as a byte).  Conformance unit
    `tests/conformance/file_truncate_d.pas`.  `st -(SP)` for `true`
    stands unmerged next to `clr.w -(SP)` in 33 adjacent pairs
    (0xE05BFC; whether those `clr.w` are word zeros or `false` is not
    known); `dpc` pushes `true` apart.
  - **Uncertain:** which pair merges in a run of three word constants
    (`dpc` rejects it).
- **Address of another unit's variable** (by address under the default
  convention, or `addr(x)` by value): `move.l #sym,-(SP)`, 374 sites, 362
  of them a map symbol's own address (UID_$NIL ×36 e.g. 0xE01824,
  TIME_$CLOCKH_EC ×24, DIR_$MUTEX ×22; 0xE00EB8 `move.l #AST_$ZERO_BUFF`).
  An address *inside* another unit's object is pushed both ways: 12
  `move.l #` (0xE32398 DIR_$MUTEX+0x14) and all 24 `pea abs.l` (0xE0F5DC
  ROUTE_$PORT+0x30, 0xE0A36C DMA+0xC0): Uncertain, rejected by `dpc`.
  The same address into a register is `movea.l #sym,An` (0xE00ECC;
  DISK_$GET_ERROR_INFO 0xE6C090).
- **`addr(x)` by value of a frame or unit-data object** (a `univ_ptr`
  or pointer value parameter, `val_param`): `pea (d,A6)` / `pea (d,A5)`,
  the same push as the object by address (no compiled site can be told
  from that one; implemented, bead domain-pascal-5aw.19; `addr` stored
  as a value: section 5.2).
- **A by-address parameter passed on by address** is its slot, pushed
  with `move.l (d,A6),-(SP)` (Verified: LOG_$READ 0xE1780C `actual_len`,
  0xE17818 `buffer`, both passed on to the worker 0xE17778).
- **An object reached through a pointer held in an address register**
  (a slot or pointer parameter loaded at entry, 12.2) is pushed with `pea
  (d,An)` / `pea (An)` (Verified: module-local 0xE32B76,
  `sio/init_dtte.c`, 0xE32B86 `pea (0xc,A2)`, 0xE32B92 `pea (0x18,A2)`,
  0xE32B9E `pea (A2)`; also `pea (A2)` at 0xE1BEB2, 0xE1D59E, 0xE3DB8C).
  Implemented; conformance unit `tests/conformance/sio_init_dtte.pas`.
- **The target of a pointer field of a record held in an address
  register, `p.f^`** (Verified; bead domain-pascal-s9j): the pointer
  field itself is pushed, `move.l (d,An),-(SP)`: 0xE1CC52 `move.l
  (0x48,A2),-(SP)` in KBD_$RCV's nested 0xE1CC10 (A2 its by-address
  parameter's slot (0xa,A6)) for `kbd.key_ec^` to
  EC_$ADVANCE_WITHOUT_DISPATCH (a `var` parameter, `ec/ec.ins.pas`);
  TTY_$I_OUTPUT_BUFFER_DRAINED 0xE1B3A4, TTY_$I_DELETE_CHAR 0xE1B588,
  TTY_$I_FLUSH_INPUT 0xE1B7DA `move.l (0x2a8,A2),-(SP)`,
  TTY_$I_BREAK_CHAR 0xE1B8EC `move.l (0x2a4,A2),-(SP)` (to the TTY
  module-local 0xE1AEF8; that callee could also take a pointer by value,
  `tty/i_advance_ec.pas`, the same bytes).  Not seen with the record's
  pointer not held: TIME_$ADVANCE_CALLBACK 0xE16440 `movea.l (8,A1),A2;
  pea (A2)` is a chain through a slot into a callee-saved register, the
  source's shape unknown (a local or a `with`?); `dpc` refuses `p.f^`
  unless the record is held, and `q^.f^` through anything else (bead
  domain-pascal-royb).
- **An object through a pointer the push loads** (Verified at the sites;
  bead domain-pascal-s9j): `movea.l <pointer>,An; pea (d,An)`.  Ad-hoc
  survey of the compiled routines with a `link` (script not kept):
  `movea.l` then a `pea` through the loaded register, 34 with a
  parameter slot `(d,A6)`, d > 0, and a displacement (KBD_$OUTPUT_
  BUFFER_DRAINED 0xE1CE9E `movea.l (8,A6),A0; pea (0x4c,A0)`;
  TTY_$I_PUT_CHARS 0xE1B0BC..0xE1B2B6, nine `movea.l (8,A6),A0; pea
  (0x3d2,A0)`, the slot reloaded for each call); 25 with a frame
  slot, d < 0, a pointer local (AST_$PURIFY 0xE0599E `pea (0x28,A0)`);
  35 + 11 through another address register (the enclosing routine's slot from a
  nested routine, 0xE1AFB8, 0xE1AFCC, 0xE1AFDE, 0xE1AFF2 `movea.l
  (8,A2),A0; pea (0x3d2,A0)` in TTY_$I_PUT_CHARS' nested 0xE1AFA2, also
  0xE1BD16, 0xE1BDF0; a pointer in the unit's data, AUDIT_$LOG_EVENT_S
  0xE70E88 `movea.l (0x198,A5),A0; pea (0xc,A0)`, 13 such in AUDIT;
  the rest chains and array elements).  The register is the
  allocator's, as for any pointer loaded as a base (12.4: A0, A1 and
  A2..A4 all occur; kept apart within an argument list, 0xE6154A A0 then
  0xE61552 A1).  **Uncertain** there: a pointer local reloaded into the register
  that already held it from an earlier statement (A1 at 0xE39D00,
  0xE53AF4, 0xE74484) where `dpc` takes the lowest free one (bead
  domain-pascal-0os2).
  **Not modelled, refused:** one argument list reading the same loaded
  pointer twice: the original loads it once (0xE74706 `movea.l
  (-0xbc,A6),A1; pea (-0x38,A1); pea (-0x34,A1)`, 0xE3CA4E, 0xE748A4; bead domain-pascal-ux1w);
  the target `p^` of a pointer in the unit's data or of another unit's
  pointer (no site told apart from a pointer pushed by value; bead
  domain-pascal-royb).
  Implemented (`isel.rs`, `Selector::push`, `reloaded_pointer`,
  `field_pointer`); golden `calls/pointer_targets`, exec test
  `pointer_targets`.
- **A pointer local's own target `p^`** (Verified; review of bead
  domain-pascal-s9j, ad-hoc survey of the compiled routines with a
  `link`, script not kept): the pointer is pushed, `move.l
  (d,A6),-(SP)`, as for a parameter slot (LOG_$READ above).  By-address
  sites, each a uid (8 bytes, by address in every convention, ref 5.2)
  at the start of the record the local points to: module-local 0xE53A18
  (`dir/do_op_fix_dir.c`, `orig_handle` at (-0x180,A6)) 0xE53DB0,
  0xE53DEE, 0xE53E26 to FILE_$FW_FILE, 0xE53D32, 0xE53DD6 to
  FILE_$TRUNCATE (whose length argument is a constant passed by
  address, `pea (d,PC)`: default convention).  The image has 7 `movea.l
  (-k,A6),An; pea (An)` (AST_$RESERVE 0xE0680E, 0xE39D00, 0xE4AA60,
  0xE535F8, 0xE53AF4, 0xE59EE6, 0xE87476); all go to routines taking
  other arguments by value (0xE0250C, DBUF_$SET_BUFF, OS_$DATA_COPY,
  module-local 0xE4B340, EC_$WAIT's by-value arrays), where the same
  argument position also gets `move.l (-k,A6),-(SP)` from a pointer
  local (DBUF_$SET_BUFF 11 times, e.g. 0xE39E04 in the routine of
  0xE39D00; 0xE4B340 11, e.g. 0xE53A60 for the same local as 0xE53AF4;
  OS_$DATA_COPY 10; EC_$WAIT 3), so the pointer is most likely passed
  by value there.  In 6 of the 7 the same register was set to the
  pointer earlier in the routine (A0 at AST_$RESERVE 0xE067DA, A1 at
  0xE39C68, 0xE4AA3C, 0xE535C6, 0xE53AD4, 0xE59E28); not 0xE87476.
  `dpc` pushes `move.l` (by address or by value); the `pea (An)` form
  is **Uncertain**, its trigger unknown, not modelled (bead
  domain-pascal-mrsu; the register choice, bead domain-pascal-0os2).
- **Implemented** (`isel.rs`, `Selector::push`, `push_long_const`): all
  the forms above that are Verified; `addr(x)` (ref "Addr", p. 4-13) in
  `dpc-sema`.  Golden `tests/golden/data/through_worker.pas` (LOG_$READ's
  instruction sequence).
- **Constant by address (Verified by survey; implemented; bead
  domain-pascal-5aw.9).**  This is the default convention for a constant
  actual (ref 5.2 Table 5-1: value and `in` parameters are passed by
  address unless `val_param`/`internal`).  The compiler puts the constant
  in a **cell in the code section** and passes `pea (d,PC)` (`487a dddd`).
  Survey: `tools/pcsurvey.py` (compiled code 0xE00800..0xE88834, routines
  split and followed as in `tools/a5survey.py`).
  - **Forms.**  1047 `(d,PC)` operands outside `case` tables: 1025 `pea`,
    22 `lea (d,PC),An` (13 + 6 + 2 + 1 for A3/A0/A1/A2: set constants
    tested with `btst Dn,(0,An,Dm.w)`, OLD_DIR UNMAP_CASE 0xE54170, and
    record copies, PROC1_$BIND 0xE14DAE; not implemented).  9 `pea`
    name a routine's address, not a constant (DIR 0xE4C310 `pea` of the
    routine at 0xE4C9E4: a procedure parameter).
  - **Placement: after the routine that uses the constant first.**  Of
    the 1038 constant sites, 733 name a cell right after their own
    routine's code (before the next routine starts), 305 a cell after an
    *earlier* routine's code, none a later one.  So the cells are laid
    out per routine, in routine (emission) order, after the routine's
    `rts`; the last routine's cells end the module's code and the
    module-end padding follows them (TERM_$INIT 0xE3321E `0002 0001
    2048`; MMAP_$RECLAIM 0xE0D9E6 `0006 000d 0000` in non-aligning code).
    The four "final `0x0000` words" of section 7 that are cells (NET_IO
    0xE0E8C2, OS_TERM 0xE1AB26, WIN_ 0xE303D6, DIR_SERVER 0xE5839E) are
    such last cells.  A map routine's extent therefore includes its
    cells (MST_$REMAP 0xE439D4, size 0x38 = 0x36 of code + the cell
    0xE43A0A).
  - **Sharing.**  A later routine names an earlier routine's cell for an
    equal constant (305 sites; SMD_$INVERT_S 0xE6DDE4, SMD_$INQ_KBD_TYPE
    0xE6E142 and ten more name SMD_$ASSOC's `0000` at 0xE6D92C; CRASH_SYSTEM
    status cells, e.g. AST_ 0xE00EAC from five routines).  Sharing is
    module-wide (no distance limit seen: DIR 0xE52F3E names a cell 0x7C02
    bytes back).  But 25 cells repeat the bytes of an earlier cell of
    their module: 11 inside one routine's own cells, all `0000`
    (TONE_$TIME 0xE1735C/0xE1735E), 14 across routines (MST_$REMAP
    0xE43A0A and MST_$SET_TOUCH_AHEAD_CNT 0xE4472C; SMD `0001` at
    0xE6D92A and 0xE6DFF8, each named again by later routines).  A cell's
    identity is therefore more than its bytes; the type of the constant is
    the natural candidate (a word `0000` can be an `integer` 0, `false`
    as a byte in its word, an enumeration's first value) but is
    **Uncertain**.  `dpc` shares a cell when the bytes and the type agree
    (the parameter's type; for a `univ` parameter, below).
  - **Order within a routine's new cells (Verified by survey: 110 of 110
    routines with two or more; 157 have one).**  Two groups: cells whose
    size is 2 mod 4 first, then cells of a multiple of 4.  In each group
    the cell used first (in code order) comes first, then the others in
    *reverse* order of first use.  TIMER_$INIT 0xE16340 uses five word
    cells in the order 0xE1639C, A4, A2, A0, 9E and lays them out 9C, 9E,
    A0, A2, A4; LOG_$SHUTDN 0xE17590 uses the longword 0 (0xE17608)
    before the word 4 (0xE17604); DIR_$OLD_FIX_DIR 0xE56094 has three
    words then three longwords, each group in this order.  Strings fall in
    the group of their size (OS_$INIT 0xE347CA: 31 cells).  (A cell's
    size is read off the spacing of the targets: `tools/pcsurvey.py
    order`; one fit needs the last cell's size guessed.)
  - **Alignment.**  A byte constant takes the high byte of a word (`ff00`,
    17 sites naming an earlier `ff00` cell).  The second group (sizes a
    multiple of 4) starts 4-aligned in routines that align their loop
    heads (section 7), after a `0x2048` filler when needed: 45 fillers,
    all in routines the loop survey classes aligning or cannot class
    (LOG_$SHUTDN 0xE17606, PROC2_$QUIT 0xE3F15A, SYSBUS_$DEFINE_INT
    0xE0ABCE); 102 such groups are 4-aligned by themselves.  15 start at
    2 mod 4: 13 in routines classed non-aligning (ACL_$SET_ACL_CHECK
    0xE4787A, the OLD_DIR and DIR ones), one unclassed (0xE13272), one
    (0xE74A6A) in the routine at 0xE743CE, the last of PROC2, whose
    module ends with the zero-word pad of non-aligning code.  So the
    loop-alignment switch also governs the cells (`tools/pcsurvey.py
    align`).
  - **The same cell twice in a row** is pushed the second time with
    `move.l (SP),-(SP)` (`2f17`): TERM_$SEND_KBD_STRING 0xE1AB0C `pea
    (0x18,PC)`, 0xE1AB10 `move.l (SP),-(SP)`; all 33 `pea (d,PC)`
    followed by an argument naming the same cell, none pushed with a
    second `pea` (`tools/pcsurvey.py dups`).  Other addresses repeated
    are pushed both ways (`pea (An)` twice ×10, `pea (d,A6)` then `move.l
    (SP),-(SP)` ×7), not implemented.
  - **Implemented** (`dpc-ir` `pool_const`, `Addr::Const`; `dpc-m68k`
    `Selector::push`, `Selector::call`, `emit.rs` `Pool::place`):
    scalar constants of 1, 2 or 4 bytes (integers, enumerations,
    booleans, characters) by address; cells laid out as above, labels
    `.Lc<n>`, `pea (.Lc<n>,%pc)`; the branch sizer counts the cells.
    Not yet: strings, sets, records, `nil` (no `nil` passed by address
    is known); `lea (d,PC)`.  Conformance
    units (`make conform`): `term_send_kbd_string.pas`, `mst_maps.pas`
    (MST_$REMAP), `mst_set_touch_ahead_cnt.pas`, `sysbus_define_int.pas`,
    `pchist_interrupt.pas`, `file_set_len.pas`, `asknode_info.pas`,
    `proc2_quit.pas`; goldens `calls/const_pool`,
    `calls/const_pool_noalign`.  A unit that is a fragment of a module
    must not name a cell placed after a routine outside it (305 sites
    do); such units are not supported by the harness.
  - **To a `univ` parameter (reference; bead domain-pascal-4be).**  Ref
    5.3.3 p. 5-11: "When you pass an expression argument (as opposed to
    a variable argument) to a univ parameter, Domain Pascal extends the
    expression to be the same size as the univ parameter" (message
    "Expression passed to UNIV formal NAME was converted to NEWTYPE").
    `dpc`: a constant of the parameter's size keeps its own type; an
    integer constant for a larger integer parameter gets a cell of the
    parameter's type (`0` for `univ integer32`: `0000 0000`); any other
    size mismatch (a larger constant, a non-integer constant or
    parameter) is "not supported yet".  The cell holds the value (a
    2-byte `0..65535` constant 65535 becomes `0000 ffff`, -2 becomes
    `ffff fffe`).  `dpc-sema` gives the reference's warning (Appendix B
    message 246) for an expression smaller than its `univ` formal.
    Image (lead: ../source pascal-translation-notes findings 16, 25):
    **constants are not extended to the size of a `univ` formal
    (Verified where the formal is Apollo's own).**  vfmt_$write2's
    heading in Apollo's customer insert (section 0.1;
    `sys/ins/vfmt.ins.pas`: `IN control_string: UNIV vfmt_$string_t`,
    `ARRAY [1..200] OF char`; `IN a1, a2: UNIV
    vfmt_$generic_signed_arg`, an `integer32`) is the formal at the
    module-local 0xE2FF7C (LOG_'s OS_INIT_PROC segment, `jsr
    VFMT_$WRITE2` 0xE2FF96, 0xE2FFAA, 0xE2FFC0).  Its strings' cells
    have the strings' own sizes, not 200: 0xE2FFD4, 37 characters plus
    a pad byte, and 0xE30000, 32 bytes.  A constant *larger* than its
    formal is not cut either: the 36-character path at 0xE30020 goes to
    `a1` (4 bytes) whole (`pea (0x66,PC)` 0xE2FFB8), the same cell
    LOG_$INIT passes to NAME_$RESOLVE (0xE3005E).  The integer 0 for
    `a1`/`a2` is a 4-byte cell, 0xE2FFFC, which LOG_$INIT names again
    for LOG_$ADD's data argument (`pea` 0xE30214), as LOG_$SHUTDN
    0xE175A0 names its own 4-byte 0 (0xE17608); LOG_$ADD's heading is
    not Apollo's (../source's reading: `univ`, 192 bytes), so that pair
    is Reported.  KBD_$INIT 0xE3338E `pea (0x4a,PC)` names 0xE333DA, a
    2-byte `32 00` cell (`'2'`, followed by the next segment, SIO_IO
    0xE333DC), for a string parameter of the module-local 0xE1CA8C
    (4 bytes in ../source's reading; Reported).  So the reference's
    sentence (p. 5-11) does not describe the cells; what message 246's
    "converted" means for a by-address constant is not seen.  `dpc`
    differs (extends integer constants, refuses the rest; bead
    domain-pascal-c5u).
    **The cell rule above holds at two more routines (Verified; lead:
    finding 25):** LOG_$INIT's 10 cells 0xE3022A..0xE30267 (word group
    22A, 22C, 232, 238, 23A, 23C, 23E, 244; `0x2048` at 0xE3024E; then
    250, 258) and 0xE2FF7C's (group 1 0xE2FFD4; filler 0xE2FFFA; group
    2 FFFC, 30000, 30020, 30044) are in exactly the order above.
    LOG_$INIT has two `00 00` word cells (0xE30238, 0xE3023C; a word 0
    and `false` in ../source's reading), more evidence that a cell's
    identity is more than its bytes.  The constant 36 has a word cell
    (0xE3022A) and a longword one (0xE30044).
- **Expression by address (Verified at the sites; implemented).**  A
  non-variable, non-constant actual passed by address is first stored in a
  frame temporary, and that temporary's address is pushed, right where
  the argument is pushed (after the arguments to its right).  TONE_$ENABLE
  0xE1ACFC: `lea (0x1268,A5),A0; move.l A0,(-8,A6); pea (-8,A6)`.
  - The temporaries lie below the other frame objects ("locals, result,
    then temporaries", section 2; TONE_$ENABLE `link #-8`, status local
    at -4).  Two in one call are allocated downward in push order and
    use A0 then A1 for `addr`: SIO_$INIT 0xE32C28/0xE32C42 (-8 then
    -12), PCHIST_$CNTL 0xE5CF06..0xE5CF1A (-40 in A0, then -44 in A1).
  - Calls reuse the same slots: 80 routines store a value into a frame
    slot and push its address right away (`move.<s> X,(d,A6); pea
    (d,A6)`), 18 of them with one offset for two or more calls
    (GET_BUILD_TIME 0xE38052: `(-6,A6)` for five calls; `tools/pcsurvey.py
    temps`; some of these may be declared locals).
  - **Implemented** (`dpc-ir` `Arg::Temp`, `assign_temps`; `dpc-m68k`
    `Selector::push`): `addr(x)` of a frame or static object as `lea;
    move.l An,(d,A6); pea (d,A6)`, other values computed and stored;
    each word aligned, longwords longword aligned.  Uncertain: the
    order of temporaries for a call nested in another call's arguments
    (dpc puts them below the outer call's).  An expression of another
    size than the parameter (to a `univ` one: extended, ref 5.3.3 p.
    5-11; but by-address constants are not extended in the image, 1.5
    "To a `univ` parameter") is rejected: its conversion code is not catalogued (bead
    domain-pascal-4be).  Conformance:
    `tone_enable.pas`.
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
  0xE6BDA8; bead domain-pascal-5aw.15).  A module-local routine the map
  does not name is declared `internal; extern` under a reconstructed
  name (`internal` for the by-value convention of a routine of the same
  module, ref 5.2 p. 5-4), and the manifest gives that name with the
  address, `module-routines=NAME=0xADDR`; the harness resolves the
  local symbol with `--defsym` (MST_$MAPS 0xE439C4 `bsr.w` to the
  module-local 0xE43182; bead domain-pascal-3h9;
  `tests/conformance/README.md`, "Routines outside the unit").  This is
  test scaffolding: the reference allows calls of an `internal` routine
  only within its compilation unit (5.2, p. 5-4), so a whole-module
  translation has the body; without `--module-routine` naming it, `dpc`
  rejects a call of an `internal` routine with no body in the unit, at
  the statement holding the call (driver test
  `module_routine_unnamed_by_the_map`).
- **Other modules:** `jsr abs.l` (0xE00EF6 `jsr 0x00E20B12`).
- **Another code section of the same module:** `jsr abs.l` too, never
  `bsr` (191 sites, none by `bsr`; PROC1_$INIT 0xE2F9EC `jsr 0xE14B12`,
  an unnamed routine of PROC1_ in WIRED_PROC called from OS_INIT_PROC).
  **Implemented** (`dpc_ir::Callee::Section`; section 11).  Inside one
  segment, `jsr abs.l` only where `bsr.w` cannot reach (DIR 0xE535FE,
  0x82BE bytes back; not implemented).
- **Procedure variables:** `movea.l ...,An; jsr (An)` (0xE0B3F0, 0xE1CE5E).
  **Survey (Verified; `tools/indcallsurvey.py summary|list|addr`; bead
  domain-pascal-5aw.8).**  84 `jsr (An)` in 0xE00800..0xE88834, 3 in hand
  assembly; no `jsr (d,An)` at all.  Of the 81 in compiled segments, 7 are
  in three routines whose frames are not the compiler's (module-local
  0xE6B074 in VFMT: `movea.l A0,A5`, `adda.l #20,SP`; 0xE70396 and
  0xE706C6 in SMD_UTILS: `unlk` before `movem`) and are left out.  The
  other **74 sites in 61 routines** (the tool's `summary` counts all 81:
  the 7 left out are A0 sites loaded after the pushes, 2 of them reading
  D0, and VFMT's `movea.l A0,A5` is its one "A5 written after the call"):
  - **Where the pointer comes from.**  A field of a record reached through
    an address register (54 `movea.l (d,An),Am`, 5 `movea.l (An),Am`):
    driver and handler tables (MAC_OS_$OPEN 0xE0B3EC `movea.l
    (0x3c,A4),A1`; IO_$ACQUIRE 0xE1A65A, an element of a table in the
    unit's data, `lea (0,A5,D0.w),A0; movea.l (0x6c,A0),A4`;
    TTY/SIO descriptors held in A2 since the entry, SIO_$I_CTS_CHANGE
    0xE1C724 `movea.l (0x38,A2),A0`); the unit's data (RING_INTERRUPT
    0xE0AB1C `movea.l (A5),A0`); a data register holding the pointer (10
    `movea.l Dn,Am`: a local or a value tested against `nil` first,
    DISK_$REVALID 0xE3DB86 `move.l (0xc,A0),D2; beq` then 0xE3DB8E
    `movea.l D2,A1`; IO_$INIT 0xE329CC `move.l A0,D3; pea (A3); movea.l
    D3,A0`); and in 5 sites the A0 a pointer function returned, kept
    from an earlier statement (NET_$OPEN 0xE5A1D8 `move.l A0,(-4,A6)`
    after `bsr.w 0xE5A128`, then a test and the pushes, 0xE5A1F4 `jsr
    (A0)`; NET_$CLOSE, NET_$IOCTL, NET_$RCV, NET_$SEND).  No call goes
    through a by-value routine pointer parameter or a register loaded at
    the entry.
  - **When it is loaded.**  In all 69 sites whose register is defined in
    the straight-line code before the call, the definition (the whole
    chain of loads and index arithmetic) comes **after the last push**
    and the pad: MAC_OS_$OPEN 0xE0B3DC `subq.l #2,SP; move.l
    (0x10,A6),-(SP); move.l D7,-(SP); move.w (0x30,A3),-(SP); movea.l
    (-4,A6),A4; movea.l (0x3c,A4),A1; jsr (A1)`; IO_$MAP_CSR
    0xE1A8B0..0xE1A8C8.  The other 5 are the NET_ sites above.  (In the
    DISK_ sites the pointer is fetched into D2 before the pushes, 0xE3DAB4
    `move.l (0x10,A0),D2`, but only copied into the call register after
    them, 0xE3DAC6 `movea.l D2,A1`: a register local, section 12.5.)
  - **Which register.**  A0 at 35 sites, A1 at 33, A4 3, A3 2, A2 1.  A1
    where the argument list uses A0 (TTY_$I_ERR 0xE1BE14..0xE1BE1A
    `movea.l (8,A6),A0; move.l (A0),-(SP); movea.l (0x2c0,A0),A1`), and
    also where A0 looks free (DISK_$REVALID 0xE3DB8E, MAC_OS_$OPEN):
    the allocator's choice (section 12.4), Uncertain.  A callee-saved
    register only where the chain goes through one (MAC_OS_$CLOSE
    0xE0B4B2 `movea.l (0x40,A1),A4`; IO_$XLATE_IOVA 0xE1A9B2).
  - **Pushes, pad, pops** as for any call (1.1-1.4): 36 calls not popped
    (the `unlk` follows, RING_INTERRUPT 0xE0AB1E, or a branch to a pop),
    `addq.w #4/#8`, `lea (n,SP)` for 12..36 bytes.  Values are pushed by
    value in word slots (MAC_OS_$OPEN 0xE0B3E4 `move.w (0x30,A3),-(SP)`,
    KBD_$RCV 0xE1CE56 `move.b D0,-(SP)`, TTY_$I_RCV 0xE1BA52 `st
    -(SP)`): the pointer types were `val_param`.
  - **Results.**  7 calls read D0 after the call (TTY_$I_ERR 0xE1BE22
    `move.l D0,(-4,A6)`; IO_$MAP_CSR 0xE1A8CC `move.l D0,D4`;
    DISK_$SPIN_DOWN 0xE3DB34 `cmp.w D0,D2`); none reads A0.
  - **A5.**  No caller saves, reloads or restores A5 around the call
    (0 writes of A5 after a call other than the epilogue restore): the
    callee is a top-level routine (ref 3.13.3, p. 3-50: only their
    addresses can be taken) and sets its own A5 (section 4).  A pointer
    read off A5 is an ordinary use of the unit's data (RING_INTERRUPT).
  - **Routine addresses** (44 operands naming a routine start, `link` or
    `rts` there): `move.l #sym,-(SP)` 17 (ATBUS_ 0xE0AB36
    `#DISK_INTERRUPT`, SIO 0xE1C8D0 `#SIO_DELAY_RESTART` of the same
    module), `move.l #sym,(d,A6)` 8 (MAC_OS 0xE0B9A8 `#MAC_$DEMUX`),
    `move.l #sym,(d,An)` / `(An)` 7 (PROC1_ 0xE14B7A
    `#PROC1_$TS_END_CALLBACK,(0x18,A2)`; OS_TERM_INIT 0xE32BCA), `move.l
    #sym,Dn` 3 (OS 0xE339B4), and `pea (d,PC)` 9, all of the DIR
    module-local 0xE4C9E4 (0xE4C310: a routine passed as a routine
    parameter, ref 5.4, not `addr`).  So `addr(r)` (ref p. 4-145, C.3.5)
    is the absolute symbol as an immediate.
  - **Reference.**  Routine options may follow a routine pointer type:
    p. 4-146 (checked in the PDF) declares `pptr : ^Procedure(IN i, j :
    integer; OUT a : char; VAR r : real); EXTERN;`, and Appendix B
    message 181 ("INTERNAL option is illegal for PROCEDURE^ or FUNCTION^
    types") and 182 ("Incompatible VAL_PARAM options ... The type
    declaration did not specify val_param, but the function heading
    did") presuppose them; `../source`'s TTY translation writes
    `^procedure(...); options(val_param);` (`tty/tty.ins.pas`, an
    agent-written reconstruction for the planned Pascal tree, not an
    Apollo include file: not evidence for the syntax).  The
    meaning of `extern` there, and whether `default_routine_options`
    (5.6.4) apply to such a type, are not stated.
  - **Implemented** (beads domain-pascal-5aw.8, -5aw.16): routine pointer
    types with `val_param` (short or `options(...)` form) after the type
    in `type`, `var` and record field declarations (`dpc-syntax`
    `routine_pointer_options`); the type's conventions and pad from
    Table 5-1 (`dpc-sema` `Checker::signature`); calls `p^(...)`,
    `p^` (a function with no arguments, ref p. 4-145), with
    `Arg::Address::writable` from the type's parameter modes (`dpc-ir`
    `call`); the pointer read after the pushes into a new address
    register, `movea.l <ea>,An` or `movea.l Dn,An`, kept apart from the
    argument list's registers, the copy pinned before the `jsr` by the
    scheduler (`dpc-m68k` `Selector::call_target`, `sched.rs`); data
    results in D0; `addr(r)` of a top-level, non-`internal` routine,
    checked against the pointer type it is assigned to (parameter
    modes, name-compatible types, result, `val_param`), pushed or stored
    as `move.l #sym,<ea>`.  Refused: `internal` on the type (message 181,
    an error), every other option on it, a type without options under
    `default_routine_options`, a pointer that is not a variable, field or
    element (`get^(...)`), a pointer held in an address register (the
    NET_ sites' A0: not modelled), a pointer reached through a parameter
    slot the arguments also read (TTY_$I_ERR loads the slot once for
    both; `dpc` would load it twice), pointer results, routine parameters
    (5.4).  Conformance: `tests/conformance/atbus_ring_interrupt.pas`
    (RING_INTERRUPT 0xE0AB0C); golden `tests/golden/calls/routine_pointers`.
- All kernel addresses are absolute.  This kernel image has no
  PC-relative inter-module calls.

---

## 2. Frame, prologue and epilogue (Verified)

- **Prologue:** `link A6,#-locals`.
  - `link.w` is always used.  No `link.l` appears in the kernel image.
  - The size is sometimes larger than the locals that are used.  IO_$ALLOC
    0xE1A9C2 and PROC1_$TST_LOCK 0xE148CA have `link #-4` and never touch
    the slot: their function result variables ("Frame size" below).
  - The displacement is always a multiple of 4: all 1171 `link.w A6,#d`
    found at routine addresses named in the map have `d mod 4 = 0` (a scan
    of every `4e56` word in 0xE00800..0xE88834 finds one exception, not at
    a routine start, possibly data).  The total is rounded, objects do
    not get 4-byte slots each: 282 lone word objects sit at 2 mod 4
    ("Frame layout" below).  `dpc-sema` records the raw size
    (`Routine::frame_size`) and leaves the rounding to the backend.
  - **Empty frame (Verified):** `link.w A6,#0` when there are no locals:
    TIME_$Q_INIT_QUEUE 0xE16C5E, LOG_$READ 0xE17800.  The `link` is not
    omitted.
  - **Implemented:** `dpc-m68k` rounds the *total* up to a multiple of 4
    (`M68k::link_size`): 1 byte (TST_LOCK's boolean result variable) gives
    `#-4`, 0 gives `#0`.  This fits every site above.  PKT_$NEXT_ID
    0xE1248E favours it over 4-byte slots per object: a word local at
    `(-2,A6)` (0xE124A8) and its word result variable share `link #-4`
    (one site; the Uncertain point stays open).
  - **Frame size (model; items 1 and 3's slot Verified by survey, the
    result's position Uncertain; bead domain-pascal-9mr):**
    the frame holds, from the top down,
    1. a **4-byte slot for a routine that is not a global symbol**
       (nested, or `internal`: unnamed in the map), never addressed except
       by nested routines that store a static link there (section 6,
       "Frame");
    2. local copies of value parameters (place Uncertain: at the top in
       29 routines, below other objects in 34, not always in parameter
       order; section 1.2, "Value parameters passed by address"; `dpc`
       puts them first, in parameter order) and the locals, whether or
       not the code ever keeps them in memory ("Untouched frames" below);
    3. a function's **result variable**, below the locals (its place is
       Uncertain: sites both ways, see below);
    4. the **temporaries** of expressions passed by address (section
       1.5: TONE_$ENABLE 0xE1ACE8, the temporary at -8 below the status
       at -4), shared by the calls of the routine (implemented, bead
       domain-pascal-5aw.9);

    and the total is rounded up to 4.  Evidence (`tools/nestsurvey.py
    slots` and `frames`; routines split as in section 4, a *function*
    being one some caller reads D0 after calling, section 3):
    - Unnamed routines that are not nested: 1 of 355 has `link #0`
      (0xE6B074, a VFMT segment start, not called by a `bsr`) and 1
      addresses `(-4..-1,A6)` (0xE70396, not called by a `bsr` either),
      against 190 `link #0` and 320 that address it among the 1194
      map-named routines.  The 130 unnamed routines called by two or more
      map-named routines, which cannot be nested in any, have no
      exception.  Nested routines: 0 `link #0`, and `(-4,A6)` only as the
      static-link store (3 of 86).
    - Map-named routines reserve nothing: their first locals sit at the
      top (PKT_$NEXT_ID's word at `(-2,A6)`; FILE_$REMOVE_WHEN_UNLOCKED
      0xE5FC8E passes a byte at `(-2,A6)` by address in `link #-4`;
      REM_NAME_$REGISTER_SERVER `link #-8` is not explained by this, see
      `tests/conformance/rem_name_register_server.pas`).
    - Functions: none of the 109 map-named functions has `link #0` (21
      have `#-4`, like TST_LOCK and IO_$ALLOC with no locals), against 190
      of the 1085 other map-named routines; cross-checked with the C
      reconstructions in `../source` (map names whose C definition is not
      `void`: 1 `link #0` among 195, MST_$MAP_AT 0xE42F54, a wrapper
      whose A0 is the callee's left over; both callers ignore it, so it
      may well be a procedure).  The 72 unnamed, not nested functions
      and the 11 nested ones all have `link #-8` or more (slot + result),
      while 31 and 10 of the others have just `#-4` (the slot).
    - Where the result variable goes is **Uncertain** (bead
      domain-pascal-67c): no fixed place in declaration order fits every
      site, and `dpc` keeps the majority, below the locals.  Sites where
      the link size and the C's locals force the order (the result held
      in a register, its slot never addressed; a byte takes a word,
      "Frame layout" below):
      - *Result below a local* (a local at the top of the frame, so the
        result is not first): PKT_$NEXT_ID 0xE1248E (token `move.w
        D0,(-2,A6)`, word result in D2, `link #-4` = 2 + 2);
        PROC2_$GET_PID 0xE40CBA (status at `(-4,A6)` by address, the
        index word in D0, the word result in D2: `link #-8` = 4 + 2 + 2);
        KBD_$GET_CHAR_AND_MODE 0xE724C4 (a pointer in A2, a word at
        `(-6,A6)` by address, the byte result in D2: `#-8` = 4 + 2 + 2);
        SOCK_$PUT 0xE1614E (a pointer at `(-4,A6)` by address, a byte
        result: `#-8`); NETBUF_$GET_HDR_COND 0xE0ED6C and MMAP_$ALLOC_FREE
        0xE0D870 (the spin-lock token at `(-2,A6)`, a byte and a word
        result).
      - *Result above a local*: NETBUF_$RTNVA 0xE0ED26 (`link #-8`, the
        token at `(-6,A6)`, a 4-byte result `move.l D2,D0`): 4 + 2 fits 8
        only with the result at -4 above the token (with the result last,
        the token would be at -2; with a 4-byte local first as well, the
        frame would need 12).  SIO_$I_ERR 0xE67D9C (`link #-12`): its
        result, assigned on every path at `(-8,A6)` and returned from
        there, lies below an untouched longword (the C's `taken`, a
        4-byte set held in D2, whose slot 12.5 says stays allocated) and
        above the token at `(-10,A6)`: in the middle.
        TTY_$I_GET_DESC 0xE66738 (`link #-12`): a status local at
        `(-4,A6)` passed by address (`pea (-4,A6)`, 0xE6678E), the
        pointer result at `(-8,A6)` (stored 0xE667B0, loaded into A0
        0xE667B6), the real line held in a data register (its slot
        below): in the middle.  SIO_$I_GET_DESC 0xE667C6 (`link #-8`):
        the result at `(-4,A6)` (0xE66808, 0xE6680E), the real line held
        in D1: above a local.  Both count only under 12.5's rule that a
        register-held local keeps its slot, as for SIO_$I_ERR (lead:
        ../source pascal-translation-notes finding 13; bead
        domain-pascal-67c).
      - *Not decisive*: a slot written on every path and returned with
        `move.l (-n,A6),D0` can be a local copied by a final `f :=
        local`, which goes straight to D0 (section 3).  So FLP_$CINIT
        0xE3E002 (`(-4,A6)`, `link #-28`; its early exit is `move.l
        #imm,D0; bra` to the epilogue, the result assigned directly),
        PEB_$GET_STATUS 0xE5ADCA (`move.l (-8,A6),(-16,A6); move.l
        (-16,A6),D0` in `link #-16`), the PACCT module-local 0xE5A9CA
        (a packed word built at `(-8,A6)` and returned, `link #-12`),
        BAT_$ALLOC_FM 0xE3AD20 and SMD_$N_DEVICES 0xE70024 fit either
        place.  FLP_$CINIT fits "locals, result, then temporaries" if
        `(-4,A6)` is a local status: the gap at -24..-21 for the result
        and the `addr(...)` temporary at `(-28,A6)` passed by address.
      Six forced sites put the result below, two above and two in the
      middle.  Possible causes not settled: declarations the C does not
      show; a result variable without a slot of its own (it would fit
      NETBUF_$RTNVA with a 4-byte local first, but not the functions with
      no locals and `link #-4`, TST_LOCK, nor the 0 of 83 unnamed
      functions with only the reserved slot); an order the compiler does
      not take from the source (a symbol-table walk).  Allocating upward
      from the bottom (hypothesis b) is the same as allocating downward
      in reverse declaration order, except for where the rounding pad
      goes, and fits these sites no better.
    - Image sites the model explains with no stand-in locals: 0xE32B76
      `link #-4` (internal, no locals), 0xE68864 `#-8` (internal, word
      result), 0xE3CCCE `#-8` (internal, one pointer local held in A1),
      PROC1_$TST_LOCK / PROC1_$GET_LOCKS `#-4` (result only), 0xE1359C
      `#-4` (nested, no locals), 0xE5A9CA `#-12` (internal, a 4-byte local
      at `(-8,A6)` read back as a word, a word result).  Partly: 0xE00EB0
      `#-12` (slot + `for` index = 8; 4 bytes unexplained), 0xE17778
      `#-16` (slot + pointer + index = 12; 4 unexplained).

    **Implemented** (`dpc-sema` `routine_body`: the frame of a routine that
    is not `global` starts at 4; the result variable is allocated after
    the declarations; test `frame_size_model` in
    `crates/dpc-sema/src/tests/table5_1.rs`).  The conformance units that
    declared stand-in locals for 0xE32B76, 0xE68864 and 0xE3CCCE no longer
    do; `ast_page_zero.pas` and `pending/log_read.pas` keep 4 stand-in
    bytes each, `rem_name_register_server.pas` its 8.
  - **Frame layout: alignment (Verified by survey; implemented; bead
    domain-pascal-ek7).**  Every frame object is at least word aligned,
    and 4-byte scalars are longword aligned.  `tools/framesurvey.py
    align` classes the plain `(d,A6)` operands of the 1635 routines with
    a `link`.  Among the *lone* objects (accessed at one size only,
    nothing else addressed in their word or extent) of frames where no
    frame address is taken below them (an address taken below may make
    them fields of a record or array there):
    - bytes: 121 at even offsets (53 at 0 mod 4, 68 at 2 mod 4), none at
      odd ones;
    - words: 450 at even offsets (168 at 0 mod 4, 282 at 2 mod 4), none
      at odd ones;
    - longwords: 659 at 0 mod 4, none at 2 mod 4.
    (In frames with an address taken below them, 85 lone longwords sit
    at 2 mod 4 and 34 lone bytes at odd offsets: fields of word-aligned
    records or arrays, not counted above.)  The 13 odd byte offsets
    accessed in the other frames all lie in another object's word or
    extent, the low byte of a word or longword: SMD_$VIDEO_CTL 0xE6F838
    `(-1,A6)` of the word at -2, VTOCE_$READ 0xE394EC `andi.b`/`or.b` on
    `(-1,A6)`, the low byte of the longword at -4.  A lone byte local
    sits at `(-2,A6)` in `link #-4`: FILE_$REMOVE_WHEN_UNLOCKED 0xE5FC8E
    (`pea (-2,A6)`, then `move.b (-2,A6),D0`).  ACL_$INHERIT_SUBSYS
    0xE49138 and ACL_$OVERRIDE_LOCAL_LOCKSMITH 0xE49254 also have a byte
    at `(-2,A6)` in `link #-4`, but store it and reload it at once
    (`move.b D0,(-2,A6); ...; move.b (-2,A6),D2`, a bit index `& 7`),
    so it may be a compiler temporary rather than a declared local.
    Limits of the survey: bytes sharing a word with another accessed
    offset are not lone by construction, so packing would show only as
    odd lone bytes (none) and is ruled out by the pairs below; accesses
    by nested routines through the static link are not seen in the
    parent (28 of the 121 lone bytes are in the 59 parents of nested
    routines; without them 93, still none odd); `btst`/`bset`/`bclr`
    and `movem` operands are not classed (an offset with a `btst` is
    never lone); indexed `(d,A6,Xn)` operands are not counted, but none
    lies below a lone object.  Byte locals are not packed two to a
    word: `tools/framesurvey.py byte-pairs` finds 17 routines with lone bytes in adjacent words, e.g.
    PROC2_$WAIT 0xE3FDD0 (booleans at -150, -148, -146 and -144, set
    with `clr.b`/`st` and tested with `tst.b`; by-address bytes at -142
    and -140) and PMAP_$FLUSH 0xE1376C (-86/-88, -94/-96; its nested routines
    0xE1359C and 0xE1360C address only -80 and -64 of its frame).  So a byte
    takes the high byte of a word, as in an argument slot (1.3).  For
    objects longer than a byte this is the record rule of ref 3.10.5.5
    p. 3-26 ("all objects longer than a byte are aligned on word
    boundaries"), but records pack bytes and frames do not (nor do
    module data blocks, section 11, "Byte-sized module variables").
    **Order: not sorted by size** (`tools/framesurvey.py order`): 200
    map-named routines have a lone byte or word object above a lone
    longword, 105 have every longword above them.  Declaration order is
    the natural reading but cannot be checked without the sources;
    `dpc` allocates downward in declaration order.
    **Implemented** (`dpc-sema` `alloc_frame`, `FRAME_MIN_ALIGN`): each
    object at `round_up(frame + size, max(align, 2))`, longword
    alignment from the freestanding alignment of 4-byte scalars and
    pointers (ref 3.2.4 p. 3-5, 3.13.5 p. 3-52).  PROC1_$TST_LOCK's
    boolean result variable is now at `(-2,A6)` (still `link #-4`).  Test
    `frame_objects_word_aligned` (`crates/dpc-sema/src/tests/
    table5_1.rs`); golden `frame/byte_locals`.  No conformance unit has
    a byte local in memory yet.  FILE_$REMOVE_WHEN_UNLOCKED, the smallest
    (0x34 bytes), calls the unnamed module-local 0xE5E8E0, which a
    fragment can now name (`module-routines=NAME=0xADDR`, bead
    domain-pascal-3h9), and `dpc` reproduces it up to its last
    statement: the image copies the byte through D0, `move.b
    (-2,A6),D0; movea.l (0xc,A6),A0; move.b D0,(A0)` (0xE5FCB0), where
    `dpc` stores memory to memory, `movea.l (0xc,A6),A0; move.b
    (-2,A6),(A0)`.  Of the frame locals stored through a parameter's
    pointer right after it is loaded, 129 are memory to memory
    (TIME_$Q_REMOVE_ELEM 0xE16E88; longwords 106, words 22, bytes 1,
    0xE6FA68) and 7 go through a data register first (0xE5FCB0; 0xE42E20,
    0xE43590, 0xE443E8, 0xE0BC20, 0xE18B5C, 0xE64CA8); at 0xE42E20,
    0xE43590 and 0xE443E8 a `bne` on the flags of the store follows (the
    value tested as well as stored), elsewhere the reason is not seen
    (ad-hoc survey, not kept).  The ACL_ ones need bit-array stores.
  - **Objects of 8 bytes or more at multiples of 8 (Reported; rule
    Uncertain; lead: ../source pascal-translation-notes finding 6; bead
    domain-pascal-1edl).**  Module-local 0xE1C204 (../source
    tty_$i_wait, `link #-108`): an eventcount (12 bytes) at -0x10, a
    time queue element (26 bytes) at -0x30, the two 3-element
    EC_$WAITN lists at -0x40 and -0x50; each object right below the one
    above it under longword alignment would give -0xC, -0x2C, -0x3C and
    -0x4C.  The 6-byte clocks below (-0x58, -0x60) are longword placed.
    (Every 8+-byte object there is also at a multiple of 16; the routine
    cannot tell 8 from 16.)  The module-local 0xE4232A (`link #-16`)
    has its 8-byte result at -0x10..-0x9 (`move.l (A0)+` to -0x10 and
    -0xC, 0xE42338) with nothing at -8..-1.
    Temporaries do not follow it: MSG_$SHARE_SOCKET 0xE5A02C (`link
    #-12`, an 8-byte set at -0xC, nothing else addressed),
    PROC2_$UPGID_TO_UID 0xE4100C (an 8-byte temporary at -0x14).  An
    ad-hoc survey (not kept) of 8-byte copies into the frame (exactly
    two `move.l (An)+,(d,A6)`): d mod 8 = 0 at 154 map-named and 97
    unnamed sites, 4 at 69 and 9, 2 or 6 at 15 and 1; copies out of the
    frame 28 at 0, 26 at 4.  Many off-rule sites are runs 8 apart
    (elements or fields, e.g. 0xE4AF52 -44/-36/-28), but some are not
    explained: VTOC_$DISMOUNT 0xE387E0 (-12, words at -4 and -2 above),
    VTOC_$MOUNT 0xE38658 (-20), the module-local 0xE323E6 (a source at
    -52).  Not modelled: `dpc` aligns frame objects to at most 4.
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
    (a)), and `tests/conformance/ast_page_zero.pas` declares a 4-byte
    stand-in local for the part of 0xE00EB0's 12 bytes that the frame-size
    model leaves unexplained (its header and the declaration say so).  If
    (b) is confirmed, that stand-in goes.  Evidence gathered for
    bead domain-pascal-nda in section 7.4 ("Hidden frame cost"): five
    one-loop routines have no frame bytes beyond their locals, and three
    routines of one loop shape have `link #-8`, `#-12` and `#-16`, so a
    `for` loop costs no hidden slot of 4 bytes or more; (a) is favoured.
    The unnamed routines among these (0xE00EB0, the KBD module-local
    0xE1CA62, ...) have 4 bytes fewer to explain: their frames start
    with the reserved slot ("Frame size" above).
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
    is refused: a bare `rts` has no frame for the copy, section 1.2), and
    never loads A5 (section 4).
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
  `d0_return`, 5.5.11 `a0_return`; `ARCH_RESULT_A0` in
  `../source/domain_os/arch/m68k/arch.h`).  "Pointer results" below has
  the survey.  (TIME_$RTE_INT / TIME_$VT_INT, cited here before, end
  with `movea.l #0,A0` at 0xE163D6 / 0xE16426, but no compiled caller
  reads their A0 and `../source` has them as procedures: not evidence.)
- **Procedures** leave D0/A0 as whatever the body left.  Callers never read
  them.
- **Result assignment as the last statement (Verified, one site):** in
  PROC1_$TST_LOCK the body is the single statement `f := x in s`.  The
  value is computed into a scratch register and moved straight into D0
  (0xE148E0 `move.b D1,D0`), followed by the epilogue.  The frame has
  room for a result variable (`link #-4`), but the slot is never written
  or read.  `dpc` compiles a function body of one simple statement this
  way.
- **Result variable (Verified by survey; implemented; beads
  domain-pascal-5aw.11, -8aj).**  `tools/resultsurvey.py` classes the
  epilogue of every *D0 function* (a routine some compiled caller reads
  D0 right after calling: 192, each with one epilogue) by the instruction
  that falls into it: `move.<s> Dn,D0` 73, a constant into D0 37, another
  instruction writing D0 31, D0 set earlier 24, `move.<s> (-k,A6),D0` 18,
  a call (the tail wrapper below) 8.
  - *Held in a register up to the epilogue* (the 73): 67 from a
    callee-saved register, 66 of them with a call between the register's
    first write and the end (`list reg`): PKT_$NEXT_ID 0xE1248E (`move.w
    (0x5c,A5),D2` at 0xE124AC, then `addq`, an `if`, the call
    ML_$SPIN_UNLOCK, `move.w D2,D0` at 0xE124D0); PROC2_$GET_PID
    0xE40CBA (assigned only inside an `if`, 0xE40CF6, then a call;
    `move.w D2,D0` at 0xE40D0E; on the other path D2 is what it was);
    NETBUF_$RTNVA 0xE0ED26 (a call's result, `move.l D0,D2` at 0xE0ED4E,
    across the next call); SMD_$N_DEVICES 0xE70024 (`clr.w D2`, then
    `move.w D4,D2` in a loop).  6 from D1, with no call after: the
    value could not be built in D0 (PROC1_$TST_LOCK's `sne D1`;
    0xE6FAA8, `seq D1` while the parameter holds D0).  Boolean constants
    are `clr.b`/`st` (0xE41216 `st D2`, 0xE4125C `clr.b D2`; no `moveq`
    writes the register of any of the 31 byte results of `list reg`).
  - *In D0 all along* when no call follows: PARITY_$CHK_IO 0xE0B174
    (`moveq #1,D0`, `moveq #2,D0`, `clr.w D0` on three paths, then a
    `beq` on the last one's flags at 0xE0B1AC); PROC1_$SUSPENDP 0xE14876
    (`st D0` at 0xE14886, `sne D0` at 0xE148BC, status stores after it);
    the early exits of 0xE0B202, 0xE7630C, 0xE76352, 0xE76422 (`move.w
    D2,D0` or `st D0`, then a `bra` to the epilogue from inside a loop).
  - *From a frame slot* (the 18): a local returned by a final `f :=
    local` fits every one (see section 2, "Not decisive"): its address is
    taken at BAT_$ALLOC_FM 0xE3AD20 and 0xE062FA, only callees write it
    at PROC2_$INIT 0xE303D8 and REM_FILE_$CREATE_AREA 0xE62622, and most
    others are status codes stored on several paths (0xE2FA86, 0xE43F40;
    SIO_$I_ERR 0xE67D9C, which no compiled caller reads, likewise) or a
    packed record built in place (0xE5A9CA).  None of them shows that
    the slot is the result variable's.
  - *Reading the result variable* is not part of the language: inside
    `f`, the name `f` in an expression calls `f` (ref 5.8, p. 5-26,
    `factorial(n-1)`; `dpc-sema` reports a missing argument).  Sites that
    read a returned value back read a variable that shares its register
    (PARITY_$CHK_IO's `beq` at 0xE0B1AC).

  **Implemented** (`dpc-ir` `result_assignment`; `dpc-m68k` `isel.rs`
  `set_result_web`, `select_body`; `regs.rs` `allocate_apart`): in a
  function body of several statements (or of one `if`, `case` or loop),
  every `f := e` writes one virtual data register: a constant with
  `clr`/`moveq`/`move #`, `true` with `st`; a value in a register (a
  call's D0, a parameter loaded at entry) with a `move`; memory with a
  load; a computed value is computed in it.  At the start of the
  epilogue, where `return` branches to, `move.<s> Rn,D0`, dropped when the
  allocator gives the variable D0.  The variable is coloured before every
  other value, so it is D0 unless a call follows an assignment whose
  value is still needed (then the lowest free callee-saved register,
  12.4 R3).  Golden `tests/golden/results/result_variable`; conformance
  `tests/conformance/pkt_next_id.pas`.  A register value the variable is
  assigned from may share its register where that value dies (12.4 R2
  does not apply to it), so a final `f := g(x)` is the tail wrapper below
  in a body of several statements too.  Not reproduced: a parameter
  assigned to the result and still used after shares its register with
  it (0xE15222, `(8,A6)` into D0 at 0xE15226, compared after, returned
  from there; MST_$VA_TO_SEGNO 0xE0DCE0); dpc copies it.  Refused: assigning an enclosing function's result
  from a nested routine (no site seen).  ISO's "No assignment to function
  variable" case (a body with no `f := e`) is refused too.
- **Pointer results (Verified at the sites; implemented narrowly).**  40
  *A0 functions* (some compiled caller reads A0 right after calling;
  none of them also a D0 function), by the instruction falling into the
  epilogue (`tools/resultsurvey.py pointer`): `movea.l Rn,A0` 16 (A2 in
  12, A3 in 1, held across calls: DISK_$GET_BLOCK 0xE3BB80, `movea.l
  A0,A2` at 0xE3BBBC after the call, `movea.l A2,A0` at 0xE3BBCA; a D
  register in 3), `movea.l (-k,A6),A0` 8, A0 set directly 7 (`suba.l
  A0,A0` for `nil` 4, e.g. ROUTE_$FIND_PORTP 0xE15B84; `lea` 3), other
  7, a call 1.
  Unlike a data result, a pointer assigned by the last statement goes
  through the result variable's frame slot: from memory `move.l
  <src>,(-k,A6); movea.l (-k,A6),A0` (the module-local 0xE6AA38, `link
  #-8`, the slot below the reserved 4 bytes; SIO_$I_GET_DESC 0xE66808,
  TTY_$I_GET_DESC 0xE667B0, KBD_$GET_DESC 0xE1AA92, 0xE5A180); a computed
  value is stored and moved, `move.l D0,(-12,A6); movea.l D0,A0`
  (0xE4D5A6); a call's A0 is stored and left in A0, `move.l A0,(-4,A6)`
  (MST_$MAP 0xE438BE, MST_$MAPS 0xE439C8).  The three other `movea.l
  (-k,A6),A0` reload a slot written before a call (BAT_$ALLOC_VTOCE
  0xE3AEC0, MST_$REMAP_PRIVI 0xE43A0C, 0xE43182).  `d0_return` (both registers,
  ref 5.5.10 p. 5-20) is seen at no compiled routine.
  **Implemented** (`isel.rs`, `set_pointer_result`,
  `set_pointer_result_web`): as the only statement of the body, the
  memory form (conformance `tests/conformance/vfmt_get_next_arg.pas`,
  0xE6AA38) and the call form, `move.l A0,(-k,A6)` with A0 left as the
  call set it (MST_$MAP, MST_$MAP_TOP, MST_$MAP_GLOBAL, MST_$MAPS:
  conformance `tests/conformance/mst_maps.pas`; bead domain-pascal-ae9);
  in a body of several statements, the result variable held in an
  address register like the data result variable above, assigned from
  a pointer function's A0 (`movea.l A0,An`) or from memory (`movea.l`),
  moved to A0 at the start of the epilogue, dropped when it is A0
  itself (DISK_$GET_BLOCK: conformance
  `tests/conformance/disk_get_block.pas`; golden
  `results/pointer_callers`).  Refused: a computed value, `nil`
  (`suba.l`; or the result slot cleared and reloaded, NET_$FIND_HANDLER
  0xE5A178; bead domain-pascal-bdi0) and a computed address (`lea`, EC2_$GET_EC1_ADDR 0xE42B0A)
  as pointer results, `d0_return`/`c_param` pointer results, and
  `a0_return` on a non-pointer result (no site).  Also refused: a
  register-held pointer result variable that the allocator would put in
  A1 (assigned a call's A0, no call after; bead domain-pascal-500): the
  12 in A2 and the one in A3 all have a call after their first
  assignment, the one with none is in D0 (0xE6C21A: `move.l A0,D0` at
  0xE6C244, `clr.l D0` on another path, `movea.l D0,A0` at 0xE6C2AA),
  others are in their frame slot (TTY_$I_GET_DESC 0xE667BA).
- **Callers of pointer functions (Verified by survey; implemented; bead
  domain-pascal-ae9).**  `tools/resultsurvey.py callers` takes the 234
  calls of A0 functions after which the caller reads A0, and classes the
  first instruction that reads it (after the pop, when there is one; 20
  are not popped):
  - `movea.l A0,An` 108, in A2 57 (e.g. KBD_$SET_KBD_TYPE 0xE7253E,
    DISK_$GET_BLOCK 0xE3BBBC), A3 41, A4 7, A1 3: a register-held local
    or result variable; in 106 a call follows before An is written again;
  - `move.l A0,Dn` 48, D0 in 34 (the TTY_$K_* routines after
    TTY_$I_GET_DESC, e.g. TTY_$K_INQ_FUNC_ENABLED 0xE6763C; the pointer
    is moved back with `movea.l D0,A0` where it is dereferenced,
    0xE67642), D2..D6 in the others (AREA_$REMOVE_SEG 0xE099AC `move.l
    A0,D3`);
  - `move.l A0,(-k,A6)` 34: a local's slot (AST_$INVALIDATE 0xE06664,
    VTOC_$LOOKUP 0xE3904C) or a wrapper's result variable (above);
  - `move.l A0,<memory>` 21: the unit's data (TIME_$GET_EC 0xE16734
    `move.l A0,(0x1620,A5)`), another unit's variable (PEB_$LOAD_WCS
    0xE32072 `move.l A0,abs.l`), through a pointer held in a register
    from before the call (MAC_$OPEN 0xE0BA56 `move.l A0,(A2)`);
  - `cmpa.w #0,A0` 12: `= nil` / `<> nil` (AST_$LOAD_AOTE 0xE023BA; the
    NET module-local 0xE5A128 at 0xE5A150 after ROUTE_$FIND_PORTP);
  - A0 as a base 8: `tst.w (A0)` (the DIR module-local 0xE4E960 at
    0xE4E990), `clr.w (A0)` (0xE4EA0E), `move.w (2,A0),D7` (VFMT_$MAIN
    0xE6AD10), `move.l (A0),(28,A2)` (OS_PROC_WIRED 0xE008F8);
  - never pushed as an argument (no `move.l A0,-(SP)` or `pea (A0)`);
  - 3 others are false positives (a `movea.l A0,A0` filler, 0xE649E6,
    0xE101AA; a `suba.l A0,A0` after a procedure, 0xE0236C).
  **Implemented** (`dpc-ir`: `Expr::Call`'s `reg`; `isel.rs`, `call_a0`
  and its uses): the call's value is A0; a store moves it to memory, to
  a register-held local (`move.l A0,Dn`, 12.5) or to a loop pointer
  local (`movea.l A0,An`, 7.4), a comparison with `nil` is `cmpa.w
  #0,A0`, a dereference uses A0 as the base, `f := g(...)` is the call
  form above.  A store whose destination needs a register of its own is
  refused as for any call value (whether the original loads it before
  or after the call is not catalogued; MAC_$OPEN's A2 is held from
  before).  Other uses (an argument, an operand of an expression,
  a comparison other than with `nil`) are refused (`dpc-m68k`
  `Selector::eval`; driver test `value_of_a_pointer_function_is_read_from_a0`).
  A0 is consumed in the statement; a value that must live across a later
  call is copied to a callee-saved register by the allocator (12.4 R3,
  DISK_$GET_BLOCK's A2).  `d0_return` callees are read from D0 (ref
  5.5.10; no compiled site).  Not reproduced: the reuse of A0 after the
  first store to memory other than the local's slot (SMD_$MAP_DISPLAY_U
  0xE6F94E `move.l A0,(A4)`, then `move.l A0,(-232,A1)` at 0xE6F964, a
  common subexpression, 12.4 R7).

  **Pointer locals (Verified by survey; implemented; bead
  domain-pascal-500).**  Who keeps a pointer local assigned from a call
  in a register (`tools/resultsurvey.py ptrlocals`; the frame-slot and
  `move.l A0,Dn` sites checked by hand):
  - the 108 `movea.l A0,An` copies are never pushed by value (`move.l
    An,-(SP)`: none), while `pea (An)` follows 65 of them in their
    routine (KBD_$SET_KBD_TYPE 0xE7253E `movea.l A0,A2`, `pea (A2)` at
    0xE72550: the object passed by address); a later `nil` test is on the register (`movea.l A0,An;
    cmpa.w #0,An` 30 times, e.g. 0xE02F56 `movea.l A0,A3; cmpa.w
    #0,A3`), a dereference too (12 right after, none through A0);
  - of the 28 locals stored to their frame slot instead (`move.l
    A0,(-k,A6)`, 34 stores; wrappers' result slots left out), 8 (12
    stores) are pushed by value later (AST_$RESERVE 0xE067BA, `aote` passed to
    ast_$lookup_aste; AST_$TRUNCATE 0xE05CC8; AST_$PURIFY 0xE056D8;
    VTOC_$LOOKUP 0xE39056; VTOCE_$TRUNCATE 0xE39F3A), 2 (4 stores) are
    reached by a nested routine (AST_$INVALIDATE 0xE0666A and the AST_ module-local
    0xE05214 at 0xE0528C, `ast/invalidate.c`,
    `ast/set_attribute_internal.c`), 5 hold a routine pointer that is
    called (NET_$OPEN 0xE5A1D8 and its siblings), the rest are not
    explained (DIR_ module-locals 0xE4DC74, 0xE53B02, 0xE53E12;
    0xE511B0);
  - the 48 `move.l A0,Dn` copies (34 into D0: the TTY_$K_* routines
    after TTY_$I_GET_DESC; 0xE1C3FE moves the D0 copy to A2 before the
    next call, 0xE52418 `move.l A0,D0; movea.l D0,A0`): those into D0
    of a local that no call follows are "Pointer locals held in D0"
    below; the rest are not reproduced beyond 12.5's register locals.
  `dpc` (`isel.rs`, `call_pointer_locals`) holds a pointer local in one
  virtual address register for the whole body when every assignment to
  it is a pointer function's value, the first reference is an
  assignment, and it is otherwise only dereferenced or read whole
  outside call arguments: never pushed by value, never addressed (bead
  domain-pascal-pe9), never reached by a nested routine (**Uncertain**
  beyond these sites).  Each assignment is `movea.l A0,An`; the
  allocator gives An, A2 and up when the local lives across a call
  (R3); a call whose arguments use it last holds it through that call
  in a body with branches too (12.4 R4(c): the 13 such copies are all
  callee-saved).  A local that no call follows: see "Pointer locals
  held in D0" below.  Conformance `tests/conformance/cal_shutdown.pas`
  (CAL_$SHUTDOWN 0xE3E714 `movea.l A0,A3`, `pea (A3)` at 0xE3E730);
  golden `results/pointer_locals`.
  **Pointer locals held in D0 (Verified at the sites; implemented as
  integer locals, "The integer reading" below, the default since bead
  domain-pascal-78n2; the D0 holding of pointer-typed locals retired;
  beads domain-pascal-dgb, -yzxr).**
  `tools/resultsurvey.py ptrhold` follows every call of an A0 function
  along its fall-through path to the first instruction that reads A0
  (not only the next one, as `callers` does): `movea.l A0,An` next 105
  and later 3, `move.l A0,D0` next 34, `move.l A0,Dn` (D1..D6) 14,
  `movea.l A0,A1` 3, A0 read in place next 78 and later 28.  Of the 34
  D0 copies, D0 is next read by `movea.l D0,A0` 17 times (15 TTY_$K_*
  routines after TTY_$I_GET_DESC, e.g. TTY_$K_INQ_FUNC_ENABLED 0xE6763C
  / 0xE67642; RIP_$UPDATE_INT 0xE159B6, after a `bne.s` on the copy's
  flags, a `nil` test; the DIR_ module-local 0xE52394 at 0xE52418, right
  after the copy), by `movea.l D0,An` with An callee-saved 14 times
  (TTY_$K_FLUSH_INPUT 0xE1C0AE `movea.l D0,A2`, a call follows; the
  SIO_$K_* routines), and otherwise 3 times (the result variable of
  0xE6C21A; arithmetic, 0xE40186, 0xE51ACA).  So a pointer local that
  no call follows (the TTY ones assigned once, `tty := tty_$i_get_desc
  (line, status)`, and only dereferenced; ../source's TTY Pascal) is
  copied to D0 right after the call and moved to A0 by the first
  statement that dereferences it:
  - as that statement's first instruction (TTY_$K_INQ_INPUT_FLAGS
    0xE676DA `movea.l D0,A0; movea.l (0xc,A6),A1; move.l (0x14,A0),(A1)`;
    TTY_$K_SET_SESSION_ID 0xE679B0, the destination's base;
    TTY_$K_SET_DELAY 0xE67A40; TTY_$K_INQ_PGROUP 0xE6796C);
  - in an `if` whose two parts dereference it, just before the
    condition's branch, after the condition's slot load, which takes A1
    (TTY_$K_SET_INPUT_FLAG 0xE67684..0xE6768C `movea.l (0x10,A6),A1;
    movea.l D0,A0; tst.b (A1); bpl.b`; TTY_$K_SET_OUTPUT_FLAG,
    TTY_$K_SET_ECHO_FLAG; TTY_$K_ENABLE_FUNC 0xE675DC..0xE675E2 the
    same with A0 and A2);
  - later dereferences share A0 (0xE6C24E..0xE6C284, across
    statements), and D0 stays the local's up to the last of them,
    though nothing reads it there (TTY_$K_SET_INPUT_FLAG builds its set
    in D1 and D2, saving D2, with D0 free; TTY_$K_SET_DELAY 0xE67A42
    `move.w (A2),D1`; 0xE5243E `move.w (22,A0),D1`).
  The 3 A1 copies are locals whose range has D0 and D1 busy (0xE4D7FC
  `movea.l A0,A1; tst.w (10,A1)`, D0 and D1 written at 0xE4D806 and
  0xE4D812; 0xE528CE; 0xE09C5E).  Against the D0 sites, 3 locals stay in
  A0 (TTY_$I_INQ_RAW 0xE67414 `move.b (1241,A0),(A1)` after `tst.l
  (A2); bne.s`; TTY_$K_INQ_FLAGS 0xE674BA; KBD_$INQ_KBD_TYPE 0xE72584,
  D0 busy there).  TTY_$I_INQ_RAW has the code around it of
  TTY_$K_INQ_INPUT_FLAGS instruction for instruction (call, status
  test, slot load into A1, one dereference); it differs only in being
  `val_param` (the line pushed from (8,A6)) and in its frame, `link
  #-4` against the K routines' `link #-8`, and neither decides alone
  (the DIR_ module-local 0xE52394, a D0 site, takes its arguments by
  value).  TTY_$K_INQ_FLAGS is not the same shape: a statement that
  does not dereference the local comes first (`clr.w (A1)`, 0xE674B8)
  and the first dereference is an `if`'s condition (`btst
  #4,(9,A0)`, 0xE674BA), which dpc refuses.  ../source's Pascal for
  both is the same as for the D0 routines, so what decides is not seen
  (a `with` statement in the D0 routines would
  explain the separate D0 variable and A0 base, and the 4 frame bytes
  nothing addresses, `link #-8`, TTY_$K_FLUSH_INPUT's TODO in
  ../source; but TTY_$K_SET_FLAG and TTY_$K_INQ_FLAGS have those bytes
  too: Uncertain).  The 14 locals copied from D0 to a callee-saved
  register where a call follows (`tools/resultsurvey.py parkd0`:
  TTY_$K_FLUSH_INPUT 0xE1C0A8, _FLUSH_OUTPUT, _SIMULATE_TERMINAL_INPUT,
  _GET, _RESET, _SET_FUNC_CHAR, _ENABLE_FUNC, _SET_INPUT_BREAK_MODE,
  _DRAIN_OUTPUT, _PUT; SIO_$K_TIMED_BREAK, _SIGNAL_WAIT, _SET_PARAM,
  _INQ_PARAM; D0 next read by `movea.l D0,A2` 12 times, A3 once, A4
  once) are against 105 `movea.l A0,An` right after the call elsewhere.
  **No rule found (bead domain-pascal-bks):** the callee nearly decides
  but not quite: all 14 sites, and 15 of the 17 `->A0` ones below, follow
  a call of OS_TERM's TTY_$I_GET_DESC 0xE66738 or SIO_$I_GET_DESC
  0xE667C6, and no call of any other routine is followed by this form;
  but of the 33 calls of those two, 4 do not park, and they are exactly
  the run 0xE673AA..0xE674D2 (TTY_$I_SET_RAW 0xE673CC and TTY_$K_SET_FLAG
  0xE67446 `movea.l A0,An`; TTY_$I_INQ_RAW 0xE67414 and TTY_$K_INQ_FLAGS
  0xE674BA keep A0; the routine 0xE4B340, whose callers are all in DIR,
  has 2 D0 callers, 0xE518BC and 0xE52394, among its 35 call sites
  whose first read of A0 is the next instruction).  A lead (review of
  bks), now tested, "The integer reading" below: the parking callers
  may hold the result in a local of an integer type (a function declared
  with an integer result and `a0_return`, ref 5.5.11, or a type transfer
  at the assignment), turned into a pointer by a type transfer where it
  is dereferenced (`movea.l D0,An`), and the four others a pointer-typed
  local.  Nor the status test that follows
  (of the calls followed by `tst.l (An); b<cc>` on an `out` status, 14
  park and about 25 copy straight to An: AREA_$ASSOC 0xE096D4, KBD_$PUT
  0xE1CEBE, VTOC_$MOUNT 0xE385FC, FM_$READ 0xE3A3E2, CAL_$SHUTDOWN
  0xE3E70A, KBD_$SET_KBD_TYPE 0xE72536, ...); nor whether the register
  is free (TTY_$K_PUT 0xE67BF0 parks with A2 unused before; TTY_$K_FLUSH_INPUT
  parks with A2 the status's); nor the position in the module (in TTY's
  .TEXT segment the four routines TTY_$I_SET_RAW .. TTY_$K_INQ_FLAGS
  0xE673AA..0xE674D2 do not park, those on both sides do, so no one
  switch per segment as for A5, section 4, or loops, section 7).  The
  form is the same at all 14: `move.l A0,D0` right after the call's pop;
  `movea.l D0,An` once the status test has branched, as the first
  instruction of the next statement (TTY_$K_FLUSH_INPUT 0xE1C0AE, before
  the spin lock's push; TTY_$K_RESET 0xE6730A; TTY_$K_SET_INPUT_BREAK_MODE
  0xE6784C) or after that statement's slot load or pad (TTY_$K_ENABLE_FUNC
  0xE675E0, an `if` whose two parts use it; SIO_$K_SET_PARAM 0xE680DA;
  SIO_$K_TIMED_BREAK 0xE67F0E), except TTY_$K_SET_FUNC_CHAR 0xE67516 and
  TTY_$K_INQ_FUNC_CHAR 0xE6759E (A0), which move it in the `else` part of the next `if`,
  the first that uses it; An is the lowest callee-saved register free from
  there (the dead status's A2 at TTY_$K_FLUSH_INPUT, TTY_$K_RESET,
  TTY_$K_ENABLE_FUNC; A2 beside a live A3 at TTY_$K_PUT; A4 at
  SIO_$K_SET_PARAM with A2 and A3 live); D0 is the local's up to its last
  use before the next call (TTY_$K_ENABLE_FUNC builds its masks in D1/D2)
  and free after it (TTY_$K_FLUSH_INPUT 0xE1C0BE `move.w D0,(-6,A6)`).
  **Retired** (bead domain-pascal-78n2): `dpc --park-d0=R,...` held a
  pointer local in D0 where a call follows too in the routines it named,
  `tools/srcsurvey.py` giving it to the routines
  `resultsurvey.park_d0_routines` lists (TTY_$K_FLUSH_INPUT and _OUTPUT
  conformed so).  The same code now comes from an integer local with no
  option ("The integer reading" below; conformance
  `tests/conformance/tty_k_flush_int.pas`, exec test
  `tests/exec/park_int_calls.pas`, formerly `park_d0.pas` with pointer
  locals and the option).  `tools/resultsurvey.py parkd0` is kept as
  evidence; nothing gives dpc options from it.
  (Also fixed with it: a pointer local was taken as addressed when a
  smaller local lay just below its slot, `t: integer` at -6 under the
  pointer at -4, so TTY_$K_FLUSH_INPUT's `tty` stayed in its frame slot.)
  **The integer reading (bead domain-pascal-yzxr; fits every site
  compiled, Reported: the original declarations are not known; dpc's
  only D0 form since bead domain-pascal-78n2, and ../source's
  translation since its c8f980f8).**  If
  TTY_$I_GET_DESC is declared `integer32; options(a0_return)` (ref
  5.5.11, p. 5-20: "allowed on any function signature", the result in
  A0), the D0 form and the A0 forms of its 33 callers follow from the
  type of each caller's local alone, with no per-routine option: an
  integer local is a data value, so the result goes to a data register
  (`move.l A0,D0`), and the type transfer to the pointer type at the
  first dereference gives `movea.l D0,An`, which later dereferences
  share; a pointer-typed local assigned the transferred value
  (`tty := tty_$desc_ptr_t(tty_$i_get_desc(line, status))`) is the
  call's A0 as for a pointer function: copied to An where a call follows
  (`movea.l A0,A2`, TTY_$I_SET_RAW 0xE673CC, TTY_$K_SET_FLAG 0xE67446)
  or left in A0 (TTY_$I_INQ_RAW 0xE67414, TTY_$K_INQ_FLAGS 0xE674BA).
  The run 0xE673AA..0xE674D2 is then the four routines whose author
  used a pointer local, and the "no rule found" above dissolves for
  OS_TERM's callers.
  - *dpc* (`isel.rs`, `integer_call_local`, `Park`; no option): a
    4-byte integer local whose every assignment is a call with its result
    in A0, assigned once by a statement of the routine's own statement
    part, and whose every read is a type transfer to a pointer that is
    the base of a dereference (at least one), is parked (the D0 form
    above) whether a call follows or not.  Any other integer local
    assigned an A0 result is as before: read only whole, an ordinary
    register-held local (12.5, `move.l A0,D0` and the reads from D0);
    read whole and dereferenced, or assigned twice, its frame slot
    (`move.l A0,(-4,A6)`).  So data-register integer locals in general
    are not changed.  A same-width type transfer of a call whose result
    is in A0 is that call retyped (`dpc-ir` `reinterpret`), so
    `p := ptr_t(f(...))` is `movea.l A0,An`.  A pointer-typed local
    that no call follows stays in A0 itself, the copy dropped
    (TTY_$I_INQ_RAW's `out` slot then in A1), refused when anything
    writes A0 before its last use (`Selector::a0_locals`).  This was
    `dpc --no-park-pointer-locals` (manifest token
    `no-park-pointer-locals`) until bead domain-pascal-78n2 made it the
    default and removed the option, `--park-d0` and the D0 holding of
    pointer locals with it (the owner's decision).
  - *Test* (options of the time, since removed: `tools/srcsurvey.py --source <clone> --no-park-d0
    --dpc-opt=--no-park-pointer-locals`, on a scratch clone of
    ../source c871dbaa with TTY_$I_GET_DESC so declared in tty.ins.pas,
    the D0 routines' `tty` locals `integer32` with `tty_$desc_ptr_t(tty)^`
    at every dereference, and TTY_$I_SET_RAW, TTY_$I_INQ_RAW,
    TTY_$K_SET_FLAG and TTY_$K_INQ_FLAGS converting at the assignment):
    no `--park-d0` given, TTY 23 OK / 4 MISMATCH / 32 STUB against 22 / 4
    / 33 for the present translation with `--park-d0`.  Per routine:
    - D0 where a call follows: TTY_$K_FLUSH_INPUT, TTY_$K_FLUSH_OUTPUT
      OK; TTY_$K_ENABLE_FUNC MISMATCH as before (A2/A3 only, bead
      domain-pascal-80rr); TTY_$K_SIMULATE_TERMINAL_INPUT, _GET,
      _RESET, _SET_FUNC_CHAR, _SET_INPUT_BREAK_MODE, _DRAIN_OUTPUT,
      _PUT stubbed for the refusals they had before (12.4 R4(b),
      adjacent stores, `in_range`, an 8-byte record assignment, an array
      element, a 4-byte argument to a word parameter), except that
      TTY_$K_GET stopped first at the move's placement (its first
      dereference is inside `if status.all = status_$ok then begin`,
      `movea.l D0,A2` at 0xE1C40A the then part's first instruction;
      before, at a computed boolean store), until bead
      domain-pascal-1j61 placed the move there ("Placement inside an
      `if`" below): it stopped at the computed boolean store again
      (k_get.pas `break_mode_flag := ... = tty_$break_on_eol`), then,
      after bead domain-pascal-k1sv, at a branch to a `bra` (7.1; threaded
      since bead domain-pascal-o6p, it stops at register pressure now).  Cut down to the statements
      up to the first call after the status test, TTY_$K_RESET and
      TTY_$K_SIMULATE_TERMINAL_INPUT give the image's `move.l A0,D0;
      tst.l (An); bne; movea.l D0,A2` (0xE67302..0xE6730A,
      0xE1C16C..0xE1C174); the rest of such cut-down units differs where
      the cut changes the register pressure (not evidence either way);
    - D0 where none follows: the 9 compiled ones OK
      (TTY_$K_INQ_FUNC_ENABLED .. TTY_$K_INQ_ECHO_FLAGS,
      TTY_$K_SET_SESSION_ID, TTY_$K_INQ_SESSION_ID); TTY_$K_INQ_FUNC_CHAR,
      _INQ_INPUT_BREAK_MODE, _SET_PGROUP, _INQ_PGROUP, _SET_DELAY,
      _INQ_DELAY stubbed as before;
    - not parked: TTY_$I_SET_RAW OK; TTY_$I_INQ_RAW OK (MISMATCH with
      the translation of the time, the known divergence above);
      TTY_$K_INQ_FLAGS compiles (refused before) and differs only
      elsewhere: the `out` slot loaded at entry into A2 where the image
      loads it after the status test into A1 (0xE674B4, bead
      domain-pascal-udmb) (the member test was `move.w (8,A0),D0; btst
      #4,D0` where the image tests the byte, `btst #4,(9,A0)`, until bead
      domain-pascal-578, 5.1); TTY_$K_SET_FLAG stubbed as before (a branch to a `bra`;
      threaded since bead domain-pascal-o6p, it stops at a slot load
      after a branch, 12.4 R4(b), now).
    The 4 SIO_$K_* routines that park where a call follows have no
    Pascal in ../source yet: not tested.  Conformance
    `tests/conformance/tty_k_flags_int.pas`, `tty_k_flush_int.pas`,
    `tty_i_raw_int.pas`; golden `results/integer_parked`; driver tests
    `integer_a0_result_local_is_parked`,
    `integer_a0_result_local_read_whole_is_not_parked`,
    `transferred_a0_result_in_pointer_locals`; exec tests
    `tests/exec/park_int.pas`, `park_int_edge.pas` (review: a parked
    local dereferenced in a loop with a call, two parked locals, read
    whole, read uplevel, a D0 function's value transferred, an A0-held
    local in a loop).  Also from the review: an integer local of this
    shape that HEAD kept in its frame slot is now refused where the
    parked form's placement is not seen (an M$ARITH call before the
    first dereference, D0 busy; a call in the status test's `then`
    part), as for pointer locals.
  - *KBD.*  KBD_$GET_DESC 0xE1AA26 has 4 callers (`jsr abs.l`, a byte
    search): KBD_$PUT 0xE1CEC6, KBD_$GET_CHAR_AND_MODE 0xE724E0,
    KBD_$SET_KBD_TYPE 0xE7253E copy A0 to A2 right after the call (a
    call follows), and KBD_$INQ_KBD_TYPE keeps A0 (0xE72584 `move.w
    (68,A0),D0`, no call follows); none parks.  So under the reading all
    four hold a pointer-typed local (KBD_$GET_DESC a pointer function, or
    the transfer at the assignment), and KBD_$INQ_KBD_TYPE needs no "D0
    busy" explanation: it is the TTY_$I_INQ_RAW form.  Not compiled (its
    ../source Pascal stops at a `for` loop's bounds).
  - *R1b* (12.4).  Five of R1b's six sites are callers of the two
    descriptor functions (TTY_$I_SET_RAW, TTY_$K_SET_FLAG, KBD_$PUT,
    KBD_$SET_KBD_TYPE, KBD_$GET_CHAR_AND_MODE), all on the pointer side
    of the split.  The reading explains why no parking routine is an R1b
    site: there the address register is written after the status test,
    when the status's register is dead (TTY_$K_FLUSH_INPUT reuses A2), so
    the two never interfere.  It does not explain the order R1b
    describes (the call result below the status parameter): R1b stays
    as it is.
  - *Not covered*: the D0 copies after other callees (RIP_$UPDATE_INT
    0xE159B6, a `nil` test on the copy's flags, bead
    domain-pascal-avry; the DIR_ callers 0xE518BC and 0xE52394 of the
    module-local 0xE4B340, which has 35 callers; the 3 A1 copies,
    0xE4D7FC, 0xE528CE, 0xE09C5E) fit the same reading but are not
    tested.
  - *Default (bead domain-pascal-78n2, the owner's decision).*  ../source
    c8f980f8 adopted the reading: TTY_$I_GET_DESC `integer32;
    options(extern, val_param, a0_return)` in tty.ins.pas, the parking
    callers with `integer32` locals and `tty_$desc_ptr_t(tty)^` at each
    dereference, the four routines 0xE673AA..0xE674D2 with pointer
    locals converted at the assignment.  `tools/srcsurvey.py` at
    c8f980f8 with `--no-park-d0 --dpc-opt=--no-park-pointer-locals`
    (the behaviour now the default) gave no regression against the
    baseline (OK 31, CODE-OK 4, MISMATCH 4, STUB 57, BLOCKED 8;
    TTY_$I_INQ_RAW MISMATCH to OK, TTY_$K_INQ_FLAGS STUB to MISMATCH).
    So the A0 form for pointer locals is the default and the D0 holding
    of pointer locals (by default where no call follows, with
    `--park-d0` where one does) is retired, the options removed.  The
    conformance unit `tty_k_flags.pas` (pointer locals held in D0) was
    dropped for `tty_k_flags_int.pas`, the same routines in the integer
    form; `tty_k_session_id.pas` and `tty_sections.pas` were rewritten
    to the integer form, as ../source c8f980f8 has them
    (domain_os/tty/k_pgroup.pas).
  **Implemented** (`isel.rs`, `Park`, `Selector::park_home`,
  `integer_call_local`, `parkable_pointer_local`): an integer local of
  the reading's shape (assigned exactly once from a call whose result is
  in A0, by a statement of the routine's own statement part, and
  otherwise only dereferenced through a type transfer) is held in a data
  register: `move.l A0,Dn` at the assignment, `movea.l Dn,An` before the
  first statement that refers to it or calls a routine after it (first
  in a simple statement; for an `if` whose condition is one test not
  reading it and whose two parts both dereference it, moved down to the
  branch after allocation), An the base of every later dereference, Dn
  held to the last of them or to the next call.  Where no call follows,
  Dn must come out D0 and An A0, else refused (A1 in the image where D0
  is busy, not modelled).  Inside an `if`, see "Placement inside an
  `if`" below.  Refused: the first dereference inside a loop, `case` or
  nested statement, and the `if` shapes that paragraph leaves out.  A
  pointer local of `call_pointer_locals` that
  allocation puts in A0 or A1 (live across no call) stays in A0 (the
  TTY_$I_INQ_RAW / TTY_$K_INQ_FLAGS form), refused when assigned twice,
  inside a statement, read whole (a `nil` test: RIP_$UPDATE_INT
  branches on the `move.l A0,D0` flags) or with A0 written before its
  last use.  Until bead domain-pascal-78n2 such a pointer local was
  held in D0 in the integer form's place (retired, above).  Goldens
  `results/integer_parked_forms` (formerly `results/pointer_parked`, its
  Pascal rewritten to integer locals with the expected output unchanged)
  and `results/integer_parked`; conformance
  `tests/conformance/tty_k_flags_int.pas` (TTY_$K_INQ_FUNC_ENABLED ..
  TTY_$K_INQ_ECHO_FLAGS 0xE67618..0xE6781E) and `tty_k_session_id.pas`
  (0xE67986..0xE67A02); driver tests
  `parked_integer_local_moves_to_a0_where_dereferenced`,
  `parked_integer_local_refusals`,
  `pointer_local_that_no_call_follows_stays_in_a0`.

  **Placement inside an `if` (bead domain-pascal-1j61; Verified at the
  sites below: 1 site for the condition form, 1 for the `then` part
  form; the `else` part form has no site of its own, see below).**  Survey (scratch
  script over `tools/resultsurvey.py`'s `ptrhold_sites`, the 31 `move.l
  A0,D0` copies next read by `movea.l D0,An`): every one of the 31
  routines has exactly one `movea.l D0,An`, so the move is never
  duplicated per branch.  29 are callers of TTY_$I_GET_DESC or
  SIO_$I_GET_DESC whose status test (`tst.l; bne`) comes between the
  copy and the move, and in all 29 the move is after that test's branch,
  never before it.  28 of those branches go to the register restore
  (`if status.all <> status_$ok then return`, or a `then` part running
  to the end: the same code); one, TTY_$K_GET 0xE1C406 `bne.w
  0xE1C614`, goes to the result assignment (`move.w D3,D0`) after the
  `if`, so the routine's statements are the `then` part of `if
  status.all = status_$ok then begin ... end`, and the move is that
  part's first instruction (0xE1C40A `movea.l D0,A2`, the first
  statement `pea (A2); bsr.w tty_$i_lock`).  Of the 28, two move it at
  the target of a guard's `bls.b`, past a `then` part ending in `bra.b`
  to the epilogue (`if ... then begin ...; return end`):
  TTY_$K_INQ_FUNC_CHAR 0xE6759E into an assignment, and
  TTY_$K_SET_FUNC_CHAR 0xE67516 as the first instruction of an `if`
  whose condition dereferences it (`movea.l D0,A2; move.l (0x20,A2),D2;
  btst.l D1,D2; beq.b`, `if func in tty^.func_enabled then`), before
  the condition's own code.  4 hoist it before an `if`'s branch, both
  parts dereferencing it (above).  The other 2 copies (RIP_$UPDATE_INT
  0xE159C2, the DIR_ module-local 0xE52394 at 0xE5241A) follow other
  callees and are not this form.  So, in dpc (`Selector::park_home`):
  - an `if` whose condition is one test (a compare or a boolean) that
    reads the local and calls nothing: the move is the `if`'s first
    instruction (TTY_$K_SET_FUNC_CHAR 0xE67516);
  - an `if` whose condition does not read the local and calls nothing,
    with no later statement referring to it, and either a `then` part
    referring to it and no `else` part (TTY_$K_GET 0xE1C40A), or an
    `else` part referring to it after a `then` part that ends in
    `return` and neither refers to it nor calls: the move goes down
    into the referring part, placed there by the same rules as in the
    statement part.  The `else` form is an inference, not a site: dpc
    emits for it the same code as for `if ... then begin ...; return
    end` followed by the statement, which is TTY_$K_SET_FUNC_CHAR's and
    TTY_$K_INQ_FUNC_CHAR's guard; no image site tells the two source
    shapes apart.
  Not seen, so refused: a condition reading it that is not one test
  (`and then`, `or else`, `not`, ...); a `then` part with an `else`
  part (even one ending in `return`); an `else` part after a `then`
  part falling through to the join (`bra`), or calling; a reference
  after the `if`; a call in the condition.  The `then` form accepts a
  condition that is not one test (K_GET's is `tst.l; bne`): the move
  is in the part, after all of the condition's code, and D0 is held
  over that code as over any statement before the move.  Cut down to the in_range guard (`func > 31` for
  `not in_range(func)`) and without the refused byte store
  `func_chars[func] := ch`, ../source's TTY_$K_SET_FUNC_CHAR compiles
  with `movea.l D0,A3` first in the condition's `if` (A3 where the image
  has A2: dpc gives func A2 and status A3, the image the reverse; and
  func reloaded where the image keeps it in D1, bead domain-pascal-rvv9:
  not this rule).  Golden
  `results/integer_parked_if`; exec tests `tests/exec/park_int_if.pas`
  (both sides of each test) and `tests/exec/park_int_if_edge.pas` (D0
  held over a compound condition, statements, a loop and a nested `if`
  before the move; an out parameter's register after it); driver tests
  `parked_integer_local_in_one_if_part`,
  `parked_integer_local_in_if_condition`,
  `parked_integer_local_refusals`.

  A local kept in its frame slot (pushed by value, addressed, reached by
  a nested routine, or assigned something other than a call) is stored
  with `move.l A0,(-k,A6)`, and then:
  - `= nil` / `<> nil` right after the store branches on the store's
    flags (AST_$RESERVE 0xE067BA `move.l A0,(-28,A6); bne.s`, 0xE067DA;
    AST_$TRUNCATE 0xE05CC8, 0xE05CEA; AST_$PURIFY 0xE056D8 `beq.w`),
    except for a local a nested routine reaches, which is tested again
    with `cmpa.w #0,A0` (AST_$INVALIDATE 0xE0666A, 0xE0668C; 0xE05214 at
    0xE0528C, 0xE052B0).  Over all stores of a register into memory
    followed by a test of the same value (`tools/resultsurvey.py
    storetest`): 50 branch on the store's flags
    (9 of them stores of A0), 9 test the register again (those 4; also
    0xE554A8 `tst.w D0` at a join), 11 test the memory (none of A0).
    Not seen, refused: such a test while A0 still holds the local but
    not right after its store;
  - a dereference while A0 still holds the local on every path uses A0
    as the base: right after the store (DIR_ 0xE4DC78 `tst.w (A0)`,
    0xE53B06, 0xE53E16), on the `nil` test's branch path (0xE067E2,
    0xE066A8 `bset #6,(191,A0)`), after the join of `p := f; if p = nil
    then p := g` (AST_$RESERVE 0xE067E8 `tst.b (185,A0)`, AST_$TRUNCATE
    0xE05D1E), but for a local a nested routine reaches the slot is
    reloaded at such a join (AST_$INVALIDATE 0xE066AE `movea.l
    (-20,A6),A0`; 0xE05214 at 0xE052C4 into A1).  `dpc`
    (`Selector::a0_holds`, a must-analysis over the instructions so far:
    a store or reload of the slot via A0 makes it, a call, a write of A0,
    a possible write of the slot or a frame address taken ends it, and
    for a nested-reached local any join) uses A0 there, outside loops.
    Pushes of the local still read the slot (no site seen either way;
    A0 is never pushed, above).
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
  values are rejected (not catalogued).
- **Computed booleans stored and pushed (survey Verified; implemented,
  bead domain-pascal-k1sv).**  `tools/boolsurvey.py summary`, kind
  `scc`: of the 462 `s<cc>` other than `st`/`sf` in compiled code, none
  writes memory; 56 go into a data register followed by `move.b
  Dn,<mem>`, 35 by `move.b Dn,-(SP)` (all 35 with the compare or test
  right before the `s<cc>`), 4 by `move.b Dn,Dm`.  So a comparison, `odd`
  or a set member stored is formed by the rule above and stored with
  `move.b`: TTY_$K_GET 0xE1C506 `cmp.w (A3),D3; seq D1; move.b
  D1,(-0x1e,A6)`; TTY_$K_RESET 0xE67316 `btst.b #1,(0x17,A2); sne D0;
  move.b D0,(-8,A6)`; PCHIST_$CNTL 0xE5CF30 `cmpi.w #3,(A0); seq D1;
  move.b D1,(294,A5)`; ASKNODE_$INTERNET_INFO 0xE650CC `cmpi.w
  #1,abs.l; sgt D3; move.b D3,(10,A1)`.  Passed by value, it is formed
  right before its push, in push order (TTY_$K_GET 0xE1C54E `btst.b
  #1,(0x17,A2); sne D1; move.b D1,-(SP)`; AREA_$DEACTIVATE_ASTE
  0xE0A196 `tst.l D0; seq D1; move.b D1,-(SP)`; VTOCE_$TRUNCATE
  0xE3A246).  Assigned to a register-held local (12.5), the `s<cc>` goes
  into the local's register when the compare computes nothing into a
  data register (TTY_$K_GET 0xE1C42E `tst.w (0x38,A2); seq D5`,
  break_mode_flag; a set member, 0xE1C41E `btst.b #1,(1,A4); sne D4`);
  when it does (a loaded word, an index), whether the `s<cc>` goes into
  that register and is moved, or everything into the local's register,
  is not seen: refused.  Tests: `tests/golden/stores/bool_stores`,
  `tests/exec/bool_stores.pas`.
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
  one statement.  Results in A0: "Callers of pointer functions" above.  The tail wrapper above
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
    - The OS_TERM .TEXT segment 0xE66738 (TTY_$I_GET_DESC ..
      TERM_$INQUIRE), right before TTY's 0xE671DC, is `nnnnnnn`: none
      of its 7 routines loads A5, although OS_TERM has a .DATA block
      (0xE82428, TERM_$PCHIST_FLAG, which no instruction of the segment
      addresses).  That is used mode with no users, so the segment
      shows no mode; OS_TERM's WIRED_PROC segment 0xE1A9D4 does load A5
      (`nn(S)n`).  In TTY's segment TTY_$I_DXM_SIGNAL and
      TTY_$I_ENABLE_CRASH_FUNC 0xE67292 load no A5; the module-local
      0xE6726E loads it (`lea 0xE8242C,A5`, 0xE67274) and uses it
      (`move.l (36,A5),-(SP)`, 0xE67280) for the module-local 0xE6720E,
      which inherits it (`lea (A5),A1`, 0xE67226); every named routine
      from TTY_$K_RESET 0xE672DE on loads A5, and of those only
      TTY_$K_SET_INPUT_BREAK_MODE uses it (`(40,A5)`, 0xE67860,
      0xE6788C) (lead: ../source pascal-translation-notes finding 15,
      which reads OS_TERM as "never"; the image does not show a mode
      difference between the two modules).
    - In used mode, 3 routines address their own block absolutely
      instead of loading A5: AUDIT_$SUSPEND 0xE70DB6 and AUDIT_$RESUME
      0xE70DD6 (`lea 0xE854D8,A0`), REM_NAME_$REGISTER_SERVER 0xE4A4AE
      (`lea 0xE7DBB8,A0`).  Not explained; size is not it, as the
      neighbouring AUDIT_$IS_PROCESS_AUDITED 0xE70D94 reaches the same
      array (`(-2,A5,D0.w)`) after loading A5.  (DISK_$DIAG_IO 0xE6BC18,
      listed here before, reaches 0xE7ACCA in the *other* DISK_ unit's
      block 0xE7A1CC; its own segment loads 0xE826C4.  LOG_$INIT
      0xE30054 pushes the LOG_ address from the OS_INIT unit, which has
      no LOG_ block.  DISK_$DIAG_IO is the `extern` case; LOG_$INIT is
      LOG_'s own code in OS_INIT_PROC, so it may equally be the
      module's block of another data section (section 11, "Data per
      section"); the image does not tell them apart.  The survey of
      section 11, "Data per section", finds `move.l #block,-(SP)` the
      only form of the first variable of a module's block of another
      data section passed by address, 12 sites, so `dpc` emits it so.)
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
      `--a5-always` list is not a use, nor is a call through a routine
      pointer (section 1.6: the callee sets its own A5; a pointer read
      off A5 is a use).  An exported routine that does not
      use the data loads nothing (PROC1_$TST_LOCK), instead of the old
      "not settled" rejection.  `internal` routines never load A5.  A5
      is set as the first instruction after the register save and saved
      like any callee-saved register (section 12.1): `pea (A5)` alone,
      in the `movem` otherwise.
    - `--a5-always-from=ROUTINE,...`: from ROUTINE on, within its code
      section (one routine per section, as `--loop-align-off-from`,
      section 7), every exported routine with a frame loads A5.  A
      bare-`rts` routine never does (section 2).  A routine's A5 is the
      block of its data section (section 11).
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
| subrange | size of base type (ref 3.8.1) | 2 bytes up to `0..65535` (ref 5.3.4.1 p. 5-13: `0..65535` and integer are "both 16-bit integers"; Table 7-2 p. 7-30: `unsigned short`); 4 bytes once more than 16 bits are needed (3.4 p. 3-10); in packed records the bits of the extreme values (5.3) |
| `boolean` | 1 | true = 0xFF, false = 0x00 (ref 3.5.3); tested with `tst.b`/`bmi`/`bpl`, i.e. **signed** byte |
| `char` | 1 | |
| enumeration | 2 | ordinal in a word (ref 3.7.1) |
| pointer, `univ_ptr` | 4 | |
| set | ⌈(max ordinal + 1)/16⌉ words | ref 3.9.3; see 5.1.  The high end of the base type decides, whatever the low end: `set of 80..170` is 11 words, `set of 'a'..'z'` 8 words (p. 3-17); checked by `crates/dpc-sema/src/tests/layout.rs` |
| record | ref 3.10.5 (alignment rules: "natural" vs default word alignment; `aligned`/`unaligned` records; attributes and directives: section 5.4) | **Reported: default word alignment** (ref 3.10.5.5: fields longer than a byte on word boundaries, record size even).  `route_$port_stats_t` (`../source/domain_os/route/route.h`) has longwords at 0x2 and 0x6, which natural alignment cannot produce: ROUTE_RTWIRED code `addq.l #1,(0x6,A2)` 0xE87660, `addq.l #1,(0xA,A2,D1.l)` 0xE8765A (seen; that A2 is the record start is the C's claim).  `proc1_t` (`proc1/proc1.h`) also matches word rules.  Field offsets in `_Static_assert`s remain ground truth |
| arrays | 1-based is common in the kernel; element N at base + (N-lo)*size | |

**Signedness (implemented in `dpc-ir`, `lower.rs` `sign_of`).**  Every IR
value carries a width and a sign (`dpc_ir::Scalar`).  Verified for the
2-byte `0..65535`-style subrange: 0xE17778 (C `uint16_t` parameters)
zero-extends with `clr.l D1; move.w D0,D1` (0xE177A6..0xE177AA), compares
with `cmpi.w #0x400,D1; bcc` (0xE1779A) and `bls` (0xE177BA), and halves
with `lsr.w #1` (0xE177D8).  `integer`/`integer32` signed (ref 3.2.1);
boolean signed (the `tst.b`/`bmi` tests above); char, pointers and sets
unsigned.

**A non-negative subrange stored in 4 bytes is unsigned (reference and
image Verified; implemented, bead domain-pascal-5j9).**  `0..65536` up to
`0..2147483647`, Apollo's own `linteger = 0..2147483647` ("positive 31
bit integer", `base.ins.pas`, section 0.1).  Reference: 3.4 p. 3-9
("large unsigned subranges (from 0..2^30 up to 0..2^31-1), you can
achieve much of the effect of having true unsigned types"; `u32 :
unsigned_32`, `0..2147483647`); 4.3.3 pp. 4-6..4-7 (PDF pp. 139-140,
checked: `>`, `>=`, `<`, `<=`, `min`, `max`, `div`, `mod` are unsigned
when both operands are unsigned run-time values or one is and the other
a constant in 0..2147483647, signed otherwise; the example `u_type =
0..2147483647`, `u_type(s32) DIV u32` "generates an unsigned divide");
"Max"/"Min" pp. 4-111, 4-113 ("The unsigned scalar data types are
non-negative subranges of integers, Boolean, character, and
enumerated").  Image: longwords that are neither pointers nor
zero-extended words compared, divided and multiplied unsigned:
PMAP_$PURIFIER_L 0xE13EB0..0xE13EB8 and 0xE13F1A..0xE13F22 `move.l
(4,An),Dn; cmp.l (32,An),Dn; bhi` (two working-set counts, `page_count`
and `ws_floor` of `ws_hdr_t` in `mmap/mmap.h`); 0xE1404E..0xE1405E
`moveq #11,D4; add.l MMAP_$WSL+0x70,D4` (WSL[3]'s `page_count`) divided
by 12 with `M$DIU$LLW`; AST_$GET_SEG_MAP 0xE06E10..0xE06E2E, two
run-time longwords by `M$DIU$LLL` and `M$OIU$LLL`, the remainder then
`lsr.l #8; lsr.l #2` (D4 is computed with a `sub.l` at 0xE06D2C, so its
sign comes from a declared type, not from the operation);
CAL_$DECODE_TIME 0xE68794 `M$DIU$LLW` by 60; PCHIST_$UNIX_PROFIL_ADDUPC
0xE5CFF6..0xE5D008 `cmp.l D2,D1; bcs`, `sub.l`, `M$MIU$LLL`; the PACCT
module-local's `cmpi.l #8192,D0; bcc` (0xE5A9E6, 5.3).  Addresses
compared as integers: EC2_$INIT 0xE42C68 `cmpi.l #1000,D0; bls`,
EC2_$READ 0xE42C86 `bls`, EC2_$ADVANCE 0xE42CBC `bhi`, PROC1_$FREE_STACK
0xE1512A `cmp.l (0xc3c,A5),D0; bls` (conformance unit `ec2_init.pas`,
`linteger(addr(ec)) > 1000`).  Survey (`tools/cmpsurvey.py summary`,
kind `ord`): after a longword compare 269 unsigned ordered branches
(`cmpi.l` 79, `cmp.l` with a memory source 126, with a register 53,
`cmp.l #` 3, `cmpa.l` 8) against 153 signed (34, 38, 81); the image does
not show which of the unsigned ones are pointers.  In `dpc-ir` such a
subrange is `Sign::Unsigned` without `Scalar::pointer` (a pointer is
also an unsigned longword, but held in address registers and returned in
A0: the two are told apart by that flag).

**Uncertain:** enumerations (taken as unsigned, as the "Max" entry says)
and non-negative `integer16` subranges such as `0..100` (taken as signed,
like their host).  The "Max"/"Min" entries call every non-negative
subrange unsigned, which would make `x < 10` on a `0..100` word `bcs`;
but the image cannot tell a `0..100` word from a `0..65535` one (after a
word compare: 518 unsigned ordered branches, 336 signed, `ord`), and
what the original did for such a word against an `integer` (both
widened to longwords, as for `0..65535`, or a word compare, as the
range allows) is not catalogued (bead domain-pascal-4x2).  Booleans, which
the same entries call unsigned, are `Scalar::BOOLEAN` (signed) for the
`tst.b; bmi` tests; no ordered compare of booleans is supported.
One enumeration compare is catalogued in the image (no survey for
others), and it is unsigned (Verified
at two sites): TTY_$K_SET_FUNC_CHAR 0xE67500..0xE6750C `move.w (A3),D1;
cmpi.w #31,D1; bhi.s; cmpi.w #17,D1; bls.s` on a tty_$func_t word
(Apollo's customer tty.ins.pas, section 0.1; 31 its last ordinal), and
TTY_$K_INQ_FUNC_CHAR 0xE67586..0xE67592 the same (lead: ../source
pascal-translation-notes finding 9).  The first compare is implied by
the second; ../source `tty/k_func.pas` reads it as `not
in_range(func)` (10.1), which the image cannot tell from a compare
against the last enumerator.  Of the seven `cmpi.w #31,Dn` in the image
(also 0xE03B1A, 0xE03C5A, 0xE1A4B2, 0xE44A22, 0xE646C6) no other has
this pair.

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
  (`crates/dpc-m68k/src/isel.rs`, `set_member_reg`).  **Uncertain:**
  whether "address of the right operand before the value of the left
  one" is a general rule of the compiler's binary operators or specific
  to `in`; and the code for one-word sets held in a register (not seen
  yet; `dpc` rejects them).
- **Constant member of a one-word or one-longword set (Verified;
  implemented):** see "`in` on sets in memory, constant member" below:
  a set in memory is read only as far as the part holding bit k at the
  same bit number (the byte for k < 8, `btst #k,(d,An)`; the low word of
  a longword for 8..15); otherwise, and for a set held in a register,
  the set's value in a data register, then `btst #k` on it:
  REM_NAME_SERVER_LOCAL 0xE4A412..0xE4A41A `move.w (22,A0),D0; btst
  #13,D0; sne D0` (a boolean function, `sne` in the set's register);
  0xE031B2 `btst #14,D5; bne.s`.  `dpc` (`isel.rs`, `set_bit_test`)
  rejects a non-constant element on a word set (`btst Dn,Dm` takes the
  bit mod 32, not catalogued for words), and a constant element beyond
  the set's bits (`20 in s` for a word set would test a bit of the
  register above the loaded word; the value is false, ref 3.9.3, p.
  3-16, but the original's code for it is not catalogued).
- **Member 0 (or bit 0) as `lsr.b #1` (Verified at 9 sites; read as
  `odd`, Uncertain; implemented, bead domain-pascal-3xwx):** a value
  loaded whole into a data register, then `lsr.b #1,Dn` and the carry:
  0xE1B0F6 `move.l (12,A0),D1; lsr.b #1,D1; bcc.s` (bit 0 of TTY's
  output_flags, module-local 0xE1B00A); 0xE07B66 `move.w (46,A2),D0;
  lsr.b #1,D0; bcc.w`; 0xE08826, 0xE0905A, 0xE093CC, 0xE431EC, 0xE75820
  (`bcc`/`bcs`); as a value 0xE052FA and 0xE70E98 `lsr.b #1; scs` into
  another register.  These are all the `lsr.b #1,Dn` in the image, each
  right after a word or longword load (`tools/setsurvey.py onebit`).
  Against them, `btst #0` after a whole load 4 times: AST_$TRUNCATE
  0xE05C5E (the loaded word reused for `btst #1` next), DTTY_$INIT
  0xE34C4C (a device word read in two `if` parts, tested at the join,
  `sne D1`), PEB_$GET_STATUS 0xE5ADEA `move.l (A0),D0; btst #0,D0;
  beq`, TERM_$INQUIRE 0xE66E9E `btst #0,D0; seq D1`; and `btst #0` on a
  register that holds a variable or a value kept from before 32 times
  (FIM_$BUILD_DF 0xE0A6F6).  The fields read by `lsr.b #1` are tested
  for their other bits by `btst #k` after a whole load in the same
  routines (tty_$i_put_chars on output_flags; AREA_$COPY on its word at
  46, 0xE09098 `btst #1`; AUDIT's word at (168,A5), `btst #1` in
  AUDIT_$SERVER 0xE71132), so the source wrote bit 0 differently from
  the others.  **The reading taken:** `lsr.b #1` is `odd(x)` of an
  integer in memory, `btst #0` after a whole load is `(x & 1) <> 0`
  (the `&` of one bit, 7.1 "One bit"), and `odd` of a register-held
  value is `btst #0` (`lsr` would destroy the register; AUDIT_$LOG_EVENT_S
  tests its word at (168,A5) with `lsr.b #1` at 0xE70E98 and a
  register-held word with `btst #0` at 0xE70EFA, both read as the
  rounding idiom `if odd(n)`).  The image cannot show the source:
  **Uncertain**; `0 in s` is narrowed like any member (below), so it is
  not this form.  `dpc` (`isel.rs`, `odd_test`): `odd` of a word or
  longword in memory as `move.<s>; lsr.b #1; bcs`/`bcc` in a condition;
  as a value refused (`scs` into another register at both sites, the
  load in D3 at 0xE70E98: bead domain-pascal-uwfe); `odd` of a byte is
  refused by `dpc-ir`.  Golden `control/one_bit`; exec test `onebit`.
  (The earlier lead: ../source pascal-translation-notes finding 2; bead
  domain-pascal-ogzd.)
- **Set constants** sit in data, for example the two 32-byte `set of char`
  tables at the start of the OLD_DIR data segment 0xE7FD24 (Reported:
  `name-dir-subsystems.md`); initialised static sets in the routine's
  data section (the TTY WIRED_DATA `set of char` at 0xE2DDB4 and the
  longword masks at 0xE2DDD4, ../source `tty/i_word_erase.pas`,
  `tty/i_set_raw_mode.pas`; dpc's encoding of them is byte-identical,
  `tests/conformance/tty_wired_data.pas`, section 11 "Initial values").  A register-sized constant used as an
  operand is its bit mask in an immediate (`move.l #0x1ffef,(0x20,A0)`,
  TTY_$I_INIT 0xE33290; `moveq #0x1f,D1`, TTY_$K_SET_DELAY 0xE67A1A);
  larger constant sets used by `in` are cells in the code, reached with
  `lea (d,PC)` (UNMAP_CASE 0xE54170, NAME_$INIT 0xE31842).

#### Set operators (survey Verified; implemented, bead domain-pascal-7qo)

Survey: `tools/setsurvey.py summary` over the compiled routines.  A set
of one word or one longword is an integer to the 68010, and most of its
code (`and`, `or`, `andi`/`ori` to memory, `tst`) cannot be told from
integer `&`/`!` code or from packed boolean fields (5.3).  The forms
below are taken from sites the ../source TTY reconstruction reads as set
operations (its Pascal, `domain_os/tty/*.pas` at ../source 3898a2ce,
was written from the image and Apollo's customer `tty.ins.pas`, whose
flag fields are sets), and from the forms only a set gives (a one-member
set built with `clr; bset Dk,Dn`; the byte offset of member k).  `dpc`:
lowering `crates/dpc-ir/src/lower.rs` (`set_value`, `set_compare`,
`set_member_at`, constants in `const_of`), selection
`crates/dpc-m68k/src/isel.rs` (`set_update`, `set_of`, `set_op_value`,
`set_member_reg`, `set_member_at`); golden `tests/golden/sets/set_ops`;
tests `crates/dpc-m68k/tests/sets.rs` (forms, refusals, and a 68010
model run against the bit model of Figure 3-6).

- **`s := s + [c]`, `s := s - [c]`, one constant member (Verified at the
  TTY sites):** `bset`/`bclr #c mod 8` on the byte that holds it, `size-1
  - c div 8`: TTY_$I_OUTPUT_BUFFER_DRAINED 0xE1B39E `bclr #0,(9,A2)` and
  TTY_$K_SET_FLAG 0xE6745C/0xE6747C `bset`/`bclr #4,(9,A2)` on the word set
  at 8; TTY_$I_RCV 0xE1B952 `bset #1,(0xb,A2)` on the word at 0xA;
  TTY_$K_INQ_FLAGS 0xE674C2 `bset #0,(1,A1)`; tty_$i_put_chars 0xE1B362
  `bset #0,(9,A0)`.  The same narrowing as packed booleans (5.3).
  **Uncertain:** no member of 8 and above is known to be a set's (the
  TTY sets use members 0..7); PROC2_$FORK 0xE732B4 `bclr #0,(-186,A2)` is
  bit 8 of a flag word of unknown type.  `dpc` narrows every member so.
- **Several constant members (Verified once):** `andi #~mask` to memory
  for `-` (TTY_$I_RCV 0xE1B98E `andi.w #-0xe1,(8,A2)`, `- [5, 6, 7]`),
  `ori #mask` for `+` (PROC2_$INIT 0xE3065A `ori.w #0x180,(42,A2)`, two
  members of the flag word, type not known).  One exception: the TTY
  module-local 0xE1C006 `moveq #-8,D1; and.w D1,(8,A2)` (the ../source
  reading `- [0, 1, 2]`), one of 6 sites with a constant `and`/`or`ed
  into memory through a register (0xE1557C, 0xE1CD32, 0xE3225E, 0xE68E52,
  0xE68F68), against 385 `andi`/`ori` to memory: perhaps `s := s * c` or a
  constant left operand; not settled.  Another reading fits both TTY
  sites: the constant through a register when `moveq` loads it (-8 at
  0xE1C006; -0xe1 at 0xE1B98E does not fit).  Five of the six register
  sites have a `moveq` constant, and four are integer updates in
  ../source's reading (KBD_$RCV 0xE1CD32 `|= 0xF`, which ../source's
  Pascal reads as a packed field, 5.3; RIP 0xE68E52/0xE68F68
  `&= 0x3F`, ../source `rip/table.c`; 0xE3225E, `peb/load_wcs.c`), but
  `andi`/`ori` to memory with a `moveq` constant is common too (e.g.
  `andi.l #15` ×8, 0xE09CE4).  So `dpc`'s `andi.w #-8,(8,A2)` for `s := s
  - [0, 1, 2]` does not match 0xE1C006.  `dpc` refuses `s := s * x` in
  place.
- **`[k]`, one run-time member (Verified, 46 sites):** the register
  cleared at the set's size, then `bset Dk,Dn` (mod 32: no range check).
  The element is evaluated first when computed (DBUF_$GET_BLOCK 0xE3A726
  `moveq #15,D0; and.b (12,A4),D0; clr.w D1; bset D0,D1`;
  PROC2_$DELIVER_FIM 0xE3EE22 `subq.w #1,D0; clr.l D1; bset D0,D1`),
  loaded after the `clr` when it is a variable (TTY_$K_SET_INPUT_FLAG
  0xE6768E `clr.l D1; move.w (A2),D2; bset D2,D1`; 0xE3EE64), used in its
  register when held (EC2_$RELEASE_EC1 0xE42BA4).  Then: `or Dn,<mem>`
  for `s := s + [k]` (×17: 6 word, 11 longword; TTY_$K_SET_INPUT_FLAG
  0xE67694, AST_$DISMOUNT 0xE069F8 `or.w D3,(1056,A5)`), `not Dn; and
  Dn,<mem>` for `s := s - [k]` (×11; DBUF_$INVALIDATE 0xE3AA92 `not.w D0;
  and.w D0,(2332,A5)`, TTY_$K_SET_INPUT_FLAG 0xE676A0), a push as an
  argument (×3, DISK_$FORMAT 0xE3D4E6 `move.w D1,-(SP)`), other uses
  (×15, combined with other values: BAT_$FREE 0xE3B698 `bset D5,D1; or.l
  D2,D1`).  So a member above the set's bits is taken mod 32: of a word
  set, members 16..31 are lost when the word is used, 32 and above wrap
  (`dpc` reproduces this; ref 3.9.3 has no such member).  `s := [k]` of a
  32-bit frame set is built *in memory* with a guard instead
  (AST_$GET_SEG_MAP 0xE06E44..0xE06E54 `clr.l (-124,A6); moveq #31,D5;
  sub.w D0,D5; bcs.s; lsr.w #3,D5; lea (-124,A6),A1; bset D0,(0,A1,D5.w)`;
  FILE_$GET_SEG_MAP 0xE740CE, REM_FILE_$GET_SEG_MAP 0xE62044): refused.
- **`s := s + t`, `s := s - t`, `s := s + (t * c)` (Verified at the TTY
  sites):** the right operand into a register, then `or` (for `-`, `not`
  and `and`) into the set: 0xE1C034 `move.l (0x18,A2),D1; or.l
  D1,(0x14,A2)`; 0xE1BF9E `move.l (0x24,A5),D1; not.l D1; and.l
  D1,(0x14,A2)`; TTY_$I_RCV 0xE1BACE `move.w D4,D1; andi.w #0x80,D1; or.w
  D1,(8,A2)` (×3, also 0xE1BAE2, 0xE1BAF6).
- **Values (Verified forms; operand order Uncertain):** `x * c` as the
  integer `x & c` (section 8): TTY 0xE1BFFE `moveq #7,D1; and.w
  (8,A2),D1`, TTY_$K_PUT 0xE67D04; a held value and a constant `moveq`
  cannot load: copied, then `andi` (0xE1BACE; survey: 24 copies and
  `andi` with such a constant, no constant-first form; 32 `moveq #c,Dn;
  and Dm,Dn` with one it can, 5 copies and `andi`).  `x * y`: the right
  operand loaded, the left `and`ed in (0xE1BF92 `move.l (0x24,A5),D1;
  and.l (0x14,A2),D1; move.l D1,(0x18,A2)`; which source operand was the
  right one cannot be seen).  `x - y`: the right operand loaded,
  complemented, the left `and`ed in (×16 followed by `beq`/`bne`:
  PROC2_$SIGPAUSE 0xE3FAC6 `move.l (-108,A4),D1; not.l D1; and.l
  (-100,A4),D1; beq.s`, PROC2_$SET_SIG_MASK 0xE3F9C6, PROC2_$SIGSETMASK
  0xE3F70E; whether these are sets or `x & ~y` integers is not known:
  the code is the same).  `x + y` and `x - c` as values are not
  catalogued: refused.
- **`[]` (Verified):** `clr` of the set (TTY_$I_INIT 0xE3325E `clr.w
  (8,A0)`; 0xE1C044 `clr.l (0x18,A2)`; tty_$i_wait 0xE1C282 `clr.w
  (-0x1e,A6)`).  `s <> []` / `s = []`: `tst` of the set (TTY_$K_GET
  0xE1C5D4 `tst.w (10,A2)`); of an intersection or difference, the flags
  of its `and` (0xE1BFFE `...; and.w (8,A2),D1; beq.s`; 0xE3FACC).
  Other set comparisons (`=` with a non-empty set, `<=`, `>=`) have no
  catalogued site: refused.
- **`x in c`, a constant register-sized set (Verified):** the element,
  then the constant in a register, `btst Dk,Dn` (TTY_$K_SET_DELAY
  0xE67A18..0xE67A1E `move.w (A2),D0; moveq #0x1f,D1; btst D0,D1;
  bne.b`; tty_$i_wait 0xE1C264 `move.w (0x38,A2),D0; moveq #3,D5; btst
  D0,D5`).  A variable element of a longword set in a condition: the set
  in a register and the branch on Z (tty_$i_set_funcs 0xE67228 `btst
  D3,D0; beq.b` on a set parameter held since the entry, 0xE67230
  `move.l (0x20,A0),D4; btst D3,D4; beq.b`).
- **`in` on a set of more than a longword (Verified, 40 byte-offset
  sites):** `moveq`/`move.w #bits-1,Dx`, the element, the set's address
  with `lea` into a new address register (even `lea (A5),A0`), `sub.w
  Dk,Dx; lsr.w #3,Dx`, `btst Dk,(0,An,Dx.w)`, the branch on Z: the TTY
  module-local 0xE1B740..0xE1B754 for a char in a `set of char` (`clr.w
  D1; move.w #0xff,D2; move.b (0x2d1,A0),D1; lea (A5),A0; sub.w D1,D2;
  lsr.w #3,D2; btst D1,(0,A0,D2.w); beq.b`: the char zero-extended around
  the constant's load), MST_$ALLOC_ASID 0xE42D5C (64 bits), MSG_$RCVI
  0xE596E0 and 10 more on 64-bit sets.  Where the element's type reaches
  beyond the set's bits a `bcs` after the `sub.w` skips the `btst`, and
  the branch is `ble`/`bgt` (N set by the borrow, or Z): 14 sites, all on
  constant sets in the code (NAME_$INIT 0xE31836..0xE3184A, UNMAP_CASE
  0xE54166..0xE54178); refused.  **Uncertain:** the `lea` comes after the
  `lsr` for a set in a record reached through a register (MSG_$CLOSEI
  0xE59432, MSG_$FORK 0xE73F38); `in` on a large set as a value (no site).
- **Sets of more than a longword as operands (Reported, not implemented):**
  built and combined in frame temporaries: `[k]` as `lea (-20,A6),A0;
  clr.l (A0)+; clr.l (A0)+` and the byte `bset Dk,(0,A0,Dx.w)` (6 sites,
  MSG_$OPENI 0xE59248..0xE59260), `s - [k]` as a `dbf` loop `move.l
  (A3)+,D0; not.l D0; and.l (A1)+,D0; move.l D0,(A4)+` into a second
  temporary, then a copy back (MSG_$CLOSEI 0xE59454..0xE5948C; 12 such
  loops); `s = []` as `tst.l (A1)+; bne` per longword (0xE59496).
  Refused.  PAS_$SET_BUILD 0xE11FA8 (hand assembly: copy a set, then
  `bset` each member of a range clamped to it) has no compiled caller: a
  run-time range `[a..b]` is refused.
- **`in` on sets in memory, constant member (survey Verified; reading
  of the exceptions Uncertain; implemented, bead domain-pascal-578).**
  Survey: `tools/setsurvey.py summary`, kind `ctest` (every `btst #k`
  with a constant bit in compiled code, on memory or on a data register
  that a `move` from memory loaded just before; 713 sites).
  - **The rule:** the set is read only as far as the low-order part
    that holds bit k *at the same bit number*, never renumbered:
    - k < 8: `btst #k` on the set's last byte (an odd address for the
      word and longword fields): tty_$i_kill_line 0xE1B6B6 `btst
      #2,(0x1f,A2)`, TTY_$I_RCV 0xE1BAB6 `btst #0,(23,A2)` on the
      longword at 20, PROC1_$RESUME 0xE1479A `btst #3,(85,A2)`, PROC2's
      word at -186 `btst #1..6,(-185,An)`; through a base just reloaded
      too (PROC2's module-local 0xE748F8 `movea.l (-188,A6),A1; btst
      #3,(-185,A1)`; AST 0xE028C4).  As a value `btst; sne Dn` (TTY_$K_GET
      0xE1C54E `btst #1,(0x17,A2); sne D1`).  Member 7 is the byte's
      sign, `tst.b; bmi`/`smi` (0xE1B63E `tst.b (9,A2); bmi`): no `btst
      #7` (nor `#15`, `#31`) exists in compiled code.  Memory forms: 356
      at odd addresses, 66 at even ones (byte objects or packed booleans,
      5.3, which take any byte; no field shows an even-byte `btst` and a
      `move.w` of the same word with k >= 8).
    - k = 8..15 of a longword: its low word loaded, `btst #k` (TTY_$I_RCV
      0xE1B972 `move.w (22,A2),D0; btst #12,D0`).
    - Otherwise the whole set: k = 8..15 of a word (PROC2's word at -186:
      members 8..13 by `move.w` in 29 routines, members 1..6 in memory in
      11, 5 of them with both; FIM_ (20,A6), WIN_ (-26,A6) likewise), k = 16..30 of a
      longword (PROC2_$SET_SIG_MASK 0xE3F94A `move.l (-100,A2),D?; btst
      #17`, ×5; 0xE3EF4A; NETLOG_$PROC_START 0xE71AF0).  An *update* is
      narrowed with renumbering (above, `bset #c mod 8`; PROC2_$FORK
      0xE732B4 `bclr #0,(-186,A2)`, member 8 of the word whose tests of
      8..13 are `move.w`).
  - **The exceptions:** 78 tests read more than that part (`whole` in
    the survey): a word loaded for k < 8 ×61 (0xE013BA..0xE013BE `move.w
    (190,A2),D0; btst #4,D0`, AST's word at 190 in 5 routines; AREA_'s
    field at 46 in 11; NETWORK (-878,A2); RING (-48,A2); FILE_
    0xE5EDB4..0xE5EE58), a longword for k < 16 ×13 (tty_$i_put_chars
    0xE1B0E6, 0xE1B166, 0xE1B1E8 `move.l (12,A0),D1`; TTY_$I_ERR
    0xE1BE3C, 0xE1BE54; TTY_$I_STORE_PARITY 0xE1BD0A; TERM_$INQUIRE
    0xE66E68.. on frame longwords), an indexed operand ×4 (0xE4437E,
    0xE5F4C8, 0xE5F7B8, 0xE5FB70: no `btst #k,(d,An,Dn)` exists).  What
    they have in common is the *field*, not the code around the test:
    - per routine, a field is tested one way only: of the routine
      fields with a narrowable test, 2 have both forms, and in both the
      loaded copy is reused (PROC1_$SUSPEND 0xE1482A `move.w (84,A3),D1;
      btst #3,D1; ...; moveq #6,D2; and.w D1,D2; ...; btst #1,D1`, then
      after a call `btst #1,(85,A3)`; TTY_$I_RCV 0xE1B994 tests a copy
      saved in D4 before an `andi` to the field: a local);
    - no context rule fits.  Not the base: AST's word at 190 and its
      byte 15 are tested through the same A2, held since the entry, in
      the same routine (0xE013BA whole, 0xE013C6 `btst #0,(15,A2)`);
      the TTY reading (base reloaded from a frame slot: whole; held:
      narrowed; ../source pascal-translation-notes finding 1) is refuted
      by PROC1, PROC2 0xE748F8 and AST 0xE028C4 (reloaded, narrowed).
      Not an up-level reference: FILE_$PRIV_LOCK's word parameter at
      0x14 is tested in memory by FILE_$PRIV_LOCK itself (0xE5F15E
      `btst #1,(21,A6)`) and
      by one nested routine (0xE5ECAE `btst #5,(21,A0)`, A0 its static
      link), whole by the other (0xE5EDB4 `move.w (20,A3),D0`, A3 the
      static link).  Not the count of tests, the context (condition or
      value), the module or its place in the image (whole tests in 14 of
      the 52 code segments with sites, 11 of them with memory tests too);
    - 7 of the 9 `lsr.b #1` bit-0 tests (above) are on such fields
      (tty_$i_put_chars 0xE1B0F6, AREA_ 0xE07B66/0xE08826/0xE0905A on
      46, AREA_/MST_ (-2,An), AUDIT (168,A5)), and compiled code has no
      `and` with a one-bit mask tested for zero (33 with several bits),
      so a one-bit `(x & c) <> 0` must become a `btst` too.  The reading
      taken: the whole form is an integer bit test (`&` of one bit,
      `odd`), possibly of a set viewed through a type transfer (ref
      4-197), and `in` is always narrowed.  **Uncertain:** the image
      cannot show the source.  The integer form is implemented (7.1,
      "One bit"; bead domain-pascal-3xwx).  A routine's word read both
      ways supports the reading for `in`: WIN's module-local 0xE19276
      (../source `win_$check_disk_status`) tests its frame word at -0x1A
      narrowed for bits 3, 4, 1, 6, 5 (0xE192C0 `btst #3,(-0x19,A6)`) and
      loaded for bits 11, 13 and 12 (0xE192AA `move.w (-0x1a,A6),D1;
      btst #11,D1`, that copy also `andi.w #0xfa` at 0xE192BA; loaded
      again at 0xE1932C for bit 13, bit 12 at 0xE19356 on that second
      copy): the `in` rule exactly, so
      under this reading that word is a set there, not the integer the
      translation's `&` makes it.
  - `dpc` (`isel.rs`, `set_bit_test`; tests `crates/dpc-m68k/tests/sets.rs`
    `constant_members_in_memory` and the 68010 model run
    `constant_membership_in_memory_follows_the_bit_model`; goldens
    `expr/calls_sets`, `stores/bool_stores`): the rule above for a set at
    `(An)`, `(d,An)` or an absolute address; the whole set for an
    indexed operand, a volatile set (not catalogued) and a set held in a
    register.  Refused: member 7, 15 or 31 tested in a register (member
    15 of a word or of a longword's low word, 31 of a longword, 7 of a
    set read whole: volatile, indexed or held), since no `btst #7`,
    `#15` or `#31` exists on any operand among the 916 `btst #k` of
    compiled code (a lead for 7: 0xE19106..0xE1910C `move.w (6,A0),D3;
    tst.b D3; bpl` on the WIN_ controller's status word, the type not
    known); a loaded
    part reused for several tests (PROC1_$SUSPEND, TTY_$I_RCV 0xE1BBC2
    `move.w (22,A2),D1` for members 9, 3 and 4) is not reproduced: `dpc`
    tests each member in memory (bead domain-pascal-ozaq).

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
  - **6 and 8 bytes (Verified counts; `tools/storesurvey.py blocks 6
    8`, bead domain-pascal-fbg).**  Runs of adjacent `move.l`/`move.w`
    over consecutive memory, in every compiled routine (1635):
    - **8 bytes: the source post-incremented, the destination by
      displacement.**  865 runs of two `move.l`: 855 `move.l (Am)+,<d>;
      move.l (Am)+,<d+4>` (into a register's object 426 + 11 indexed,
      the frame 401, the data area 12, an absolute address 5); no `(Am)+,
      (An)+` pair.  The other 10 move both operands by displacement
      from A5/A6/an absolute address, and are adjacent field assignments
      where the C is known (REM_NAME_$GET_ENTRY 0xE4AD5C, `node :=
      curr_node; net := curr_net`, two integer32 fields).  The source
      register is formed as for longer copies: `lea (d,An)` 228 and `lea
      (An)` 94, A5 among them (UID_$GEN 0xE1A0B6 `lea (A5),A0`;
      TTY_$K_SET_INPUT_BREAK_MODE 0xE6784E `lea (A2),A0`, an `in`
      parameter held in A2), `movea.l (d,A6)` 150, `lea (d,A6)` 146,
      `movea.l Dn` 35, and `movea.l #abs` for another unit's object
      (121 name UID_$NIL, 0xE00F80): a source is never addressed by
      displacement.
    - **Order and registers when the destination is a pointer slot's
      object (Verified at the sites).**  Of the 90 copies whose two
      sides each take one instruction, the source's register is the
      lower (coloured first) at 80; of the other 10, 3 are destinations
      held or copied from another register (TTY_$K_SET_PGROUP 0xE67930,
      0xE45862, 0xE53D94), the rest unexplained (0xE478F4, 0xE496E4,
      0xE2FB0C, 0xE3179A, 0xE440D8, 0xE48D8C, 0xE4D23C).  The source's
      load comes first with a frame source (18 sites, UID_$GEN
      0xE1A0E2 `lea (-8,A6),A0; movea.l (8,A6),A1`) or another unit's
      (11, 0xE00F80; 6 others load the destination first: 0xE478F4,
      and 0xE48E22, 0xE4A082, 0xE4A26E, 0xE50848, 0xE57810 into
      callee-saved A2-A4), the destination's first with a source in
      the data area (all 3: NAME_$GET_ROOT_UID 0xE589C8 `movea.l (8,A6),A1; lea
      (0x38,A5),A0`, 0xE589EA, 0xE58A0C) and with one reached through a
      parked integer local (section 3), whose `movea.l D0,A0` comes
      first (TTY_$K_INQ_INPUT_BREAK_MODE 0xE678E6 `movea.l D0,A0;
      movea.l (0xc,A6),A3; lea (0x38,A0),A1`, A3 because A1 is the
      source's and A2 holds the status; TTY_$K_INQ_PGROUP 0xE6796C).
      Destinations loaded into a callee-saved register away from the
      copy (0xE48E22, 0xE4A26E, 0xE50848 A2-A4) and destinations copied
      from another register (0xE45502 `movea.l A3,A2`) come first
      too; no rule found for them.  A destination needing no register
      of its own (the frame, the data area, a held register, a parked
      local's `movea.l D0,A0`, TTY_$K_SET_PGROUP 0xE6792A) is set up
      first.
    - **6 bytes: two moves memory to memory**, `move.l <s>,<d>; move.w
      <s+4>,<d+4>` (122 of 127 runs; no 6-byte run post-increments):
      KBD_$RCV 0xE1CDB8 `move.l (-0x10,A6),(0x20,A2); move.w
      (-0xc,A6),(0x24,A2)`; CAL_$SET_DRIFT 0xE68588 `move.l (A0),abs.l;
      move.w (4,A0),abs+4.l`.  The other 5 are `move.w` then `move.l`
      (4) or three `move.w` (1), field assignments by their shape.
      Both operands are set up as a longword store's (12.4, "Stores whose destination and value both need a
      register"): the source first at 0xE05104, 0xE1542A, 0xE65184,
      0xE747FC, 0xE74828, the destination at 0xE16DF8 (a slot into
      callee-saved A2).
    **Implemented** (`isel.rs` `copy6`, `copy8`): the forms above.  An
    8-byte copy into an element indexed at run time is refused (11
    sites, 0xE38CA2, order not settled; bead domain-pascal-0qy3), as
    is one whose destination needs other set-up, a source through another pointer expression and
    volatile objects.  Byte-identical in `make conform`:
    NAME_$GET_ROOT_UID, _GET_NODE_UID, _GET_NODE_DATA_UID
    (`name_uid_getters.pas`), TTY_$K_SET_PGROUP (`tty_k_set_pgroup.pas`),
    CAL_$SET_DRIFT (`cal_set_drift.pas`), TTY_$K_INQ_PGROUP
    (`tty_k_inq_pgroup.pas`: its slot load after the status test passes
    over A2, the status's dead entry load, to A3, 12.4 R4(b); bead
    domain-pascal-i24w; TTY_$K_INQ_INPUT_BREAK_MODE likewise, OK in
    srcsurvey); golden `data/small_copies` has their order without the
    test, `regalloc/late_slot_pass` with it.
    TTY_$K_SET_INPUT_BREAK_MODE compiles, its copy byte-identical, and
    differs only in the test after it (bead domain-pascal-0g98).
  - **Displacement moves of 12 and 16 bytes** are rare: XNS_IDP_$GET_STATS
    0xE18FE6 `move.l (A5),(A0); move.l (4,A5),(4,A0); move.l
    (8,A5),(8,A0)` and 0xE1ABEA (16 bytes) against 59 + 42 `(An)+`
    copies.  Taken to be field-by-field assignments in the source
    (Uncertain: the image cannot tell; the C of XNS_IDP_$GET_STATS
    assigns three named fields).
  - **Copies of value parameters from their slot** (section 1.2): 6
    bytes `move.l (An),(k,A6); move.w (4,An),(k+4,A6)` (6 sites), 8 bytes
    `move.l (An)+,(k,A6)` twice (124 sites, the DIR_$ADD_MOUNT form
    above); implemented for those copies only (`Selector::param_copies`).
  - **Implemented** (`crates/dpc-m68k/src/isel.rs`, `copy`): records and
    arrays that are not 1, 2 or 4 bytes, 10 bytes and more, with A0/A1;
    the source or destination in the data area, in the frame, or the
    object a pointer in the frame points to, or a field of the object a
    pointer held in an address register points to (`lea (d,An),Am`, the
    form above; added with the record comparisons of 7.1, bead
    domain-pascal-aaj).  6 and 8 bytes: above.  Rejected: another
    unit's object (`movea.l #abs,A1` at 0xE336E2, one site; the record
    comparisons of 7.1 load such addresses so 201 times, a copy only
    once), a field through a pointer not held in a register, statements
    of bodies with several statements.
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
    otherwise a multiply: `mulu.w #size` for sizes of 128 and more,
    BAT_$GET_BAT_STEP 0xE3BB6A `mulu.w #0x234`, `proc2/suspend.c`
    0xE412AE `mulu.w #0xE4`; for sizes of 1..127 the size first, `moveq
    #size,Dn; mulu.w <index>,Dn`, TPAD_$SET_UNIT_MODE 0xE69824).  The
    `idx*0xF8` = `(idx<<8)-(idx<<3)` of `vtoc-disk-layouts.md` is the
    ×248 form of section 8 (`lsl #3; move; neg; lsl #5; add`, 0xE38994).
  - **Scalings by element size (survey Verified; `tools/arithsurvey.py
    scale` / `scales`, bead domain-pascal-r3yy; implemented for `.w`
    indices).**  Every product of a value by a constant formed in a block
    of compiled code, simulated register by register, with what reads it
    first (an indexed operand or another instruction):

    | Size (odd part) | Form | Sites (index `.w` / `.l` / other) |
    |---|---|---|
    | 2, 4, 8, 16, 32, 64, 128, 1024 | shifts | 1150 indices (`add.w Dn,Dn`; `lsl`, split above 8) |
    | 6, 10, 12, 17, 18, 20, 24, 36, 40, 48, 72, 96, 144, 257 (2^k + 1) | shifts | 378 |
    | 28, 56, 60, 120, 248 (2^k - 1) | shifts with `neg` | 106 |
    | 26 (13) | bit by bit, `add.w D1,D1; move.w D1,D0; lsl.w #2,D0; add.w D0,D1; add.w D0,D0; add.w D0,D1` | 1 (NETLOG_$LOG_IT 0xE71BBA) |
    | 44 (11), 46 (23), 84 (21), 92 (23), 100 (25) | `moveq #size,Dn; mul<s>.w x,Dn` | 43: 21 / 20 / 2 |
    | 92 (23) | `mulu.w #92` in place | 1 (RIP_$BROADCAST 0xE872F8, after `clr.w D5; move.b (14,A2),D5`) |
    | 150, 153, 204, 228, 244, 250, 268 (67), 300, 336 (21), 564, 580 (145), 1083, 1244, 1624, 5240, 12345 | value loaded or copied, `mul<s>.w #size` | 261 |

    The survey models a mask (`and`, `andi`) as a new computed value
    (review of bead domain-pascal-r3yy: without it the two ×84 scalings
    of HINT_$GET_HINTS 0xE4999A and module-local 0xE49A2C at 0xE49A68,
    `uid_key & 63`, and 25 power-of-two scalings were missed).  It does
    not follow a product read past a branch target: PROC1_$LOADAV_CALLBACK
    0xE14C56 `muls.w #363,D1` (a held register multiplied in place, its
    last use) is not in the 261 (an independent scan of every
    `mul<s>.w #c` in compiled code: 263, 262 of them by 128 or more).

    - **The rule:** a constant whose odd part is not 1 or 2^k ± 1 is
      multiplied with `mul<s>.w`; a constant of 1..127 (one `moveq` can
      load it) goes into the product's register first and the value is
      the source operand: a variable in memory (`mulu.w (A4),D2` through a
      by-address parameter held since the entry, TPAD_$SET_UNIT_MODE
      0xE69826, and the five other TPAD unit routines, RIP_$TABLE_D
      0xE68E5E, 0xE45CAC through a pointer field; `muls.w (380,A5),D0` a
      unit variable, TPAD_$DATA 0xE691D0; `muls.w (-2,A6),D1` a local,
      RINGLOG_$CNTL 0xE722A2), or a register (a value held since before
      the block, VTOC_$MOUNT 0xE38612 `moveq #100,D0; mulu.w D2,D0`,
      RIP_$NET_LOOKUP 0xE15504 `moveq #44,D0; muls.w D1,D0`; a by-value
      parameter loaded first, module-local 0xE383B0 at 0xE383C8..0xE383CE
      `move.w (12,A6),D2; moveq #100,D0; mulu.w D2,D0`, VTOC_$GET_NAME_DIRS
      0xE39406, VTOC_$SET_NAME_DIRS 0xE394AE).  43 of the 44 products by
      such constants.  Of the three with a computed value, the two masks
      have the constant first too (HINT_$GET_HINTS 0xE4998E..0xE4999A
      `moveq #63,D3; moveq #84,D5; and.w D0,D3; movea.l #sym,A1; muls.w
      D3,D5`, the mask's `moveq` and the product's before the `and`;
      0xE49A5C..0xE49A68) and a zero-extended byte is multiplied in place
      (RIP_$BROADCAST 0xE872F8 `mulu.w #92,D5`).  Constants of 128 and more: the value
      in a register (a load, or a copy of a held one), then `mul<s>.w
      #c`: 261 of 261; no `move.w #c,Dn` multiplied by the value.  As for
      `x + c` (section 8), a constant `moveq` loads goes first.
    - **The order:** the `moveq`, then the operand's address (a
      by-address parameter's slot: TPAD_$INQUIRE_UNIT 0xE6999C..0xE699A2
      `moveq #44,D0; movea.l (8,A6),A0; mulu.w (A0),D0`;
      PKT_$BLD_INTERNET_HDR 0xE121E6..0xE121F2), then another unit's
      array base, then the multiply (RIP_$TABLE 0xE6904C..0xE69054
      `moveq #92,D1; movea.l #sym,A0; mulu.w D0,D1`; 0xE0B8F2, 0xE121E6,
      RIP_$UPDATE 0xE690FA, RIP_$TABLE_D 0xE68E56; 5 of 6, RIP_$TABLE_D
      0xE68F6C has the base after the multiply and a `lea` between).  A
      by-value parameter is loaded before the `moveq` (above).
    - **The sign:** a `.w` index is always `mulu.w` (21 of 21 with the
      constant first, 94 with `#c`), a `.l` index `muls.w` for a signed
      index (20 of 20 with the constant first) and `mulu.w` for an
      unsigned one (below).  The width, not the index type's sign, goes
      with the mnemonic: the TPAD unit routines' `in unitp: integer` is
      `mulu.w`/`.w` (0xE69826) where TPAD_$DATA's `tpad_$unit: integer`
      into the same array is `muls.w`/`.l` (0xE691D0); when an index is
      `.l` is bead domain-pascal-wkg.  `mulu.w` for a `.w` index of a
      negative word is right: the low word of the product is the same for
      both signs, and the index is that word sign-extended (exec test
      `index_mul`, indices down to -13200).  Other uses: the one word product,
      module-local 0xE4519C at 0xE458CC..0xE458D2 `moveq #44,D3; muls.w
      D2,D3; addi.w #52,D3`, a signed word (`acl/prim_create_internal.c`
      `out_idx`).
    - **Odd parts of three bits:** 11, 21, 25, 67 and 145 are multiplied;
      13 is done bit by bit (×26, one site).  No rule (the bit count, a
      68000 cycle count) separates them; `dpc` refuses the other odd
      parts of three bits (19, 35, 37, 41, ...) and 13 (bead
      domain-pascal-irb4).
    - **Implemented** (`isel.rs`, `by_mul_insn`, `mul_const_first`,
      `const_first`; `scaled_index`, `mul`): `.w` indices and word
      products of a signed word, with the constant first for a variable
      (a unit variable, a local, a by-address parameter's object or a
      pointer's target, a parameter or register-held value); a
      zero-extended byte in place.  The longword product of a signed word
      (`.l` index evidence, TPAD_$DATA 0xE691D0) the same way.  Refused (no
      site): an index or factor that is another unit's variable, an array
      element or an enclosing routine's variable; a word or longword
      product of an unsigned word by such a constant.  Refused (not
      settled): other computed values (`i + 1`, a mask; the masks above
      have the constant first, scheduled with the mask's own `moveq`;
      bead domain-pascal-q322).  Golden `data/index_mul`, exec test
      `index_mul`, tests `crates/dpc-m68k/tests/expr.rs`
      (`products_with_the_constant_first`).  TPAD_$SET_PUNCH_IMPACT and
      TPAD_$INQ_PUNCH_IMPACT now compile but differ in register choice
      (the by-address parameters' address registers, the product's D2;
      bead domain-pascal-qhda).
  - `mulu.w #c` and `muls.w #c` (survey, ad-hoc script: what reads the
    product): `mulu.w` feeding a `.w` index ×94 (0xE0B2D0), a `.l` index
    ×43 (0xE39364, after an unsigned compare of the index); `muls.w`
    feeding a `.l` index ×118 (0xE1D7A2); no `muls.w #c` feeds a `.w`
    index.  So the product's sign matters only for `.l` indices.
    One variable gets both forms: TPAD_$DATA 0xE691CA..0xE691D4 `moveq
    #44,D0; muls.w (380,A5),D0; lea (0,A5,D0.l),A2` reads the module's
    unit number off A5, and TPAD_$SET_UNIT_MODE 0xE69824..0xE69828
    `moveq #44,D2; mulu.w (A4),D2; lea (0,A5,D2.w),A1` reads it through
    its by-address parameter (TPAD_$SET_MODE passes `pea (380,A5)`,
    0xE697E2), which the routine compares signed (0xE6980A `tst.w (A4);
    ble`; 0xE69814 `cmp.w (A4),D0; slt`).  So the form depends on how
    the index is reached, not only on its type (Verified at the sites;
    lead: ../source pascal-translation-notes finding 39; bead
    domain-pascal-wkg).
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
  - **Another unit's array (survey Verified; implemented in straight-line
    code, bead domain-pascal-rlnm).**  `tools/boolsurvey.py summary`
    (kind `ebase`: every indexed operand `(d,An,Dx)` through A0-A4 of
    compiled code, 1952, with how An was set in the block and where that
    sits among the instructions that form Dx; `list ebase` for the sites).
    - *The split: the immediate is the variable's symbol, every offset in
      the displacement.*  Of the 1043 `movea.l #imm,An` bases, 958 are
      exactly a map symbol or a data block's start; the displacement holds
      the lower bound and field offsets (module-local 0xE4649C `movea.l
      #PROC1_$TYPE,A0; ...; cmpi.w #2,(-2,A0,D1.w)`, a 1-based word array;
      DISK_$REVALIDATE 0xE6C06A `movea.l #DISK_$DVTBL,A0`, `pea
      (-0x48,A0,D0.w)`, ×72 1-based).  The other 85 are the start of a
      data section plus 60000 (`ACL_$DATA+0xEA60`, 66 sites, ACL_$IN_SUBSYS
      0xE470AA `movea.l #0xE97294,A0` and `tst.w (-15706,A1)`, although
      ACL_$DATA is only 0xAD98 bytes; `FILE_$LOT_DATA+0xEA60` = 0xEA202C,
      19 sites, a section of 0x1086C bytes): variables of those
      sections, which have no map symbols of their own.  (An earlier
      version of this entry read 0xE4649C's immediate as 0xE2622C; it is
      0xE2612C, PROC1_$TYPE.)  `lea abs.l,An` occurs for the unit's own
      data addressed absolutely (AUDIT_$SUSPEND 0xE70DC0, 4 operands).
    - *The order: the index's value, then the base, then the scaling.*
      The tool simulates the scheduler of 12.4 R11 (W = 3) over the index's
      chain (back from the operand to its first full write: a load, a
      copy, `clr` and the move into it, through a shift-and-add's
      temporaries), the base and the instruction before them, for two
      generation orders: the value (the chain less its trailing scaling),
      the base, the scaling; or the whole chain, then the base.  The rule
      "value, base, scaling; for a scaling with `neg` (2^s (2^k - 1),
      section 8), the whole index first" fits 518 of the 641 `movea.l
      #sym` bases it can judge (348 of 427 with a one-instruction scaling,
      147 of 189 with more, 23 of 25 with none); 69 of 82 for the 60000
      bases.  Sites: module-local 0xE4649C `move.w (8,A6),D0; movea.l
      #PROC1_$TYPE,A0; move.w D0,D1; add.w D1,D1` (the parameter's load,
      then the base, then the copy and the scaling: the base in the
      copy's stall); TTY_$I_WAIT 0xE1C22E `move.w PROC1_$AS_ID,D0;
      movea.l #FIM_$QUIT_EC,A0; lsl.w #2,D0; move.w D0,D1; add.w D1,D1;
      add.w D1,D0` (×12); 0xE042D6 `clr.l D0; movea.l #0xED5000,A1;
      move.w (14,A0),D0; lsl.l #7,D0` (a zero-extension: the base in its
      stall); 0xE1D79A `move.w D2,D0; movea.l #0xE2E3FC,A1; muls.w #268,D0`
      (`mul<s>.w`).  With `neg` the base is after the scaling, which R11
      moves above its last `add` (0xE1AA50..0xE1AA62 ×56, 0xE19750 ×28,
      0xE33178; 37 of the 48 bases with a `neg` within the eight
      instructions before the operand; the "value first" order would put
      it right after the load).  The ×26 of NETLOG_$LOG_IT (0xE71BBA, bit by bit)
      has a pointer base, below.  Of the 123 misfits, most are other
      shapes: a register added to the element in place (`add.l
      D1,(-8,A0,D0.w)`, the value's load in the index's stall and the base
      after the scaling, 0xE02C30; 9 of the 10 such operations), which
      dpc does not do (it loads, adds and stores); two elements in one
      instruction (below); an index shared by two elements
      (FIM_$ACKNOWLEDGE, 12.4 R7).
    - *Restricted to what `dpc` forms* (review of bead domain-pascal-rlnm,
      ad-hoc classification of `list ebase` with `tools/imgdis.sh`): of
      the 641 judged sites, 506 lie outside every backward branch's range.
      165 of those are `lea (0,An,Dx),Am` used as an address (refused,
      below), 115 a `lea` for one access at a displacement beyond a byte
      (refused, below), 16 updates in place (two elements in one
      statement).  The other 210 are element operands and `pea`s: 173
      fit.  Of the 37 misfits, 15 have two elements of other units'
      arrays in one statement, 10 are 8-byte record stores (`move.l
      (A0)+,(10,A1,D1.w)` twice, PROC2_$FORK 0xE731FA; `dpc` refuses an
      8-byte copy to an indexed destination), 1 is a store whose value
      needs an address register (TERM_$CONTROL 0xE66B56, below), 3 use a
      base set by an earlier statement (0xE30B62, 0xE5B756 below; WIN_$CINIT
      0xE3039E, the base held from field stores through it), 3 use a
      scaled index of an earlier statement and get the base right before
      the operand as `dpc` does (FIM_$ACKNOWLEDGE 0xE0A9B0, SIO2681_$INIT
      0xE3346E, SIO6509_$INIT 0xE33532; the tool's chain runs back into
      the earlier statement), 2 zero-extend a word index to a `.l` one
      (0xE430F8, 0xE43154; `dpc` uses `.w` or refuses, above), 1 spills
      the scaled index to the frame (AST_$COPY_AREA 0xE03A9A), 1 reads an
      element whose address it also forms (FILE_$CHECK_PROT 0xE5D1D6).
      One misfit is within the conditions: PKT_$SAR_INTERNET
      0xE71F26..0xE71F36 `move.w PROC1_$AS_ID,D7; addq.l #1,D6; lsl.w
      #2,D7; movea.l #FIM_$QUIT_VALUE,A4; move.l (0,A4,D7.w),D7`, the
      base after the shift (the previous statement's `addq` in between),
      where R11 from "value, base, scaling" puts it before the shift.
    - *Refused besides* (the same review): the same array in a later
      statement with no call between, where the image keeps the base (of
      the 78 `movea.l #` bases that indexed operands of their block use
      again, none is loaded a second time while a register holds it:
      MMAP_$FREE_PAGES 0xE0CF5C..0xE0CF74, 0xE5B744..0xE5B756 `clr.l
      (0,A1,D2.w); move.w (8,A6),D3; lsl.w #2,D3; clr.l (0,A1,D3.w)`;
      after a call it is loaded again, 27 against 3 held; across a branch
      target not seen); an index read from the same variable, through the
      base (SIO_$INIT 0xE32C9C..0xE32CB4 `movea.l #0xE2C9F0,A1; move.w
      (5000,A1),D0; ...`), or a store that reaches another part of the
      same variable (0xE19F7A..0xE19F8A, A1 and A2 both loaded with
      0xE7AEF4, `move.l (236,A1),(300,A2)`), where `dpc` reads it
      absolutely; an element at a displacement beyond a byte,
      `lea (0,An,Dx),Am` into the base's register or another (with the
      base dead after one use, 84 own, SIO_$INIT 0xE32CB0, against 105
      another, 0xE0CC80 `lea (0,A0,D0.l),A2`; 12.4 R2); a store whose value
      needs a register set up, when that is an address register or when
      the destination takes more instructions (`dpc` then puts the
      destination first, 12.4 "Stores whose destination and value both
      need a register"; the image sets the value up before the base:
      TERM_$CONTROL 0xE66B40..0xE66B56 `...; add.w D2,D1; movea.l
      (16,A6),A0; movea.l #DTTE,A1; move.b (A0),(36,A1,D1.w)`, NET_IO_$SEND
      0xE0E8A6, PROC2_$FORK 0xE731EA); the first argument pushed after a
      pad (`dpc` pads first; the image after the set-up: NAME_$FREE_ASID
      0xE74DD8 `movea.l #NAME_$NODE_DATA_UID,A1; subq.l #2,SP; pea
      (40,A1,D2.w)`); follow-ups, bead domain-pascal-lnzy, with the
      PKT_$SAR_INTERNET misfit).
    - *Two elements of other units' arrays in one statement:* 14 moves
      with two such operands (ad-hoc count).  At 8, `fim_$quit_value[as]
      := fim_$quit_ec[as].value` (FIM_$ACKNOWLEDGE 0xE0A972..0xE0A994,
      FIM_$INIT_ASID 0xE0AA54, PROC2_$WAIT 0xE400A4, MSG_$WAITI 0xE59CC4,
      ASKNODE_$WHO_NOTOPO 0xE66310, TTY_$K_PUT 0xE67D7A, ...): the value's
      index in full (×12), the destination's index load, the value's base,
      the destination's scaling, its base, the move.  Three read the index
      once for both (TIME_$WAIT 0xE1662E), two index one array twice with
      one base (PROC2_$FORK 0xE72EE2), one has the value's index from
      before (0xE614C2).  Not modelled: refused (bead
      domain-pascal-0r1v, with the in-place `add.l` above).
    - *In loops:* such bases are loaded in the loop (202 operands,
      KBD_$FETCH_KEY 0xE1CBAA `movea.l #SMD_$KTT,A0`) and also held in a
      register from before the loop (TTY_$K_DRAIN_OUTPUT 0xE67AF0
      `move.l #FIM_$QUIT_VALUE,D7`, `movea.l D7,A1` in the loop, with
      FIM_$QUIT_EC, PROC1_$AS_ID and two field addresses).  Not settled:
      refused (bead domain-pascal-dw25).
    - *The element's address as a value* (`lea (0,An,Dx),Am` stored to
      memory): Am is An at 12 and another register at 15 with
      displacement 0, another at 14 of 15 otherwise (ad-hoc count; 0xE142BA
      `lea (0,A0,D0.l),A0`; TTY_$I_WAIT 0xE1C242 `lea (0,A0,D0.w),A1`), a
      register choice dpc's allocator does not make (12.4 R2): refused
      (bead domain-pascal-o1kl).
    **Implemented** (`isel.rs`, `index_ea`, `other_unit_index`,
    `index_reg_with`): the rule above, in straight-line code, one such
    element per statement and per array between calls, at displacements
    within a byte, with any index dpc forms that does not read the same
    variable, less the stores and padded calls refused above (test
    `dpc-m68k` `other_units_array_elements` has one case of each).  A scaled index held from earlier (12.4 R7) is
    read where it is; the base follows it.  Another data section's block
    (11) stays refused (bead domain-pascal-hxwq).  Golden
    `data/other_unit_index`; conformance `acl_is_process_type_2.pas`
    (module-local 0xE46498, byte-identical); exec test
    `elem_bases`; test `dpc-m68k` `other_units_array_elements`.
  - **Bounds checks:** none in the image; the C of FIM_$FREE_ASID
    (`fim/free_asid.c`) notes "the original performs no bounds check".
    `dpc` emits none.
  - **Arrays through a pointer** (survey, kind `ebase` as above; bead
    domain-pascal-rlnm).  A pointer held in an address register since the
    entry is the base itself (implemented).  A pointer loaded from memory
    (a parameter's slot, a frame or unit pointer variable, a field through
    another register):
    - *Elements of 1 byte* (no scaling, so both orders above give one
      place): the index, then the pointer.  The tool judges 28 operands
      through such pointers with no scaling instruction in the block, 18
      fitting, but those include wider elements whose index was scaled
      before and pointers held since the entry.  Of the 1-byte elements
      with the index formed in the block (review of bead
      domain-pascal-rlnm), 10 fit (KBD_$FETCH_KEY 0xE1CB8A `clr.w D0;
      movea.l (0x34,A4),A1; move.b (A2),D0; move.b (0,A1,D0.w),(A2)`;
      0xE4D13A `move.w (A2),D0; movea.l (8,A6),A0; cmpi.b
      #47,(-1,A0,D0.w)`; 0xE4D180, 0xE4D1C6, 0xE4D292, 0xE4D2EC; VFMT_$MAIN
      0xE6AB5E, 0xE6AB84, 0xE6AC98, 0xE6AF62), all within a loop's range;
      the other two are a padded call's first argument (MSG_$RCV_CONTIGI
      0xE59900 `movea.l (44,A6),A3; move.w (A4),D1; subq.l #2,SP; pea
      (0,A3,D1.w)`) and a store whose value needs a register (0xE6AA1C,
      the value's pointer first), both refused as above.  Others read the
      index through the same pointer, loaded once for both (0xE1B2E0
      `movea.l (8,A6),A0; move.w (980,A0),D1; lea (0,A0,D1.w),A1`;
      0xE4DE58), which dpc does not do.  **Implemented** (`pointer_index`);
      an index read through the same pointer expression is refused, and
      so are displacements beyond a byte, as above.
    - *Wider elements:* not settled.  The pointer's load sits between the
      index's value and its scaling (DXM_$SCAN_QUEUE 0xE17178 `move.w
      (A2),D0; movea.l (0x18,A2),A0; lsl.w #4,D0`; TTY_$I_SIGNAL 0xE1B878
      `move.w D2,D1; movea.l (8,A6),A0; lsl.w #2,D1; ...` ×12; 0xE0C664)
      at 14 of the 56 operands judged, after the scaling and not pulled
      into it by R11 at most others (PMAP_$FLUSH 0xE137E0 `move.w D6,D0;
      lsl.w #2,D0; movea.l (12,A6),A0; lea (0,A0,D0.w),A1`; BAT_$ALLOCATE
      0xE3B27A, 0xE3B30E `move.w D4,D0; lsl.w #2,D0; movea.l (3384,A5),A0`;
      0xE1370C; 0xE4D8C2); neither generation order, nor R11 with an
      interlock on loads only, explains both groups.  Refused (bead
      domain-pascal-nqg1; the
      address of such an element as a value, `lea_into`'s NETLOG_$LOG_IT
      form below, is unchanged).
  - **Byte indices (survey; implemented, bead domain-pascal-ji9).**
    `tools/cmpsurvey.py summary` (kind `bidx`): of the 27 indexed
    operands of compiled code whose index register was last written by a
    `move.b`, 21 have `clr.w Dn` right before the `move.b`, 5 have it
    earlier (scheduled: 0xE1CB8A `clr.w D0; movea.l (52,A4),A1; move.b
    (A2),D0`; SIO2681_$SET_LINE 0xE1D2F6), one has `clr.l` (0xE7062E);
    all use a `.w` index; none `andi.w #255` or `ext.w`.  Scales 1
    (VTOC_$LOOKUP 0xE38FD2..0xE38FD8 `clr.w D0; move.b (28,A3),D0; lea
    (0,A5,D0.w),A0`), 2 (tty_$i_enable_crash_func 0xE672AA..0xE672B4
    `clr.w D2; move.b D0,D2; add.w D2,D2; lea (0,A0,D2.w),A1; move.w
    #0x11,(0xa4,A1)`, ch a parameter held in D0; TTY_$K_SET_INPUT_BREAK_MODE
    0xE67874 from memory; tty_$i_set_funcs 0xE67238 the byte an array
    element itself), 4 (0xE00DD2).  **Implemented** (`dpc-ir`, `addr_of`):
    an index of an unsigned byte type (`char`, a char subrange) is
    zero-extended to an unsigned word (`clr.w Dn; move.b x,Dn`, the
    widening of section 8), then scaled and used as a word index under
    the 16-bit rule above (a 0..255 index times the element size).  A
    signed byte index (a boolean) is refused (`ext.w` is not seen).
    Golden `tests/golden/data/byte_index`.  The six TTY routines with
    such indices (k_get, set_funcs, set_raw_mode, rcv,
    k_set_input_break_mode, enable_crash_func) get past the index now
    and stop at other refusals (bead domain-pascal-ji9's report).
- **Pointer arithmetic through type transfers (Verified shapes by survey;
  implemented, bead domain-pascal-5aw.17).**  Ref "Type Transfer
  Functions", p. 4-197: a transfer "does not change any bits in the
  argument"; p. 4-199 gives the idiom, `namepointer :=
  UNIV_PTR(INTEGER32(namepointer) + 2)` ("It is illegal to perform
  mathematical operations on a pointer variable.  However, by using type
  transfer functions you can temporarily make a pointer variable into an
  integer variable").  In the image a pointer sum is computed in a data
  register like any integer sum (section 8) and moved to an address
  register, `movea.l Dn,An`, where it is used as one.  Survey (ad-hoc
  script, not kept, over the compiled routines `tools/loopalign.py`
  splits; the last definition of a `movea.l Dn,An` source within six
  instructions, no branch or call between): after `add.l` 42, `addi.l` 8
  (six of them `addi.l #0` on a relocated `move.l #abs`: symbol
  arithmetic, 0xE2F3D2, 0xE32EE2), `addq.l` 6, `subq.l` 5, `subi.l` 1,
  `sub.l` 1; against `adda.l Dn,An` 7, `adda.w Dn,An` 5, `adda.l #imm` 7,
  `adda` from memory 4 (most `adda`s are in 0xE70308..0xE7067A).  The
  shapes:
  - a word offset first, the pointer added: `move.w (14,A3),D4; ext.l D4;
    add.l A3,D4; movea.l D4,A0` (0xE4EFCA..0xE4EFD2; 0xE538B8..0xE538BE
    and 16 more `ext.l; add.l An`); LOG_$READ's worker 0xE177D0..0xE177DA
    `move.l D1,D3; ...; add.l (0x14,A5),D3; ...; movea.l D3,A1`, the
    zero-extended `offset` held since 0xE177A6 (12.4 R7);
  - a constant of 9..127 first, the pointer added, the pointer in memory
    (0xE00AB4 `moveq #30,D0; add.l (-64,A6),D0`, as section 8 says for
    `x + c`) or held in an address register (0xE196DE `moveq #28,D5; add.l
    A1,D5`; 0xE1CF10 `#12`; 0xE4D87C `#18`; 0xE67FEA `#104`; 0xE7140E
    `#16`; also 0xE1CF0A `#8` and 0xE3DCAE `#5`);
  - 1..8 on a copy of the pointer: 0xE4D5A2, 0xE4EA1E, 0xE51ACA `move.l
    A0,D0; addq.l #1,D0`; 0xE028DE `move.l A2,D0; subq.l #4,D0`; 0xE33876,
    0xE532AA `move.l A5,D1; addq.l #4/#8,D1`.

  `dpc` (`dpc-ir` `lower.rs`, `transfer`): a transfer between types of
  one size emits no code (a load or a constant retyped, any other value
  in a same-width `Expr::Convert` the selector evaluates as its operand);
  between integers it converts as an assignment does: widened by the
  operand's sign, or computed at the narrower width where `narrow` can
  (p. 4-198, `INTEGER16(x + y) > 5`); other narrowings are refused (the
  further forms of an assignment, section 8 "Narrowing and widening",
  are not applied to transfers).  The arithmetic is section 8's; the `movea.l` comes from the
  use.  Not modelled: a constant of 9..127 added to a pointer held in a
  register (section 8 refuses `x + c` with `x` in a register; the
  address-register sites above all put the constant first, as do two with
  5 and 8).  Golden `tests/golden/expr/transfer`; test `dpc-ir`
  `type_transfers`.
- **`with` (ref "With", pp. 4-211..4-214; C.3.18; the evaluation
  Verified at one site, the rest Uncertain; implemented, bead
  domain-pascal-5aw.19).**  P. 4-212: "Domain Pascal evaluates the
  expression v1 only once, and this evaluated expression is implied
  within the body of the with statement", and of `WITH p^ DO ... p :=
  nil ...`: "changing p does not affect access to the record identified
  by the with statement".  The image site: NETLOG_$LOG_IT
  0xE71BB6..0xE71BFC computes `count * 26` into D1, then `movea.l
  (116,A5),A0; lea (0,A0,D1.w),A0` and stores eleven fields through A0,
  `(-26,A0)` .. `(-2,A0)`: an element of a 1-based array through a
  pointer, its address formed once with displacement 0, the lower bound
  (-26) folded into each field's displacement (the C,
  `netlog/log_it.c`, has an `entry` pointer local instead; the code is
  the same).  `tools/boolsurvey.py elem` counts the element addresses
  `lea (0,An,Dx),Am` used through Am: 1424, of which 170 serve two or
  more memory operands before a branch, call or label (the shape of a
  `with` or of a common subexpression; not told apart).  A record whose
  address has no run-time part needs no code: its references are those
  of the variable.  **Implemented** (`dpc-ir` `Cx::with_stmt`,
  `Function::with_temps`; `isel.rs` the pointer locals of 12.5,
  `lea_into`):
  - a variable, a field or constant-indexed element of one, a `var`
    parameter's record through its slot: bound to that address, no code
    (`with v : r` binds `r` the same way);
  - `v^` (or a field of it) for a pointer local or by-value parameter
    of the routine that the body does not write, whose address the
    routine never takes or passes, and that no nested routine reaches:
    through `v` as written (loaded at entry when it is a parameter,
    12.2), since it holds the same pointer for the whole body;
  - anything else (a pointer in the unit's data or one the body
    assigns, an element at a run-time index): the run-time part of the
    address, the pointer or the element address `base + index * size`
    without its constant displacement, computed once at the statement's
    start into a temporary that the selector holds in an address
    register for the body (a callee-saved one across calls, 12.4 R3) and
    never stores, the constant displacements in each reference; an
    element through a pointer in memory as at NETLOG_$LOG_IT (index,
    pointer, `lea`).
  Uncertain: whether the original reserved a frame slot for the
  temporary (NETLOG_$LOG_IT's `link #-24` has 6 bytes no instruction
  addresses below its 18 of locals, enough for a 4-byte temporary or a
  local), and whether it copied a pointer already held in a register
  (dpc goes through the variable when it is stable, as above, and
  copies a register-held local's pointer with `movea.l Dn,An`
  otherwise).  Refused: a `with` over a volatile record reached at run
  time; a `goto` into a `with` (ref p. 4-95; 7.5).  Golden `data/with`;
  exec test `withs`; dpc-ir test `with_statements`.
- **`addr(x)` as a value (ref "Addr", p. 4-13; Verified by survey;
  implemented, beads domain-pascal-5aw.19, domain-pascal-j28d).**
  Stored: `lea <ea>,An; move.l An,<dst>` (`lea X,An` directly followed by `move.l An,<mem>`:
  X `(d,A5)` 19 times, 0xE00964 `lea (24,A5),A0; move.l A0,(-108,A6)`;
  `(d,A6)` 18, 0xE11B1A; `(d,An)` 97, 0xE012CC; an element `(d,An,Dx)`
  122, 0xE03576 `lea (-48,A1,D0.l),A2; move.l A2,(-76,A6)`, the lower
  bound folded in; offset 0 of the data area `lea (A5)`, as for copies
  above); into a pointer local held in an address register, `lea`
  straight into it; another unit's object `move.l #sym` / `movea.l
  #sym,An` (1.5).  Passed by value: `pea`, the push of the same address
  by address (1.5).  A type transfer of it to another 4-byte type
  (`integer32(addr(x))`, p. 4-197: no bits changed) is stored and passed
  the same way: NETBUF_$INIT 0xE2F63C `move.l #NETWORK_$BUFPOOL,
  (0x330,A0)`; SOCK_$INIT 0xE2FE4C `lea (0x4,A2),A1; move.l A1,(0xc,A0)`;
  FLP_$DINIT 0xE3E15E `pea (0x80,A5)` for MMU_$VTOP's by-value `va`
  (the ../source reconstructions of those routines; `dpc` peels the
  transfer, `isel.rs` `addr_value`).

  **In `+` and `-`** (bead domain-pascal-j28d; `tools/storesurvey.py
  addr`, over every compiled routine's reachable instructions):
  - *Another unit's object (a map symbol's own address) as an operand
    is an immediate in the immediate-EA encodings.*  In compiled code
    `sub.l #imm,Dn` (`90bc`+) occurs 10 times, `add.l #imm,Dn`
    (`d0bc`+) 3, `cmp.l #imm,Dn` (`b0bc`+) 3, `cmpa.l #imm,An` 1, every
    one with a map symbol's address as the immediate; no
    `addi`/`subi`/`cmpi` has one (and the numeric immediates are all
    `addi`/`subi`/`cmpi`, section 13).  Sites: NET_$OPEN 0xE5A1C0 `sub.l
    #NET_IO_$NIL_DRIVER,D0`, NET_$CLOSE 0xE5A230, NET_$IOCTL 0xE5A28C,
    NET_$RCV 0xE5A2E8, NET_$SEND 0xE5A350, ACL_$INIT 0xE310B2 `#RELOC`,
    PROC2_$GET_CR_REC 0xE40188 `#PROC2_$EC`, AREA_ 0xE078CE and 0xE07A72
    `#NETPOOL_END`, 0xE09E6E `#PIT_PAGES`; `add.l` 0xE0F14E
    `#AST_$COPY_BUFF`, 0xE0F15E `#AST_$ZERO_BUFF`, OS_$INIT 0xE339FA;
    `cmp.l` FIM_$BUILD_DF 0xE0A58E, OS_$INIT 0xE33982, 0xE34452; `cmpa.l`
    FIM_$BUILD_DF 0xE0A552 `#FIM_$COLD_BUS_ERR`.  The other operand is
    in a data register first as in any sum (section 8): a variable
    loaded (0xE09E6A `move.l (8,A2),D3; sub.l #PIT_PAGES,D3`), a
    computed value in place (0xE0F148 `andi.l #1023,D1; add.l
    #AST_$COPY_BUFF,D1`), a pointer held in an address register copied
    (0xE078CC `move.l A3,D0`).  gas writes `addi`/`subi` for `add.l
    #sym`, so `dpc` emits these as `.short` with the symbol a `.long`
    (`insn.rs`, section 13).
  - *An address as the left operand of `-` goes to a data register*:
    another unit's object `move.l #sym,Dn` (OS_$INIT 0xE3397C `move.l
    #VM_TABLES_END,D0`, a compare's left operand; the immediate of the
    stores and pushes); an address inside another unit's object `lea
    (sym+d).l,An; move.l An,Dn` (NET_$OPEN 0xE5A1B8 `lea (0xe2451c).l,A0;
    move.l A0,D0`, NET_IO_$NIL_DRIVER's field at +0x28, `integer32(addr(
    net_io_$nil_driver.svc_open)) - integer32(addr(net_io_$nil_driver))`
    in net/open.pas; the four other NET entries; ACL_$INIT 0xE310AA
    `lea (0xe935cc).l,A0`, the start of the map's FILE_$LOT_DATA segment,
    no symbol: these are all 6 `lea abs.l,An` read as values); any
    other object its `lea`, then `move.l An,Dn` (survey: `lea; move.l
    An,Dn` with X `(d,A5)` 16, `(d,A6)` 5, `(d,An)` 18, indexed 25;
    most are pointer locals held in data registers, 12.5).  A0 and D0 at
    NET's five sites; D1 at ASKNODE_$SERVER 0xE65DA8 and FLP_$DINIT
    0xE3E17E `lea (0x80,A5),A0; move.l A0,D1; andi.l #0x3ff,D1` where D0
    looks free (**Uncertain**; `dpc` takes the lowest free register;
    bead domain-pascal-i2na).
  - *Two addresses subtracted, both needing a `lea`: the subtrahend's
    first* (6 of 6 sites, the `sub.l An,Dn` after `lea`s in compiled
    code): the DIR module-local 0xE4BD48 at 0xE4BDCE `lea (-1304,A6),A0;
    lea (0,A3,D2.l),A1; move.l A1,D0; sub.l A0,D0`, likewise 0xE4BE7A,
    0xE4BF4A, 0xE4BFE4; ASKNODE_$SERVER 0xE65DA0 `lea (-592,A6),A0; lea
    (-582,A6),A1; move.l A1,D1; sub.l A0,D1`; ASKNODE_$INTERNET_INFO
    0xE6519C (the minuend `lea (38,A1),A1` into its base's register: not
    modelled, 12.4 R2).
  - **Refused** (no form settled; bead domain-pascal-awzk): an address
    and a constant (`addr(x) + c`: no `lea; move.l An,Dn; addq/addi` in
    compiled code, and whether the constant went into the `lea`'s
    displacement is not seen); an address that needs a `lea` with a
    value that is not an address (NETBUF_$GETVA
    0xE0ECC8..0xE0ECE2 forms the element's address first, `movea.l
    #NETWORK_$BUFPOOL,A0; ...; lea (0,A0,D3.l),A1`, then the masked
    value, `add.l A1,D3`: one site, of the unsettled element form, bead
    domain-pascal-o1kl; DIR_$GET_ENTRYU 0xE4D648 `clr.l D0; lea
    (12,A2),A0; move.b (1,A2),D0; add.l A0,D0` and the other `add.l
    An,Dn` after a `lea` or `movea.l #`, 8 sites in all, the source's
    operand order unknown); an address formed with a `lea` or an
    immediate together with the address a by-address parameter's slot
    holds in a data register since the entry (12.2; no site seen); the
    sum of two addresses; `&`, `!`, `xor`, `*`, shifts.
  - **Comparisons** with an address: not implemented (refused; bead
    domain-pascal-awzk).  The image compares a symbol's address as an
    immediate operand, `cmp.l
    #sym,Dn` after the variable is loaded (FIM_$BUILD_DF 0xE0A58A `move.l
    (-6,A6),D2; cmp.l #GLOBAL_B,D2; bhi`), a pointer with `cmpa.l
    #sym,An` after loading it (0xE0A54C `movea.l (0x8).l,A3; cmpa.l
    #FIM_$COLD_BUS_ERR,A3; beq`), and two symbols `move.l #sym,D0; cmp.l
    #sym,D0` (OS_$INIT 0xE3397C).  The ../source reconstructions of
    these are not compiled yet, and the forms of an address that needs a
    `lea` are not seen.

  Refused besides: an address inside another unit's object or a constant
  stored or pushed (`move.l #sym+d` / `pea abs.l` for arguments, 1.5:
  not settled), and into a register-held data local.  `dpc-sema` gives
  `addr` the type `univ_ptr` (p. 4-13).  Golden `data/with`,
  `expr/addr_arith`; exec tests `withs`, `addr_values`; driver test
  `addr_values_in_sums_and_differences`.
- **`addr` of a parameter** (bead domain-pascal-mbtx; ad-hoc survey of
  every compiled routine's reachable instructions, the regalloc/a5survey
  routines, script not kept).  Any mode: "Addr", p. 4-13, excepts only
  routines with the internal attribute, and 5.3.2 (p. 5-7) forbids only
  assigning to an `in` parameter, so `addr` of an `in` parameter or a
  part of one is accepted like a `var` one (it was refused until this
  bead; reference-clarifications open question 18; Apollo's SR10.2
  example `domain_examples/streams/sasc/imex.pas` line 81 takes `addr(
  mgr_data )` of an `IN mgr_data: UNIV string`).  The image cannot
  tell the modes apart: the code depends only on how the parameter is
  passed (ref 5.2 Table 5-1, p. 5-5).
  - *Passed by address, the whole object: the address the slot holds
    (Verified at the reconstructed sites).*  Stored, from the frame,
    `move.l (d,A6),<dst>`: MAC_OS_$PUT_INFO 0xE0C322 `move.l
    (0x8,A6),(-0x8,A6)` and PKT_$COPY_TO_PA 0xE12538 (`in`, the latter
    an array under `val_param`), TTY_$I_INIT 0xE33302 `move.l
    (0x8,A6),(0x5c,A3)`, SIO2681_$INIT 0xE3342C, 0xE3344A, 0xE3346E,
    0xE33490, 0xE33496, SIO6509_$INIT 0xE33526 (`var`), as ../source
    reconstructs them.  Compiled code has 58 `move.l (d,A6),<mem>` with
    d > 0 (27 to the frame, 27 through an address register, 4 to the
    unit's data); a by-value pointer parameter stored is the same bytes,
    so only the source tells them apart.  From the register that holds
    the slot, when it is held (12.2): EC_$INIT 0xE15208, 0xE1520C
    `move.l A0,(0x4,A0)` (`out`; conformance unit `ec_init.pas`);
    EC2_$READ 0xE42C94 `move.l D2,(-0x8,A6)` (`in`, held in a data
    register since it is also compared; `dpc` gives D0 where the image
    has D2, srcsurvey MISMATCH, bead domain-pascal-hmz6).  Whether
    such a store counts towards the two uses that load the slot at
    entry (12.2) is **split**: the module-local 0xE32A60 (../source's
    OS_TERM_INIT, os/term_init.c; the default convention, its pointer
    arguments passed through frame cells, 0xE32C2C) loads `(0xc,A6)` at
    entry, 0xE32A6C `movea.l (0xc,A6),A0`, for one dereference (0xE32A9C
    `move.l A2,(0x2c,A0)`) and one store of the address (0xE32A80
    `move.l A0,(0x48,A2)`), no call between; MAC_OS_$PUT_INFO
    dereferences `info` once (0xE0C240 `movea.l (0x8,A6),A1; cmpi.l
    #0x1,(A1)`) and stores its address after a call from the slot
    (0xE0C322), not loaded at entry.  A call between the uses would
    separate them, but that is not checked further, so `dpc` refuses a
    slot whose stores decide it (fewer than two other uses, two or more
    in all; bead domain-pascal-m2w3).  A slot whose one use is the store
    is read in place (PKT_$COPY_TO_PA 0xE12538, no other reference to
    `(0x8,A6)`).  SIO2681_$INIT and SIO6509_$INIT do not follow `dpc`'s
    entry loads at all (a slot loaded at its first statement and kept
    across statements: `(0x28,A6)` into A3 at 0xE33400, its address
    stored from A3 at 0xE33428 and 0xE33450 and from the slot at
    0xE33496 once A3 is reused; `(0x10,A6)` at SIO6509_$INIT stored from
    the slot at 0xE33526, then loaded into A4 at 0xE3354C for three
    dereferences); both are STUB in srcsurvey.  Passed by value
    (`univ_ptr` under `val_param`): the push of the parameter passed on by
    address, `move.l (d,A6),-(SP)` (LOG_$READ 0xE1780C), `pea (An)` or
    `move.l Dn,-(SP)` when held (1.5); the image cannot tell `addr(p)`
    by value from `p` by address.
  - *A field: `lea (k,An),Am` off the loaded slot.*  Stored, `movea.l
    (d,A6),An; lea (k,An),Am; move.l Am,<dst>`, the form `dpc` already
    had: in compiled code 4 to the frame (MAC_$RECEIVE 0xE0BE9A `lea
    (0x1c,A3),A2`, `chain_head := addr(pkt_desc.buffers)` in
    ../source's mac/receive.pas; the module-local 0xE1327E at 0xE132A2;
    EC2_$WAIT 0xE423DA, 0xE423EE), 1 through an address register
    (DISK_$INIT_QUE 0xE3C59E), the sources of the last four not checked;
    passed by value, as by
    address, `movea.l (d,A6),An; pea (k,An)` (1.5,
    KBD_$OUTPUT_BUFFER_DRAINED 0xE1CE9E).
  - *Not `addr`: `lea (An),Am` read as a value.*  Every one of the 29
    `lea (An),Am` (displacement 0) whose register is then moved
    (`tools/storesurvey.py addr -v`; review of this bead) starts a
    pointer for a loop (7.4), never `addr(p)` in ../source: 20 are
    followed by the `dbf` loop that steps the register (e.g. the
    module-local 0xE01872 at 0xE018A4, MAC_$OPEN 0xE0B92E, TTY_$I_INIT
    0xE332F0, which stores `addr(tty)` itself from the slot in that
    loop, 0xE33302 `move.l (0x8,A6),(0x5c,A3)`); 8 go to a frame cell, which is stepped
    (AREA_$COPY 0xE09192, 0xE09198 `addq.l #0x4` 0xE0936E, 0xE0936A;
    MAC_OS_$PUT_INFO 0xE0C2A2 `add.l D4` 0xE0C2FE; MAC_OS_$INIT 0xE2F536
    `addi.l #0xf4` 0xE2F5E6; BAT_$ALLOCATE 0xE3B290, BAT_$FREE 0xE3B588
    `addq.l #0x4` 0xE3B372, 0xE3B6C2; VFMT_$MAIN 0xE6AB50, "the
    compiler's temporary" in vfmt/main.pas) or kept as the start of a
    second loop (AST_$GET_SEG_MAP 0xE06B3A, read again at 0xE06ED2 for
    the loop at 0xE06ED8, `output[i]` in ast/get_seg_map.c); the 29th,
    FLP_$CINIT 0xE3E0E0 `lea (A5),A0; move.l A0,(-0x1c,A6)`, is the
    unit data's object at offset 0 (the `(d,A5)` form above).  Of
    these, the ones off a parameter's slot are the module-locals
    0xE01566 (at 0xE015B8) and 0xE01872, AST_$GET_SEG_MAP, MAC_$OPEN,
    TTY_$I_INIT, BAT_$ALLOC_VTOCE 0xE3B068, BAT_$ALLOCATE, BAT_$FREE and
    VFMT_$MAIN.  So `dpc`'s earlier form for `addr` of a `var` or `out`
    parameter's object stored, `movea.l (d,A6),An; lea (An),Am; move.l
    Am,<dst>`, has no `addr` site and is gone (EC_$INIT, above, is
    byte-identical now).  ../source writes the two BAT ones `cursor :=
    integer32(addr(blocks))`, for which `dpc` emits `move.l
    (d,A6),(-k,A6)`; both routines are refused earlier.
  - *Passed by value (`val_param`, `internal`: an `in` scalar of 4 bytes
    or less): the slot's own address*, as for a value parameter: `pea
    (d,A6)` (76 in compiled code with d > 0, none told apart from the
    parameter passed on by address), stored `lea (d,A6),An; move.l
    An,<dst>` (no compiled site with d > 0: **Uncertain**, the
    frame-object form above, which `dpc` already gave `addr` of a value
    parameter under `val_param`); the slot is then read from the frame at
    each use (12.2, bead domain-pascal-pe9).
  - **Refused:** the whole object's address stored where those stores
    decide the slot's entry load (above; bead domain-pascal-m2w3); an
    enclosing routine's parameter's, stored (the generic form would be
    `lea (An),Am; move.l Am`, which has no `addr` site, above; `addr(p^)`
    of a pointer still takes that form, unverified, bead
    domain-pascal-ucss); a
    part reached through a slot held in a data register,
    passed by value (as by address, 1.5); an element at a run-time index
    passed by value; the target of a type transfer of it,
    `T(addr(x))^[i]` (PKT_$BRK_INTERNET_HDR 0xE12478, PKT_$BLD_INTERNET_HDR
    0xE12300 `pea (-0x1,A2,D3.l)`; bead domain-pascal-u7gg).
  Implemented (`dpc-sema` `Checker::builtin_fn`; `isel.rs`
  `Selector::store`, `Selector::push`, `entry_loads`); golden
  `params/addr_in`, exec test `addr_in`, conformance unit `ec_init.pas`,
  dpc-ir test `addr_of_in_parameters`, driver test
  `addr_of_in_parameter`.
- **Packed arrays (ref 3.11.3 p. 3-45, 3.11.4.2 Table 3-5 p. 3-47, 3.15.5
  p. 3-62, read in the PDF; layout from the reference, element code
  Verified at the sites given; implemented, bead domain-pascal-51e).**
  - *Layout* (`crates/dpc-sema/src/types.rs`, `packed_array_elem`,
    `packed_elem_bits`).  Table 3-5: subranges and enumerations take the
    bits they need ("Exact number of bits required; bit aligned"),
    booleans "1 bit; bit aligned", characters 8 bits, integer 16, integer32
    and pointers 32, records and arrays of non-characters "exact number of
    words"; below the table, for every element: "if the element size is
    less than 16 bits, then the element is padded up to the nearest power
    of 2 bits" (`packed array [1..3] of 0..7`: 4 bits each, "2 bytes").
    3.15.5 p. 3-62: "Domain Pascal supports arrays of 1-, 2-, and 4-bit
    elements.  The order of elements within words is from most
    significant bit to least significant bit", six 4-bit elements in "3
    bytes".  So a packed array of 1-, 2- or 4-bit elements is a bit string
    from the most significant bit of its first byte, in whole bytes (no
    pad byte for an odd count, unlike an unpacked boolean array, 3.11.4.1);
    an integer subrange of 5 to 8 bits is one byte per element, read like
    a `[byte]` subrange (3.11.3: packed arrays store "byte integers"); 9
    to 16 bits and the other types as unpacked.  Refused (reference
    unclear, [reference-clarifications](reference-clarifications.md)
    question 42, or not implemented): elements of 17 to 31 bits (footnote
    2, "word aligned"), sets, varying strings, one-byte enumerations, arrays
    of non-characters of an odd size, a bit size attribute.  Image:
    PCHIST's `pchist_$proc_bitmap: packed array[1..64] of boolean` (../source
    `pchist/pchist.pas`), 8 bytes at +0x18 of PCHIST's WIRED_DATA block
    (0xE2C204), element p in bit 7 - (p - 1) mod 8 of byte (p - 1) div 8
    (below); ACL's three 64-bit bitmaps (`acl/acl_internal.h`), the same
    bit order.  Initial values: element k is the bit field `(k - lo) * n`,
    width n, as a packed record's (`check.rs`, `init`); none in the image
    (the bitmaps are zero).  FIM_'s frame size table (FIM_'s block
    0xE2126C+0x124, 16 bytes) is declared `packed array[0..15] of 0..255`
    in `tests/conformance/fim_get_fim_addr.pas` (byte-identical data);
    FIM_$BUILD_DF reads it as unsigned bytes (0xE0A486..0xE0A492 `clr.l D2;
    ...; lea (0,A5,D1.w),A2; move.b (0x124,A2),D2`).
  - **Packed arrays of booleans: the elements (Verified at the sites
    given).**  An ad-hoc survey of compiled code (every `btst`/`bset`/
    `bclr`/`or.b`/`and.b` with a register operand on memory, with `#7`,
    `#0x80`, `#-0x81` or `lsr #3` in the 14 instructions before it, read
    by hand; not kept) finds bit-array elements only in PCHIST and ACL_,
    always a 64-element 1-based boolean array indexed by a word variable
    (a process number): 9 tests and 9 stores of a constant, and 2 stores
    of a computed value found from the bitmaps' displacements.
    - *Test* (`btst`, then `beq`/`bne` or `sne`): on the index's **low
      byte**, 9 sites, PCHIST_$COUNT 0xE1A156..0xE1A16C:
      `move.b D2,D0; clr.w D1; subq.b #1,D0; move.b D0,D1; lsr.w #3,D1;
      lea (0,A5,D1.w),A1; moveq #7,D1; moveq #7,D3; and.b D0,D1; sub.b
      D1,D3; btst D3,(0x18,A1); beq`.  The byte from memory is read at the
      word's address + 1: PCHIST_$UNIX_PROFIL_CNTL 0xE5CB3A `move.b
      (-0x21,A6),D0` (pid at -0x22; again 0xE5CBA8), PCHIST_$UNIX_PROFIL_FORK
      0xE5CC44 `move.b (0x00e20609).l,D0` (PROC1_$CURRENT, 0xE20608) and
      0xE5CC9E `move.b (0x1,A2),D0` (an `in` parameter's address held in
      A2).  The element's byte is always formed with `lea (0,An,Dx.w),Am`
      and the array's offset kept for the operand, `(0x18,Am)`, though the
      indexed mode's 8-bit displacement would hold it.  As a function
      result: ACL_$USED_SUSER 0xE492DA..0xE492FE, `btst D2,(-15568,A1);
      sne D0`.  With other bases: ACL_$SET_ACL_CHECK 0xE47384, ACL_$ALLOC_ASID
      0xE73C9A, the module-local 0xE464B8 at 0xE4655C (whose bit number is
      `andi.b #7,D0; ...; moveq #7,D1; sub.b D0,D1`).
    - *Set and clear* (`or.b`/`and.b` of a mask): on the index's **word**,
      9 sites, PCHIST_$UNIX_PROFIL_FORK 0xE5CCDC..0xE5CCF4: `move.w
      (A2),D0; moveq #7,D1; subq.w #1,D0; move.l #0x80,D2; and.w D0,D1;
      lsr.l D1,D2; move.w D0,D1; lsr.w #3,D1; lea (0,A5,D1.w),A1; or.b
      D2,(0x18,A1)`; cleared with `move.l #-0x81` (0xFFFFFF7F shifted right:
      all of the low byte but the element's bit) and `and.b`
      (PCHIST_$UNIX_PROFIL_CNTL 0xE5CBF2..0xE5CC0C).  The same at ACL_$INIT
      0xE310E4 (a `for` index held in D3, `move.w D3,D0`),
      ACL_$SET_LOCAL_LOCKSMITH 0xE49204, the module-local 0xE463E4 at
      0xE4646C, ACL_$SET_ACL_CHECK 0xE473AC, ACL_$FREE_ASID 0xE74D5E (set,
      then a clear of another bitmap reusing the count and the address,
      0xE74D80..0xE74D88).
    - *A computed value stored*: 2 sites, ACL_$INHERIT_SUBSYS
      0xE49146..0xE49182 and ACL_$OVERRIDE_LOCAL_LOCKSMITH 0xE49286..0xE492BC:
      the bit number stored to a frame byte and reloaded as the shift count,
      the mask complemented, the byte and the value masked and `or`ed,
      `move.b` back.
    - **Implemented** (`isel.rs`, `bit_elem_test`, `store_bit_elem`; IR
      `Expr::BitElem`, `StmtKind::StoreBitElem`): the test and the
      set/clear forms above for an array in the unit's data through A5,
      the index a word variable (register-held, in the frame, the unit's
      or another unit's, through a held address), a lower bound of 1..8
      (`subq`); an element as a condition or as the result of a
      one-statement function.  Every site's array has 64 elements and a
      lower bound of 1: other sizes (up to 256 for a test) and lower
      bounds 2..8 take the same forms by extrapolation, unverified.  The test is one scheduling unit (R11's
      window would put the second `moveq #7` before the `lea`, which no
      site has); the stores are left to R11, which keeps the order.  The
      allocator gives the store's byte offset the count's register, free
      after the `lsr.l` (FORK, ACL_$INIT, LOCKSMITH).  **Not reproduced**
      (bead domain-pascal-0bkd): PCHIST_$UNIX_PROFIL_CNTL 0xE5CB6C, 0xE4647A
      and ACL_$SET_ACL_CHECK 0xE473BE take a new register for it (D3, D5,
      D7), its copy then before the `lsr.l`; ACL_$USED_SUSER's test has the
      position and the offset in D1 and D0; the 0xE4655C bit number; other
      bases (ACL's `movea.l #ACL_$DATA+0xEA60,An`, then the `lea` into An at
      ACL_$ALLOC_ASID 0xE73CAE but into another register at ACL_$USED_SUSER
      0xE492EE); ACL_$FREE_ASID's reuse across two statements.
      **Refused**: a computed value stored (bead domain-pascal-ysp5), and
      the forms no site shows (bead domain-pascal-pubp): a constant index,
      another lower bound, a test on more than 256 elements (the byte
      arithmetic), a computed or non-word index, an element as another
      value or passed by address, arrays in the frame, through a pointer or
      of another unit, 2- and 4-bit elements.  Tests: golden
      `packed/bool_arrays`, `crates/dpc-m68k/tests/packed_arrays.rs`,
      `crates/dpc-sema/src/tests/packed_arrays.rs`, `crates/dpc-ir`
      `packed_boolean_elements`, exec tests `packed_bools` and
      `packed_bools_all` (every element of a 64-element array set and
      cleared alone, every byte and element read back; 256-element arrays;
      bit and byte initial values).  No routine is
      a conformance unit yet: PCHIST's routines also need overlay sections
      (bead domain-pascal-raxl) and other forms; ACL's bitmaps have the
      refused base.
- **Packed records / bit fields:** see 5.3.

### 5.3 Packed records (ref 3.10.7; layout Verified by cross-check, codegen Verified at the sites given; implemented, bead domain-pascal-xlm)

**Layout** (`crates/dpc-sema/src/types.rs`, `packed_field_bits`,
`layout_packed_fields`, `packed_record_layout`; ref 3.10.7 pp. 3-35..3-37
and Table 3-3 read in the PDF, the OCR scrambles the table):

- The record is a big-endian bit string: "Pascal allocates fields left to
  right within bytes and then by increasing byte address" (p. 3-35), so
  the first field takes the most significant bits of the first byte.
  Figure 3-17 (p. 3-37) and the image agree: `status_$t.fail`, the first
  bit, is `bset #7` on the record's first byte (PROC2_$RESUME 0xE41444).
- Bits per field (Table 3-3, p. 3-36): integer 16, word aligned;
  integer32, real 32 and double 64, word aligned; boolean 1; char and
  subranges of char 8, byte aligned; an enumeration the bits of its
  largest ordinal; another subrange "the number of bits required for
  their extreme values" (a negative low bound adds a sign bit:
  `-100..40` is 8 bits, p. 3-37), bit aligned; a set of fewer than 32
  elements one bit per element, else its size, bit aligned; arrays,
  records and pointers their ordinary size, word aligned ("Structured
  types, except for sets, are aligned on word boundaries", p. 3-35).
  The list under Table 3-3 (p. 3-37) annotates `Class : 'A'..'Z'` "8
  bits (word aligned)" against the table's "byte aligned"; in the
  example both put it at bit 48, so it does not decide.  The table is
  followed (Uncertain: no image or insert case found that separates the
  two).
- **A subrange needing 31 bits takes 32** (ref 3.4 p. 3-10: "When you have
  an unsigned subrange that is large enough to require 31 bits, the
  compiler allocates 32 bits").  Apollo's `linteger = 0..2147483647` is
  32 bits in its packed records; the alternative, 31 bits, breaks the
  checks below.
- Placement (p. 3-35): the first field at bit 0; then a field starts at
  the next free bit unless it would cross two or more 16-bit boundaries
  (a boundary strictly inside the field), then at the next 16-bit
  boundary.  Byte- and word-aligned types round up to the byte or word.
  Variant arms all start where the fixed part (and tag) ends.
- Size: the bits used rounded up to whole words, at least 16 bits (p.
  3-35; Figure 3-17: 56 bits used, "eight bytes").  Alignment of a packed
  record as a variable: the allocation algorithm of 3.10.5.6 applied to
  the fields that are whole objects of their type (Uncertain: stated for
  records in general); as a field, a word.
- A set whose base type does not start at ordinal 0 is refused: how many
  bits "one bit per element" gives it is not settled (an open question
  for the owner, not yet asked).

Cross-checks (`crates/dpc-sema/src/tests/packed.rs`; declarations copied
from the SR10.2 customer inserts, section 0.1):

| Declaration (insert) | Check | Source of the check |
|---|---|---|
| `status_$t` (base) | `fail` bit 0, `subsys` 1..7, `modc` 8..15, `code` the word at 2, `all` the longword; 4 bytes, longword aligned | image 0xE41444 `bset #7,(-0x4,A6)`, 0xE4142A `tst.w (-0x2,A6)`, PROC2_$FORK 0xE72CC2/0xE72CC8 (section 9) |
| `uid_$t` (base) | `high` 0, `low` 4, 8 bytes | `../source` base/base.h `_Static_assert`s |
| `time_$clock_t` (base, the `%ELSE` form) | `high` 0 (32 bits), `low` 4; `high16` 0, `low32` 2 (32 bits); 6 bytes | base/base.h size 6; tty/tty.h 0x2C4/0x2C8; image TTY_$I_WAIT 0xE1C2AA `clr.w (-0x60,A6)`, 0xE1C2C0 `move.l D0,(-0x5e,A6)` |
| `cal_$timezone_rec_t`, `cal_$timedate_rec_t` (cal) | 0, 2, 6; 12 bytes | cal/cal.h `_Static_assert`s |
| `fontn_$table_tver1` (fontn) | `image_offset` 0x28, `image_size` 0x2C, `index_table_offset` 0x34, `desc_table_offset` 0x38, `inter_space_dflt` 0x5A, `space_size` 0x6E; `old_proportional` (1 bit) and `space_type` (`0..16#7FFF`, 15 bits) one word; 128 bytes | smd/smd_internal.h `smd_font_v3_t` `_Static_assert`s; the insert's "8-byte multiple" |
| `smd_$font_table_t`, `smd_$v1_fdte_t` (smdu), `fontn_$desc` (fontn) | 13 words, `index_table` 0x1A, `desc_table` 0x9A; entries of 8 bytes, bytes 0..5, word at 6 | smd/smd_internal.h `smd_font_v1_t`, `smd_glyph_metrics_t` |
| `smd_$display_modereg_t` (smdu) | masks 8000, 7E00, 100, 80, ..., 1 | the insert's own comments |
| `stream_$IR_rec_t` (streams) | `hdr_lgth` at bit 112 | the insert's `unused6 : set of 0..21; { nxt field needs word alignment }` (needs 32-bit `linteger` and n+1 bits for `set of 0..n`) |
| `num_var_t` (prf) | `int16` at bit 16, `int8` at bit 24 (`pad_byte: 0..16777215` is 24 bits) | the insert's padding variants |
| `Characteristics` (ref Figure 3-17) | Mass 0, Shape 32, B 35, Purpose 36, Low_temp 40, Class 48; 8 bytes | the figure |

**Code** (`crates/dpc-ir/src/lower.rs` `bit_field`, `store_bits`;
`crates/dpc-m68k/src/isel.rs` `bits_value`, `store_bits`; 68010: no
`bfext`/`bfins`).  A field that is a whole object of its type at a byte
(byte) or even (word, longword) address is read and written like an
unpacked field (`status.all`, `status.code`, `uid.high`, a char).  Any
other is a **bit field**, reached through a *unit*: the byte it lies in;
else the word at an even address; else the longword at the even address
at or below its first byte.  Survey (ad-hoc scripts over the compiled
routines `tools/regalloc.py` splits, not kept): `andi #~mask,<mem>` with a
contiguous field mask followed within four instructions by `or` into
the same operand: byte units 93 one-bit and 50 wider merges of a
register, 31 of a constant; word units 10 (and one `ori.b`, below);
longword units 57 of a register, 4 of a constant.  Byte units: 0xE0089A, 0xE158BA; word: 0xE00994 `andi.w #-0x7f81,(40,A2)`,
0xE5BFBE; longword across an odd byte: 0xE0086E `andi.l
#-0x7f8001,(38,A2)` (bits 9..16 of 0x26), 0xE0E346 (a 16-bit field in
bytes 5..6).  Many `bset`/`bclr`/`btst #k,<mem>` are set operations on
sets in memory rather than packed fields (SIO's `state` at 0x74, a word
set: 0xE1C6BE `bclr #0,(0x75,A2)`; TTY's flags); the forms coincide for
a constant member (5.1, "Set operators": `dpc` gives `s := s + [c]` the
same `bset #c mod 8` on the member's byte).

- **Boolean field, test:** `btst #k,M` and `beq`/`bne` (0xE0088A); bit 7
  as the sign, `tst.b M` and `bpl`/`bmi` (no `btst #7` on memory in
  compiled code: k = 0..6 only, 422 sites).
- **Boolean field, value:** `btst #k,M; sne Dn` (0xE16F0C; XPD_$INHERIT_
  PTRACE_OPTIONS 0xE5B17C, conformance unit
  `tests/conformance/xpd_inherit_ptrace_options.pas`); bit 7 `tst.b M;
  smi Dn` (0xE066B2, 0xE025E4).
- **Boolean field, constant:** `bset #k,M` / `bclr #k,M` (0xE41444,
  0xE00E02, 0xE5DF08 in FILE_$SET_PROT).
- **Boolean field, value:** bit 7: `andi.b #0x7f,M; andi.b #0x80,Dn; or.b
  Dn,M` with the value computed first (0xE00892..0xE008A4 `btst; sne D0;
  andi.b #127,(39,A2); andi.b #-128,D0; or.b`; OS_$GET_EC 0xE6D6DE `tst.l
  (A2); sne D0; ...`), a variable read after the `andi` (0xE5BFA2), a held
  one copied then (0xE1D1EE).  Bit k < 7: the value made 0/1 first,
  `lsr.b #7,Dn`, then `andi.b #~(1<<k),M`, `lsl.b #k,Dn` (`add.b Dn,Dn`
  for 1, none for 0), `or.b Dn,M` (e.g. 0xE04D0E, 0xE1D4B2 `add.b`,
  0xE04A96 bit 0; from a variable 0xE04C5C `move.b (A1),D1; lsr.b #7,D1`; from
  a bit-7 field 0xE025E4 `tst.b; smi; lsr.b #7`).
- **Wider field, value:** `move.W #mask,Dn; and.U M,Dn; lsr.W #s,Dn`, W the
  wider of the unit and the value, the mask a `moveq` when it fits, the
  shift split at 8: RIP 0xE158B6 `move.w #192,D0; and.b (16,A0),D0; lsr.w
  #6,D0`, 0xE0E00A `moveq #124,D1; and.b (-1012,A4),D1; lsr.w #2,D1`,
  0xE5BFBE (word), 0xE15868 (`move.l #0xfffff`, no shift), 0xE106A0
  (`lsr.l #8; lsr.l #7`).  A field at the top of a unit as wide as the
  value: `move.U M,Dn; lsr.U #s,Dn` (0xE08434, 0xE0A232).  Against 0, the
  branch reads the flags of the `and` or the shift (0xE1BFFE, 0xE5BFBE).
  Fields of negative values (sign extension) are refused: not catalogued.
- **Wider field, constant:** `andi.U #~mask,M; ori #c<<s,M` (0xE158CE);
  the `andi` alone for 0 (0xE1565E), the `ori` alone when every bit is set
  (0xE15656).  The `ori` is unit-wide (0xE0A1DE `ori.l #16,(8,A0)`;
  0xE33E96; 0xE6605A; 0xE4A834), except a word's whose low byte is 0, an
  `ori.b` of the high byte (0xE008CE `ori.b #1,(40,A2)`; one site).  A
  longword's with a zero low byte stays an `ori.l` when it has bits in
  both halves, which no narrower `ori` could set (0xE4A83C `ori.l
  #0x8001e00,(50,A0)` after 0xE4A834 `andi.l #255,(50,A0)`; one site);
  with bits in one half only it is refused (not catalogued); so is a
  constant into a one-bit field that is not a boolean.  A lead for the
  unit (site Verified, reading Uncertain): KBD_$RCV 0xE1CD32 `moveq
  #15,D0; or.w D0,(A3)` sets the low 4 bits of byte 1 of a 2-byte
  record, the "every bit set" `or` but on the record's word and through
  a register, where the reads of the same bits use the byte, `moveq
  #15,D0; and.b (1,A3),D0; cmpi.w #15,D0` (0xE1CE1E, and module-local
  code at 0xE1CBC8).  ../source `kbd/kbd.ins.pas` reads it as a 4-bit
  packed field (`next_state := 15`), its C as an integer `|= 0xF` (5.1,
  the six constant `and`/`or` through a register; beads
  domain-pascal-iv5, domain-pascal-d89) (lead: ../source
  pascal-translation-notes finding 31).  An 8-bit field
  of a wider type on a byte boundary (`0..255`) is refused: the general
  forms would give `andi.b #0,M`, which the image never has, and how
  the original read and wrote it is not catalogued (a lead:
  0xE15894..0xE15898 `clr.w D0; move.b (15,A0),D0; cmp.w`, a byte of
  unknown type next to RIP's packed `state` byte).
- **Wider field, value:** `andi.U #~mask,M`, the value shifted up (`lsl`
  split at 8, `add` for 1), `or.U Dn,M`.  A variable is read only as wide
  as the field needs (a byte up to 8 bits, a word up to 16), at its low
  end: 0xE18192 `clr.w D1; move.b 0xe2060b,D1` for the word 0xE2060A
  (compared as a word at 0xE18A0C); all the byte-merge sources read from
  memory are at odd addresses (0xE1D220 `move.b (133,A3),D1`).  Read at
  the unit's width it comes after the `andi` (0xE0DFB0 `andi.w
  #-512,(-1014,A4); move.w (14,A3),D0; or.w`; 0xE0B372; a held register
  copied, PACCT 0xE5AA00..0xE5AA16 `andi.b #31,(-8,A6); move.b D1,D3;
  lsl.b #5,D3; or.b D3,(-8,A6); andi.w #-8192,(-8,A6); move.w D0,D3; or.w
  D3,(-8,A6)`); narrower, it is zero-extended before (byte to word `clr.w;
  move.b`, 0xE00994, 0xE0F216; byte to longword `move.b; andi.l #255`,
  0xE0086E; word to longword `clr.l; move.w`, 0xE0E346).  A computed value
  is used at its own width or wider (0xE106A0: a longword merged into a
  byte); narrower ones are refused.  **Masking (Uncertain):** the shifted
  value is and-ed with the field mask when its type admits negative
  values (0xE31DF2: `ext.w`, then `lsl.w #4; andi.w #0x3f0`), not when it
  is non-negative however wide (PACCT 0xE5AA14: an unsigned longword,
  compared with `bcc`, merged into 13 bits unmasked); the masked byte
  merges 0xE38C72 and 0xE5BD6E have sources of unknown type.  So an
  unmasked value outside the field's range spills into the neighbouring
  bits, in dpc as in the image: PACCT's 0xE5A9CA keeps its longword
  below 8192 by a loop (0xE5A9E6 `cmpi.l #8192,D0; bcc`), not by a mask
  (an out-of-range value is an error in Pascal anyway).  Not settled:
  the same routine merges its exponent, a word counter (`clr.w D1`,
  `addq.w #1,D1`) of unknown type, into 3 bits unmasked (0xE5AA06
  `move.b D1,D3; lsl.b #5,D3; or.b`); if that counter is an `integer`,
  the rule is wrong.
- **Refused:** a packed field passed to `var`/`out`/`in out` or to
  `addr` (ref 3.10.3 p. 3-21: "you cannot pass a field from a packed
  record as an argument"; an error), passed by value (not seen), copied
  into another field (the image copies at the same position through the
  unit, masked in place: 0xE19DE6 `move.b (2,A0),D0; andi.b #96,D0; or.b
  D0,(2,A1)`; the general form is not catalogued), a destination and a
  value that both need a register (the order of 12.4 "Stores whose
  destination and value both need a register" is not applied to bit
  fields: not seen), set fields as
  operands.  Packed arrays of bit elements: 5.2, "Packed arrays".
- Tests: `crates/dpc-m68k/tests/packed.rs` (the forms, the refusals, and
  a model of the instructions run over random records: stores keep the
  neighbouring bits, loads give the value), `crates/dpc-ir/src/tests.rs`
  (`packed_field_units`), golden `tests/golden/packed/fields`.

### 5.4 Alignment attributes and environments (ref 3.10.5, 3.10.6, 3.15.6, 4.7; layout from the reference's figures; implemented, bead domain-pascal-l9p)

**Layout** (`crates/dpc-sema/src/types.rs`: `layout_fields`,
`record_layout`, `record_spec`, `attributed`; `check.rs`: `attribute`,
`apply_attrs`, `lay_out`).  The figures were read in the PDF (pp.
3-65..3-73); the OCR loses them.

- `aligned(n)` is 2**n-byte alignment, `aligned` alone `aligned(0)`
  (byte); `natural` the type's natural alignment (ref 3.15.6.1 p. 3-63).
  An attribute overrides the default rules and any alignment directive
  (3.15.6.2 p. 3-64): the object, freestanding or as a field, and the
  record allocation algorithm (3.10.5.6) take it.  A record's size is
  rounded up to it, never below a multiple of 2 (3.15.6.3 p. 3-70:
  "Records created by Domain Pascal are always 2-byte multiples").  Other
  types keep their size.  `aligned(3)`/`aligned(4)` are refused (not
  needed so far); so is an alignment attribute on a set or varying type
  (their alignment is not stated), and an array of elements aligned on
  more than their size (whether they are padded is not stated).
- A record inherits the largest alignment specification of its fields
  (3.15.6.2 p. 3-67): Figure 3-21, `S = record a: [NATURAL] integer32;
  b: integer16 end` is 8 bytes, longword aligned (likewise
  `[ALIGNED(2)]` on the record or on `a`); Figure 3-22, `S1 = record x:
  integer16; y: S; z: integer16 end` has `y` at 4, `z` at 12, 16 bytes.
- `aligned(0)` removes padding (3.15.6.3, Figures 3-23/3-24: `a: char;
  b, c: [ALIGNED(0)] integer16` at 0, 1, 3, 6 bytes), except that "a
  byte aligned field may not cross two word boundaries" (3.10.5.5 p.
  3-26): a byte-aligned integer32 after a char goes to 2 (Figure 3-27
  p. 3-72, "the same layout as specifying word alignment").
- `%natural_alignment` (ref 4.7 p. 4-57) lays records out by the natural
  rules: each field at its natural alignment, the record aligned on and
  rounded to its largest (Figure 3-25: `a: char; b: integer32; c: char`
  at 0, 4, 8, 12 bytes); `[ALIGNED(1)]` on that record removes the final
  word (Figure 3-26, 10 bytes), on `b` too moves `b` to 2 (Figure 3-27,
  8 bytes).  `%word_alignment` restores the default, `%push_alignment` /
  `%pop_alignment` save and restore the mode (p. 4-58; Table 4-11 p. 4-45
  swaps them, reference-clarifications "errata").  A record laid out by
  the natural rules carries its alignment as a specification, so it is
  aligned the same where it is used (3.15.6.3 p. 3-70: "The
  %natural_alignment directive applies not only to each field, but also
  to the entire record").  `aligned record` is laid out by the natural
  rules and `unaligned record` by the word rules whatever the
  environment (3.10.6, Figures 3-15/3-16).  Refused under the natural
  rules: array, set and varying fields without an attribute (ref
  3.10.5.2 gives the natural alignment of simple types and records only).
  The kernel is compiled in word mode; no image site of the natural rules
  is known.
- Packed records: the packing rules do not depend on the environment
  (3.10.7).  An attribute on a packed record aligns it (`time_$clock_t`);
  one on a packed record's field is accepted where it agrees with Table
  3-3 (`aligned(1)` on a word-aligned field, `cal_$timezone_rec_t`'s
  `drift: [aligned(1)] time_$clock_t`), refused otherwise.
- Apollo's inserts (section 0.1), checked in
  `crates/dpc-sema/src/tests/attrs.rs`: base.ins.pas `time_$clock_t =
  [aligned(1)] PACKED RECORD` under `%PUSH_ALIGNMENT; %WORD_ALIGNMENT`
  is 6 bytes, word aligned (`../source` base/base.h `_Static_assert`s:
  size 6, `_Alignof` 2; without the attribute the 3.10.5.6 algorithm
  would allocate it on a longword, its `high` being a naturally aligned
  `linteger`); `ec2_$eventcount_t = [volatile] RECORD value:
  [aligned(1)] integer32; awaiters: pinteger END` is 6 bytes, value at 0,
  awaiters at 4, word aligned (ec/ec.h "Size: 6 bytes"); tty.ins.pas's
  `timer: [aligned(1)] linteger` is at 4 of an 8-byte variant.
  `dpc --check-only` over a copy of SR10.2 base.ins.pas is now clean
  (before: the `[aligned(1)]` and `[volatile]` refusals, so every insert
  that includes it failed), and over each of the other 55 inserts, with
  base.ins.pas included first, 31 are clean; the other 24 fail for other
  reasons (types from inserts not included, reals, set and record
  function results, 4 files not UTF-8, an unequal-size type transfer in
  kbd.ins.pas, and rws.ins.pas's attribute on a parameter type, `OUT
  Status: [aligned(1)] Status_$T`, which ref 5.1 p. 5-2 does not allow
  but Apollo's compiler evidently did).
- **What `aligned(n)` changes in the code on the 68000 family: nothing
  observed.**  Word alignment is all the 68010 needs; a word-aligned
  integer32 is referenced with `.l` instructions like any other (ref
  3.15.6.6 describes split references for objects known not to be
  naturally aligned "Series 10000 Only").  EC2_$INIT 0xE42C72 clears an
  eventcount with `clr.l (A0); clr.w (4,A0)`, EC2_$ADVANCE 0xE42CD0
  increments its `value` with `addq.l #1,(A0)` (`ec2/` in `../source`).
  A byte-aligned (`aligned(0)`) object of more than one byte at an
  address not known to be even is refused when referenced (`dpc-ir`,
  `check_even`): the 68010 traps on an odd word or longword access, and
  how the original referenced one is not known.  This covers loads,
  stores, by-value arguments, block copies and `for` fills, and an
  element or field of a byte-aligned array (whose own type has no
  attribute), and a target reached through a pointer whose base type is
  byte aligned (ref 3.15.6.7 pp. 3-75..3-76); objects passed by address
  are assumed naturally aligned by the callee (ref 3.15.6.8 p. 3-76).
- More `%push_alignment` than `%pop_alignment` directives is an error
  (ref 9 p. 9-45, "Unmatched %PUSH_ALIGNMENT directive"), as is the
  reverse ("Alignment stack underflow").

### 5.5 Volatile, atomic and device references (ref 3.15.1-3.15.3, 3.15.7; implemented, bead domain-pascal-t36)

**The rules** (ref pp. 3-55..3-59, 3-76..3-77):

- `volatile`: "memory contents may change in a way that the compiler
  cannot predict"; the compiler must not keep the value in a register or
  assume it unchanged (the 3.15.1 example: `i: [volatile] integer`,
  modified through a pointer by a called routine).
- `atomic` (scalar variables only: "integer, Boolean, character, and
  enumerated"): implies volatile; an assignment whose right side contains
  the variable is done so that other processes cannot change it in
  between; Table 3-6: "perform updates with single instruction when
  possible".
- `device`: implies volatile; also no merging of adjacent references
  ("the compiler optimizes the two 16-bit assignments by merging them
  into one 32-bit assignment") and no "gratuitous read-modify-write
  references ... instructions that do unnecessary reads"; `device(read)`
  / `device(write)` make writes / reads errors (p. 3-59).
- Inheritance (3.15.7): a type defined from a volatile or device type
  inherits it (`resource = array[1..10] of semaphore`); every field of a
  volatile or device record is volatile or device; a device field makes
  its record device, a volatile field does not make its record volatile;
  pointer types do not inherit, a dereference applies the target type's
  attributes.

**Implementation.**  `dpc-sema` gives an attributed type its own id
whose `canon` is the original for compatibility (`semaphore` is still an
integer).  `dpc-ir` wraps the address of every load, store and bit field
of a volatile designator in `Addr::Volatile` (with `device` and `atomic`
flags; `lower.rs`, `volatile_access`, `access_of`); taking an address
(`addr`, passing by address) is not a reference and is not wrapped.  The
selector (`isel.rs`) and the scheduler (`sched.rs`) then:

- **reuse nothing:** a volatile variable is not a CSE leaf (12.4 R7), a
  volatile local is not a register candidate (12.5) and stays in its
  frame slot, a stored value is not forwarded, a test right after a store
  reads the object again rather than branching on the store's flags
  (7.1), a volatile value parameter is read from its slot at each use
  (not loaded at entry, 12.2).  Golden `stores/volatile`: FIM_$ACKNOWLEDGE's
  shape with a volatile `proc1_$as_id` re-reads and rescales it in each
  statement where golden `regalloc/cse` reuses `D2`;
- **keep the order:** the instructions that reference volatile objects
  are marked, and the scheduler orders any two of them (`sched.rs`,
  `volatile_references_keep_their_order`); other loads may pass them
  (WIN_'s module-local 0xE190C8 reads TIME_$CLOCKH among the entry
  loads; whether the original ordered more is not seen);
- **device:** a zero is stored with `move.<s> #0` and `true` with `move.b
  #0xff`, never `clr`/`st` (which read their operand on the 68000/68010;
  section 13 "device registers": WIN_ 0xE19112, RING 0xE758E2, 0xE0A33E);
  no in-place update (`addq`, `addi`, `andi`, `ori`, `bset`/`bclr` to
  memory): `x := x + 1` is a load, an add and a store (**Uncertain**: no
  image site of an update of a device register is catalogued); a
  read-modify-write instruction on a device operand is refused after
  selection, so no path can produce one silently; two adjacent device
  stores are not a merging candidate (the refusal of section 13, "Store
  merging", does not apply); a `for` fill of device elements falls back
  to the loop, storing `move.<s> #0` per element;
- **device pointers in parameters (Verified at one site):** a by-value
  pointer parameter or a by-address slot through which a device object
  is referenced is read from its frame slot before each reference, into
  the lowest free register, not loaded at entry: the module-local
  0xE0A328 (DMA_; `dma/init_m68450_channel.c`) reloads `movea.l
  (8,A6),A0` before each of its five stores to the MC68450 channel
  registers, while loading its word parameter at entry (`move.w
  (0xc,A6),D0`), against the rule of 12.2 for parameters used more than
  once.  How its parameter was declared is not known; the device
  reading explains both the reloads and the `move.b #n` stores.  The
  conformance unit `tests/conformance/dma_init_m68450_channel.pas` is
  byte-identical.  A pointer *local* keeps the normal rules: WIN_'s
  0xE190BC keeps the register block's address in A0 across its four
  device references (0xE190EC..0xE19112).  A parameter both used as a
  device base and otherwise is refused (not seen);
- **atomic:** `x := x op c` (`+`, `-`, `&`, `!` with a constant) is the
  one-instruction in-place update (`addq`, `subi`, `andi`, `ori` to
  memory); any other assignment whose right side reads the variable is
  refused (what the original did "when not possible" is not known), as
  is an atomic device update.  No image site of `atomic` is known.
- **refused:** a store into a packed field of a volatile or device
  record (the field form reads its unit and writes it back, twice for a
  field of several bits: `andi` then `ori`; for a device a gratuitous
  read-modify-write, for a volatile object a shape not seen); copying a
  record or array with volatile or device parts
  (the block copy's shape for them is not seen), by assignment or as a
  value parameter's copy on entry (`move.l (A0)+` would merge device
  references), `in` on a volatile set
  of more than 4 bytes in memory, a volatile `for` index, `address`
  (3.15.4) and the size attributes (3.15.5).
- `device(read)` / `device(write)` violations are errors (`dpc-ir`,
  `check_device_access`).

---

## 6. Nested routines (Verified by survey; implemented for one level)

PMAP_$FLUSH is at 0xE1376C.  Its nested routines are at 0xE1359C and
0xE1360C, and they come **before** their parent in the code.

**Survey** (bead domain-pascal-5aw.3; `tools/nestsurvey.py`, routines split
as `tools/a5survey.py` does, reachable instructions only).  A routine is
taken as nested when it is not map-named (a nested routine is a local
symbol, ref 5.5.4 p. 5-17) and either reads A1 before writing it on its
straight-line entry path, or moves from `(A6)` (a `lea (A6)` is not a
read of the link: 0xE4C02C, 0xE7649E were false hits).  Every `bsr` is
classified by what its caller last wrote to A1 before it.  86 nested
routines (`summary`): 11 take the link in A1, 75 read `(A6)`.

- **Static link in A1.** The caller loads the enclosing routine's frame
  pointer into **A1** before the `bsr`:
  - From the parent: `movea.l A6,A1; bsr.w 0xE1359C` (0xE13928, 0xE13978).
  - From a sibling nested routine: `movea.l A4,A1; bsr.w 0xE1359C`
    (0xE13718), where A4 = that sibling's `(A6)` = the parent's frame.
  - From the routine itself (recursion): 0xE397D0, 0xE39BC2, 0xE4F3BA,
    0xE50D5E each call themselves passing their own copy of A1.
  - The callee copies it: 0xE135A4 `movea.l A1,A2`, then reaches the
    parent's locals as `(d,A2)`.
- **Dynamic link used as the static link.** 0xE1360C is called only from its
  parent, and the parent does not load A1 (0xE13856, 0xE13950, 0xE13992).
  The callee does `movea.l (A6),A4` (0xE13616): the saved caller frame
  pointer, which here *is* the parent frame.  Also PROC2_$SUSPEND's
  0xE4120C (`movea.l (A6),A2` at 0xE41214), RIP_$SEND's 0xE870DC,
  KBD_$RCV's 0xE1CC10, RIP_$UPDATE_INT's 0xE15888 and 71 more.
- **The rule (Verified, 86 of 86):** the link is passed in A1 exactly when
  some call of the routine comes from a routine other than its enclosing
  one.  All 11 A1 routines have such a caller (4 themselves, 7 a sibling
  or a deeper routine: 0xE10510 from 0xE10628, 0xE5CD02 from 0xE5CD66,
  0xE62CEA from 0xE62D4A and 0xE63096, ...; 3 are called by siblings
  only: 0xE4CB6A, 0xE4EA9C, 0xE62D4A); all 75 `(A6)` routines have exactly
  one caller, their parent.  (The rule could equally be stated "A1 when
  the routine is called from inside a nested routine": at one level of
  nesting these coincide.)  The survey only sees routines that read
  their link.  A nested routine that reaches no enclosing frame looks
  like an ordinary internal routine, and the image passes it nothing:
  no `bsr` anywhere follows a load of A1 (`movea.l A6,A1` or `movea.l
  An,A1`) when the callee does not read A1 (0 sites).  Two such routines
  lie inside a family, so they are nested, and are called by siblings
  without A1.  ROUTE_$SERVICE's 0xE69E40 lies between its nested
  0xE69CCC and 0xE69EC2, has `link #-4` and no frame or link access,
  and is called by 0xE69EC2 at 0xE69F3C (`subq.l #2,SP; clr.w -(SP);
  move.w D2,-(SP); st -(SP); bsr.w`).  DIR's 0xE4EF42 lies inside
  0xE4F3BA's family and is called by 0xE4EF7C and 0xE4F2DA.
  Three more linkless routines lie inside families and are called once,
  by the family's parent (Verified: a scan for `bsr` and absolute
  references finds no other caller): KBD_$RCV's 0xE1CC64 (`link #-8`,
  between the nested 0xE1CC10 and KBD_$RCV 0xE1CCC0; `bsr.w`
  0xE1CE50), AUDIT's 0xE712BA (between 0xE7128A and its parent
  0xE7131C; `bsr.w` 0xE71424), FLOP_BOOT's 0xE323E6 (between 0xE323A8
  and FLOP_$BOOT 0xE3254C; `bsr.w` 0xE32558).  None reads A1 or
  `(A6)`.  Their position makes them nested (Reported: a module-local
  routine called only by the parent would look the same).  Read as
  nested, the AUDIT and FLOP_BOOT families become adjacent (below)
  (lead: ../source pascal-translation-notes finding 34).  So the
  link is passed only to a routine that needs it: one that reaches the
  enclosing frame, or that passes the link on to a sibling (or itself)
  that takes it.  `dpc` computes this as a fixpoint.  **Implemented**
  (`dpc-ir` `set_static_links`: `Nesting`, `StaticLink::{Passed,
  CallerFrame}`, `Call::link` with `LinkArg::{OwnFrame, OwnLink}`;
  `dpc-m68k` `isel.rs` `load_link`, `call`).
- **Where the callee loads the link** (`entry`): after its parameter
  loads at entry (12.2) in 41 routines (0xE082A8 `move.l (8,A6),D7;
  move.w (0xc,A6),D3; movea.l (A6),A4`; 0xE397D0, 0xE62CEA with A1),
  first in the 45 that load no parameter before it.  Of these, 3 load a
  parameter only after it: 0xE1359C `movea.l A1,A2; move.l
  (0xc,A6),D2`, whose first statement reads the link (a scheduler's
  choice? not modelled), 0xE5B188, and 0xE4F3BA (which stores its link,
  below).  4 of the 41 load one more parameter after the link (0xE0B522,
  0xE0BD2C, 0xE18C5E `movea.l (8,A6),A0`, probably a slot loaded where
  needed, 12.2; 0xE4F034, interleaved with its chain walk).  `dpc` loads it after the entry loads, only when the body
  reaches the enclosing frame or passes the link on.
- **Which register** (`entry`): a callee-saved one when the link lives
  across a call, like any value (12.4 R3): A2 in 44 routines with calls,
  A3 in 7, A4 in 4 (0xE1360C: the allocation order of 12.4 is not
  settled there, A2/A3 go to later values); in routines without calls A0
  (8) or A1 (4: 0xE15888 `movea.l (A6),A1` beside a parameter in A0).  A
  few keep it in a D register (0xE01872 D3, 0xE62D4A `move.l A1,D2`; a
  pointer also used as data, 12.2) or in a frame slot (0xE064B0); 7
  routines with calls hold it in A0 (0xE1A4DC, 0xE1CC10: its last use
  comes before the first call).  `dpc`: a virtual address register for the
  allocator.  Where the link's last use is in a call's arguments (an
  uplevel operand, or the link passed on), in a body with branches,
  `dpc` refuses (12.4 R4(c), not settled; 0xE1359C has this shape, its
  link A2 being callee-saved anyway because it lives across the first
  `jsr`).  In a body without branches it is held through the call like a
  parameter (12.4 R4(c)), which fits FILE_$SET_PROT's 0xE5DEF4: `movea.l
  (A6),A2` (0xE5DEFA, saved with `pea (A2)`), last used by `pea
  (-0x44,A2)` among the arguments of the `jsr` at 0xE5DF20.
- **Uplevel accesses.**  The parent's by-value parameters and locals as
  `(d,An)` (0xE135AA `btst #0,(0x15,A2)`, the low byte of a word set; the
  local `(-0x50,A4)` read at 0xE1362E and cleared at 0xE1375E); a
  by-address parameter's slot read through the link, then dereferenced
  (0xE135DA `movea.l (8,A2),A0; move.w (0xc,A0),-(SP)`) or passed on
  (0xE135B8 `move.l (8,A2),-(SP)`); the address of a parent's local
  (0xE13648 `pea (-0x40,A4)`).  **Implemented** (`dpc-ir`
  `Addr::Enclosing`, `enclosing`; `isel.rs` `ea_of`, `push`); a nested
  routine assigning the enclosing routine's parameters is refused, as
  for a routine's own.
- **The parent's side** (bead domain-pascal-thc).  What its nested
  routines reach stays in memory: PMAP_$FLUSH keeps its word
  `(-0x50,A6)` in the frame (0xE13784 `clr.w`, 0xE13938 `addq.w #1`,
  0xE1398C `tst.w`) and re-reads its parameters `(8,A6)` (a slot read by
  0xE1359C; reloaded at 0xE1378A, 0xE1379C, 0xE139AE, 0xE139CA) and
  `(0x16,A6)` (read by 0xE1360C; 0xE1377E, 0xE1385A, 0xE13916, ...) at
  each use, into the lowest free register (A0 each time), while its
  `(0x12,A6)`, which no nested routine reads, is loaded into D5 at entry
  (0xE1377A) and its boolean in D3 (0xE137A8 `sne D3`) is a register
  local.  RIP_$UPDATE_INT 0xE15922 likewise loads `(8)`, `(0x10)` and
  `(0x16)` at entry but tests its byte `(0x14,A6)`, read by its nested
  0xE15888 as `(0x14,A1)`, in the frame (0xE15962, 0xE159C4).
  RINGLOG_$CNTL 0xE72226 steps the `for` index its nested 0xE721CC also
  uses (0xE721F0 `move.w #1,(-2,A2)`, 0xE7220A `addq.w #1,(-2,A2)`) in
  its frame, 0xE72296..0xE722B0 `clr.w (-2,A6)` ... `muls.w (-2,A6),D1`
  ... `addq.w #1,(-2,A6)`, beside the `dbf` count in D0.
  **Implemented:** `Function::uplevel_refs` lists the parent's frame
  objects its nested routines reach (from their IR); `entry_loads` skips
  such parameters and `eval_a` reloads such slots like any memory load;
  `scalar_locals` leaves such locals out, so `register_locals` and
  `loop_pointer_locals` never hold them; a `for` loop whose index a
  nested routine reaches is refused (the memory-index form above is not
  implemented, `check_uplevel_indices`).
- **Frame: a 4-byte static-link slot** (`slots`).  No nested routine has
  `link #0`, and none addresses `(-4..-1,A6)` except three that store a
  link there: 0xE4F3BA `move.l A1,(-4,A6)` (twice, 0xE4F3C2/0xE4F3C6),
  read by its own nested routines as `(-4,A0)`; 0xE4EB40 and 0xE4DFFE,
  level-2 routines that store the outermost frame there.  0xE1359C has
  `link #-4` and no locals.  So a nested frame starts with a slot for the
  link, locals below it.  **Implemented** (`dpc-sema` `routine_body`:
  a nested routine's frame allocation starts at 4).
  The same holds for the unnamed (internal) routines that are not
  nested: of 355, one has `link #0` (0xE6B074, a VFMT segment start) and
  one addresses `(-4..-1,A6)` (0xE70396), against 190 `link #0` and 320
  such routines among the 1194 map-named ones.  **Implemented** for
  `internal` routines too (bead domain-pascal-9mr; section 2, "Frame
  size").
- **Deeper nesting** (observed, not implemented; refused): 6 level-2
  routines and 1 level-3 routine, all in DIR (`summary`, levels).  An
  A1-linked level-1 routine with nested routines stores its link at
  `(-4,A6)` (0xE4F3BA); its children (all `(A6)`-linked) reach the
  outermost frame with `movea.l (A6),A0; movea.l (-4,A0),A2` (0xE4EF8C;
  0xE4F2E2 through D5); the level-3 0xE4F034 walks `movea.l (A6),A0; movea.l
  (A0),A0; movea.l (-4,A0),A2`; a child of an `(A6)`-linked level-1
  routine uses that routine's dynamic link, `move.l (A6),(-0x1a4,A6);
  movea.l (-0x1a4,A6),A0; move.l (A0),(-4,A6)` (0xE4DFFE).  `dpc`
  refuses a routine nested in a nested routine and a reference more than
  one routine out.
- **Conformance: one nested routine alone; no whole family** (beads
  domain-pascal-5aw.3, -5aw.18).  A family (its nested routines, then the
  parent, contiguous) is the natural unit, and every family in the image
  is large: `tools/nestsurvey.py families` lists them by extent (first
  nested routine to the parent's end), smallest 0x112.  The nested code
  reaches the parent's frame, so a reconstruction must reproduce the
  parent's heading and the frame offsets the nested code uses; the
  parent's *body* need not be compared.  The manifest token
  `enclosing=NAME` (`tests/conformance/README.md`, "Nested routines")
  compiles the parent after the nested routines and leaves it out of the
  comparison; it works when the unit's last nested routine ends where the
  parent starts (`families`, column `adj`: 50 of the 56 families
  with a single top-level parent).
  **Byte-identical:** `tests/conformance/ring_discard_packet.pas`, the RING
  nested routine at 0xE76470 (0x2E, `ring/receive_packet.c`), nested in
  the module-local 0xE7649E: `movea.l (A6),A2` (the dynamic link, live
  across the first call), `pea (8,A6)` (its own by-value pointer passed
  by address), `movea.l (0x16,A2),A0; move.w (A0),-(SP)` (the parent's
  by-address parameter), one stand-in local for 4 frame bytes the model
  of section 2 does not explain (`link #-8`, never addressed).
  What blocks the other candidates (reconstructions from `../source`'s C
  compiled with `dpc`, or read with `tools/imgdis.sh`; smallest first):
  - DIR_$GET_ENTRYU 0xE4D500 with 0xE4D460 (family 0x112, the smallest):
    the parent's copy of its 8-byte by-value UID (0xE4D512..0xE4D51A) is
    reproduced since bead domain-pascal-5aw.12 (section 1.2), but the
    parent still differs: it reads its `in` length through the slot once
    into D0 and tests it twice (`movea.l (0x10,A6),A0; move.w (A0),D0;
    tst.w D0; ...; cmpi.w #0xff,D0`, 0xE4D51E..0xE4D528) and its status
    likewise (`move.l (A2),D0; cmpi.l; beq; cmpi.l`, 0xE4D53E..0xE4D548):
    a reuse of a value read through a by-address slot, which 12.4 R7 does
    not model (`dpc` compares in memory twice, and loads the length's
    slot into A2 at entry as a slot used more than once, where the
    original passes the slot on with `move.l (0x10,A6),-(SP)`); the
    nested routine has a byte-copy `for` loop with a
    run-time count, `lea (0,A6,D1.w),A0; move.b (-1,A1,D1.w),(-0x129,A0)`
    (0xE4D480..0xE4D488), a byte store at an odd frame offset (0xE4D494
    `move.b #0x44,(-0x1b5,A6)`, a record field), 8-byte copies into frame
    displacements `lea (-8,A3),A0; move.l (A0)+,(d,A6)` (0xE4D49A), a
    status stored from memory and its flags tested (0xE4D4CE `move.l
    (-0x24,A6),(A2); bne.s`).
  - The PROC2 module-local 0xE420B8 with 0xE42028 (0x146): three zero
    words pushed as `clr.w; clr.l` (0xE42088, the run of 1.5), another
    unit's table indexed through `movea.l #sym,A0` with `ext.l; lsl.l #3`
    and `muls.w #228` (0xE4203A..0xE42044, 0xE42060), `while` loops over
    it, `subq.w #1,(d,A0)` with its flags tested (0xE42048).
  - RINGLOG_$CNTL with 0xE721CC (0x168): 0xE721CC's `for` index is the
    parent's local (`move.w #1,(-2,A2)` ... `addq.w #1,(-2,A2)`, the
    memory-index form, refused in `check_uplevel_indices`) and it holds
    another unit's data base from `movea.l #sym,A0`; the parent adds a
    `case` table and a 4604-byte copy loop.
  - FILE_$SET_PROT with 0xE5DEF4 (0x176; 0xE5DEF4 alone is 0x46): a
    packed record field cleared with `bclr #6,(-0xb,A6)` (0xE5DF08;
    section 5.3), 8-byte copies into frame
    displacements; the parent a `case` table whose arms share code.
  - RIP_$UPDATE_INT with 0xE15888 (0x1A4): two-bit fields extracted with
    `move.w #0xc0,D0; and.b (d,An),D0; lsr.w #6,D0` and stored with
    `andi.b; ori.b` (0xE158B6, 0xE158CE), a 10-byte copy `(A2)+,(A3)+`
    in longwords and a word (0xE158EC).
  - Nested routines alone, adjacent to their parent: PROC2_$SUSPEND's
    0xE4120C (0x5E) reads an element of another unit's array, `movea.l
    #sym,A0; mulu.w #0xe4,D0; move.w (-0x4a,A0,D0.w),-(SP)` (0xE41222..
    0xE4122C; 5.2, "Another unit's array") and sets bit 31 of a status with `bset #7,(-8,A2)` after testing
    its low word, `tst.w (-6,A2)` (the layout of `status_$t` is Uncertain,
    9); AST_$FREE_PAGES's 0xE03FBC (0x50) updates a byte field
    `move.b (0x10,A0),D1; sub.b D0,D1; movea.l (8,A2),A0; move.b
    D1,(0x10,A0)` (0xE03FF2..0xE03FFC, the pointer reloaded; the value
    set up first, 12.4 "Stores whose destination and value both need a
    register");
    PCHIST_$CNTL's 0xE5CD66 (0x50, the sibling call `movea.l A2,A1; bsr.w
    0xE5CD02` at 0xE5CDA2) needs 0xE5CD02 before it, whose `for` index is
    the parent's local `(-0x18,A2)` with an unsigned count guarded by
    `tst.w; beq` (7.4, not supported either); IO_$ACQUIRE's 0xE1A4DC
    (0x4C) is a `case` whose four labels share one arm, a comparison as
    the result; the ACL_ 0xE45154 (0x48) compares 8-byte records with
    `cmpm.l (A3)+,(A2)+`.  The AUDIT 0xE7128A (0x30) and FLOP_BOOT
    0xE323A8 are not adjacent to their parents (`families`): the
    linkless 0xE712BA and 0xE323E6 lie between (above).
  PMAP_$FLUSH (0xE1359C..0xE13A14) needs, in 0xE1359C alone: a run of
  three word constants
  (0xE135D0 `clr.l -(SP); clr.w -(SP)` for three zero words: the
  rightmost pair merged, one site; 1.5 rejects such runs); `pea (d,An)`
  of an object through a pointer loaded from memory (0xE135E2..0xE135EA
  `movea.l (8,A2),A1; movea.l (4,A1),A3; pea (0x10,A3)`); an inner `if`
  whose false branch goes to the epilogue (0xE135CC `bpl.s 0xE13602`,
  7.1); the link's last use in a call's arguments in a body with
  branches (above); the link copy before the parameter load (0xE135A4);
  no register reuse within an argument list (0xE135DA..0xE135E6 A0, A1,
  A3; 12.4 R4(a)).  0xE1360C and PMAP_$FLUSH add loop-invariant
  constants held in registers (0xE13696..0xE136A2), a strength-reduced
  pointer kept in a frame temporary (0xE137E4..0xE137F0 `move.l
  A1,(-0x78,A6); movea.l (-0x78,A6),A1`), a function body of many
  statements (12.4 R5), and more.  The rest of the nesting rules are
  covered by goldens (`tests/golden/nested/`; nested functions called by
  a sibling, in expressions, held across a call, returning a pointer,
  reading the parent's parameters and locals, recursive: `fn_*`).
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
  (PMAP_$FLUSH; `summary`: 83 of the 83 nested routines whose parent is
  seen lie before it), in source order: the 19 calls between nested
  routines of one parent (other than parent to child) all go to an
  earlier routine, as declaration order requires (0xE1360C calls
  0xE1359C).  Children of a level-1 routine come before it too (DIR
  0xE4F034 before its parent 0xE4F100, both before 0xE4F3BA).
  **Implemented** (`dpc-ir` `lower_family`); a nested routine declared
  `forward` is refused (heading or body order not seen).
  The same holds for module-local workers placed before their first
  user: 0xE17778 comes before LOG_$READ, and 0xE00EB0 before
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
  order in `Routine::nested`.

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
    - Another register's self-copy: LOG_$ADD 0xE176F0 `movea.l A1,A1`
      (`2249`), followed by 0xE176F2, at 2 mod 4 and no loop head (that
      routine's loop-head fillers are 0xE176BA and 0xE1772E).  It is
      the only `movea.l An,An` other than A0's in 0xE00800..0xE88834
      (word scan).  A1 holds `addr(buf^.data[buf^.tail])` just stored to
      `(8,A5)`; the copy is read as an assignment to a register-held
      local the allocator also put in A1 (Reported reading; lead:
      ../source pascal-translation-notes finding 19).  So a copy into
      the same register is kept, not dropped; `dpc` renames and emits
      no move (bead domain-pascal-reso).
  - **What the filler precedes:**
    - 315 loop heads.  Examples: 0xE00ED6, 0xE0149E, 0xE128DA, and
      0xE1CE42 before KBD_$RCV's loop at 0xE1CE44.
    - The end of a segment's code, 61 times: 0xE1A9D2 before OS_TERM
      0xE1A9D4, 0xE1AD16 before BITPAD 0xE1AD18.
    - 34 4-aligned constant cells after a routine's code
      (MMAP_$AVAIL 0xE0CCB6, MMAP_$WIRE 0xE0CD16, PKT 0xE12D32): the
      filler before a routine's longword cells in aligning code
      (section 1.5, "Alignment"; `tools/pcsurvey.py align` counts 45
      such fillers, some of them after the routine's word cells).
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
      `pea (d,PC)` (section 1.5, the cells after the last routine): NET_IO 0xE0E8C2 (from
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
    `--loop-align-off-from=ROUTINE,...` (on before ROUTINE, off from it
    on, within ROUTINE's code section; at most one routine per code
    section, since each map segment has its own switch: bead
    domain-pascal-lco, section 11).  The order is the unit's routine
    order within a code section, which is the order of the routines'
    bodies: a routine declared `forward` (or `extern`) earlier is placed
    at its body (section 6, "Order", Verified).  A routine that does not align gets no
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
  source went to the register cannot be seen in the image.  Survey
  (`tools/cmpsurvey.py summary`, kinds `mem2`/`both`; bead
  domain-pascal-aaj): 358 `move.<s> M1,Dn; cmp.<s> M2,Dn` with both
  operands in memory (`.w` 217, `.l` 132, `.b` 9; e.g. TTY_$K_SET_FLAG
  0xE67462 `move.w (0x2cc,A2),D0; cmp.w (0x2ca,A2),D0; beq.b`,
  ACL_$CLEAR_SUPER 0xE4701A `move.w PROC1_$CURRENT,D0; cmp.w
  (0xb72,A5),D0; bne.s`; char compares `move.b; cmp.b; bls`, 0xE01E22),
  against 4 with both loaded (`move; move; cmp Dm,Dn`, 0xE54D80).
  **Implemented** (bead domain-pascal-aaj; `isel.rs`, `compare` and
  `compare_computed`): the left operand loaded into a new data register
  (Uncertain: the image cannot say which source operand that was), the
  right one the `cmp` operand.  Refused: a variable in memory against one
  held in a register (load the left one, or `cmp` against the held one
  with the relation reversed?); two pointer variables (20 `movea.l M1,An;
  cmpa.l M2,An` in compiled code, e.g. 0xE01A7E, against `move.l; cmp.l`
  pairs that may be integers or pointers: not told apart).  Conformance
  unit `acl_clear_super.pas` (byte-identical).
- **Mixed widths and signs (Verified at the sites; survey; implemented,
  bead domain-pascal-aaj).**  Ref 4.3.1 p. 4-5 ("computes operands smaller
  than 32 bits with 32 bits of precision when necessary to achieve correct
  arithmetic") and p. 9-32, message 250 (a negative constant against an
  unsigned subrange is folded away, as it can never match): comparisons
  have their mathematical meaning.  A signed word against an unsigned one
  (`integer` against `0..65535`) is done in 32 bits, both widened, `ext.l`
  and `clr.l; move.w`, then `cmp.l Dr,Dl` and a signed branch: TTY_$K_PUT
  0xE67C42..0xE67C4A `move.w D2,D0; clr.l D1; ext.l D0; move.w (A4),D1;
  cmp.l D1,D0; bge.b` (avail_space < count); tty_$i_put_chars
  0xE1B050..0xE1B058 `clr.l D2; move.w D1,D4; move.w D3,D2; ext.l D4;
  cmp.l D4,D2; ble.b` (count > free_space, the unsigned one on the
  left); tty_$i_err 0xE1BD7C..0xE1BD8A; the PROC2 module-local 0xE42224
  at 0xE4224C (`=`, `bne`).  The survey (kind `widen`) finds 26 `cmp.l`
  of two widened words (sign/zero in either order); every ordered one
  branches signed (`blt`/`ble`/`bge`/`bgt`).  In each TTY site the
  register compared into is the left operand's.  A word against a
  longword: the word widened by its own sign, `cmp.l <ea>,Dn` (kind
  `widen1`, 38: `ext.l` with `ble`/`bge`/`blt`, 0xE179BC; `clr.l; move.w`
  with an unsigned branch, 0xE0D7CE).  **Implemented** (`dpc-ir`
  `widen_operands`; `isel.rs`, `compare_computed`): both words widened to
  signed longwords, the left one first; dpc's scheduler (12.4 R11)
  interleaves the two widenings exactly as at 0xE67C42 and 0xE1B050
  (not as at 0xE1BD7C, where the right operand's pointer load comes
  after the left's `ext.l`).  A word against a longword when the word
  is on the left (the longword on the left would be `cmp.l Dw,Dl`, not
  catalogued: refused).  A signed longword or word against an unsigned
  longword (`integer32` or `integer` against `0..2147483647`, bead
  domain-pascal-5j9): signed, as ref 4.3.3 (pp. 4-6..4-7) says of mixed
  operands, the word widened with `ext.l` and the unsigned longword
  compared as it is (its values fit); **Uncertain:** no image site is
  identified, the instructions are those of the forms above (golden
  `tests/golden/expr/ulong`).  Other mixes (bytes against wider values)
  are refused; Pascal's typing leaves few of them (`0..255` is a word,
  ref 3.8).  Golden `tests/golden/control/compares_mixed`.
- **Records (Verified at the sites; survey; implemented, bead
  domain-pascal-aaj).**  Ref Table 4-1 p. 4-2: `=` and `<>` apply to
  "All types".  The image compares records in memory with `cmpm`
  (`tools/cmpsurvey.py`, kind `cmpm`, 393 runs): the two addresses in
  address registers (another unit's object with `movea.l #abs,An`, 201
  of the address loads before a `cmpm`, e.g. `movea.l #UID_$NIL,A1`), a
  count `moveq #n-1,Dn`, then:
  - 8 bytes (UIDs): two `cmpm.l (Ay)+,(Ax)+` with a `bne` after the
    first, the count loaded but unused (all 383 runs of two have the
    `moveq #1`): TTY_$K_SIMULATE_TERMINAL_INPUT 0xE1C1AC..0xE1C1BC `lea
    (-8,A6),A0; lea (0x4c,A2),A1; moveq #1,D0; cmpm.l (A1)+,(A0)+;
    bne.b; cmpm.l (A1)+,(A0)+; beq.b`; the ACL_ routine 0xE45154;
  - 6 bytes (48-bit clock values): a `cmpm.w` loop, `moveq #2; cmpm.w;
    dbne; beq` (TIME_$Q_SCAN_QUEUE 0xE16EC2; 0xE4CEAA);
  - 32 and 36 bytes: a `cmpm.l` loop with `dbne` (DISK_IO 0xE3D84E,
    `moveq #7`; ACL_$SET_RE_ALL_SIDS 0xE48462, `moveq #8`, 6 sites);
  - a 6-character array: a `cmpm.b` loop (0xE6C278, `moveq #5`), so the
    unit is not the size's alone;
  - as a value: the last `cmpm` (or the `bne` of the first, to it)
    followed by `sne`/`seq` (PACCT_$ON 0xE5A9C0 `sne D0`, the count's
    register; 0xE57786 `seq D0`, 0xE56C20 `seq D1`, 0xE60F76 `seq D4`;
    0xE31652 and 0xE4E564 into D2, a register variable).
  Which operand is the destination of `cmpm` varies (0xE412BE the first
  address loaded, 0xE48096 the second): Uncertain, and immaterial to the
  result.  **Implemented** (`dpc-ir` `record_compare`, `Cond::Block`;
  `isel.rs` `block_compare`, `block_flags`, `compared_block`,
  `test_value`; `Insn::Cmpm`, `Insn::Dbf { until }`): records of 8 bytes
  (unrolled), 6 bytes (`cmpm.w` loop), multiples of 4 from 32 (`cmpm.l`
  loop, an ordinary loop head, section 7), the left operand's address
  first and the `cmpm` destination, then the right one's, then the count;
  as a value `s<cc>` into the count's register.  The addresses: the
  unit's data or the frame (`lea`), the object a pointer slot points to
  (`movea.l (d,A6),An`), a field through a pointer held in an address
  register (`lea (d,An),Am`, 0xE1C1B0), another unit's object (`movea.l
  #sym,An`).  Refused: other sizes (12..28 bytes not seen), arrays and
  strings (the `cmpm.b` site), records with bytes no field covers
  (padding: the image compares every byte, and the reference gives no
  rule for padding; packed records count their fields' bits), ordering
  records.  Conformance unit `pacct_on.pas` (byte-identical); golden
  `control/compares_mixed`.
- **Booleans compared (`b = true`, `b1 = b2`; Uncertain, refused):** the
  survey (kind `cmpb`) finds 2 `cmpi.b #-1` (0xE1B96C, 0xE53FDE, not
  shown to be booleans), 134 other `cmpi.b` and 16 `cmp.b`, which chars
  explain as well.  Still not catalogued.
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
  (0xE0B640 `move.l (A1),D0; tst.l D0; bmi`; 0xE0BEAE).  `tools/cmpsurvey.py
  opflags` counts a same-size move into Dn then the branch 398 times
  (`beq`/`bne` 295, `bmi`/`bpl` 78, signed 25; 3 at joins) against 14
  with `tst` between (10 more at joins): still no rule.  A load of
  **another size** than the test is tested: `move.b M,Dn; tst.w Dn` 83
  times, after a `clr.w Dn` (SOCK_$CLOSE
  0xE15FB8..0xE15FC0 `clr.w D0; move.b (21,A2),D0; tst.w D0; beq.s`, a
  `[byte] 0..255` field against 0).  **Implemented** for that case (bead
  domain-pascal-qlzu, `isel.rs` `Selector::tst`); a same-size load is
  still refused ("testing a register the previous instruction loaded"),
  and so is a load at any other pair of sizes: no `move.w` then `tst.l`
  of the register (a word widened to a longword, `clr.l; move.w`) is in
  compiled code (one `move.l; tst.w`, at a join, 0xE0EB82; one `move.w;
  tst.b`, 0xE19106).
- **A test right after a store (Verified by survey for the frame;
  implemented there; bead domain-pascal-dgb):** `tools/resultsurvey.py
  storetest` takes every store of a register into memory (`move.<s>
  Rn,<mem>`, not a push) whose next instruction tests the stored value.
  Of a data register into a local's frame slot: the conditional branch
  itself, on the store's flags, 20 times (0xE19BE6 `move.l D0,(-4,A6);
  bne.s`, a call's result; 0xE3BCB8 `move.w D4,(-92,A6); bne.s`;
  0xE6B180 `seq D1; move.b D1,(-60,A6); bmi.s`, a boolean; 0xE4F09C
  `move.w D1,(-66,A6); bmi.s`); `tst` of the slot once (0xE0B61C, after
  `not.b`; not explained); `tst` of the register twice, at a join the
  other path reaches without the store (0xE554A8, 0xE55514).  Of a data
  register into other memory: on the store's flags 21 times (0xE03DDC
  `move.l D0,(4,A2); bne.s`; 0xE0AF58 `move.b D1,(A5); bpl.s`), `tst` of
  the memory 10 times (0xE02806 `move.w D0,(0x412,A0); tst.w
  (0x412,A0)`; 0xE19C7A `move.l D0,(88,A5); tst.l (88,A5)`), of the
  register 3 times: no rule seen.  Of an address register: section 3,
  "Pointer locals".  A `move` sets N and Z from the value moved, at its
  size, and clears V and C, as `tst` of that value does, so the branch
  tests the same relation whatever it is.  The survey's branches are
  all `beq`/`bne`/`bmi`/`bpl`; no frame store is followed by a signed
  or unsigned ordered branch (`bgt` .. `bcs`) or by a `tst` and one (an
  ad-hoc count over the same routines, bead domain-pascal-dgb review),
  so whether an ordered test right after a store branches on the
  store's flags or tests again is not seen either way (Uncertain).
  An ordered compare against a nonzero constant right after a store to
  other memory compares the register: module-local 0xE1CA8C (../source
  kbd_$set_type) at 0xE1CABC `move.w
  D0,(0x44,A0); cmpi.w #2,D0; bge.s`, D0 compared unsigned with 2 just
  before (0xE1CAA0 `moveq #2,D1; cmp.w D1,D0; bls.s; move.w D1,D0`);
  the signed branch is read as a test of the integer field just stored,
  its value reused from D0 (Reported; the reading is the translator's,
  the image shows only the signed compare of the register; lead:
  ../source pascal-translation-notes finding 32).
  **Implemented** (`isel.rs`, `Selector::tst`): a test of a local's slot
  that would directly follow a store of a data register into that slot,
  at the same size, is left out when the branch is `beq`/`bne`/`bmi`/
  `bpl` (the `if`'s branch reads the store's flags; `sched.rs` keeps the
  store the last flags setter of its run, R10); for any other branch
  (`x > 0`, `x <= 0`: `bgt`/`ble`) it is refused, "an ordered test of a
  local right after its store ... not supported yet" (bead
  domain-pascal-1vp; dpc used to emit `move.w D0,(-2,A6); ble.s`, the
  rule extended without evidence).  Driver test
  `ordered_branch_on_a_store_is_refused`.  A label between them
  (a join) keeps the `tst` of the slot (the image tests the register
  there, not reproduced); other memory is tested again, as before.
  Golden `tests/golden/control/store_flags`; driver test
  `branch_on_the_store_flags_only_right_after_the_store`.
- **A computed value against 0 (Verified by survey; implemented; bead
  domain-pascal-qlzu):** `tools/cmpsurvey.py opflags` takes every
  conditional branch and `s<cc>` in compiled code (SMD_UTILS
  0xE702F4..0xE707FC left out: it saves registers before its `link` and
  takes A5 from A0, hand assembly by section 0's signs) whose previous
  instruction is an operation on a data register (add, sub, and, or,
  eor, a shift, neg, not, clr, ext, swap, mul, div, move, moveq), or a
  `tst`, a `cmpi #0` or a store of that register between them: 1156
  sites, 993 on the operation's own flags, 144 after a `tst`, 14 after a
  store of the register, 5 after a store and a `tst`, **none after
  `cmpi #0`**.  Left out the loads (move, moveq, clr; "Flags of a load"
  above), by operation and relation:

  | operation | `beq`/`bne` | `bmi`/`bpl` | `bgt`/`ble`/`blt`/`bge` |
  |---|---|---|---|
  | add, sub | flags 19 (2 at joins); `tst` 4, all at joins | flags 315, none with `tst` | `tst` 8 (1 at a join), flags 0 |
  | and, or | flags 115; `tst` 2, both at joins; `tst` 6 at another size | flags 70 (`and.b`/`or.b` of booleans) | none |
  | lsr | flags 36; `tst` 2 at another size | none | none |
  | asr, lsl, not, muls | `tst` 1 at another size (asr.l) | flags 2, 1, 1, 2 | none |
  | divs, divu | `tst` 2 (1 at a join) | `tst` 1 | none |
  | swap | `tst.w` 3 (another size) | none | none |
  | eor, neg, ext | none | none | none |

  (Unsigned `bcs`/`bcc`/`bhi`/`bls` after sub 20 and after `lsr.b` 9
  read the carry: compiler expansions, not a relation with 0.)  The rule:
  the original trusts the **N and Z** flags of an add, sub, and, or,
  shift, `not` and multiply, **not their V flag**, and no flags of a
  divide (N and Z are undefined after an overflowing `divs`/`divu`):

  - `=`, `<>`, `<`, `>=` (`beq`/`bne`/`bmi`/`bpl`) read the operation's
    flags: SOCK_$ALLOCATE 0xE15EFA `move.w D3,D0; or.w D4,D0; beq.s`
    (../source `(hdr_pages ! data_pages) <> 0`), SOCK_$CLOSE 0xE16004,
    SIO6509_$INQ_LINE 0xE7267C `move.l D0,D1; andi.l #0x180,D1; beq.s`
    (`(mask & 16#180) <> 0`).  The same after an assignment to a
    register-held local, tested by the next statement: MAC_OS_$SEND's nested
    routine (module-local at 0xE0B522) at 0xE0B57C `sub.w D5,D3; bne.b`
    (`remaining := remaining - chunk; if
    remaining = 0`), PROC1_$ALLOC_STACK 0xE150EA `subi.w #0x400,D2;
    bne`, tty_$i_put_chars 0xE1B046 `subq.w #1,D1; bpl.s` (`free_space
    := head - tail - 1; if free_space < 0`), TTY_$K_PUT 0xE67C1A,
    TTY_$K_GET 0xE1C516 `sub.w (716,A2),D0; bpl.s`, netbuf_rtnva_locked
    0xE0E8E0 `asr.l #2,D2; bmi.s` (`slot := (va - base) div 1024; if
    slot < 0`), NETBUF_$RTN_HDR 0xE0EED6.  Of the ~300 other `sub;
    bmi/bpl` most are the compiler's own: a run-time `for` count guard
    (`subq.w #1; bmi`), `abs` (0xE1AE02 `sub.l D0,D3; bpl.s; neg.l D3`),
    the rounding of a signed `div` by 2^k (0xE06D5A `sub.l D3,D2; bpl.s;
    addi.l #1023,D2; asr.l #8,D2`).
  - `>`, `<=` (`bgt`/`ble`, which read V) after an add or sub: `tst`
    first, 7 of 7: in a condition, SMD_$SIGNAL 0xE6F258..0xE6F260 `sub.l
    D1,D0; moveq #39,D1; sub.l D0,D1; tst.l D1; bgt.s` and 0xE6F26E
    `sub.l D1,D0; subq.l #1,D0; tst.l D0; bgt.s`; after an assignment,
    NETWORK_$WRITE 0xE0FBF6, NETWORK_$READ_AHEAD 0xE1018A,
    PKT_$COPY_TO_PA 0xE125E0, SHAKE 0xE3E4CC, LOG_$ADD 0xE1766A `addq.w
    #4,D2; tst.w D2; ble.w`.
  - At another size than the operation (an `and.b` tested as a word,
    0xE0A544 `moveq #7,D1; and.b (-64,A6),D1; tst.w D1; bne.s`; `and.w`
    as a longword, 0xE3EE6E; `lsr.l`, `asr.l`, `swap` as a word,
    0xE1701C, 0xE130AA, NETWORK_$PAGE_SERVER 0xE11822), after a divide
    (0xE11BF4 `divs.w #60,D1; tst.w D1; beq.s`; 0xE6966E), after a call
    (`tools/cmpsurvey.py callcmp`: 31 branches after `tst` of a call's
    D0, SOCK_$PUT_INT 0xE161EA `tst.w D0; seq D0` as a value; 7 after an
    M$ARITH call, 0xE14C02 `tst.l D0`): `tst`.
  - At a join (a label between the operation and the branch) the `tst`
    is there (0xE08F80, 0xE3C930, 0xE4445E, 0xE3DF66, 0xE3EF72).
  - **One bit (survey Verified; the reading of the memory forms
    Uncertain; implemented, bead domain-pascal-3xwx):** no `and` with a
    one-bit mask, immediate or loaded into a register, is followed by a
    branch or an `s<cc>` (92 with other masks); `(value & 4) <> 0` of a
    register-held parameter is `btst #2,D0; sne D2` (pacct_$compress
    0xE5A9DE).  The mask on the left (`4 & x`) is taken the same way:
    the source order cannot be seen, and the `and` it would otherwise
    give is not seen.  `tools/setsurvey.py onebit` takes every one-bit
    test of compiled code, 926:

    | form | operand | branch | `s<cc>` | other |
    |---|---|---|---|---|
    | `btst #k,<byte>` | memory, odd address | 285 | 65 | 6 (merged tails) |
    | `btst #k,<byte>` | memory, even address | 64 | 2 | |
    | `btst #k,Dn` | a register-held value (not written in the block) | 159 | 21 (all into another register) | |
    | `btst #k,Dn` | a word/longword just loaded, bit in the part a set test would load (`fits`) | 182 | 34 (16 same register, 18 another) | 1 |
    | `btst #k,Dn` | a word/longword just loaded, more than that part (`whole`) | 65 | 12 (2 same, 10 another) | 3 |
    | `btst #k,Dn` | a byte just loaded | 3 | 1 | |
    | `btst #k,Dn` | computed (`lsr.w`, `andi`), a copy | 13 | | |
    | `lsr.b #1,Dn; bcc`/`bcs`/`scs` | a word/longword just loaded | 7 | 2 (another register) | |
    | `tst.b Dn; bpl` | a word just loaded (bit 7) | 1 | | |
    | `and` with a one-bit mask | any | 0 | 0 | |

    and no `btst #7`, `#15` or `#31` on any operand.  The rule taken
    (the 5.1 reading: a set in memory is narrowed to the part holding
    the bit, an integer is read whole):
    - `(x & 2^k) <> 0`/`= 0` of a register-held `x`: `btst #k,Dx`;
    - of a word or longword variable in memory: `move.<s> M,Dn; btst
      #k,Dn` at its own size, whatever k (tty_$i_put_chars 0xE1B01E
      `move.w (8,A0),D1; btst #5,D1; beq.s`, 0xE1B0E6 `move.l
      (12,A0),D1; btst #3,D1`; TTY_$I_ERR 0xE1BE3C; the module-local
      0xE1BCFC at 0xE1BD0A `move.l (20,A0),D0; btst #12,D0` and
      0xE1BDCA `move.w (8,A0),D0; btst #4,D0`; AST 0xE013BA; FILE_
      0xE5F7B8 on an indexed word); the 80 `whole` sites are the ones
      that show it, the `fits` ones agree;
    - of a computed value: `btst #k` on its register (DIR_$SET_PROTECTION
      0xE522D0..0xE522DA `and.w (4,A4),D1; lsr.w #4,D1; btst #4,D1;
      bne`; DIR_$SET_DEFAULT_ACL 0xE530D6; FILE_$SET_PROT 0xE5DF5C);
    - `odd(x)`: section 8 (`lsr.b #1` for a word or longword in memory,
      `btst #0` in a register).

    A loaded copy is tested again for another bit later in 11 places
    (TTY_$I_ERR 0xE1BE54 `move.l (20,A0),D2; btst #10,D2; ...` then
    0xE1BE62 `btst #11,D2` in the next `else if`; AREA_ 0xE07C78 `move.w
    (A4),D0; btst #4,D0; bne` then `btst #0,D0`; TERM_$INQUIRE 0xE66EF0,
    0xE66EF6): not reproduced, dpc loads again for each test (bead
    domain-pascal-gwpz; for sets, domain-pascal-ozaq).
  - **Multiply:** the two branches after a `muls.w` (0xE69626, 0xE69630)
    are `abs` of a word product, `muls.w; bpl.s; neg.w`: the 32-bit
    flags read for a value used as a word, no `tst.w`.  A product tested
    as a longword reads the flags; tested as a word it is refused (a
    `tst.w` first is not seen, and those sites suggest it is not there).

  N and Z after an add or sub are those of the 16- or 32-bit result, as
  `tst` would give them, so `bmi`/`bpl`/`beq`/`bne` on the operation's
  flags test the value Pascal keeps (no overflow detection); `bgt`/`ble`
  there would read V and test the exact sum or difference
  (16#7FFFFFFF + 1 > 0), which the `tst` avoids.  No semantic divergence
  follows from the implemented forms (exec test `computed_zero`, with
  32-bit overflows both ways).  How wide a word sum is in a comparison is
  a separate open question (bead domain-pascal-9js, exec test
  `wide_compare`); these forms follow dpc's present word width.

  **Implemented** (`isel.rs` `Selector::compare_computed_zero`,
  `flags_test`, `tst`): all of the above, for a value computed for the
  test and for a register-held local an assignment just formed (it used
  to be tested again, `sub.w D2,D3; tst.w D3; bne`, against MAC_OS_$SEND's nested
  routine 0xE0B522 at 0xE0B57C).  Refused, not seen either way: `>`, `<=` after an and, or,
  shift, `not` or multiply (V is clear after them; whether the original
  re-tests is not seen); a word product tested as a word (above); any
  test right after an eor, a `neg`, an `ext`
  or a `swap` tested as a longword (no branch or `tst` follows one in
  compiled code) (bead domain-pascal-bkoz); unsigned orderings against
  0, as for variables.  Of the one-bit tests (`isel.rs`
  `Selector::one_bit_test`, `odd_test`; bead domain-pascal-3xwx), refused:
  bit 7, 15 or 31 alone (no `btst #7`/`#15`/`#31`; a lead for bit 7 of
  a loaded word, 0xE19106..0xE1910C `move.w (6,A0),D3; tst.b D3; bpl`;
  bead domain-pascal-fhd4; dpc used to emit `btst #31` on a register);
  a byte variable, and a `0..65535` variable with an integer mask, both
  widened for the `&` by dpc-ir (`btst` on a byte in memory, or `clr.w;
  move.b` first?  `clr.l; move.w; btst` is not seen; bead
  domain-pascal-kqa2, and domain-pascal-9js for the width); a variable
  in memory as a boolean value, `&` or `odd` (the `s<cc>` register:
  the loaded one at 0xE04C3C and 0xE5EC4A, another at 0xE02CD8,
  0xE052FE, 0xE70E98; bead domain-pascal-uwfe); a call's result (not
  seen).  Golden `control/computed_zero`, `control/one_bit`; exec tests
  `computed_zero`, `onebit`; driver tests `computed_value_against_zero_*`,
  `one_bit_mask_is_btst`, `one_bit_of_memory_is_a_whole_load`,
  `odd_of_memory_is_lsr`, `one_bit_test_refusals`,
  `assigned_register_against_zero_on_the_operation_flags`.
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
- **Branch threading (Verified by survey; implemented; bead
  domain-pascal-o6p).**  Survey: `tools/threadsurvey.py summary`
  (every compiled routine, followed as `tools/cmpsurvey.py` does).
  - **No branch to a `bra`:** none of the `b<cc>`, `bra` or `dbf`
    branches, nor any `case` jump-table entry, targets a `bra` (`tobra`:
    0; 0xE34BC8, the one hit of an earlier linear sweep, is data).
  - **The branch goes to the `bra`'s target instead** wherever the
    plain layout would branch to a `bra`: an `if` part that ends in an
    inner `if` followed by the outer `bra` to its join or to the
    epilogue, an `exit` before such a `bra`, a `goto` label before one.
    `thread` counts 845 forward branches that go past a `bra T` to T
    itself, where the `bra` ends an outer `then` part (the instruction
    after it is the target of a branch from before the threaded one):
    443 `bra`s and 402 `b<cc>`s; T the epilogue 216 times.  TTY_$K_SET_FLAG
    0xE6745A..0xE67482 (`if value then begin ...; if a <> b then
    tty_$i_signal(...) end else ...`): the inner test's `beq.b 0xE67482`
    goes past the `then` part's `bra.b 0xE67482` (0xE6747A) to the
    epilogue; the TTY module-local 0xE1B6AC (tty.pas `tty_$i_kill_line`)
    at 0xE1B700 (`beq.s 0xE1B70E` past `bra.s 0xE1B70E` at 0xE1B706);
    the TTY module-local 0xE1B716 (`tty_$i_word_erase`) at 0xE1B754, an
    `exit` from the first of two `while`s, `beq.s 0xE1B79C` to the
    second loop's test, past its opening `bra.s 0xE1B79C` at 0xE1B764 (which stays: the
    first loop's test falls into it).
  - **A `bra` that nothing reaches any more is dropped:** no `bra` in
    compiled code follows an unconditional transfer unreached (`dead`:
    0).  So a `while` whose opening `bra` follows a `bra` (an `else` part
    that starts with the loop) is entered only by the threaded branch to
    its test: TTY_$K_GET 0xE1C44A `bpl.w 0xE1C4FC` (`if break_mode_flag
    then ... else while ...`, k_get.pas 0xE1C448), the `then` part's
    `bra.s 0xE1C50E` at 0xE1C4D4 then the head 0xE1C4D6; TTY_$K_PUT
    0xE67C0E (`if 1 in options then begin ...; return end`, then a `while`) `beq.w 0xE67D82`, straight to the
    loop's bottom test, the `then` part's `bra.w 0xE67D92` (the
    epilogue) at 0xE67C5A and the head 0xE67C5E right after it (lead:
    ../source pascal-translation-notes finding 11).  `loop` finds 13
    loops closed by a backward `b<cc>` whose head follows an
    unconditional transfer and that a branch from outside enters at
    the test (some of them `goto` targets, not `while`s).
  - **Alignment:** the head of such a `while` is still aligned in
    aligning code: the module-local 0xE4AFA8 (aligning) `bne.s 0xE4B0FC`
    to the test, the `then` part's `bra.s 0xE4B106` at 0xE4B0F4, the
    0x2048 filler at 0xE4B0F6 (unreached), the head 0xE4B0F8.
    TTY_$K_GET and TTY_$K_PUT are in non-aligning code (heads at 2 mod 4).  One exception (`tools/loopalign.py`
    `head_rows`, entry `bra-out` in an aligning routine): 0xE7469E in the
    module-local 0xE743CE, at 2 mod 4 with no filler, entered at
    0xE746A2 past its first instruction, not at a `while`'s test (a
    different shape; aligning routines have other heads at 2 mod 4,
    section 7).
  - **A loop head that would fall into a `bra`** (a `repeat`, or a
    backward `goto`'s label, right before a `while`: its back edge
    threaded to the `while`'s test): no 0x2048 filler precedes a `bra`
    reached by falling through in compiled code (`tools/loopalign.py
    fillers`: only 0xE649E6, before a label reached through a jump table,
    section 7, and 0xE313B2 in data), so whether the
    original aligns such a head is not seen.  Refused (bead
    domain-pascal-tfae).  A `while` whose body is a `while` has no such
    head: the outer opening `bra` passes it, the inner one is dropped,
    and one alignment stays before the inner head.
  - **Sizes** are those of the threaded layout (section 7, "Branch
    sizes": every `.w` out of reach): 263 of the 845 threaded branches
    are `.w` though a `.s` would have reached the `bra` they pass.
  - **Not seen:** a `case` table entry threaded (no entry goes to the
    epilogue unless the table's default does; an arm of only `return`
    or `goto` in a table), a `dbf` to a label before a `bra` (both
    refused, bead domain-pascal-r4yn).
  - **Tail merging is separate:** PARITY_$CHK_IO 0xE0B1A4 (`if ... then
    if ... then r := 1 else if ... then r := 2 else r := 0 else r :=
    0`) has the inner `else` part's `bne.s 0xE0B1AA` go to the outer
    `else` part's identical `clr.w D0` rather than a copy of it before
    the threaded `bra` ("Identical ends" below, bead
    domain-pascal-sggk); dpc keeps both copies (srcsurvey MISMATCH).
  - **Implemented** (`crates/dpc-m68k/src/isel.rs` `thread_branches`,
    after selection, before register allocation): every `Insn::Branch`
    to a label that a `bra` follows (other labels between) takes that
    `bra`'s target, transitively; a `bra` after an unconditional `bra`
    with no label still named between is dropped (replaced by an unused
    label, so the side tables' indices hold); a loop head's `Align4`
    stays (a loop head's alignment between two `bra`s does not keep the
    second; two alignments left together become one).  An `exit`,
    `next` or `goto` whose `bra` is dropped (in an inner `else` part,
    reached by the inner test's threaded branch alone) is skipped by the
    checks of `loop_jumps` and `goto_jumps`.  The emitter sizes the
    branches afterwards.  Refused: a `case` table entry or a `dbf` whose
    target a `bra` follows, a cycle of `bra`s, a branch that would go to
    the next instruction once a `bra` is dropped (not seen; a compare
    chain whose last arm is only `return`), and a loop head falling into
    a `bra` (above).  Tests: `crates/dpc-m68k/tests/control.rs`
    (`branches_to_a_bra_are_threaded`,
    `loop_jumps_and_nested_whiles_are_threaded`), `crates/dpc/tests/driver.rs`
    (`goto_to_a_label_before_a_bra_is_threaded`), golden
    `tests/golden/control/threading.pas`, exec `tests/exec/threading.pas`
    and `tests/exec/threading_paths.pas`.
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
- **Identical ends of the two parts (Verified that it happens at the
  sites; when is not settled; not modelled; bead domain-pascal-sggk):** the pop is shared at
  the join (section 1.1), and RIP_$SEND_UPDATES shares a `jsr`: one part
  is `subq.l #2,SP; st -(SP); bra.s 0xE688BE`, the other `subq.l #2,SP;
  clr.w -(SP)` falling into the `jsr RIP_$BROADCAST` at 0xE688BE.  A
  whole statement is shared too: LOG_$ADD 0xE176C8..0xE176D2, `if size
  = 0 then head := 1 else begin head := head + size; if head >= 16#1fe
  then head := 1 end`, is `beq.s 0xE176D2; add.w D1,(A0); cmpi.w
  #0x1fe,(A0); blt.s 0xE176D6; 0xE176D2: move.w #1,(A0)`: the `then`
  part is only a branch to the copy inside the inner `if` (the `beq` on
  the flags of the load at 0xE176C4, no `tst`) (lead: ../source
  pascal-translation-notes finding 18; 7.3 "Shared code" has the same
  across `case` arms and separate statements).  But 0xE381A8 and 0xE381D2 are two `jsr 0xE825E4` ending the two parts of
  one `if`, not shared, and 0xE1A9F0/0xE1AA00 two identical `move.w
  0xE2E00E,D0` (of the 3324 forward `bra`s to a join reached by falling
  through, 4 have the same instruction before the `bra` and before the
  join).  dpc never shares anything but rejects the shared-pop case
  (bead domain-pascal-myk).
- **`and then` / `or else`** (ref pp. 4-18..4-19, 4-134): jumping code,
  each operand with its own branch: 0xE031A2..0xE031B6 `cmp.w D4,D2;
  bcc.s <exit>; ...; btst #13,D5; bne.s <exit>; tst.w D5; bmi.s <exit>;
  btst #14,D5; bne.s <head>` (a loop test `c1 or else ... or else c4`).
- **Plain `and` and `or` (survey Verified; implemented, bead
  domain-pascal-5aw.19).**  Ref "And", p. 4-19: with `and` "the system
  may evaluate both boolean_1 and boolean_2 to test if the statement is
  true.  Also, there is no guarantee that the system will evaluate the
  two operands in the order in which they appear" ("Or", p. 4-135, the
  same but "may have to evaluate both"; C.3.3: `and then`/`or else` "guarantee that Domain Pascal will
  evaluate Boolean expressions in the order that you write them").  So
  the compiler may evaluate both or short-circuit; the image evaluates
  both: each operand's boolean is formed in a data register and the two
  are combined with `and.b`/`or.b`, whose flags the branch reads.
  `tools/boolsurvey.py andor` (133 `and.b`/`or.b` into a data register
  with a boolean operand or a sign branch after; of their 266 operands):
  formed by `s<cc>` 167 times, an earlier `and.b`/`or.b` (a chain) 32, a
  boolean variable in memory as the source operand 20, `move.b` of a
  variable from memory or a register 6 and with `not.b` after it 18, not
  found within six instructions 22; the result read by `bpl` 39, `bmi` 31,
  a store 11, a further `and.b`/`or.b` 6 (and most of the 33 "other"
  readers are the next operand's compare), `move.b` into a register 5,
  `not.b` then `bmi` once.  Sites: TTY_$I_ERR 0xE1BE48..0xE1BE60 `cmpi.l
  #0x36000b,D0; seq D1b; ...; btst.l #0xa,D2; sne D3b; and.b D3b,D1b;
  bmi.b` (the `if` of ../source's `tty/i_err.pas`); 0xE4BC2A..0xE4BC4A
  `seq D0; ...; seq D1; or.b D1,D0; ...; seq D1; or.b D1,D0` (`a or b or
  c`, left to right, the right operand's register reused); 0xE01A20
  `move.b D2,D0; and.b D3,D0; bmi.s` (two register-held booleans);
  0xE39ED8 `move.b D4,D6; and.b (20,A6),D6; bpl.w`; 0xE0F552 `sgt D0;
  or.b (844,A5),D0; bpl.s`; 0xE1009C `move.b (26,A6),D3; not.b D3; and.b
  D3,D0; bpl.w`; 0xE0AED6 `or.b D1,D0; not.b D0; bmi.w`; stored 0xE38630
  `and.b D4,D1; move.b D1,(631,A3)`; into a register-held local 0xE072B6
  `or.b D1,D0; move.b D0,D4`.  LOG_$INIT 0xE30168..0xE3017A (../source
  `docs/pascal-translation-notes.md`, finding 17) has one inside an `or
  else`.  Which jumping-code sites of the image are plain `and`/`or`
  compiled as jumps cannot be told without the source; the TTY source
  reads them as `and then`/`or else`.
  **Implemented** (`dpc-ir` `Cond::And`/`Cond::Or`; `isel.rs`
  `logic_value`, `bool_into_reg`, `bool_operand`): the left operand into
  a register of its own (`s<cc>` of a comparison, `odd` or a constant
  set member; `move.b` of a boolean variable, from memory or its
  register; `not.b` after it for `not v`; a function's result in D0; a
  nested plain `and`/`or`), then `and.b`/`or.b` of the right one (a
  boolean variable as it is, in memory or in its register; anything
  else formed in a register of its own); in a condition `bmi`/`bpl` on
  its flags, `not (a or b)` with `not.b`; stored with `move.b Dn,<dst>`;
  into a register-held local formed apart and moved.  A comparison's
  `s<cc>` reuses only a register the comparison itself loaded or
  computed (0xE4BC64..0xE4BC6E `clr.w D1; move.b (9,A6),D1; cmpi.w
  #17,D1; seq D1`), never a register-held local's, a parameter's or a
  held value's that it merely reads (`(n > 3) or b` ending a `repeat`
  over a register-held `n`: a fresh register; exec test `bools`,
  BOOLS_$LOOP).  Refused: `not` of a comparison as an operand (0xE13B8A forms `shi` and then `not.b`s it:
  not explained), an `and then`/`or else` inside a plain `and`/`or`,
  pushing the value as an argument (no site), a right operand that
  calls a function (next item).  Golden `control/and_or`; exec test
  `bools`; dpc-ir test `plain_and_or_and_odd`.
- **A plain `or`/`and` whose right operand calls a function (Verified
  at the sites given for the form; the register partly Uncertain; a
  reading, the source unknown; beads domain-pascal-i1by, -1rwo):** seven
  TPAD routines validate a unit as `tst.w (A4); ble.s <then>; jsr
  SMD_$N_DEVICES; cmp.w (A4),D0; slt D2; tst.b D2; bpl.s <else>`
  (TPAD_$SET_UNIT_MODE 0xE6980A..0xE6981A; TPAD_$SET_UNIT 0xE699EE with
  `sge D2` for the `and` of the opposite tests): the left operand is
  jumping code, the right one, which contains the call, formed as a
  boolean and tested.  A boolean local would explain the `s<cc>`, but
  TPAD_$SET_UNIT has `link #0` (register-held locals keep their slot,
  12.5).  Read as `(unitp <= 0) or (unitp > smd_$n_devices)` (ref "Or",
  p. 4-135: no order of evaluation guaranteed): the left operand
  short-circuited, the right evaluated as a value, no `or.b` (lead:
  ../source pascal-translation-notes finding 36).
  Survey (`tools/cmpsurvey.py summary`, kinds `callcmp` and `andor`; the
  first instruction after a call, its pops skipped, that compares D0;
  calls of M$ARITH left out, since they compute `*`, `div`, `mod`, not a
  function's result, section 11):
  - of the 76 word and longword compares, 75 have the call's result as
    the *destination* (`tst.<s> D0`, `cmpi.<s> #c,D0`, `cmp.<s> X,D0`),
    a variable in memory read after the call; 1 has it as the source,
    DISK_$SPIN_DOWN 0xE3DB34 `jsr (A1); addq.w #4,SP; cmp.w D0,D2; bge.s
    +2; move.w D0,D2`, a call through a routine pointer whose result is
    kept afterwards (a `max`-like select, not `if f() > v`);
  - compares with a variable: formed as a boolean and tested (`s<cc> Dn;
    tst.b Dn; bpl/bmi`) at 14, branched on only at 0xE3DB34.  The four
    other branched compares a first count included are M$ARITH results,
    each a `min`-like select (NETWORK_$READ_AHEAD 0xE0FD5A M$DIU$LLW
    `cmp.l (d,A6),D0; ble.s +4; move.l (d,A6),D0`; MST_$INIT 0xE30CD2
    M$MIU$LLW, `bls`; the module-locals 0xE10628 at 0xE10AA2 M$DIS$LLL
    `cmp.l D0,D4; ble.s +2; move.l D0,D4` and 0xE4DFFE at 0xE4E060
    M$DIU$LLW `cmp.l D0,D3; bls`).  Of the 14, 12 are in a chain of
    jumping tests to the same label (the 7 TPAD sites, 0xE10D0C,
    0xE184FA, 0xE3D7F4, 0xE68B0E, 0xE75ECE) and 2 stand alone: DISK_IO
    0xE3D86E, an `if` reached by a `beq` to the call's argument push
    (0xE3D856), `cmp.w (0x3a,A3),D0; sne D2; tst.b D2; bpl` past the
    `then` part; and 0xE5E590, the end of a `repeat` (no branch into
    the test), `cmp.l D3,D0; sne D5; tst.b D5;
    bmi` back to the head (../source check_same_volume.c `while
    (dism_seqn_end != dism_seqn_start)`: the loop continues on `<>`, so
    an `until x = y` formed with the negated relation, the source
    unknown).  No compare of a function's result with a variable
    branches on the compare.  The 14 by the variable's kind and the
    `s<cc>` register: through a pointer, D2 (the 5 TPAD sites, 0xE184FA,
    DISK_IO 0xE3D7F4 and 0xE3D86E); register-held, D5 with D2..D4 live
    (0xE5E590, 0xE68B0E); a static variable off A5, D0 (TPAD_$SET_UNIT_CURSOR
    0xE698E0, TPAD_$INQUIRE_UNIT 0xE6998A); an enclosing routine's
    variable through the dynamic link, D1 (the module-locals 0xE10628 at
    0xE10D0C, 0xE75DE4 at 0xE75ECE).  The register therefore depends on
    the variable's kind: not explained;
  - compares with 0 or a constant: branched on at 40 (`tst.w D0; beq`
    14, ...), formed and tested at 20 (EC2_$WAIT 0xE4262A `cmpi.w
    #2,D0; seq D2; tst.b D2; bpl` stands alone), every one in a callee-saved
    register (D2 11, D3 4, D7 2, D4, D5, D6; 0xE6EBE2 `sne D3` with D2
    live; 0xE12B6A `seq D7`), formed as a function's result at 1
    (SOCK_$PUT_INT 0xE161EA `seq D0`); FILE_$SET_PROT_INT
    0xE5DE5A/0xE5DE7C are the last operands of a chain of jumping tests,
    the others not classified;
  - `and.b`/`or.b` into a data register: 318, none whose right operand
    calls (a call between the left operand's register being formed and
    the combination).
  **Implemented** (`isel.rs`, `Selector::branch`, `call_compare_test`):
  a plain `and`/`or` in a condition whose right operand calls and whose
  left operand is a comparison without a call: the left operand as `and
  then`/`or else` jumping code, the right one a comparison of a D0
  function's result with 0 (`tst`), a constant (`cmpi`), a variable
  read through a pointer or a register-held local (`cmp.<s> v,D0`, the
  relation reversed when the source has the call on the right), `s<cc>`
  into a data register coloured as if live across the call (kept apart
  from D0 and D1, and from the compared register: the lowest
  callee-saved one free), `tst.b`, `bmi`/`bpl`.  TPAD_$SET_UNIT
  0xE699DC compiles byte-identical (srcsurvey); TPAD_$SET_UNIT_MODE,
  _SET_PUNCH_IMPACT and _INQ_PUNCH_IMPACT then stop at the element
  scaling by 44 (bead domain-pascal-r3yy).  Refused: a static
  variable (D0) or an enclosing routine's (D1), not explained; a frame
  variable, another unit's, an element, one whose address is loaded
  for the compare (not seen in this form); a left operand other than a
  comparison (0xE5DE5A's chain: a nested plain `and` or `and then`?);
  the value outside a condition or under `not` (no site); and a compare
  of a function's result with a variable anywhere else, `if f() < v`
  included (no site branches on such a compare; the two that stand
  alone form the boolean, but in a `repeat` with the negated relation,
  and which relation an `if`/`while`/`until` forms is not settled;
  bead domain-pascal-l78t).  Before this, dpc compiled `if f() < v`
  with the variable loaded before the call and `cmp.w Dn,D0; b<cc>`,
  a form no site has.  Golden
  `control/and_or_call`; exec test `and_or_call`; driver test
  `call_result_compares_outside_a_plain_and_or_are_refused`.
- **Implemented** (`crates/dpc-ir`, `Cond`, `StmtKind::If`/`Return`;
  `crates/dpc-m68k/src/isel.rs`, `Selector::if_`, `branch`, `compare`):
  all of the above that is Verified, with `not`, `and then`, `or else`.
  A computed left operand (an expression, a function result, an array
  element's value) is compared in its register with `cmpi #c,Dn` (0xE1216E
  `add.l D6,D7; cmpi.l #0x500,D7`) or `cmp.<s> x,Dn` (`compare_computed`);
  against 0 the branch reads the operation's flags, or `tst` comes first
  ("A computed value against 0" above; bead domain-pascal-qlzu).
  A function's result against a variable is refused outside the right
  operand of a plain `and`/`or` (formed as a boolean at all 14 sites,
  2 of them outside a chain; the relation formed in a loop test not
  settled: "A plain `or`/`and` whose right operand calls a function"
  above; bead domain-pascal-l78t).  A
  function's boolean result is tested with `tst.b D0` (section 3); `k in
  s` for a constant k with `btst #k` (section 5.1).  Not supported yet: comparing booleans
  (`b = true`: compiled code tests booleans with `tst.b`, and has two
  `cmpi.b #0xFF` only, 0xE1B96C, 0xE53FDE, not shown to be booleans);
  a constant on the left of a comparison; the refusals of "Two
  variables", "Mixed widths and signs" and "Records" above; unsigned
  ordering against 0; a test right
  after a load of its register at the test's size (above); an `if` with an empty `then`
  part; a `return` at the
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
  A `while` can also be entered by a conditional branch to its test
  with no `bra` at all, when the statement before it ends in a `bra`
  (TTY_$K_PUT 0xE67C0E, TTY_$K_GET 0xE1C448; 7.1 "Branch threading":
  the opening `bra`, no longer reached, is dropped and the head stays
  aligned in aligning code, the module-local 0xE4AFA8 at 0xE4B0F8;
  **implemented**).


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
  Three more in KBD (Verified at the sites, jump tables decoded; lead:
  ../source pascal-translation-notes finding 27): KBD_$RCV, table at
  0xE1CCFE, arm 2 (0xE1CD18) is `jsr MMU_$NORMAL_MODE; tst.b D0; bmi.b
  0xE1CD3A`, and 0xE1CD3A is the code of labels 1, 10, 11 and 12
  (`subq.l #2,SP; pea (A2); move.b (12,A6),-(SP); bsr.w 0xE1CC10;
  addq.w #8,SP; bra.w 0xE1CE1E`), so arm 2's `else` part, the same
  call, exists only as that later arm; module-local 0xE1CAFE (../source
  kbd_$fetch_key), table at
  0xE1CB66, the arm of labels 1, 2, 13 and 14 ends in label 10's `st
  D3; bra.s 0xE1CBC8` (0xE1CB9C), reached by its `bne.s` (0xE1CB88) and
  its `bra.s` (0xE1CB96); and across two statements, KBD_$GET_DESC
  0xE1AA4C `bcc.s 0xE1AA68`, the first `if`'s `then` part (`status :=
  16#000B000D; return`) merged into the second's (`move.l
  #0xB000D,(A0); bra.s 0xE1AA98`) with `clr.l (A0)` and a test between
  (no single condition).  At these sites, as at AS_$GET_ADDR, the
  merged sequence ends in a `bra` (to a join or the epilogue) and the
  copy kept is the later one (Reported as a rule; no counter-survey).
  Not modelled (beads domain-pascal-pil, domain-pascal-myk,
  domain-pascal-sggk).

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
in a jump table (`return`: its entry would name a `bra`, and no threaded
table entry is seen, 7.1 "Branch threading"; in a compare chain the
`beq` is threaded to the epilogue, and a `case` at the end of an `if`
part followed by a `bra` has its arms' branches threaded); label ranges
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
(0xE5C694, 0x36C bytes) is blocked at its first instructions: five of
its seven by-address slots are loaded at entry into D3, D4, A2, D5, D6
and moved to A0/A1 for each use (`movea.l D6,A0; clr.l (A0)` at
0xE5C6B0); `dpc` has the copies (12.2, "Pointers in data registers",
bead domain-pascal-pze) but not the rule that puts these
dereference-only slots in D registers (bead domain-pascal-0b1; the
unit's other blockers: bead domain-pascal-2q2); then `btst #5,(1,A1)`
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
- **A `while` condition's value spilled to the frame (Verified at the
  site).**  KBD_$RCV 0xE1CCC0 saves only A2/A3/A5 and no data
  register.  Its loop `while kbd_$fetch_key(...) do if fetched_mode = 0
  then ...` (../source's reading) stores the call's boolean to the
  frame at the test, then loads the body's first operand into D0 before
  testing the stored byte: `bsr.w 0xE1CAFE; lea (12,SP),SP; move.b
  D0,(-0x1e,A6); move.w (-0x14,A6),D0; tst.b (-0x1e,A6); bmi.b
  0xE1CE44` (0xE1CE6C..0xE1CE80); the head 0xE1CE44 is `tst.w D0; bne.s
  <test>`.  The byte at -0x1E lies below the -0x1C temporary (`move.l
  A0,(-0x1c,A6); pea (-0x1c,A6)`, 0xE1CDFC), so it is a temporary
  (section 2, "Frame size", item 4).  No other site looked for.  `dpc`
  tests D0 straight after the call and reads the local in the body
  (lead: ../source pascal-translation-notes finding 28; bead
  domain-pascal-b8dz).
- **A loop entered past its test's `tst` (Verified at the site).**  The
  module-local 0xE1CAFE (../source kbd_$fetch_key), `while (not found)
  and then (head <> kbd.ring.tail) do`, is entered by `move.b D3,D5;
  bra.w 0xE1CBE4` (0xE1CB28; D3 is `found`, just cleared): the target
  is the test's `bmi.s` (0xE1CBE4), past its `tst.b D3` (0xE1CBE2),
  the `move.b` having set the flags from D3 already.  Compare the
  module-local 0xE1A4A4 (section 9), which branches past a `tst.l` to
  its `seq` (bead domain-pascal-gaj).  `dpc` branches to the `tst`
  (lead: ../source pascal-translation-notes finding 30; bead
  domain-pascal-b8dz).
- **`while p <> nil` on a pointer variable** (ad-hoc script over the
  bottom tests): 45 loops test an address register (`cmpa.w #0,An`), 12 a
  frame slot (`tst.l (d,A6)`), a few a D register or the flags of a
  `move.l`.  So a list walk's pointer local is usually in an address
  register (12.5).
- **`repeat`**: fall-through heads, aligned (section 7.2).  The `bra`
  rows (a test at the top, a `bra` back; 7.2): source construct not
  identified; `dpc` never emits it.
- **A pointer walk in a `while` loop (Verified at the site; the rule
  Uncertain).**  The stepped pointers below are not only in `for`
  loops, and not only for elements wider than a byte: the module-local
  0xE1B00A (../source tty_$i_put_chars) has a `while` loop
  0xE1B07A..0xE1B32C, entered by `bra.w` at 0xE1B076, that reads the
  characters of its by-address buffer with `move.b (A2)+,D5`
  (0xE1B07A), A2 the parameter's slot loaded at 0xE1B072 (`movea.l
  (12,A6),A2`): the in-place forming.  The count of characters handled
  is kept apart in D2 (`clr.w D2` 0xE1B070; `addq.w #1,D2` 0xE1B07E),
  since the test and the result read it (`move.w D2,D0` 0xE1B388).
  How often `while` loops get a stepped pointer is not surveyed.  `dpc`
  steps pointers in `for` loops only (lead: ../source
  pascal-translation-notes finding 3; bead domain-pascal-j8pw).
- **Loop invariants held in registers in some loops only (Verified at
  the sites; which loops get it Uncertain; ref 6.4.24 p. 6-29,
  "Removal of invariant expressions from loops").**  The module-local
  0xE1B716 (../source tty_$i_word_erase) has two `while` loops with
  the same references.  Before the first it forms two field addresses
  (`lea (0x2ce,A2),A3; lea (0x2ca,A2),A4`, 0xE1B722), used by the body
  and the test (`cmpi.w #1,(A3)` 0xE1B72C; `move.w (A3),D0; cmp.w
  (A4),D0` 0xE1B75E); the second loop (0xE1B766..0xE1B7A4) reaches the
  same fields as `(0x2ce,A2)` and `(0x2ca,A2)`, although A3 and A4 are
  still saved and unchanged.  Not a `with`: the rest of the first loop
  still goes through A2.  (Both exits of the first loop go straight to
  the second loop's test: `beq.s 0xE1B79C` at 0xE1B754, `bra.s
  0xE1B79C` at 0xE1B764.)  TTY_$K_DRAIN_OUTPUT 0xE67AAE (saves
  D2-D7/A2-A5) forms before its loop head 0xE67B0C the bases of
  FIM_$QUIT_EC and FIM_$QUIT_VALUE (`move.l #0xe22002,D6; move.l
  #0xe222ba,D7`), PROC1_$AS_ID's address (`movea.l #0xe2060a,A4`), and
  two field addresses from a copy of the descriptor pointer (`movea.l
  A2,A3` 0xE67ADE; `move.l #0x3d2,D4; move.l #0x3d4,D5; add.l A3,D4;
  add.l A3,D5`, 0xE67AEA..0xE67B0A), used as `movea.l D6,A1`, `move.w
  (A4),D1`, `movea.l D4,A1; move.w (A1),D2` (0xE67B1E..0xE67B48).
  TTY_$K_PUT 0xE67BB2 (saves D2-D5/A2-A5) builds the same two EC_$WAITN
  lists in its `while` body (0xE67CBE..0xE67D00) and forms nothing:
  `move.w (0xe2060a).l,Dn` (D2 at 0xE67CD0, D0 at 0xE67CEC) and
  `movea.l #0xe22002,A1` / `#0xe222ba,A1` at each use,
  though every path from its head 0xE67C5E to the back edge passes
  there and D6/D7 are free (not saved).  Neither register pressure nor
  the loop kind explains it: DRAIN's loop is entered by falling
  through, K_PUT's by a branch to its test, and word_erase's two
  `while` loops differ.  `dpc` hoists no invariants (only the bases of
  stepped pointers, below) (lead: ../source pascal-translation-notes
  findings 4 and 12; bead domain-pascal-9k9m).

**`for` loops** (`fors`; 963 `dbf` loops):

- **A counted loop is a `dbf` loop with a count of N-1** (section 7).
  With **constant bounds** the count is `moveq #N-1` (558 loops) or
  `move.w #N-1` above 127 (47; ad-hoc script: the last definition of the
  `dbf` register before the head).  A count of 0 (one pass) is `clr.w
  Dn`, never `moveq #0` (Verified, 6 of 6 `dbf` counters set to 0, review
  of bead domain-pascal-xm5w: SMD_$DM_COND_EVENT_WAIT 0xE6F012 `clr.w
  D0`, SMD_$FREE_ASID 0xE75262 `clr.w D3`, SMD_$INIT 0xE34DB8 `clr.w D2`,
  each a constant one-pass loop; REM_FILE_$GET_SEG_MAP 0xE61FF0, 0xE62014,
  FILE_$GET_SEG_MAP 0xE740B0).
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
  RING_$KICK_DRIVER 0xE768BC, 0xE42244).  **Forming** (bead
  domain-pascal-5aw.17; the preheaders of the guarded loops below and of
  the sites above): after the count and the index; a pointer reached
  through a pointer variable is loaded into an address register (or is
  the one loaded at entry), all such bases first, then each stepping
  pointer formed with `lea (from × size,An),Ap` into a new register
  (0xE4F010 `movea.l D6,A4; lea (2,A3),A1; lea (2,A4),A0`; 0xE6FCC6 loads
  its second base before forming the first pointer; 0xE037EA `movea.l
  (18,A6),A3; lea (4,A3),A0`; 0xE7707A; `lea (A3),A2` for 0, 0xE3E4C6).
  In place instead, the base register becoming the pointer: 0xE7632C,
  0xE76378, 0xE76448 `movea.l (d,A6),A0; ...; lea (12,A0),A0`; 0xE6FCC6
  `movea.l (32,A6),A1; ...; addq.l #2,A1` (not reproduced; R2 gives a new
  register, as at 0xE177E0 where A1 dies; `ptrs` below: 202 new
  registers against 14 in place from a pointer, no rule).
- **Arrays of the unit's data, of the frame and of other units (survey
  Verified, rule fitted; implemented; bead domain-pascal-xm5w).**
  `tools/loopalign.py ptrs [-v]` takes every stepped pointer of the
  `dbf` loops (a register stepped at the bottom by `addq.l`/`subq.l`/`lea
  (s,An),An`, its preheader definitions back to the first that is not an
  in-place adjustment; loops of one instruction, block copies and
  fills, counted apart) and classes it by that first definition:

  | Formed from | `lea (k,B),Ap` | `movea.l B,Ap` (k = 0) | `movea.l B,Ap` + `addq.l`/`lea` in place |
  |---|---|---|---|
  | the unit's data (B = A5) | 28 | 15 | 17 |
  | the frame (B = A6) | 35 | 5 | 13 |
  | an absolute address (`#sym`) | 0 | 16 | 26 |
  | a pointer in a register | 202 | (53 copies) | 3 |
  | a pointer loaded from the frame | | 27 (the load itself) | 11 |

  - *Where the array's offset goes:* never into the pointer.  The
    pointer is `B + from × size` and each access's displacement is the
    array's offset from B plus the field's less `lo × size`: DBUF_$
    INVALIDATE 0xE3AA0E `lea (36,A5),A2` (36-byte entries from 1, the
    array at 0x10, `e_flags` at 0xC reached as `(-8,A3)`, 0x10 + 0xC -
    36); EC2_$REGISTER_EC1 0xE42968 `movea.l A5,A0; moveq #1,D1; addq.l
    #4,A0`, `(2700,A0)`; the absolute base is the variable's symbol
    itself (MAC_$NET_TO_PORT_NUM 0xE0C376 `movea.l #ROUTE_$PORTP,A2`;
    NET_IO_$CREATE_PORT 0xE5A5C4 `movea.l #ROUTE_$USER_STAT,A0; ...; lea
    (144,A0),A0`, the elements at `(-144,A0)`; MMAP_$INIT 0xE31946
    `movea.l #MMAP_$WS_OWNER,A0`; DISK_$DISMOUNT 0xE6D05C
    `#DISK_$DVTBL`; every immediate the map names is a variable's
    start, the others data blocks).
  - *`lea` or `movea.l` for A5/A6:* `lea (k,B),Ap` when every
    displacement the body reaches an element at through the pointer (or
    through a `with` copy of it, below), and that displacement plus `from
    × size` (the element's offset from B in the first pass), lie in
    -128..127; `movea.l B,Ap` adjusted in place otherwise.  The frame:
    52 of 52 pointers with displacements (OS_$INSTALL_DISPLAY_ASTE: two
    loops in a row, 0xE6D2BE `movea.l A6,A0; moveq #35,D0; addq.l #4,A0`
    with `(-180,A0)`, 0xE6D2CE `moveq #7,D0; lea (4,A6),A0` with
    `(-36,A0)`; 0xE134D6 both forms in one preheader, `(-76,A1)` and
    `(-140,A0)`; FILE_$FW_PAGES 0xE5E7D2 in place, -132 though -128 in
    the first pass).  The unit's data: 57 of 60 (FLP's module-local
    0xE3DC78, ../source flp_format_track, at 0xE3DCB6 in place with
    `(124,A0)`..`(127,A0)`, 128..131 in the first pass;
    NETWORK_$INSTALL_NET 0xE0F208 `lea (8,A5),A2` with `(60,A2)`); the
    three others, ROUTE_$FIND_PORT 0xE15B12, ROUTE_$FIND_PORTP 0xE15B5E
    (`(4,A0)`) and the module-local 0xE17EAE (`(68,A2)`, while its inner
    loop at 0xE17EC4 has `lea (A5),A1` with `(32..36,A1)`), are `movea.l
    A5,An` with `from × size` 0 and are not explained (bead
    domain-pascal-s9tx).  Not an exported
    array (PKT_$MISSING_NODE, DBUF and RING_$CTL are exported and formed
    with `lea`); all three load a pointer element straight into an
    address register (`movea.l (4,A0),A1`), as do the in-place
    AREA_$SHUTDOWN 0xE07F42 and DISK_$ADD_QUE 0xE3C914 and no `lea`
    site of the unit's data, but the frame's AST_$UPDATE 0xE01E72 `lea
    (4,A6),A3` is one (`movea.l (-20,A3),A2`, 0xE01E76): too few sites to
    tell.  A test
    of `from × size` alone, or of the displacements alone, does worse
    (D0 alone: 106 of 112; the first pass alone: 108).  An absolute base
    is always adjusted in place, whatever the displacements (HINT_$INIT_
    CACHE 0xE313DA `movea.l #block,A0; moveq #1,D0; lea (12,A0),A0` with
    `(-12..-4,A0)`, where HINT_$LOOKUP_CACHE 0xE49D34 has `lea (12,A5)`
    for the same array).
  - *Placement:* `lea` after the count and the index, as from a pointer
    (FIND_VOLX 0xE6B0DA `moveq #5,D3; moveq #1,D4; lea (32,A5),A0`; DBUF_$
    INVALIDATE 0xE3AA0C `move.w D0,D4; lea (36,A5),A2`, the count's
    copy).  In place with k = 0, `movea.l` after the count and the index
    (`CM` 13, `CIM` 7: AREA_$SHUTDOWN 0xE07F3E `moveq #10,D3; movea.l
    A5,A2`; XNS_IDP_$OS_DEMUX 0xE185B6 `moveq #15,D0; clr.w D1; movea.l
    A5,A1`).  With an adjustment the `movea.l` comes one instruction
    before the last count or index instruction (ad-hoc script, the
    preheaders of the in-place pointers: `MCA` 14, DISK_$INIT 0xE3C102
    `movea.l A5,A2; moveq #63,D2; lea (28,A2),A2`; `CMIA` 11, TIME_$INIT
    0xE2FE94 `moveq #63,D3; movea.l #abs,A2; moveq #1,D4; lea
    (12,A2),A2`, FILE_$LOCK_INIT 0xE32788; `MIA` 3, EC2_$REGISTER_EC1;
    RIP_$SERVER 0xE68D90 `movea.l A6,A3; move.w D1,D4; addq.l #6,A3`
    before the count's copy), but stays right before its adjustment
    after a branch (`MA` 4, EC2_$WAIT 0xE427D8 `bmi.s; movea.l A6,A1;
    addq.l #2,A1`), after another pointer's `lea` (0xE134D6 `lea
    (4,A6),A1; movea.l A6,A0; addq.l #4,A0`), after an instruction that
    reads the register (0xE3DCB6, A1 read by the `lea (6,A1),A2`
    before), and after a store (FILE_$FW_PAGES 0xE5E7CC).  The unsigned
    guard (DISK_$ADD_QUE 0xE3C90A `move.w D2,D1; movea.l A5,A0; subq.w
    #1,D1; moveq #1,D0; addq.l #4,A0`) does not fit; it is refused
    anyway.  Two sites keep the `movea.l` after the last count or index
    instruction where another preheader instruction follows to stand
    between it and its adjustment (review of bead domain-pascal-xm5w):
    AST_$COPY_AREA 0xE03B66 `move.w D0,D2; movea.l A6,A4; move.l
    #0xEB4800,D5; addq.l #4,A4`, PROC2_$INIT 0xE3049A `moveq #68,D0;
    moveq #2,D1; movea.l #abs,A1; movea.l #abs,A2; lea (456,A1),A1`: as
    if one instruction must separate the two, taken from below when there
    is one; dpc emits no such instruction there.  Not seen: a run-time
    count copied (a call in the body) together with the index; dpc puts
    the `movea.l` between the copy and the index, as the rule says.
    dpc's top-down R11 cannot move an instruction above an
    earlier one, so the selector places the `movea.l` itself.  Several
    pointers: 0xE6BB2C forms `movea.l A6,A4; lea (4,A6),A3; movea.l
    A6,A2; addq.l #4,A4; lea (32,A2),A2` (in first-use order, the
    adjustments after), but 0xE0E37A forms a pointer variable's `lea`
    before the frame's: not settled (bead domain-pascal-883c).
  - *Sharing:* one pointer serves two arrays of one element size at
    MSG_$$SEND 0xE0DB80 (`move.l (-116,A0),(-20,A0)`) and DIR_$GET_
    DEFAULT_ACL 0xE53418 (`(5460,A0)`, `(5524,A0)`), two at RIP_$SERVER
    0xE68D64 (`lea (A6),A1; lea (A6),A0`, `move.w (-10,A1),(-44,A0)`):
    not settled (bead domain-pascal-y1yj).  Different forms never share
    (0xE134D6, 0xE6BB2C).
  - *`with a[i] do`:* the body's record is a copy of the stepped pointer,
    `movea.l Ap,Aw` where the `with` stands, the array's offset and the
    field's in each reference (DBUF_$INVALIDATE 0xE3AA12 `movea.l
    A2,A3`, `(-8,A3)`, `(A3)`; XNS_IDP_$OS_DEMUX 0xE185BC `movea.l
    A1,A0`, `(216,A0)`; PROC2_$PGROUP_INFO 0xE42244; RING_$KICK_DRIVER
    0xE768BC; DISK_$REGISTER 0xE3D9EC; 30-odd copies at the heads of the
    loops of the table).  An element passed by address goes through the
    pointer: `pea (d,Ap)` (XPD_$INIT 0xE3236C `pea (1144,A2)`; TIME_$INIT
    0xE2FEA4 `pea (4860,A2)`).
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
  back).  By the loop's shape (bead domain-pascal-5aw.17; ad-hoc script
  over the same guards, not kept): without a call, an index register and
  no stepped pointer, in place 27, copied 22; an index register and
  stepped pointers 4 / 7; no index register and stepped pointers (LOG_$READ's
  worker's shape) 29 / 11; neither 4 / 5.  So the worker's shape does not
  decide: 0xE4F010 copies the count (`addq.w #1,D4; asr.w #1,D4; subq.w
  #1,D4; bmi; move.w D4,D0`) for a two-pointer word copy like the
  worker's, and one routine has both forms from the same `move.w D3,D0`
  (0xE02898: 0xE02980 copied, 0xE02A46 in place).  In 35 of the 45
  copies without a call `T`'s register is read in the body, usually as
  the index (`move.w D0,D1; moveq #1,D0`, 0xE3EB3C, 0xE4084C): the copied
  counter is a value of its own (12.4 R2), and keeping the count in place
  a choice made before allocation.  Without a call `T` is live after the
  loop in 5 copies and 2 in-place loops (approximated by a linear scan
  from the exit), so liveness does not decide either.
- **A guard formed as a boolean (Verified at the site; cause
  Uncertain).**  TPAD_$INIT 0xE33570, `for unit := 1 to smd_$n_devices
  do`, has `jsr SMD_$N_DEVICES; subq.w #1,D0; move.w D0,D2; slt D3;
  tst.b D3; bmi.w 0xE33628` (0xE33584..0xE33592), then `moveq #1,D3`,
  the index.  The `slt` reads the flags of the `move.w`, which clears
  V, so it is the sign of n-1, not the `subq`'s N xor V.  A byte scan
  of 0xE00800..0xE88834 finds no other `slt Dn` within two instructions
  of a `subq.w #1,Dn`; all seven `slt Dn` in the image are TPAD's
  (0xE3358E; 0xE69816, 0xE698E4, 0xE6998E, 0xE69A4E, 0xE69AFE,
  0xE69B54, the last six section 7.1's plain `or` with a call).
  DTTY_$WRITE_STRING, whose bound is a slot, branches on the `subq`
  (0xE1D6AC `subq.w #1,D0; bmi.s`).  Whether a bound that is a function
  result causes the boolean form is not known: one site (lead:
  ../source pascal-translation-notes finding 38; bead
  domain-pascal-b8dz).
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
  -4 and the index stored to -6.  This favours hypothesis (a) of
  section 2 (declared locals the compiler kept out of memory) over (b).
  (The DISK_ module-local 0xE3CCCE, `while`, one 4-byte local held in
  A1, `link #-8`, once counted here as unused space, is explained by the
  reserved slot of a routine that is not a global symbol: section 2,
  "Frame size".)
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
`stepped_arrays`, `stepped_pointers`, `loop_pointer_locals`, `CmpaZero`):

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
  - a `for` loop of the `Fill` shape stays as in section 7;
  - arrays of elements wider than a byte indexed by the index itself
    and reached through a pointer variable the body leaves unchanged
    (a by-address parameter, a pointer parameter or local, a pointer
    in the unit's data or another unit's when the body makes no call; a
    local a nested routine reaches only when the body makes no call; an
    assignment, block copy or address of a record holding the pointer
    counts as changing it),
    the array at offset 0 from the pointer: one stepped pointer per
    array and element size (bead domain-pascal-5aw.17), formed after
    the count and the index as above (`lea (from × size,An),Ap`, the
    bases loaded first), in the order of first use in evaluation order
    (the value of an assignment before its destination), the elements
    at `(d,Ap)` with the lower bound and field offsets in `d`, stepped
    after the index in the reverse order (`addq.l`/`subq.l #s` up to 8,
    `lea (±s,Ap),Ap` above); the index register only when the body
    reads the index otherwise.  The pointer's base load counts as one
    use before the loop for the entry loads of 12.2 (LOG_$READ's
    worker loads `buffer` at 0xE177CC, not at entry).
  - arrays of the unit's data, of the frame and of another unit's
    object (bead domain-pascal-xm5w; `isel.rs` `step_plan`,
    `step_in_place`): `lea (from × size,A5|A6),Ap` with the pointers
    through pointer variables when the displacements fit a byte as
    above, else, and always for another unit's object, `movea.l
    A5|A6|#sym,Ap` with its `addq.l`/`lea` adjustment, the `movea.l`
    placed before the last count or index instruction when adjusted;
    the array's offset in the displacements;
  - `with a[i] do` over a stepped element: `movea.l Ap,Aw` where the
    `with` stands, the array's offset in the references (the `with`
    temporary rebased, `with_rebase`); an element passed by address,
    `pea (d,Ap)`.
- `exit`, `next`, and `if c then exit|next` as one conditional branch.
- A slot used inside a loop body is loaded at entry like one used more
  than once (12.2: 0xE3CCD6; NAME_$VALIDATE's module-local 0xE4A004 loads
  the slots its loops use at entry, 0xE4A018/0xE4A01C, and the one used
  once outside them where it is needed, 0xE4A03C).
- A pointer local assigned in a loop is held in an address register
  (12.5, `loop_pointer_locals`).

**Not supported yet** (positioned errors): run-time bounds other than
the case above (no rule for the copy without calls; `tst.w; beq` for
unsigned counts; run-time `lo`; `downto`); stepped pointers through
arrays of another data section's block or the enclosing routine's frame
(not surveyed), from the start of the unit's data area with
displacements in a byte (11 `lea (A5)` against 3 `movea.l A5`), in
place together with another stepped pointer (order), two arrays of one
element size in the unit's data or the frame (sharing), the unit's data
addressed absolutely, at an offset from their pointer, through a
pointer the body may change, at an index computed from the loop index
(`a[i + 1]`), or reached at displacement 0 (`(An)+`, 0xE00ED8, 0xE0287A);
the address of a stepped element other than a `with` record's; the
in-place forming from a pointer (0xE7632C, 0xE6FCC6); an index initial value
outside -128..127 or equal to the count; byte or longword indices; an
index of another scope (0xE721F8, a nested routine's loop over its
parent's variable) or one a nested routine reaches (RINGLOG_$CNTL
0xE72296, the index stepped in the frame; section 6) or read after its
loop (MST_$ALLOC_ASID
0xE42D7C pushes D2 after leaving the loop with a `goto`); a loop that
never runs; an empty loop body; an `exit`/`next` that would branch to the
next instruction.

Goldens `tests/golden/loops/while.pas`, `loops/for.pas`,
`loops/stepped.pas`, `loops/stepped_data.pas`; exec tests
`tests/exec/stepped_data.pas`, `tests/exec/stepped_data_more.pas`; tests `crates/dpc-m68k/src/isel.rs`
(`for_with_index_and_call`, `for_forms`, `stepped_pointer_loops`,
`constant_address_stepped_loops`, `unsettled_for_loops`, `while_shape`,
`loop_pointer_local_rule`), `tools/tests/test_loopalign.py`
(`SteppedPointersTest`),
`crates/dpc-ir/src/tests.rs` (`for_loops`, `while_exit_next`).
Conformance (`make conform`): `tests/conformance/disk_unassign_all.pas`
(DISK_$UNASSIGN_ALL 0xE6BE1C; its `bsr.w` to DISK_$UNASSIGN, outside the
unit, through the manifest's `module-routines=` token and `dpc
--module-routine`), `tests/conformance/disk_set_chain_status.pas` (the
DISK_ module-local 0xE3CCCE).

### 7.5 `goto` and labels (ref "Goto", pp. 4-95..4-97; shapes Verified at the sites given; implemented for local `goto`s, bead domain-pascal-o4q)

**Language** (ref 2.2.2.1 p. 2-11; "Goto" pp. 4-95..4-97; 6.4.18
p. 6-23; Appendix D p. D-4; 9 p. 9-37).  A label is an identifier or an
unsigned integer declared in a `label` part, and prefixes one statement
of the routine that declares it ("(Name_of_Label) has not been declared
in routine (name_of_routine)", p. 4-95); labels are not permitted at
module level (p. 9-37).  A `goto` may leave any statement, may go to a
label of an enclosing routine ("you can use goto to jump directly from a
nested routine to an outer routine", p. 4-97), may enter an
`if`/`then`/`else` statement unless compiled with `-iso` (p. 6-23,
`-niso` the default) and may jump between the arms of a `case`
(Appendix D, ISO 6.8.1, p. D-4).  Jumping into a `for`, `while`,
`case`, `with` or `repeat` statement from outside it is illegal by
p. 4-95 (its `bad_gotos` example), while p. D-4 says Domain Pascal
permits jumps "from one structured statement into the middle of
another": reference-clarifications open question 30.

**Image** (bead domain-pascal-o4q; ad-hoc script over the C files of
`../source` at 3898a2ce with a `goto` (249 files; 59 backward `goto`s)
and `tools/loopalign.py`'s `head_rows`, not kept):

- **Forward `goto`: a `bra` or `b<cc>` to the label, which is a join
  like any other** (not aligned, section 7 "Other branch targets").
  `if c then goto L` is one conditional branch on `c` (no conditional
  branch over a `bra` in compiled code, 7.1).  `if c then T else goto
  L`: a branch on the negated condition to L, then T (TTY_$I_RCV
  0xE1BBCE..0xE1BBD2 `btst #3,D1; beq.s 0xE1BC38`, the crlf arm's `else
  goto normal_char`, a label on the normal arm 0xE1BC38 of the same
  `case`; `tty/i_rcv.pas` at 3898a2ce).  Whether a forward `goto` there
  is a source `goto` or the compiler's merging of identical code cannot
  always be told (`tty/i_rcv.pas`, `tty/i_err.pas`).
- **Backward `goto`: its label is a loop head (Verified as part of the
  loop-head rule of section 7).**  The targets of the C's backward
  `goto`s that are backward-branch targets in the image: in aligning
  routines 3 have the filler (NETWORK_$READ_AHEAD 0xE0FDA0; the REM_FILE
  module-local 0xE60FD8 at 0xE61130 and 0xE611E0), 4 are 4-aligned
  (BAT_$ALLOCATE 0xE3B294, FLP_DO_IO 0xE3DE64, PROC2_$WAIT 0xE3FE84,
  PKT_$LIKELY_TO_ANSWER 0xE12AE4) and 1
  is at 2 mod 4, a 0x3D0-byte outer loop (NETWORK_$READ_AHEAD 0xE0FDE2,
  like the large loops of section 7); in non-aligning routines none has
  the filler (0xE09AA6, 0xE0263E, 0xE3C752, 0xE116DE, 0xE1981A at 2 mod
  4; 0xE05C98, 0xE3A608, 0xE5F1A0 4-aligned).  The C's `goto`s are
  reconstructions: what this measures is the backward branch, whatever
  the source construct was.
- **Non-local `goto`** (out of a nested routine): not looked for.
- **A `goto` to the next instruction**, a label right before a `bra`, a
  `case` arm that is only a `goto` (7.3: the one jump arm seen goes to
  its target directly): not seen.

**Implemented** (`crates/dpc-sema/src/check.rs`, `declare_label`,
`label_owner`; `crates/dpc-ir/src/lower.rs`, `routine_labels`;
`StmtKind::Label`/`Goto`; `crates/dpc-m68k/src/isel.rs`,
`place_user_label`, `goto_selected`, `in_loop`, `label_tails`, `if_`):

- sema: the checks of the language above (module level, the declaring
  routine, one statement per label, a `goto`'s label prefixes a
  statement);
- lowering: `L: s` is `Label(L)` then `s` in the same sequence; jumps
  into loops, `with` and `case` from outside are refused (question 30),
  as are non-local `goto`s;
- selection: `goto` a `bra`; `if c then goto L` and `if c then T else
  goto L` as above; a label that a `goto` selected later still branches
  to gets the loop head's `Align4` (dropped in non-aligning routines),
  and from it to that `goto` the code counts as inside a loop, so the
  analyses that see only the code selected so far (12.5 "Stored locals
  pushed from the register", `forwarded`; `a0_holds`; 12.4 R7,
  `cse_valid`) do not hold a value across its back edge; a branch from
  before such a value to a label after it ends it as for any join.  The
  pop rule (1.1) follows a `goto` to its label: a call whose
  continuation is a `goto` is not popped when the code from the label to
  the epilogue is clean (**Uncertain**: the rule for joins applied
  through the `bra`, no site checked).  Refused: a `goto` to the next
  instruction, a statement after a `goto` that no label starts.  A
  `goto` to a label followed by a `bra` goes to the `bra`'s target (7.1,
  "Branch threading").

Goldens `tests/golden/control/goto.pas`, `regalloc/goto_reuse.pas`;
tests `crates/dpc-ir/src/tests.rs` (`goto_and_labels`, `goto_targets`),
`crates/dpc-sema/src/tests/checks.rs` (`goto_and_labels`),
`crates/dpc-m68k/tests/control.rs` (`pop_rule_through_goto`),
`crates/dpc/tests/driver.rs` (`goto_refusals`,
`backward_goto_label_alignment`).  **No conformance unit:** TTY_$I_RCV
(0xE1B92A) stops at the assignment to its value parameter `ch`
(`tty/i_rcv.pas`, "assigning to a parameter is not supported yet") and
needs the merged arm tails of 7.3; no other compiled routine with a
`goto` has been written in Pascal.

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
  rule, `dpc` uses the majority form.  Except: a value held in a data
  register with a constant `moveq` cannot load is copied and masked with
  `andi` (TTY_$I_RCV 0xE1BACE `move.w D4,D1; andi.w #0x80,D1`: 24 such
  copies, no constant-first `move #c,Dn; and Dm,Dn` with any register;
  `tools/setsurvey.py`, "andc"; implemented, bead domain-pascal-7qo).
  On a computed value: `andi` in place (0xE11024 after `lsr.w`).
- **`!`, `xor` (Uncertain):** on a computed value, `ori`/`eori` in place
  (0xE03498 `andi.w #-0x4001,D1; ori.w #0x2000,D1`); with a variable both
  orders occur (4 constant-first `or`s, 5 loads and `ori`): rejected.
  `eor` is rare (7 `eor`, 1 `eori` in compiled code).
- **`-x`, `~x`:** `neg.<s>`, `not.<s>` on the value in a register (`neg.w`
  ×97, many from the multiplications below; `not.w` ×13, `not.l` ×46,
  `not.b` ×43).
- **`abs(x)` (ref "Abs", p. 4-12; Verified at the sites, one routine;
  not implemented):** a copy, then `bpl.s` over `neg.w`, on the flags
  of the instruction that formed the value: `move.w D2,D5; bpl.s; neg.w
  D5` (TPAD_$DATA 0xE6924A, 0xE6927E; into D3 at 0xE69764; `move.w
  D1,D0` at 0xE692D4), a `muls.w` (0xE69624
  `move.w D2,D5; muls.w (-0x10,A2),D5; bpl.s; neg.w D5`: the 32-bit
  product's sign, only the low word negated), or a `tst.w` after
  `divs.w` (0xE6966E).  The result is then treated as a non-negative
  word: compared signed with an integer variable (0xE69250 `cmp.w
  (-0x12,A2),D5; bge.s`), unsigned with a constant (0xE69674 `cmpi.w
  #5,D0; bls.s`), and zero-extended where it enters longword arithmetic
  (0xE69284 `clr.l D0; ...; move.w D5,D0; addi.l #10,D0`, then a call of
  M$MIS$LLW, not the `muls.w` of two signed words; 0xE6976A..0xE69778
  `clr.l D5; move.w D3,D5; ...; clr.l D6; move.w D3,D6; add.l D6,D5`,
  compared `cmp.l` with an `ext.l` word).  As if the result's type were
  a non-negative subrange (the forms agree for 0..32767; abs(-32768) is
  the one value where they do not).  One routine; Reported as a general
  rule (lead: ../source pascal-translation-notes finding 37).  dpc folds
  `abs` of a constant (dpc-sema) and refuses a run-time one (dpc-ir,
  "this predeclared function is ..."; bead domain-pascal-1che).
- **`odd(x)` (ref "Odd", p. 4-128; register form Verified at two sites;
  memory form read off 9 sites, Uncertain; implemented, beads
  domain-pascal-5aw.19, domain-pascal-3xwx).**  `btst #0` on the integer
  in a data register, the branch on Z: FIM_$BUILD_DF 0xE0A6F6 `btst
  #0,D5; beq.s` and 0xE0A8EE `btst #0,D0; bne.s`, both register-held
  longwords (`fim/build_df.c`: `(new_usp & 1) != 0`, `(fp & 1) != 0`).
  No `andi #1` tests a value in compiled code.  `tools/boolsurvey.py
  bit0` counts every `btst #0`: on a data register 25 `beq`, 4 `bne`, 6
  `sne`, 1 `seq`; on memory at an odd displacement 42 `beq`, 27 `bne`,
  13 `sne`, at an even one 12.  Member 0 of a set held in a register is
  the same instruction (ref 3.9.3), so the image cannot tell `odd` from
  `0 in s` there; the code is the same either way.  As a value (the
  operand of a plain `and`/`or`, 7.1), `sne` into a new register when
  the tested one holds a variable (AST_$GET_LOCATION 0xE0472E `btst
  #0,D2; sne D0`; DISK_$ADD_QUE 0xE3C744 `btst #0,D2; sne D4`), into the
  tested one when it holds a value computed for the test, as `in` does
  (5.1, 0xE4A412).  A word or longword variable in memory: loaded
  whole, then `lsr.b #1` and the carry, `bcs` when odd (tty_$i_put_chars
  0xE1B0F6 `move.l (12,A0),D1; lsr.b #1,D1; bcc.s`; AREA_$COPY 0xE0905A;
  all 9 `lsr.b #1` of compiled code, 5.1 "Member 0 (or bit 0) as `lsr.b
  #1`", where the reading is argued: `btst #0` after a whole load is
  `(x & 1) <> 0`, 7.1 "One bit"; `0 in s` of a set in memory is
  `btst #0` on its last byte).  **Not supported** (`isel.rs`,
  `odd_test`): `odd` of a variable in memory as a value (`lsr.b #1;
  scs` into another register at both sites, 0xE052FA and 0xE70E98, the
  latter's load in D3: bead domain-pascal-uwfe), `odd` of a byte
  (`dpc-ir`), and storing `odd(x)` of a register (like any comparison
  stored, 7.1).  Goldens `control/and_or`, `control/one_bit`; exec tests
  `bools`, `onebit`.
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
  | other, c of 1..127 | `moveq #c,Dn; mul<s>.w x,Dn`, the variable the source operand (section 5.2, "Scalings by element size") | 44 (TPAD_$SET_UNIT_MODE 0xE69824), 46, 84 (HINT_$GET_HINTS 0xE4999A), 92, 100 |
  | other, c of 128 and more | `mulu.w #c,Dn` (word results) | 150, 153, 204, 228, 244, 250, 268, 300, 336, 363, 564 (BAT_$GET_BAT_STEP 0xE3BB6A), 580, 1083, 1244, 1624, 5240, 12345 |

  The one exception is ×26 at 0xE71BBA (odd part 13, done bit by bit:
  `add.w D1,D1; move.w D1,D0; lsl.w #2,D0; add.w D0,D1; add.w D0,D0;
  add.w D0,D1`), while other odd parts of three bits (11, 21, 25, 67,
  145: ×44, ×336 and ×84, ×100, ×268, ×580) use a multiply: `dpc` multiplies by
  those and rejects the other odd parts of three bits other than 2^k ±
  1, and 13 (bead domain-pascal-irb4).  A 68000 cycle count does not
  separate the two groups (268 and 336 would be cheaper with shifts).
  A constant of 1..127 multiplied with `mul<s>.w` goes into the
  register first (43 of 44 sites, `tools/arithsurvey.py scale`, bead
  domain-pascal-r3yy; the other a zero-extended byte multiplied in
  place, RIP_$BROADCAST 0xE872F8 `mulu.w #92,D5`; other computed values
  refused, bead domain-pascal-q322), as `x + c` does for 9..127
  (above): a word product of a signed word is `muls.w` (module-local
  0xE4519C at 0xE458CC..0xE458D2 `moveq #44,D3; muls.w D2,D3`; one
  site), one of an unsigned word is refused.  `muls.w #c` occurs where the product is a
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
  - a non-M$ARITH word result kept in a register is widened the same
    way: of 71 `jsr ML_$SPIN_LOCK` sites, 68 store `move.w D0,(d,A6)`
    and the 3 that keep the token in a register zero-extend it, `clr.l
    Dn; move.w D0,Dn` (LOG_$ADD 0xE1768A D5; UID_$GEN 0xE1A03E,
    0xE1A0B0 D3); every later use is a word (`move.w D5,-(SP)`
    0xE1775E; `move.w D3,-(SP)` 0xE1A07A).  ML_$SPIN_LOCK is `move.w
    SR,D0; ori.w #0x700,SR; rts`.  Reported reading: an unsigned word
    result assigned to a longword local (../source declares the token
    `integer32`; how a register-held *word* local would be loaded is
    not separated) (lead: ../source pascal-translation-notes finding
    20);
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
    D0,(A0)+`; Uncertain: either gives the low word; VTOC_$GET_UID
    0xE3925A `mulu.w D4,D6`, once cited here, is `moveq #100,D6` times a
    word, the scaling of an index, section 5.2).  **Uncertain:** which of two variables goes to the
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
    exceptions listed there).  Unsigned word `div` by 2^k: `lsr.w`, as
    `lsr.l` for longwords (no `divu.w #2^k` anywhere in compiled code,
    against 234 `lsr.w #k,Dn`; LOG_$READ's worker 0xE177D2..0xE177D8
    `move.w (A0),D0; lsr.w #1,D0` halves a `0..65535` word, C `>> 1`, our
    reconstruction `div 2`); implemented (bead domain-pascal-5aw.17).
    Refused: unsigned `mod` by 2^k (no site: mask or division?).
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
    relational operators, `min`, `max`) are unsigned when both operands
    are unsigned run-time values, or one is and the other a constant in
    0..2147483647.  The image's unsigned entries (`M$DIU`, `M$OIU`,
    `M$MIU`) take zero-extended `0..65535` words, their sums, and
    unsigned longwords (0xE1C2B4; AST_$GET_SEG_MAP 0xE06E16 `M$DIU$LLL`
    of two run-time longwords; CAL_$DECODE_TIME 0xE68794); `dpc` gives
    `*` the same rule (Uncertain, above).  A non-negative subrange stored
    in 4 bytes (`0..2147483647`) is unsigned (section 5, "Signedness";
    bead domain-pascal-5j9), so `M$DIU$LLL`, `M$OIU$LLL`, `M$MIU$LLL`,
    `M$DIU$LLW`, `M$OIU$WLW` and `lsr.l` are reached from such types
    (golden `tests/golden/expr/ulong`, exec test `ulong`); ref 4.3.3's
    type transfers pick the sign (`u_type(s32) DIV u32` unsigned, `s32
    DIV integer32(u32)` signed).
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
  between them is not known; bead domain-pascal-9js).  TPAD_$DATA has
  4 widened sites in one routine (0xE69448..0xE6945A `move.w
  (0x170,A5),D5; ext.l D5; addi.l #50,D5; move.w (-0x1e,A2),D2; ext.l
  D2; cmp.l D2,D5`; 0xE69474..0xE69482, 0xE694B8..0xE694C4,
  0xE694DE..0xE694EC with `subi.l #50`), while the same sum stored is
  a word (0xE6945E `moveq #50,D2; add.w (0x170,A5),D2`) (lead:
  ../source pascal-translation-notes finding 39).  The same routine
  forms `v - e` both as `e` negated plus `v` (0xE692E8 `muls.w
  (-0x24,A2),D5; neg.w D5; add.w (0x178,A5),D5`, which ../source reads
  as a unary minus, `-dy * y_scale + accum_y`) and as `v` loaded minus
  `e` (0xE693AE `move.w (-0x24,A2),D3; sub.w D2,D3`) (Verified at the
  sites; finding 39).  An integer32
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
- **Narrowing and widening (bead domain-pascal-8td; the forms Verified
  at the sites given, the choice between them Uncertain where noted;
  implemented).**  An integer value stored into an object, passed by
  value (ref 5.2, Table 5-1) or returned as a function result of another
  size.  (An expression of another size passed by address, in a frame
  temporary, stays refused: its conversion is not catalogued.)
  *Reference:* no section states the rule for integers of different
  sizes; 4.3.1 (p. 4-5) expands operands, messages 79 and 83 (pp. 9-15,
  9-16) require compatible types, 5.1 (p. 5-3) says "if the second
  parameter is declared as integer16, the second argument must be an
  integer16 value", and the reference checks no ranges: "Currently,
  Domain Pascal does not support subrange checking" (3.8, p. 3-14);
  "Domain Pascal does not detect an overflow or underflow on integer
  arithmetic" (Appendix D, ISO 6.7.2.2, p. D-4); "You may make an
  assignment from an integer expression that includes a value outside
  one of the variables' declared subranges. Also, you may pass an integer
  argument that is outside the corresponding parameter's declared
  subrange" (Appendix D, ISO 6.4.6, p. D-3), which does not say whether
  `integer16` counts as a subrange of `integer32` nor what value the
  target holds (reference-clarifications 41).  `dpc-sema` already
  accepted integers of any size for one another (`ordinal_compatible`);
  the image gives the code.  Survey: `tools/arithsurvey.py narrow`.
  - *Widening* (by the value's sign, as for operands): a word or byte
    pushed for a longword or word value parameter is extended into a new
    register and pushed, `clr.l Dn; move.w x,Dn; move.l Dn,-(SP)` for an
    unsigned word (47 pushes; APP_$RECEIVE 0xE008E6..0xE008EC, a
    `0..65535` field for ROUTE_$FIND_PORTP's integer32 `val_param`),
    `ext.l Dn; move.l Dn,-(SP)` for a signed one (31; 0xE0B56A),
    `clr.w Dn; move.b x,Dn; move.w Dn,-(SP)` for a `[byte] 0..255` (20;
    SOCK_$CLOSE 0xE16008..0xE1600E).  No `andi.l #0xffff` or `ext.w`
    before a push.  **Uncertain** (bead domain-pascal-sjzf): the pad is
    scheduled after the `clr.l` at NET's module-local 0xE5A128 at
    0xE5A13C (`clr.l D0; subq.l #2,SP; move.w D2,D0; move.l D0,-(SP)`);
    `dpc` emits the pad first (R11).
  - *Narrowing a variable* (a variable, a parameter, a field of one):
    its low bytes read in place.  In memory, at the higher address
    (big-endian): APP_$DEMUX 0xE00AF6 `move.w (0x1a,A0),(-0x14,A6)` (a
    longword field at 0x18 of a record passed by address); MMAP_$FREE_
    PAGES 0xE0CF2C `move.w (-0x12,A6),(-0x1ffa,A4)` (a longword local at
    -0x14, `mmap/free_pages.c` `(word)next`); AREA_$COPY 0xE092DE
    `move.w (-10,A6),-(SP)` (a longword stored at -12 by 0xE090BC);
    PACCT_$LOG 0xE5AB26 `movea.l (0x10,A6),A0; move.b (1,A0),(-0x18e,A6)`
    (a word `in` parameter into a byte).  `lowword` finds 67 frame slots
    read as a longword at d and a word at d+2 (some are records of two
    words) and 27 word/byte pairs.  In a register, the register at the
    narrow width: LOG_$ADD 0xE1775E `move.w D5,-(SP)` (a longword held in
    D5); SOCK_$ALLOCATE 0xE15ECE `move.b D4,(0x1b,A2); move.b
    D3,(0x1a,A2)` (word parameters held in D3/D4); NETLOG_$LOG_IT
    0xE71BCE `move.b D2,(-0x1a,A0)`; module-local 0xE0A328 at 0xE0A35A.
    **Refused** (`dpc-ir`, `assign_convert`; bead domain-pascal-vusy):
    an array element, a
    pointer's target and a `with` record: AST_$TOUCH 0xE0350E `move.l
    (A1),D5; move.w D5,-(SP)` and AST_$TOUCH_AREA 0xE039FC load an
    element's longword through a by-address pointer whole (`ppn_array[0]`
    in the C), 0xE03974 another (`*ppn_out++`); which source forms give
    which code is not known.  A volatile object (an access of another
    size) and a longword narrowed to a byte (not catalogued) are refused
    too.
  - *Narrowing a computed value:* computed at the narrow width when every
    part allows it (above, `dpc-ir` `narrow`); otherwise computed at its
    own width and the low bytes used (`narrowed` survey, word stores and
    pushes of a register a longword instruction formed: `lsr.l` 29,
    `asr.l` 14, `sub.l` 10, `addq.l` 5, ...): DBUF_$GET_BLOCK
    0xE3A5F2..0xE3A5F6 `move.l D3,D1; lsr.l #5,D1; move.w D1,-(SP)`;
    DBUF_$INIT 0xE3ABE8..0xE3ABF4 `lsr.l #8; lsr.l #2; lsl.l #4; move.w
    D0,(0x91a,A5)` (the product of a `rshft` at the longword width too);
    MAC_OS_$SEND 0xE0B650..0xE0B658 `move.w (-62,A6),D0; ext.l D0; add.l
    (A1),D0; move.w D0,(-62,A6)` (a word plus a longword in memory);
    PARITY_$CHK 0xE0B034..0xE0B04A (`ext.l D2; ...; add.l D1,D2`, a word
    plus a `rshft`); NET_$OPEN 0xE5A1BE..0xE5A1C6 `move.l A0,D0; sub.l
    #abs,D0; move.w D0,-(SP)`; DIR's module-local at 0xE4BE82..0xE4BE88 `move.l A2,D0;
    sub.l A1,D0; addq.l #1,D0; move.w D0,(-298,A6)`; 0xE4F25E..0xE4F268
    `move.l (10,A0),D1; andi.l #1023,D1; move.w D1,-(SP)`.  Except `x &
    c` with `x` a longword held in a data register and `c` a `moveq`
    constant: a word `and` on the register's low word, the constant
    first (`heldop` survey, `and.w` from a longword register 3:
    0xE0630E..0xE06312 `moveq #31,D3; and.w D4,D3`, 0xE064CC..0xE064D0,
    RIP_$NET_LOOKUP 0xE154F6..0xE154FC; and DBUF_$GET_BLOCK 0xE3A5EC
    `moveq #31,D0; and.w D3,D0`, D3 loaded at the entry).  **Uncertain**
    (bead domain-pascal-lpqr): the other operators on a held longword,
    `dpc` computes them at the longword width; 0xE4F252..0xE4F256
    `move.l A2,D0; move.l A4,D1; sub.w D1,D0; move.w D0,-(SP)`
    subtracts two held pointers at the word width, against the DIR site
    above.  A call in the value is refused (no site catalogued).
  - Implemented: `dpc-ir` `assign_convert` (stores, value arguments,
    function results), the selector's `convert` (`isel.rs`: `displace`,
    `narrows_in_place`, `narrow_held_and`), `push`/`store` from memory.
    The selector's in-place read also serves `chr(x)`, the other
    narrowing `Expr::Convert`: a word variable or field is read as its
    low byte in place, a held register at the byte width (**Uncertain**,
    bead domain-pascal-4tu1: no `chr` site of a variable is catalogued; before bead
    domain-pascal-8td `dpc` loaded the word into a register first); a
    volatile object, an element with a computed index and a longword are
    still read whole.  Tests: `dpc-ir` `narrowing_store`; golden
    `expr/narrow`; exec test `narrow`.
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

- **The type (declaration: Apollo insert file; layout Verified for
  `fail` and `code`).**  Ref 9.1.1-9.1.2 (pp. 9-1..9-3) says
  `status_$t` is declared in `/sys/ins/base.ins.pas`.  The SR10.2
  customer copy of that file is now available (section 0.1:
  `sys/ins/base.ins.pas`, header "BASE.INS.PAS, /sys/ins, ceb-jrw,
  01/02/89"); it declares

  ```
  status_$t = PACKED RECORD CASE integer OF
      1:  (fail:  boolean;            { true if module couldn't handle error }
           subsys: 0..127;            { subsystem code }
           modc: 0..255;              { module code }
           code: integer);            { module specific error }
      2:  (all: integer32);           { used for testing for specific value }
      END;
  ...
  status_$ok = 0;                     { returned if called proc successful }
  ```

  (its history: "04/28/83 jrw removed async bit from status_$t").  With
  the packing rules of ref 3.10.7 and Table 3-3 (p. 3-35: first field
  on a word boundary; fields "left to right within bytes and then by
  increasing byte address"; boolean 1 bit, subranges the bits of their
  extreme values, integer 16 bits word aligned) the record is 4 bytes:

  | field | bits of `all` | byte offset | evidence |
  |---|---|---|---|
  | `fail` | 31 | 0, bit 7 | **Verified:** PROC2_$RESUME 0xE41444 `bset #7,(-0x4,A6)` on its local status; PROC2_$FORK 0xE72CC8 `bset #7,(A1)` |
  | `subsys` | 30..24 | 0, bits 6..0 | packing rule; stcode db: the codes with a zero top byte are headed "OS / ..." |
  | `modc` | 23..16 | 1 | packing rule; stcode db: (b000e) is under "OS / terminal manager", (350005) under "OS / terminal handler" |
  | `code` | 15..0 | 2, a word | **Verified:** PROC2_$RESUME 0xE4142A `tst.w (-0x2,A6)`, the low word of the status at -4; PROC2_$FORK 0xE72CC2 `tst.w (0x2,A1)` then 0xE72CC8 `bset #7,(A1)` (i.e. `if status.code <> 0 then status.fail := true`) |
  | `all` | 31..0 | 0, a longword | the stores and tests below |

  `../source` types it `int32_t` (`base/base.h`); bit 31 is the `fail`
  bit (`proc2/resume.c`, `proc2/fork.c`).  Ref 9.1.1 already said `all`
  is the integer32 view (`OPEN(f, pathname1, 'NEW', err_stat.all)`; "Improper
  status argument", p. 9-21) and that `code` "contains a number that
  corresponds to a particular error" (p. 9-3).  `status_$ok` is 0 (9.1,
  p. 9-1; "Open", pp. 4-132..4-133).  `dpc` takes the packed
  declaration as it is (section 5.3; bead domain-pascal-xlm): `all` and
  `code` are plain longword and word accesses, `fail` a bit field
  (`tests/golden/packed/fields.pas`).  Older conformance sources declare
  `status_$t = record all: integer32 end` (only `all`, at offset 0, 4
  bytes: the size, offset and alignment of the packed record's `all`, so
  the code is the same); `tests/conformance/name_convert_acl_status.pas`
  uses a plain integer32.  Status codes in sources
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
- **In a data register** when A2-A4 are taken (registers Verified at
  the sites; "when A2-A4 are taken" as the cause Reported; lead:
  ../source pascal-translation-notes finding 14):
  TTY_$K_DRAIN_OUTPUT 0xE67ABC `move.l (0xc,A6),D3` (A2 the
  descriptor, A3 its copy, A4 PROC1_$AS_ID's address, A5 data), pushed
  as `move.l D3,-(SP)` (0xE67AC2), tested via `movea.l D3,A0; tst.l
  (A0)` (0xE67AD4), written via `movea.l D3,A1` (0xE67B80).  TTY_$K_PUT
  holds two by-address slots likewise, D4 and D5 (0xE67BC0, 0xE67BC4;
  `movea.l D4,A0` 0xE67C06, `movea.l D5,A0` 0xE67C6A), with A3/A4
  holding its count and status slots.  The shape of 12.2's pointer
  parameters in data registers (beads domain-pascal-0b1,
  domain-pascal-izc).
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
  - A status whose last use is an argument of a call on every path
    after a join, in a body with branches (12.4 R4(c); bead
    domain-pascal-cs2n).  Elsewhere such a status is held through the
    call (bead domain-pascal-9j1z), e.g. `status.all := status_$ok; if
    ... then begin status.all := ...; return end; p(..., status)`, where
    the `return` path avoids the call (likewise an error store in one
    part of an `if` and the call in the other: EC2_$ADVANCE 0xE42CB4
    A2, `pea (A2)` at 0xE42CD8).
  - A slot used once after a branch or a call whose lowest free
    register is dirty on some path, or A0 after a call with A1 written
    before (12.4 R4(b), "After the first branch or call").
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
  0xE15298 (section 12.4 R7).  `dpc` selects statements in order, and
  its scheduler (12.4 R10/R11, `crates/dpc-m68k/src/sched.rs`) treats
  every SR instruction as a barrier, so it does the same; the register
  allocator treats the SR instructions as using no register.

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
  - `in_range(x)` of an enumeration (ordinals 0..n-1, an unsigned word):
    `x <= n-1`, one unsigned compare with the last ordinal (**Reported,
    implemented**, bead domain-pascal-k1sv): TTY_$K_SET_FUNC_CHAR
    0xE67502 and TTY_$K_INQ_FUNC_CHAR 0xE67588 `cmpi.w #0x1f,D1; bhi`
    under ../source's reading `not in_range(func) or else ...`
    (tty/k_func.pas; tty_$func_t's last ordinal is 31).  The image
    cannot tell it from `func > <last enumerator>` (below); the reading
    is the one the routine's source has.  As a value it is the
    comparison's `s<cc>` (`sls`, section 3), not seen.  Of other types
    (a subrange, an integer, a char) it stays refused.  Tests:
    `tests/golden/control/in_range`, `tests/exec/bool_stores.pas`,
    `tests/exec/store_order.pas`.
  - `max`, `min`, and `in_range` of other run-time values:
    **refused**.  The only
    candidates in the image are the five `x := min(x, c)`-shaped sites of
    12.4 R8 (`move.w #c,Dn; cmp.w Dn,Dx; ble.s; move.w Dn,Dx`: 0xE19ED0,
    0xE2F08A, 0xE66008, 0xE66372, 0xE874D6; and unsigned with `moveq`,
    module-local 0xE1CA8C at 0xE1CAA0 `moveq #2,D1; cmp.w D1,D0; bls.s; move.w
    D1,D0`, lead: ../source pascal-translation-notes finding 32), which
    an `if x > c then x := c` would give too (0xE66372 even skips the store to memory when
    the value is in range).  No site that must be one of these built-ins
    is known.  A lead for `in_range`: TTY_$K_SET_FUNC_CHAR 0xE67502 and
    TTY_$K_INQ_FUNC_CHAR 0xE67588 `cmpi.w #31,D1; bhi` on an
    enumeration word whose last ordinal is 31, followed by the real
    bound `cmpi.w #17,D1; bls` (section 5; ../source `tty/k_func.pas`
    reads `not in_range(func) or else ...`); for an unsigned
    enumeration from 0 it would be one compare against the last
    ordinal.  Uncertain: a compare against the last enumerator gives the
    same code (lead: ../source pascal-translation-notes finding 9).
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
  `PROCEDURE$`/`DATA$` exist besides `.TEXT`/`.DATA` (see "PROCEDURE$
  and DATA$" below).  They may be another
  compiler's defaults or older defaults.  Section names are at most 16
  characters in the map (`PROC2_CREATE_PRO` beside `PROC2_CREATE_DAT`,
  `COLOR4_$WIRED_DA`): truncated, by the binder or the object format
  (Uncertain); `dpc` does not truncate.
- **Modules in several sections (Verified from the map:
  `tools/sectsurvey.py modules`; bead domain-pascal-5aw.10).**
  52 module names have code segments in more than one output section,
  and all but DISK_ at most one data segment per data section: e.g. PROC1_
  (`I E1471C` WIRED_PROC, `I E2F958` OS_INIT_PROC; `D E254E8`
  WIRED_DATA only), PROC2 (OS_INIT_PROC, .TEXT, PROC2_CREATE_PRO,
  PROC2_DELETE_PRO; `D` in OS_INIT_DATA, .DATA, PROC2_CREATE_DAT,
  PROC2_DELETE_DAT), PKT (WIRED_PROC, OS_INIT_PROC, PROCEDURE$), SMD
  (WIRED_PROC, OS_INIT_PROC, .TEXT, PROCEDURE$, PROC2_DELETE_PRO; four
  `D`), SCSI_ (`I E34F04` OS_INIT_PROC, `I E88800` SCSI_WIRED_PROC, `D
  E88804` SCSI_WIRED_DATA).  The code sections pair with data sections
  by name (WIRED_PROC/WIRED_DATA, OS_INIT_PROC/OS_INIT_DATA,
  .TEXT/.DATA, PROCEDURE$/DATA$, PROC2_CREATE_PRO/PROC2_CREATE_DAT,
  RING_PROC/RING_DATA, SCSI_WIRED_PROC/SCSI_WIRED_DATA).  A few names
  have two segments in one section: DISK_ (three in .TEXT, three `D` in
  .DATA), AUDIT and NETLOG (a second 4-byte .TEXT segment), i.e. several
  compilation units of one name (section 4, "Hypotheses refuted").
- **Calls between a module's sections (Verified: `tools/sectsurvey.py
  calls [-v]`, over `tools/a5survey.py`'s routines).**  No `bsr` crosses a segment: of the
  calls between routines of one module name, every one into another
  segment of it is `jsr abs.l` (191: 103 to map-named routines, 88 to
  unnamed ones; PROC1_$INIT 0xE2F9EC `jsr 0xE14B12`, WIRED_PROC;
  AST_$ACTIVATE_ASTE_CANNED 0xE2F350 `jsr 0xE0250C`; TTY_$K_RESET
  0xE6730E `jsr 0xE1AED0`), and every `bsr` stays in its segment (1879).
  The unnamed callees are module-local routines reached by absolute
  address.  23 more `jsr abs.l` stay inside one segment, all in DIR
  (0xE535FE `jsr 0xE4B340`, 0x82BE bytes back): the segment is 0x8FD0
  bytes, beyond `bsr.w`'s reach (not implemented: no unit is that
  large).
- **Data per section (Verified at the sites and by survey; reload rule
  across branch targets Uncertain; implemented for straight-line
  routines).**  A routine's A5 is its
  module's block in the data section paired with its code section
  (section 4, "What A5 points at"); an unnamed routine called from
  another section loads its own (0xE14B12 `lea 0xE254E8,A5`, PROC1_'s
  WIRED_DATA block, although its caller PROC1_$INIT is in OS_INIT_PROC,
  where PROC1_ has no block; 0xE0250C `lea 0xE1DC80,A5`, called from
  AST_$ACTIVATE_ASTE_CANNED, whose A5 is 0xE35004).  A routine reaches
  its module's block of *another* data section by its absolute address
  in a register, `movea.l #block,An` then `(d,An)`: PROC1_$INIT 0xE2F95C
  `movea.l #0xE254E8,A0; move.l #..,(0xc40,A0)` (three more stores off
  A0), PROC2_$INIT 0xE3048E `movea.l #0xE7BE84,A1; move.w #2,(0x1e2,A1)`
  (PROC2's .DATA block, from OS_INIT_PROC), KBD_$INIT 0xE3336A (KBD's
  WIRED_DATA 0xE2DDE4, twice: 0xE3336A and 0xE3339E, after a call).
  The encoding is `movea.l #imm` (`207c`), not the `lea abs.l` of the
  `--a5-absolute` sites (section 4).  LOG_$INIT (OS_INIT_PROC) reaches
  LOG_'s WIRED_DATA block 0xE2B280 this way, `movea.l #0xE2B280,A0` at
  0xE300B2, 0xE30198, 0xE301B0, 0xE301DC and `movea.l #...,A1` at
  0xE30180 (A0 holding a pointer there).  It pushes the block's first
  variable by address as `move.l #0xE2B280,-(SP)` (`2f3c`; 0xE30054,
  0xE3007A, 0xE3010E, 0xE30148), where WIRED_PROC's LOG_$SHUTDN has
  `pea (A5)` (0xE175EE).  A defined global of the block is reached off
  the register too, LOG_$LOGFILE_PTR as `(0x14,A0)` (0xE301B6), while
  another module writes it by symbol (`clr.l 0xE2B294`, 0xE14104).
  - *Survey (Verified counts; `tools/sectsurvey.py otherdata [-v]`,
    bead domain-pascal-a2g).*  Every operand of compiled code (reachable
    instructions, a5survey's routines) naming a `D` segment of the
    routine's own module name, by whether that segment is in the data
    section paired with the routine's code section ("own") or not
    ("other"), and by form.  Other-section operands: `movea.l #block,An`
    205 (OS_INIT_PROC: 87 WIRED_DATA, 47 .DATA, 3 RING_DATA; PROCEDURE$
    18; PROC2_CREATE_PRO 28; PROC2_DELETE_PRO 17; WIRED_PROC 5, FLP_$INT
    and SMD_$COPY_FONT_TO_MD_HDM reaching .DATA); `move.l #block,-(SP)`
    12 (an argument by address, always the block's first variable: LOG_$INIT
    0xE30054 x4, RIP_$INIT 0xE2FD94, PCHIST_$INIT 0xE32398, SIO_$INIT
    0xE32E24, TERM_$INIT 0xE330B2, 0xE33152, PROC2_$INIT 0xE3070C, NAME_$INIT
    0xE31860, the PROC2 module-local 0xE74526); `move.l #block,Dn` 3
    (MAC_OS_$INIT 0xE2F518, SOCK_$INIT 0xE2FE02, TERM_$INIT 0xE32ED6, the
    block then kept in a data register); `lea block,A5` 8, which are the
    A5 cases above (PCHIST .TEXT, SMD's PROCEDURE$ module-local 0xE72690,
    the DATA$ 4-byte blocks).  **No absolute operand `(block+d).l`** and
    no `lea block,An` reaches another section's block.  Every `movea.l
    #block` loads the block's start (offset 0) except 2 in OS_INIT_PROC
    (+8); the variables are `(d,An)` off it (`pea (d,An)` 27 times right
    after the load), the first one `(An)` (EC2_$INIT_S 0xE309A8 `clr.l
    (A0)`).  (Own-section operands, for comparison: `lea block,A5` 859;
    the 3 `--a5-absolute` `lea abs,An` of section 4; 27 `movea.l #` and
    one `tst.b abs.l` in DISK_ that reach *another unit's* DISK_ block at
    nonzero offsets, the `extern` case.)
  - *When the register is loaded (same survey, in address order over
    the 70 routines).*  70 first loads; 122 loads after a call (or after
    the register was overwritten): the block's register never survives a
    call (DXM_$INIT 0xE2FF54/0xE2FF66 around `jsr EC_$INIT`; KBD_$INIT
    0xE3339E), and it is not held through a call whose arguments use it
    last either: XNS_IDP_$INIT 0xE3026E `movea.l #0xE2B314,A0` in one
    statement, `pea (0x520,A0)` for the next one's call, in A0 (contrast
    section 12.4 R4(c) for parameters).  340 uses of a register still
    holding the block, across statements without calls (SUMA_$INIT
    0xE33228: one load, four statements; PROC1_$INIT 0xE2F95C: one load,
    four stores).  Only 5 loads while a register holds the block with no
    call or branch target in between, none a statement boundary: FLP_$INT
    0xE19F82 (two registers in one statement, one stepped to an element),
    NAME_$FORK 0xE73EAC/0xE73EB2 (each array element address from its own
    load), RINGLOG_$CNTL 0xE72270/0xE7227E (`case` arms, jump-table
    targets).  Across branch targets the image does both: 8 loads again
    at a join (AST_$ACTIVATE_ASTE_CANNED 0xE2F266, NAME_$INIT 0xE317C0,
    NET_IO_$BOOT_DEVICE 0xE31C66, 0xE31C7A, ...), and 26 uses past one
    with the register kept (NETWORK_$READ_SERVICE 0xE71D9A after an `if`;
    loops whose block load precedes them, AREA_$INIT 0xE2F3FC, TPAD_$INIT
    0xE335A4).  Which decides is not known (Uncertain).
  - *Placement at entry.*  When the first statement starts with the
    block (a store, KBD_$INIT; a test, NETWORK_$READ_SERVICE and the
    RINGLOG_ module-local), its load precedes the parameter loads of the
    entry and the static link, as A5's does: KBD_$INIT 0xE3336A before
    `movea.l (8,A6),A2`; NETWORK_$READ_SERVICE 0xE71D80 (into A1) before
    `movea.l (8,A6),A0`; the RINGLOG_ module-local 0xE721D4 before `movea.l
    (A6),A2`.  No routine with entry loads has them first when its first
    statement starts with the block (FLP_$INT 0xE19F74 loads a pointer
    parameter used once, within the statement).
  - *What `dpc` emits (implemented, bead domain-pascal-a2g).*  A static
    variable of another data section than the routine's (top-level
    routine's for a nested one) is `dpc_ir::Addr::OtherStatic`; it does
    not count as a use of the data base (section 4).  The selector
    (`Selector::other_base`) loads the block's label `movea.l
    #.Ldata.<SECT>,An` into a virtual address register where the first
    operand needs it, keeps it for the following statements, forgets it
    at every call (`jsr`, `bsr`, M$ARITH), and never holds it through a
    call (`held_through_calls`); the variables are `(d,An)`, `(An)` at
    offset 0; by address, the block's first variable is `move.l
    #block,-(SP)` and the others `pea (d,An)`; `addr(x)` as a value is
    `lea (d,An),Am` (KBD_$INIT 0xE333A4).  A first statement storing a
    constant into the block loads it before the entry loads.  Refused:
    any branch or label in a routine that reaches another section's
    block (the reload rule above); `addr` of the block's first variable
    as a value (not seen); a first statement that reaches the block
    otherwise, in a routine with entry loads (the order is not seen); a
    block load before entry loads of which one lands in A0 or A1 (the
    one such site, NETWORK_$READ_SERVICE 0xE71D80, has `movea.l
    #0xE248FC,A1; movea.l (8,A6),A0`, the parameter in the lower
    register, which lowest-free allocation in load order does not give;
    KBD_$INIT and the RINGLOG_ module-local agree only because their
    parameter or static link is in A2); a block register live across a
    call within one statement (`l := b + g(x)`, the operand's base taken
    before the call: never seen, where the original loads it is not
    known);
    array elements of the block (the element address forms, NAME_$FORK
    above, are not modelled: the existing refusal of elements outside
    the unit's data, the frame or a held pointer); the `move.l
    #block,Dn` form (bead domain-pascal-hxwq).  Conformance: `tests/conformance/suma_init.pas`
    (SUMA_$INIT) and `dxm_init.pas` (DXM_$INIT), byte-identical; golden
    `sections/other_data_section` (the KBD_$INIT shapes); exec test
    `other_section_data`.
- **PROCEDURE$ and DATA$ (Verified at the sites; pairing Uncertain;
  lead: ../source pascal-translation-notes finding 33).**  KBD's
  PROCEDURE$ segment 0xE724C4 loads no A5 and reaches KBD's WIRED_DATA
  block absolutely (`movea.l #0xE2DDE4,A0`, `207c`, 0xE724FC); KBD has
  no DATA$ block.  DXM_WIRED_'s DXM_$ADD_SIGNAL_CALLBACK 0xE72184
  (PROCEDURE$) loads DATA$ (`lea 0xE85708,A5`, 0xE7218C).  But DATA$ is
  not PROCEDURE$'s partner only: MST_$FORK 0xE739F8 (PROC2_CREATE_PRO)
  loads MST_UNWIRED's DATA$ block (`lea 0xE85714`, 0xE73A00: the end
  of the 4-byte block 0xE85710, the "A5 = its end" case of section 4),
  SMD_$FREE_ASID 0xE75250 (PROC2_DELETE_PRO) SMD's (`lea 0xE85718`,
  the end of 0xE85714; neither routine addresses off A5),
  and SMD's PROCEDURE$ module-local at 0xE72690 loads SMD's
  *WIRED_DATA* block (`lea 0xE2E060,A5`, 0xE72696; used, `pea (A5)`
  0xE726B8; called by `jsr` from 0xE34EF0 and 0xE6FFA4).  So
  "PROCEDURE$/DATA$" is not a one-to-one pair.  Which data section a
  PROCEDURE$ routine's attribute named is not settled.
- **Overlay sections (Verified from the map; addressing at the sites).**
  Sections with no per-module segment, named like variables (ACL_$DATA
  0xE88834, FILE_$LOT_DATA, RINGLOG_$DATA, XPD_$DATA, PROC2_$DATA
  0xEA551C; NET_PORT_TABLE, REM_FILE_$DATA 0xE2E380, SMD_$WIRED_DATA),
  fit `var (name)` overlays (ref 3.14, p. 3-52: "just like a named common
  block in FORTRAN").  Code reaches them by absolute address:
  PROC2_$INIT 0xE30466 `movea.l #0xEA551C,A0; ... clr.w (0x3eb6,A0)`,
  REM_FILE 0xE61178 `move.l #0xE2E380,-(SP)` (an address argument).
- **What `dpc` emits for named sections (implemented, bead
  domain-pascal-5aw.10).**  The module heading's sections (ref 7.1.1,
  p. 7-2) and `[section(c,d)]` (ref 5.7.1, p. 5-24; a nested routine
  inherits its outer routine's) are kept per routine (`dpc_ir::Function`
  `code_section`, `data_section`).
  - *Output sections.*  The default code section stays `.text`; a named
    code section `S` of unit `U` is `.text.<U>.<S>`, upper-cased
    (`crates/dpc-m68k/src/emit.rs`, `code_section_name`).  Routines are
    grouped by code section, in body order within each; each section has
    its own constant pool and module-end padding.  The data area of a
    named data section `D` is the NOBITS `.bss.moddata.<U>.<D>`, labelled
    `.Ldata.<D>` (`.bss.moddata.<U>` and `.Ldata` for `.data`, as
    before; `.moddata.<U>[.<D>]` with contents when it has initial
    values, below).  Evidence for the scheme, `../source`'s link
    (`domain_os/tools/gen_layout_ld.py`, read only): a code input section
    is any `.text` or `.text.*` section, keyed by the earliest map symbol
    it defines (its name is not looked at), and a section whose map
    symbols straddle another placed section is an error, so one input
    section per unit and code section, like the image's one segment per
    module and section, is what that link can order.  The unit name
    keeps two units' `S` apart (the generator also qualifies duplicate
    names by object), and the `.` between the names keeps them apart
    from every `.text.<symbol>` there (`-ffunction-sections`, the
    hand-written `.section ".text.<symbol>"`).  The data blocks are not
    linked into `../source` as such (its blocks are keyed by C
    `MODULE_DATA_DEFINE` sites): for a link test the C object at the
    block's image address owns the bytes, and the unit, compiled with
    `dpc --data-base=SECTION=SYMBOL` (test scaffolding), allocates
    nothing and loads A5 from that symbol, with unchanged bytes (bead
    domain-pascal-3bp, `tests/link/README.md`, "Module data").
  - *Calls.*  A call of a routine of the unit in another code section is
    `jsr (SYM).l` (`dpc_ir::Callee::Section`), a local symbol for an
    `internal` routine (gas relocates it against the section); within a
    section it stays `bsr`.  Calls in statements and in expressions
    alike (bead domain-pascal-86o: the code section is applied after
    lowering, `set_call_sections`).
  - *A5.*  A routine's data base is its data section's block
    (`lea (.Ldata.<D>).l,A5`); a nested routine's is its top-level
    routine's.  An `internal` top-level routine that a routine of another
    data section calls loads its own like an exported routine (the
    0xE14B12 shape; whether the original's rule is "called from another
    data section" or "called from another code section" cannot be told
    apart, the pairs always coincide).
  - *Refused:* overlay sections
    (`var (name)` with a name that is no data section of the unit), a
    nested routine in another code section than its parent, section
    names with characters other than letters, digits, `_`, `$`, `.`.
  - *Fragments of several code sections* (test scaffolding, bead
    domain-pascal-xdh.6): `--fragment-origin=SECTION=0xADDR` per code
    section, each with its own pad and module-end padding; a routine of
    the module outside the unit, `--module-routine=SECTION:NAME=0xADDR`,
    is a `bsr` from its own section and stays `jsr abs.l` from another,
    as calls across sections are above.
  - *Options per section* (bead domain-pascal-lco): `--loop-align-off-
    from=R,...` and `--a5-always-from=R,...` take one routine per code
    section and switch only that section, from R on (each map segment
    has its own switch, sections 4 and 7).
  - Conformance: `tests/conformance/scsi.pas`, the whole SCSI_ module
    in two code sections and a data section, byte-identical, each
    section at its map segment (`tools/conform.py` links a code section
    named in `--section-addr` there and compares it whole);
    `proc1_get_loadav.pas` and `smd_busy_wait.pas` name their real
    sections (WIRED_PROC, WIRED_DATA) and are unchanged in bytes;
    `tty_sections.pas`, a fragment of TTY's WIRED_PROC and .TEXT, each
    section at its own origin.  Goldens `tests/golden/sections/
    named_sections`, `sections/section_fragments`.
- **Constants (Verified by survey; section 1.5):** constants passed by
  address sit in the code section, in cells right after the code of the
  routine that first uses them (`rts`, then the cells; 733 + 305 sites,
  none naming a later routine's cell), so a map routine's extent
  includes its cells and a module's last cells precede its end padding.
  `UID_$NIL` (0xE1737C) and the other constant UIDs are not such cells:
  they are 8-byte cells of a separate code segment, `I E1737C UID_LIST`,
  named by absolute address (`move.l #UID_$NIL,-(SP)`, section 1.5).
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
- **Defined and other variables interleave in a block (Verified from
  the map and the code; lead: ../source pascal-translation-notes
  finding 26).**  LOG_ 0xE2B280 (WIRED_DATA, size 0x1C) has the map
  names LOG_$LOGFILE_UID at +0 and LOG_$LOGFILE_PTR at +0x14.  Between
  and after them the code addresses unnamed variables: +8 (`(8,A5)`
  LOG_$ADD 0xE176EC), +0xC (`pea (12,A5)`, the spin lock), +0x10
  (0xE3019E), +0x18 (a byte, `st (24,A1)` 0xE30186).  So defined
  globals are not grouped apart from the unit's other variables.
  Which declaration order produces this is a language question (ref
  7.2; bead domain-pascal-ptd); ../source log/log.pas gets it by
  interleaving `define` and `var` parts (not re-checked here).
- **Byte-sized module variables (Verified by survey; implemented; bead
  domain-pascal-yvko).**  Every static object (unit-level, `define`, a
  routine's `static`) starts at an even offset of its block: a `char` or
  `boolean` variable takes the high byte of a word of its own, and two
  are never packed into one word, as in a frame (section 2, "Frame
  layout: alignment").  The reference does not say how variables are
  padded in a section: 3.14 p. 3-52 "Domain Pascal allocates variables
  defined in a var declaration part sequentially within the specified
  section"; 3.5.3 p. 3-11 ("stores freestanding Boolean objects on byte
  boundaries") and 3.6.4 p. 3-13 ("freestanding char variables are byte
  aligned") give booleans and chars byte alignment, which 3.10.5.1
  p. 3-23 defines as the addresses the compiler *can* use ("any byte
  boundary"), so an even one is within the rule.  The reference's
  sample `-map` listing (6.4.20, p. 6-25, PDF p. 406) agrees without
  settling it: section MICROS has X: INTEGER16 at 0, Q: CHAR at +2,
  R5: DOUBLE at +4 (Q's pad would also be R5's alignment; the listing
  is alphabetical, the declaration order not shown), and the frame two
  CHARs A, A2 at -8 and -6.  The image's modules use the default
  alignment environment; under `%natural_alignment` / `-natural`
  (6.4.22 p. 6-25) the rule is assumed, not verified.  Evidence (`tools/datasurvey.py`, which maps
  the `(d,A5)` and absolute operands of every compiled routine to the
  map's `D` segments):
  - `names`: the map names 343 addresses in compiled modules' blocks,
    one odd (LPR_$DATA_END 0xE854D7, a marker at the end of PEB_WIRED,
    no variable).  26 map-named variables there are accessed only as
    bytes with nothing at the next address; none is at an odd address,
    and the 10 followed by another map name have it 2 bytes on:
    NETWORK_$ACTIVITY_FLAG 0xE24C42, _$LOOPBACK_FLAG 0xE24C44,
    _$DO_CHKSUM 0xE24C46, _$USER_SOCK_OPEN 0xE24C48, _$REALLY_DISKLESS
    0xE24C4A (`tst.b`, `st`, `clr.b`; booleans), RIP_$STD_RECENT_CHANGES
    0xE26EDE, RINGLOG_$MBX_SOCK 0xE2C35E, _$WHO_SOCK, _$NIL_SOCK,
    DISK_$DIAG 0xE7ACCA.  A map name is a global symbol, which `define`
    gives a variable (ref 7.2, pp. 7-3..7-7) and a record's fields do
    not get, so these are separate byte variables one to a word.
  - `align`: lone bytes (accessed only as bytes, nothing else accessed
    in their word) in compiled modules' blocks: 56 at even offsets, 5 at
    odd ones (and, next above an indexed base, 4 even and 1 odd); the
    blocks are 4-aligned, so block and absolute parity agree.  The odd
    ones are inside other objects: SUMA's sample record at BITPAD
    0xE2DD88+0xF (../source suma/suma.pas `suma_$prev`, +0x4..+0x13),
    FLP_$SREGS (+0x70) at FLP_ 0xE7AEF4+0x75 (`btst` on the low bytes
    of its words +0x71, +0x73, +0x75, 0xE3E386..0xE3E460; +0x72 read
    as a word, 0xE3E3F0), the low byte of the word at FLP_+0x102
    (`move.b (259,A5)` 0xE3DCCC, a narrowing), bit 0 of the longword at
    DIR 0xE7DBF8+0x2044 (A5 = block+8: `btst`/`bset #0,(0x203f,A5)`
    0xE4B8DA/0xE4B8E6 on its low byte, `or.l D0,(0x203c,A5)` 0xE4B936,
    `and.l` 0xE4B9A2), the three flags SMD 0xE82B8C+0x173D..+0x173F
    (`st`/`clr.b`, 0xE6D838; inside a block that ../source models as one
    record, smd_globals_t, which the image cannot confirm; the map name
    SMD_REQUEST_QUEUE at +0x17F0 makes the block at least two variables,
    so such a record would end by +0x17F0), and MST_$DISKLESS_MSG's
    characters (MST_WIRED 0xE24304+0x4F).
  - `pairs`: 18 pairs of lone bytes in adjacent words in compiled
    modules' blocks (NETWORK +0x346..+0x350, RINGLOG_ +0x32..+0x3A,
    SIO_IO 0xE2DEB8 +0x54..+0x5C, DISK_ +0xAFA..+0xB00, PCHIST +0x124 /
    +0x126, RIP_WIRED +0xC86 / +0xC88, NETLOG +0x7E / +0x80).  The 11
    words of such blocks whose two bytes are both accessed only as bytes
    all lie inside one named object or a record of ../source: PEB_$INFO
    0xE24C78 (+0x1A..+0x21, the one map name of PEB_PARITY), SUMA's
    sample records (BITPAD +0x10..+0x13, +0x20..+0x23), SMD +0xDC/+0xDD
    and +0x173E/+0x173F, MST_$DISKLESS_MSG +0x50/+0x51.
  SIO_IO's WIRED_DATA 0xE2DEB8 shows it with chars: `70 00 60 00` at
  +0x48 (read `move.b (0x48,A5),(5,A0)` / `(0x4a,A5)`,
  SIO2681_$SET_BREAK 0xE1D148 / 0xE1D13E), `10 00 45 00 2A 00 3A 00 0A
  00` at +0x54..+0x5D; the two templates at +0x5E / +0x60 are one-char
  packed records, a word each by ref 3.10.7 p. 3-35 ("The minimum size
  of a packed record is 16 bits"), so ../source's types there are not
  the cause.  Not distinguishable in the image: whether a lone byte was
  declared `char`/`boolean` or as a one-field record of one (a record is
  2 bytes, 3.10.5.5 p. 3-26); either gives this layout.
  **Implemented** (`dpc-sema` `alloc_static`, `STATIC_MIN_ALIGN`): each
  static object at `round_up(offset, max(align, 2))`, in declaration
  order; objects longer than a byte already had word alignment from
  their types.  An `[aligned(0)]` object (3.15.6) is word aligned there
  too, as in a frame, so only a field can put one at an odd address.
  Tests `static_byte_variables_word_aligned` (`crates/dpc-sema/src/
  tests/checks.rs`), golden `data/byte_statics`, exec `byte_statics`;
  `tools/srcsurvey.py` turned SIO2681_$SET_BREAK, SIO6509_$XMIT,
  SIO6509_$SET_LINE and SIO6509_$INQ_LINE from MISMATCH (data only) to
  OK.  ../source flp/flp.pas (its last two variables `flp_initialized:
  boolean` at +0x138, `flp_pad_139: array[1..3] of char` at +0x139,
  "the block's rounding to 4") cannot be laid out that way: dpc puts
  the pad at +0x13A, so the block is 0x13D, emitted as 0x140 against
  the map's `FLP_ size = 13C`; without the pad it is 0x139, rounded to
  0x13C, the image's 3 zero bytes (a ../source change, for the owner).
- **Objects of 8 bytes or more in a block (lead, not settled; ../source
  pascal-translation-notes finding 29; bead domain-pascal-1edl).**
  KBD's block (`D E2DDE4 KBD size = D4`) has TERM_$TPAD_BUFFER at +0x58
  (0xE2DE3C, map) and, as ../source's translation lays it out, its
  other 8+-byte objects at multiples of 8 from the block start, with 4
  unaddressed bytes at +0xBC; but FIM_'s block (`D E2126C`, FIM_IN_FIM
  at +0, FIM_$USER_FIM_ADDR at +0x3C, 0xE212A8, map) has an 8+-byte
  object at 4 mod 8 from its start.  Both blocks start at 4 mod 8, so
  8-alignment neither from the block nor from address 0 fits both.  The
  frame analogue is in section 2 ("Objects of 8 bytes or more").
  Uncertain; the object sizes inside the blocks are ../source's, not
  checked here.
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
  `--section-addr .bss.moddata.<UNIT>=0x<D segment>`.  Named data
  sections: above.  Not supported yet: overlay sections (`var (name)`).
- **Initial values (Verified at the sites below; implemented, bead
  domain-pascal-63t).**  The image is the kernel file as loaded, so a
  module's `D` segment holds its static data's initial values (ref
  3.2.2 p. 3-4, 3.5.1, 3.6.2, 3.9.2 p. 3-15, 3.10.4 pp. 3-21..3-22,
  3.11.1 pp. 3-38..3-43, 3.13.4 p. 3-51; 7.2.2 for `define`), and
  initialised and zero data share one segment per module and data
  section: TTY's WIRED_DATA 0xE2DDB4 is a 32-byte `set of char` then two
  longword sets, all initialised; FIM_ 0xE2126C is 0x124 zero bytes
  (FIM_$USER_FIM_ADDR's table among them) and then a 16-byte initialised
  table (+0x124); TTY's OS_INIT_DATA 0xE351B0 ends in `00 00` after its
  last datum (+0x3A, the rounding to 4).
  - *What `dpc` emits.*  `dpc-sema` flattens an initialiser into stores
    (`InitStore`: offset, type, constant, and the bits of a packed
    record's bit field); `dpc-ir` encodes each as an
    integer of its type's size (a boolean 0xFF/0, a `char` one byte, an
    enumeration a word, `nil` 0) or as bytes whose order the language
    fixes (a set's bit string, member k in byte `size-1 - k/8`, bit `k
    mod 8`, ref 3.9.3 Figure 3-6; a string's characters), in
    `dpc_ir::DataArea::init`; the emitter writes integers big-endian.  A
    data section with at least one initial value is one block with
    contents, the section `.moddata.<UNIT>[.<SECT>]`, `"aw",@progbits`
    (`.byte` rows, `.space` for runs of 16 zeros or more and the tail),
    every byte not given a value zero: the rest of the variables, the
    padding between them, and the rounding to 4.  A block with no initial
    value stays NOBITS `.bss.moddata.<UNIT>[.<SECT>]`.  The prefixes are
    `../source`'s (`arch/m68k/arch.h`: `.moddata.<name>` is a
    `MODULE_DATA_DEFINE_INIT` block, "the image's initial contents").
  - *Addresses (layout Verified; source Uncertain; implemented, bead
    domain-pascal-bjz).*  Compiled modules' `D` segments hold routine
    entry addresses as plain big-endian longs: FLP_ 0xE7AEF4 has
    FLP_$SHUTDOWN 0xE3E228, FLP_$DINIT, FLP_$REVALIDATE, FLP_$DO_IO at
    +0x04..+0x10 (`../source` `flp/flp_data.c`: the jump table
    DISK_$REGISTER reads), WIN_ 0xE2B89C has WIN_$SPIN_DOWN 0xE19BC0 at
    +0x10, WIN_$DINIT at +0x18, WIN_$DO_IO, WIN_$ERROR_QUE,
    WIN_$GET_STATS at +0x20..+0x28; both modules' routines are compiled
    (FLP_$SHUTDOWN 0xE3E228 `link.w A6,#-12; movem.l; lea 0xE7AEF4,A5`).
    Whether those longs came from Pascal initialisers or from another
    source of the data is not known.  `addr(...)` as an initial value
    is the owner's ruling on 3.13.4 p. 3-51
    (docs/reference-clarifications.md); `dpc` accepts `addr` of a
    top-level non-`internal` routine (ref 3.13.3 p. 3-50) or of a
    unit-level, `static`, `define`d or `extern` variable, or a field or
    element of one at a constant offset, and emits `.long SYMBOL[+n]`
    (a routine, another unit's variable) or `.long .Ldata[.<SECT>]+n`
    (the unit's own variables, which may have no symbol), one
    `R_68K_32` relocation each (`dpc_ir::InitValue::Address`; golden
    `data/addr_init`, exec test `addr_init`).  A later initial value
    over part of an address is refused.  No conformance or link unit
    has such a section yet, so neither `tools/conform.py` nor
    `tools/linktest.py` has been exercised on one; `linktest.py` refuses
    to compare a block's initial bytes where the dpc object has a
    relocation (its bytes there are not final), as it already did for
    the C object's.
  - *Rules taken from the reference, not seen in the image:* components
    with no initialisation data are "not initialized" (ref 3.11.1 p.
    3-38) and are zero here, as the rest of a NOBITS block is; a string
    shorter than its `array of char` fills only its first elements (ref
    3.11.1.1: "elements 1 through 21"), not blank-padded as an
    assignment would be (p. 4-204), and one character fills element 1
    (ref 3.11.1.1 p. 3-39, `blank_line : STRING := chr(10)`); a later value for the same field
    overwrites an earlier one (ref 3.10.4, "field c gets reinitialized");
    a variant's field is initialised by name (ref 3.10.4, `lb := 'a'`;
    status_$t's `all`, TTY_$I_INIT's statics in `../source`).
  - *Packed records' bit fields (Verified at KBD; implemented, bead
    domain-pascal-c0j8).*  A packed record's field that is whole bytes
    holding its type's whole object (status_$t's `all`, a `char`) is
    initialised as in an unpacked record; any other is a bit field
    (`InitStore::bits`, `dpc_ir::InitValue::Bits`): the value's ordinal
    in the field's bits, most significant bit first, the record a
    big-endian bit string whose bit 0 is the most significant bit of
    its first byte (ref 3.10.7 p. 3-35, "Pascal allocates fields left to
    right within bytes and then by increasing byte address"; the layout
    is `types.rs` `layout_packed_fields`, codegen-patterns 5.3).  A
    later value replaces only its own bits.  Image: KBD's
    kbd_$state_table, WIRED_DATA 0xE2DDE4+0xC0, `FB 20 E8 31 DF 31 FE B4
    FF 10 00 10 00 42 00 53 00 6F 00 C0`: entries of a `char` and two
    `0..15` fields, the first (`action`) in bits 7..4 of the second byte
    (`2`, `3`, `3`, `0xB`, ...), the second (`next_state`) in bits 3..0;
    `../source` kbd/kbd.pas now compiles to those bytes (`make
    srcsurvey`: KBD_$CRASH_INIT, KBD_$PUT, KBD_$SET_KBD_MODE,
    KBD_$SET_KBD_TYPE OK, no longer CODE-OK).  *From the reference, not
    seen in the image:* a boolean is one bit, set for true (Table 3-3,
    "1 bit"; the run-time stores `bset`, PROC2_$RESUME 0xE41444); an
    enumeration its ordinal; a negative subrange value its two's
    complement in the field's bits (p. 3-37: `-100..40` takes 8 bits;
    the run-time stores of signed values mask them to the field,
    0xE31DF2..0xE31E00); fields across a byte boundary as the layout
    places them.  Refused: a set field that is not whole bytes (where a
    packed set's elements lie in its bits is not stated, Table 3-3 gives
    only the size, and no read of one is catalogued; bead
    domain-pascal-bk8k).  Golden
    `data/packed_init`, exec test `packed_init`.
  - *Not supported yet:* a bare (positional) value for or after a
    variant's field (ref 3.10.4 shows variants by name only); a packed
    record's set field that is not whole bytes; a `varying` string
    (length word, the trailing null of p. 4-205); reals; overlay
    sections.
  - *Harness.*  `tools/conform.py` compares a section with contents byte
    for byte with the image at its `--section-addr`, which must start a
    `D` segment of the map that holds the whole section (a unit may
    declare a prefix of the block).  `tools/linktest.py` (C owns the
    bytes in the link, `dpc --data-base`) compares the unit's declared
    initial bytes with the C owner's initial bytes (a NOBITS C object is
    zeros; a relocated C byte fails as not comparable).
  - Byte-identical in `make conform` (data compared whole):
    `tty_set_dfl_funcs.pas` (TTY .DATA 0xE8242C, 0x2C bytes: 18 words,
    two sets of a 32-value enumeration, one a routine's `static` between
    unit-level variables), `tty_wired_data.pas` (TTY WIRED_DATA
    0xE2DDB4, 0x28 bytes: a 256-member `set of char`, two longword
    sets; `.moddata.TTY.WIRED_DATA`), `fim_get_fim_addr.pas` (FIM_
    0xE2126C, 0x134 bytes, the frame size table at +0x124; also equal to
    `../source`'s FIM_$DATA in `make linktest`).

---

## 12. Register allocation and other "unoptimised" artefacts (Verified)

These are not optimisations.  They are part of the output we must
reproduce.

- **Parameters loaded into registers at entry.** Pointer and value
  parameters are loaded into callee-saved registers right after the
  `movem` (OSINFO_$GET_MMAP 0xE5C69C..0xE5C6AC: `move.l (8,A6),D3`, ...,
  pointers kept in **D** registers and moved to an A register only when
  used, `movea.l D6,A0`: 12.2, "Pointers in data registers").  In routines without calls, into scratch
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
| OSINFO_$GET_MMAP 0xE5C694 | `link #-0x18` | `movem.l D2-D7/A2-A4,-(SP)` (`48e7 3f38`) 0xE5C698 / `movem.l (-0x3c,A6),D2-D7/A2-A4` (`4cee 1cfc ffc4`) 0xE5C9F6 | Parameters at entry: `(8)`→D3, `(0xc)`→D4, `(0x10)`→A2, `(0x1c)`→D5, `(0x20)`→D6 (0xE5C69C..0xE5C6AC); pointers kept in D registers and moved to A0/A1 when used (`movea.l D6,A0`, `movea.l D3,A1`), a copy read again while its register holds it (A1 at 0xE5C8C8, 0xE5C952; 12.2 "Pointers in data registers": none of the four is used as data, rule Uncertain, bead domain-pascal-0b1). D2 is a temporary (0xE5C76C, 0xE5C77A, 0xE5C7BC). |
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
  (OSINFO_$GET_MMAP): when also used as an integer (compared with a
  number, stepped, masked; not when only stored; Verified,
  implemented), otherwise by a rule not settled (12.2, "Pointers in data
  registers").
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
  PROC1_$SET_TYPE 0xE152F2 D2, D3 around a `jsr`).  Which of them a
  by-address parameter held across a call takes beside a pointer local
  assigned that call's result: 12.4 R1b (the local first when the
  parameter dies in the assignment's block, KBD_$PUT 0xE1CEB4 A3).  A pointer may be
  kept in a D register and copied to an A register where it is
  dereferenced: when it is also used as an integer (EC2_$INIT 0xE42C64,
  `movea.l D0,A0` at 0xE42C70), and in other cases whose rule is not
  settled (ACL_$ADD_PROJ 0xE47EC0, `movea.l D4,A0` at 0xE47ED4, a status
  slot only dereferenced); "Pointers in data registers" below.
- *Functions differ:* PARITY_$CHK_IO 0xE0B180..0xE0B18A loads `(8)` into
  **D1** and `(0xc)` into D0: the result variable takes D0, and the
  parameter used later shares it (12.4 R5; implemented).
- *One-statement bodies:* PROC1_$TST_LOCK reads its parameter inside the
  expression, after the set's address (0xE148CE..0xE148D4, section 5.1);
  TIME_$Q_FLUSH_QUEUE 0xE16C84 loads its pointer right before the store.
  That is a *simple* statement (an assignment).  **A body of one `if`,
  `case` or loop statement loads the by-value parameters it reads at
  entry, like a body of several statements** (Verified at every site
  found; bead domain-pascal-s4a5):
  - the srcsurvey sites (`../source`'s Pascal): KBD_$RCV's nested
    0xE1CC64 `move.b (8,A6),D1` before its `case` chain (selector
    widened into D0); tty_$i_echo_char 0xE1B3DA `move.b (0xc,A6),D2`;
    tty_$i_set_raw_mode 0xE1BF82 `move.b (0xc,A6),D0; bpl.w` (read once,
    in the condition only); TTY_$I_ENABLE_CRASH_FUNC 0xE6729C..0xE672A4;
    tty_$i_set_funcs 0xE6721A..0xE6721E (a `for`); DBUF_$UPDATE_VOL
    0xE3AAB0 (a `for`; at `../source` HEAD 61469f98);
  - `tools/localsurvey.py onestmt` (C functions whose body is one `if`,
    `switch` or loop reading a non-pointer by-value parameter, test
    directories left out; at `../source` 61469f98): 10 parameters of 9
    routines, 9 `entry` (loaded before the first branch or call), none
    read in its slot; the tenth is in a nested routine whose reads are
    uplevel (pmap_$update_seg_map 0xE1359C, `none`).  Among them:
    PROC2_$SET_CLEANUP 0xE41580 `move.w (8,A6),D0` read only in the body,
    past a test of a global; NET_IO_$CLEANUP_NIL 0xE74ED2,
    NET_IO_$CLEANUP_USER, RIP_$HALT_ROUTER 0xE873A2, SIO_$I_INHIBIT_XMIT
    0xE1C9D8 read only in the condition, with a call after;
  - over all compiled routines (`regalloc.py`'s), only three have a
    parameter slot as the operand of their first test (`cmp`/`tst`/`btst
    (d,A6)` after the prologue and entry loads): the one-statement
    assignment function 0xE4BC26 (`cmpi.l #k,(8,A6)` five times, a
    simple statement); MEM_$PARITY_LOG 0xE0ADBE and FILE_$PRIV_CREATE
    0xE5D382, bodies of several statements (bead domain-pascal-wxlw).
  - *The test of the parameter loaded last is the load's flags:* every
    `move.<s> (d,A6),Dn` (d >= 8) made before a routine's first branch or
    call and followed by a test of Dn is followed by the branch itself,
    20 of 20, all `beq`/`bne`/`bmi`/`bpl` (0xE1BF82, 0xE672A0,
    AREA_$DELETE 0xE07C34, 0xE1C9D8; no `tst Dn` there).  `movea.l
    (d,A6),An` entry loads after that load leave its flags (MOVEA sets
    no condition codes) and the branch follows them: 10 of 10, none with
    a `tst Dn` (PROC1_$RESUME 0xE14774..0xE1477C `move.w (8,A6),D0;
    movea.l (0xa,A6),A0; beq.s`; 0xE02844, 0xE08CF8, 0xE71214; review
    of bead domain-pascal-s4a5, which first emitted `tst.w D0` there;
    exec test `entry_load_flags`).  A parameter
    loaded before another is tested with `tst Dn` (PROC1_$SET_TYPE
    0xE152F2..0xE152FA `move.w (8,A6),D2; move.w (0xa,A6),D3; tst.w D2`;
    12 such sites).  After a later load the image does both (a load from
    memory followed by its test: 302 branches on the load's flags, 20
    `tst`; not settled).
- *Entry loads are interleaved with other code* (an instruction
  scheduler; Verified that it happens, rule unknown; 12.4 R9–R12; `dpc`'s
  scheduler does not reproduce these, 12.4 R11 "Not explained"): `movea.l #imm,A0`
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
    3.15.3; implemented as the device rule of section 5.5, conformance
    unit `dma_init_m68450_channel.pas`).
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
- *A by-value parameter whose own slot is passed by address* (bead
  domain-pascal-pe9; survey of every compiled routine that both `pea`s a
  `(d,A6)` slot, d >= 8, and reads that slot into a D register: 22
  pairs; scratch script, not kept):
  - **to a value or `in` parameter (the default convention): held in its
    register across the call** (Verified at the sites; the callee copies
    a value parameter, ref 5.2 p. 5-4, and may not assign an `in` one,
    5.3.2 p. 5-7).  MST_$TOUCH 0xE0DD40: `(8)`→D3 at 0xE0DD4E, `pea
    (8,A6)` to PEB_$TOUCH at 0xE0DEFC, D3 pushed at 0xE0E07A;
    AST_$PMAP_ASSOC 0xE042B0 (`pea (0xe,A6)` to MMAP_$INSTALL_LIST at
    0xE0440A, D3 stored at 0xE04414); AST_$ASSOC_AREA 0xE04672 likewise
    (D2); 0xE4094C (`pea (0xa,A6)` to PROC1_$GET_INFO at 0xE40972, D3
    pushed at 0xE40990); 0xE3366C (to ASKNODE_$INTERNET_INFO, 0xE3368C;
    D2 at 0xE336A8); ROUTE_$SEND_USER_PORT 0xE87CBC (to
    NET_IO_$COPY_PACKET; D2 at 0xE87CD0).  AST_$PURIFY 0xE0567A, whose
    parameter `(0xc)` lives in D5 and is changed there (`andi.w
    #0x7fff,D5` at 0xE05AA8), first writes D5 back to the slot (0xE05B28
    `move.w D5,(0xc,A6)`) and then passes `pea (0xc,A6)` to
    REM_FILE_$PURIFY (which only reads it, `rem_file/purify.c`), and
    keeps using D5 after (0xE05B70).  This relies on the callee: a
    `dpc`-compiled callee copies a value parameter passed by address on
    entry and writes only the copy (section 1.2, bead
    domain-pascal-5aw.12), and `dpc-sema` refuses any write to an `in` parameter or a
    field or element of one: assignment (whole: "Assignment to IN
    argument", ref 5.3.2 p. 5-7; part, `with` included: not supported),
    passing it to a `var`, `out` or `in out` parameter, and `addr`
    (`Checker::in_param_part`).
  - **to a `var` parameter: not loaded at entry; read from the slot after
    the call** (one site): AREA_$TOUCH 0xE094FE loads `(8)`, `(0xe)`,
    `(0x16)` at entry (0xE0950C..0xE09514) but not `(0xc)`, passes `pea
    (0xc,A6)` to the module-local 0xE0939C (`area_$find_entry_by_uid`,
    whose `bste_ptr` is a `var` word it rewrites,
    `area/area_internal.h`) at 0xE095A6, and loads `move.w (0xc,A6),D4`
    right after that call (0xE095BA), holding D4 to the end.  No site was
    found that holds such a parameter in a register across a call that
    may write it.
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
way.  A body of one `if`, `case` or loop statement loads the by-value
parameters it reads at entry the same way ("One-statement bodies" above;
bead domain-pascal-s4a5), and a test of the parameter its last entry
load loaded, made right there, branches on that load's flags
(`Selector::tst`, `entry_end`; an ordered branch there is refused, not
seen).  Refused: a function of one such statement that assigns a
by-value parameter to its result (KBD_$RCV's nested 0xE1CC64 keeps `ch`
in D1 apart from the result in D0, `move.b D1b,D0b` at 0xE1CCBA, where
R5's result variable shares a register with a parameter it is assigned
from, 0xE15222; bead domain-pascal-0zo8).  Not reproduced: the image keeps
the register of a parameter whose last use is the `if`'s condition out
of the arms (TTY_$I_ENABLE_CRASH_FUNC 0xE672AA D2, not D1; bead
domain-pascal-d85r).  Golden `stores/one_stmt_entry_loads`; exec test
`one_stmt_params`; unit test
`one_statement_if_loads_value_parameters_at_entry`.  A slot used inside a loop body
counts as used more than once and is loaded at entry (bead
domain-pascal-5aw.15; the DISK_ module-local 0xE3CCD6 `movea.l
(0xc,A6),A0`, the status slot of its `while` loop's one store;
NAME_$VALIDATE's module-local 0xE4A004 loads the slots its loops use at
0xE4A018/0xE4A01C and the one used once outside them where it is
needed, 0xE4A03C; section 7.4).  A slot used once in a body of several
statements takes a register no earlier statement left a value in (12.4
R4(b), bead domain-pascal-5aw.2; IIC_$RECEIVE 0xE70AC0..0xE70AD0) when
no branch, call or label comes before the load (bead domain-pascal-5aw.12:
in routines that do branch or call, 496 of the 553 slot loads before the
first branch or call whose register is dead after it fit R4(b), the
others mostly argument lists, 12.4 R4(a), or values held through a call,
R4(c); ad-hoc script over `tools/nestsurvey.py`'s routines, not kept;
DIR_$GET_ENTRYU 0xE4D51E).  After one, it takes the lowest free
register only where that register is clean on every path, or where the
rule restarted at a call in straight-line code decides, and is refused
otherwise (12.4 R4(b), "After the first branch or call"; bead
domain-pascal-178); a simple statement right after a call in a body
that loaded nothing at entry takes the lowest free registers
(0xE00ECC..0xE00ED2: A0, D0 after the `jsr` at 0xE00EC2), its slot
loads checked the same way.
The entry copies of value parameters count as earlier statements
(section 1.2): after them a one-statement body's slot loads are fresh
too (SMD_$SET_TP_CURSOR 0xE6E98C, A2 after the copies' A0 and A1).  A parameter that is only pushed as a whole argument is not
"read" for this rule: it is pushed from the frame at each call (0xE00EBE,
0xE00EDE; AST_$SET_ATTRIBUTE 0xE053DA).  A parameter whose slot the body
passes to a `var`, `out` or `in out` parameter, or takes with `addr`,
anywhere in the routine is never loaded at entry and is read from its
slot at each use (bead domain-pascal-pe9: it used to be held in a
register that went stale across the call; `escaped_frame_objects`,
`dpc_ir::Arg::Address::writable`); passed to a value or `in` parameter
it is held as before (MST_$TOUCH above).  **Not reproduced:**
AREA_$TOUCH's load of the parameter into a register right after the
last such call (0xE095BA); and storing it (`g := x`) is refused like
any parameter not loaded at entry.  A one-statement body keeps its
own evaluation order (PROC1_$TST_LOCK, PROC1_$GET_LOCKS,
TIME_$Q_FLUSH_QUEUE stay byte-identical).  A function body of several
statements loads its parameters the same way and holds its result
variable in a register (section 3, "Result variable"; beads
domain-pascal-5aw.11, -8aj); the order in which the allocator colours
the result and the entry loads is R5 (12.4).  IO_$ALLOC 0xE1A9C2 and
IIC_$RECEIVE_CHECK 0xE70ADE (section 9) are such bodies, with the result
in D0.  A pointer parameter also used as an integer is held in a data
register ("Pointers in data registers" below).  Reported as not
supported yet: storing
a parameter that was not loaded at entry (one-statement bodies);
assigning to a parameter; in a body with branches, a parameter whose
last use is an argument of a call that lies on every path after a join,
or a pointer parameter held in a data register whose last use is one
(12.4 R4(c); beads domain-pascal-cs2n, -b3wz).  Otherwise a parameter
loaded at entry whose last use is an argument of a call is held through
the call, in a callee-saved register: in a body without branches
(PROC1_$SET_ASID 0xE148FE D2, a conformance unit) and with branches,
the call before the first branch, in an `if` part or after a join
inside one (12.4 R4(c), bead domain-pascal-9j1z: XPD_$GET_FP_INT
0xE5C560 A2, a conformance unit); one live across an earlier call is
callee-saved anyway, and held (CAL_$SHUTDOWN 0xE3E6E4 keeps `status`
in A2 across DBUF_$GET_BLOCK and passes it last with `pea (A2)` at
0xE3E72A; bead domain-pascal-500).

**Pointers in data registers** (bead domain-pascal-pze; `tools/regalloc.py
ptrdregs`, `ptrcopies`, over the routines `tools/regalloc.py` analyses).
A pointer held in a data register is dereferenced through a copy,
`movea.l Dn,An`.

- *Also used as data: held in a D register (Verified at the sites).*
  The parameters whose pointer value the body also uses as an integer
  (compared with a number, stepped or masked) and dereferences are in D
  registers: EC2_$INIT 0xE42C64 (`cmpi.l #0x3e8,D0` at 0xE42C68,
  `movea.l D0,A0` at 0xE42C70; its parameter is `OUT ec:
  ec2_$eventcount_t`, ec2.ins.pas, so the slot's address is the value:
  `addr(ec)`), EC2_$READ 0xE42C82, EC2_$ADVANCE 0xE42CB8 (also passed
  on, `move.l D0,-(SP)` at 0xE42CDA), PROC1_$FREE_STACK 0xE15126
  (compared, `subq.l #4`, stored at 0xE15132), DISK_$DIAG_IO 0xE6BCBC
  (`andi.l #0x3ff` on a copy), the NET_IO module-local 0xE0E238
  (0xE0E270 `tst.l D3`, see below): 6 parameters (`ptrdregs`: param,
  uses other than `push`; its seventh, 0xE42CF4 in the MST_UNWIRED
  module-local 0xE42CEC, is masked, `andi.w #0xfc00,D0`, and the masked
  value moved to A2, not a copy of the parameter).
- *Stored whole: not such a use (Verified at the sites).*  A pointer
  parameter that is dereferenced and stored to memory as it is stays in
  an address register and is stored from it: EC_$INIT 0xE15202..0xE1520C
  (`movea.l (8,A6),A0; clr.l (A0); move.l A0,(4,A0); move.l
  A0,(8,A0)`), the AST module-local 0xE00F7C (0xE00F86 `movea.l
  (8,A6),A1`, 0xE00FA0 `move.l A1,(0x3ec,A5)`), DISK_$ADD_QUE 0xE3C716
  (`(0x12,A6)` loaded into A1 at 0xE3C7D6, before a loop rather than at
  entry, stored at 0xE3C7EE and 0xE3C804).  No D-register parameter in
  `ptrdregs` has stores as its only other use.  For a by-address slot,
  whether `addr(p)` stored counts towards the two uses that load it at
  entry is split (the module-local 0xE32A60 at 0xE32A6C, 0xE32A80
  against MAC_OS_$PUT_INFO 0xE0C240, 0xE0C322; 5.2, "`addr` of a
  parameter"): refused where it decides (bead domain-pascal-m2w3);
  EC_$INIT's slot is loaded for its three dereferences.
- *Compared with `nil` (Uncertain, refused; bead domain-pascal-nys).*
  The AREA_ module-local
  0xE09E48 loads `(0xc,A6)` into A3 at entry, compares it `cmpa.w #0,A3`
  (0xE09EAC) and dereferences it (0xE09EC2 `(4,A3)`); the NET_IO
  module-local 0xE0E238 loads `(8,A6)` into D3 (0xE0E240), tests it
  `tst.l D3` (0xE0E270) and copies it (0xE0E29E `movea.l D3,A3`).  What
  decides between them is not known.  A comparison with another pointer
  is counted as data: 0xE01A7A..0xE01A7E `movea.l (8,A6),A1; cmpa.l
  (4,A0),A1` re-reads its slot rather than holding it, so it does not
  settle where a held pointer goes; nor does VTOC_$GET_UID 0xE392BC
  `cmpa.w #0,A2`, whose A2 is a local (`suba.l A2,A2` at 0xE392AA).
- *Copies.*  The image copies again only where no address register
  still holds a copy of the same value: of 981 copies whose data
  register is defined on every path, 969 are made where none holds one,
  6 into the register that holds it (RINGLOG_$LOGIT 0xE1A258) and 6
  into another (0xE09CA4; `ptrcopies`).  OSINFO_$GET_MMAP: the copy
  of D3 in A1 made at 0xE5C870 is read at 0xE5C8C8 and 0xE5C952, past
  two `if`s whose parts do not write A1, and made again at 0xE5C9A2
  after a loop that wrote A1, at 0xE5C798 after `case` arms that call,
  and at 0xE5C870 after the `jsr` at 0xE5C864; EC2_$INIT makes one copy
  for its two stores (0xE42C70..0xE42C74).  A copy takes its register
  as a slot loaded for one use does (12.4 R4(b): 0xE5C6B4 A1 after the
  copy in A0 at 0xE5C6B0, which is dead there).
- *Passed on to a `var` parameter:* `move.l Dn,-(SP)` (0xE42CDA;
  OSINFO_$GET_MMAP 0xE5C72C, 0xE5C860, 0xE5C862), where an address
  register would give `pea (An)` (0xE3E72A).
- *Dereference-only pointers (Uncertain; bead domain-pascal-0b1).*  157
  parameters only dereferenced (and 120 also passed on) live in D
  registers.  For 103 (91) no address register is free over their range
  (`ptrdregs` "Afull": OSINFO_$GET_MMAP's D3-D6, 0xE5C69C..0xE5C6AC,
  whose loops use A0, A1, A3, A4 with A2 holding `(0x10)`; PROC1_$GET_LIST
  0xE15370; module-local 0xE01C52 D5, A2-A4 busy), which fits register
  pressure; for 54 (29) one is (MAC_$DEMUX 0xE0BC5A D2 with A4 free;
  XNS_IDP_$REGISTER_ADDR 0xE19018 D2 with A3 and A4 free and no call;
  ACL_$ADD_PROJ is "Afull").  Which pointer gives up the address
  register is not settled.

**Implemented** (`crates/dpc-m68k/src/isel.rs`): in a body that loads
parameters at entry, a by-value pointer or a by-address slot the body
both dereferences and uses as data (a slot's data use is `addr(p)` in an
expression; passed as an argument it stays a base; a store of the whole
value, `g := p`, is not a data use, `ParamUse::Stored`; compared with
`nil` or 0 and otherwise only dereferenced is refused,
`ParamUse::ZeroCompare`) is loaded into a
data register (`move.l (d,A6),Dn`, `Selector::entry_loads`); each
dereference copies it (`Selector::copy_to_a`) into a register chosen as
for a slot load (12.4 R4(b), `fresh_slot_loads`, `check_late_slot_loads`);
after allocation a copy whose value an address register holds on every
path is dropped and that register read instead, renamed in where the
copy's uses lie in straight-line code that writes neither register
(`reuse_pointer_copies`); passed on by address it is pushed with `move.l
Dn,-(SP)` (a part of the object passed by address is refused, not
seen).  A dereference-only pointer keeps an address register (as before:
the allocator refuses when none is free).  Golden
`tests/golden/regalloc/ptr_data_regs`; exec tests `ptr_dregs`,
`ptr_dregs_flow`; unit tests
`pointer_used_as_data_is_held_in_a_data_register`,
`pointer_copies_are_reused_while_held`.  Conformance unit
`ec2_init.pas` (EC2_$INIT 0xE42C60, byte-identical; bead
domain-pascal-kwu): the unsigned `bls` (0xE42C6E) from
`linteger(addr(ec)) > 1000` (section 5, "Signedness"), but its `link
#-4` from a stand-in local, `unused_4`: what the 4 bytes held is not
settled (EC2_$READ `link #-12` with 8 used, EC2_$ADVANCE `link #-4`
with none: a local held in a register, 12.5, would fit all three).
OSINFO_$GET_MMAP: bead domain-pascal-2q2.

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
   **Done** (beads domain-pascal-g5q.5, -g5q.7, -g5q.8): the selector
   gives `clr.l D1; move.w D0,D1; clr.l D2; move.w (A0),D2; add.l D1,D2`,
   the sum in the operand that is not reused, and the scheduler (12.4
   R11) moves the second `clr.l` up, as in the image (golden
   `regalloc/cse`).
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
   into A2 first (0xE177CC) while A1 is free (section 12.2).  The reuse
   and `move.l D1,Dn; add.l (0x14,A5),Dn` are **done** (bead
   domain-pascal-g5q.7, 12.4 R7; golden `regalloc/cse` with an integer
   variable); the type transfers `integer32(ptr)` and `univ_ptr(...)` are
   lowered since bead domain-pascal-5aw.17 (no code; section 5.2,
   "Pointer arithmetic"); the register (D3, item 4/R6) and the order are
   not.
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
   `dpc` compiles `for` loops since bead domain-pascal-5aw.15 and the
   stepped pointers since bead domain-pascal-5aw.17 (section 7.4; the
   `lea`/`move.w (-2,A0),(-2,A1)`/`addq.l` part in golden
   `loops/stepped`), and `actual_len div 2` as `lsr.w #1` (section 8).
   It still refuses a run-time count in a loop without calls: kept in
   place 64 times, copied 45 times, and the worker's own shape (no
   index register, stepped pointers) 29 against 11 (section 7.4), with
   0xE4F010, a word copy like this one, copying: no rule.
7. The frame: `link #-0x10` with nothing in it addressed (section 2,
   untouched frames); the reconstruction declares stand-ins.

Items 2–5 are taken up by the model in 12.4.

**Status after bead domain-pascal-5aw.17** (`dpc` on
`tests/conformance/pending/log_read.pas`).  The first diagnostic is now
item 6's run-time count without a call (line 81).  With that and the
next refusal forced in a scratch build (count in place; the fresh load
of `buffer` allowed in a body with branches), the worker compiles to the
image's shape except for, in image order:

- 0xE177AE: `cmpi.l #0x400,D2` where the image holds the constant,
  `move.l #0x400,D3; ...; cmp.l D3,D2` (item 3; R8, one site);
- 0xE177C4: `move.w #0x400,D2` where the image has D3, and 0xE177D0
  the sum in D0 (`move.l D1,D0; add.l (0x14,A5),D0`) where the image has
  D3 (item 4; R6);
- 0xE177CC..0xE177DE: the reconstruction's `src` local is not held in a
  register (12.5: a dereferenced pointer local stays in the frame), so
  dpc stores the sum to `(-8,A6)` and reloads it after the guard where
  the image has `movea.l D3,A1` before it; `buffer`'s fresh load (12.4
  R4(b) after a branch: A0 is dirty there, so refused) comes after
  the guard in dpc (section 7.4, forming: the other sites load bases
  there; the worker hoists it to 0xE177CC), so the registers differ too
  (dpc: A0 source base, A1 `buffer`, A2/A0 pointers; image: A1, A2,
  A0/A1), and the interleaving of R11 cannot happen across the `bmi`;
- the frame: `link #-0x10` and `movem.l D2/A2` against the image's
  `D2-D3/A2` (follows from 0xE177AE/0xE177C4).

Each needs evidence beyond this routine (R8's constant, R6, a rule for
register-held pointer locals that are dereferenced, R4(b) with branches,
the hoisting of the base load); the unit stays pending.

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
  Nor did Chow-Hennessy, Chaitin/Briggs, usage-count or two-level orders
  (R1a below).
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
  - Inside one argument list: 0xE1359C (A0, A1, A3; section 12.1);
    MST_$MAP 0xE43886..0xE438AA, five by-address slots read once by the
    next push, in A0, A1, A2, A3, A4 (A2-A4 saved by the `movem` for
    this alone; also MST_$MAP_TOP 0xE438E6, MST_$MAP_GLOBAL 0xE43946).
    **Survey (bead domain-pascal-ae9; ad-hoc script over
    `tools/nestsurvey.py`'s routines, not kept):** of the argument
    lists with two or more `movea.l (d,A6),An` read once by the next
    push, 39 take a new register for each and 4 reuse one, each when
    the address registers run out (MST_$MAP_AT 0xE42F9C reuses A0 for
    the sixth after A0-A4; 0xE59AB6, 0xE59B50, 0xE59E4E with A2-A4 or
    A3 busy).  **Implemented for address registers** (`isel.rs`,
    `argument_list_loads`: the address values defined in a call's
    argument list are kept apart, `regs::allocate_apart`; more than
    A0-A4 is refused; golden `results/pointer_callers`, conformance
    `mst_maps.pas`).  Data values in an argument list are not covered
    (not surveyed).
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
  - **After the first branch or call (bead domain-pascal-178;
    `tools/regalloc.py slots`, kept, with tests).**  The loads: a `movea.l
    (d,A6),An` (d > 0) after the routine's first branch, call or label,
    whose slot no other instruction names and whose value lives in one
    block and across no call (what `dpc` loads where it is needed: slots
    used more than once are loaded at entry, 12.2); 350 in the routines
    of 0x30-0x150 bytes.  "Dirty" registers: those an earlier instruction
    left a value in, a value used up by `(An)+`/`-(An)` not counting; a
    reload of the same slot into the register holding it allowed.  Hits
    of each rule (the image's register), by the routine's shape:

    | Rule | branches and calls (274) | branches only (42) | calls only (34) | all (350) |
    |---|---|---|---|---|
    | `colU`: lowest free over the range (R1; hypothesis (a)) | 188 | 14 | 27 | 229 (65%) |
    | lowest free, not dirty on *any* path, a call making the registers it writes clean (hypothesis (b), the straight-line rule over the CFG) | 222 | 33 | 32 | 287 (82%) |
    | the same, dirty only when dirty on *every* path | 231 | 35 | 32 | 298 (85%) |
    | (b) with calls not cleaning (hypothesis (c) negated), any / every path | 170 / 195 | 33 / 35 | 29 / 29 | 232 / 259 |
    | (b), the set emptied at branch targets / at every block start (hypothesis (d)) | 217 / 215 | 23 / 22 | 32 / 32 | 272 / 269 |
    | the load's range extended back to its block start or the last call | 206 | 19 | 31 | 256 |
    | a stack of free registers (a dead value stays "cached"; `(An)+` returns its register among the unused ones; a call pushes A0 then A1) | 214 | 34 | 31 | 279 |

    Also tried, no better: only loads, `lea`s and constants (no index or
    data register) make a register dirty, 293-294; dirtiness carried in
    address order rather than over the CFG, 284-297; A0 left dirty by
    every call (as if holding a result), 12 of 35 after-call loads in
    routines with calls only.  So calls do clean the registers they write
    (c), labels do not reset the state (d), and no rule reaches 90%.  For
    slot loads named more than once ("reloads", 71 with branches and
    calls) taking the register of the slot's first earlier load first
    fits 65 against 45, but `dpc` loads such slots at entry.

    What is certain: **when the lowest free register is clean on every
    path, the original takes it** (164 loads, 162 hits; the misses are
    MAC_$DEMUX 0xE0BCD2, A1 after a copy used up A0 and A1, and 0xE6AA1C
    A3 with A1 free and clean); **when every dirty register was written
    since a call that precedes the load in its block, the straight-line
    rule restarted at that call decides** (the lowest free register not
    dirty: 10 loads, 10 hits; PROC2_$PGUID_TO_UPGID 0xE410B8 A1 after
    0xE410B2 A0, both after ML_$UNLOCK); **except A0 after a call when A1
    was written before**: after a call, with A0 the lowest free register
    and clean on every path, the original takes A0 in all 70 loads where
    nothing wrote A1 before (`slot_survey`'s `written_sets`), but where
    something did, A0 or A1 (by the final rule's count, 62
    loads: A0 40, A1 17, A2-A4 5), e.g. A1 in PROC2_$DEBUG 0xE41710,
    PROC2_$GET_EC 0xE40148, XPD_$SET_PTRACE_OPTS 0xE5B066, PROC2_$SIGNAL_
    PGROUP 0xE3F2B2, DISK_$AS_READ 0xE6B896, A0 in the like-shaped
    PROC2_$SET_NAME 0xE3EABE, PROC2_$SIGNAL 0xE3F080, DISK_$AS_WRITE
    0xE6B920.  Neither the order in which A0 and A1 were last used, nor
    their dirtiness before the call (any or every path), nor a free-stack
    order separates the two (Uncertain).  When the lowest free register
    is dirty on some path otherwise (114 loads), the lowest one that is
    not dirty on every path fits 86; among the rest, callee-saved registers while
    a scratch one is free and clean (0xE0D6DE A3, 0xE0D6E4 A4, 0xE69834
    A4, 0xE6CDA0 A2) and a dirty A0 reused (0xE19158 and 0xE161C6, after a
    `lea (0,A5,Dn.l),A0` that only formed the address of a load into a
    callee-saved register) (Uncertain).
  - **Past dead slot loads (bead domain-pascal-i24w; `tools/regalloc.py
    slotpass`, kept, with tests).**  The same loads, by what reaches each
    candidate register (free over the load's range, `colU`'s set) on
    every path to the load (`slot_states`, `reach_kind`): nothing
    ('entry'), a call, a block copy that used it up ('incdec'), a dead
    parameter slot load ('slot': `movea.l (d,A6),An`, d > 0, the entry
    loads included), or another value (`movea.l (-d,A6)`, constants,
    copies, `lea`s with and without an index, ...).  Over the whole image
    (474 such loads, no size bound), a register reached only by a slot
    load is passed over 145 times and taken 41; one reached by nothing
    is taken 65 times and passed over once (0xE6AA1C, above); one used
    up, 60 and 3; one a call wrote, 231 and 29; the other kinds go both
    ways (an indexed `lea`, 14 and 17; a constant, 4 and 16).  Candidate
    rules on the loads the rule above refuses for a dirty register (180
    in the whole image; 114 in 0x30-0x150 bytes), with the hits in
    alternate routines (address order):

    | Rule | whole image (180) | halves | 0x30-0x150 (114) | halves |
    |---|---|---|---|---|
    | `colU` | 53 | 32 / 21 | 27 | 14 / 13 |
    | lowest not dirty on any path, a call cleaning (`yc`) | 103 | 41 / 62 | 75 | 41 / 34 |
    | the same, dirty on every path (`lc`) | 122 | 54 / 68 | 86 | 45 / 41 |
    | `yc`, the set emptied at branch targets (`ycL`) | 100 | 49 / 51 | 60 | 30 / 30 |
    | entry loads not making a register dirty (`ycE` / `lcE`) | 82 / 99 | 35 / 47, 45 / 54 | 55 / 63 | 28 / 27, 30 / 33 |
    | the registers the load's own statement wrote free (`ycS` / `lcS`: the set at the statement's start, after the last call or store) | 90 / 109 | 37 / 53, 50 / 59 | 63 / 74 | 37 / 26, 41 / 33 |
    | a stack of free registers (`stack`) | 109 | 45 / 64 | 79 | 42 / 37 |
    | **past dead slot loads (`pass`), where it applies** | **38 of 38** | 13 / 25 | **34 of 34** | 19 / 15 |

    No rule fits the whole set (`lc`, the best, 68%); counterexamples to
    `lc` include a dirty A0 reused after an indexed `lea` (0xE161C6,
    0xE38452, 0xE3E1E0; MSG_$TEST_FOR_MESSAGE 0xE5A010 and SMD_$READ_
    CRSR_BITMAP 0xE6FD4E reuse A2 after `lea (0,A2,D1.w),A2`), and
    the loads where every free register is dirty, where the choice looks
    round-robin (MSG_$SEND_HW 0xE59B6E..0xE59B8C A2, A0, A1, A4, A2, A0,
    A3, the dead status, never taken; TPAD_$SET_UNIT_MODE 0xE69834 A4,
    0xE6983C A0, 0xE6987C A4; PCHIST_$UNIX_PROFIL_CNTL 0xE5CAB6..0xE5CAD2
    neither round-robin nor most-recent; bead domain-pascal-aerg).  A
    dirty register reused after an indexed `lea` or other values: bead
    domain-pascal-d0hd.  **What is certain (Verified): when every free
    register below the lowest one reached by nothing (or used up on every
    path) holds a dead parameter slot load on every path, at least one,
    the original takes that register** (the `pass` rule): 41 loads in the
    whole image, 41 hits (38 of them refused by the rule above, 3 taken
    by its since-call form too), by the register taken and those passed
    over: A1 past A0 (8: 0xE58C28, 0xE5971C, 0xE70A2E, 0xE763C4, ...), A2
    past A0, A1, A0 and A1, A3 past A1 (0xE699B0, PBU_$ADVANCE_EC_INT
    0xE88432), A3 past A2 (TTY_$K_INQ_INPUT_BREAK_MODE 0xE678E8, TTY_$K_
    INQ_PGROUP 0xE6796E: the status loaded at entry into A2, dead after
    its test; SMD_$READ_CRSR_BITMAP 0xE6FD54), A4 past A0-A3 (0xE4410C,
    MSG_$SEND_HW 0xE59B64), ...; used up: XNS_IDP_$DEMUX 0xE18C26 A1 past
    A0 after an `(A1)+` copy, 0xE18C2C A4, 0xE440FC A3, 0xE6041E A2,
    0xE32B08 A3.  Not settled: past a dead slot load to a register a
    call wrote (A1 past A0: taken 24 times, VTOCE_$LOOKUP_FM 0xE39A9C,
    MST_$CHANGE_RIGHTS 0xE435DC, SMD_$INVERT_DISP 0xE703C6, 0xE703D2,
    0xE706E2 reuse A0; TTY_$K_SET_FLAG 0xE67454 takes A1; bead
    domain-pascal-quio); a lower
    register reached by a slot load on some paths only (A0 'call+slot'
    taken 9 times, passed over once, VFMT_$MAIN 0xE6AC88); past
    registers holding other values.  The `pass` rule holds for slots
    no other instruction names (the population above), not for slots
    named elsewhere too (review of bead domain-pascal-i24w; ad-hoc
    script, own data flow over the whole image, not kept: the same 41
    single slots in 35 routines of 24 map segments, 41 hits; 35 loads
    of the shape whose slot is named elsewhere): past a dead load of
    the same slot the original reuses that register 8 times of 8 (the
    AREA_ module-local 0xE08816 at 0xE0887C, 0xE08894, 0xE0889E,
    0xE088C6, 0xE08A00 A0; VTOCE_$TRUNCATE 0xE39E78, 0xE39E86 A0;
    IO_$ACQUIRE 0xE1A6AC A1); past dead loads of other slots it takes
    the `pass` register 22 times and the lowest one 5 (VTOCE_$TRUNCATE
    0xE39E90; the OLD_DIR module-local 0xE55220 at 0xE55308, 0xE55314;
    XPD_$PUT_REGISTERS 0xE5C3B6;
    FILE_$READ_LOCK_ENTRYI 0xE60BC0 A0) (Uncertain).  Copies
    `movea.l Dn,An` of the same shape (same script): 9, 9 hits (0xE09A94
    A1 past A0, 0xE16BC2 A3 past A2, 0xE769AA A3 past A0-A2).
  - **Implemented** (`crates/dpc-m68k/src/isel.rs`, `fresh_slot_loads`,
    `pass_dead_slot_loads`, `check_late_slot_loads`): before the body's first branch, call or
    label, the allocator keeps each such slot apart from the address
    values of earlier statements that no `(An)+` used up
    (`regs::allocate_apart`; bead domain-pascal-5aw.12: 496 of 553 such
    loads in routines with branches or calls fit it, 12.2; the
    value-parameter copies, 133 of 135, section 1.2).  After it, the
    slot takes the lowest free register, kept apart only from the
    address values of earlier statements since a call that the load
    follows in straight-line code; after allocation the body is refused
    unless that register is clean on every path to the load (a data flow
    over the body: written by a non-call instruction, made clean by a
    call that writes it and by `(An)+`/`-(An)`), unless a lower dirty
    register was passed over while a register dirty from before that
    call remains, and unless it is A0 after a call with A1 written
    before.  A register reached only by dead slot loads is passed over
    (bead domain-pascal-i24w, `pass_dead_slot_loads`: the slot is kept
    apart from those loads and the body coloured again), and the load is
    then accepted only in the `pass` rule's shape (every free register
    below it reached only by a dead slot load, itself by nothing or used
    up on every path); a load so moved that ends elsewhere (a register a
    call wrote, another value, no register left) is refused.  Only a
    slot that no other instruction of the body names, or a copy
    `movea.l Dn,An`, is so passed (`passes_dead_slot_loads`); a slot read
    from the frame at each use (an escaped parameter) stays refused (the
    original reuses the register of its own dead load, above).  So it
    takes the 208 loads of the survey above (0x30-0x150 bytes) that the
    three certain rules cover (206 hits; `tools/regalloc.py slots`'s
    `dpc` column) and refuses the other 142 (176 before bead
    domain-pascal-i24w).  This covers a simple statement right after a
    call too (0xE00ECC's lowest-free shape).
    The check runs after the other register checks, so a pointer local
    that `dpc` puts in A1 against the image's D0 (TTY_$K_*, bead
    domain-pascal-500) is reported as that.  The slot loads of block
    copies (`block_address`) still take the lowest free register
    unchecked (ACL_ 0xE47944 suggests the same rule applies there: A2
    after an `(A2)+` copy while the dead A0 of an earlier statement is
    passed over; not implemented).  Conformance: `tests/conformance/
    iic.pas`, `xns_idp_get_stats.pas`; after a call, `file_truncate_d.pas`
    (FILE_$TRUNCATE_D 0xE74022 A0), `ring_svc_open.pas` (RING_$SVC_OPEN
    0xE76E12 A0), `time_q_remove_elem.pas` (TIME_$Q_REMOVE_ELEM 0xE16E84
    A0), `proc2_pguid_to_upgid.pas` (PROC2_$PGUID_TO_UPGID 0xE410B2 A0,
    0xE410B8 A1); past dead slot loads, `tty_k_inq_pgroup.pas`
    (TTY_$K_INQ_PGROUP 0xE6796E A3; TTY_$K_INQ_INPUT_BREAK_MODE OK in
    srcsurvey); driver tests `slot_loads_after_branches_and_calls`,
    `slot_loads_pass_over_dead_slot_loads`
    (`crates/dpc-m68k/tests/control.rs`); golden
    `regalloc/late_slot_pass`; execution test `late_slot_pass`.
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
      (`move.l D0,-(SP)` at 0xE42CDA); the routine at 0xE4FE0A (map:
      DIR_$FIND_NET+0x1556), 0xE4FE36 A0 (`pea (A0)` at 0xE4FE76); the
      routine at 0xE4CFFA (map: DIR_$DO_OP+0xFCE), 0xE4D00E A0
      (0xE4D040); TTY_$I_INIT 0xE3325A A0 (0xE332DC); the routine at
      0xE01AD2 (map: AST_$UPDATE+0x402), 0xE01B6E A0 (0xE01B82);
      MAC_$SEND 0xE0BB88 A0 (0xE0BB9C); the routine at 0xE55220 (map:
      DIR_$OLD_CLEANUP+0x6F6), 0xE5523C A0, `clr.l (A0)` then `pea (A0)`
      at 0xE5524C, the shape of 0xE1D592;
    - temporaries computed inside the argument list: 1198 scratch, 276
      callee-saved.

    So the hypothesis holds without exception only in routines without
    branches, by this proxy.  The proxy mixes kinds of parameter load,
    though; separated, the branch case is settled for loads at entry
    (next item).
  - **Survey with branches (bead domain-pascal-9j1z; `tools/regalloc.py
    callparams`, kept, with tests):** the parameter loads that cross no
    call and whose last use on some path is an argument *push* (`pea`,
    `move -(SP)`) of a call statement they are not part of, by kind of
    load: "entry" (before the routine's first branch, branch target or
    call) or "late"; "once" (no other instruction reads the slot: the
    parameter lives in that one register, as `dpc`'s entry loads do,
    12.2) or "reload" (the slot is read again elsewhere); by where the
    call lies: before the first branch ("prefix"), in a conditional part
    with no join before it ("cond"), after a join inside a conditional
    part ("join"), on every path after a join ("every"); by the other
    paths: "dead" (every last use is a push for that call), "multi"
    (pushes for several calls, e.g. one in each part of an `if`),
    "other" (another last use is not a push: the value is live on
    another path); by class.  Counted separately: "forced" values, for
    which liveness alone leaves no scratch register free (colU), so the
    register shows nothing about holding.  Whole image, routines with
    branches, not forced:

    | load / slot | position | callee-saved | scratch | forced (all callee-saved) |
    |---|---|---|---|---|
    | entry / once | prefix | 2 (VOLX_$GET_INFO 0xE6B5D4 A3, 0xE6B5D8 A4) | 0 | 1 |
    | entry / once | cond | 26 | 1 (EC2_$ADVANCE 0xE42CB8 D0) | 15 |
    | entry / once | join | 5 (SOCK_$PUT_INT 0xE161A2 A2; the nested 0xE1CC10 of KBD_$RCV, 0xE1CC1A A2) | 0 | 14 |
    | entry / once | every | 0 | 0 | 4 |
    | entry / reload | any | 1 (0xE41A32) | 3 (0xE02572, TTY_$I_INIT 0xE3325A, 0xE5523C) | 0 |
    | late / once | any | 1 (DISK_$DIAG_IO 0xE6BCBC D2) | 1 (0xE4D00E A0 in the routine at 0xE4CFFA, map: DIR_$DO_OP+0xFCE) | 0 |
    | late / reload | any | 3 | 43 (0xE01B6E in the routine at 0xE01AD2, map: AST_$UPDATE+0x402; MAC_$SEND 0xE0BB88; 0xE4FE36 in the routine at 0xE4FE0A, map: DIR_$FIND_NET+0x1556) | 0 |

    The entry/once loads with branches, by the other paths: dead 16 of
    17 callee-saved, multi 7 of 7 (XPD_$GET_FP_INT 0xE5C560 A2, `move.w
    (A2),-(SP)` at 0xE5C574 in the `then` part, `pea (A2)` at 0xE5C57E
    in the `else`), other 10 of 10 (SIO_$I_INHIBIT_XMIT 0xE1C9D4 A2,
    `bset` through it in one part, `pea (A2)` at 0xE1C9EC in the other);
    by class: A 29 of 29, D 4 of 5 (AREA_$ASSOC 0xE096B0 D2, `move.w
    D2,-(SP)` at 0xE096D2 inside an `if`).  Without branches: 5 of 5
    (above).

    Recounted independently (review of bead domain-pascal-9j1z: an own
    instruction model, liveness and dominators over the same routine
    list, with `case` tables decoded, so the routines `regalloc.py`
    skips for a computed `jmp` are in): the same sites and the same
    forced counts, plus two in `case` routines, TERM_$INQUIRE 0xE66D98
    A2 (held, after a join, a call in many arms) and OS_$BOOT_ERRCHK
    0xE34B22 (forced): 34 of 35 held.  The 35 sites lie in 30 routines
    of 23 map segments.  Sites in one routine are not independent where
    two such values compete for the one free scratch register
    (VOLX_$GET_INFO A3 and A4: A1 is free for either, not for both).

    **Rule (Reported; 33 of 34 sites):** a parameter loaded once at
    entry whose last use is an argument of a call is held through the
    call, so it takes a callee-saved register, whether the call lies
    before the first branch, in an `if` part or after a join inside one,
    and whether or not the parameter is used on another path.  So
    neither a later branch target nor the call being in an `if` part
    matters.  Not seen: a call on every path after a join (its 4 sites
    are forced).  The one exception, EC2_$ADVANCE 0xE42CB8, is a
    by-address slot whose address is kept in a data register because
    it is also used as data (12.2, "Pointers in data registers"):
    `move.l D0,-(SP)` at 0xE42CDA, inside an `if`, D0 not held.  It is
    the only entry/once pointer of this survey that is also used as
    data and for which a scratch register is free.  The two other
    data-register pointers with one free are held: NAME_$VALIDATE
    0xE49F54 D2, dereferenced only through a copy (`movea.l D2,A3` at
    0xE49F76; bead domain-pascal-0b1), and DIR_$OLD_GET_ENTRYU 0xE58064
    D2, only passed on (`move.l D2,-(SP)` at 0xE5809C and 0xE580B0, one
    call in each part of an `if`).  Why EC2_$ADVANCE
    differs is not known (bead domain-pascal-b3wz).

    The other kinds do not bear on the rule.  A slot read again
    elsewhere (reload; nearly all the late loads) means that the original
    did not keep the parameter in one register: such a load is a
    temporary, mostly scratch (TTY_$I_INIT 0xE3325A pushes A0 at
    0xE332DC and reloads `(8,A6)` into A0 at 0xE332E6).  `dpc` does not
    reproduce these reloads (12.2).  The routine at 0xE4CFFA (map:
    DIR_$DO_OP+0xFCE) loads its `(0x16,A6)`, used three times, after the
    routine's first call, not at entry (0xE4D00E), and does not hold it
    (`pea (A0)` at 0xE4D040, inside an `if`); `dpc` would load it at entry, live across that call (R3).

    **Implemented** (`held_through_calls` in
    `crates/dpc-m68k/src/isel.rs`): a parameter loaded at entry
    (`Selector::entry_loads`) whose last use is in a call statement's
    arguments is held through the call in a body with branches too.
    Refused: a call on every path from the entry after a branch
    (`on_every_path`, `after_a_branch`; bead domain-pascal-cs2n); a
    pointer parameter held in a data register (`data_ptr_regs`; bead
    domain-pascal-b3wz); and any other value, a register-held local or a
    computed value (not surveyed: DTTY_$TSTART 0xE1D6E0 keeps its local
    `current` in D0, a scratch register, for `pea (5,A3,D0.w)` at
    0xE1D6F8 inside an `if`; bead domain-pascal-ig8d; refused so in
    `../source`'s Pascal at 343e0939: the procedure pointer local
    `handler` of NET_$OPEN, NET_$CLOSE, NET_$IOCTL, NET_$RCV and
    NET_$SEND, and locals of DTTY_$TSTART, BAT_$DISMOUNT and
    MAC_OS_$DEMUX).  Pointer locals assigned from calls and values live
    across an earlier call stay held as before.  Conformance unit
    `tests/conformance/xpd_fp_int.pas` (XPD_$GET_FP_INT,
    XPD_$PUT_FP_INT, byte-identical); golden
    `regalloc/args_held_branches`; execution test `args_held`; driver
    tests `argument_last_use_in_a_branchy_body_is_refused`,
    `argument_last_use_of_a_data_pointer_or_local_is_refused`; unit test
    `call_positions_for_held_arguments`.
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

**R5. Function results (rule Uncertain, fitted to eight sites;
implemented; beads domain-pascal-5aw.11, -8aj).**
- The result variable of a body of several statements is one value
  (section 3, "Result variable"): D0 when no call follows an assignment
  whose value is still needed, else a callee-saved register moved to D0
  at the end (66 of 67 such moves, `tools/resultsurvey.py list reg`).
- Parameters loaded at entry beside it, in functions without a call
  (`tools/resultsurvey.py entry`, the D0 functions loading two or more
  data parameters into scratch registers):
  - PARITY_$CHK_IO 0xE0B180/0xE0B18A `(8)`->D1, `(0xc)`->D0: both die
    before the result is first written; `(0xc)` is used later
    (0xE0B1A0) than `(8)` (0xE0B196);
  - 0xE65904 `(8)`->D1, `(0xc)`->D0 (`(8)` compared at 0xE65940, `(0xc)`,
    a byte, tested at 0xE65956); 0xE76352 and 0xE76422 `(8)`->D1,
    `(0xc)`->D0, `(0x14)`->D2 (`(0xc)` compared after `(8)` inside the
    loop, 0xE76380 / 0xE76450);
  - 0xE0B202 and 0xE7630C `(8)`->D0, `(0x10)`->D1 (`(0x10)` copied into
    the loop count at entry, `(8)` compared in the loop): parameter order
    and "used later" agree;
  - 0xE54B58 `(0xc)`->D0, `(0xe)`->D1: `(0xe)` is read by the `divu`
    that computes the result in D0, so it cannot have D0;
  - PROC1_$SUSPENDP 0xE1487E `(8)`->D1: live while the result is in D0
    (`st D0` at 0xE14886).
  Procedures keep parameter order (12.2), so these orders are not the
  colouring order of R1.  A rule that fits all eight: colour the result
  variable first, then the data parameters loaded at entry that do not
  interfere with it, the one whose last use comes latest first; then the
  other values in address order.  **Implemented** that way (`regs.rs`,
  `allocate_apart`'s `first`; unit test `result_variable_on_several_paths`;
  golden `results/result_variable`, `chk_io`).  A function body of one
  statement keeps the evaluation order of PROC1_$TST_LOCK (no result
  variable).
- Not explained: MST_$VA_TO_SEGNO loads the parameter it returns straight
  into D0 (0xE0DCE0) and the first parameter into D2 (0xE0DCD6), not D1;
  0xE15222 returns its parameter from the D0 it was loaded into
  (0xE15226), the result and the parameter sharing one register (no copy
  coalescing elsewhere, R2).

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

**R1a. The allocation order: classic allocators simulated (bead
domain-pascal-jjh; `tools/regalloc.py orders`, `simmisses`; no algorithm
identified).**  This tests whether a published allocation order explains
R1's misses.  Method: per routine, a graph over the values (webs) that does
not depend on the original's registers.  An edge joins two values of one
class that overlap (Chaitin) or where one is defined by an instruction
reading the other (R2).  Each value also has a set of forbidden registers:
those a call inside its range writes (R3), and those of occupants that are
not allocation decisions (call results).  An order is scored in two ways:
- `loo`: the value takes the lowest register left free by its neighbours
  *earlier in the order*, those neighbours keeping the original's
  registers.  In address order this is close to `colQU` (R1) but not
  equal: 130 values differ, `colQU` right on 111 and `loo` on none.  93
  of them are registers read by the defining instruction whose value is
  defined later in address order (a loop-carried read): `colQU` refuses
  them, `loo` does not see that neighbour yet.  Hence 71.6 / 72.1 below
  against R1's 72.8 / 73.1.
- `sim`: a full simulation, the neighbours keeping the simulated
  registers.  Errors compound, so `sim` is lower.

The order-free row is `colU` computed on this graph.  It is also what the
oracle order (values sorted by the original's register) achieves.  It
differs from `summary`'s `colU` on 41 values: the graph applies R2 to a
load into its own dying base (`movea.l (4,A0),A0`, 0xE190E0), which
`colU` lets through for the original's register.

Routines of 0x30–0x150 bytes (1099; 10136 values), as `loo / sim` %.
"Global" means live in more than one block or across a call (4705 values);
"local" is the rest (5431).  The two halves are alternate routines in
address order (550 / 549; bit 1 of the start address would split by the
loop-alignment mode, section 7).  Nothing is fitted, so the halves are a
stability check, not a held-out test.  Chow-Hennessy is simulated without
live-range splitting (its `sim` leaves at most 4 values without a
register).  "Rev. ties" breaks priority ties in
reverse address order instead of address order.

| Order | D (4978) | A (5158) | global | local | all | even half | odd half | rev. ties | whole image (22629) |
|---|---|---|---|---|---|---|---|---|---|
| order-free (`colU` on the graph) | 80.6 | 81.2 | 91.8 | 71.5 | 80.9 | 80.2 | 81.7 | 80.9 | 75.5 |
| address order (first definition) | 71.6 / 60.7 | 72.1 / 62.8 | 76.0 / 63.4 | 68.3 / 60.3 | **71.9** / 61.8 | 72.0 | 71.7 | 71.9 | **65.9** / 55.7 |
| reverse address order | 53.3 / 39.8 | 57.5 / 42.4 | 62.6 / 39.1 | 49.3 / 42.9 | 55.5 / 41.1 | 54.7 | 56.3 | 55.5 | 53.3 / 40.4 |
| usage count (Freiburghouse; defs + uses, most first) | 64.6 / 50.1 | 70.4 / 58.6 | 71.2 / 53.1 | 64.4 / 55.5 | 67.6 / 54.4 | 67.3 | 67.8 | 62.1 | 63.5 / 51.3 |
| the same, loop-weighted (10^depth) | 62.3 / 47.5 | 68.9 / 55.7 | 69.2 / 49.5 | 62.5 / 53.5 | 65.6 / 51.7 | 65.5 | 65.8 | 62.1 | 62.5 / 50.0 |
| Chow-Hennessy, refs / instructions in range | 55.7 / 42.2 | 59.5 / 43.7 | 65.8 / 40.3 | 50.5 / 45.3 | 57.6 / 43.0 | 56.9 | 58.3 | 57.2 | 55.4 / 42.1 |
| the same, unconstrained values last | 57.5 / 43.6 | 62.3 / 46.9 | 66.5 / 43.7 | 54.3 / 46.6 | 60.0 / 45.3 | 59.5 | 60.4 | 59.6 | 57.6 / 44.0 |
| Chow-Hennessy, refs / blocks in range | 58.6 / 45.1 | 65.0 / 51.7 | 69.8 / 44.8 | 54.9 / 51.6 | 61.8 / 48.5 | 61.4 | 62.3 | 58.9 | 58.9 / 46.6 |
| the same, unconstrained last | 60.6 / 47.3 | 67.8 / 55.7 | 70.7 / 49.5 | 58.8 / 53.5 | 64.3 / 51.6 | 64.1 | 64.4 | 61.1 | 61.2 / 49.3 |
| Chow-Hennessy, loop-weighted refs / instructions, unconstrained last | 57.9 / 43.8 | 62.8 / 47.7 | 66.9 / 44.4 | 54.7 / 47.0 | 60.4 / 45.8 | 59.9 | 60.9 | 60.1 | 58.0 / 44.5 |
| Chow-Hennessy, loop-weighted refs / blocks, unconstrained last (`chbwd`) | 61.0 / 47.2 | 67.6 / 55.2 | 70.2 / 49.1 | 59.3 / 53.1 | 64.4 / 51.3 | 64.3 | 64.5 | 61.4 | 61.4 / 49.3 |
| Chaitin = Briggs simplify/select, lowest address removed first | 54.5 / 40.1 | 59.4 / 43.0 | 64.0 / 40.0 | 50.9 / 43.0 | 57.0 / 41.6 | 56.3 | 57.7 | 71.9 | 55.0 / 40.4 |
| the same, smallest degree removed first | 58.6 / 40.4 | 61.7 / 43.7 | 64.4 / 40.4 | 56.4 / 43.5 | 60.2 / 42.1 | 59.8 | 60.5 | 71.7 | 57.7 / 41.5 |
| two-level: global values in address order, then local in address order | 71.1 / 60.1 | 71.9 / 62.7 | 74.2 / 63.5 | 69.1 / 59.7 | 71.5 / 61.4 | 71.8 | 71.2 | 71.5 | 65.6 / 55.3 |
| two-level, global values by `chbwd` | 67.4 / 54.8 | 70.2 / 58.8 | 68.5 / 53.4 | 69.1 / 59.7 | 68.9 / 56.8 | 68.6 | 69.1 | 67.2 | 64.7 / 53.4 |
| two-level, global values by Chaitin/Briggs | 65.4 / 51.3 | 66.1 / 52.4 | 61.9 / 43.9 | 69.1 / 58.8 | 65.8 / 51.9 | 65.2 | 66.3 | 71.5 | 61.8 / 49.0 |
| the other way: local values first, then global (address order) | 64.2 / 50.4 | 68.3 / 57.5 | **78.9** / 55.6 | 55.4 / 52.7 | 66.3 / 54.0 | 66.3 | 66.3 | 66.3 | 61.6 / 49.3 |
| end of the live range, latest first (`rlast`, a backward scan; bead domain-pascal-ue7) | 68.0 / 53.8 | 68.9 / 55.3 | 72.5 / 56.0 | 64.9 / 53.2 | 68.4 / 54.5 | 68.0 | 68.9 | 68.4 | 63.3 / 50.5 |
| parameter loads at their first use (`fu`; ue7) | 67.6 / 55.9 | 69.9 / 58.7 | 72.3 / 56.7 | 65.8 / 57.9 | 68.8 / 57.4 | 68.3 | 69.2 | 68.8 | 64.1 / 53.0 |
| values live in several blocks first, a call not counted (`2lb-addr`; ue7) | 71.0 / 60.0 | 71.9 / 62.6 | 74.2 / 63.3 | 69.1 / 59.6 | 71.5 / 61.3 | 71.9 | 71.1 | 71.5 | 65.6 / 55.3 |
| the same with `fu` (`2lb-fu`; ue7) | 68.7 / 57.4 | 70.2 / 59.2 | 70.5 / 57.8 | 68.6 / 58.8 | 69.5 / 58.3 | 69.5 | 69.5 | 69.5 | 64.4 / 53.4 |

Findings:
- **No published order separates from the others, and none beats a plain
  top-down pass.**  Address order is the best order-dependent model.  It
  is stable across the halves (72.0 / 71.7) and leaves 9 points to the
  order-free bound.  Chow-Hennessy is 7–14 points lower in every variant
  (block or instruction size, with or without loop weights, with or without
  deferring unconstrained values), and it is tie-sensitive (`chbwd` drops
  3 points with reverse ties).  The usage count is 4 points lower.
- **Chaitin and Briggs cannot be told apart here.**  No routine's graph
  ever blocks: in all 1505 routines with values, simplify always finds a
  node with fewer neighbours than free registers (k = 8 data, 5 address,
  less the forbidden ones).  The test is not vacuous: 3151 values (1259
  in the 0x30–0x150 set) start with at least k neighbours and are removed
  only after some of them.  There is never a spill or optimistic
  decision.  The select order is then the removal tie-break, apart from
  those late removals.  Removing the highest address first selects
  nearly in address order and matches that row on all but 3 of the 10136
  values ("rev. ties", 71.9; data 71.7 / 60.7 against 71.6 / 60.7).  So
  the image cannot
  distinguish "Chaitin/Briggs with an address-order tie-break" from a
  one-pass top-down greedy colouring.
- **Two-level (the getreg hypothesis)**: colouring the global values first
  gains nothing over address order (71.5 against 71.9).  Local values in
  address order after them score 69.1, against 68.3 when interleaved.
  The reverse, local values first, is the best model for global values
  (78.9) but the worst for local ones (55.4).  Global values seem to see
  the registers of later local values (the parameters in D3/A3 and above
  under R1, 0xE01C5A), but no fixed split reproduces it.
- **The residual is not an ordering effect.**  Almost every miss of
  address order (2766 of 2853) and of the order-free bound (1802 of 1935)
  is a value in a *higher* register than predicted.  Of those, 1359 and
  996 are in a callee-saved register where a scratch one was free; 239 of
  the 996 are in routines without calls.  In `loo`, a value's prediction
  under any order is never above its `colU` prediction (it avoids a subset
  of the same neighbours' registers), so no colouring order explains the
  1802.  The free register must be occupied in the original's view, so
  lifetimes are longer than the web's liveness (R4).
  Examples of misses of the order-free bound:
  - MEM_$PARITY_LOG 0xE0ADF8..0xE0AE04 and again 0xE0AE42..0xE0AE4E.
    The strength-reduced `index * 18` (`ext.l; add.l Dn,Dn; move.l
    Dn,Dm; lsl.l #3,Dm; add.l Dm,Dn`) is computed in D3/D4 while D2 is
    free, then in D5/D6 while D2–D4 are free (`colU` predicts D2 for
    the product at both sites).  The product is then copied into D0 for
    `addq.w #1,(4,A5,D0.l)` (0xE0AE06) and `move.w #1,(4,A5,D0.l)`
    (0xE0AE50).  D0, the enclosing `dbf` counter (0xE0AE12), is dead on
    the first path; address order predicts D0 at both sites.
  - APP_$DEMUX 0xE00A9C, 0xE00AA0: `(12)`→D2 and `(16)`→D3 in a routine
    where D1 is free over those webs (`colU`).  They are later copied
    into A4 and A3 before a call (0xE00B1A, 0xE00B16).  A copy (a `move`)
    into a call-crossing value reads 24 of the 996 values (16 where the
    copy is their last use), so it explains few of them.

  Misses of address order that `colU` explains, i.e. later values seem
  to be coloured first (the two-level finding above):
  - Module-local 0xE0C5AE loads its parameters `(12)`→D2 and `(14)`→D1
    (0xE0C5B6, 0xE0C5BA), the first-loaded in the higher register.
  - Module-local 0xE01C52 (R1 above): A3, D5, D6, A4 where address order
    predicts A2, D2, D2, A2.
- Consequence for `dpc`: keep the address-order greedy colouring (R1,
  `colQU`).  The next step is the lifetime model (R4), not the order.

**R1b. A pointer local assigned a call's result before a parameter that
dies in its block (bead domain-pascal-ue7; survey `tools/regalloc.py
paramlocal`; rule Uncertain, a shape with no counterexample in the image;
implemented).**  The shape: `p := f(..., status); if status.all <>
status_$ok then ...`, the status address loaded at entry and pushed, `p`
kept in a callee-saved address register.  Address order gives the status
A2 and `p` A3; the image the reverse: KBD_$PUT 0xE1CEB4 `(0x18)`->A3,
0xE1CEC6 `movea.l A0,A2`; KBD_$SET_KBD_TYPE 0xE7252C / 0xE7253E;
TTY_$I_SET_RAW 0xE673B8 / 0xE673CC.
- *The survey.*  Pairs of interfering values of one class in
  callee-saved registers, one a parameter load (P) and the other not (L),
  that could swap registers (each register free of the other's remaining
  neighbours): which gets the lower register?  Whole image, P defined
  first:

  | L (the other value) | address: L lower / P lower | data: L lower / P lower |
  |---|---|---|
  | any (P live across a call) | 113 / 186 | 224 / 352 |
  | any (P not) | 13 / 32 | 18 / 47 |
  | a call's result kept (`movea.l A0,An` / `move.l D0,Dn` after the call and its pop), P live across that call | 22 / 9 | 10 / 13 |

  So neither "locals before parameters" nor address order holds in
  general; the orders tried on the whole graph (R1a table: `rlast`, `fu`,
  `2lb-addr`, `2lb-fu`) all score below address order.  Kept call results
  of the address class, unfiltered (no swap test), by what P does after
  the assignment:

  | P after `movea.l A0,An` | L lower | P lower |
  |---|---|---|
  | used, and last used in the assignment's block | **6** | **0** |
  | used again after the block, used before the call too | 13 | 13 |
  | used again after the block, not used before the call | 8 | 3 |

  The six: KBD_$PUT, KBD_$SET_KBD_TYPE, TTY_$I_SET_RAW above;
  TTY_$K_SET_FLAG 0xE67430 / 0xE67446; KBD_$GET_CHAR_AND_MODE 0xE724CC /
  0xE724E0; the AUDIT_ routine at 0xE712BA (map: AUDIT_$SERVER+0x1F4),
  0xE712C6 A4 / 0xE712D8 A2 (there the result comes in D0 from the
  `bsr.w` to the module-local 0xE7120C, `movea.l D0,A2`, so the
  implemented rule below, which looks for `movea.l A0,An`, covers five
  of the six).  Against a wider rule: CAL_$SHUTDOWN 0xE3E6F2
  keeps the status in A2 and the result in A3 (0xE3E714), the status
  pushed again by the later call (0xE3E72A).  Data: one pair each way
  (0xE03556 / 0xE0377E, 0xE02B0A / 0xE02B22).
- *How much the six show (ue7 review).*  The condition "last used in the
  assignment's block" was chosen after looking at the six, and they are
  one idiom (`p := f(..., status); if status.all <> status_$ok then
  ...`, the status address used only by the push and the test; three
  callees: KBD_$GET_DESC 0xE1AA26 three times, TTY_$I_GET_DESC twice,
  the module-local 0xE7120C once).  "Live across the
  call" adds nothing: a parameter loaded before the call that interferes
  with the result is live across it.  Against the base rate the 6 / 0 is
  weak: kept call results are already lower in 22 of 31 swappable pairs
  (71%), 16 of 25 (64%) without the six, and 0.64^6 is about 0.07.  A
  parameter dying in the block of a value that is *not* a call result
  shows no preference (9 lower / 13 higher, swappable pairs).  The
  split of the other rows depends on how "used before the call" is
  defined (counting from the start of the call's argument pushes: 9 / 9
  and 11 / 7, unfiltered; 20 / 16 in all, against the table's 21 / 16).  What the six do
  settle: address order is wrong for all of them, and no site in the
  analysed image (1588 routines; 47 with a `case` table skipped)
  contradicts the rule, so it changes no prediction elsewhere.  A
  broader rule (every kept call result before a parameter live across
  the call) would match 22 of 31 against 15 of 31 for R1b plus address
  order, but breaks CAL_$SHUTDOWN (conformance test); not adopted
  (bead domain-pascal-80rr).
- *Target before right-hand side?  (bead domain-pascal-u6di,
  `tools/rhssurvey.py`; refuted.)*  Hypothesis: the original colours an
  assignment's target before the values first read in its right-hand
  side (a parameter's first read counted after its entry load).  Pairs
  (entry-loaded parameter P, other value L; callee-saved, interfering),
  split by where P is first read: before L's statement, in it (the
  call's argument pushes, or L's defining instruction), or after.
  - Kept address call results with P first pushed in the call's
    arguments: 8 L-lower / 3 P-lower, which is exactly the six R1b sites
    (P dies, 6/0) plus 2/3 where P lives on.  P is lower at
    CAL_$SHUTDOWN (`pea (A2)` 0xE3E6F8 in the arguments, result to A3
    0xE3E714) and AST_$MSTE_ACTIVATE_AND_WIRE (`pea` 0xE02F4E, result
    to A3 0xE02F56).  Against the base rate of the other pairs (18/13)
    p ~ 0.25: nothing beyond R1b.  Those two sites fit R1b's "P dies in
    the block" and contradict "right-hand side first".
  - Data call results: 4/0 (1/0 swappable).
  - Non-call right-hand sides that read P (`movea.l (32,A2),A3`
    0xE17D52; `lea (718,A2),A3` 0xE1B722): P is *lower*, address 4/21,
    data 25/51, below the base rates.  Weak evidence: in the wider
    statement window some first reads belong to an interleaved
    statement (0xE00CB4/0xE00CC0).
  - A full top-down first-use order scores below address order (address
    non-call, swappable: 55% against 68%).
  - The counts do not depend on whether a statement starts after the
    previous statement or at the first argument push.
  - Parameters keep their entry order among themselves (AUDIT_ routine
    0xE712BA: `(8)`, first read after the call, A3 below the status A4).
- *Rule (`regs::call_results_first`, `allocate_apart`'s `late`).*  A
  pointer local assigned a pointer function's result (`movea.l A0,An`
  right after the call and its pop) is coloured before an address
  register parameter loaded at entry that is live across that call and
  whose last use is in the assignment's block; the parameter then takes
  the next free register.  Other values keep address order.  Why the
  original does this is not known (no order or lifetime rule tried
  explains the six together with the 13 / 13 row).
- Not covered: the parameter used again after the block, in particular
  the AUDIT_ routine's `(8)` (0xE712C2 A3, used after the test, above the
  result's A2) and TTY_$K_ENABLE_FUNC 0xE675BC `(0xc)`->A3 beside the
  result's A2; there `dpc` keeps address order (bead
  domain-pascal-80rr).  TTY_$K_ENABLE_FUNC also
  parks the result in D0 until the status (A2) dies (0xE675D6,
  0xE675E0), bead domain-pascal-bks.
- Conformance: `tests/conformance/tty_i_set_raw.pas` (TTY_$I_SET_RAW);
  `tools/srcsurvey.py`: KBD_$PUT and KBD_$SET_KBD_TYPE now CODE-OK,
  TTY_$I_SET_RAW OK.

#### Common subexpressions

**R7. Operator results are reused across statements, across an `if` and
its join, and past stores to other objects (Verified at the sites; the
kill rules Verified by survey with the limits below; implemented, bead
domain-pascal-g5q.7).**
- 0xE177A6 `clr.l D1; move.w D0,D1`, the zero-extended `offset`, is
  reused at 0xE177D0 after the `if` (0xE177A6..0xE177CA) and after the
  stores through `actual_len`.
- FIM_$ACKNOWLEDGE: `PROC1_$AS_ID * 12` (0xE0A972..0xE0A97E, D2) is used
  at 0xE0A994 and again at 0xE0A9B0, past the stores at 0xE0A994 and
  0xE0A9A6 (other static arrays).
- Two-address arithmetic goes into the operand that is not reused:
  0xE177B4 `add.l D1,D2` keeps D1 (the CSE).  A sum of the held value and
  a variable copies the held value and adds the variable to the copy:
  0xE177D0..0xE177D4 `move.l D1,D3; ...; add.l (0x14,A5),D3`.
- A register known to hold 0 stands in for the constant 0 as a bit
  number (Verified at three sites; rule Reported): TTY_$K_GET 0xE1C3EA
  `clr.w D3` (a register-held local set to 0), then, after a call,
  0xE1C414 `btst D3,(1,A4); sne D0` for member 0 of a word set, where
  member 1 next is `btst #1,(1,A4)` (0xE1C41E); TTY_$K_PUT 0xE67BFA
  `clr.w D3` ... 0xE67C32 `btst D3,(1,A0)` (member 1 at 0xE67C08 is
  `btst #1,(1,A0)`); RING_$SENDP 0xE759B4 `clr.b D4` ... 0xE759C6
  `btst D4,(1,A1); sne D7`.  The value survives a call in two of the
  three (callee-saved registers); compare FLP_$SHUTDOWN's copy of a
  register-held 0 (7.4, 0xE3E24A).  `dpc` emits `btst #0` (lead:
  ../source pascal-translation-notes finding 7; bead domain-pascal-wgme).

*Survey* (`tools/csesurvey.py`; routines of 0x30..0x150 bytes, 1099
analysed).  Each routine is evaluated symbolically: a register holds an
expression over memory reads, constants and operators, with the events
since it got it (stores, stores through a pointer, stores to one of its
operands, calls, branches, joins, back edges).  A *reuse* is a read of a
register holding a computed expression after such an event; *lost* is a
computation of an expression computed before on every path but no longer
in a register.  `agg` (the strongest event between; "sto" = the
register's value was itself stored, i.e. a variable forwarded as in 12.5,
not a CSE; set apart):

| kind | no call: reuse / lost | after a call: reuse / lost |
|---|---|---|
| zero-extension (`clr.l; move.w`) | 25 / 2 (6 / 1 after a store to the operand: register-held locals, e.g. 0xE0C6AC) | 3 / 3 |
| sign-extension (`ext.l`) | 5 / 3 | 11 / 6 |
| product by shifts and adds | 8 / 4 (the 4: HINT_$ADD_CACHE 0xE49E12, 0xE49E24, after stores into the unit's data at a run-time index) | 8 / 8 (and 5 computed again while still held) |
| shift (`lsl`, a scaled index) | 32 / 15 | 8 / 24 |
| sum, difference | 23 / 13 | 25 / 162 |

The model cannot tell a CSE from a register-held local (12.5) or a
strength-reduced pointer; the sites behind the rules below were checked
by hand.

- **What is reused** (Verified at the sites): zero- and
  sign-extensions of a variable, constant products and scaled indices of
  a variable (FIM_$ACKNOWLEDGE 0xE0A9B0; NETWORK_$REMOVE_NET's `idx * 8`
  in D1 at 0xE0F29C, read at 0xE0F2A4, 0xE0F2AE, 0xE0F2B4 across its
  stores).  Sums and other operators: not checked (many hits are locals).
- **Stores to other objects do not end a reuse**: stores to other named
  arrays (0xE0A994, 0xE0A9A6), to an element at the same run-time index
  (0xE0F2AA), through a pointer parameter when the operand is a parameter
  (0xE177C0, 0xE177CA).
- **What ends it** (the "lost" sites with no call, checked): a store to
  the operand itself (0xE4B388, after `bchg #0,(31,A0)` writes the low
  byte of the word `(30,A0)`); a store into the unit's data at a run-time
  index or through a pointer when the operand is in the unit's data
  (0xE53462: `(5464,A5)` scaled again after stores through `lea
  (0,A5,D3.l),A0`; 0xE6E924, 0xE6E932, 0xE6E940; NETLOG_$LOG_IT
  0xE71C0E; HINT_$ADD_CACHE 0xE49E0C..0xE49E14); a variable of the enclosing routine is read again anyway
  (R8; MST_ 0xE30B5C).  The other "lost" sites are the first `lsl` of a
  ×12 that a ×4 of the same variable shared (TIME_$WAIT 0xE165D8,
  MSG_$WAITI 0xE59C66, SIO_$K_TIMED_BREAK 0xE67F5C: a product is not
  built from another product's partial result, R10), or are inside
  expressions the model splits.
- **Calls** (Uncertain): after a call most values are computed again
  (sums 25 / 162), some are held in a callee-saved register (sign
  extensions 11 / 6; TIME_$WAIT 0xE1662C computes `AS_ID * 12` again
  while D3 still holds it from before a call).  No rule found.
- **Loops**: values defined before a loop are reused past its back edge
  (shifts 4 / 2 with a back edge).  Module-local 0xE1C9FC: `n * 4` in D1
  (0xE1CA0C) is read at 0xE1CA4A after the `dbf` loop
  0xE1CA2C..0xE1CA46 (and from the `bmi` that skips it), which `dpc`
  also does.  Whether a value is reused inside a loop: not studied.
- Within one statement (ref 6.4.24, p. 6-27, `-dba`): TIME_$WAIT
  0xE16610 reads `PROC1_$AS_ID` once for its ×4 and ×12 (`move.w D0,D1`)
  where FIM_$ACKNOWLEDGE reads it twice (0xE0A972, 0xE0A980): R8.

**Implemented** (`crates/dpc-m68k/src/isel.rs`, `cse_value`,
`cse_valid`, `store_may_write`): the widening of a plain variable (a
by-value parameter, a frame local, a variable of the unit's data or
another unit's; not the enclosing routine's, not a register-held
local), its scaled index and its product by a constant (one key: a
scaled index and a product are the same value).  The body is selected
twice: a probe records which computations a later one may reuse, the
second selection holds exactly those in their register (`Val::Reg`, so
users copy it or work on the other operand) and the later ones read it.
A reuse is valid when, between the two, there is no call, no label a
branch from before the first targets (so the first dominates: one in an
`if`'s condition reaches after the `if`, one in a `then` part does not),
no loop around either, and no store that may write the variable: the
variable itself (overlapping bytes), an element at a run-time index of
the same frame, unit data or named array, a store through a pointer when
the variable is in the unit's data or another unit's, or in the frame
once some frame object's address escaped (the `forwarded` rule of 12.5);
any memory write outside a recorded store statement counts as one, and
so does a recorded store that may write the variable whatever it
emitted (a store to a register-held variable writes no memory).
Stores to other variables do not count (FIM_$ACKNOWLEDGE).  Everything
else is computed again, as before (where the image reuses after a call
or into a loop, `dpc` diverges: not refused, since no rule is known).
Golden `tests/golden/regalloc/cse` (FIM_$ACKNOWLEDGE's ×12 with unit
data, LOG_$READ's worker to 0xE177CA plus a sum with the held value);
driver test `computed_values_are_reused_only_while_valid` (operand
written on one path, computed in one arm, stores through pointers to the
unit's data and to a pointer parameter, an escaped local, a call, a loop,
a scaled index with and without its variable written); unit test
`store_may_write_cases`.  No conformance unit yet: FIM_$ACKNOWLEDGE needs
two elements of other units' arrays in one statement, which `dpc`
refuses (5.2, "Another unit's array": the order of their bases is not
modelled); the LOG_$READ worker the rest of 12.3.

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
  - LOG_$ADD 0xE1763E reads `buf^.head` once for both operands of an
    `and then`: `move.w (A0),D1; cmp.w (2,A0),D1; blt.b <exit>; cmp.w
    D0,D1; ble.b <head>` (0xE176D6..0xE176E0, its loop test), the shape
    of 0xE0283C.  `dpc` loads the operand again for the second compare
    (lead: ../source pascal-translation-notes finding 22; bead
    domain-pascal-g5q.6).
- Uplevel variables (through the static link) are re-read even within
  one statement: 0xE03FEE/0xE03FF8; 0xE135DA/0xE135E2.
- Uncertain against: AREA_$DELETE reads the flags word once
  (0xE07C78) for the `while` test and for the next `if` (0xE07C80).
- Uncertain against (Reported), also LOG_$ADD: its copy loop reads
  `data[i]` once for two statements, `move.w (-2,A0),D6; move.w
  D6,(6,A2); move.w D6,(10,A1)` (0xE17730..0xE17738); and
  TIME_$CURRENT_CLOCKH is read once (`move.l (0xe2b0e4).l,D6`, 0xE17708)
  for two stores (0xE1770E, 0xE17712), though ../source's C reads it
  into a local, so a register-held local (12.5) explains that site too
  (lead: ../source pascal-translation-notes finding 22).  TPAD_$DATA's
  `if ... else if` clamp reads one field once for both tests (D3, the
  cursor line, `cmp.w (-0x14,A2),D3` 0xE696F6, no reload) and reloads
  the other (`move.w (0x162,A5),D3` at 0xE6970A and again at 0xE69722,
  on a path where D3 still holds it): not explained (finding 39).
- **Another unit's object's address held in a register across
  statements (Verified at the sites; rule Uncertain):** straight-line
  code, not a loop invariant.  LOG_$ADD `move.l #0xe0000c,D1`
  (LOG_$LAST_ENTRY, 0xE176F2), copied to A2 for three stores of
  separate statements (`movea.l D1,A2` 0xE176FA; stores 0xE176FC,
  0xE17704, 0xE17712) and
  to A4 for the loop (`movea.l D1,A4` 0xE17724); LOG_$INIT 0xE30048
  `movea.l #0xe0000c,A2` (0xE301BA), live across `jsr LOG_$ADD`
  (0xE301D4) and used after it (0xE301E6, `clr.l (A2)` 0xE301EC), then
  `movea.l #0xe00000,A2` (CRASH_$RECORD, 0xE301EE).  A `with` would
  explain the LOG_$INIT sites.  `dpc` addresses such fields as
  `(sym+d).l` in each statement.  How often the image does either is
  not surveyed (lead: ../source pascal-translation-notes finding 23;
  bead domain-pascal-sht0).
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
- The pad before the parameter load: AREA_$FREE_ASID 0xE07E8E; before
  the static link's load, 0xE13614.
- A zero-extension's `clr.l` in a load's stall: 0xE382BE.
- A base `movea.l #` inside a ×56: 0xE1AA5A (R10).

**R10. Barriers (Verified at the sites; the store rule supported by
survey).**
- Nothing moves across a call.
- A call's pad is not hoisted above the previous statement's store
  (0xE00C12/0xE00C16; 0xE0774C/0xE07750), although nothing in the
  registers stops it.
- **Stores** (bead domain-pascal-g5q.8): a load through a pointer is not
  hoisted above the store before it (0xE3F2DE `move.w (A0),(-0x12,A6)`,
  then 0xE3F2E2 `movea.l (0x10,A6),A1`, which nothing but the stores
  orders; 0xE16B26 `movea.l (0xc,A6),A0` after the two stores from
  0xE16B1A, although the `move.l (0xc,A3),...` right after `movea.l
  A0,A3` stalls).  The model "a store, other than a
  push, is ordered against every instruction of its block" covers these
  and the pad rule above, and halves the contradictions of every
  top-down rule in the survey of R11 (`tools/schedsurvey.py summary`,
  model `stores` against `alias`: 748 against 1592 for the rule `raw`).
  No site was found where an instruction crosses a store; the source
  order of the stores of XPD_$RESET_PTRACE_OPTS (0xE5B15E..0xE5B16C,
  offsets 0xC, 0, 0xD, 4, 8) is then the source's own (conformance unit
  `xpd_reset_ptrace_opts.pas`, Uncertain).
- Pushes are not interleaved with the loads of later arguments
  (0xE00D26 `move.w (-8,A4),-(SP)` before 0xE00D2A `movea.l
  (-0x10,A4),A0`, which nothing else orders).
- A call's pop stays right after it and the next call's pad right after
  the pop: `addq.w #8,SP; subq.l #2,SP; movea.l (8,A2),A0` at 0xE1AFC8,
  0xE1AFDA, 0xE1AFEE, 0xE1BD24, 0xE1BD36 (11 `pop; pad; non-push` in
  the image, 311 `pop; pad; push`).
- A nested routine's static link is loaded into A1 right before its
  `bsr` at all 34 calls of the 11 nested routines that take it in A1
  (`tools/nestsurvey.py list`; section 6): 33 `movea.l An,A1`, one
  `movea.l (-0x80,A6),A1` (0xE4EE94).  The other `movea.l A6,A1` of
  the image (e.g. 0xE133A6, 0xE6B992) form addresses and are scheduled
  like any instruction.
- Shift-and-add multiplications and `(An)+` block copies: the ×12 at
  0xE0A972..0xE0A97E is not interleaved although an independent
  `movea.l #` was ready; nor 0xE190D2..0xE190DC, 0xE14A2E..0xE14A3A
  (inside an argument list) and the copy 0xE04ADA..0xE04AE2 (stores).
  **But** the ×56 of 0xE1AA50..0xE1AA60 has a `movea.l #,A1` before its
  last `add.w` (`move.w D2,D3; lsl.w #3,D3; move.w D3,D0; neg.w D3;
  lsl.w #3,D0; movea.l #0xE2DC90,A1; add.w D0,D3`).  Both are what R11
  with a window of 3 gives (below); a multiplication is not a unit.
- The SR instructions (`disable`, `enable`, `set_sr`): section 10.

**R11. The ordering policy (Uncertain; a candidate implemented).**
- *The rule.*  Top-down list scheduling: take the first instruction, in
  the order the compiler generated them, whose predecessors are placed
  and that does not read a register written by the instruction just
  placed; if there is none, the first whose predecessors are placed.
  Only the first *W* instructions not yet placed are candidates (a
  window).
- *Survey* (`tools/schedsurvey.py summary`, 1099 routines of
  0x30..0x150 bytes): for each adjacent pair where the second reads a
  register the first writes, is there another instruction that was ready
  there and would not have?  If so the unwindowed rule is contradicted
  whatever the generation order.  Model `stores` (R10): 748 of 11586
  pairs (A7 counted; 581 of 7045 without it).  Bottom-up (from the end,
  the same test mirrored): 1381.  Other "interlocks" (an address
  register used to form an address: 256 of 5833; two memory accesses in
  a row: 677 top-down, 308 bottom-up of 9600) are contradicted too.  So
  no rule of this family without a window explains the whole image.  Most
  contradictions have a `movea.l #`, `lea` or `moveq` far ahead (e.g.
  0xE74C82, 0xE5C0C8, 0xE0CA0A: an array base loaded only right before
  its use, after a scaled index, while it was ready from the start).
- *The window.*  With W = 3 (or 4), R11 reproduces, from the generation
  orders given:
  - 0xE177A6..0xE177B8 from `dpc`'s own order (golden
    `regalloc/cse`), and with the register-held constant generated after
    the `add`;
  - 0xE177CC..0xE177DE with `movea.l (8,A6),A2` (`buffer`) first (a
    loop invariant hoisted to the block before the loop, ref p. 6-29;
    Uncertain).  Needs W >= 3;
  - FIM_$ACKNOWLEDGE 0xE0A972..0xE0A994 with each array reference's
    index before both bases (`×12`, the second index load and shift,
    `movea.l #` ×2, the move): the window keeps the first base out of the
    ×12, which the unwindowed rule pulls in.  Needs W <= 4;
  - the ×56 of 0xE1AA50..0xE1AA60 (R10), with its base generated after
    it;
  - 0xE0102C..0xE01034 with the zero-extended operand first;
  - 0xE382B4..0xE382C4: the `clr.l D2` of a zero-extension in the stall
    after the load, before `divu.w` (dpc's order; `crates/dpc-m68k/
    tests/expr.rs`, `division_forms`);
  - MST_$VA_TO_SEGNO 0xE0DCD6..0xE0DCEA, but only if `(0x10,A6)` is
    generated after the shift statement (`va_to_segno := default_result`
    second), not with the entry loads in parameter order.
- *Not explained: entry loads interleaved with the first statement*
  (12.2).  PARITY_$CHK_IO 0xE0B180 (load, `movea.l #`, load),
  module-local 0xE758C4, NET_IO_$BOOT_DEVICE 0xE31C1A likewise;
  PMAP_$PURGE_WS 0xE146C2 `move.w (8,A6),D2; subq.l #2,SP; move.b
  (0xa,A6),D3; move.w #20,-(SP)`; AREA_$FREE_ASID 0xE07E8E `subq.l
  #2,SP; move.w (8,A6),D2; move.w #14,-(SP)`; 0xE13614 `subq.l #2,SP;
  movea.l (A6),A4; move.w #20,-(SP)`.  PMAP_$PURGE_WS and
  AREA_$FREE_ASID have the same first block (entry loads, then
  `ML_$LOCK(const)`, the same prologue shape) with two and one loads;
  with the loads generated first, in parameter order, no rule that looks
  only at the instruction just placed and at the generation order, top-
  down or bottom-up, gives both.  Bottom-up R11 (latest generated first)
  gives PARITY_$CHK_IO, PMAP_$PURGE_WS, AREA_$FREE_ASID and 0xE13614, but
  not MST_$VA_TO_SEGNO, and is contradicted more often in the survey.  A
  scratch experiment over the 924 first blocks that load parameters
  (loads first in parameter order, the rest in image order; top-down and
  bottom-up; interlocks `raw`, `mem`, `rawmem`; W = 2, 3, all) found no
  variant that reproduces more than 15 of the 263 blocks the order needs
  to change.
- *Alternating memory accesses* (a candidate of the earlier review) fits
  PARITY_$CHK_IO, 0xE758C4 and PMAP_$PURGE_WS, but not AREA_$FREE_ASID.
- *What `dpc`'s units say* (bead domain-pascal-g5q.8): little.  Under
  R11, the selector's order of all goldens and conformance units has two
  places where an instruction is moved (both in golden `regalloc/cse`,
  0xE177A6 order, toward the image); variants that pick the ready
  instruction with the longest path to the end, or loads first, or
  non-memory instructions first, change no conformance unit either;
  picking the last ready instruction breaks 13 routines.
- **Implemented** (`crates/dpc-m68k/src/sched.rs`; `dpc
  --schedule=off|r11|r11:N`, default `r11:3`): R11 with a window of 3,
  after allocation, over runs between labels, branches and barriers, with
  the dependences of R10 (registers; memory, a store ordered against
  everything, pushes against all memory; the flags, with their liveness
  over the body's control flow; calls, SR instructions, writes of A5;
  pops; static links).  The default changes no conformance unit; it
  changes golden `regalloc/cse` (0xE177A6) and `division_forms`
  (0xE382BE), both toward the image.  Unrolled `(An)+` copies are units
  of the scheduler (stores anyway); multiplications are not (0xE1AA5A).

**Stores whose destination and value both need a register (survey
supported, rule Uncertain; implemented, bead domain-pascal-k1sv).**
- *Survey* (`tools/storesurvey.py summary`): stores `move.<s>
  SRC,DST` to memory (not pushes) where both sides read registers set up
  in the store's statement (its block, back to the previous store or
  call, R10; A5/A6/A7 and registers held from earlier statements set
  nothing up), the two setups disjoint, no call between: 210.  The side
  with more set-up instructions comes first at 116 of the 126 where they
  differ; of the 84 ties the value (SRC) comes first at 71.  The rule
  "more instructions first, the value on a tie" fits 187 of 210.  Most
  of the 13 ties set up destination first are a destination that is a
  copy of a held pointer or a D0-parked one (`movea.l D0,A0`, TTY_$K_
  SET_SESSION_ID 0xE679B0; `movea.l A2,A0`, XNS_ERROR_$SEND 0xE17B90) or
  an entry load of a callee-saved register (TIME_$Q_ADD_CALLBACK
  0xE16DF0, 0xE32A68): dpc places those outside the statement (the park
  move at the statement's start, entry loads in the prologue), so they
  do not count there.  Of the 10 the size predicts wrongly, two are a
  zero-extension's `clr.l` scheduled up (MMAP_$FREE_PAGES 0xE0CEF4,
  MMAP_$GET_IMPURE 0xE0D696, R9); the rest (0xE02B6C, 0xE334F0,
  0xE3BDD6, 0xE3DD66, 0xE508B4, 0xE55558, 0xE6AA2A, 0xE716B8) are not
  explained (bead domain-pascal-z4gy).  Where the destination's set-up
  is the larger, the only case `dpc` reorders, the rule fits 22 of 29.
- *The TTY sites*: TTY_$K_SET_DELAY 0xE67A40 `movea.l D0,A0; move.w
  (A2),D1; movea.l (0x10,A6),A1; add.w D1,D1; move.w (A1),(0x40,A0,D1.w)`
  (the element, the destination, first; the value's slot load scheduled
  into the index's stall, R11); TTY_$K_INQ_DELAY 0xE67A96 the same with
  the element as the value; TTY_$K_GET 0xE1C4D6 `lea (0,A2,D2.w),A0;
  movea.l (-4,A6),A1; move.b (0x2d1,A0),(A1)` (a tie, the value first).
- *Implemented* (`isel.rs`, `store`): the value's instructions are
  generated, then the destination's; when the destination's are more,
  they are moved before the value's (refused if they read a register the
  value's wrote).  A call on either side is refused (the survey leaves
  calls out).  Bit fields of packed records (5.3) stay refused.  Tests:
  `tests/golden/stores/both_regs`, `tests/exec/store_order.pas`.
  TTY_$K_SET_DELAY and TTY_$K_INQ_DELAY compile byte-identical in
  `make srcsurvey`.

**R12. Registers are chosen before scheduling (Uncertain; supported;
implemented: the scheduler runs after allocation).**
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
| 1 | Value-based allocation (R1–R3) in place of pinned parameters: the entry loads of 12.2 stay; a parameter's register is released after its last use; each new value takes the lowest register free over its whole live range; no coalescing of copies and operator results | 0xE17780..0xE17788, 0xE177A0, 0xE177A6..0xE177AE, 0xE177D2, 0xE177DA..0xE177E4 | R1–R3 Verified as properties of the output; the order of allocation is not (`colQU` 73%, R1).  **Implemented** (bead domain-pascal-g5q.5, `crates/dpc-m68k/src/regs.rs`): address order of first definition; R2 for register operands and `lea` bases, not for loads through a dying base; M$ARITH calls write only D0/A0; R4(c) for parameters a call's arguments read last, in bodies without branches, and with branches (bead domain-pascal-9j1z) unless the call lies on every path after a join.  New units: module-local 0xE32B76, PROC1_$SET_ASID |
| 2 | Global CSE of operator results (here the zero-extension), with the two-address result in the non-CSE operand | 0xE177A6, 0xE177B4, 0xE177D0 | Verified at the sites; kills by survey (R7).  **Implemented** (bead domain-pascal-g5q.7): widenings, scaled indices and constant products of plain variables, no reuse across calls nor into or out of a loop (past one: yes); golden `regalloc/cse` |
| 3 | `max_len & 16#FFFE` as `moveq #-2,D2; and.w D1,D2` (constant into the result register first) | 0xE177A0 | Same order at 0xE60F4A, 0xE60F52 (R8).  **Implemented** (section 8, `&`; bead domain-pascal-d7i) |
| 4 | The 32-bit compare against a constant held in a register | 0xE177AE | Speculative (R8: 11 sites) |
| 5 | D3 at 0xE177C4 and 0xE177D0 | 0xE177C4, 0xE177D0 | Speculative (R6) |
| 6 | The `for` loop as strength-reduced pointers (12.3 item 6) | 0xE177DC..0xE177F2 | Shape Verified.  **Implemented** (bead domain-pascal-5aw.17, section 7.4) except the run-time count kept in place without a call (no rule) |
| 7 | A block scheduler run after register choice, with the barriers of R10 | 0xE177A6..0xE177B8, 0xE177CC..0xE177DE | Exists: Verified; policy Uncertain (R11).  **Implemented** (bead domain-pascal-g5q.8, `crates/dpc-m68k/src/sched.rs`): R11 with a window of 3, default on; 0xE177A6..0xE177AC reproduced (golden `regalloc/cse`); 0xE177CC needs `buffer`'s load first in the selector's order |

**Consistency with the units `dpc` already reproduces** (review,
2026-10-12; `tests/conformance/manifest.txt`).  Every value in
PROC1_$GET_LOCKS, TIME_$Q_INIT_QUEUE, TIME_$Q_FLUSH_QUEUE, module-local
0xE00EB0, AST_$PAGE_ZERO, PROC1_$GET_LOADAV, PROC2_$GET_BOOT_FLAGS and
NAME_CONVERT_ACL_STATUS is predicted by both `colU` and `colQU`.  Two
points need care:

- PROC1_$TST_LOCK 0xE148DE `sne D1`: D0 (the element, dead after the
  `btst`) is free.  Address-order allocation of a fresh value gives D0.
  `dpc` puts the `sne` in the tested operand's register
  (`set_member_reg`), which is what the image shows; keep that as an
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

### 12.5 Local variables held in registers (rule Verified on the TTY Pascal, 41 of 43 locals; supported by the C corpus; implemented)

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
D2 at PROC1_$SET_VT is assigned in an `if` too).  The rule is in "The
rule" below (bead domain-pascal-iyd); the first survey follows.

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
  0xE1A0D6), so "passed by value means frame" is not absolute either
  (the one such exception of the corpus below).  Pointer locals are
  often in address registers (MMAP_$FREE's `page` in A2 from 0xE0CADE;
  KBD_$PUT's descriptor in A2 from 0xE1CEC6); `dpc` holds those
  assigned from pointer functions in an address register (section 3,
  "Pointer locals"; bead domain-pascal-500) and those of loops (below);
  any other pointer local that is dereferenced stays in the frame.
  That is a known divergence (bead domain-pascal-dgb review): an ad-hoc
  count over compiled code finds 186 `movea.l <memory>,An` (A2..A4, not
  a parameter slot, not a call's A0) whose register is dereferenced
  after a later call, against 26 longwords stored from memory into a
  frame slot and reloaded later as a base (`movea.l (-k,A6),An`, then
  `(An)`).  The first kind mixes locals with common subexpressions held
  across calls, so it is no rule, but some are pointer locals: AST_$ADD_AOTES
  0xE010C0 `movea.l (1012,A5),A2`, the C's `aote = AST_$AOTE_LIMIT`
  (`ast/add_aotes.c`, "A2"), MMAP_$FREE's `page` and KBD_$PUT's
  descriptor above.  Which pointer locals assigned from memory the
  original keeps in a register is not settled (Uncertain).
- Expression temporaries live across a call are spilled to the frame
  (SMD_$BLT_U 0xE6FB1E; module-local 0xE5AA28 at 0xE5AA6C, across an
  M$ARITH call); not this rule's business.

That first survey led to the narrow rule of bead domain-pascal-0wc
(assigned once, at the top level, never read inside a call argument),
fitted to two routines.  The corpus below replaces it.

**The corpus (bead domain-pascal-iyd; `tools/localsurvey.py`, kept, with
tests).**  Every scalar local whose source is known, with the facts of
each (`dpc --dump-locals`, `dpc_m68k::locals`): its assignments and
reads, each at the routine level or inside an `if`/loop/`case`, a read
inside a call's argument and whether the argument *is* the local (it is
pushed as it is), inside a condition, whether a call lies between an
assignment and a read; and whether the image addresses its frame slot
(in the frame) or not (in a register; the slot stays allocated).  Only
"plain" locals are scored: read and assigned as a whole, not
dereferenced, address never passed or taken, not a `for` index.

- *TTY Pascal* (`localsurvey.py run --rev 3898a2ce`): ../source's
  `tty/*.pas`, written from the C and checked against the image, the
  closest thing to the original source we have.  56 routines lower; 43
  plain locals in routines with a header address (30 in a register, 13 in
  the frame; I_SIGNAL's and I_STORE_PARITY's not placed: dpc's frame
  differs or no address).
- *C corpus* (`localsurvey.py c --rev 3898a2ce`): ../source's C, parsed
  with clang (the host build's flags), every scalar local whose
  declaration carries a register or frame note (`/* D3 */`, `/*
  A6-0x14 */`), the frame ones checked against the image when the map
  names the function.  status_$t locals are left out (a Pascal record,
  base.ins.pas), calls of M$ARITH and of ../source's ARCH_ helpers count
  as operators.  A C local is not always a local of the original (a
  decompiler names register values too), so this is weaker evidence.

| Rule ("held in a register when ...") | TTY (43) | C, data locals, routines saving at most 4 of D2-D7 (459) |
|---|---|---|
| narrow rule of bead 0wc | 17 | 215 |
| never read inside a call argument | 38 | 351 |
| ... and not live across a call | 24 | 251 |
| every plain local | 30 | 346 |
| **unless every read pushes it as an argument** | **41** | **408** |
| unless some read pushes it | 39 | 360 |

C data locals by the number of D2-D7 the routine saves, register/frame
(`localsurvey.py c`, first table): a local some read uses otherwise 67/4,
60/5, 82/7, 66/10 for 0..3 saved, 67/21 for 4, 110/55 and 193/107 for 5
and 6; a local every read pushes 2/20, 0/13, 2/14, 0/9, 0/10, 1/25,
0/34.  So the frame is the rule for the second kind at any pressure, and
the first kind spills more the more registers the routine uses: under
pressure the original keeps some register candidates in the frame by a
priority not found.

**The rule (Verified on the TTY corpus; the C corpus agrees, 89% of the
459 and 92% where the routine saves at most 3).**  A plain local
assigned at least once is held in a data register for the whole body,
unless every read of it is a call argument that pushes it as it is.
Examples:

- in a register, assigned in both arms of an `if`, one a constant:
  TTY_$I_BUF_INSERT's `new_tail` D1 (0xE1AF20 `moveq #1,D1`; 0xE1AF24
  `move.w (2,A0),D1; addq.w #1,D1`), TTY_$I_BUF_PUT's D0 (0xE1AF6C);
  assigned in a loop and read in conditions and arguments,
  TTY_$I_PUT_CHARS' `ch` D5 and `chars_processed` D2, TTY_$K_GET's
  `chars_read` D3 and `read_pos` D2; a `case` index used only inside an
  argument's address, TTY_$I_SIGNAL's `signal_index` D2 (0xE1B858..);
  booleans, TTY_$I_NEWLINE (`st D0`, `clr.b D2`), TTY_$K_GET's flags;
  a set, TTY_$I_RCV's `state_flags` D4; a function's result,
  TTY_$I_WAIT's `result` D2 (0xE1C368 `move.w D0,D2`);
- in the frame: every spin-lock token of TTY (8, e.g. TTY_$I_BUF_PUT
  0xE1AF60 `move.w D0,(-6,A6)`, pushed at 0xE1AF88), TTY_$K_GET's
  `wait_flag`, TTY_$I_WAIT's `num_ecs`, TTY_$K_RESET's `xon_xoff`, all
  read only as pushed arguments; the earlier survey's MMAP_$FREE,
  SIO2681_$SET_BREAK, PROC1_$TS_END_CALLBACK sites.

Exceptions: TTY_$I_PUT_CHARS' `col_mod` (frame, 0xE1B1DA `move.w
D4,(-0x1e,A6)`, then 0xE1B1E0 `sub.w (-0x1e,A6),D4`, with D6 and D7 free)
and TTY_$K_GET's `done` (frame, 0xE1C426, D7 free) are not explained;
UID_$GEN's `token` (D3, pushed only, in a loop) is the one C exception
not explained by an operator call or a pointer.  How in-place updates
look: `x := x op c` and `x := x op y` operate on the local's register
(TTY_$I_PUT_CHARS 0xE1B04A `addi.w #256,D1`, 0xE1B04E `sub.w D0,D1`,
0xE1B07E `addq.w #1,D2`); other values are computed into it
(0xE1B03E..0xE1B046 `move.w (978,A0),D1; sub.w (980,A0),D1; subq.w
#1,D1`).  A constant `moveq` cannot load as the value written (16#FFFF in
a 0..65535 word) is `move.w #-1,Dn` (0xE149A4; 12 such `move.w` of a
negative byte-sized word in the image against 57 negative `moveq`s,
`moveq` 1978 times for positive ones).

**A stored value is not pushed from its register after a register-held
local took it (TTY_$I_BUF_PUT).**  The token stored from D0 at 0xE1AF60
is pushed from its slot at 0xE1AF88, with no call between: `new_tail`
took D0 at 0xE1AF6C.  The original gives variables their registers
before it tracks the values its registers hold.

**Implemented** (`dpc-m68k` `isel.rs`, `register_locals`,
`Selector::store_web`; `crate::locals` for the facts; beads
domain-pascal-0wc, -iyd).  The rule above, over `Function::scalar_locals`
(1, 2 or 4 bytes of an ordinal, set or pointer type: not a record or an
array, as the image keeps `status_$t` locals in the frame with registers
to spare, MST_$ALLOC_ASID 0xE42DB2, FLP_$CINIT 0xE3E0D6; not volatile;
not reached by a nested routine, which
reads and writes them through the frame: `dpc-ir` `scalar_locals` with
`Function::uplevel_refs`, bead domain-pascal-thc; PMAP_$FLUSH keeps such a
word in its frame and the boolean no nested routine reads in D3, section
6; not passed or taken by address, bead domain-pascal-pe9, below).  A
local of the narrow rule (assigned once, by a routine-level statement,
with a non-constant value, read only by later routine-level statements)
is a data value as before: the assignment computes or loads it into a
new virtual register (`move.l (abs).l,Dn`, the `move.w SR,Dn` of
`set_sr`), which its reads use (TIME_$GET_ADJUST D1 at 0xE16AB4).  Every
other register-held local is a *web*: one virtual data register for the
whole body, which every assignment writes: `x := x op c` and `x := x op
y` in place (`addq`/`subq`/`addi`/`subi`/`andi`/`ori`/`eori`, or the
operation with `y`), `true` as `st`, other constants as `moveq`/`clr`/
`move` (one `moveq` cannot tell from `move.w #-1` is refused), other
values computed into it (the computation's register renamed), or, when
the computation reads the local after starting its result, computed
apart and moved (seen, below); an M$ARITH result, in D0 from the `jsr`,
moved (`move.l D0,Dn`; for a local of the narrow rule too, which D0
itself would not keep past the next call; not seen in the image).
Computed in another register and moved (the moves Verified at the
sites; that their destinations are register-held locals is ../source's
reading, Reported; lead: ../source pascal-translation-notes finding 39;
bead domain-pascal-er7r):
TPAD_$DATA 0xE692CC `move.w D0,D1`, where D1 held a subterm (the
product `muls.w (-0x24,A2),D1`, 0xE692B4) that the computation, started
in D0, reads (`sub.l D1,D0`): close to the case above, the conflict
being with a subterm in the local's register rather than with the
local's old value.  Also 0xE6920E `move.w
D2,D3` (D3 used as a temporary in the sum) and 0xE6960C/0xE6961A
`move.w D5,D2` / `move.w D5,D1` (computed in D5), so moves occur
without the read conflict too.  Which locals the registers hold is
../source's reading.  A
web read before the code first assigns it (a read the back edge of a
loop reaches, the assignment later in the body; or a local not yet
assigned) is refused: where the original colours such a value is not
known.  A computed boolean (`x := a = b`) is refused
as for memory (section 7.1).  The allocator gives the register like any
other value (12.4 R1-R3: a callee-saved one across calls; a body that
runs out of registers is refused, where the original would spill by its
unknown priority).  Assignments to register-held locals are not memory
stores for the check of adjacent stores (ref 3.15.3, p. 3-57; `isel.rs`
`reject_mergeable_stores`), and they end the
pushing of a stored local from its register (`Selector::forwarded`,
TTY_$I_BUF_PUT above).  A local every read of which pushes it stays in
its slot (with the forwarding below).  The frame slot of every local
stays allocated (`dpc-sema` frame size).  Not reproduced: the two
exceptions above (in a register in `dpc`), register pressure (refused
when it runs out; the original spills earlier, at four or more of D2-D7
in the C corpus), TTY_$I_NEWLINE's `send_cr := true` as `move.b D0,D2`
(a copy of the register already holding the constant) and its branch
threading.  Unit tests `register_local_rule`, `body_uses_facts`; driver
tests `register_held_locals`, `dump_locals_prints_the_uses_of_scalar_locals`,
`marith_result_into_register_held_local`,
`register_held_local_read_before_assigned_is_refused`;
golden `tests/golden/sysprog/interrupts` (`held_sr`); conformance
`tests/conformance/time_get_adjust.pas`, `tty_buf.pas` (TTY_$I_BUF_INSERT
and TTY_$I_BUF_PUT, new; srcsurvey of ../source 3898a2ce: 19 of 26 TTY
routines identical, was 17).  The tests of section 7.1's store flags
(golden `control/store_flags`, driver tests
`branch_on_the_store_flags_only_right_after_the_store`,
`ordered_branch_on_a_store_is_refused`) pass their locals to a `var`
parameter now, which keeps them in the frame; the image's frame locals
of that shape are a status record (WIN_$SPIN_DOWN 0xE19BE6) or in
routines that save D2-D6 or more (0xE55480, 0xE6B180, 0xE4F09C).

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

**Stored locals pushed from the register (Verified by survey;
implemented narrowly; bead domain-pascal-5aw.11).**  A local stored from
a data register and later pushed by value as an argument is pushed from
that register when no call came between (`tools/resultsurvey.py
pushes`; the tool gives up at a write of the register or the slot, the
slot's address taken, a backward branch, or a label a branch from before
the store targets): from the register 86 times (50 stores of a call
result, e.g. the spin-lock token of PKT_$NEXT_ID, `move.w D0,(-2,A6)` at
0xE124A8 and `move.w D0,-(SP)` at 0xE124C4 across an `if`; NETBUF_$GETVA
0xE0ECAC), from the slot 5 times (0xE02D6E, 0xE141CA, 0xE611E2,
0xE6B828, 0xE771F4; not explained).  With a call between, from the slot
38 times (NETBUF_$RTNVA: stored at 0xE0ED40, pushed at 0xE0ED52 after
the `bsr`), from the register 10 times (a callee-saved one).  A reload
into a register to compute with is another matter: `move.w
(-0xa6,A6),D3` at 0xE085E8 after `move.w D1,(-0xa6,A6)` at 0xE085DA,
D1 unchanged, then `lsr.w #3,D3` (also 0xE30D5A, 0xE871DE).
**Implemented** (`isel.rs`, `Selector::forwarded`, used by `push`): a
by-value argument that is a local of the routine (not held in a register
by the rules above) is pushed from the register a store of the body put
into it, when no instruction between the store and the push is a call,
writes that register or the slot, writes the frame at a run-time
index (another element of the same array), writes through a pointer
while some frame object's address has escaped, or takes a frame
address, no label
between them is the target of a branch or `case` table from before the
store, neither is inside a loop, and the local's address is never passed
or taken (`escaped_frame_objects`).  Driver test
`stored_local_is_pushed_from_its_register_only_while_valid`,
`stored_local_not_forwarded_past_a_possible_overwrite`; conformance
`tests/conformance/pkt_next_id.pas`.  Other reads of a stored local load
it from its slot, as before.

**Address-taken locals (audit, bead domain-pascal-pe9).**  Neither rule
holds a local whose address the body passes or takes: an argument passed
by address (any mode) or `addr` makes the reference "other" than a whole
read or assignment (`register_locals`), or an addressed reference
(`loop_pointer_locals`, which also excludes bodies with calls); a `for`
index that is addressed is refused (`dpc-ir` `check_for_indices`).  So
such a local is read from its slot after the call (driver test
`local_passed_to_var_is_reread`).  For parameters see 12.2.

---

## 13. Instruction encodings (Verified, matters for byte comparison)

Counts below are from a sweep of the `.TEXT` range 0xE38000..0xE71D7C.

- Immediate compares and arithmetic use the **CMPI/ADDI/SUBI/ANDI** forms
  (`0c40` cmpi.w ×996, `0c80` cmpi.l ×355, `0240` andi.w, `0680` addi.l, …),
  which is what gas emits by default.
- **Exceptions:**
  - `sub.l #imm,Dn` with the immediate-EA encoding `90bc`, seen 6 times
    (e.g. 0xE40188).  Over all compiled code (`tools/storesurvey.py
    addr`): `sub.l #` 10, `add.l #` (`d0bc`) 3, `cmp.l #` (`b0bc`) 3,
    `cmpa.l #` 1, every immediate a map symbol's address, and no
    `addi`/`subi`/`cmpi` of one: the form of an address operand
    (section 5.2, "`addr(x)` as a value"; implemented for `add`/`sub` as
    `.short` plus a `.long` of the symbol).
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
  - **`nil`** (Verified at the sites; implemented, `dpc-ir`
    `stored_const`; bead domain-pascal-j4ip): the pointer 0, stored like
    any longword 0.  `tools/nilsurvey.py store`: 1160 longword zeros
    stored to memory in compiled code, every one `clr.l` (through an
    address register 785, frame 231, `(An)+` 54, A5 40, indexed 25,
    absolute 25), no `move.l #0,<ea>`.  Sites the ../source translations
    write as `nil`: BAT_$ALLOC_VTOCE 0xE3AEDA `clr.l (-0x14,A6)` (a
    frame local), BAT_$ALLOCATE 0xE3B2EE and BAT_$DISMOUNT 0xE3B90A
    `clr.l (0xd38,A5)`, HINT_$INIT 0xE3139A and HINT_$SHUTDN 0xE49924
    `clr.l abs.l`, PKT_$SEND_INTERNET 0xE12670 `clr.l (-0x10,A6)`,
    MAC_OS_$CLOSE 0xE0B506 `clr.l (0x7a4,A2); clr.l (0x7a0,A2)` (two
    adjacent longwords, a routine pointer among them, not merged); a
    local held in a data register, IO_$GET_DCTE 0xE1A466 `clr.l D2`.
    Golden `stores/nil_stores`, exec test `nil_stores`.  The whole image
    has no `move.l #0,<ea>` to memory (a sweep of every even offset of
    0xE00800..0xE88834 for the opcode words: only pushes, and two
    decodes of FIM_WIRED data), so a longword zero to a `device` object,
    which `dpc` stores with `move.l #0` (section 5.5), has no site.  An
    odd-address `clr.l` cannot arise: a 4-byte field may not cross two
    word boundaries (3.10.5.5 p. 3-26), so `[aligned(0)]` moves it to an
    even offset (section 5.4).  Refused: `nil` into an address register
    (`suba.l An,An` 24 sites, e.g. ROUTE_$FIND_PORTP 0xE15B84,
    DBUF_$GET_BLOCK 0xE3A832; of the 17 `movea.l #0,An`, 15 are
    relocated addresses of a symbol at 0, section 1.5 "Zero", and the
    other two end TIME_$RTE_INT 0xE163D6 and TIME_$VT_INT 0xE16426, whose
    A0 no caller reads, section 3; `tools/nilsurvey.py areg`) and `nil`
    as a pointer function's result (`suba.l A0,A0` before the epilogue,
    ROUTE_$FIND_PORTP; the result slot cleared and reloaded,
    NET_$FIND_HANDLER 0xE5A178), bead domain-pascal-bdi0; `nil` passed
    by address (no site known).  `nil` passed by value is `clr.l -(SP)`
    (section 1.5, "Zero"; implemented, bead domain-pascal-73sx);
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
    is not known.  Implemented for `device` objects (section 5.5): `move
    #0`, and `move.b #0xff` for `true` (`st` reads its operand too);
    0xE0A328 (five `move.b #n` stores to `device` registers, among them
    0xE0A33E `move.b #-1,(A0)`) is a byte-identical conformance unit.
- **Store merging (reference; merges and non-merges Verified by survey;
  which constant pairs merge Uncertain; non-candidates implemented, bead
  domain-pascal-2m0):** ref 3.15.3, p. 3-57: "By default, the compiler
  optimizes certain adjacent references by merging them into one large
  reference" (`a := 0; b := 0` on two `integer16`s becomes one 32-bit
  store); ref 6.4.24, p. 6-28 lists "Merges assignment statements where
  possible" among the `-opt 1` optimizations (the default is `-opt 3`),
  without saying when it is possible.
  - **Survey** (`tools/storesurvey.py merge [-v]`): in the 1635 compiled
    routines of `tools/regalloc.py`, 420 pairs of stores made one after
    the other (only register set-up between them: no store, call, branch
    or branch target, the base register not reloaded) to adjacent memory
    of one base, of 2 or 4 bytes together, all *unmerged*:
    - 118 of two constants with the low address even: 1+1 bytes 35 in the
      frame, 24 through an address register, 2 off A5, 1 absolute; 2+2
      words 22 through an address register, 13 in the frame, 13 absolute
      (MST_UNWIRED 0xE30A04.., a descending run), 8 off A5.  Ascending
      and descending, at 0 and 2 mod 4, zero and non-zero alike, through
      pointer locals and parameters as well as statics: TTY_$I_INIT
      0xE3332E..0xE33352 (A4 = (8,A6), the buffer stores TTY_$K_RESET
      merges), AREA_$INIT 0xE2F4A4 (`clr.w (8,A0); move.w #-1,(0xa,A0)`,
      an element through a cursor), XNS_IDP_$PROC2_CLEANUP 0xE18F8C,
      TPAD_$INIT 0xE3363E (TPAD's block through `movea.l #`),
      TPAD_$RE_RANGE_UNIT 0xE69A68 (a `with` element), TPAD_$SET_UNIT_
      CURSOR 0xE69902 and NETLOG_$CNTL 0xE719A4 (off A5), ASKNODE_$SERVER
      0xE65A3E (two frame words), PKT_$BLD_INTERNET_HDR 0xE1217E (bytes
      through a by-address parameter); RING 0xE2FAC4 `move.w #0` is the
      `device` zero above;
    - 19 copies of adjacent memory to adjacent memory in the same
      arrangement (0xE31EA2 `move.w (0x800,A1),(A0); move.w
      (0x802,A1),(2,A0)`, 0xE3C344);
    - 270 whose values are not both constants and not such a copy: a
      computed or register value, a load from elsewhere, a constant next
      to either (TPAD_$SET_UNIT_MODE 0xE69830..0xE69840, three words from
      three by-address parameters; 0xE02D40 `move.w D2,(-0x4e,A6); move.w
      D3,(-0x50,A6)`);
    - 13 byte pairs whose lower byte is at an odd displacement: a word
      there is an odd-address reference, which the 68010 traps.  One is
      absolute (MST_$DISKLESS_INIT 0xE30DD2, 0xE24353), one in the frame
      (0xE62A42, (-0xf7,A6)), 11 through an address register, whose
      parity is the displacement's only if the pointer is even (0xE04CAC,
      0xE19F1A).  None is in A5's data: every `lea abs.l,A5` target (867)
      and every `D` segment is 4-aligned (section 11).
    - Apart from these, pairs of a word and a longword (6 bytes; `merge`
      prints them on their own line): 363, 66 of two constants, 13 of
      them with the word at the low address (FIM's 0xE0A896 below would be
      one if not merged): 0xE14CBC `clr.w (-0x14,A6); move.l
      #1250000,(-0x12,A6)`, 0xE1465E, 0xE2F8BA, 0xE71AC6 (off A5), 0xE2FF2C
      (absolute), 0xE0C958 (descending).  The other 53 have the longword
      at the low address, as a merge split at the other boundary would.
  - **Merges** (`move.l #imm` / `move.w #imm` candidates with both halves
    non-zero, 1183, less the 964 longwords that are status codes of the
    stcode database, read against ../source's C; a `clr.l` of two words
    cannot be told from a longword clear and is known only from the C):
    TTY_$K_RESET 0xE67322 `move.l #0x10001,(0x2ca,A2)`, 0xE6732A `move.l
    #0x10100,(0x2ce,A2)`, 0xE67332 `move.l #0x10001,(0x3d2,A2)` (then the
    lone `move.w #0x100,(0x3d6,A2)`, 0xE6733A) and 0xE67340 `clr.l
    (0x56,A2)`, A2 the pointer TTY_$I_GET_DESC returns; the module-local
    0xE1BF70 at 0xE1BFE4 `move.l #0x10001,(0x38,A2)` (A2 = (8,A6));
    LOG_$ADD 0xE176B0 `move.l #0x10001,(A0)`; LOG_$INIT 0xE3017C `moveq
    #1,D0; move.l D0,(A0)`; KBD_$INIT 0xE33374 `clr.l (0x58,A0)` (A0 =
    `movea.l #` of its block), 0xE3337C `clr.l (0x3a,A2)` between the
    unmerged `clr.w (0x38,A2)` and `clr.w (0x3e,A2)`, 0xE333B6/0xE333C4
    `move.l #0x10001,(88,A2)` and `(158,A2)`, each followed by an
    unmerged `move.w #64`; PROC1_$BIND 0xE14DA6 `move.l
    #0x10010,(0x56,A2)` (inh_count, sw_bsr in ../source) and 0xE14DD8
    `move.w #0xa,(0x54,A2)` (the bytes pri_min, pri_max); PROC1_$INIT
    0xE2F998 the same longword; ASKNODE_$PROPAGATE_WHO 0xE65EBC `move.l
    #0x100be,(-0x21c,A6)` (two words of a frame record, then the lone
    `clr.w (-0x218,A6)`) and 0xE65ED0 `move.l #0x30046,(-0x200,A6)`;
    0xE65ED8 `clr.l (-0x1fc,A6)` (four words of a frame array, in pairs).
    MAC_OS_$INIT 0xE2F590 `move.l #0x10001,(4,A0)` may be one (../source
    reads one longword).  A word and a longword of constants merged and
    split again at the other boundary (Reported, ../source
    pascal-translation-notes finding 68, fim/build_df.pas): FIM_$BUILD_DF
    0xE0A896 `clr.l (0x54,A2); clr.w (0x58,A2)` for `sr := []; pc := 0`
    of a record whose word sr is at 0x54 and longword pc at 0x56 (the
    other arm stores them `move.w D1,(0x54,A2); move.l (-0x3e,A6),
    (0x56,A2)`, 0xE0A87A).  Every merge the image shows by itself is of
    two constants.  Merged copies cannot be told from longword copies
    there: ../source reads SOCK_$GET 0xE16108 `move.l (0x3e8,A1),
    (0x2a,A2)` and SOCK_$PUT_INT_INT 0xE1628A as two word assignments
    merged, since MAC_$DEMUX fills the record's two words apart (0xE0BCB6,
    0xE0BCBC; pascal-translation-notes finding 75), or one longword field
    (sock/get.pas, Uncertain).
  - **No merge of other values found** (a review scan of the compiled
    routines, not kept as a tool): a merged pair with a computed value
    would need the two values put together in a register before one
    store.  None found: the 46 `swap`s take remainders of `divu`/`divs`,
    take high halves apart or push them, multiply by 0x10000 (`swap;
    clr.w`), or store a longword's two halves apart (0xE70716); the 14
    `lsl.w #8` are arithmetic (TPAD 0xE693E4, XPD_$CAPTURE_FAULT 0xE5B414
    `lsl.w #8; or.w (A4)`); RING_$GET_ID 0xE2FA76 stores a longword it
    builds from three bytes with `swap` and `lsl.w #8`, also its result
    (`move.l D2,D0`), an expression in ../source.
    The 55 zero-extensions stored as a longword (`clr.l Dn; move.w x,Dn;
    move.l Dn,mem`), which a merged `0` and a word would also give, are
    longword objects where ../source shows (PROC1_$INIT_TS_TIMER 0xE14B86,
    0xE3B838, 0xE1222C; NET_IO 0xE5A5F6's longword at 0x34 is read as one
    at 0xE6A6E4), except acl_$convert_image 0xE44F86 `move.l
    D1,(0x18,A3)`, whose low word ../source reads alone (`and.w
    (0x1a,A0)`, 0xE46228): a longword set narrowed or a merged pair,
    unsettled.
  - **What decides is Uncertain.**  No property of the pair separates the
    two lists: base kind (frame, pointer local, by-address parameter,
    `movea.l #` block), alignment (merged pairs at 0 and 2 mod 4), zero
    or not, the size.  The two lists meet in one module (TTY_$I_INIT's
    unmerged stores are TTY_$K_RESET's merged ones, in another code
    section) and in one map segment (ASKNODE 0xE64598: ASKNODE_$SERVER
    0xE65A3E unmerged, two locals in ../source, and
    ASKNODE_$PROPAGATE_WHO 0xE65EBC merged).  Within a run the pairing is
    not "first two, next two": KBD_$INIT leaves 0x38 alone and merges
    0x3A with 0x3C (source order unknown).  Otherwise the lists are apart
    by segment (of the 9 segments with a merge, 8 have no unmerged
    constant pair), which suggests a compilation setting such as the
    `-opt` level, but nothing in the image names it (bead
    domain-pascal-qb9q).  The grouping is strong by store (17 merged
    and 118 unmerged pairs in 41 segments, 1 segment with both; shuffled
    within the same segment sizes, 9.2 on average, never as few as 1 in
    20000) but weak by routine (8 merging and 77 unmerging routines, 1
    segment with both against 3.25 on average, p about 0.05): the merges
    come in runs within single routines.  It does not follow the
    per-routine settings already known: loop alignment (section 7,
    `loopalign.align_classes`) is on in the merging LOG_$ADD, MAC_OS_$INIT,
    ASKNODE_$PROPAGATE_WHO and TTY_$K_RESET and off in PROC1_$BIND and
    0xE1BF70, and A5 loading (section 4, `a5survey.py switch`) is
    `always` in ASKNODE_$PROPAGATE_WHO and TTY_$K_RESET and `used` in
    PROC1_$BIND, PROC1_$INIT, LOG_$INIT, KBD_$INIT; the unmerged pairs
    are in routines of every combination too (69 aligning and `used`, 12
    not aligning and `always`, 7 aligning and `always`).
  - **`dpc`** (`isel.rs`, `reject_mergeable_stores`): two consecutive
    stores to adjacent objects of one base that fit in a word or a
    longword are refused when one reference could store both: two
    constants, or two loads of adjacent memory in the same arrangement.
    A word and, above it, a longword of constants are refused too
    (FIM_$BUILD_DF 0xE0A896 against 0xE14CBC); with the longword the
    lower, merged (split at the longword, as at 0xE0A896) or not they are
    the same two stores, compiled in source order (0xE70B5E `clr.w
    (0x1a0,A5); clr.l (0x19c,A5)`).
    Not candidates, and compiled as two stores: a pair with any other
    value (no one reference stores it; the 270 above apart, no merge of
    such values found); two
    bytes whose lower one is at an odd address in the frame or the
    module's static data (an even base; the parity through a pointer or
    a symbol is not known, so such a pair stays refused); `device`
    objects, which the reference exempts from merging (section 5.5).
    Golden `stores/adjacent_unmergeable`, exec test `adjacent_stores`.
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
