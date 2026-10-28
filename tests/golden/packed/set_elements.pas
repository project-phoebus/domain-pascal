{ codegen-patterns 5.1 ("One-byte sets"), 5.2; ref 3.11.4.2 Table 3-5
  footnote 1 (a set element of a packed array takes its bits padded to a
  power of 2: 5 to 8 members one byte); bead domain-pascal-nb0x.  The
  SAU9 SCSI driver's SCSI_$PPN_OWNERS, `packed array[16#10000..16#1FFFF]
  of set of 0..7', one byte per physical page: an element read into a
  word, `clr.w Dn; move.b M,Dn', tested with `btst Dd,Dn' on a run-time
  member, a member added with `clr.w; bset Dd,Dm; or.w Dn,Dm' and taken
  away with `bset; not.w; and.w', the byte stored back, and compared
  with `[]' by `tst.w' (scsi_wire_page 0x3C4A67B8, scsi_unwire_page
  0x3C4A6836, scsi_unwire_all 0x3C4A68A2, scsi_va_to_ppn 0x3C4A4E3C;
  ../source/docs/pascal-translation-notes.md gap 300).  The run-time
  member tests and the stores of a word set local into the element
  (`move.b Dn,M' of the low byte, 0x3C4A6824) are best effort in dpc (no
  MC68010 site; beads domain-pascal-bkjm, -nb0x), so this golden keeps
  to the verified forms: the element read into a word, compared with
  `[]', and cleared (`clr.b'); the SCSI module exercises the rest.  The image keeps
  the `owners' local in a frame byte and reloads it (0x3C4A67E2 `move.b
  ...,(-14,A6); clr.w D4; move.b (-14,A6),D4'), which dpc does not
  (the local held in a register); an index that starts at 16#10000 is
  reached, on the MC68020, from the symbol biased by 60000
  (`m68020/' goldens would show `movea.l #SCSI_$PPN_OWNERS-60000,A0;
  move.b (-0x15a0,A0,D2.l),Dn', 0x3C4A67DC); here the MC68010 form,
  `lea (0,An,Dx.l),Am; adda.l #-0x10000,Am'. }
module set_elements;
type
  scsi_owners_t = set of 0..7;
  linteger = integer32;
var
  scsi_$ppn_owners: extern packed array[16#10000..16#1FFFF] of scsi_owners_t;
  scsi_$busy: packed array[0..15] of scsi_owners_t;
  busy_count: integer;
function owned(ppn: linteger): boolean; options(val_param);
var
  owners: scsi_owners_t;
begin
  owners := scsi_$ppn_owners[ppn];
  owned := owners <> [];
end;
procedure clear_busy(unit: integer); options(val_param);
begin
  scsi_$busy[unit] := [];
end;
procedure count_busy;
var
  unit: integer;
begin
  for unit := 0 to 15 do
    if scsi_$busy[unit] <> [] then
      busy_count := busy_count + 1;
end;
