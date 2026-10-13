{ MMAP_$ALLOC_CONTIG, 0xE0D8F0, 0x24 bytes (tools/imgdis.sh E0D8F0 24).
  Semantics: ../source/domain_os/mmap/alloc_contig.c: no contiguous pages
  are ever allocated; the count is cleared and the status set to
  status_$mmap_contig_pages_unavailable (0x6000E; 10.2 status-code
  database, "OS / MMAP manager": "contiguous pages unavailable").

  A fragment of the MMAP_ module (code segment 0xE0C514), compiled with
  `dpc --fragment-origin 0xE0D8F0'.  Its data area, `.bss.moddata.MMAP_',
  is placed at the map's `D E23284 MMAP_' (manifest --section-addr).

  MMAP_$ALLOC_CONTIG is the first `s' routine of its segment (codegen-
  patterns 4; `tools/a5survey.py switch MMAP_', segment 0xE0C514: every
  routine before it is `S'), so it loads A5 without using the data: the
  manifest passes `a5-always=MMAP_$ALLOC_CONTIG' (an always-from switch at
  it would do the same).

  Patterns exercised (docs/codegen-patterns.md):
  - 4    always mode: `pea (A5); lea 0xE23284,A5' ... `movea.l
         (-4,A6),A5' (0xE0D8F4, 0xE0D90C);
  - 1.2, 1.3  val_param: a word by value at (8,A6), never read; the
         by-address slots at (0xa,A6) and (0xe,A6);
  - 9, 12.4 R4(b)  two slots used once, A0 then A1: `clr.l (A0)',
         `move.l #0x6000e,(A1)' (0xE0D8FC..0xE0D906).

  Reconstructed (not known from the original): the names (the routine's
  from the map, the others from ../source); `n' as integer (the C has
  uint16_t, never read); `out' for both by-address parameters (`var'
  passes the same address, ref 5.2 Table 5-1); val_param (a word slot at
  (8,A6) followed by slots at 0xa and 0xe: the first argument is passed
  by value, ref 5.2 Table 5-1); `mmap_$data_stand_in', a variable only so
  that the unit has a data area (this routine reads none of it).  The
  status constant is ../source's name. }
module mmap_;
const
  status_$mmap_contig_pages_unavailable = 16#0006000E;
type
  status_$t = record
    all: integer32;
  end;
var
  mmap_$data_stand_in: integer32;   { stand-in, see above }

procedure mmap_$alloc_contig(n: integer; out pages_alloced: integer32;
                             out status: status_$t); options(val_param);
begin
  pages_alloced := 0;
  status.all := status_$mmap_contig_pages_unavailable;
end;
