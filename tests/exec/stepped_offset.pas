{ Execution test, bead domain-pascal-m2m2: `for' loops stepping a pointer
  through an array at an offset from its pointer variable (`p^.f[i]', a
  record parameter's field: `lea (from * size,An),Ap', the offset in the
  displacements) and through an array of the module's block in another
  data section (`movea.l #block,Ap' adjusted in place, the array's offset
  in the displacements); codegen-patterns 7.4; ref "For", pp. 4-88..4-90;
  "With", pp. 4-211..4-214.  Covered: from 0, 1, 2 and 4, 2-, 4-, 6- and
  8-byte elements, a call in the body (the pointer in a callee-saved
  register), `with' over the element, an element passed by address (`pea
  (d,Ap)', from 0), `downto', a record parameter by address and a pointer
  parameter.  A routine of the block's own section reads the values back
  through A5.  Expected values are the Pascal semantics of each routine.

  call: SOF_$INIT
  call: SOF_$FILLW
  call: SOF_$FILL8
  call: SOF_$SHOW -> l
  call: SOF_$SETUP
  call: SOF_$SUMREC -> l
  call: SOF_$WITHREC -> l
  call: SOF_$LOGREC
  call: SOF_$DOWNREC -> l
  call: SOF_$PTRSUM -> l
  call: SOF_$BLOCKLOG
  expect:
    => 29215
    => 1965
    => 2580
    1
    2
    3
    4
    5
    40
    80
    120
    160
    => 4321
    => 45
    2
    4
}
module sof, wired_proc, wired_data;
%include 'exec.ins.pas';

type
  e6 = record a: integer; b: integer; c: integer; end;
  e8 = record id: integer; pad: integer; v: integer32; end;
  rec = record
    hdr: integer32;
    n: integer;
    words: array[0..4] of integer;       { +6 }
    six: array[1..5] of e6;              { +0x10 }
    longs: array[1..4] of integer32;     { +0x2E }
  end;
  recp = ^rec;
var
  first: integer32;                      { WIRED_DATA +0 }
  pad: array[1..3] of integer;
  bw: array[0..9] of integer;            { +0xA }
  b8: array[1..6] of e8;                 { +0x1E }
  tail: integer;
  r: rec;

{ the block's scalars, from another section (no loop: a routine that
  reaches the block and branches is refused, codegen-patterns 11) }
[section(init_proc, init_data)]
procedure sof_$init;
begin
  first := 5;
  tail := 77;
end;

{ from 0, the pointer the block's address: bw[i] = 11 * i }
[section(init_proc, init_data)]
procedure sof_$fillw;
var
  i: integer;
begin
  for i := 0 to 9 do
    bw[i] := i * 11;
end;

{ from 1, `with' over the element: (i, -1, 1000 * i) }
[section(init_proc, init_data)]
procedure sof_$fill8;
var
  i: integer;
begin
  for i := 1 to 6 do
    with b8[i] do begin
      id := i;
      pad := -1;
      v := i * 1000;
    end;
end;

{ read back through A5: first 5; the ids 21, the values 21000, the pads
  -6; bw[1..9] 11 * 45 = 495; bw[0] * 1000 = 0; tail * 100 = 7700:
  5 + 21015 + 495 + 0 + 7700 = 29215 }
function sof_$show: integer32;
var
  i: integer;
  s: integer32;
begin
  s := first;
  for i := 1 to 6 do
    s := s + b8[i].id + b8[i].v + b8[i].pad;
  for i := 1 to 9 do
    s := s + bw[i];
  s := s + bw[0] * 1000 + tail * 100;
  sof_$show := s;
end;

{ r: words 1..5, six[i] = (i, 10 * i, 100 * i), longs[i] = 40 * i }
procedure sof_$setup;
var
  i: integer;
begin
  r.words[0] := 1;                       { no two stores adjacent }
  r.hdr := 9;
  r.words[2] := 3;
  r.n := 5;
  r.words[4] := 5;
  r.words[1] := 2;
  r.words[3] := 4;
  for i := 1 to 5 do begin
    r.six[i].a := i;
    r.six[i].b := i * 10;
    r.six[i].c := i * 100;
  end;
  for i := 1 to 4 do
    r.longs[i] := i * 40;
end;

{ through a record parameter by address: words 15; six.b 150; six.c from
  2, 1400; longs 400: 1965 }
function sof_$sumrec_in(var x: rec): integer32;
var
  i: integer;
  s: integer32;
begin
  s := 0;
  for i := 0 to 4 do
    s := s + x.words[i];
  for i := 1 to 5 do
    s := s + x.six[i].b;
  for i := 2 to 5 do
    s := s + x.six[i].c;
  for i := 1 to 4 do
    s := s + x.longs[i];
  sof_$sumrec_in := s;
end;

function sof_$sumrec: integer32;
begin
  sof_$sumrec := sof_$sumrec_in(r);
end;

{ `with' over the element: a * c, 100 * (1 + 4 + 9 + 16 + 25) = 5500,
  less b * 20, 200 * 15 = 3000, plus 16 a pass, 80: 2580 }
function sof_$withrec_in(var x: rec): integer32;
var
  i: integer;
  s: integer32;
begin
  s := 0;
  for i := 1 to 5 do
    with x.six[i] do begin
      s := s + a * c - b * 20;
      s := s + 16;
    end;
  sof_$withrec_in := s;
end;

function sof_$withrec: integer32;
begin
  sof_$withrec := sof_$withrec_in(r);
end;

{ a call in the body: the element by address from 0, 1 .. 5 (from 1 it
  is refused, AUDIT's module-local 0xE7131C), then by value, 40 .. 160 }
procedure sof_$logrec_in(var x: rec);
var
  i: integer;
begin
  for i := 0 to 4 do
    exec_$log_w(x.words[i]);
  for i := 1 to 4 do
    exec_$logv_l(x.longs[i]);
end;

procedure sof_$logrec;
begin
  sof_$logrec_in(r);
end;

{ downto from 4: words[4..1] = 5, 4, 3, 2 as digits, 5432, less 1111 }
function sof_$downrec_in(var x: rec): integer32;
var
  i: integer;
  s: integer32;
begin
  s := 0;
  for i := 4 downto 1 do
    s := s * 10 + x.words[i];
  sof_$downrec_in := s - 1111;
end;

function sof_$downrec: integer32;
begin
  sof_$downrec := sof_$downrec_in(r);
end;

{ through a pointer parameter: six.a, 15, times 3 }
function sof_$ptrsum_in(p: recp): integer32;
  options(val_param);
var
  i: integer;
  s: integer32;
begin
  s := 0;
  for i := 1 to 5 do
    s := s + p^.six[i].a * 3;
  sof_$ptrsum_in := s;
end;

function sof_$ptrsum: integer32;
begin
  sof_$ptrsum := sof_$ptrsum_in(addr(r));
end;

{ the block's array from another section with a call in the body: the
  ids 2 and 4 logged, in that order }
[section(init_proc, init_data)]
procedure sof_$blocklog;
var
  i: integer;
begin
  for i := 1 to 6 do
    if b8[i].id = 2 then
      exec_$logv_w(b8[i].id)
    else if b8[i].id = 4 then
      exec_$logv_w(b8[i].id)
    else
      exec_$clobber;
end;
