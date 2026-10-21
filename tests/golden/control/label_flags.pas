{ codegen-patterns 7.1 ("Flags across an `if''s labels"); bead
  domain-pascal-nzs4.

  - `n := n - 1; if n < 0': `bpl' on the flags of the `subq' that
    updates n in memory, no `tst' (SAU5 0xE75B9E `subq.w #1,(0x30,A3);
    bpl.s'; SAU5 0xE59682 `subq.w #1,(2,A3); bpl.s').
  - `else if n = 0' after it: the `else' label is entered only by that
    branch, with the `subq''s flags, and the test branches on them
    (mt_$release_unit, SAU5 0xE75B9E..0xE75BAA `subq.w #1,(0x30,A3);
    bpl.s; clr.w (0x30,A3); bra.s; bne.s').  `rel' takes n through a
    `with' pointer as the image does; `rel_abs' another unit's word.
  - `if n < 0 then n := 0; if n = 0': the join is entered with the
    `subq''s flags or the `clr.w''s, both of n (PBU_$SETDOWN, SAU5
    0xE59682..0xE5968C `subq.w #1,(2,A3); bpl.s; clr.w (2,A3); bne.s').
  - `else if' after a `tst' of the same object: on the `tst''s flags,
    any relation (SAU2 0xE3FE3E `tst.w (A3); ble.s 0xE3FE4A; ...;
    bra.s; 0xE3FE4A: bne.s').

  Not on them (a `tst'): `n = 0' right after the `subq' (`=' and `<>'
  after an add or sub in memory are tested again or not by a rule not
  known: SAU2 0xE17930 against 0xE3A99C); `n > 0' after it (reads V);
  an `else if' after a compare with another constant (the flags are not
  n's against 0: 0xE4D9F0 `cmpi.w #32,(-254,A6); ...; tst.w
  (-254,A6)'); a `repeat''s `until n = 0' after `if n < 0 then n := 0'
  (only an `if' right after the join is seen on its flags). }
module lflags;
type
  unit_t = record
    pad: array [1..24] of integer;
    n: integer;
  end;
var
  n: extern integer;
  u: extern ^unit_t;
procedure z; extern;
procedure y; extern;

procedure rel;
begin
  with u^ do begin
    n := n - 1;
    if n < 0 then
      n := 0
    else if n = 0 then
      z;
  end;
end;

procedure rel_abs;
begin
  n := n - 1;
  if n < 0 then
    n := 0
  else if n = 0 then
    z;
end;

procedure setdown;
begin
  n := n - 1;
  if n < 0 then
    n := 0;
  if n = 0 then
    z;
end;

procedure tst_else;
begin
  if n > 0 then
    z
  else if n = 0 then
    y;
end;

procedure tst_else_ordered;
begin
  if n = 0 then
    z
  else if n > 0 then
    y;
end;

procedure eq_after_sub;
begin
  n := n - 1;
  if n = 0 then
    z;
end;

procedure gt_after_sub;
begin
  n := n - 1;
  if n > 0 then
    z
  else if n = 0 then
    y;
end;

procedure cmpi_else;
begin
  if n = 32 then
    z
  else if n = 0 then
    y;
end;

procedure until_after_join;
begin
  repeat
    n := n - 1;
    if n < 0 then
      n := 0;
  until n = 0;
end;
