# Link tests against ../source

CLAUDE.md section 6, item 4 (milestone 7; bead domain-pascal-xdh.1).
A dpc object takes the place of the C objects of the same routines in a
**scratch copy** of `../source`'s SAU2 kernel build, and that build's own
`make check` must pass.  `../source` is only read.

    make linktest                                   # build dpc, run the manifest
                                                    # against ../source HEAD (LINKTEST_REV)
    python3 tools/linktest.py run [MANIFEST] [--only FILE] [--keep-going] [-j N]
                                  [--rev REV]

A run takes about 20 s (most of it `../source`'s `make check`), so it is
not part of `make test`.  The harness logic has unit tests that need no
build: `tools/tests/test_linktest.py`.

## How ../source builds and checks the kernel

Read from `../source/domain_os/Makefile`, `sau2.ld` and `tools/` (2026-10-13).

- **Objects.**  Every `<subsystem>/*.c` is compiled with `m68k-elf-gcc
  -mcpu=68010 -O2 -ffunction-sections -fdata-sections` (one input section
  `.text.<symbol>` per function, one C file per routine as a rule), and
  every `<subsystem>/sau2/*.s` (hand-written assembly) with the gcc driver
  and `-mcpu=68020 -m68881`.  `ALL_SUBSYSTEM_OBJS` is the wildcard list of
  both.  No libgcc: the link refuses any reference to it.
- **Layout.**  Nothing is placed at a fixed address.
  `tools/gen_layout_ld.py` writes `build/sau2/layout.ld` (INCLUDEd inside
  `.text` of `sau2.ld`) from the map and the objects' symbol tables: a code
  input section (`.text` or `.text.*`, any name) is keyed by the earliest
  map symbol it defines and the sections are emitted in map order; module
  data blocks (`MODULE_DATA_DEFINE` sites in C, sections `.moddata.<name>`
  / `.bss.moddata.<name>`) are keyed by the address literal of their C
  site; sections without a map symbol follow their "owner" (the
  referencing section).  A section whose map symbols straddle another
  placed section is an error.  A plain `.text` section is written as
  `KEEP(<object>(".text"))`, so dpc's default section links as is.
- **`make check`** = `check-layout` (static checks, a scratch link with
  unresolved symbols ignored, then "every placed map symbol in map order"
  on that ELF), `check-asm` (`tools/asm_compare.py`: the hand-written
  objects only, compared with the image run by run, every relocation
  resolved to the image's operand), `check-guards`, `check-iodefs`,
  `check-rfc` (the real link with `--emit-relocs`, the RFC load image and
  its fixup table built from the ELF's `R_68K_32` cells, header/COLD/DUMP
  against the original, a dry-run of sysboot + COLD's MMU map) and
  `check-libgcc`.  It is **not** a byte compare of the linked kernel with
  the image: the C code is not byte-identical and addresses are not
  pinned.  The byte compare exists only for the hand-written assembly.
- **`make test`** builds host programs that `#include` the `.c` under test;
  it never sees an m68k object, so a swap cannot affect it and the
  harness does not run it.
- Toolchain: `m68k-elf-gcc` 15.2.0, binutils `m68k-elf-*` (Homebrew).  A
  clean `make -j8 check` takes about 22 s.

## What the harness does (tools/linktest.py)

1. `rsync` `../source/domain_os/` to `tmp/linktest/source/domain_os/`
   (`build/`, `dist/`, `__pycache__` excluded; the scratch `build/` is kept
   between runs).  rsync compares contents and gives a changed file the
   current time, so make rebuilds exactly what changed.  Only `domain_os/`
   is needed: nothing in the build reads outside it except the map and the
   image, which it finds under `$HOME`.
   - By default the copy is `../source`'s **working tree**, including other
     agents' uncommitted edits in progress.  A failure in files someone is
     editing there (e.g. `tty/`) may be theirs, not dpc's.
   - `--rev REV` copies the commit REV instead (`git archive` of
     `domain_os/` into `tmp/linktest/export/`), so a run is reproducible.
     The commit id is printed.
   - Either way `../source` is only read; `tools/tests/test_linktest.py`
     checks that on a fake tree.
2. Insert `-include linktest.mk` into the scratch `Makefile` just above
   `# Main target`: after `ALL_SUBSYSTEM_OBJS` is complete, before the
   first rule that uses it as a prerequisite.  The harness writes
   `linktest.mk`: it keeps the full list as the target `linktest-objs`, then
   sets `ALL_SUBSYSTEM_OBJS` to that list minus the dropped objects plus the
   dpc objects.  Every rule (link, layout, libgcc and RFC checks) then sees
   the swapped list.  After writing it, the harness reads the variable back
   from `make -p` and refuses to go on if the swap did not take.
3. Compile each manifest unit with `target/debug/dpc` with the options
   `tools/conform.py` gives the same unit (`--fragment-origin` for a
   fragment, loop-align and a5 options), and assemble it with `m68k-elf-as
   -m68010` into `domain_os/linktest/<stem>.o`.  So the object linked is the
   object the conformance gate checks.
4. `make linktest-objs`, then `m68k-elf-nm` over every object.  Each global
   that the dpc object defines must be defined only by C objects that
   define nothing else global.  Hand-written assembly is never dropped.
   Those objects are dropped.  The C sources stay, so host tests are
   unaffected.
   **Whole-module units** (bead domain-pascal-xdh.2): a unit whose
   routines span a whole map code segment (`tools/conform.py` gives it no
   fragment option: gpu.pas, pbu.pas, iic.pas, ...) replaces all of the
   module's C code there, so C globals that `../source` invented (no map
   name: a C name for a module-local routine, a shared helper) may go
   with it.  `m68k-elf-nm -u` over every object of the link decides:
   - a definer of the unit's globals may also define such helpers;
   - an object in the same directory as a definer whose globals are all
     such helpers, referenced only by dropped objects, is dropped too
     (repeated until nothing changes);
   - a helper still referenced by an object that stays refuses the unit,
     and so does a definer with a map name outside the unit (that routine
     lives elsewhere in the image).
   For any other unit the refusal of an object with a global the unit
   does not define stands.
   `tools/modsurvey.py` ranks the map's code segments as candidates for
   such units (C objects, blockers, helpers, module data).  A global that no object defines is linked in addition
   (IO_$ALLOC: `../source` has no such routine), with a note.  A unit
   with module data is mapped onto `../source`'s C data first (below,
   "Module data") and compiled again with `dpc --data-base`.
   Then dpc's run-time library (bead domain-pascal-w586;
   `runtime/README.md`): the archive `dpc --print-runtime=m68k` names is
   extracted into `linktest/runtime/`, and each of its objects replaces
   the one `../source` object that defines its globals, which must define
   exactly those names (`tools/linktest.py` `plan_runtime_members`):
   - `m_arith.o` (M$ARITH, thirteen names) replaces the copy `../source`
     extracts from the same archive, `build/sau2/runtime/m_arith.o` (once
     its own `build/sau2/math/sau2/arith.o`);
   - `pas_set_build.o` (PAS_$SET_BUILD, bead domain-pascal-jzm5) replaces
     `../source`'s C stand-in, `build/sau2/pas/set_build.o`.

   Two members may not replace one object.  The kernel is linked with the
   runtime dpc ships; steps 7 and 8 check it like a unit.
