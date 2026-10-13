{ SMD_$SET_TP_CURSOR, 0xE6E964, 0x3C bytes (tools/imgdis.sh E6E964 3C).
  Semantics: ../source/domain_os/smd/set_tp_cursor.c.  SMD_$LOC_EVENT
  0xE6E9A0, of the same module, lies outside the unit
  (`module-routines=SMD_$LOC_EVENT', a forward `bsr.s').

  A fragment of the SMD code segment 0xE6D700, which is in always mode
  from SMD_$MOVE_KBD_CURSOR 0xE6E7FA on (`tools/a5survey.py switch SMD'),
  so the manifest passes `a5-always=SMD_$SET_TP_CURSOR': A5 is loaded
  although the routine does not use the data.  The data area,
  `.bss.moddata.SMD', is placed at the map's `D E82B8C SMD'.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2  value parameters under the default convention (bead
         domain-pascal-5aw.12; ref 5.2 Table 5-1, p. 5-5): `pos' and
         `buttons' passed by address and copied on entry, in parameter
         order, `movea.l (0xc,A6),A0; move.l (A0),(-4,A6); movea.l
         (0x10,A6),A1; move.w (A1),(-6,A6)' (0xE6E972..0xE6E97E), each
         slot in a register no earlier copy left a value in (12.4
         R4(b)), the copies at the top of the frame in parameter order
         (`link #-8'), then pushed by value from the frame;
  - 12.4 R4(b)  the `in' parameter `unit', read once, through a slot
         loaded into A2 after the copies' A0 and A1 (0xE6E98C), so A2 is
         saved (`movem.l A2/A5');
  - 1.4, 1.3  the pad and word pushes, a constant 0 as `clr.w -(SP)'.

  Reconstructed (not known from the original): every name; `in' for
  `unit', which the routine reads through its address without a copy (a
  value parameter would be copied, ref 5.2); the position as one
  longword; `val_param' on SMD_$LOC_EVENT (its arguments are pushed by
  value); `smd_$data_stand_in', a variable only so that the unit has a
  data area for A5 to point at (dpc requires data for always mode). }
module smd;
type
  smd_$pos_t = integer32;
var
  smd_$data_stand_in: integer32;   { stand-in, see above }

procedure smd_$loc_event(event: integer; unit: integer; pos: smd_$pos_t; buttons: integer);
  options(val_param, extern);

procedure smd_$set_tp_cursor(in unit: integer; pos: smd_$pos_t; buttons: integer);
begin
  smd_$loc_event(0, unit, pos, buttons);
end;
