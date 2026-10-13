# Link tests against ../source

CLAUDE.md section 6, item 4 (milestone 7; bead domain-pascal-xdh.1).
A dpc object takes the place of the C objects of the same routines in a
**scratch copy** of `../source`'s SAU2 kernel build, and that build's own
`make check` must pass.  `../source` is only read.

    make linktest                                   # build dpc, run the manifest
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
   unaffected.  A global that no object defines is linked in addition
   (IO_$ALLOC: `../source` has no such routine), with a note.
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
   `build/sau2/domain_os.elf`, in code.
8. `../source`'s `tools/asm_compare.py` is run over the dpc objects.
   - Reference bytes are appended from the image file (window `VA =
     offset + 0xDFFC00`) to a copy of its reference file,
     `tmp/linktest/asm_ref.txt`.
   - The script is imported and driven from the harness.  Two changes are
     made inside that process only:
     - each `0xADDR` spec's dpc symbol is added to its `LOCAL_SYMBOLS`
       (ZERO_PAGE = 0xE00EB0), so that the worker's run is compared;
     - its `KNOWN_DEVIATIONS` table is emptied.  Those entries are about
       hand-written files, and the script reports them as stale when their
       objects are not given.
   - Result: every byte of every dpc object equals the image, and every
     relocation resolves to the image's operand.  This is the same claim
     that `make conform` makes, re-proved by `../source`'s independent
     tool.

### No section renaming

dpc's section names needed no change.  `.text` and `.text.<UNIT>.<SECTION>`
are both code sections to `gen_layout_ld.py`, which keys them by their map
symbols, not by name.  `objcopy --rename-section` is not used.

## Results (2026-10-13)

`make linktest`: **OK**.

- 14 units are linked in place of 24 C objects (IO_$ALLOC is an addition: it replaces nothing).
- `../source`'s `make check` passes:
  - 1586 map symbols are placed in map order, one more than the baseline
    (IO_$ALLOC);
  - 0 order inversions and 0 sections in the catch-all;
  - `check-asm`, the guards, `check-iodefs` and `check-libgcc` pass;
  - `check-rfc` passes: full link, fixup table, dry-run loader.
- `asm_compare.py` over the 14 dpc objects: 14 sections identical modulo
  relocations, 24 relocations resolved to the image operand, 0 unverified.

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
(a data `--section-addr`, `module-routines=NAME=0xADDR`, `enclosing=`) are
refused when the manifest is read.  The other cases are refused after
compiling or after `nm`:

- **Module data** (bead domain-pascal-3bp).  These dpc objects carry a
  `.bss.moddata.<UNIT>[.<SECTION>]` block: scsi.pas (SCSI_$PRESENT),
  cal_shutdown.pas, proc2_get_boot_flags.pas, proc1_get_loadav.pas,
  pkt_next_id.pas, mst_*.pas, ...  `../source` has no `MODULE_DATA_DEFINE`
  site for such a section.  When tried, the section became an orphan after
  `.bss`, and `sau2.ld`'s assertion `AST_PMAPS_END ... must be
  AREA_$RPMAP_CACHE` failed the link.  The block has to map onto (or
  replace) the C block keyed at the same image address.
- **Only module-local routines** (sio_init_dtte.pas, rip_packet_length.pas,
  disk_set_chain_status.pas, vfmt_get_next_arg.pas).  dpc gives them local
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
