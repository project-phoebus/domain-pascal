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
- `sets/set_ops` - codegen-patterns 5.1 (set operators, `[]`, `in` on a
  `set of char`), 8: the TTY sites TTY_$I_OUTPUT_BUFFER_DRAINED 0xE1B39E,
  TTY_$K_SET_INPUT_FLAG 0xE6768E, TTY_$I_RCV 0xE1B98E and 0xE1BACE, the
  module-local 0xE1BF70's 0xE1BF92..0xE1C044, TTY_$K_GET 0xE1C5D4,
  TTY_$K_SET_DELAY 0xE67A18, the module-local 0xE1B716's 0xE1B740.
- `sets/in_const_set` - codegen-patterns 5.1 ("`x in c`"; bead
  domain-pascal-bitu): AS_IO_SETUP 0xE6B75C, DISK_$DISMOUNT 0xE6CFF8,
  TTY_$K_SET_DELAY 0xE67A18, PROC1_$TST_LOCK 0xE148DE.
- `sets/byte_sets` - codegen-patterns 5.1 ("One-byte sets"; bead
  domain-pascal-tq8w): `[byte] set of 0..7` fields as ../source's ACL_
  and APP_ records have them: `btst #k` on the byte (ACL_$MIN_RIGHTS
  0xE46922, APP_$RECEIVE 0xE00904), `clr.b`/`move.b #c` stores
  (acl_$convert_image 0xE44EDC, acl_$def_acldata 0xE47916), `bset`/`bclr`
  updates (0xE44FBE, acl_$image_internal 0xE47D40), widening into a held
  word set and its in-place `not; and`/`or` (0xE4691C..0xE46956,
  acl_$eval_acl_entries 0xE462C0), `<> []` widened (acl_$prim_create_
  internal 0xE45270), a widened argument after the pad and a push
  (0xE4535A), narrowing (acl_$expand_default_acl 0xE45A50,
  acl_$find_acl_slot 0xE45F24).
- `stores/entry_loads` - codegen-patterns 12.2, 13, 1.3, 2:
  TIME_$Q_INIT_QUEUE 0xE16C5E.
