{ Execution test: `lshft' and `rshft' of a 16-bit operand, an integer32
  (owner's ruling, reference-clarifications "Lshft of a 16-bit operand;
  the sign of a shift of an unsigned word"; bead domain-pascal-4rfc;
  codegen-patterns 8, "Shifts of a 16-bit operand").  A `0..65535' word
  is widened with zeros and shifted in 32 bits ("Rshft", p. 4-171), so no bit is
  lost above bit 15; stored into a word the low word is kept (a
  `0..65535' widened back with zeros, an `integer' with its sign).  The
  result is unsigned: `lshft(32768, 16)' (0x80000000) is greater than 0,
  and so is its sum with another word.

  Per call, a word a, a count k (a variable), a `0..2147483647' u and a
  word base = 16#1234:
  lshft(a, k) and rshft(a, k) as integer32; lshft(a, k) into a
  `0..65535'; rshft(a, k) into an `integer'; lshft(a, k) into an
  `integer'; lshft(a, k) > u; base + lshft(a, k) > u.  Then the
  constant-count forms with a = 65535.

  An `integer' i shifted by a count n (in a variable), expanded by its
  sign: lshft(i, n) as an integer32 and stored into an `integer' (ref
  p. 4-109's example, lshft(2000, 7): 256000 and -6144); rshft(i, n) as
  an integer32 and into an `integer' (ref p. 4-171's example, rshft(-9,
  3): 16#1FFFFFFE and -2); lshft(i, n) > 5 as an integer32 (true for
  2000 shifted by 7).  Then `div' and `mod' of shifts of a `0..65535'
  word in their unsigned forms (ref 4.3.3): rshft(a, k) div u,
  lshft(a, k) div 2 (lshft(65535, 16) = 16#FFFF0000, unsigned) and
  lshft(a, m) mod u (M$OIU$LLL; m keeps the dividend below 2^31, as
  M$OIU$LLL is the signed routine, one entry with M$OIS$LLL at 0xE0AD7C,
  codegen-patterns 8).

  call: SWX_$RUN w:-1 w:1 l:100000 w:4660
  call: SWX_$RUN w:-25536 w:15 l:2147483647 w:4660
  call: SWX_$RUN w:-32768 w:16 l:0 w:4660
  call: SWX_$RUN w:1 w:4 l:15 w:4660
  call: SWX_$RUN w:-32767 w:3 l:262144 w:4660
  call: SWX_$CONST w:-1
  call: SWX_$SIGNED w:2000 w:7
  call: SWX_$SIGNED w:-9 w:3
  call: SWX_$SIGNED w:-1 w:1
  call: SWX_$DIV w:-1 w:3 l:3 w:3
  call: SWX_$DIV w:-1 w:16 l:7 w:15
  expect:
    0x0001fffe
    0x00007fff
    0x0000fffe
    0x00007fff
    0xfffffffe
    0x00000001
    0x00000001
    0x4e200000
    0x00000001
    0x00000000
    0x00000001
    0x00000000
    0x00000000
    0x00000000
    0x80000000
    0x00000000
    0x00000000
    0x00000000
    0x00000000
    0x00000001
    0x00000001
    0x00000010
    0x00000000
    0x00000010
    0x00000000
    0x00000010
    0x00000001
    0x00000001
    0x00040008
    0x00001000
    0x00000008
    0x00001000
    0x00000008
    0x00000001
    0x00000001
    0x7fff8000
    0x0000000f
    0x00001fff
    0x0000ffe0
    0x0003e800
    0xffffe800
    0x0000000f
    0x0000000f
    0x00000001
    0xffffffb8
    0xffffffb8
    0x1ffffffe
    0xfffffffe
    0x00000000
    0xfffffffe
    0xfffffffe
    0x7fffffff
    0xffffffff
    0x00000000
    0x00000aaa
    0x0003fffc
    0x00000000
    0x00000000
    0x7fff8000
    0x00000001
}
module swx;
%include 'exec.ins.pas';

type
  pinteger = 0..65535;
  linteger = 0..2147483647;

var
  v: pinteger;
  i: integer;
  l: integer32;

procedure swx_$run(wa: pinteger; wk: pinteger; u: linteger; base: pinteger);
  options(val_param);
begin
  l := lshft(wa, wk);
  exec_$log_x(l);
  l := rshft(wa, wk);
  exec_$log_x(l);
  v := lshft(wa, wk);
  l := v;
  exec_$log_x(l);
  i := rshft(wa, wk);
  l := i;
  exec_$log_x(l);
  i := lshft(wa, wk);
  l := i;
  exec_$log_x(l);
  if lshft(wa, wk) > u then l := 1 else l := 0;
  exec_$log_x(l);
  if base + lshft(wa, wk) > u then l := 1 else l := 0;
  exec_$log_x(l);
end;

procedure swx_$const(wa: pinteger); options(val_param);
begin
  l := lshft(wa, 15);
  exec_$log_x(l);
  l := rshft(wa, 12);
  exec_$log_x(l);
  v := rshft(wa, 3);
  l := v;
  exec_$log_x(l);
  v := lshft(wa, 5);
  l := v;
  exec_$log_x(l);
end;

procedure swx_$signed(i: integer; n: integer); options(val_param);
var
  j: integer;
begin
  l := lshft(i, n);
  exec_$log_x(l);
  j := lshft(i, n);
  l := j;
  exec_$log_x(l);
  l := rshft(i, n);
  exec_$log_x(l);
  j := rshft(i, n);
  l := j;
  exec_$log_x(l);
  if lshft(i, n) > 5 then l := 1 else l := 0;
  exec_$log_x(l);
end;

procedure swx_$div(wa: pinteger; wk: pinteger; u: linteger; mk: pinteger);
  options(val_param);
begin
  l := rshft(wa, wk) div u;
  exec_$log_x(l);
  l := lshft(wa, wk) div 2;
  exec_$log_x(l);
  l := lshft(wa, mk) mod u;
  exec_$log_x(l);
end;