5. Undefined symbols of the dpc objects that nothing in the link defines
   must be map names at or above IODEFS_GUARD 0xF4FC00, i.e. absolute
   addresses in the I/O region.  They are defined in `linktest/absdefs.o`
   with `.equ`, as `../source`'s `arch/m68k/sau2/iodefs.s` does for its
   I/O pages.  Today there is one: AST_$ZERO_BUFF 0xFF8C00.  Any other
   undefined symbol is a failure.
6. `make check` in the scratch copy, after deleting the scratch
   `domain_os.elf`, `layout.ld` and `layout-check.elf`, so the kernel is
   always relinked from the current object list.
7. Each global of each dpc unit must appear once in the linked
   `build/sau2/domain_os.elf`, in code.  A `define`d variable of a block
   now owned by C must, if `../source` also names it, be at the owner's
   address plus its offset in the unit.
8. `../source`'s `tools/asm_compare.py` is run over the dpc objects and
   the runtime object.
   - Reference bytes are appended from the image file (window `VA =
     offset + 0xDFFC00`) to a copy of its reference file,
     `tmp/linktest/asm_ref.txt`.
   - The script is imported and driven from the harness.  Two changes are
     made inside that process only:
     - each `0xADDR` spec's dpc symbol is added to its `LOCAL_SYMBOLS`
       (ZERO_PAGE = 0xE00EB0), so that the worker's run is compared;
     - its `KNOWN_DEVIATIONS` table is emptied.  Those entries are about
       hand-written files, and the script reports them as stale when their
       objects are not given;
     - each C data owner (below) is added to `LOCAL_SYMBOLS` at its image
       address.  asm_compare resolves a block or a map name itself, but
       not a `DATA_ANCHORS` name (TERM_$DATA).
   - Any relocation it leaves unverified fails the run, so the `lea
     OWNER.l,A5` of every data unit is proved against the image's operand.
   - Result: every byte of every dpc object equals the image, and every
     relocation resolves to the image's operand.  This is the same claim
     that `make conform` makes, re-proved by `../source`'s independent
     tool.