- `stores/one_stmt_entry_loads` - codegen-patterns 12.2 ("One-statement
  bodies"; bead domain-pascal-s4a5): a body of one `if` loads its
  by-value parameters at entry and branches on the last load's flags,
  TTY_$I_ENABLE_CRASH_FUNC 0xE67292; `tst` of an earlier one, PROC1_$SET_TYPE
  0xE152FA.
- `stores/const_stores` - codegen-patterns 13 (clr, st, move #n, byte
  immediates 0..255), 12.2.
- `stores/const_pairs_apart` - codegen-patterns 13 ("Store merging",
  bead domain-pascal-qb9q): adjacent constant stores one by one in a
  unit built without `--merge-constant-stores` (TTY_$I_INIT 0xE3332E,
  AREA_$INIT 0xE2F3C0, PKT_$BLD_INTERNET_HDR 0xE1217E, NETLOG_$CNTL
  0xE719A4).
- `stores/const_pairs_merged` - the same with `--merge-constant-stores`
  (`.args`): two ascending word constants as one longword, the first two
  of three (TTY_$K_RESET 0xE67332, 0xE67340; KBD_$INIT 0xE333B6),
  `moveq` for a merged value in its range (LOG_$INIT 0xE3017C), a frame
  record's words (APP 0xE00BA8); the shapes merging units keep apart as
  two stores (SMD 0xE6D824, 0xE6D838 off A5; ACL_ 0xE44EC6 bytes;
  FLP_DO_IO 0xE3DE4A descending).
- `stores/nil_stores` - codegen-patterns 13 ("Stores of constants",
  `nil`; bead domain-pascal-j4ip): `nil` stored as `clr.l` to a frame
  local (BAT_$ALLOC_VTOCE 0xE3AEDA), a variable of the unit (BAT_$ALLOCATE
  0xE3B2EE), another unit's variable (HINT_$SHUTDN 0xE49924), two
  adjacent longwords through a pointer (MAC_OS_$CLOSE 0xE0B506), a local
  in a data register (IO_$GET_DCTE 0xE1A466).
- `stores/pointer_consts` - codegen-patterns 13 ("Pointer constants";
  bead domain-pascal-gw19): a pointer constant in `moveq`'s range stored
  from an address register, `movea.l #n,An; move.l An,<ea>`, in a `dbf`
  loop over a frame record's handlers (SAU5 PBU_$ACQUIRE_INT 0xE588D4)
  and through a pointer; one outside it stored directly, `move.l #n`
  (PEB_$LOAD_REGS 0xE5AE6A); compared in memory, `cmpi.l` (SAU5
  PBU_$SETUP 0xE59842); a loop's pointer local in an address register
  assigned a constant, `movea.l #n,An` (PEB_$GET_STATUS 0xE5ADD6), and
  `nil`, `suba.l An,An` (DISK_ 0xE3C620), and compared with a constant,
  `cmpa.l #n,An` (FIM_$BUILD_DF 0xE0A552's form with an address).
- `stores/bool_stores` - codegen-patterns 3 ("Computed booleans stored
  and pushed"; bead domain-pascal-k1sv): a comparison stored with `s<cc>`
  into a data register and `move.b` (TTY_$K_GET 0xE1C506), into a
  register-held local's register (0xE1C42E), to a frame local, and passed
  by value right before its push (0xE1C54E).
- `stores/not_values` - codegen-patterns 3 ("Other conditions as boolean
  values"; bead domain-pascal-2lse): `not` of a variable, a field, a
  function's result and a plain `or` as `not.b` (0xE19610, 0xE0B616,
  0xE12A26, 0xE0AED6), of a one-bit field as the opposite `s<cc>`
  (0xE725EC); best effort: `not` of a comparison, `and then`/`or else`
  values (the shape of 0xE129F4..0xE12A18).
- `stores/both_regs` - codegen-patterns 12.4 ("Stores whose destination
  and value both need a register"), 10.1 (bead domain-pascal-k1sv): the
  side with more instructions first, the value on a tie (TTY_$K_SET_DELAY
  0xE67A40, TTY_$K_INQ_DELAY 0xE67A96, TTY_$K_GET 0xE1C4D6).
- `control/in_range` - codegen-patterns 10.1 (bead domain-pascal-k1sv):
  `in_range` of an enumeration, `cmpi.w #31; bhi` (TTY_$K_SET_FUNC_CHAR
  0xE67502), and as a value.
- `control/in_range_subrange` - codegen-patterns 10.1 (bead
  domain-pascal-bzkf): `in_range` of word subranges, 0..3 as `cmpi.w #3`
  in memory, 1..3 as `clr.l; move.w; subq.l #1; cmpi.l #2`
  (SIO_$K_SET_PARAM 0xE68252, 0xE68278).
- `expr/const_minus_widened` - codegen-patterns 10.1, 12.4 R8/R11 (bead
  domain-pascal-bzkf): a constant minus a word widened to a longword
  (NETWORK_$READ_AHEAD 0xE10000, AREA_$INVALIDATE 0xE08EF4).
- `expr/min_max` - codegen-patterns 10.1 (bead domain-pascal-bzkf):
  `min`/`max` as compare-and-select, the first operand in the result's
  register: word arithmetic widened (NETBUF_$ADD_PAGES 0xE0E95A),
  unsigned (MMAP_$GET_IMPURE 0xE0D624, NET_IO_$COPY_PACKET 0xE0E612), a
  constant second operand (PKT_$DAT_COPY 0xE12850), `max(x, 0)` on the
  flags (NETBUF_$DEL_PAGES 0xE0EB64).
- `expr/abs` - codegen-patterns 8, "abs" (bead domain-pascal-g6xa):
  `bpl.s` over `neg` on the flags of a load (PROC2_$WAIT 0xE3FE5E, an
  argument; TIME_$ADJUST_TIME_OF_DAY 0xE168F4), a `sub.l` (LOCATE_SERVER
  0xE4A440), a copy, a `muls.w` and after `tst.w` (TPAD_$DATA 0xE6924A,
  0xE69624, 0xE6966E); compared unsigned with constants and `linteger`s,
  signed at word width with an `integer`, zero-extended into a new
  register for a longword sum (0xE69764).
- `stores/store_from_memory` - codegen-patterns 13 (memory to memory in a
  one-statement body): PROC2_$GET_BOOT_FLAGS 0xE41B62.
- `stores/volatile` - ref 3.15.1-3.15.3; codegen-patterns 13 (device
  registers: `move.<s> #0`, `move.b #0xff`, WIN_ 0xE19112, RING
  0xE758E2), 12.4 R7 (no reuse of a volatile variable's computed
  value), 12.5 (a volatile local stays in its frame slot).
- `data/byte_index` - codegen-patterns 5.2 ("Byte indices"): `char`
  indices zero-extended with `clr.w; move.b`, scaled, `.w` indices
  (tty_$i_enable_crash_func 0xE672AA, TTY_$K_SET_INPUT_BREAK_MODE
  0xE67874, VTOC_$LOOKUP 0xE38FD2).
- `data/other_unit_index` - codegen-patterns 5.2 ("Another unit's array",
  "Arrays through a pointer"), 12.4 R11 (bead domain-pascal-rlnm):
  `movea.l #sym,An` after the index's value and before its scaling
  (module-local 0xE4649C, byte-identical; TTY_$I_WAIT 0xE1C22E ×12),
  after a `neg` scaling (0xE1AA50 ×56), in a byte index's stall
  (KBD_$FETCH_KEY 0xE1CBAA, 0xE1CB8A for a pointer loaded from memory).
- `data/kept_base` - codegen-patterns 5.2 ("Base kept across
  statements"), 12.4 R7 (bead domain-pascal-lnzy; `tools/basesurvey.py
  pairs`): another unit's array's base read again by a later statement,
  no call, loop or other address register's value between, its load
  dominating: one base for two statements' elements (the module-local
  0xE5B704 at 0xE5B744..0xE5B756, other index registers there), past an
  `if` that returns (SIO_$I_GET_DESC 0xE667E8..0xE66808), past an `if`'s
  join (the module-local 0xE3ECEA at 0xE3ED8A).
- `data/elem_addr` - codegen-patterns 5.2 ("The element's address as a
  value"), 12.4 R2 (bead domain-pascal-o1kl): a `with` record of another
  unit's array read twice in the base's own register (XPD_$FIND_INDEX
  0xE5AF60), one live across a call in a callee-saved register with the
  base in A0 (MMAP_$AVAIL 0xE0CC80).
- `data/elem_once` - codegen-patterns 5.2 ("The element's address as a
  value", read once), 12.4 R2 (bead domain-pascal-1ibr): from a base in
  A0, the address in A1 when A1 is free and clean (the module-local
  0xE1C204 at 0xE1C242, an address stored; NETBUF_$RTN_DAT 0xE0F082 and
  ACL_$IN_SUBSYS 0xE470B2, elements beyond a byte, the latter off a
  large overlay section's base 60000 bytes in; 0xE01244 after a call),
  in A0 itself when A1 holds an earlier address (SIO_$INIT 0xE32CD6).
- `data/index_mul` - codegen-patterns 5.2 ("Index scaling"), 8
  ("Multiplication by a constant"; bead domain-pascal-r3yy): a constant
  of 1..127 done with a multiply goes first, `moveq #c,Dn; mul<s>.w
  x,Dn` (TPAD_$SET_PUNCH_IMPACT 0xE69B0E ×44 through a by-address
  parameter, TPAD_$INQUIRE_UNIT 0xE6999C, TPAD_$DATA 0xE691CA, module-local
  0xE383B0 ×100 of a by-value parameter, RIP_$TABLE 0xE6904C ×92 of
  another unit's array, 0xE458CC a word product); ×268 `mulu.w #c`.
- `data/wide_index` - codegen-patterns 5.2 ("Word indices used as `.l`";
  bead domain-pascal-wkg): into arrays whose bounds times the element
  size do not fit in 16 bits, a `pinteger` held in a register
  zero-extended, the base between the `clr.l` and the copy (AREA_$DELETE
  0xE07C4A; a parameter and a register local), an `integer` with `ext.l`
  (AREA_'s 0xE075CA at 0xE076DC), `muls.w #228` / `mulu.w #150`
  (0xE3EBB2, 0xE54D76), an unscaled `.l` index (0xE17088).
- `data/static_data` - codegen-patterns 4, 13, 11: `pea (A5)` / `lea` /
  restore around a routine that uses the unit's data (PROC1_$GET_LOADAV
  0xE14BBE..0xE14BD2), `(A5)` / `(d,A5)` operands (XNS_IDP_$GET_STATS
  0xE18FE6, PROC2_$GET_BOOT_FLAGS 0xE41B66), an internal routine that
  inherits A5 (0xE17778), the `.bss.moddata.<UNIT>` block.
- `data/initialised` - codegen-patterns 11 ("Initial values"), 5.1:
  initialised static data as `.moddata.<UNIT>[.<SECT>]` blocks with
  contents (words, a variant's field by name, a string shorter than its
  array, boolean, enumeration, `nil`, sets; values from TTY's .DATA
  0xE8242C, WIRED_DATA 0xE2DDB4, OS_INIT_DATA 0xE351B0), a section with
  no initial value left NOBITS.
- `data/byte_statics` - codegen-patterns 11 ("Byte-sized module
  variables"; bead domain-pascal-yvko): char and boolean module
  variables one to a word, the high byte (SIO_IO's WIRED_DATA 0xE2DEB8
  from +0x48, `70 00 60 00 ...`, read as in SIO2681_$SET_BREAK
  0xE1D13E/0xE1D148; NETWORK_$ACTIVITY_FLAG 0xE24C42 ..), one-char
  packed records a word each.
- `data/packed_init` - codegen-patterns 11 ("Initial values", packed
  records), ref 3.10.7 p. 3-35 (bead domain-pascal-c0j8): bit fields of
  packed records in the data block, the first field in the most
  significant bits (KBD's kbd_$state_table, WIRED_DATA 0xE2DDE4+0xC0);
  booleans, an enumeration, a negative subrange, fields across a byte,
  folded constants, a repeat count.
- `data/addr_init` - codegen-patterns 11 ("Initial values",
  addresses): `addr(...)` initial values as relocated `.long`s (a
  routine, another unit's variable plus an offset, the unit's own data
  area plus an offset; FLP_ 0xE7AEF4, WIN_ 0xE2B89C), owner's ruling on
  3.13.4 p. 3-51.
- `data/self_addr` - codegen-patterns 11 ("Initial values",
  addresses; bead domain-pascal-i5mf): a variable's own address in its
  initial value (owner's ruling, reference-clarifications "A variable's
  own address in its initializer"), unit-level, `static`, `define`d, a
  routine's static and an overlay section's variable; ../source
  network/network.pas network_$rqst_done_ec, the image's NETWORK D
  segment 0xE248FC at +0x258 `0000 0000 00e2 4b54 00e2 4b54`.
- `data/with` - codegen-patterns 5.2 ("`with`", "`addr(x)` as a
  value"), 1.5: records named in place, a unit pointer held for the
  body, an element through a pointer (NETLOG_$LOG_IT 0xE71BB6..0xE71BFC),
  a stable pointer parameter, `lea; move.l`, `pea`.
- `data/with_field_array` - codegen-patterns 5.2 ("`with`"; bead
  domain-pascal-3m13): an element of an array at +0x3C of another unit's
  record, `lea (0,A0,D0.w),A3`, the 0x3C in every field's displacement
  (SAU5 MT_PURGE_UCB 0xE75770..0xE757CA).
- `data/block_copy` - codegen-patterns 5.2, 7: block copies of 12 and 14
  bytes unrolled (0xE14BC6, 0xE419B0), of 144 and 22 bytes as `dbf`
  loops (0xE01494, 0xE19DA2).
- `data/block_addresses` - codegen-patterns 5.2 "Block addresses", 7.1:
  another unit's record as a block copy's source or destination,
  `movea.l #sym,An` (CAL_$GET_INFO 0xE6856C, DISK_$GET_ERROR_INFO
  0xE6C090, 0xE336E2); a pointer held in an address register, `lea
  (An),Am` (0xE588D8, 0xE3DBBE), in a data register, `movea.l Dn,An`
  (TIME_$Q_SCAN_QUEUE 0xE16EBC); a loop's stepped element pointer, `lea
  (-0x48,An),Am` (DISK_$LVUID_TO_VOLX 0xE6D182).
- `data/char_copies` - codegen-patterns 5.2 "Arrays of char": an array
  of char copied byte by byte, 4 bytes as four `move.b (A0)+,(A1)+`
  (CAL_$READ_TIMEZONE 0xE3E592), 8 bytes and more as a `moveq #n-1;
  move.b; dbf` loop (ACL_ 0xE45130, PROC1_$BIND 0xE14DB6, 0xE40C0C), a
  record holding one by longwords (CAL_$WRITE_TIMEZONE 0xE3E626).
- `data/device_copies` - codegen-patterns 5.5, 5.2 (bead
  domain-pascal-e07e): a `[device]` window's 12-byte descriptor saved and
  restored by the ordinary block copy, three `move.l (An)+,(Am)+` each
  way (sau5 WIN_ RESTORE 0xE19A1E/0xE19A50, SEEK 0xE19B14/0xE19B50), a
  copy between two device sides, a 2-byte field as one `move.w`
  (win_$ecc_correct 0xE19898).
- `data/device_updates` - codegen-patterns 5.5 (bead domain-pascal-eaw7):
  `[device]` objects updated in place, the sau5 MBUS_$ENABLE_DEV
  0xE0AD52 and MBUS_$DISABLE_DEV 0xE0AD68 (`or.w`/`and.w Dn` into
  PBU_CTL), VME_$INT's `andi.w #-0x4001` (0xE0ABCC), sm_$command's packed
  unit field and command byte (0xE6F1A2..0xE6F1B2), a counter `addq.w`;
  one bit of a device with `ori`/`andi` of its mask (bead
  domain-pascal-pjh6): sm_$wire's `ori.b #-0x80,(0x44,A2)` (0xE6EFEE),
  OS_$SHUTDOWN's `andi.b #-9,(0xff9431).l` (0xE6ED30).
- `data/device_set_members` - codegen-patterns 5.5 (bead
  domain-pascal-791y): one constant member added to or removed from a
  `[device]` set with `ori`/`andi` of the mask at the set's width, the
  sau5 COLOR2_$ENABLE_GPU_INT 0xE7355C/0xE73564 (KOLOR_CTL's ctl),
  COLOR2_$BUSY_WAIT 0xE734DE/0xE734FE (COLOR3_$CR_CONTENTS),
  COLOR_$INQ_DONE_WAIT 0xE730C8/0xE73128 (COLOR_SUP's int_ctl); byte and
  longword sets the same; a set that is not a device keeps `bset`/`bclr`.
- `calls/const_pool` - codegen-patterns 1.5, 11, 2: constants passed by
  address as cells after the routine that first uses them, `pea (d,PC)`,
  `move.l (SP),-(SP)` for the same cell twice (TERM_$SEND_KBD_STRING
  0xE1AB0C), cells shared by later routines when bytes and type agree,
  the order of a routine's new cells (TIMER_$INIT 0xE1639C), word cells
  before longword cells after a 0x2048 filler (LOG_$SHUTDN 0xE17604); an
  `addr(...)` through a frame temporary (TONE_$ENABLE 0xE1ACFC).
- `calls/const_pool_noalign` - codegen-patterns 1.5, 7: with
  `--loop-align=off` the longword cell follows the word cell at 2 mod 4,
  no filler (ACL_$SET_ACL_CHECK 0xE4787A).
- `calls/univ_small_const` - codegen-patterns 1.5, "To a `univ`
  parameter" (bead domain-pascal-45yc): an integer constant for a 4-byte
  non-integer `univ` parameter is its integer32 value in a 4-byte cell
  (SAU5 MBUS_$UNDEF_INT 0xE0AA0C, cell 0xE0AA20 `0000 002a`), shared
  with an integer32 parameter's cell of the same value.
- `calls/str_const_args` - codegen-patterns 1.5, "A string constant to
  an array of char" (bead domain-pascal-jgtd): a string constant for a
  value or `in` array of char that is not `univ` is a cell of the
  parameter's size, padded with blanks (SAU5 tape_$check_block 0xE33F12,
  cell 0xE33FA0, 200 bytes), one cell for equal constants (SAU5
  TAPE_$BOOT 0xE344C0/0xE344D4/0xE344E4, cell 0xE346DE).
- `calls/univ_symbol_addr` - codegen-patterns 1.5, "Address of a symbol
  by address" (bead domain-pascal-68tc): `addr(x)` of another unit's
  object or of a routine passed by address is a 4-byte code cell holding
  the address, whatever the parameter's size (SAU5 VME_$INT 0xE0ABBA, cell
  0xE0AC40 PBU_CTL for LOG_$ADD's 192-byte `univ` data; SAU2
  PEB_$LOAD_WCS 0xE32216, DXM_$ADD_SIGNAL 0xE172B6).
- `calls/calls` - codegen-patterns 1.1, 1.3, 1.4, 1.5, 1.6, 12.2: pushes
  of constants (`clr`, `pea (n).w`, `move.l #n`, merged word pairs, `st`),
  values and addresses, pads, `addq.w`/`lea` pops, no pop when no push
  follows (0xE41050), `jsr` and `bsr.s`/`bsr.w` (0xE00EB0, AST_$PAGE_ZERO 0xE00EEC).
- `calls/byte_consts` - codegen-patterns 1.5, 1.3 (bead
  domain-pascal-g5xv): byte constants by value, `move.w #n<<8,-(SP)` with
  the byte in the high half (TTY 0xE1AFC0 chr(254) `move.w #-0x200`),
  chr(0) `clr.w` (0xE1AFD4), a char next to `true` pushed separately
  (SIO_$INIT 0xE32E60/0xE32E62); `[byte]` subranges likewise.
- `calls/word_const_runs` - codegen-patterns 1.5 (bead
  domain-pascal-9pq8): runs of three or more word constants paired from
  the rightmost argument, an odd run's leftmost word alone (SOCK_$OPEN
  from NETWORK_$PAGE_SERVER 0xE115FA, SOCK_$ALLOCATE_USER from MSG_$SARI
  0xE59DE0, NETLOG_$LOG_IT 0xE135CE), `false` inside a run and left over
  (PACCT_$SHUTDN 0xE5A71A, 0xE548BE), `true` splitting one (0xE51510).
- `loops/align_off_from` - codegen-patterns 7: `dpc
  --loop-align-off-from` (options in `align_off_from.args`); fillers
  before the named routine, none from it on, the module end padded
  with zero words (VTOC_$MOUNT 0xE3871E / VTOC_$DISMOUNT 0xE387CE).
- `loops/fill` - codegen-patterns 7, 5.2: `for` loops over an array whose
  index is used for nothing else, as `movea.l #sym,A0`, count, filler,
  `clr.<s> (A0)+`, `dbf` (0xE00ECC..0xE00EDA).
- `calls/nil_push` - codegen-patterns 1.5, "Zero" (bead
  domain-pascal-73sx): `nil` passed by value, `clr.l -(SP)`, pushed after
  word values and `st -(SP)` (NET_IO_$PUT_IN_SOCK_INT 0xE0E506,
  0xE0E4FC).
- `calls/nil_by_address` - codegen-patterns 1.5, "`nil` by address"
  (bead domain-pascal-dtbg): one longword 0 cell, `pea (d,PC)`, for the
  module's `nil`s (SAU5 PBU_$RELEASE_INT 0xE59498, PBU_$SHUTDOWN
  0xE59BE0, PBU_$FREE_ASID 0xE5A77A: the cell 0xE594B0), shared with
  integer32 0 and `p(0)` (Uncertain for `nil`; a pointer constant sharing with
  an equal integer32 is Verified, OS_$INIT 0xE34AA0).
- `calls/pointer_targets` - codegen-patterns 1.5 (bead domain-pascal-s9j):
  objects reached through a pointer passed by address: `p.f^` with the
  record held, `move.l (d,A2),-(SP)` (0xE1CC52 in KBD_$RCV's nested
  0xE1CC10,
  TTY_$I_OUTPUT_BUFFER_DRAINED 0xE1B3A4); a field through a slot not
  held (KBD_$OUTPUT_BUFFER_DRAINED 0xE1CE9E), the enclosing routine's
  slot (0xE1AFB8), a pointer local (AST_$PURIFY 0xE0599E) or a pointer
  in the unit's data (AUDIT_$LOG_EVENT_S 0xE70E88): `movea.l
  <pointer>,A0; pea (d,A0)`; a pointer local's own target `q^`: the
  pointer pushed, `move.l (d,A6),-(SP)` (0xE53DB0 to FILE_$FW_FILE).
- `calls/routine_pointers` - codegen-patterns 1.6, 1.4, 3: calls through
  routine pointers (`val_param` on the type, ref p. 4-146), the pointer
  read into an address register after the pushes (SIO_$I_CTS_CHANGE
  0xE1C71C..0xE1C728; IO_$INIT 0xE329CC `movea.l D3,A0`), a function
  pointer's result in D0 (TTY_$I_ERR 0xE1BE22), a local passed to a
  `var` parameter of the type re-read after the call (bead
  domain-pascal-5aw.16), `addr` of a routine stored as `move.l #sym`
  (MAC_OS 0xE0B9A8).
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
- `control/and_or` - codegen-patterns 7.1 ("Plain `and` and `or`"), 8
  (`odd`): `s<cc>` operands combined with `and.b`/`or.b` and the sign
  branch (TTY_$I_ERR 0xE1BE48, 0xE4BC2A), boolean variables as operands,
  `not.b`, a stored value; `btst #0` of held parameters (FIM_$BUILD_DF
  0xE0A6F6).
- `control/and_or_call` - codegen-patterns 7.1 ("A plain `or`/`and`
  whose right operand calls a function"; bead domain-pascal-i1by): the
  left operand jumping code, the right one's compare of the call's D0
  formed with `s<cc>` into the lowest callee-saved register and tested
  (TPAD_$SET_UNIT 0xE699EE, TPAD_$SET_UNIT_MODE 0xE6980A; against 0,
  FILE_$SET_PROT_INT 0xE5DE5A; a constant, EC2_$WAIT 0xE4262A; a
  register-held local, 0xE5E590).
- `control/compares_mixed` - codegen-patterns 7.1, 8, 12.4 R11: two
  variables in memory (the left one loaded, TTY_$K_SET_FLAG 0xE67462),
  a signed and an unsigned word widened to longwords (TTY_$K_PUT
  0xE67C42, tty_$i_put_chars 0xE1B050), a word against a longword
  (0xE179BC), records compared with `cmpm` (8 bytes unrolled,
  TTY_$K_SIMULATE_TERMINAL_INPUT 0xE1C1AC; 6 bytes `cmpm.w`/`dbne`,
  0xE16EC2; 32 bytes `cmpm.l`/`dbne`, 0xE3D84E; another unit's record
  by `movea.l #abs`) and as a boolean value with `sne`/`seq` (PACCT_$ON
  0xE5A9C0).
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
- `sections/named_sections` - codegen-patterns 11, 4, 1.6: sections
  named in the module heading and in `[section(c,d)]`, one output
  section per Pascal section with its own module-end padding, A5 per
  data section, `jsr abs.l` into another code section from a statement
  and from an expression (PROC1_$INIT 0xE2F9EC), an internal routine so
  called loading its own A5 (0xE14B12).
- `sections/empty_code_section` - codegen-patterns 11, 4 (bead
  domain-pascal-zzru): `module name, , data_section;`, the code in
  `.text`, the data in the named section, A5 loaded with that block
  (PCHIST_$CNTL 0xE5CDBE).
- `sections/section_fragments` - codegen-patterns 11, 1.6, 7: a unit of
  two code sections compiled as a fragment of each (`.args`:
  `--fragment-origin=SECTION=0xADDR`), one origin symbol, pad and
  module-end padding per section; routines of the module outside the
  unit (`--module-routine=SECTION:NAME=0xADDR`) called with `bsr` from
  their own section (an encoded `bsr.s`, a `bsr.w`) and `jsr abs.l` from
  another or from a section the unit lacks (TTY_$K_RESET 0xE6730E).
- `sections/other_data_section` - codegen-patterns 11, "Data per
  section": another data section's block by `movea.l #block,An` and
  `(d,An)`, loaded before the entry loads for a first constant store,
  again after each call, `lea (d,An),Am` for `addr`, `move.l
  #block,-(SP)` for the block's first variable by address (KBD_$INIT
  0xE3336A, DXM_$INIT 0xE2FF54, LOG_$INIT 0xE30054).
- `sections/overlay_sections` - codegen-patterns 11, "Overlay sections"
  (bead domain-pascal-raxl): `var (name)` overlay sections laid out as
  PCHIST_UNWIRED_D, their variables absolute off the section's symbol
  (PCHIST_$CNTL 0xE5CD1A, 0xE5CDA8), `move.l #sym,-(SP)` for the
  section's start and `pea (sym+d).l` after it by address (XPD_$INIT
  0xE32334; PCHIST_$CNTL 0xE5CEF6, 0xE5CF00), an element off `movea.l
  #sym,An` (PCHIST_$UNIX_PROFIL_FORK 0xE5CC76), A5 for the unit's own
  block; `.comm`, and a definition for a section with initial values.
- `regalloc/live_ranges` - codegen-patterns 12.4 R1-R3, 12.2, 1.5: the
  allocator (bead domain-pascal-g5q.5): parameters held across calls in
  A2/D2 with `pea (d,A2)` pushes (module-local 0xE32B76), a third data
  value in D2 without a call (0xE177A0), registers reused after a
  parameter's last use, a value dead before a call in a scratch register,
  a parameter live around a `repeat` with a call in it, and (12.4 R4(c))
  parameters whose last use is a call's argument held through the call in
  D2 / A2 (PROC1_$SET_ASID 0xE148FE; module-local 0xE1D592).
- `regalloc/args_held_branches` - codegen-patterns 12.4 R4(c) in bodies
  with branches (bead domain-pascal-9j1z): a parameter loaded at entry
  whose last use is a call's argument is held through the call in D2 /
  A2 when the call lies in an `if` part (AREA_$ASSOC 0xE096D2), in each
  part (XPD_$GET_FP_INT 0xE5C574/0xE5C57E), with the parameter used on
  the other path (SIO_$I_INHIBIT_XMIT 0xE1C9EC), after a join inside an
  `if` part (SOCK_$PUT_INT 0xE161E4) or before the first branch
  (VOLX_$GET_INFO 0xE6B5D4).
- `regalloc/cse` - codegen-patterns 12.4 R7: computed values reused
  across statements (bead domain-pascal-g5q.7): FIM_$ACKNOWLEDGE's
  `PROC1_$AS_ID * 12` in D2 (0xE0A972..0xE0A97E, reused at 0xE0A9B0)
  with the tables in the unit's data; LOG_$READ's worker to 0xE177CA, the
  zero-extended `offset` reused after the `if` (0xE177D0), the sum in the
  operand not reused (0xE177B4 `add.l D1,D2`), both `clr.l` first
  (0xE177A6..0xE177AC, the scheduler, 12.4 R11).
- `regalloc/ptr_data_regs` - codegen-patterns 12.2 ("Pointers in data
  registers", bead domain-pascal-pze): a pointer or by-address parameter
  also used as data held in a data register and copied with `movea.l
  Dn,An` where it is dereferenced (EC2_$INIT 0xE42C64..0xE42C74,
  PROC1_$FREE_STACK 0xE15126..0xE15136), passed on with `move.l
  Dn,-(SP)` (EC2_$ADVANCE 0xE42CDA), copied again after a call and read
  again across `if`s while its register still holds it (OSINFO_$GET_MMAP
  0xE5C870, 0xE5C8C8, 0xE5C952); a pointer only stored whole stays in an
  address register (`stored`: the AST module-local 0xE00F7C, EC_$INIT
  0xE15202).
- `regalloc/schedule` - codegen-patterns 12.4 R9-R11 (bead
  domain-pascal-g5q.8, `crates/dpc-m68k/src/sched.rs`): a zero-extension's
  `clr.l` moved into a load's stall (0xE382BE), a store ordered against
  the next statement's code, a call's pop and the next pad kept together
  (0xE1AFC8).
- `regalloc/late_slot_pass` - codegen-patterns 12.4 R4(b) (bead
  domain-pascal-i24w): a slot loaded for one use after a branch passes
  over the free registers holding a dead slot load on every path to the
  first one never written: the status in A2 passed over after its test
  (TTY_$K_INQ_PGROUP 0xE6796E, TTY_$K_INQ_INPUT_BREAK_MODE 0xE678E8 A3),
  A0 and A1 after the straight-line rule's slots (A2).
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
- `expr/byte_ints` - ref 3.15.5 (`[byte]` subranges); codegen-patterns
  7.1, 8, 1.3, 7.3, 5.2: TPAD_$DATA 0xE691D8 (`clr.w; move.b; cmpi.w`),
  0xE691EE and 0xE693EC (`moveq`/`move.w #c; and.b`), 0xE69228 (`move.b;
  ext.w` of a type transfer to a signed byte), 0xE6936A (`lshft` of a
  zero-extended byte); byte stores, a byte argument by value, a byte
  `case` selector and index.
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
- `frame/byte_locals` - codegen-patterns 2, "Frame layout": byte locals
  word aligned, one to a word, a longword after them longword aligned
  (FILE_$REMOVE_WHEN_UNLOCKED 0xE5FC8E `pea (-2,A6)` in `link #-4`;
  PROC2_$WAIT 0xE3FDD0).
- `loops/while` - codegen-patterns 7.4, 12.5: `while` loops as `bra` to
  the bottom test, the head aligned like any loop head (MAC_OS_$CLOSE
  0xE0B4CC), an `and then` test (0xE4A028), a walk pointer held in an
  address register and tested with `cmpa.w #0,An` (0xE3CCCE), `exit` and
  `next` as branches past or to the test.
- `loops/forever` - codegen-patterns 7.2 ("Loops with no test"; bead
  domain-pascal-pkle): `repeat ... until false`, the bottom `bra` back
  to the aligned head (MEM_LITES 0xE0C3D2..0xE0C410), `next` to the
  head, a final `if ... then exit` as the negated branch back
  (PKT_$SAR_INTERNET 0xE72152), a final `exit` with no `bra` (MSG_$SARI
  0xE59F76..0xE59F9E), no epilogue after a loop with no way out
  (DXM_$HELPER_COMMON 0xE1721C).
- `loops/for` - codegen-patterns 7.4: `for` loops with an index the body
  uses: the `dbf` count, then the index (`clr.w` / `moveq`), the step
  before `dbf` (`subq.w` for `downto`), a byte array indexed with the
  index register (0xE1CA70), count and index callee-saved across calls
  (DISK_$UNASSIGN_ALL 0xE6BE24), a run-time upper bound with its `subq.w
  #1; bmi` guard and the count copied (DTTY_$WRITE_STRING 0xE1D6AA), a
  count above 127 as `move.w #n`, `exit` and `next`.
- `loops/for_runtime` - codegen-patterns 7.4 ("Run-time bounds", "The
  guards by shape"; bead domain-pascal-jnsb): from 0 to a variable,
  `tst.w; bmi` and the count copied, into the bound's own register
  without a call (XNS_IDP_$REGISTER_ADDR 0xE19020), callee-saved across
  one (DISK_$AS_XFER_MULTI 0xE6BB26), the bound of the unit's data; from
  0 to `e - 1` across a call, `subq.w #1; bmi`, the copy, `clr.w`
  (PEB_$LOAD_WCS 0xE3214A); the constant of `e - c` folded into one
  `subq.w` (DISK_$AS_XFER_MULTI 0xE6BA42).
- `loops/for_runtime_nocall` - codegen-patterns 7.4 ("The count's
  register"; bead domain-pascal-fbah): from 0 to `count - 1` and from 1
  to `count` with no call in the body, the bound a value parameter dying
  at the bound: the count kept out of its register (R2), the counter in
  the lowest register free over the loop, the bound's, below the count,
  and the copy (the MAC_OS_ module-local 0xE0B202 at 0xE0B20C..0xE0B218;
  the RING_ module-local 0xE76422 at 0xE76432..0xE7643C); with no index,
  another unit's array stepped in place, its `movea.l` before the copy
  (SHOW_CURSOR 0xE6E2E2..0xE6E2E8).
- `loops/stepped` - codegen-patterns 7.4, 5.2: `for` loops over arrays of
  elements wider than a byte through pointers stepped by the element size:
  formed with `lea (from*size,An),Ap` after the count and the index, the
  source of a copy first, stepped in the reverse order (LOG_$READ's worker
  0xE177E0..0xE177F2), `subq.l` for `downto`, `lea (s,An),An` above 8
  (0xE313F4), a pointer loaded at entry, an index register and a call
  besides (PKT_$DUMP_DATA 0xE12800..0xE12826).
- `results/pointer_callers` - codegen-patterns 3 ("Callers of pointer
  functions", "Pointer results"), 12.4 R4(a), 12.5: a pointer function's
  A0 stored to memory, a wrapper's result slot (MST_$MAPS 0xE439C8),
  copied into a register-held local (`move.l A0,D0`, 0xE6763C; `movea.l
  A0,A2` with `pea (A2)`, KBD_$SET_KBD_TYPE 0xE7253E), compared with
  `nil` (`cmpa.w #0,A0`), used as a base; a
  pointer result variable held in A2 across a call (DISK_$GET_BLOCK
  0xE3BBBC); slot loads of one argument list in A0, A1, A2 (MST_$MAP
  0xE43886).
- `results/pointer_locals` - codegen-patterns 3, 12.4 R4(c), 12.5
  (bead domain-pascal-500): a pointer local assigned from pointer
  functions held in an address register (`movea.l A0,A2`, `cmpa.w
  #0,A2`, `pea (A2)` held through the last call: KBD_$SET_KBD_TYPE
  0xE7253E); kept in its frame slot when
  pushed by value (`= nil` on the store's flags, A0 as the base on the
  branch's path and after the join: AST_$RESERVE 0xE067BA..0xE067E8),
  reached by a nested routine (`cmpa.w #0,A0`, reloaded at the join:
  AST_$INVALIDATE 0xE0666A..0xE066AE), or passed by address.
- `results/integer_parked` - codegen-patterns 3 ("Pointer locals held in
  D0"; beads domain-pascal-yzxr, -78n2): an integer
  local assigned an `a0_return` function's value and dereferenced through
  a type transfer, held in D0 and moved to An where first dereferenced,
  where a call follows too (TTY_$K_FLUSH_INPUT 0xE1C0A8 / 0xE1C0AE) or
  not (TTY_$K_INQ_INPUT_FLAGS 0xE676DA); pointer locals assigned the
  transferred value copied to A2 (TTY_$I_SET_RAW 0xE673CC) or left in A0
  (TTY_$I_INQ_RAW 0xE67414).  Its `.args` file (`--no-park-pointer-locals`)
  was removed when that behaviour became the default (bead
  domain-pascal-78n2); the expected output did not change.
- `results/integer_parked_forms` - codegen-patterns 3 ("Pointer locals
  held in D0"), 12.4 (beads domain-pascal-dgb, -78n2): an integer local
  under the integer reading, live across no call, held in D0 (`move.l
  A0,D0`) and moved to A0 where first dereferenced, before an `if`'s
  branch when both parts dereference it, D0 left alone to its last
  dereference (TTY_$K_INQ_INPUT_FLAGS 0xE676D4, TTY_$K_SET_INPUT_FLAG
  0xE6767E..0xE676A2, TTY_$K_SET_SESSION_ID 0xE679AA).  Formerly
  `results/pointer_parked` with pointer locals; renamed and its Pascal
  rewritten to integer locals with type transfers when pointer locals
  stopped being held in D0 (bead domain-pascal-78n2); the expected output
  is unchanged.
- `results/integer_parked_if` - codegen-patterns 3 ("Placement inside
  an `if`"; bead domain-pascal-1j61): a parked integer local's `movea.l
  D0,An` as the first instruction of the one `if` part that refers to
  it (TTY_$K_GET 0xE1C40A) and as the first instruction of an `if` whose
  condition dereferences it (TTY_$K_SET_FUNC_CHAR 0xE67516).
- `control/store_flags` - codegen-patterns 7.1 (bead domain-pascal-dgb):
  a frame local stored from a data register and tested next, the branch
  on the store's flags (0xE19BE6, 0xE6B180, 0xE4F09C); `tst` after a
  join and for the unit's data (0xE02806).
- `control/label_flags` - codegen-patterns 7.1 ("Flags across an
  `if`'s labels"; bead domain-pascal-nzs4): `n := n - 1; if n < 0` on
  the `subq`'s flags (SAU5 0xE75B9E, 0xE59682); the `else if n = 0`
  after it (SAU5 0xE75BAA) and the `if n = 0` after `if n < 0 then n :=
  0` (SAU5 0xE5968C) on the flags their label is entered with; an `else
  if` after a `tst` (SAU2 0xE3FE3E); `tst` for `=` right after the
  `subq`, for `>`, and after a compare with another constant (0xE4D9F0).
- `control/computed_zero` - codegen-patterns 7.1 ("A computed value
  against 0"; bead domain-pascal-qlzu): `=`/`<>`/`<`/`>=` on the flags of
  the add, sub, and, or that formed the value (SOCK_$ALLOCATE 0xE15EFA,
  SIO6509_$INQ_LINE 0xE7267C, tty_$i_put_chars 0xE1B046, MAC_OS_$SEND's nested
  routine 0xE0B522 at 0xE0B57C), `tst` before `>`/`<=` after an add or sub (SMD_$SIGNAL
  0xE6F25C) and after a call, a divide or a widened byte (SOCK_$PUT_INT
  0xE161EA, 0xE11BF4, SOCK_$CLOSE 0xE15FBE), `btst` for one bit
  (pacct_$compress 0xE5A9DE).
- `control/top_bit_zero` - codegen-patterns 7.1 ("Signed or unsigned";
  bead domain-pascal-wkm5): a pinteger (`0..65535`) against 0 with
  `tst.w` and the signed condition, its other bound unsigned
  (MT_$ACQUIRE SAU5 0xE760B6, SAU5 0xE75916, SOCK_$PUT_INT 0xE161AA,
  PKT_$BLD_INTERNET_HDR 0xE122D6, PBU_$GET_DCTE SAU5 0xE59212).
- `control/one_bit` - codegen-patterns 7.1 ("One bit"), 8 (`odd`), 5.1
  (bead domain-pascal-3xwx): a word or longword in memory loaded whole,
  then `btst #k` (tty_$i_put_chars 0xE1B01E, 0xE1B0E6; module-local
  0xE1BCFC at 0xE1BD0A), `odd` of one as `lsr.b #1` and the carry
  (0xE1B0F6), `btst` on a register-held or computed value
  (pacct_$compress 0xE5A9DE, DIR_$SET_PROTECTION 0xE522D4).
- `control/goto` - codegen-patterns 7.5 (bead domain-pascal-o4q): `goto`
  as a `bra` to its label, `if c then goto` one conditional branch,
  `else goto` a branch on the negated condition (TTY_$I_RCV 0xE1BBD2),
  a backward `goto`'s label aligned like a loop head, jumps out of
  loops and between `case` arms, a label before the epilogue.
- `control/threading` - codegen-patterns 7.1 ("Branch threading"), 7.2
  (bead domain-pascal-o6p): a branch to a label that a `bra` follows
  goes to the `bra`'s target (TTY_$K_SET_FLAG 0xE6746A), a `bra` no
  longer reached is dropped, and a `while` that starts an `else` part is
  entered at its test with its head still aligned (TTY_$K_GET 0xE1C448;
  the module-local 0xE4AFA8 at 0xE4B0F4..0xE4B0F8).
- `regalloc/goto_reuse` - codegen-patterns 7.5, 12.4 R7, 12.5 (bead
  domain-pascal-o4q): a label a `goto` reaches is a join for the stored
  local pushed from D0 (PKT_$NEXT_ID 0xE124A8) and for reused computed
  values (FIM_$ACKNOWLEDGE 0xE0A96C): reloaded or computed again when a
  branch from before or a later backward `goto` reaches the use, kept
  when the `goto` comes after; register-held locals across `goto`s.
- `results/result_variable` - codegen-patterns 3, 12.4 R5, 12.5: the
  result variable of a body of several statements held in one register
  (D2 across a call, PKT_$NEXT_ID 0xE1248E; D0 without, on three paths,
  with the entry loads of PARITY_$CHK_IO 0xE0B180..0xE0B18A), `st`/`clr.b`
  for booleans, a call's result copied into it (NETBUF_$RTNVA 0xE0ED4E),
  a stored local pushed from D0 or reloaded after a call, a pointer result
  through its frame slot into A0 (0xE6AA38).
- `sysprog/interrupts` - codegen-patterns 10, 12.5, 1.1: `disable` /
  `enable` as `ori.w #0x700,SR` / `andi.w #0xF8FF,SR` (TIME_$GET_ADJUST
  0xE16AB0, 0xE16ABA), `set_sr` as `move.w SR,Dn; move.w newsr,SR`
  (PROC1_$SET_VT 0xE149B6), its result stored, held in a local's register
  or discarded (0xE149F2), `discard` of a function call.
- `expr/builtins` - codegen-patterns 10.1, 13, 8: `sizeof`, `firstof`,
  `lastof`, `succ`, `pred`, `max`, `min`, `in_range` folded to constants;
  `succ`/`pred` of a variable as `addq`/`subq` in place (0xE00B3E).
- `expr/transfer` - ref pp. 4-197..4-199, codegen-patterns 5.2 ("Pointer
  arithmetic"): type transfers with no code of their own, pointer sums
  computed in a data register (LOG_$READ's worker 0xE177A6..0xE177D4; a
  constant first, 0xE00AB4; `addq`/`subq`, 0xE4D5A2, 0xE028DE) and moved
  to an address register where used as one (0xE4EFCA..0xE4EFD2), integer
  transfers converting like assignments.
- `expr/transfer_const_ptr` - ref p. 4-197 with the owner's ruling
  (docs/reference-clarifications.md, integer constant to a pointer type;
  bead domain-pascal-gqp): `ec2_$ptr_t(1)` stored, passed by address and
  by value as a 4-byte constant (codegen-patterns 13, 1.5); stored from
  an address register, `movea.l #1,An` (codegen-patterns 13, "Pointer
  constants", bead domain-pascal-gw19).  No image site identified.
- `expr/transfer_neg_const_ptr` - ref 3.2.3 p. 3-5, p. 4-197 with the
  owner's ruling of 2026-10-21 ("Allow, 32-bit"; bead domain-pascal-j1bt):
  `p(-1)` is 16#FFFFFFFF, stored into a pointer field (`movea.l #-1,An`,
  SAU5 PBU_$ACQUIRE_INT 0xE588D4's form; bead domain-pascal-gw19), passed by address
  (a `ffff ffff` cell) and by value (`pea (-1).w`), and compared as
  `p(integer32(-1))` is (`cmpi.l #-1`, the form of SAU5 PBU_$SETUP
  0xE59842).
- `expr/addr_arith` - ref p. 4-13, p. 4-197, codegen-patterns 5.2
  ("`addr(x)` as a value"), 13 (bead domain-pascal-j28d): an address
  inside another unit's record minus the record's address (NET_$OPEN
  0xE5A1B8..0xE5A1C6, `sub.l #sym` in the `90bc` encoding), `add.l
  #sym` / `sub.l #sym` with a computed value and a variable (0xE0F148,
  0xE09E6A), two frame addresses subtracted, the subtrahend's `lea` first
  (ASKNODE_$SERVER 0xE65DA0), `integer32(addr(x))` stored and passed by
  value as `addr(x)` (NETBUF_$INIT 0xE2F63C, SOCK_$INIT 0xE2FE4C,
  FLP_$DINIT 0xE3E15E).
- `expr/narrow` - codegen-patterns 8 ("Narrowing and widening"; bead
  domain-pascal-8td): a longword field by address stored into a word
  (APP_$DEMUX 0xE00AF6), a word `in` parameter into a byte (PACCT_$LOG
  0xE5AB26), a longword in the frame and in a register pushed as words
  (0xE0CF2C, LOG_$ADD 0xE1775E), `rshft` and `&` of a held longword
  pushed (DBUF_$GET_BLOCK 0xE3A5EC..0xE3A5F6), a longword product stored
  into a word (DBUF_$INIT 0xE3ABE8), a word plus a longword (MAC_OS_$SEND
  0xE0B650), words and bytes widened for value parameters (APP_$RECEIVE
  0xE008E6, SOCK_$CLOSE 0xE16008).
- `expr/shift_widen` - codegen-patterns 8 ("Shifts of a 16-bit
  operand"; bead domain-pascal-4rfc): `lshft`/`rshft` of a `0..65535`
  word into a longword, widened with zeros and shifted in 32 bits
  (AS_$GET_ADDR 0xE58418, UID_$GEN 0xE1A04A; a count in a word variable,
  module-local 0xE3C1CE..0xE3C1EA), into a word at the word's width
  (module-local 0xE07386, 0xE3CB9E), compared unsigned (AREA_$REMOVE_SEG
  0xE0998E, MST_$INVALIDATE 0xE44222), added to another word unsigned
  (module-local 0xE09412..0xE0942A), divided unsigned (M$DIU$LLL,
  M$OIU$LLL, `lsr.l`); of an `integer`, `ext.l` then the 32-bit shift
  (PEB_'s module-local 0xE31DEA), `lshft` stored at the word's width;
  `arshft` of an `integer` unchanged (`asr.w; ext.l`).
- `expr/narrow_store` - codegen-patterns 8 ("Narrowing in an
  assignment"; bead domain-pascal-d59a): an element's, a `with` record's,
  a pointer's target's and a packed field's low bytes read in place (MMAP
  0xE0C602, NETBUF_$ADD_PAGES 0xE0EAA8, MMAP_$FREE_PAGES 0xE0CFCC, RING
  0xE760F4, RINGLOG_$LOGIT 0xE1A3A0 and 0xE1A3F4), a longword parameter's
  low byte (DISK_IO 0xE3D630), function results stored at the narrow
  width from D0 (RIP_$TABLE_D 0xE68F24, NETWORK 0xE0F526).
- `expr/element_update` - codegen-patterns 8 ("Array elements"; bead
  domain-pascal-9pkq): `x[i] := x[i] ± 1` on a word array of the unit's
  data indexed by another unit's word, loaded, changed and stored within
  a byte (AUDIT_$CONTROL 0xE715CC), `lea` and `addq`/`subq` in place
  beyond (ACL_$ENTER_SUPER 0xE46FA4); a field of another unit's array and
  a byte element in place (PMAP_ 0xE13146, AST_ 0xE00E2C).
- `nested/static_link` - codegen-patterns 6, 12.5: nested routines in the
  shape of PMAP_$FLUSH 0xE1376C: emitted before their enclosing routine;
  the static link passed in A1 to a routine a sibling also calls
  (0xE13928, 0xE13718, 0xE135A4), the dynamic link `(A6)` used by one
  only its parent calls (0xE13616); uplevel slots, objects and locals
  through the link register (0xE135B8, 0xE135DA, 0xE1362E); the 4-byte
  link slot of nested frames; the parent's uplevel local in its frame.
- `nested/uplevel_index_mul` - codegen-patterns 8, 5.2, 6 (bead
  domain-pascal-dctl): another unit's array of 46-byte records indexed by
  the enclosing routine's local, `moveq #46,D0; movea.l #sym,An; mulu.w
  (-d,A2),D0` through the link (sau5 MT_$ACQUIRE's nested routine
  0xE75C14..0xE75C22), and `muls.w` for a `.l` index of the same local.
- `nested/recursion` - codegen-patterns 6, 4: a recursive nested routine
  passing its own link (0xE397D0), a nested function reading the
  enclosing local through `(A6)`, A5 inherited by nested routines.
- Nested functions (codegen-patterns 6, 3, 12.4; bead
  domain-pascal-5aw.18):
  - `nested/fn_sibling` - a nested function called by a sibling
    (`movea.l An,A1`, 0xE13718 `movea.l A4,A1`) and by its parent (`movea.l A6,A1`,
    0xE13928), its link copied to A2 across its call (0xE135A4), the
    result computed in a scratch register and moved to D0 (0xE148E0);
  - `nested/fn_expr` - a nested function's D0 in its parent's arithmetic
    (copied first, 0xE01BA2) and in a comparison (0xE0082C);
  - `nested/fn_held` - a nested function's result variable held in D2
    across a call and moved to D0 (NETBUF_$RTNVA 0xE0ED4E, PKT_$NEXT_ID
    0xE124D0), the link in A2;
  - `nested/fn_pointer` - a nested pointer function, the memory form
    through its result slot into A0 (0xE6AA38), its A0 dereferenced by
    the parent (0xE4EA0E);
  - `nested/fn_uplevel` - the parent's by-value parameter, by-address
    parameter (slot, then object: 0xE135DA) and local read through the
    link;
  - `nested/fn_recursion` - a recursive nested function passing its own
    copy of A1 (0xE397D0), parameter and link in D2/A2 across the call,
    the result variable in D0, the unpopped call before the join
    (0xE04700).
- `params/value_copies` - codegen-patterns 1.2 ("Value parameters passed
  by address"), 2, 5.2, 12.4 R4(b) (bead domain-pascal-5aw.12): value
  parameters under the default convention copied on entry, in parameter
  order, at the top of the frame: byte, word and longword copies through
  A0, A1, A2 (FILE_$INVALIDATE 0xE75166..0xE75186), 6 bytes (TONE_$TIME
  0xE17300), 8 bytes with `(A0)+` after the entry load of the status slot
  (DIR_$GET_ENTRYU 0xE4D50E..0xE4D51A), 14 and 24 bytes as block copies;
  an assignment to a value parameter stored to the copy (`addq.w
  #1,(-2,A6)`), never through the caller's address; `addr` of it and
  passing it to a `var` parameter name the copy (0xE4D55E `pea (-8,A6)`).
- `params/addr_in` - codegen-patterns 5.2 ("`addr` of a parameter"), 12.2
  (bead domain-pascal-mbtx): `addr` of an `in` parameter or a part of
  one.  By address: stored from the slot when that is its one use,
  `move.l (8,A6),(d,A6)` (PKT_$COPY_TO_PA 0xE12538), or from the register
  holding the slot, `move.l A0,(4,A0)` (EC_$INIT 0xE15208); by value as
  the parameter passed on by address (`move.l (d,A6),-(SP)`, `pea
  (k,An)`); a field stored, `lea (k,An),Am; move.l Am,<dst>`.  Under
  `val_param` an `in` integer32's slot: `pea (d,A6)`, `lea (d,A6),An`.
- `params/assigned` - codegen-patterns 12.2 ("Assigned parameters"), 12.5
  (bead domain-pascal-jyba): by-value parameters assigned in the register
  of their entry load: in place (mst_$set_mstes 0xE43E36 `ori.w #2,D0`;
  MMAP_$ALLOC_PURE 0xE0D7F0 `sub.w D6,D2`), constants (0xE4BD7C,
  0xE1594A), `min` (0xE5961C), a computation from the parameter made in
  its register (area_$internal_create 0xE077FE; PROC1_$ALLOC_STACK
  0xE15040 and its `subi.w #0x400,D2; bne`, 0xE150EA; pacct_$compress
  0xE5A9E4), a narrowed `&` at the character's width (tty_$i_rcv
  0xE1B966 `andi.b #0x7f,D3`); in their slots: read by a nested routine
  (area_$resize 0xE0883C..0xE0884C), only pushed (AST_$GET_SEG_MAP
  0xE06BC6 `clr.l (0x10,A6)`).
- `params/subrange_var_args` - codegen-patterns 1.2, 13, 12.5 (bead
  domain-pascal-4tm; reference-clarifications Q38): an integer subrange
  and an integer type of the same size passed to each other's `var`,
  `out` and `in out` parameters, directly and through a routine pointer,
  are pushed with the same `pea (d,A6)` as an exactly-typed argument
  (TONE_$TIME 0xE1732C `pea (-8,A6)`); an integer constant outside an
  integer subrange is stored, pushed (a byte `val_param` push too),
  pooled for a by-address argument, loaded into a register-held local or
  given as a static initial value as its low bytes, and stored to a
  packed bit field or initialiser as its low bits (no image site).
- `packed/fields` - codegen-patterns 5.3, 9 (bead domain-pascal-xlm):
  fields of packed records (ref 3.10.7): `status_$t` as Apollo's
  base.ins.pas declares it, `bset #7` for `fail` after `tst.w (2,A0)` of
  `code` (PROC2_$FORK 0xE72CC2), a computed boolean merged into bit 7
  (OS_$GET_EC 0xE6D6DE), a boolean field as a result with `btst; sne`
  (XPD_$INHERIT_PTRACE_OPTIONS 0xE5B17C), an enumeration field read with
  the mask, `and.b`, `lsr.w` and set with `andi`/`ori` (RIP 0xE158B6),
  a word variable's low byte merged into a longword unit (APP_$RECEIVE
  0xE00864).
- `packed/bool_unreduced` - codegen-patterns 5.3 ("A boolean constant";
  bead domain-pascal-6zdt): a boolean field set or cleared on an indexed
  or absolute operand with the bit's number in the word (`bclr #15`,
  VTOCE_$TRUNCATE 0xE3A07A and 0xE3A0AE), through a pointer with the
  bit's number in its byte (PROC2_$INIT 0xE30560); a boolean in the low
  byte on an indexed operand, `bclr #k` on its byte (best effort: the
  image's `andi.w` on the word, VTOCE_$WRITE 0xE39786).
- `packed/word_sign` - codegen-patterns 5.3 ("Boolean field, test"; bead
  domain-pascal-g9kx): a boolean in bit 15 of a word of its record tested
  as the word's sign, `tst.w`/`bmi`, `smi` as a value, on another unit's
  record (sau5 MT_$ACQUIRE 0xE75CFE, PBU_$UNWIRE_USER_PAGES 0xE59D90),
  through a pointer (MSG_$$SEND 0xE0DBBA) and for `status_$t.fail`
  (0xE01C2C); bits 14..8 of another unit's record loaded, `move.w;
  btst #k,Dn` (PBU_$WIRE_PBU 0xE5A436, 0xE59D98); stores keep the byte
  (PBU_$INIT 0xE5A680); the low byte's top bit stays `tst.b` (0xE066B2).
- `packed/bool_arrays` - codegen-patterns 5.2 ("Packed arrays"; bead
  domain-pascal-51e): PCHIST's `packed array[1..64] of boolean` at +0x18
  of its data: tests on the index's low byte (PCHIST_$COUNT 0xE1A156,
  PCHIST_$UNIX_PROFIL_FORK 0xE5CC44 and 0xE5CC9E), `not` with `bne`, set
  with `move.l #0x80; lsr.l; or.b` (0xE5CCDC), cleared with `#-0x81` and
  `and.b` (PCHIST_$UNIX_PROFIL_CNTL 0xE5CBF2), a function result with
  `sne` (ACL_$USED_SUSER 0xE492FA), a register-held `for` index (ACL_$INIT
  0xE310E4); FIM_'s frame size table as an initialised `packed array of
  0..255`.
