{ Execution test: `for' loops over an unsigned word index (`0..65535',
  Apollo's pinteger) to run-time bounds, at the edges 0, 1, 16#7FFF,
  16#8000 and 16#FFFF (bead domain-pascal-e9gw): `from <= to' of two
  unsigned words compares unsigned (ref 4.3.3, pp. 4-6..4-7; ref "For",
  p. 4-90: no pass for an empty range), so a bound above 16#7FFF runs its
  passes and 0 runs none.  The image's guard for such a loop is `tst.w;
  beq' (ct_transfer 0xE7732A and 0xE77484 in SAU5, MMAP_$RELEASE_PAGES
  0xE0D01A; codegen-patterns 7.4).  The shapes: a bound in a register,
  in the unit's data, through a pointer, under a `with', a `var'
  parameter, a frame slot whose address is passed, an array element, a
  function result, `n div 2', `n - 1', from 0 and from 2, `downto', both
  bounds at run time, a run-time `from' to a constant, a bound and an
  index a nested routine reaches, an index of the unit, an `integer'
  bound (compared unsigned in the index's type, PCHIST_$CNTL's nested
  0xE5CD1A), with and without a call in the body, a body that sums the
  index, and arrays the body reaches by the index (stepped pointers, as
  MMAP_$RELEASE_PAGES 0xE0D03A `lea (0x4,A0),A3`).

  call: FORUI_$A w:0
  call: FORUI_$A w:1
  call: FORUI_$A w:32767
  call: FORUI_$A w:32768
  call: FORUI_$A w:65535
  call: FORUI_$B @w:0
  call: FORUI_$B @w:1
  call: FORUI_$B @w:32767
  call: FORUI_$B @w:32768
  call: FORUI_$B @w:65535
  call: FORUI_$C w:0
  call: FORUI_$C w:1
  call: FORUI_$C w:32767
  call: FORUI_$C w:32768
  call: FORUI_$C w:65535
  call: FORUI_$RT w:0 w:65535
  call: FORUI_$RT w:1 w:0
  call: FORUI_$RT w:32767 w:32768
  call: FORUI_$RT w:32768 w:32767
  call: FORUI_$RT w:65535 w:65535
  call: FORUI_$RT w:0 w:0
  call: FORUI_$K w:0
  call: FORUI_$K w:1
  call: FORUI_$K w:32767
  call: FORUI_$K w:32768
  call: FORUI_$K w:65535
  call: FORUI_$I w:0
  call: FORUI_$I w:1
  call: FORUI_$I w:32767
  call: FORUI_$ARR w:0
  call: FORUI_$ARR w:1
  call: FORUI_$ARR w:8
  expect:
    0
    0
    1
    0
    0
    0
    1
    0
    0
    0
    1
    1
    2
    0
    1
    1
    2
    1
    0
    0
    32767
    32767
    32768
    32766
    32767
    32767
    32768
    536854528
    16383
    32766
    32768
    32768
    32769
    32767
    32768
    32768
    32769
    536887296
    16384
    32767
    65535
    65535
    65536
    65534
    65535
    65535
    65536
    2147450880
    32767
    65534
    0
    0
    0
    0
    1
    1
    1
    1
    32767
    32767
    32767
    32767
    32768
    32768
    32768
    32768
    65535
    65535
    65535
    65535
    0
    0
    0
    0
    0
    0
    1
    1
    1
    1
    1
    1
    1
    2
    32767
    32767
    32767
    32767
    32767
    32767
    32768
    32768
    32768
    32768
    32768
    32768
    32768
    32769
    65535
    65535
    65535
    65535
    65535
    65535
    65536
    65536
    65536
    0
    0
    2
    2
    0
    0
    1
    1
    1
    1
    40001
    40001
    40000
    40000
    7234
    7234
    7233
    7233
    0
    0
    0
    1
    1
    2
    32767
    32768
    0
    0
    3
    3
    108
    108
}
module forui;

type
  exec_$uword = 0..65535;

procedure exec_$logv_l(v: integer32); options(val_param); extern;
procedure exec_$clobber; extern;

type
  geom_t = record
    n: exec_$uword;
  end;
  gptr_t = ^geom_t;
  arr_t = array[1..2] of exec_$uword;

var
  cnt: integer32;
  bound: exec_$uword;
  ui: exec_$uword;
  ua: arr_t;

{ from 1 to n in a register, in the unit's data; from 0; to n - 1, from
  0 to n - 1, from 2; downto 1 and 0; the index summed; n div 2 }
procedure forui_$a(n: exec_$uword); options(val_param);
var
  i: exec_$uword;
begin
  cnt := 0;
  for i := 1 to n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  bound := n;
  cnt := 0;
  for i := 1 to bound do
    exec_$clobber;
  for i := 1 to bound do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 0 to n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 1 to n - 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 0 to n - 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := n downto 1 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := n downto 0 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 1 to n do
    cnt := cnt + i;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 1 to n div 2 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 2 to n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

procedure forui_touch(var x: exec_$uword); options(internal);
begin
  cnt := cnt + x - x;
end;

function forui_f(x: exec_$uword): exec_$uword; options(val_param, internal);
begin
  forui_f := x;
end;

procedure forui_var(var n: exec_$uword); options(internal);
var
  i: exec_$uword;
begin
  cnt := 0;
  for i := 1 to n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ through a pointer, under a `with' (a call in the body and not), a
  `var' parameter }
procedure forui_$b(in g: geom_t);
var
  i: exec_$uword;
  p: gptr_t;
  m: exec_$uword;
begin
  p := addr(g);
  cnt := 0;
  for i := 1 to p^.n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  with p^ do
    for i := 1 to n do begin
      exec_$clobber;
      cnt := cnt + 1;
    end;
  exec_$logv_l(cnt);
  cnt := 0;
  with p^ do
    for i := 1 to n do
      cnt := cnt + 1;
  exec_$logv_l(cnt);
  m := g.n;
  forui_var(m);
end;

{ a frame slot whose address is passed, an array element, a function
  result; a nested routine's bound and index; an index of the unit }
procedure forui_$c(n: exec_$uword); options(val_param);
var
  i: exec_$uword;
  m: exec_$uword;
  j: exec_$uword;

  procedure forui_up;
  var
    k: exec_$uword;
  begin
    cnt := 0;
    for k := 1 to m do
      cnt := cnt + 1;
    exec_$logv_l(cnt);
  end;

  { `<> 0', not `> 0': a pinteger against 0 takes the original
    compiler's signed condition, which counts 16#8000..16#FFFF as below
    0 (bead domain-pascal-wkm5; exec test top_bit_zero). }
  procedure forui_idx;
  begin
    if j <> 0 then
      cnt := cnt + 1;
  end;

