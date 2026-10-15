{ XPD_$GET_FP_INT and XPD_$PUT_FP_INT, 0xE5C55A..0xE5C5C2, 0x34 bytes
  each (tools/imgdis.sh E5C55A 68).  Semantics:
  ../source/domain_os/xpd/registers.c.  Externals (SAU2 map): PEB_$INFO
  0xE24C78 (the flag the routines test is at +0x20, 0xE24C98, the C's
  M68881_$SAVE_FLAG), FP_$GET_FP 0xE21D48, FP_$PUT_FP 0xE21D94,
  PEB_$GET_FP 0xE5AEEC, PEB_$PUT_FP 0xE5AF12.

  The unit is a fragment of the XPD_KER segment, so tools/conform.py
  compiles it with `dpc --fragment-origin 0xE5C55A'.

  Patterns exercised (docs/codegen-patterns.md):
  - 12.4 R4(c) (bead domain-pascal-9j1z): `asid', loaded at entry
         (12.2, two uses), is last read by the arguments of a call in
         each part of the `if' (0xE5C574 `move.w (A2),-(SP)', 0xE5C57E
         `pea (A2)') and is held through the call, in a body with
         branches: A2 (0xE5C560), not A0, saved with `pea (A2)' and
         restored from (-4,A6);
  - 12.4 R4(b): the status slot, used once, in A0 (0xE5C564);
  - 13   the status cleared on entry, `clr.l (A0)';
  - 1.2/1.3 a val_param word argument: pad `subq.l #2,SP', then the
         word; a by-address argument with `pea';
  - 1.1  no pop before the `bra' to the epilogue or before `unlk'.

  PEB_$INFO's layout is reduced to the one field used; the filler array
  and the field's name are ours.  status_$t is reduced to its `all'
  word.  The callees' parameter declarations
  (a val_param word; one by address) are reconstructed from the pushes. }
module xpd_fp_int;
const
  status_$ok = 0;
type
  status_$t = record
    all: integer32;
  end;
  peb_$info_t = record
    pad: array[1..16] of integer;   { 0x00..0x1F }
    m68881: boolean;                { 0x20 }
  end;
var
  peb_$info: extern peb_$info_t;

procedure fp_$get_fp(asid: integer); options(val_param, extern);
procedure fp_$put_fp(asid: integer); options(val_param, extern);
procedure peb_$get_fp(in asid: integer); extern;
procedure peb_$put_fp(in asid: integer); extern;

procedure xpd_$get_fp_int(in asid: integer; out status: status_$t);
begin
  status.all := status_$ok;
  if peb_$info.m68881 then
    fp_$get_fp(asid)
  else
    peb_$get_fp(asid);
end;

procedure xpd_$put_fp_int(in asid: integer; out status: status_$t);
begin
  status.all := status_$ok;
  if peb_$info.m68881 then
    fp_$put_fp(asid)
  else
    peb_$put_fp(asid);
end;
