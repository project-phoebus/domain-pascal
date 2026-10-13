{ PROC1_$GET_LOADAV, 0xE14BBA, 0x20 bytes (tools/imgdis.sh E14BBA 20).
  Semantics: ../source/domain_os/proc1/get_loadav.c; data layout:
  proc1_$data_t in ../source/domain_os/proc1/proc1.h (loadav[3], three
  longwords at +0 of the PROC1_ data block, _Static_assert "loadav (A5)";
  map `D E254E8 PROC1_ size = CC4').

  The unit is a fragment of the PROC1_ module (code segment 0xE1471C), so
  tools/conform.py compiles it with `dpc --fragment-origin 0xE14BBA'.  The
  module heading names the map's sections, WIRED_PROC and WIRED_DATA
  (codegen-patterns 11; the bytes are the same with the default
  sections): the code is `.text.PROC1_.WIRED_PROC', the data area the
  NOBITS section `.bss.moddata.PROC1_.WIRED_DATA', placed at the PROC1_
  data segment by the manifest (`--section-addr'); the `lea' of the data
  base is then checked symbolically against 0xE254E8.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2  default convention: the out parameter is passed by address,
         (8,A6) holds its address;
  - 2    link.w A6,#0 with no locals;
  - 4    an exported routine that uses its unit's data saves A5 with
         `pea (A5)' (the only saved register, 12.1), loads it with the
         absolute address of the data area, restores it with
         `movea.l (-4,A6),A5' before unlk;
  - 5.2  a 12-byte array assignment as three `move.l (A0)+,(A1)+', source
         address first (`lea (A5),A0' for offset 0 of the data area, not
         `movea.l A5,A0'), then the destination (`movea.l (8,A6),A1').

  Reconstructed (not known from the original): the names; the array type
  and its bounds (the C has int32_t[3], 8.24 fixed point; a record of
  three integer32 fields would compile the same); `out' as the parameter
  mode (`var' passes the address the same way under the default
  convention, ref 5.2 Table 5-1, so the image cannot tell them apart; the
  routine only writes the parameter, which is what `out' says, ref
  5.3.2).  Only `loadav' of the 0xCC4-byte PROC1_ data block is declared:
  it is at offset 0, and nothing else is needed here. }
module proc1_, wired_proc, wired_data;
type
  proc1_$loadav_t = array[1..3] of integer32;
var
  loadav: proc1_$loadav_t;      { +0 of the PROC1_ data block }

procedure proc1_$get_loadav(out avg: proc1_$loadav_t);
begin
  avg := loadav;
end;
