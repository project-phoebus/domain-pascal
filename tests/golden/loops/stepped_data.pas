{ Golden: codegen-patterns 7.4 ("Forming"; bead domain-pascal-xm5w),
  `for' loops stepping a pointer through an array of the unit's data, of
  the frame, or of another unit's object.  The unit's data and the frame:
  `lea (from * size,A5|A6),Ap' when every displacement through the pointer,
  and that displacement plus from * size, fit in a byte (DBUF_$INVALIDATE
  0xE3AA0E `lea (36,A5),A2', then a `with' copy `movea.l A2,A3' and
  `(-8,A3)'; FIND_VOLX 0xE6B0DA `moveq #5,D3; moveq #1,D4; lea (32,A5),A0';
  MSG_$$SEND 0xE0DB7A `lea (4,A6),A0'), else `movea.l A5|A6,Ap' adjusted in
  place (EC2_$REGISTER_EC1 0xE42968, `(2700,A0)'; OS_$INSTALL_DISPLAY_ASTE
  0xE6D2BE `movea.l A6,A0; moveq #35,D0; addq.l #4,A0').  Another unit's
  object: always `movea.l #sym,Ap' adjusted in place (TIME_$INIT 0xE2FE94
  `moveq #63,D3; movea.l #abs,A2; moveq #1,D4; lea (12,A2),A2').  The
  in-place `movea.l' goes before the last count or index instruction. }
module sd;
type
  entry = record
    next: integer32;
    flags: integer;
    count: integer;
    blk: integer32;
  end;
  slot = record
    id: integer32;
    pad: array[1..4] of integer32;
  end;
var
  hdr: array[1..4] of integer32;
  tab: array[1..8] of entry;
  n: integer;
  pool: array[0..15] of integer32;         { moves slots out of reach }
  slots: array[1..40] of slot;
  ext_tab: extern array[1..64] of entry;

procedure ext_$put(x: integer32); options(val_param, extern);
procedure ext_$init(var e: entry); extern;

{ DBUF_$INVALIDATE 0xE3AA02..0xE3AA12: a run-time bound and a call, the
  count copied; `with' copies the pointer at the head. }
procedure sd_$invalidate;
var
  i: integer;
begin
  for i := 1 to n do
    with tab[i] do
      if blk = 0 then begin
        ext_$put(next);
        flags := 0;
      end;
end;

{ FIND_VOLX 0xE6B0DA..0xE6B0E4: the index read, the pointer after it. }
function sd_$find: integer;
var
  i: integer;
begin
  sd_$find := 0;
  for i := 1 to 8 do
    with tab[i] do
      if count = 1 then begin
        sd_$find := i;
        return;
      end;
end;

{ Out of a byte's reach (the array at 0x10C and more): in place. }
procedure sd_$clear;
var
  i: integer;
begin
  for i := 1 to 40 do
    slots[i].id := hdr[1];
end;

{ The frame: `lea (4,A6)' for the near array, in place for the far one
  (OS_$INSTALL_DISPLAY_ASTE 0xE6D2BE and 0xE6D2CE). }
procedure sd_$frame;
var
  near: array[1..8] of integer32;
  far: array[1..36] of integer32;
  i: integer;
begin
  for i := 1 to 36 do
    far[i] := hdr[2];
  for i := 1 to 8 do
    near[i] := hdr[3];
  ext_$put(far[1] + near[1]);
end;

{ Another unit's array (TIME_$INIT 0xE2FE94): `movea.l #sym', the index
  between it and its adjustment; an element passed by address. }
procedure sd_$init_ext;
var
  i: integer;
begin
  for i := 1 to 64 do begin
    ext_$init(ext_tab[i]);
    ext_tab[i].count := i;
  end;
end;
