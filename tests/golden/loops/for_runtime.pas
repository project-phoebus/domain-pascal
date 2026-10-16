{ Golden: codegen-patterns 7.4 ("Run-time bounds"), `for' loops from 0
  or 1 to a run-time upper bound (bead domain-pascal-jnsb).
  - From 0 to a variable: the bound tested with `tst.w', then copied into
    the `dbf' count, into the bound's own register where the bound dies
    there and no call follows (XNS_IDP_$REGISTER_ADDR 0xE19020..0xE1902A
    `move.w (1336,A5),D0; tst.w D0; bmi.s; move.w D0,D0; clr.w D1'; the
    module-local 0xE17EBA..0xE17EC2), into a callee-saved register across
    a call (DISK_$AS_XFER_MULTI 0xE6BB26 `tst.w D5; bmi.s; move.w D5,D3';
    AST_$LOAD_AOTE 0xE029A8).  The bound here is of the unit's data, as
    at both sites that test a bound in memory.
  - From 0 to `e - 1', a call in the body: `subq.w #1' on e's value, the
    guard on its flags, the count copied, then the index `clr.w'
    (PEB_$LOAD_WCS 0xE3214A..0xE32154 `move.w (2,A2),D0; subq.w #1,D0;
    bmi.s; move.w D0,D2; clr.w D3'; MAC_OS_$OPEN 0xE0B2F8; MST_$FORK
    0xE73A3E; MST_$INIT 0xE30C2C).
  - The constant of `e - c' folded into the count's one subtraction
    (DISK_$AS_XFER_MULTI 0xE6BA42 `subq.w #2', a call in the body;
    DIR_$FIND_NET 0xE4EAAE `subq.w #2' from 1). }
module flr;
var
  tab: array[0..7] of char;
  count: integer;
  w: extern integer;
  len: extern integer;

procedure ext_$put(x: integer); options(val_param, extern);

procedure flr_$find; internal;
var
  i: integer;
begin
  for i := 0 to count do
    if tab[i] = 'x' then
      w := i;
end;

procedure flr_$all; internal;
var
  i: integer;
begin
  for i := 0 to count do
    ext_$put(i);
end;

procedure flr_$init; internal;
var
  i: integer;
begin
  for i := 0 to len - 1 do
    ext_$put(i);
end;

procedure flr_$tail; internal;
var
  i: integer;
begin
  for i := 1 to len - 1 do
    ext_$put(i);
end;
