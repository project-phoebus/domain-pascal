{ Execution test: `for' loops over a signed word index (`integer') to an
  unsigned word bound (`0..65535', Apollo's pinteger): `from <= to'
  compares an unsigned run-time value with a non-negative constant, an
  unsigned comparison (ref 4.3.3, pp. 4-6..4-7; reference-clarifications
  question 32), so a bound above 16#7FFF runs its passes, as the image's
  flp_format_track does (0xE3DC9E `tst.w (0x20,A0); beq.b; move.w
  (0x20,A0),D0; subq.w #1,D0', an `integer' index to a `pinteger' field;
  codegen-patterns 7.4; bead domain-pascal-3iws).  Before, the guard was
  signed (`subq.w #1; bmi', `tst.w; bmi' from 0): such a bound ran no
  pass.  From 1 (the bound through a pointer, in a register, in the
  unit's data), from 0, from 2, to `n - 1', `downto', both bounds at run
  time, and from a negative constant (signed, unchanged).  Review
  additions: a volatile bound read once (ref 3.15.1, p. 3-56; two reads
  ran 2 passes for 1 and 65536 for 65535), `1 to v' and `0 to v - 1';
  bounds computed from unsigned words, kept at 32 bits (`32767 to n -
  1', `n - 1 downto 0', `a to b - 1', `a downto b - 1', `b - 1 downto
  a', `a + 1 to b').  Second review: `to a - c' with c beyond a word
  (`1 to a - 39999' ran 25537 passes for a = 0); a function result and a
  bit field as unsigned words (`a to f(b) - 1', `f(a) - 1 downto b',
  `pk.x - 1 downto b'); an unsigned and a signed run-time bound (`a to
  n' ran 32774 passes for a = 32768, n = 5); a bound whose range
  overflows 64 bits (dpc panicked).  The body never reads the index,
  whose values past 32767 lie outside its type.

  call: FORUB_$PTR @w:0
  call: FORUB_$PTR @w:5
  call: FORUB_$PTR @w:40000
  call: FORUB_$PTR @w:65535
  call: FORUB_$REG w:0
  call: FORUB_$REG w:1
  call: FORUB_$REG w:32768
  call: FORUB_$UNIT w:3
  call: FORUB_$UNIT w:50000
  call: FORUB_$FROM0 w:0
  call: FORUB_$FROM0 w:7
  call: FORUB_$FROM0 w:65535
  call: FORUB_$FROM2 w:1
  call: FORUB_$FROM2 w:2
  call: FORUB_$FROM2 w:40000
  call: FORUB_$MINUS w:0
  call: FORUB_$MINUS w:1
  call: FORUB_$MINUS w:40000
  call: FORUB_$DOWN w:0
  call: FORUB_$DOWN w:4
  call: FORUB_$DOWN w:40000
  call: FORUB_$RTFROM w:3 w:5
  call: FORUB_$RTFROM w:6 w:5
  call: FORUB_$RTFROM w:1 w:40000
  call: FORUB_$RTFROM w:40000 w:5
  call: FORUB_$NEG w:0
  call: FORUB_$NEG w:5
  call: FORUB_$VOL w:1
  call: FORUB_$VOL w:5
  call: FORUB_$VOL w:65535
  call: FORUB_$HI w:0
  call: FORUB_$HI w:32768
  call: FORUB_$HI w:65535
  call: FORUB_$HI w:8001
  call: FORUB_$AB w:1 w:65535
  call: FORUB_$AB w:1 w:0
  call: FORUB_$AB w:3 w:5
  call: FORUB_$AB w:5 w:3
  call: FORUB_$AB w:40000 w:1
  call: FORUB_$AB w:0 w:0
  call: FORUB_$AB w:65535 w:5
  call: FORUB_$AB w:0 w:40000
  call: FORUB_$BIGC w:0
  call: FORUB_$BIGC w:40000
  call: FORUB_$BIGC w:65535
  call: FORUB_$FN w:65535 w:0
  call: FORUB_$FN w:1 w:40000
  call: FORUB_$FN w:40000 w:1
  call: FORUB_$FN w:1 w:65535
  call: FORUB_$MIXED w:32768 w:5 w:32768
  call: FORUB_$MIXED w:3 w:5 w:1
  call: FORUB_$MIXED w:40000 w:-1 w:0
  call: FORUB_$MIXED w:1 w:0 w:40000
  call: FORUB_$MIXED w:0 w:0 w:0
  call: FORUB_$HUGE w:0
  expect:
    0
    5
    40000
    65535
    0
    1
    32768
    3
    50000
    1
    8
    65536
    0
    1
    39999
    0
    0
    39999
    0
    4
    40000
    3
    0
    40000
    0
    2
    7
    1
    1
    5
    5
    65535
    65535
    0
    0
    1
    32768
    32768
    65535
    0
    8001
    65534
    0
    65534
    65534
    0
    3
    0
    0
    2
    0
    2
    2
    0
    4
    0
    0
    0
    40001
    0
    0
    0
    2
    0
    0
    0
    65532
    0
    0
    40000
    0
    40000
    40000
    0
    0
    0
    1
    0
    0
    25536
    0
    0
    0
    65535
    65535
    39999
    0
    0
    0
    39999
    39999
    65534
    0
    0
    0
    0
    3
    0
    0
    0
    0
    19999
    1
    0
    0
}
module forub;

{ exec.ins.pas's declarations this test uses, the device register read
  as an unsigned word (each read returns the next value) }
type
  exec_$uword = 0..65535;

procedure exec_$logv_l(v: integer32); options(val_param); extern;
function exec_$id_l(in v: integer32): integer32; extern;

var
  exec_$counter: extern [volatile] exec_$uword;

type
  geom_t = record
    spt: exec_$uword;
  end;

  pk_t = packed record
    tag: 0..15;
    x: exec_$uword;                       { bits 4..19: a bit field }
  end;

var
  pk: pk_t;
  cnt: integer32;
  bound: exec_$uword;

{ from 1 to a field reached through the parameter's address
  (flp_format_track's shape) }
procedure forub_$ptr(in g: geom_t);
var
  i: integer;
begin
  cnt := 0;
  for i := 1 to g.spt do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ from 1 to a by-value parameter held in a register }
procedure forub_$reg(n: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := 1 to n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ from 1 to a variable of the unit }
procedure forub_$unit(n: exec_$uword); options(val_param);
var
  i: integer;
begin
  bound := n;
  cnt := 0;
  for i := 1 to bound do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ from 0: `0 <= n' always holds }
procedure forub_$from0(n: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := 0 to n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

procedure forub_$from2(n: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := 2 to n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ to `n - 1': no pass for n = 0 (the bound's value is -1) }
procedure forub_$minus(n: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := 1 to n - 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

procedure forub_$down(n: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := n downto 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ both bounds unsigned run-time values }
procedure forub_$rtfrom(a, b: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := a to b do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ from a negative constant: compared signed, as before }
procedure forub_$neg(n: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := -1 to n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ a volatile bound is read once (ref 3.15.1, p. 3-56): the counter set to
  n, the loop runs n passes }
procedure forub_$vol(n: exec_$uword); options(val_param);
var
  i: integer;
begin
  exec_$counter := n;
  cnt := 0;
  for i := 1 to exec_$counter do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  exec_$counter := n;
  cnt := 0;
  for i := 0 to exec_$counter - 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ bounds computed from unsigned words: their values, not their low words
  read as signed }
procedure forub_$hi(n: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := 32767 to n - 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := n - 1 downto 0 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

procedure forub_$ab(a, b: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := a to b - 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := a downto b - 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := b - 1 downto a do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := a + 1 to b do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ `to x - c' with c beyond a word: computed at 32 bits, not folded }
procedure forub_$bigc(a: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := 1 to a - 39999 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 1 to a - 65535 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 0 to a - 65536 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

function forub_f(x: exec_$uword): exec_$uword; options(val_param, internal);
begin
  forub_f := x;
end;

{ a function result and a bit field as unsigned words }
procedure forub_$fn(a, b: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := a to forub_f(b) - 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := forub_f(a) - 1 downto b do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  pk.x := a;
  cnt := 0;
  for i := pk.x - 1 downto b do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ an unsigned word and a signed one: the longword domain }
procedure forub_$mixed(a: exec_$uword; n: integer; b: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := a to n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := a to b div 2 - 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ a bound whose range is far beyond 64 bits (dpc's range check of it
  overflowed and panicked) }
procedure forub_$huge(a: exec_$uword); options(val_param);
var
  i: integer;
begin
  cnt := 0;
  for i := 1 to a * 30000 * 30000 * 30000 * 30000 * 30000 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;
