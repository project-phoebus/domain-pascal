{ Execution test, bead domain-pascal-1owb: `for' loops storing a constant
  other than 0 into consecutive elements, `for k := lo to hi do a[k] :=
  c' with the index used for nothing else.  The lowering leaves them
  `For's (a constant of 0 stays a `Fill'), whose stepped pointer is the
  image's form: MMAP_$INIT 0xE3199E `moveq #55,D0; lea (4,A2),A0; move.l
  #0xfff,(-4,A0); addq.l #4,A0; dbf', RIP_$SEND 0xE8720A `move.w
  #-1,(4,A0)' (codegen-patterns 7.4).  ref "For", pp. 4-88..4-90.

  Covered: chars, `[byte]' signed bytes (-2), booleans (`true', 0xFF, ref
  3.5.3 p. 3-11), words (negative, a count of 1), longwords (in `moveq''s
  range, negative, beyond it, a count of 1), an enumeration (its ordinal
  in a word, ref 3.7.1); partial ranges and whole arrays; large counts
  (300 words, 198 longwords, 1000 chars: `move.w #n-1'); the unit's data,
  frame arrays (in a record and alone), through a by-value pointer, a
  `var' and an `out' parameter, a record's array through a `var'
  parameter (RIP_$SEND's shape), another unit's arrays (from an element
  other than the first, and a whole 0-based array from its first byte).
  Every object is first set to 0xA5 bytes (cfl_$smear) and logged whole,
  with guard fields or variables around the array, so a store outside
  the range shows.  Pointer constants other than `nil' do not exist (ref
  2.2.2.2, p. 2-12), and `nil' is 0, a `Fill'.

  link: const_fill_data.mod.pas
  call: CFL_$CHARS
  call: CFL_$SBYTES
  call: CFL_$BOOLS
  call: CFL_$WORDS
  call: CFL_$WORD1
  call: CFL_$LONGS
  call: CFL_$LONGNEG
  call: CFL_$LONG1
  call: CFL_$ENUMS
  call: CFL_$BIGW
  call: CFL_$BIGL
  call: CFL_$BIGC
  call: CFL_$FRAME
  call: CFL_$FRAMEL
  call: CFL_$PTR_RUN
  call: CFL_$VAR_RUN
  call: CFL_$OUT_RUN
  call: CFL_$HOST_RUN
  call: CFL_$EXT
  call: CFL_$EXTW
  warns: best effort (not byte-identical): a `for` loop stepping a pointer from the start of the unit's data area
  warns: best effort (not byte-identical): an element of a stepped array at displacement 0
  warns: best effort (not byte-identical): a part of an element of a stepped array at displacement 0
  expect:
    mem a5 a5 7a 7a 7a 7a 7a 7a 7a a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5
    mem a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 fe fe fe fe a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5
    mem a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 ff ff ff ff ff ff a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5
    mem a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 ff fd ff fd ff fd ff fd ff fd ff fd a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5
    mem a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 12 34 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5
    mem a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 00 00 00 05 00 00 00 05 00 00 00 05 00 00 00 05 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5
    mem a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 ff fe 79 60 ff fe 79 60 ff fe 79 60 ff fe 79 60 ff fe 79 60 ff fe 79 60 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5
    mem a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 12 34 56 78 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5
    mem a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 00 02 00 02 00 02 a5 a5 a5 a5
    mem a5 a5 7f ff 7f ff
    mem 7f ff 7f ff a5 a5
    9830100
    mem a5 a5 a5 a5 a5 a5 a5 a5 ff ff ff ff
    mem ff ff ff ff a5 a5 a5 a5 a5 a5 a5 a5
    -1
    mem a5 a5 71
    mem 71 71 a5
    113
    mem a5 a5 a5 a5 ff f9 ff f9 ff f9 a5 a5 00 01 00 00 00 01 00 00 00 01 00 00 78 78 78 78 78 a5 a5 ff ff a5 a5 a5
    mem a5 a5 a5 a5 ff ff ff f7 ff ff ff f7 ff ff ff f7 ff ff ff f7 a5 a5 a5 a5
    mem a5 a5 a5 a5 ff ff ff f7 ff ff ff f7 ff ff ff f7 ff ff ff f7 00 00 00 7f
    mem a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 ff ff ff 80 ff ff ff 80 ff ff ff 80 ff ff ff 80 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5
    mem a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 00 64 00 64 00 64 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5
    mem a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 00 00 0f ff 00 00 0f ff 00 00 0f ff 00 00 0f ff 00 00 0f ff 00 00 0f ff a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5 a5
    mem a5 a5 ff ff ff ff ff ff a5 a5
    mem a5 a5 a5 a5 a5 a5 00 00 00 4d 00 00 00 4d 00 00 00 4d 00 00 00 4d a5 a5 a5 a5 a5 a5
    mem 80 00 80 00 80 00 80 00 80 00
    11
    22
}
module cfl;
%include 'exec.ins.pas';

