{ Execution test (bead domain-pascal-sq4a): best-effort paths for
  refusals of the link test's ranking.
  - A `[byte] 0..255` widened to a longword (codegen-patterns 8,
    "Widening and narrowing": `clr.l; move.b` from memory, MST_$LOOKUP_
    OBJECT 0xE43D52; `andi.l #0xff` on a computed byte), values with the
    high bit set.
  - A signed `mod` of a longword by a word used as a longword
    (`M$OIS$WLW`, its word result sign-extended); `mod` takes the
    dividend's sign (ref p. 4-115).
  - An M$ARITH product whose operands are both computed, one or both
    with a call (`a` first, then `b`).
  - An empty `otherwise` part: an unmatched selector does nothing (ref
    "Case", pp. 4-36..4-37).
  - A `goto` into a `with`, a `for`, a `while` and a `case` arm from
    outside it (ref p. 4-95 against Appendix D p. D-4, open question 30:
    a plain jump, the code at the label not reading the statement's set
    up).
  - `&`, `!` and `xor` of an address and a constant (FLP_$DINIT
    0xE3E17A `lea; move.l A0,D1; andi.l #0x3ff,D1`), checked against the
    same operation on the address passed through a pointer.

  call: BS_$WIDEN b:200
  call: BS_$WIDEN b:255
  call: BS_$MODW l:-17 w:5
  call: BS_$MODW l:17 w:-5
  call: BS_$MODW l:100000 w:7
  call: BS_$BOTH l:6 l:7 l:4
  call: BS_$BOTH l:-3 l:5 l:-2
  call: BS_$CASE w:1
  call: BS_$CASE w:2
  call: BS_$CASE w:3
  call: BS_$GOTO_WITH w:0
  call: BS_$GOTO_WITH w:4
  call: BS_$GOTO_FOR w:0
  call: BS_$GOTO_FOR w:7
  call: BS_$GOTO_LOOP w:0
  call: BS_$GOTO_LOOP w:1
  call: BS_$GOTO_CASE w:0
  call: BS_$GOTO_CASE w:2
  call: BS_$ADDR_OPS
  warns: best effort (not byte-identical): widening a byte to a longword
  warns: best effort (not byte-identical): a signed `mod` by a word used as a longword
  warns: best effort (not byte-identical): an M$ARITH call with both operands computed
  warns: best effort (not byte-identical): an empty `otherwise` part
  warns: best effort (not byte-identical): a `goto` into a `for`, `while`, `repeat`, `with` or `case` statement
  warns: best effort (not byte-identical): `!` or `xor` of an address (`addr`) and a constant
  expect:
    200
    400
    50
    192
    255
    510
    63
    240
    -2
    99998
    2
    100002
    5
    100005
    210
    42
    18
    15
    0
    9
    10
    20
    0
    mark 0
    mark 100
    mark 5
    mark 4
    mark 100
    mark 0
    mark 99
    mark 1
    mark 2
    mark 3
    mark 7
    mark 99
    mark 0
    mark 99
    mark 99
    mark 0
    mark 99
    mark 1
    mark 99
    mark 0
    mark 99
    mark 2
    mark 40
    mark 99
    true
    true
    true
    true
}
module be_sq4a;
%include 'exec.ins.pas';
type
  ub = [byte] 0..255;
  pr = record a: ub; b: ub; end;
  wrec = record a: integer; c: integer; end;
var
  l: integer32;
  p: pr;
  u: ub;
  w: integer;
  r: wrec;
  buf: array[1..16] of integer;

procedure bs_$widen(x: ub); options(val_param);
begin
  u := x;
  l := u;
  exec_$logv_l(l);
  p.b := x;
  l := l + p.b;
  exec_$logv_l(l);
  l := rshft(x, 2) & 16#3f;
  exec_$logv_l(l);
  l := u & 16#f0;
  exec_$logv_l(l);
end;

procedure bs_$modw(a: integer32; c: integer); options(val_param);
begin
  l := a mod c;
  exec_$logv_l(l);
  l := (a mod c) + 100000;
  exec_$logv_l(l);
end;

procedure bs_$both(a: integer32; b: integer32; c: integer32); options(val_param);
begin
  { a * b by M$ARITH (a call), then times a computed value. }
  l := (a * b) * (c + 1);
  exec_$logv_l(l);
  { A computed value times a call's result. }
  l := (a + 1) * exec_$id_l(c + 2);
  exec_$logv_l(l);
  { A call's result times a computed value. }
  l := exec_$id_l(a) * (c - 1);
  exec_$logv_l(l);
end;

procedure bs_$case(x: integer); options(val_param);
begin
  l := 0;
  case x of
    1: l := 10;
    2: l := 20;
    otherwise
  end;
  exec_$logv_l(l);
end;

procedure bs_$goto_with(x: integer); options(val_param);
label 1;
begin
  if x = 0 then goto 1;
  with r do begin
    a := x + 1;
    exec_$mark(a);
    c := x;
  1:
    exec_$mark(x);
  end;
  exec_$mark(100);
end;

procedure bs_$goto_for(x: integer); options(val_param);
label 1;
var i: integer;
begin
  if x = 0 then goto 1;
  for i := 1 to 3 do begin
    exec_$mark(i);
    if i = 3 then begin
    1:
      exec_$mark(x);
      exec_$mark(99);
      return;
    end;
  end;
  exec_$mark(100);
end;

procedure bs_$goto_loop(x: integer); options(val_param);
label 1, 2, 3;
begin
  w := x;
  if x = 0 then goto 1;
  while w > 0 do begin
    w := w - 1;
  1:
    exec_$mark(w);
    if w = 0 then goto 2;
  end;
2:
  exec_$mark(99);
  if x = 0 then goto 3;
  repeat
    w := w + 1;
    exec_$mark(w);
  3:
  until w >= 0;
  exec_$mark(99);
end;

procedure bs_$goto_case(x: integer); options(val_param);
label 1;
begin
  if x = 0 then goto 1;
  case x of
    1: exec_$mark(10);
    2: begin
      exec_$mark(x);
    1:
      exec_$mark(20 * x);
    end;
    otherwise exec_$mark(30);
  end;
  exec_$mark(99);
end;

procedure bs_$addr_ops;
var
  q: univ_ptr;
  m: integer32;
  lb: array[1..8] of integer;
begin
  q := exec_$id_p(addr(buf));
  m := integer32(q);
  l := integer32(addr(buf)) & 16#3ff;
  exec_$log_bool(l = (m & 16#3ff));
  l := integer32(addr(buf)) ! 3;
  exec_$log_bool(l = (m - (m & 3)) + 3);
  l := xor(integer32(addr(buf)), 16#55);
  exec_$log_bool(l = (m + 16#55) - 2 * (m & 16#55));
  q := exec_$id_p(addr(lb));
  m := integer32(q);
  l := integer32(addr(lb[3])) & 16#ffff;
  exec_$log_bool(l = ((m + 4) & 16#ffff));
end;
