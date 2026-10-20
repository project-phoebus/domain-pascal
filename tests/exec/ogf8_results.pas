{ Execution test (bead domain-pascal-ogf8): function results.
  - An integer32 result in A0 (`a0_return', ref 5.5.11 p. 5-20),
    returned as a pointer result is (TTY_$I_GET_DESC 0xE667B0): from
    memory in a body of one statement, from a local in a body of several;
    the caller stores it, uses it in an expression (A0 parked in D0,
    `move.l A0,D0', PROC2_$GET_CR_REC 0xE40186) and passes such an
    expression on.
  - Set results of one word and one longword in D0 (ACL_$MIN_RIGHTS
    0xE469F2, PROC2_$SIGBLOCK 0xE3F6B4): a one-byte set widened into a
    word result (acl_$eval_acl_entries), a constructor with a run-time
    member, a difference; the caller stores them.
  - A 4-byte record result assigned a constant and returned
    (MST_$ALLOC_TABLE_PAGE 0xE43F72), and assigned a function's result
    (best effort).
  - A pointer result variable that no call follows, held in A1 (best
    effort; AST_$LOCATE_ASTE's shape): the element's address returned
    early, a pointer function's result otherwise.

  warns: best effort (not byte-identical): a record function result assigned other than a record variable or a constant
  warns: best effort (not byte-identical): a pointer result variable held in a register that no call follows
  call: OGF8_RESULTS_$A0 w:1
  call: OGF8_RESULTS_$A0 w:3
  call: OGF8_RESULTS_$A0_DIRECT w:2 -> a0
  call: OGF8_RESULTS_$A0_MULTI w:2 -> a0
  call: OGF8_RESULTS_$A0_MULTI w:9 -> a0
  call: OGF8_RESULTS_$SETS w:3
  call: OGF8_RESULTS_$SETS w:0
  call: OGF8_RESULTS_$SET_DIRECT w:4 -> uw
  call: OGF8_RESULTS_$RECS w:-1
  call: OGF8_RESULTS_$RECS w:7
  call: OGF8_RESULTS_$PTR w:2 w:20
  call: OGF8_RESULTS_$PTR w:2 w:21
  call: OGF8_RESULTS_$PTR w:0 w:30
  call: OGF8_RESULTS_$PTR w:0 w:99
  expect:
    1000
    900
    1001
    3000
    2900
    3001
    => 0x000007d0
    => 0x000007d0
    => 0x00000000
    mem 00 08
    mem 00 00 00 06
    mem 00 01
    mem 00 00 00 06
    mem 80 01
    mem 80 00 00 06
    => 16
    262158
    -5
    7
    20
    21
    21
    22
    30
    22
    0
    0
}
module ogf8_results;
%include 'exec.ins.pas';

type
  bs = set of 0..7;
  ws = set of 0..15;
  ls = set of 0..31;
  pr = packed record
    a: bs;
    b: bs;
  end;
  st_t = record
    all: integer32;
  end;
  ip = ^integer32;

var
  tbl: array[1..3] of integer32 := [1000, 2000, 3000];
  vals: array[1..3] of integer32 := [10, 20, 30];
  l: integer32;
  w: ws;
  lw: ls;
  rc: pr;
  rs: st_t;
  pp: ip;

function a0f(i: integer): integer32; options(val_param, a0_return);
begin
  a0f := tbl[i];
end;

function a0g(i: integer): integer32; options(val_param, a0_return);
var
  v: integer32;
begin
  a0g := 0;
  if i > 3 then
    return;
  v := tbl[i];
  a0g := v;
end;

procedure ogf8_results_$a0(i: integer); options(val_param);
begin
  l := a0f(i);
  exec_$log_l(l);
  l := a0f(i) - 100;
  exec_$log_l(l);
  exec_$logv_l(a0f(i) + 1);
end;

function ogf8_results_$a0_direct(i: integer): integer32;
  options(val_param, a0_return);
begin
  ogf8_results_$a0_direct := tbl[i];
end;

function ogf8_results_$a0_multi(i: integer): integer32;
  options(val_param, a0_return);
var
  v: integer32;
begin
  ogf8_results_$a0_multi := 0;
  if i > 3 then
    return;
  v := tbl[i];
  ogf8_results_$a0_multi := v;
end;

function sf(in q: pr): ws;
begin
  sf := q.a;
end;

function sg(x: integer): ws; options(val_param);
begin
  sg := [x] + [15];
end;

function sh(in m: ls): ls;
begin
  sh := m - [0];
end;

procedure ogf8_results_$sets(x: integer); options(val_param);
begin
  rc.a := [x];
  rc.b := [0];
  w := sf(rc);
  exec_$log_mem(w, 2);
  lw := [0..2];
  lw := sh(lw);
  exec_$log_mem(lw, 4);
  if x = 0 then begin
    w := sg(x);
    exec_$log_mem(w, 2);
    lw := sh([0..2, 31]);
    exec_$log_mem(lw, 4);
  end;
end;

function ogf8_results_$set_direct(x: integer): ws; options(val_param);
begin
  ogf8_results_$set_direct := [x];
end;

function rf(x: integer): st_t; options(val_param);
begin
  if x < 0 then begin
    rf := st_t(integer32(16#4000e));
    return;
  end;
  l := x;
  rf := st_t(exec_$id_l(l));
end;

function rg(x: integer): st_t; options(val_param);
begin
  rg := rf(x);
end;

procedure ogf8_results_$recs(x: integer); options(val_param);
begin
  rs := rf(x);
  exec_$log_l(rs.all);
  if x < 0 then begin
    rs := rg(-5);
    rs.all := rs.all - 262163;
    exec_$log_l(rs.all);
  end;
end;

function pf(i: integer; v: integer32): ip; options(val_param);
var
  p: ip;
begin
  if (i <> 0) and then (i <= 3) then begin
    p := addr(vals[i]);
    if p^ = v then begin
      pf := p;
      return;
    end;
  end;
  if v = 99 then
    pf := nil
  else
    pf := ip(exec_$id_p(addr(vals[3])));
end;

procedure ogf8_results_$ptr(i: integer; v: integer); options(val_param);
begin
  pp := pf(i, v);
  if pp <> nil then begin
    exec_$log_l(pp^);
    pp^ := pp^ + 1;
    exec_$log_l(vals[2]);
  end
  else begin
    exec_$log_l(0);
    exec_$log_l(0);
  end;
end;
