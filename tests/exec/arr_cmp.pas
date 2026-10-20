{ Execution test (bead domain-pascal-yqf6): `=` and `<>` of arrays (ref
  Table 4-1, p. 4-2: "All types"), compared in memory as records are
  (codegen-patterns 7.1, "Records"): two arrays of 8 longwords, a `cmpm.l`
  loop (DISK_IO 0xE3D846 `moveq #7,D1; cmpm.l (A1)+,(A0)+; dbne`), and an
  array of `char` against a string constant of its length, a `cmpm.b`
  loop from the constant's cell (the module-local DISK_ routine at
  0xE6C266 `lea (0x44,PC),A3; moveq #5,D2; cmpm.b (A3)+,(A1)+; dbne`),
  as a condition and as a value, with the constant on either side.

  call: ARR_CMP_$LONGS w:3 l:5
  call: ARR_CMP_$LONGS w:7 l:7
  call: ARR_CMP_$NAME w:1 b:65
  call: ARR_CMP_$NAME w:6 b:88
  call: ARR_CMP_$NAME w:2 b:80
  expect:
    false
    mark 1
    true
    mark 0
    true
    mark 0
    false
    mark 1
    true
    mark 1
}
module arr_cmp;
%include 'exec.ins.pas';

type
  lw8 = array[0..7] of integer32;
  hdr_t = record
    a: integer32;
    l: lw8;
  end;
  name6 = array[1..6] of char;
  lbl_t = record
    version: integer;
    apollo: name6;
    n: integer;
  end;
  lbl_ptr_t = ^lbl_t;
var
  x, y: hdr_t;
  lb: lbl_t;
  lp: lbl_ptr_t;

procedure arr_cmp_$longs(i: integer; v: integer32); options(val_param);
var
  k: integer;
begin
  k := 0;
  while k < 8 do begin
    x.l[k] := k;
    y.l[k] := k;
    k := k + 1;
  end;
  y.l[i] := v;
  exec_$log_bool(x.l = y.l);
  if x.l <> y.l then
    exec_$mark(1)
  else
    exec_$mark(0);
end;

procedure arr_cmp_$name(i: integer; c: char); options(val_param);
begin
  lb.version := i;
  lb.apollo[1] := 'A';
  lb.apollo[2] := 'P';
  lb.apollo[3] := 'O';
  lb.apollo[4] := 'L';
  lb.apollo[5] := 'L';
  lb.apollo[6] := 'O';
  lb.apollo[i] := c;
  lp := addr(lb);
  exec_$log_bool('APOLLO' = lp^.apollo);
  with lp^ do
    if (version > 1) or (apollo <> 'APOLLO') then
      exec_$mark(1)
    else
      exec_$mark(0);
end;
