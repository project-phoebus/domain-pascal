{ MMAP_$AVAIL, 0xE0CC64, 0x52 bytes of code, then the module-end filler
  and the routine's constant (tools/imgdis.sh E0CC64 58).  Semantics:
  ../source/domain_os/mmap/avail.c: put a page back on its working-set
  list; a list index above 0x45 crashes the system with
  status_$mmap_bad_avail (0x60004, the image's constant at 0xE0CCB8, 00
  06 00 04; ../source's name).

  A fragment of the MMAP_ module (code segment 0xE0C514), compiled with
  `dpc --fragment-origin 0xE0CC64'; its data area, `.bss.moddata.MMAP_',
  placed at the map's `D E23284 MMAP_' (manifest --section-addr), loaded
  in always mode (`lea 0xE23284,A5' with no use, as MMAP_$ALLOC_CONTIG,
  tests/conformance/mmap_alloc_contig.pas).  The module-local routine at
  0xE0C514 (mmap/mmap_internal.h's mmap_$add_to_wsl; the map names only
  its segment) is called with `bsr.w' (manifest module-routines).

  Patterns exercised (docs/codegen-patterns.md):
  - 5.2  an element of another unit's array as a `with' record: `move.l
         (8,A6),D2; movea.l #MMAP,A0; move.l D2,D0; lsl.l #4,D0; lea
         (0,A0,D0.l),A2' (0xE0CC72..0xE0CC80): the parameter loaded, the
         base, the copy and the scaling, a longword index; the record
         live across the CRASH_SYSTEM call in callee-saved A2, the base
         in A0 (bead domain-pascal-o1kl, "The element's address as a
         value"); the lower bound 16#200 folded into each reference,
         `cmpi.b #0x45,(-0x1ffc,A2)', `pea (-0x2000,A2)';
  - 1.5  a constant passed by address from the code section, `pea
         (0x32,PC)', after the 0x2048 filler;
  - 1.3  a boolean `true' pushed by value as `st -(SP)', a byte widened
         to a word `clr.w D0; move.b ...,D0; move.w D0,-(SP)';
  - 4    always mode: `lea 0xE23284,A5', restored by the `movem'.

  Reconstructed (not known from the original): the names (the C's);
  the entry's layout from ../source's mmap/mmap.ins.pas (mmap_$mmape_t),
  with `wsl_index' a char: the image compares it with `cmpi.b' and an
  unsigned `bls' and pushes it zero-extended, which a char gives (a
  `[byte] 0..255' field, as mmap.ins.pas has it, dpc compares widened,
  codegen-patterns 7.1); `with ... : page' (the alias form, ref "With",
  p. 4-212) to pass the record itself; `mmap_$data_stand_in', a
  variable only so that the unit has a data area.

  The frame, `link #-4': the `with' temporary's slot, held in A2 and
  never addressed (codegen-patterns 2, "Frame size"; bead
  domain-pascal-s9g0). }
module mmap_;
const
  status_$mmap_bad_avail = 16#00060004;
type
  status_$t = record
    all: integer32;
  end;
  mmap_$byte_t = [byte] 0..255;
  pinteger = 0..32767;
  mmap_$mmape_t = record
    wire_count: mmap_$byte_t;             { 0x00 }
    seg_offset: mmap_$byte_t;             { 0x01 }
    segment: pinteger;                    { 0x02 }
    wsl_index: char;                      { 0x04 }
    in_wsl: boolean;                      { 0x05 }
    prev_vpn: pinteger;                   { 0x06 }
    priority: mmap_$byte_t;               { 0x08 }
    flags2_7: boolean;                    { 0x09 }
    next_vpn: pinteger;                   { 0x0A }
    disk_addr: integer32;                 { 0x0C }
  end;
var
  mmap: extern array[16#200..16#FFF] of mmap_$mmape_t;
  mmap_$data_stand_in: integer32;

procedure crash_system(in status: univ status_$t); extern;
procedure mmap_$add_to_wsl(var page: mmap_$mmape_t; vpn: integer32;
                           wsl_index: integer; head: boolean);
  options(val_param); internal; extern;

procedure mmap_$avail(vpn: integer32); options(val_param);
begin
  with mmap[vpn]: page do begin
    if page.wsl_index > chr(16#45) then
      crash_system(status_$mmap_bad_avail);
    mmap_$add_to_wsl(page, vpn, ord(page.wsl_index), true);
  end;
end;
