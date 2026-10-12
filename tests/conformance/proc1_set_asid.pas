{ PROC1_$SET_ASID, 0xE148F8, 0x26 bytes: 0xE148F8..0xE1491E
  (tools/imgdis.sh E148F8 26).  Semantics:
  ../source/domain_os/proc1/set_asid.c.  Layout: proc1_t in
  ../source/domain_os/proc1/proc1.h (asid at 0x46).  Externals (SAU2
  map): PROC1_$CURRENT_PCB 0xE1EAC8, MMU_$INSTALL_ASID 0xE24204.

  The unit is a fragment of the PROC1 module, so tools/conform.py compiles
  it with `dpc --fragment-origin 0xE148F8'.

  Patterns exercised (docs/codegen-patterns.md):
  - 12.4 R4(c) (bead domain-pascal-g5q.5 review): `asid', loaded at
         entry (12.2, two uses), is last read by the call's argument push
         (0xE1490E `move.w D2,-(SP)') and is held through the call, so it
         takes D2, not D0 (0xE148FE), saved with `move.l D2,-(SP)' and
         restored from (-4,A6);
  - 1.2/1.3 val_param word argument: pad `subq.l #2,SP', then the word;
  - 1.1  no pop before `unlk'.

  The PCB layout is reduced to the one field used; the filler array is
  ours.  MMU_$INSTALL_ASID's parameter declaration (val_param word) is
  reconstructed from the push. }
module proc1_set_asid;
type
  proc1_$pcb_t = record
    pad: array[1..35] of integer;   { 0x00..0x45 }
    asid: integer;                  { 0x46 }
  end;
var
  proc1_$current_pcb: extern ^proc1_$pcb_t;

procedure mmu_$install_asid(asid: integer); options(val_param, extern);

procedure proc1_$set_asid(asid: integer); options(val_param);
begin
  proc1_$current_pcb^.asid := asid;
  mmu_$install_asid(asid);
end;
