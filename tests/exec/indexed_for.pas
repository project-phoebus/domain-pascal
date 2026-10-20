{ Execution test, bead domain-pascal-kaxz: `for' loops whose elements dpc
  reaches by index (the index kept in a register, each element `(d,An,Dx)'
  with the scaled index) where the stepped pointers of codegen-patterns
  7.4 are not modelled; best effort, not byte-identical.  Covered:
  elements at `a[i + 1]' (from another array, and from the same array, so
  each pass reads the previous pass's store), `a[i - 1]', `a[2 * i]', a
  row `g[i][2]' next to `g[i + 1][1]', a pointer the body assigns, a
  `.l' index (an
  array through a pointer whose bounds times the element size do not fit
  in 16 bits) with a call and `downto', an index computed in a call's
  argument, the index read after an `exit', two passes from 1 (the
  index's initial value the `dbf' count's), an index from 200 (beyond a
  `moveq').  Then the clearing loops
  (`StmtKind::Fill') from other than another unit's first byte: the unit's
  data from its third element, a frame array, through a by-value pointer
  and a `var' parameter, another unit's array from its third element, a
  `char' array.  Last, the address of a stepped element stored (`lea
  (d,Ap)').  ref "For", pp. 4-88..4-90.  Each routine starts from
  ixf_$init's values (wa[k] = k, wb[k] = 100 + k, the rest 0); expected
  values are the Pascal semantics.

  link: indexed_for_data.mod.pas
  call: IXF_$PLUS1
  call: IXF_$SHIFT
  call: IXF_$MINUS1
  call: IXF_$TIMES2
  call: IXF_$ROWS
  call: IXF_$PTRCHG
  call: IXF_$WIDE
  call: IXF_$CALLIDX
  call: IXF_$EXITAFTER -> w
  call: IXF_$TWO
  call: IXF_$FAR
  call: IXF_$FILLW
  call: IXF_$FILLFRAME
  call: IXF_$FILLPTR_RUN
  call: IXF_$FILLVAR_RUN
  call: IXF_$FILLEXT
  call: IXF_$FILLC
  call: IXF_$ADDRS
  warns: best effort (not byte-identical): an element at an index computed from a `for` index
  warns: best effort (not byte-identical): a `for` loop stepping a pointer through an array whose pointer is not a variable the body leaves unchanged
  warns: best effort (not byte-identical): a `for` loop stepping through an array whose bounds times the element size do not fit in 16 bits
  warns: best effort (not byte-identical): a `for` index whose initial value equals the `dbf` count
  warns: best effort (not byte-identical): a `for` index whose initial value is outside -128..127
  warns: best effort (not byte-identical): a `for` loop over an array other than another unit's, from its first byte
  warns: best effort (not byte-identical): the address of a stepped element other than a `with` record's
  expect:
    mem 00 01 00 66 00 67 00 68 00 69 00 6a 00 6b 00 6c 00 6d 00 6e
    mem 00 32 00 33 00 34 00 35 00 36 00 37 00 38 00 39 00 3a 00 3b
    mem 00 66 00 67 00 68 00 69 00 6a 00 6b 00 6c 00 6d 00 6e 00 0a
    mem 00 00 00 00 00 00 03 e8 00 00 00 00 00 00 07 d0 00 00 00 00 00 00 0b b8
    mem 00 00 00 04 00 00 00 00 00 08 00 05 00 00 00 00 00 0a 00 06 00 00 00 00 00 0c 00 00 00 00 00 00
    mem 00 03 00 02 00 03
    mem 00 65 00 06 00 09
    1014
    1009
    1005
    1002
    mem 00 00 03 e8 00 00 03 ea 00 00 03 ed 00 00 03 f1 00 00 03 f6 00 00 00 00
    4
    6
    8
    => 5
    mem 00 01 00 65 00 66 00 04
    mem 00 00 00 0a 00 0b 00 0c 00 0d 00 0e
    mem 00 01 00 02 00 00 00 00 00 00 00 00 00 00 00 00 00 09 00 0a
    mem ff ff ff ff 00 00 00 00 00 00 00 00 00 00 00 00 ff ff ff ff
    mem 00 65 00 66 00 67 00 00 00 00 00 00 00 00 00 00 00 00 00 00
    mem 00 00 00 00 00 00 00 00 00 00 00 6a 00 6b 00 6c 00 6d 00 6e
    mem ff ff ff ff ff ff ff ff 00 00 00 00 00 00 00 00 00 00 00 00 ff ff ff ff
    mem 61 00 00 00 00 66
    mem 00 00 00 07 00 00 00 0e 00 00 00 15 00 00 00 00
}
module ixf;
%include 'exec.ins.pas';

