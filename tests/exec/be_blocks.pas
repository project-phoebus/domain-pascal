{ Execution test: best-effort paths (bead domain-pascal-5kgg) of record
  copies whose addresses the image's forms do not settle: the block's
  address formed as `ea_of' gives it and taken with `lea' (codegen-patterns
  5.2, "Block addresses", "6 and 8 bytes"; beads domain-pascal-dsr5,
  -zrug).  8- and 12-byte copies from an object through a pointer loaded
  through a pointer; an 8-byte copy into an element indexed at run time;
  8- and 12-byte copies from fields of another unit's variable; 8- and
  12-byte copies from elements of another unit's arrays; a 12-byte copy
  between two elements indexed at run time; a 12-byte copy from an
  element at a computed index; an unrolled copy from an element of the
  unit's array; an 8-byte copy from the unit's array into a field of an
  object reached through a by-address pointer.

  link: be_blocks_data.mod.pas
  call: BB_$FILL
  call: BB_$VIA_T
  call: BB_$INTOIDX_T w:3
  call: BB_$FIELD_T
  call: BB_$ELEM_T w:2
  call: BB_$BOTH_T w:1 w:4
  call: BB_$COMPUTED_T w:2
  call: BB_$SLOT_T w:3
  warns: best effort (not byte-identical): an 8-byte copy from a pointer not held in an address register
  warns: best effort (not byte-identical): a block through a pointer that is not held in a register
  warns: best effort (not byte-identical): an 8-byte copy into this kind of destination
  warns: best effort (not byte-identical): an 8-byte copy from a field of another unit's variable
  warns: best effort (not byte-identical): a block at an offset into another unit's object
  warns: best effort (not byte-identical): a block that is an element of another unit's array
  warns: best effort (not byte-identical): an 8-byte copy from an element of another unit's array
  warns: best effort (not byte-identical): a block copy or comparison of two array elements indexed at run time
  warns: best effort (not byte-identical): a block that is an array element whose index is computed
  warns: best effort (not byte-identical): an unrolled block copy (up to 18 bytes) from an element
  warns: best effort (not byte-identical): an 8-byte copy from an array element indexed at run time into a destination other than
  expect:
    1111
    2222
    3131
    3232
    3333
    7777
    8888
    1001
    1002
    1003
    1004
    1005
    201
    202
    211
    212
    213
    411
    412
    413
    311
    312
    313
    411
    412
    413
    -1000
    7777
    8888
    -2000
}
module be_blocks;
%include 'exec.ins.pas';

type
  rec = record a, b: integer32; end;
  recp = ^rec;
  r12 = record a, b, c: integer32; end;
  r12p = ^r12;
  holder = record pad: integer32; p: recp; q: r12p; end;
  holderp = ^holder;
  outer = record x: integer32; r: rec; t: r12; end;
  r20 = record a: integer32; b: rec; c: integer32; d: integer32; end;
  r20p = ^r20;

var
  bb_$orec: extern outer;
  bb_$oarr: extern array[1..4] of rec;
  bb_$oarr12: extern array[1..4] of r12;
  marr: array[1..4] of rec;
  m12: array[1..4] of r12;

procedure bb_$via(in h: holderp; out x: rec);
begin
  x := h^.p^;
end;

procedure bb_$via12(in h: holderp; out x: r12);
begin
  x := h^.q^;
end;

procedure bb_$via_t;
var r: rec; t: r12; hold: holder; x: rec; y: r12;
begin
  r.a := 1111;
  r.b := 2222;
  t.a := 3131;
  t.b := 3232;
  t.c := 3333;
  hold.p := addr(r);
  hold.q := addr(t);
  bb_$via(addr(hold), x);
  exec_$log_l(x.a);
  exec_$log_l(x.b);
  bb_$via12(addr(hold), y);
  exec_$log_l(y.a);
  exec_$log_l(y.b);
  exec_$log_l(y.c);
end;

procedure bb_$intoidx(in i: integer; in x: rec);
begin
  marr[i] := x;
end;

procedure bb_$intoidx_t(i: integer); options(val_param);
var x: rec;
begin
  x.a := 7777;
  x.b := 8888;
  bb_$intoidx(i, x);
  exec_$log_l(marr[i].a);
  exec_$log_l(marr[i].b);
end;

procedure bb_$field(out x: rec);
begin
  x := bb_$orec.r;
end;

procedure bb_$field12(out x: r12);
begin
  x := bb_$orec.t;
end;

procedure bb_$field_t;
var x: rec; y: r12;
begin
  bb_$field(x);
  exec_$log_l(x.a);
  exec_$log_l(x.b);
  bb_$field12(y);
  exec_$log_l(y.a);
  exec_$log_l(y.b);
  exec_$log_l(y.c);
end;

procedure bb_$elem8(in i: integer; out x: rec);
begin
  x := bb_$oarr[i];
end;

procedure bb_$elem12(in i: integer; out x: r12);
begin
  x := bb_$oarr12[i];
end;

procedure bb_$elem_t(i: integer); options(val_param);
var x: rec; y: r12;
begin
  bb_$elem8(i, x);
  exec_$log_l(x.a);
  exec_$log_l(x.b);
  bb_$elem12(i, y);
  exec_$log_l(y.a);
  exec_$log_l(y.b);
  exec_$log_l(y.c);
end;

{ m12[i] := bb_$oarr12[j] }
procedure bb_$both(in i, j: integer);
begin
  m12[i] := bb_$oarr12[j];
end;

procedure bb_$both_t(i, j: integer); options(val_param);
begin
  bb_$both(i, j);
  exec_$log_l(m12[i].a);
  exec_$log_l(m12[i].b);
  exec_$log_l(m12[i].c);
end;

procedure bb_$computed(in i: integer; out x: r12);
begin
  x := bb_$oarr12[i + i - 1];   { computed: i + i, the 1 in the displacement }
end;

{ an unrolled (12-byte) copy from an element of the unit's array }
procedure bb_$unrolled(i: integer; out x: r12); options(val_param);
begin
  x := m12[i];
end;

procedure bb_$computed_t(i: integer); options(val_param);
var y, z: r12;
begin
  bb_$computed(i, y);
  exec_$log_l(y.a);
  exec_$log_l(y.b);
  exec_$log_l(y.c);
  bb_$unrolled(1, z);
  exec_$log_l(z.a);
  exec_$log_l(z.b);
  exec_$log_l(z.c);
end;

{ p^.b := marr[i], p by address }
procedure bb_$slot(in i: integer; in p: r20p);
begin
  p^.b := marr[i];
end;

procedure bb_$slot_t(i: integer); options(val_param);
var r: r20;
begin
  r.a := -1000;
  r.b.a := 5000;
  r.b.b := 6000;
  r.c := -2000;
  bb_$slot(i, addr(r));
  exec_$log_l(r.a);
  exec_$log_l(r.b.a);
  exec_$log_l(r.b.b);
  exec_$log_l(r.c);
end;
