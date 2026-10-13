{ Golden: codegen-patterns 7.4 and 5.2, `for' loops over arrays of
  elements wider than a byte, reached through pointers stepped by the
  element size.  Each pointer starts at `base + from * size' and the
  elements are reached at displacements with the lower bound and field
  offsets folded in: LOG_$READ's worker 0xE177E0..0xE177F2 `lea (2,A1),A0;
  lea (2,A2),A1; move.w (-2,A0),(-2,A1); addq.l #2,A1; addq.l #2,A0; dbf'
  (two 1-based word arrays, the source formed first, the destination
  stepped first); PKT_$DUMP_DATA 0xE12800 `lea (4,A0),A2' from a pointer
  loaded at entry, with an index register besides and a call in the
  body (0xE12822 `addq.w #1,D3; addq.l #4,A2; dbf'); `subq.l' for
  `downto' (0xE3C99A); `lea (s,An),An' above 8 (HINT_$INIT_CACHE
  0xE313F4 `lea (12,A0),A0'). }
module sp;
type
  words = array[1..16] of integer;
  wptr = ^words;
  entry = record
    key: integer32;
    tag: integer;
    flag: boolean;
  end;
  table = array[1..8] of entry;
  tptr = ^table;
  big = record
    a, b, c: integer32;
  end;
  bigs = array[1..4] of big;
  bptr = ^bigs;
var
  dst, src: wptr;
  tab: tptr;
  bp: bptr;

procedure ext_$put(x: integer32); options(val_param, extern);

{ 0xE177E0: two pointers, the source formed first, stepped last. }
procedure sp_$copy;
var
  i: integer;
begin
  for i := 1 to 16 do
    dst^[i] := src^[i];
end;

{ Downward, with the index read too: the index stepped first. }
procedure sp_$number;
var
  i: integer;
begin
  for i := 8 downto 1 do begin
    tab^[i].tag := i;
    tab^[i].flag := false;
  end;
end;

{ 12-byte elements: `lea (12,An),An'. }
procedure sp_$clear;
var
  i: integer;
begin
  for i := 1 to 4 do
    bp^[i].b := 0;
end;

{ PKT_$DUMP_DATA 0xE127FA..0xE12826: a by-address array used before the
  loop is loaded at entry; the pointer is formed from it after the count
  and the index; across the call both registers are callee-saved. }
procedure sp_$dump(var t: table);
var
  i: integer;
begin
  t[1].flag := true;
  for i := 1 to 4 do begin
    t[i].tag := i;
    ext_$put(t[i].key);
  end;
end;
