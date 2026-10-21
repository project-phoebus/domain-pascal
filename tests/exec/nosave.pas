{ Execution test: calls of options(nosave) routines (ref 5.5.8, p. 5-19:
  such a routine "does not preserve" D2-D7, A2-A4; it preserves A5 and
  A6; bead domain-pascal-7c4j).  The exec_$nosave* stubs clobber D0-D7
  and A0-A4, so nothing may stay in a register across them: a val_param
  parameter loaded at entry (the bead's repro), by-address parameters'
  slots, a value-parameter copy, register-held locals in a loop, a
  function result and an M$ARITH product kept for later.  The runner
  also checks that each routine gives its caller back D2-D7/A2-A4.

  call: NOSAVE_$REPRO w:5
  call: NOSAVE_$REPRO w:2
  call: NOSAVE_$BYADDR @w:7 @l:1000!
  call: NOSAVE_$COPY @w:-3 @l:70000
  call: NOSAVE_$LOOP w:4 -> w
  call: NOSAVE_$LOOP w:0 -> w
  call: NOSAVE_$RESULT l:123456 l:-7 -> l
  call: NOSAVE_$MIXED w:9 l:65537
  call: NOSAVE_$NOTHING_LIVE
  warns: best effort (not byte-identical): a call of an options(nosave) routine
  expect:
    mark 1
    mark 1
    mark 2
    7
    out 1007
    -3
    0
    70000
    -2
    70001
    1
    3
    6
    10
    => 10
    => 0
    123456
    => 123449
    9
    65537
    9
    589833
    mark 3
}
module nosave;
%include 'exec.ins.pas';

{ The bead's repro: n held in D2 across three calls. }
procedure nosave_$repro(n: integer); options(val_param);
begin
  exec_$nosave;
  exec_$nosave;
  if n > 3 then
    exec_$mark(1)
  else
    exec_$mark(2);
  exec_$nosave;
  if n > 3 then
    exec_$mark(1);
end;

{ By-address parameters (the default convention, ref 5.2 Table 5-1): the
  slots' addresses are needed after the call. }
procedure nosave_$byaddr(in a: integer; in out b: integer32);
begin
  exec_$nosave;
  b := b + a;
  exec_$nosave;
  exec_$logv_w(a);
end;

{ Value parameters by address, copied at entry (ref 5.2, p. 5-5). }
procedure nosave_$copy(a: integer; b: integer32);
var
  c: integer;
begin
  c := a + 1;
  exec_$nosave;
  exec_$logv_w(a);
  exec_$nosave_w(0);
  exec_$logv_l(b);
  exec_$nosave;
  exec_$logv_w(c);
  exec_$logv_l(b + 1);
end;

{ Register-held locals in a loop around a nosave call. }
function nosave_$loop(n: integer): integer; options(val_param);
var
  i, s: integer;
begin
  s := 0;
  for i := 1 to n do begin
    s := s + i;
    exec_$nosave;
    exec_$logv_w(s);
  end;
  nosave_$loop := s;
end;

{ A nosave function's result, with another value live across it. }
function nosave_$result(a, b: integer32): integer32; options(val_param);
var
  r: integer32;
begin
  r := exec_$nosave_id_l(a);
  exec_$logv_l(r);
  nosave_$result := r + b;
end;

{ Ordinary and nosave calls mixed; an M$ARITH product kept across a
  nosave call. }
procedure nosave_$mixed(a: integer; b: integer32); options(val_param);
var
  p: integer32;
begin
  p := b * a;
  exec_$clobber;
  exec_$nosave_w(a);
  exec_$logv_l(b);
  exec_$clobber;
  exec_$nosave_w(a);
  exec_$logv_l(p);
end;

{ Nothing live across the call: D2-D7/A2-A4 are still the caller's. }
procedure nosave_$nothing_live;
begin
  exec_$nosave;
  exec_$mark(3);
end;
