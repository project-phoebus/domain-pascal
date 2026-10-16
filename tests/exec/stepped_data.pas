{ Execution test: `for' loops stepping a pointer through arrays of the
  unit's data, of the frame and of another unit (ref "For", pp.
  4-88..4-90; "With", pp. 4-211..4-214; codegen-patterns 7.4, "Forming",
  bead domain-pascal-xm5w): `lea (k,A5|A6)' near the base, `movea.l
  A5|A6|#sym' adjusted in place otherwise, `with a[i]' through a copy of
  the pointer.  Expected values are the Pascal semantics of each loop.

  link: stepped_data_ext.mod.pas
  call: SDX_$SETUP
  call: SDX_$SUM -> l
  call: SDX_$FIND @w:5 -> w
  call: SDX_$FIND @w:9 -> w
  call: SDX_$CLEAR_FLAGS @w:4
  call: SDX_$SUM -> l
  call: SDX_$FAR -> l
  call: SDX_$FRAME -> l
  call: SDX_$EXT_SUM -> l
  expect:
    => 1252
    => 5
    => 0
    104
    => 1225
    => 764
    => 1896
    => 936
}
module sdx;
%include 'exec.ins.pas';

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
  sdx_$pair = record
    key: integer32;
    val: integer;
  end;
var
  hdr: array[1..4] of integer32;
  tab: array[1..8] of entry;
  n: integer;
  pool: array[0..15] of integer32;         { moves slots out of reach }
  slots: array[1..40] of slot;
  sdx_$ext: extern array[1..12] of sdx_$pair;

{ tab[i]: next 100 + i, flags 7, count i, blk 10 * i; slots[i].id 2;
  sdx_$ext[i]: key 13 * i, val i. }
procedure sdx_$setup;
var
  i: integer;
  k: integer32;
begin
  n := 8;
  hdr[1] := 2;
  k := 0;
  for i := 1 to 8 do
    with tab[i] do begin
      k := k + 10;
      { the higher field first: a small longword constant stored below the
        next statement's nonzero one is refused (codegen-patterns 13; bead
        domain-pascal-53xi) }
      flags := 7;
      next := 100;
      count := i;
      blk := k;
    end;
  for i := 1 to 8 do
    tab[i].next := tab[i].next + tab[i].count;
  for i := 1 to 40 do
    slots[i].id := hdr[1];
  k := 0;
  for i := 1 to 12 do begin
    k := k + 13;
    sdx_$ext[i].key := k;
    sdx_$ext[i].val := i;
  end;
end;

{ The sum of next + flags + blk over tab[1..n]: 836 + 56 + 360. }
function sdx_$sum: integer32;
var
  i: integer;
  s: integer32;
begin
  s := 0;
  for i := 1 to n do
    with tab[i] do begin
      s := s + next;
      s := s + flags;
      s := s + blk;
      exec_$clobber;
    end;
  sdx_$sum := s;
end;

{ The index of the entry whose count is c, 0 if none. }
function sdx_$find(in c: integer): integer;
var
  i: integer;
begin
  sdx_$find := 0;
  for i := 1 to 8 do
    with tab[i] do
      if count = c then begin
        sdx_$find := i;
        return;
      end;
end;

{ Clears the flags of the entries before the one whose count is c, and
  logs that one's next and sets its flags to 1; sdx_$sum then drops
  (c - 1) * 7 + 6. }
procedure sdx_$clear_flags(in c: integer);
var
  i: integer;
begin
  for i := 1 to n do
    with tab[i] do begin
      if count = c then begin
        exec_$logv_l(next);
        flags := 1;
        exit;
      end;
      flags := 0;
      exec_$clobber;
    end;
end;

{ slots, out of a byte's reach: in place.  Two ids of 2 and 38 of 20. }
function sdx_$far: integer32;
var
  i: integer;
  s: integer32;
begin
  for i := 3 to 40 do
    slots[i].id := slots[i].id * 10;
  s := 0;
  for i := 1 to 40 do
    s := s + slots[i].id;
  sdx_$far := s;
end;

{ Frame arrays, one near A6 (`lea'), one far (in place): 36 * 30 +
  8 * 102. }
function sdx_$frame: integer32;
var
  near: array[1..8] of integer32;
  far: array[1..36] of integer32;
  i: integer;
  s: integer32;
begin
  for i := 1 to 36 do
    far[i] := tab[3].blk;
  for i := 1 to 8 do
    near[i] := tab[2].next;
  s := 0;
  for i := 1 to 36 do
    s := s + far[i];
  for i := 1 to 8 do
    s := s + near[i];
  sdx_$frame := s;
end;

{ Another unit's array: the keys 13 * i summed, i = 1..12, less the
  vals: 13 * 78 - 78. }
function sdx_$ext_sum: integer32;
var
  i: integer;
  s: integer32;
begin
  s := 0;
  for i := 1 to 12 do
    s := s + sdx_$ext[i].key;
  for i := 1 to 12 do
    s := s - sdx_$ext[i].val;
  sdx_$ext_sum := s;
end;

