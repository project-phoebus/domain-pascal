{ Golden: codegen-patterns 7, loop alignment switched off part-way
  through a unit, with `dpc --loop-align-off-from=alo_$dismount'
  (align_off_from.args).  The routines before ALO_$DISMOUNT align their
  loop heads with the 0x2048 filler (a fill loop, a `repeat'); from it
  on no routine does, even where the head is at 2 mod 4 (ALO_$IDLE's
  `repeat' head, offset 0x52).  In the image: VTOC_$MOUNT has a filler
  at 0xE3871E, the next routine VTOC_$DISMOUNT 0xE38764 a copy loop at
  0xE387CE, 2 mod 4, without one.  The last routine does not align, so
  the module end is padded with zero words (`rts; 0x0000': AST_
  0xE0736E, RING, ACL_, VTOC_ .TEXT). }
module alo;
var
  page: extern array[0..255] of integer32;
  w: extern integer;

procedure alo_$mount;
var
  i: integer;
begin
  for i := 0 to 255 do page[i] := 0;
end;

procedure alo_$wait; internal;
begin
  repeat
    w := 1;
  until w = 3;
end;

procedure alo_$dismount;
var
  i: integer;
begin
  for i := 0 to 255 do page[i] := 0;
end;

procedure alo_$idle; internal;
begin
  repeat
    w := 1;
  until w = 3;
end;
