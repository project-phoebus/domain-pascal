{ Execution test: edge shapes around bead domain-pascal-yzxr's integer
  locals held in D0 (codegen-patterns 3, "Pointer locals held in D0"),
  with `--no-park-pointer-locals'.  The descriptor function is declared
  with a 4-byte integer result in A0 (`a0_return', ref 5.5.11, p. 5-20),
  as in park_int.pas.  Each routine must compute what its Pascal says
  whatever register holds the local; the stubs clobber D0, D1, A0 and A1.

  - loop: parked, moved to An before the first statement, then
    dereferenced in a loop with a call in its body;
  - two: two parked locals, the first moved to a callee-saved register
    before the second call;
  - whole: an integer local read whole too (not parked: frame slot);
  - nested: read by a nested routine (uplevel: frame slot);
  - viad0: a type transfer of a D0 integer function's value to a pointer
    (`exec_$id_l', no `a0_return'): D0 to An, not the call's A0;
  - a0loop: a pointer local left in A0, dereferenced in a loop whose
    body loads other pointers (A1).

  dpc: --no-park-pointer-locals
  link: park_int_desc.mod.pas
  call: PARK_INT_EDGE_$LOOP l:1 @l:99!
  call: PARK_INT_EDGE_$TWO l:1 @l:99!
  call: PARK_INT_EDGE_$WHOLE l:1 @l:99!
  call: PARK_INT_EDGE_$NESTED l:1 @l:99!
  call: PARK_INT_EDGE_$VIAD0 l:1 @l:99!
  call: PARK_INT_EDGE_$A0LOOP l:1 @l:99!
  call: PARK_INT_EDGE_$SHOW
  expect:
    16
    out 0
    32
    out 0
    32
    out 0
    6
    out 0
    7
    out 0
    out 0
    128
}
module park_int_edge;
%include 'exec.ins.pas';

type
  desc_t = record
    a: integer32;
    b: integer32;
  end;
  desc_ptr_t = ^desc_t;
var
  g: integer32 := 0;
  h: desc_t;
  hp: desc_ptr_t := addr(h);

function park_int_$getd(in k: integer32; out st: integer32): integer32;
  options(extern, a0_return);

procedure park_int_edge_$loop(k: integer32; out st: integer32);
  options(val_param);
var
  d: integer32;
  i: integer;
begin
  d := park_int_$getd(k, st);
  desc_ptr_t(d)^.b := 10;
  for i := 1 to 3 do begin
    exec_$clobber;
    desc_ptr_t(d)^.b := desc_ptr_t(d)^.b + i;
  end;
  exec_$logv_l(desc_ptr_t(d)^.b);
end;

procedure park_int_edge_$two(k: integer32; out st: integer32);
  options(val_param);
var
  d, e: integer32;
begin
  d := park_int_$getd(k, st);
  e := park_int_$getd(k + 1, st);
  desc_ptr_t(d)^.b := desc_ptr_t(e)^.b + desc_ptr_t(d)^.b;
  exec_$logv_l(desc_ptr_t(d)^.b);
end;

procedure park_int_edge_$whole(k: integer32; out st: integer32);
  options(val_param);
var
  d: integer32;
begin
  d := park_int_$getd(k, st);
  if d = 0 then
    return;
  exec_$logv_l(desc_ptr_t(d)^.b);
end;

procedure park_int_edge_$nested(k: integer32; out st: integer32);
  options(val_param);
var
  d: integer32;

  procedure show;
  begin
    exec_$logv_l(desc_ptr_t(d)^.a - 1);
  end;

begin
  d := park_int_$getd(k, st);
  if st <> 0 then
    return;
  show;
end;

procedure park_int_edge_$viad0(k: integer32; out st: integer32);
  options(val_param);
var
  t: integer32;
  q: desc_ptr_t;
begin
  t := park_int_$getd(k, st);
  exec_$clobber;
  q := desc_ptr_t(exec_$id_l(t));
  exec_$clobber;
  exec_$logv_l(q^.a);
end;

procedure park_int_edge_$a0loop(k: integer32; out st: integer32);
  options(val_param);
var
  q: desc_ptr_t;
  i: integer;
begin
  q := desc_ptr_t(park_int_$getd(k, st));
  g := 0;
  for i := 1 to 4 do begin
    g := g + q^.b;
    hp^.a := g;
  end;
end;

procedure park_int_edge_$show;
begin
  exec_$logv_l(g);
end;