type
  warr = array[1..10] of integer;
  wp = ^warr;
  larr = array[1..6] of integer32;
  grid = array[1..4, 1..4] of integer;
  big = array[1..20000] of integer32;
  bigp = ^big;
  lp8 = array[1..6] of integer32;
var
  wa: warr;
  wb: warr;
  wl: larr;
  g: grid;
  la: lp8;
  fa: array[200..205] of integer;
  cs: array[1..6] of char;
  gp: wp;
  q: ^integer32;
  ixf_$xl: extern larr;

procedure ixf_$xreset; extern;

procedure ixf_$init;
var
  k, m: integer;
begin
  k := 1;
  m := 101;
  while k <= 10 do begin
    wa[k] := k;
    wb[k] := m;
    k := k + 1;
    m := m + 1;
  end;
  k := 1;
  m := 97;
  while k <= 6 do begin
    wl[k] := 0;
    la[k] := 0;
    cs[k] := chr(m);
    k := k + 1;
    m := m + 1;
  end;
  { g: zero from the image's load, written by ixf_$rows alone }
end;

{ a[i + 1] from another array: wa = 1, 102 .. 110 }
procedure ixf_$plus1;
var
  i: integer;
begin
  ixf_$init;
  for i := 1 to 9 do
    wa[i + 1] := wb[i] + 1;
  exec_$log_mem(wa, 20);
end;

{ a[i + 1] from a[i]: each pass reads the last one's store, 50 .. 59 }
procedure ixf_$shift;
var
  i: integer;
begin
  ixf_$init;
  wa[1] := 50;
  for i := 1 to 9 do
    wa[i + 1] := wa[i] + 1;
  exec_$log_mem(wa, 20);
end;

{ a[i - 1]: wa = 102 .. 110, 10 }
procedure ixf_$minus1;
var
  i: integer;
begin
  ixf_$init;
  for i := 2 to 10 do
    wa[i - 1] := wb[i];
  exec_$log_mem(wa, 20);
end;

{ a[2 * i] of longwords: 0, 1000, 0, 2000, 0, 3000 }
procedure ixf_$times2;
var
  i: integer;
begin
  ixf_$init;
  for i := 1 to 3 do
    wl[2 * i] := i * 1000;
  exec_$log_mem(wl, 24);
end;

{ rows: g[i][2] = i + 3, g[i + 1][1] = 2 * g[i][2] }
procedure ixf_$rows;
var
  i, j: integer;
begin
  ixf_$init;
  j := 3;
  for i := 1 to 3 do begin
    g[i][2] := i + j;
    g[i + 1][1] := g[i][2] * 2;
  end;
  exec_$log_mem(g, 32);
end;

{ the pointer assigned in the body: wa[1] = 3, then wb[2] = 6, wb[3] = 9
  (no call, so that the pointer is held in an address register,
  codegen-patterns 7.4, 12.5; a pointer read from memory, or one a call
  in the body changes, is refused by the indexing itself, bead
  domain-pascal-nqg1) }
procedure ixf_$ptrchg_in;
var
  i: integer;
  lp: wp;
begin
  lp := addr(wa);
  for i := 1 to 3 do begin
    lp^[i] := i * 3;
    lp := addr(wb);
  end;
end;

procedure ixf_$ptrchg;
begin
  ixf_$init;
  ixf_$ptrchg_in;
  exec_$log_mem(wa, 6);
  exec_$log_mem(wb, 6);
end;

{ `.l' indices (big's bounds times 4 do not fit in 16 bits): la[1] =
  1000, la[i] = la[i - 1] + i (a running sum: a computed index into
  such an array is refused in dpc-ir, bead domain-pascal-wkg), logged downto with a call in the body }
procedure ixf_$wide_in(bp: bigp);
  options(val_param);
var
  i: integer;
  s: integer32;
begin
  s := 1000;
  bp^[1] := s;
  for i := 2 to 5 do begin
    s := s + i;
    bp^[i] := s;
  end;
  for i := 5 downto 2 do
    exec_$logv_l(bp^[i]);
end;

procedure ixf_$wide;
var
  bp: bigp;
begin
  ixf_$init;
  bp := addr(la);
  ixf_$wide_in(bp);
  exec_$log_mem(la, 24);
end;

{ an index computed in a call's argument: 2 * wa[i + 1], 4, 6, 8 }
procedure ixf_$callidx;
var
  i: integer;
begin
  ixf_$init;
  for i := 1 to 3 do
    exec_$logv_w(wa[i + 1] * 2);
end;

{ the index after an `exit': wa[6] = 0 is met at i = 5 }
function ixf_$exitafter: integer;
var
  i: integer;
begin
  ixf_$init;
  wa[6] := 0;
  for i := 1 to 9 do
    if wa[i + 1] = 0 then
      exit;
  ixf_$exitafter := i;
end;

{ two passes from 1, the index's initial value the count's: wa = 1, 101,
  102, 4 }
procedure ixf_$two;
var
  i: integer;
begin
  ixf_$init;
  for i := 1 to 2 do
    wa[i + 1] := wb[i];
  exec_$log_mem(wa, 8);
end;

{ an index from 200: fa[201..205] = 10 .. 14, fa[200] untouched (0) }
procedure ixf_$far;
var
  i: integer;
begin
  for i := 200 to 204 do
    fa[i + 1] := i - 190;
  exec_$log_mem(fa, 12);
end;

{ clearing loops: the unit's data from its third element }
procedure ixf_$fillw;
var
  i: integer;
begin
  ixf_$init;
  for i := 3 to 8 do
    wa[i] := 0;
  exec_$log_mem(wa, 20);
end;

{ a frame array, elements 2 .. 4 }
procedure ixf_$fillframe;
var
  i, k: integer;
  loc: array[1..5] of integer32;
begin
  k := 1;
  while k <= 5 do begin
    loc[k] := -1;
    k := k + 1;
  end;
  for i := 2 to 4 do
    loc[i] := 0;
  exec_$log_mem(loc, 20);
end;

{ through a by-value pointer, elements 4 .. 10 }
procedure ixf_$fillptr(p: wp);
  options(val_param);
var
  i: integer;
begin
  for i := 4 to 10 do
    p^[i] := 0;
end;

procedure ixf_$fillptr_run;
begin
  ixf_$init;
  ixf_$fillptr(addr(wb));
  exec_$log_mem(wb, 20);
end;

{ through a `var' parameter, elements 1 .. 5 }
procedure ixf_$fillvar(var a: warr);
var
  i: integer;
begin
  for i := 1 to 5 do
    a[i] := 0;
end;

procedure ixf_$fillvar_run;
begin
  ixf_$init;
  ixf_$fillvar(wb);
  exec_$log_mem(wb, 20);
end;

{ another unit's array from its third element }
procedure ixf_$fillext;
var
  i: integer;
begin
  ixf_$xreset;
  for i := 3 to 5 do
    ixf_$xl[i] := 0;
  exec_$log_mem(ixf_$xl, 24);
end;

{ a `char' array, elements 2 .. 5 (`clr.b') }
procedure ixf_$fillc;
var
  i: integer;
begin
  ixf_$init;
  for i := 2 to 5 do
    cs[i] := chr(0);
  exec_$log_mem(cs, 6);
end;

{ a stepped element's address stored, then stored through: 7, 14, 21 }
procedure ixf_$addrs;
var
  i: integer;
begin
  ixf_$init;
  for i := 1 to 3 do begin
    q := addr(wl[i]);
    q^ := i * 7;
  end;
  exec_$log_mem(wl, 16);
end;
