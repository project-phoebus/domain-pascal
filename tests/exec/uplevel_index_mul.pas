{ Execution test (bead domain-pascal-dctl): an array indexed by an
  enclosing routine's local, scaled by a constant of 1..127 done with a
  multiply (codegen-patterns 8, "Multiplication by a constant"; 5.2,
  "Scalings by element size"): the constant first, the local read
  through the static link, `moveq #46,D0; mulu.w (-d,A2),D0' (sau5
  MT_$ACQUIRE's nested routine 0xE75C0C..0xE75C22, A2 the parent's
  frame from `movea.l (A6),A2').  A `.w' index into another unit's
  array and into the unit's own (`mulu.w'), and a `.l' index (an array
  whose scaled bounds pass 32767) of a signed local (`muls.w', its
  32-bit product, negative indices too).  The nested routine sets an
  element's first and last words to values made from the index; its
  neighbours keep theirs, so a wrong scale shows.

  link: uplevel_index_mul_data.mod.pas
  call: UI_$RUN
  expect:
    0
    0
    1101
    1102
    0
    0
    2201
    2202
    0
    0
    0
    -1
    3302
    0
    0
    4403
    -1
}
module uplevel_index_mul;
%include 'exec.ins.pas';

type
  u46 = record a: integer; pad: array[1..21] of integer; z: integer; end;
var
  ui_$units: extern array[0..3] of u46;
  ui_$big: extern array[-800..800] of u46;
  own: array[0..3] of u46;

procedure set_other(in i: integer);
var
  u: integer;
  procedure init_unit;
  begin
    with ui_$units[u] do begin
      a := 1100 + u;
      z := 1100 + u + 1;
    end;
  end;
begin
  u := i;
  init_unit;
end;

procedure set_own(in i: integer);
var
  u: integer;
  procedure init_own;
  begin
    with own[u] do begin
      a := 2200 + u;
      z := 2200 + u + 1;
    end;
  end;
begin
  u := i;
  init_own;
end;

procedure set_big(in i: integer; in v: integer);
var
  u: integer;
  procedure init_big;
  begin
    ui_$big[u].z := v;
  end;
begin
  u := i;
  init_big;
end;

function get_big(in i: integer): integer;
var
  u: integer;
  function get: integer;
  begin
    get := ui_$big[u].z;
  end;
begin
  u := i;
  get_big := get;
end;

procedure ui_$run;
var
  i: integer;
begin
  for i := 0 to 3 do begin
    ui_$units[i].a := 0;
    ui_$units[i].z := 0;
    own[i].a := 0;
    own[i].z := 0;
  end;
  set_other(1);
  set_own(1);
  exec_$log_w(ui_$units[0].z);
  exec_$log_w(ui_$units[1].pad[1]);
  exec_$log_w(ui_$units[1].a);
  exec_$log_w(ui_$units[1].z);
  exec_$log_w(ui_$units[2].a);
  exec_$log_w(own[0].z);
  exec_$log_w(own[1].a);
  exec_$log_w(own[1].z);
  exec_$log_w(own[2].a);
  ui_$big[-800].z := 0;
  ui_$big[-799].z := 0;
  ui_$big[-798].a := -1;
  ui_$big[799].a := -1;
  ui_$big[800].z := 0;
  set_big(-799, 3302);
  set_big(799, 4403);
  exec_$log_w(ui_$big[-800].z);
  exec_$log_w(ui_$big[-798].pad[21]);
  exec_$log_w(ui_$big[-798].a);
  exec_$log_w(get_big(-799));
  exec_$log_w(ui_$big[799].pad[21]);
  exec_$log_w(ui_$big[800].z);
  exec_$log_w(get_big(799));
  exec_$log_w(ui_$big[799].a);
end;