type
  sbyte = [byte] -128..127;
  color = (red, green, blue);
  warr = array[1..8] of integer;
  larr = array[0..5] of integer32;
  lptr = ^larr;
  host = record
    n: integer;
    h: array[0..2] of integer;
    m: integer;
  end;
  cfl_$xrec = record
    g0: integer;
    l: array[1..6] of integer32;
    g1: integer;
  end;
  smearbytes = array[1..2000] of char;
  smearptr = ^smearbytes;

var
  cfl_$x: extern cfl_$xrec;
  cfl_$xg0: extern integer;
  cfl_$xw: extern array[0..4] of integer;
  cfl_$xg1: extern integer;

  { 76 bytes, no padding: b 0..11, sb 12..17, bo 18..23, w 24..39,
    l 40..63, e 64..73, g 74..75 }
  md: record
    b: array[1..12] of char;
    sb: array[1..6] of sbyte;
    bo: array[1..6] of boolean;
    w: warr;
    l: larr;
    e: array[1..5] of color;
    g: integer;
  end;
  bw: record
    g0: integer;
    a: array[1..300] of integer;
    g1: integer;
  end;
  bl: record
    g0: integer32;
    a: array[1..200] of integer32;
    g1: integer32;
  end;
  bc: record
    g0: char;
    a: array[1..1000] of char;
    g1: char;
  end;
  hv: host;

{ n bytes from p set to 0xA5 }
procedure cfl_$smear(p: smearptr; n: integer);
  options(val_param);
var
  k: integer;