### No section renaming

dpc's section names needed no change.  `.text` and `.text.<UNIT>.<SECTION>`
are both code sections to `gen_layout_ld.py`, which keys them by their map
symbols, not by name.  `objcopy --rename-section` is not used.

### Module data (bead domain-pascal-3bp)

A unit's static data is the NOBITS block `.bss.moddata.<UNIT>[.<SECTION>]`
labelled `.Ldata[.<SECTION>]` (codegen-patterns 4 and 11), or the block
with contents `.moddata.<UNIT>[.<SECTION>]` when it has initial values.  In `../source`
the same image bytes already belong to a C object that the module's other
C routines use: a `MODULE_DATA_DEFINE` block (PROC1_$DATA), a C object
with a map name (CAL_$TIMEZONE) or one `gen_layout_ld.py` keys by
`DATA_ANCHORS` (TERM_$DATA).  A second block would be an orphan section
(SCSI, when tried: `sau2.ld`'s AST_PMAPS_END assertion failed), and two
copies of the data would not be one module anyway.  So **C owns the bytes,
and the dpc unit refers to them**:

- `dpc --data-base=SECTION=SYMBOL` (repeatable; link-test scaffolding,
  `crates/dpc/src/main.rs`): for Pascal data section SECTION (`.data` for
  the default), dpc allocates no block, does not define its `define`d
  variables, and writes `.set .Ldata[.SECTION],SYMBOL`.  gas makes every
  reference to the label a relocation against SYMBOL, so the code is byte
  for byte the code `make conform` checks: only the `lea` relocation's
  symbol changes.  dpc also writes the absolute local symbol
  `__dpc_data_size[.SECTION]`, the bytes the unit declares there.
- The harness takes the block's image address from the manifest's
  `--section-addr [.bss].moddata...=0xADDR` (the address the conformance
  gate proves; a unit test checks the two manifests agree) and finds the
  one C data object of the link that starts there, from `../source`'s own
  placement facts only: `MODULE_DATA_DEFINE` address literals
  (`gen_layout_ld.scan_sources`), map names, `DATA_ANCHORS`.  None, or
  more than one, refuses the unit.
- **Offsets.**  A5-relative accesses carry no relocation, so the unit's
  offsets must mean the same bytes in the C object.  The conformance gate
  ties the unit's offsets to the image.  The C object's offset for the
  same datum is tied to the image only where `../source` says so: a
  `_Static_assert` on the field, or a raw offset written from the image
  (TONE_$CHANNEL is `&TERM_$DATA + 0x1268`).  Neither is checked by the
  harness, so it requires and reports what it can:
  - `__dpc_data_size` ≤ the C object's size (`nm -S`), so that no offset
    of the unit leaves the object.  Past its end lie other C objects that
    the link lays out on its own (CAL_$TIMEZONE is 0xC bytes;
    cal_shutdown.pas reaches offset 0x10).  This bounds the offsets; it
    does not show that they hit the same fields.
  - For a unit with one data block it prints the A5 offsets the unit uses
    (from its assembly) and which of them the C objects it replaces also
    reach as `OWNER+offset` relocations; the rest rest on `../source`'s
    struct layout alone (FIM_$GET_FIM_ADDR's 0x3C: the C indexes
    `FIM_$DATA` with `(as_id + 15) * 4`; `fim/fim.h` asserts the offset).
    Informational: an offset the C reaches some other way is not a
    mismatch.
- **Initial contents** (bead domain-pascal-63t).  A block with initial
  values (`.moddata.*`) says what its bytes must be, and `make conform`
  ties them to the image.  Before compiling with `--data-base`, the
  harness takes the block's bytes from the dpc object (`objcopy -O
  binary --only-section`) and compares the declared ones
  (`__dpc_data_size`) with the C owner's initial bytes: its section's
  contents at its symbol, or zeros for a NOBITS C object.  A difference
  fails the unit, and so does a relocation in the compared C bytes (a
  pointer, not final in the object).  fim_get_fim_addr.pas: its 0x134
  bytes (the frame size table at +0x124) equal FIM_$DATA's
  (`fim/fim_data.c`, `MODULE_DATA_DEFINE_INIT`); a changed table byte
  fails with "initial bytes differ from FIM_$DATA ... first at +0x12C"
  (tried, 2026-10-13).
- **Proof.**  `asm_compare.py` resolves the `lea` relocation to the
  owner's image address and checks it against the image's operand
  (`lea 0x00E254E8,A5` at PROC1_$GET_LOADAV 0xE14BC0).

Rejected: renaming dpc's block into the C block's section with `objcopy`
and dropping the C definition.  The other C routines of the module use
the C object; its initial contents (`MODULE_DATA_DEFINE_INIT`) would be
lost; the unit's block is only a prefix of the module's.

## Results (2026-10-13)

`linktest.py run --rev HEAD tests/link/manifest.txt` (`../source`
b219e6ff; again with the whole-module units below at 778ae3a5): **OK**.  `make linktest` now passes `--rev $(LINKTEST_REV)`
(default HEAD); a run on `../source`'s working tree, with other agents'
uncommitted edits, passed too.

- 28 units are linked in place of 42 C objects (IO_$ALLOC and
  ETHERNET_$INIT are additions: they replace nothing).
- `../source`'s `make check` passes:
  - 1587 map symbols are placed in map order, two more than the baseline
    (IO_$ALLOC, ETHERNET_$INIT);
  - 0 order inversions and 0 sections in the catch-all;
  - `check-asm`, the guards, `check-iodefs` and `check-libgcc` pass;
  - `check-rfc` passes: full link, fixup table, dry-run loader.
- `asm_compare.py` over the 28 dpc objects: 28 sections identical modulo
  relocations, 42 relocations resolved to the image operand, 0 unverified.

| Unit | Replaces (scratch build) |
|---|---|
| proc1_tst_lock.pas (PROC1_$TST_LOCK, PROC1_$GET_LOCKS) | proc1/tst_lock.o, proc1/get_locks.o |
| time_q_init_queue.pas (TIME_$Q_INIT, _Q_INIT_QUEUE, _Q_FLUSH_QUEUE) | time/q_init.o, q_init_queue.o, q_flush_queue.o |
| ast_page_zero.pas (0xE00EB0, AST_$PAGE_ZERO) | ast/page_zero.o (misc/zero_page.o stays: other C callers) |
| iic.pas (the whole IIC module, 9 routines) | iic/{init,acquire,release,statistics,self_test,send,exists,receive,receive_check}.o |
| mt_release.pas (MT_$RELEASE) | mt/release.o |
| tone_time.pas (TONE_$TIME) | tone/time.o |
| name_convert_acl_status.pas | misc/name_convert_acl_status.o |
| proc1_set_asid.pas | proc1/set_asid.o |
| time_get_adjust.pas (calls M$ARITH) | time/get_adjust.o |
| disk_get_block.pas | disk/get_block.o |
| sysbus_define_int.pas (a constant cell, `pea (d,PC)`) | sysbus/define_int.o |
| disk_unassign_all.pas (`bsr.w DISK_$UNASSIGN`, module-routines=) | disk/unassign_all.o |
| pchist_interrupt.pas (`bsr.w PCHIST_$COUNT`, module-routines=) | pchist/interrupt.o |
| io_alloc.pas (IO_$ALLOC) | nothing: added |
| proc1_get_loadav.pas; data: PROC1_$DATA (0xC of 0xCC4 bytes) | proc1/get_loadav.o |
| proc2_get_boot_flags.pas; data: PROC2_$UNWIRED_DATA (0x1E6 of 0x1E8) | proc2/get_boot_flags.o |
| proc2_quit.pas (`bsr.w PROC2_$SIGNAL`); data: PROC2_$UNWIRED_DATA | proc2/quit.o |
| fim_get_fim_addr.pas; data: FIM_$DATA (0x134 of 0x134, initial bytes equal) | fim/get_fim_addr.o |
| smd_busy_wait.pas (SMD_$PUTC, _$BUSY_WAIT, _$LITES); data: SMD_$BLINK_TIMER_DATA | smd/{putc,busy_wait,lites}.o |
| mmap_alloc_contig.pas; data: MMAP_$DATA | mmap/alloc_contig.o |
| pkt_next_id.pas; data: PKT_$DATA (0x5E of 0xA8) | pkt/next_id.o |
| tone_enable.pas; data: TERM_$DATA (DATA_ANCHORS, 0x126C of 0x1398) | tone/enable.o |
| gpu.pas (whole GPU module: GPU_$INIT, GPU_$TERM) | gpu/init.o, gpu/term.o |
| pbu.pas (whole PBU module: PBU_$FREE_ASID, _$FAULTED_UNITS, _$INIT) | pbu/{free_asid,faulted_units,init}.o |
| ct_release.pas (whole CT module) | ct/release.o |
| tape_boot.pas (whole TAPE_BOOT module) | tape/boot.o |
| pkt_init.pas (PKT's whole OS_INIT_PROC segment) | pkt/init.o |
| ethernet_init.pas (whole ETHERNET module) | nothing: added |

No unit of these `define`s a variable, so the step 7 check of `define`d
names has had nothing to check yet (it has unit tests).

### A difference from ../source that the link showed

The image relocates the two `#0xFF8C00` operands of the module-local
routine 0xE00EB0: its fixup table lists cells 0xE00EBA and 0xE00ECE.  On a
68020 the loader moves the I/O window (fixup window FA0000.. +0x3000000).

- **dpc:** the unit names AST_$ZERO_BUFF, so the operands are relocations,
  and the scratch image has 17 fixups in that window instead of 15.
- **../source:** `misc/zero_page.c` writes the address as a literal
  (`ast/ast.h` AST_$ZERO_BUFF, `SAU2_AST_ZERO_BUFF`), so it has no fixup
  there.

This is a known, documented gap of `../source`, not a new one.
`../source/docs/rfc-cold-start.md` section 8a ("Fixup table") says the
table "cannot see literal addresses (C integer casts, `.equ`): our table
has 15 I/O and 1 PROT cells against the image's 205 and 56".  The two
ZERO_PAGE cells are among those 190 missing I/O cells.  The original also
relocates `0xFF8C00` / `0xFF8800` / `0xFF8400` cells at 0xE03B7E,
0xE04238, 0xE0A2B4, 0xE0A2C8, 0xE0F150 and 0xE0F160.  Checked 2026-10-13
by reading both fixup tables with `../source`'s `check_rfc.py` functions.
It is `../source`'s to change, not ours.

## Units that cannot link yet, and why

`--keep-going` leaves such units out and links the rest.  The run still
exits 1.  The harness refuses each case with a reason.  Manifest options
(`module-routines=NAME=0xADDR`, `enclosing=`) are refused when the
manifest is read.  The other cases are refused after compiling or after
`nm`:

- **Module data with no single C owner** (tried 2026-10-13, `../source`
  b219e6ff).  Each needs a change in `../source` (a `MODULE_DATA_DEFINE`
  block or a `DATA_ANCHORS` entry at the block's address), which is not
  ours to make:
  - no C object there that the link can place at an image address:
    scsi.pas (0xE88804: `../source` has no SCSI_ data), bat_get_bat_step.pas
    (0xE79478: the C array `bat_$volumes`, with a phantom element 0, keyed
    by no image address), audit_is_process_audited.pas
    (0xE854D8: AUDIT_$DATA, not a map name, not anchored),
    rem_name_register_server.pas (0xE7DBB8), mst_set_touch_ahead_cnt.pas
    (0xE7CF0C: MST_$PAGE_AVAIL_BITMAP and the other fields are separate C
    objects, placed by their users), atbus_ring_interrupt.pas (0xE22904);
  - the C object there is smaller than the unit's data:
    cal_shutdown.pas declares 0x12 bytes from 0xE7B030, CAL_$TIMEZONE is
    0xC.  `../source` is not short of the image here: the image block
    (OS_CAL_WIRED, size 0x14) holds three map names, CAL_$TIMEZONE
    (0xC), CAL_$LAST_VALID_TIME and CAL_$BOOT_VOLX, and `cal/cal_data.c`
    defines exactly those three objects; the unit reads CAL_$BOOT_VOLX
    at (0x10,A5), which is a separate C object that the link places on
    its own.
  xns_idp_get_stats.pas and xns_idp_get_port_info.pas do map
  (XNS_IDP_$DATA) but are refused for their code: `xns/idp_misc.o` holds
  four routines.
- **Only module-local routines** (sio_init_dtte.pas, rip_packet_length.pas,
  disk_set_chain_status.pas, vfmt_get_next_arg.pas; tty_wired_data.pas and
  tty_set_dfl_funcs.pas, the units with TTY's initialised data blocks,
  whose blocks `../source` has no single C owner for either).  dpc gives them local
  symbols, while the C callers name C functions (e.g. a C name for
  0xE32B76).  Such a unit would add unreachable code and replace nothing.
  Swapping it needs a whole-module unit whose callers come along, or a C
  name for the routine.
- **A C object that holds more than the unit.**  Example: `xpd/ptrace_opts.o`
  defines XPD_$RESET_PTRACE_OPTS with three other routines.  Dropping it
  would lose them.  Removing one function's `.text.<symbol>` section from a
  copy of the object (objcopy) is possible, but it is not done.
- **A `bsr.s` written as its encoding.**  file_set_len.pas,
  asknode_info.pas: `module-routines=` to a routine within `bsr.s` reach.
  gas cannot relocate a byte displacement to another object's symbol, so
  dpc writes `.short 0x61dd | bsr.s NAME`, with the displacement taken
  from the image addresses and no relocation.
  - When tried, such a unit linked, and `make check` and `asm_compare.py`
    both passed (the bytes are the image's).  But ASKNODE_$INFO's `bsr.s`
    reached ASKNODE_$GET_INFO+0x26 instead of ASKNODE_$INTERNET_INFO.
    Nothing in `../source` checks where a call lands.
  - The harness therefore refuses any unit whose assembly has such an
    encoding.
  - The fix is to compile the whole module, so the callee is in the same
    section.
  - By contrast, `module-routines=` units that use `bsr.w`
    (disk_unassign_all.pas, pchist_interrupt.pas) are a PC16 relocation.
    The link resolves it to `../source`'s C routine, and ld would fail
    loudly if the C routine were out of range.  Both are in the manifest.
  - `module-routines=NAME=0xADDR` (a module-local routine outside the
    unit, e.g. mst_maps.pas, term_send_kbd_string.pas) is refused: there
    is no such name in `../source`.
- **`enclosing=` units** (ring_discard_packet): their parent is a stand-in.
