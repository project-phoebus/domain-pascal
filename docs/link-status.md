# Link-first status (2026-10-20)

The owner's priority of 2026-10-17 was to make every ../source module compile
with dpc, using best-effort but correct code, and then link the kernel.
Fidelity work comes afterwards.  This page records where that stands.  Its
counts come from `tools/linktest.py`, run against ../source 39f0fe4d.

## Compiling: 110 of 110 module files

Every module file compiles with dpc's default (non-strict) mode.  We went
from 24 of 109 at the start of the link-first work.

Where dpc had refused only because the image does not settle the exact
form, the forms are now best effort:
- the warning reads "best effort (not byte-identical): ...";
- `dpc --strict` turns each one back into the old refusal.

srcsurvey, conformance and the goldens run with `--strict`, so their
fidelity numbers stay meaningful.  The srcsurvey baseline is OK 268,
MISMATCH 402, SETUP 22, STUB 849 of 1541.  Many routines are STUB there
only because of `--strict`.

Correctness checks:
- Every best-effort path has a Musashi exec test.
- Every batch was reviewed by a separate agent, with adversarial exec cases.
- New: `tools/dispcheck.py` checks every assembled 16-bit displacement
  against gas's listing.  gas wraps out-of-range displacements without
  saying so.  exectest and linktest both run it.

## Linking: blocked in ../source

`make domain_os check` stops in ../source's `tools/gen_layout_ld.py` before
`ld` runs.

To see what lies beyond, we probed a scratch export (tmp/ only; ../source
was not touched), with the two checks below relaxed.  The remaining
failures were these, in order:

1. **DISK_ module data**
   (`build/sau2/pascal/disk/{disk,disk_dev}.o: .{bss.,}moddata.DISK_: the
   map has no single data segment of module DISK_`).
   - DISK_ is three compilation units (disk.pas, disk_dev.pas,
     disk_vol.pas) that share one module name.
   - The map has three DISK_ `.DATA` segments: 0xE7A1CC, 0xE7AD5C and
     0xE826C4.
   - gen_layout_ld keys dpc's `.moddata.<MODULE>` blocks by module and
     section only, and then reports "DISK_ already defined".
   - Each unit's header names its segment.  gen_layout_ld needs a way to
     tell the units apart, for example by object file.  dpc emits what the
     Pascal says (`module disk_;` in all three units).
2. **The residency check** (`pmap.o(.text.PMAP_.OS_INIT_PROC) (boot-only) is
   less resident than its referencer pmap.o(.text.PMAP_.WIRED_PROC)`).
   - In the image, the wired PMAP_$PURIFIER_L calls the boot-only
     pmap_$init_timers (`jsr 0x00e2f880` at 0xE13B22, quoted in
     pmap/purifier_l.pas).
   - So the check is stricter than the original kernel.
3. **Undefined `ROUTE_$PORT` (18 references) and `NETWORK_$ME` (13).**
   These were the only undefined symbols in the whole link.
   - They are the start of the map's NET_PORT_TABLE overlay section,
     0xE2E0A0.
   - ../source declares them `extern` only, because dpc used to refuse a
     `define` in an overlay section (route/route_wired.pas TODO).
   - dpc now supports that (bead domain-pascal-m15q).  ../source needs two
     changes:
     - `route/route.ins.pas`: put `route_$port` in its own
       `var (net_port_table)` part, then reopen a plain `var` for the
       externs after it.
     - `route/route_wired.pas`: `define route_$port;`.  The image's
       initial values can be given there too (port_type 1 and socket
       16#FFFF at +0x2E of each record).
   - gen_layout_ld already places the resulting `.bss.overlay.NET_PORT_TABLE`
     (or `.data.overlay.`) at ROUTE_$PORT's map address.
   - **NETWORK_$ME** (and DCTE_$NULL) share that address.  Pascal cannot
     define two variables at the same offset, so one of these is needed:
     - (a) an alias in the linker script, `NETWORK_$ME = ROUTE_$PORT;`;
     - (b) rewriting its users as `route_$port[0].network`, which gives
       the same absolute operand.

     Owner's choice.

Beads: domain-pascal-6jo5 (gen_layout_ld: items 1 and 2 and the
hardening below), domain-pascal-pu4d (NETWORK_$ME).

Suggested gen_layout_ld hardening (../source's to make):
- Refuse a COMMON overlay name that some object also defines.  Otherwise
  the units silently get two addresses.
- Check a defined overlay section's size against its map segment.

## Decisions made overnight, for the owner to confirm

- **Open question 47** (`for` bounds of another width than the index):
  - a bound of the index's own width is compared with the index's
    signedness, as PCHIST's nested routine does at 0xE5CD1A;
  - a wider bound is compared at its own width, so an empty range runs no
    pass.
- **Open question 30** (`goto` into a `for`/`while`/`repeat`/`with`/`case`):
  - compiled as a plain jump, as best effort;
  - refused when the code at the label can reach an entered `for`'s end
    or read an entered `with`'s record;
  - the kernel's three sites (MST_$TOUCH, MST_$INVALIDATE, MST_$FORK)
    compile;
  - bead domain-pascal-3ah7.
- **One-word set `in`** with a run-time element: always guarded, as AST
  does at 0xE01C0A.  Some image sites are unguarded (bead 87ec).
- **Unsigned value against 0:**
  - a type whose values fit the signed range (pinteger, 0..2^31-1) gets a
    sign test, as BAT_$FREE 0xE3B5B0 and PKT 0xE122D6 do;
  - AUDIT 0xE4BEFE conflicts with this (bead soud).
- **A constant outside the operand's type** is compared on its bits, not
  folded.  The kernel passes 16#FFFFFFFF into 0..2^31-1 parameters.
  Ordered relations against a negative constant compare signed (ref
  4.3.3).
- **Out-of-range constant index:** accepted only as the last index of
  `addr(...)`, with a warning (rem_file/server.pas:822,
  `addr(response.bytes[16#200])`).
- **Packed fields passed to value/`in` parameters** are allowed, although
  ref 3.10.3 forbids passing packed fields.
- **Results:**
  - set results of one word or one longword come back in D0, from the
    image (ACL_$MIN_RIGHTS, PROC2_$SIGBLOCK);
  - `integer32` results with `a0_return` come back in A0.
- **Overlay members** must sit in their declaration-order slots (an
  `extern` member takes no slot until defined): dpc refuses other
  shapes rather than let units disagree; open question 48.
- **Register spilling** has no image counterpart: frame slots, parking in
  data registers, and reloads from parameter slots.  Bead ei42 is to find
  the original's rule.

## Next: fidelity

Each best-effort form carries its warning text and a docs note in
docs/codegen-patterns.md.  The fidelity beads are P3/P4, among them 4akc,
f4zs, p20u, 87ec, bl45, 7voe, svrc, ladm, lwga and 91x6.
`tools/linktest.py`'s report ranks the warnings by module ("best-effort
warnings" section), and so gives the order in which to settle them against
the image.
