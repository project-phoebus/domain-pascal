{ Execution test (bead domain-pascal-yqf6): pointer forms the link test
  stopped at.

  - `T(addr(x))^`, a view of x through a pointer type transfer: x itself
    (SIO_$INIT_LINE 0xE32B40 `lea (692,A2),A0`), read and written, a
    field and an element of it.
  - A view of a volatile object is a volatile reference: each read of
    `T(addr(exec_$counter))^` reads the device word again.
  - A computed pointer dereferenced, `T(integer32(p) + n)^`, `T(-v)^`
    (best effort: the value in a data register, then `movea.l Dn,An`).
  - A pointer result assigned an address, `f := addr(a[i])` (`lea` into
    the result's register, EC2_$GET_EC1_ADDR 0xE42B0A), last and before
    a call, and a computed one, `f := univ_ptr(integer32(p) + 2)` (best
    effort).
  - A pointer local assigned from calls in a loop, its last use in the
    text before the assignment (a panic before the review of bead
    domain-pascal-yqf6), with and without a call after it.
  - A pointer local assigned from calls that no call follows, assigned
    more than once and compared with `nil` (best effort: the register
    allocation gave it, NET_$FIND_HANDLER, AST_$GET_LOCATION).

  warns: best effort (not byte-identical): a computed pointer dereferenced
  warns: best effort (not byte-identical): a computed pointer result
  warns: best effort (not byte-identical): a pointer local assigned from a call
  call: BE_PTR_VIEWS_$VIEW l:10
  call: BE_PTR_VIEWS_$VOL
  call: BE_PTR_VIEWS_$COMP w:4
  call: BE_PTR_VIEWS_$RES w:3
  call: BE_PTR_VIEWS_$PLOCAL w:5 @w:9!
  call: BE_PTR_VIEWS_$PLOCAL w:-1 @w:9!
  call: BE_PTR_VIEWS_$PLOOP w:2 w:1
  call: BE_PTR_VIEWS_$PLOOP w:4 w:0
  expect:
    6488075
    10
    5
    5
    6
    7
    77
    11
    33
    13
    31
    32
    66
    32
    out 22
    out -1
    2
    1
    3
    0
}
module be_ptr_views;
%include 'exec.ins.pas';

type
  big_t = record
    x: integer32;
    y: integer32;
    z: integer;
  end;
  pair_t = record
    a: integer32;
    b: integer;
  end;
  pair_ptr_t = ^pair_t;
  words_t = array[0..4] of integer;
  words_ptr_t = ^words_t;
  w_ptr_t = ^integer;
var
  buf: array[0..7] of integer;
  pv: univ_ptr;

procedure be_ptr_views_$view(v: integer32); options(val_param);
var
  rec: big_t;
  i: integer;
begin
  rec.x := v;
  rec.y := v + 1;
  rec.z := 7;
  i := 4;
  pair_ptr_t(addr(rec))^.b := 99;
  exec_$log_l(rec.y);
  exec_$log_l(pair_ptr_t(addr(rec))^.a);
  words_ptr_t(addr(rec))^[i] := 5;
  exec_$log_w(rec.z);
end;

procedure be_ptr_views_$vol;
var
  r1, r2: integer;
begin
  w_ptr_t(addr(exec_$counter))^ := 5;
  r1 := w_ptr_t(addr(exec_$counter))^;
  r2 := w_ptr_t(addr(exec_$counter))^;
  exec_$log_w(r1);
  exec_$log_w(r2);
end;

procedure be_ptr_views_$comp(off: integer); options(val_param);
var
  lv: integer32;
begin
  buf[0] := 10;
  buf[1] := 11;
  buf[2] := 12;
  buf[3] := 13;
  buf[1] := 7;
  exec_$log_w(buf[1]);
  pv := addr(buf);
  w_ptr_t(integer32(pv) + off)^ := 77;
  exec_$log_w(buf[2]);
  buf[1] := 11;
  exec_$log_w(w_ptr_t(integer32(pv) + 2)^);
  pv := addr(buf[3]);
  lv := -integer32(pv);
  w_ptr_t(-lv)^ := 33;
  exec_$log_w(buf[3]);
end;

function be_ptr_views_$elem(i: integer): w_ptr_t; options(val_param);
begin
  exec_$clobber;
  be_ptr_views_$elem := addr(buf[i]);
end;

function be_ptr_views_$elem2(i: integer): w_ptr_t; options(val_param);
begin
  be_ptr_views_$elem2 := addr(buf[i]);
  exec_$clobber;
end;

function be_ptr_views_$next(p: univ_ptr): w_ptr_t; options(val_param);
begin
  exec_$clobber;
  be_ptr_views_$next := univ_ptr(integer32(p) + 2);
end;

{ One statement: `lea` straight into A0 (GET_WIRED 0xE1D8E8); a computed
  value through the slot (best effort). }
function be_ptr_views_$whole: w_ptr_t;
begin
  be_ptr_views_$whole := addr(buf[6]);
end;

function be_ptr_views_$plus(p: univ_ptr): w_ptr_t; options(val_param);
begin
  be_ptr_views_$plus := univ_ptr(integer32(p) - 2);
end;

procedure be_ptr_views_$res(i: integer); options(val_param);
var
  p: w_ptr_t;
begin
  buf[3] := 13;
  p := be_ptr_views_$elem(i);
  exec_$log_w(p^);
  buf[4] := 31;
  buf[5] := 32;
  p := be_ptr_views_$elem2(i + 1);
  exec_$log_w(p^);
  p := be_ptr_views_$next(p);
  exec_$log_w(p^);
  buf[6] := 66;
  p := be_ptr_views_$whole;
  exec_$log_w(p^);
  p := be_ptr_views_$plus(p);
  exec_$log_w(p^);
end;

procedure be_ptr_views_$plocal(i: integer; out r: integer);
  options(val_param);
var
  p: w_ptr_t;
begin
  buf[5] := 5;
  if i < 0 then
    pv := nil
  else
    pv := addr(buf[i]);
  p := exec_$id_p(pv);
  r := 0;
  if p = nil then
    r := -1
  else begin
    p^ := p^ + 17;
    r := p^;
  end;
end;

procedure be_ptr_views_$ploop(i, c: integer); options(val_param);
var
  p: w_ptr_t;
  k: integer;
begin
  buf[i] := 0;
  buf[i + 1] := 0;
  p := exec_$id_p(addr(buf[i]));
  k := 0;
  while k < 3 do begin
    p^ := p^ + 1;
    if k = 1 then
      p := exec_$id_p(addr(buf[i + c]));
    if c <> 0 then
      exec_$clobber;
    k := k + 1;
  end;
  exec_$log_w(buf[i]);
  exec_$log_w(buf[i + 1]);
end;