begin
  m := n;
  forui_touch(m);
  cnt := 0;
  for i := 1 to m do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  ua[2] := n;
  cnt := 0;
  for i := 1 to ua[2] do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 1 to forui_f(n) do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  forui_up;
  cnt := 0;
  for j := 1 to n do
    forui_idx;
  exec_$logv_l(cnt);
  cnt := 0;
  for ui := 1 to n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for ui := 0 to n do
    exec_$clobber;
  for ui := 0 to n do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ both bounds at run time, up and down }
procedure forui_$rt(a, b: exec_$uword); options(val_param);
var
  i: exec_$uword;
begin
  cnt := 0;
  for i := a to b do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := b downto a do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ a run-time `from' to a constant, up and down }
procedure forui_$k(a: exec_$uword); options(val_param);
var
  i: exec_$uword;
begin
  cnt := 0;
  for i := a to 40000 do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 40000 downto a do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ an `integer' bound (non-negative here) }
procedure forui_$i(k: integer); options(val_param);
var
  i: exec_$uword;
begin
  cnt := 0;
  for i := 1 to k do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 1 to k do
    exec_$clobber;
  for i := 0 to k do
    cnt := cnt + 1;
  exec_$logv_l(cnt);
end;

{ arrays reached by the index: filled, then summed through a pointer }
procedure forui_$arr(n: exec_$uword); options(val_param);
type
  a_t = array[1..8] of integer;
  ap_t = ^a_t;
var
  i: exec_$uword;
  a: a_t;
  p: ap_t;
begin
  for i := 1 to 8 do
    a[i] := 0;
  for i := 1 to n do
    a[i] := i * 3;
  p := addr(a);
  cnt := 0;
  for i := 1 to n do
    cnt := cnt + p^[i];
  exec_$logv_l(cnt);
  cnt := 0;
  for i := 1 to 8 do
    cnt := cnt + a[i];
  exec_$logv_l(cnt);
end;