begin
  k := 1;
  while k <= n do begin
    p^[k] := chr(16#A5);
    k := k + 1;
  end;
end;

procedure cfl_$md;
begin
  cfl_$smear(addr(md), 76);
end;

procedure cfl_$chars;
var
  k: integer;
begin
  cfl_$md;
  for k := 3 to 9 do
    md.b[k] := 'z';
  exec_$log_mem(md, 76);
end;

procedure cfl_$sbytes;
var
  k: integer;
begin
  cfl_$md;
  for k := 2 to 5 do
    md.sb[k] := -2;
  exec_$log_mem(md, 76);
end;

procedure cfl_$bools;
var
  k: integer;
begin
  cfl_$md;
  for k := 1 to 6 do
    md.bo[k] := true;
  exec_$log_mem(md, 76);
end;

procedure cfl_$words;
var
  k: integer;
begin
  cfl_$md;
  for k := 2 to 7 do
    md.w[k] := -3;
  exec_$log_mem(md, 76);
end;

procedure cfl_$word1;
var
  k: integer;
begin
  cfl_$md;
  for k := 8 to 8 do
    md.w[k] := 16#1234;
  exec_$log_mem(md, 76);
end;

procedure cfl_$longs;
var
  k: integer;
begin
  cfl_$md;
  for k := 1 to 4 do
    md.l[k] := 5;
  exec_$log_mem(md, 76);
end;

procedure cfl_$longneg;
var
  k: integer;
begin
  cfl_$md;
  for k := 0 to 5 do
    md.l[k] := -100000;
  exec_$log_mem(md, 76);
end;

procedure cfl_$long1;
var
  k: integer;
begin
  cfl_$md;
  for k := 2 to 2 do
    md.l[k] := 16#12345678;
  exec_$log_mem(md, 76);
end;

procedure cfl_$enums;
var
  k: integer;
begin
  cfl_$md;
  for k := 2 to 4 do
    md.e[k] := blue;
  exec_$log_mem(md, 76);
end;

{ 300 words: the first and last 6 bytes, and a sum of all }
procedure cfl_$bigw;
var
  k, j: integer;
  s: integer32;
begin
  cfl_$smear(addr(bw), 604);
  for k := 1 to 300 do
    bw.a[k] := 16#7FFF;
  exec_$log_mem(bw, 6);
  exec_$log_mem(bw.a[299], 6);
  s := 0;
  j := 1;
  while j <= 300 do begin
    s := s + bw.a[j];
    j := j + 1;
  end;
  exec_$log_l(s);
end;

{ 198 longwords from the second: the first and last 12 bytes }
procedure cfl_$bigl;
var
  k: integer;
begin
  cfl_$smear(addr(bl), 808);
  for k := 2 to 199 do
    bl.a[k] := -1;
  exec_$log_mem(bl, 12);
  exec_$log_mem(bl.a[199], 12);
  exec_$log_l(bl.a[100]);
end;

{ 1000 chars: the first and last 3 bytes (g0 0, a 2..1001, g1 1002, the
  array word-aligned) }
procedure cfl_$bigc;
var
  k: integer;
begin
  cfl_$smear(addr(bc), 1004);
  for k := 1 to 1000 do
    bc.a[k] := 'q';
  exec_$log_mem(bc, 3);
  exec_$log_mem(bc.a[999], 3);
  exec_$log_c(bc.a[500]);
end;

{ the frame: arrays of a local record, 36 bytes (g0 0..1, w 2..11, l
  12..23, b 24..28, bo 30..32, g1 34..35, the arrays word-aligned) }
procedure cfl_$frame;
var
  k: integer;
  loc: record
    g0: integer;
    w: array[1..5] of integer;
    l: array[1..3] of integer32;
    b: array[1..5] of char;
    bo: array[1..3] of boolean;
    g1: integer;
  end;
begin
  cfl_$smear(addr(loc), 36);
  for k := 2 to 4 do
    loc.w[k] := -7;
  for k := 1 to 3 do
    loc.l[k] := 16#10000;
  for k := 1 to 5 do
    loc.b[k] := 'x';
  for k := 2 to 3 do
    loc.bo[k] := true;
  exec_$log_mem(loc, 36);
end;

{ a frame array alone, elements 2 .. 5, then 6 alone }
procedure cfl_$framel;
var
  k: integer;
  fa: array[1..6] of integer32;
begin
  cfl_$smear(addr(fa), 24);
  for k := 2 to 5 do
    fa[k] := -9;
  exec_$log_mem(fa, 24);
  for k := 6 to 6 do
    fa[k] := 127;
  exec_$log_mem(fa, 24);
end;

{ through a by-value pointer, elements 1 .. 4 }
procedure cfl_$ptr(p: lptr);
  options(val_param);
var
  k: integer;
begin
  for k := 1 to 4 do
    p^[k] := -128;
end;

procedure cfl_$ptr_run;
begin
  cfl_$md;
  cfl_$ptr(addr(md.l));
  exec_$log_mem(md, 76);
end;

{ through a `var' parameter, elements 1 .. 3 }
procedure cfl_$var(var a: warr);
var
  k: integer;
begin
  for k := 1 to 3 do
    a[k] := 100;
end;

procedure cfl_$var_run;
begin
  cfl_$md;
  cfl_$var(md.w);
  exec_$log_mem(md, 76);
end;

{ an `out' parameter, the whole array (MMAP_$INIT's shape) }
procedure cfl_$out(out t: larr);
var
  i: integer;
begin
  for i := 0 to 5 do
    t[i] := 16#FFF;
end;

procedure cfl_$out_run;
begin
  cfl_$md;
  cfl_$out(md.l);
  exec_$log_mem(md, 76);
end;

{ a record's array through a `var' parameter (RIP_$SEND's shape) }
procedure cfl_$host(var x: host);
var
  k: integer;
begin
  for k := 0 to 2 do
    x.h[k] := -1;
end;

procedure cfl_$host_run;
begin
  cfl_$smear(addr(hv), 10);
  cfl_$host(hv);
  exec_$log_mem(hv, 10);
end;

{ another unit's array, elements 2 .. 5 }
procedure cfl_$ext;
var
  k: integer;
begin
  cfl_$smear(addr(cfl_$x), 28);
  for k := 2 to 5 do
    cfl_$x.l[k] := 77;
  exec_$log_mem(cfl_$x, 28);
end;

{ another unit's whole 0-based array, from its first byte }
procedure cfl_$extw;
var
  k: integer;
begin
  cfl_$xg0 := 11;
  cfl_$xg1 := 22;
  cfl_$smear(addr(cfl_$xw), 10);
  for k := 0 to 4 do
    cfl_$xw[k] := -32768;
  exec_$log_mem(cfl_$xw, 10);
  exec_$log_w(cfl_$xg0);
  exec_$log_w(cfl_$xg1);
end;
