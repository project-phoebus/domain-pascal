{ Golden: codegen-patterns 5.1 "Set comparisons" and 1.5 "Set constants"
  (bead domain-pascal-2i6f).  `=' and `<>' of register-sized sets are the
  integer comparison of their values:
  - PRIO: `flags * [0, 1, 3] = [3]' through a pointer held in an address
    register, `moveq #11,D0; and.w (84,A0),D0; cmpi.w #8,D0; bne.s'
    (PROC1_$SET_PRIORITY 0xE152AC..0xE152B6, PROC1_$INIT 0xE2F9A8..
    0xE2F9B2);
  - TYPE4: `flags * [2, 15] = [2]' as a value, `cmpi.w #4,D0; seq D0'
    as at OS_$INIT 0xE33A0C..0xE33A10, but the intersection is NOT
    OS_$INIT's: OS_$INIT loads the set and `andi's it (0xE339EC, 0xE339F2),
    dpc loads the mask `moveq' cannot load first (`&''s majority form,
    codegen-patterns 8; not reproduced, not settled);
  - HELD: a set held in a register across a call, `cmpi.w #0x14,D2'
    (RING_$SENDP 0xE75C38 `cmpi.w #0x14,D7; bne.s');
  - SIO: two longword intersections, `moveq #1; moveq #1; and.l; and.l;
    cmp.l' (SIO_$K_SET_PARAM 0xE680F6..0xE68102); the image has the left
    operand in D1 (`moveq #1,D1; moveq #1,D0; ...; cmp.l D0,D1'), dpc in
    D0 (the integer 0xE49C2E's order): the register depends on the
    context, not reproduced here;
  and set constants passed by address are code-section cells of the set's
  size, the mask big-endian:
  - READ: TERM_$READ 0xE66864/0xE66870, cells 0xE66896 `0001' ([0]) and
    0xE66898 `0000' ([]); READ_COND names the first again (TERM_$READ_COND
    0xE668B0 `pea (-0x1c,PC)');
  - INQ: longword masks in the second group (TERM_$INQUIRE 0xE66FAE, cells
    0xE671AC `0000 0001' ..). }
module set_compares;
type
  w_t = set of 0..15;
  l_t = set of 0..31;
  pcb_t = record
    pad: array[1..42] of integer;
    flags: w_t;          { 84 }
  end;
  pcb_p = ^pcb_t;
  par_t = record
    flags1: l_t;
    flags2: l_t;
  end;
var
  cur: extern pcb_p;
  gw: extern integer;
procedure reorder(var p: pcb_t); extern;
procedure go; extern;
procedure k_get(in line: integer; in o: w_t; out s: integer32); extern;
procedure inq(in line: integer; in o: l_t; out s: integer32); extern;

procedure prio;
var p: pcb_p;
begin
  p := cur;
  if p^.flags * [0, 1, 3] = [3] then
    go;
end;

function type4(in f: w_t): boolean;
begin
  type4 := f * [2, 15] = [2];
end;

procedure held(var hw: w_t);
var h: w_t;
begin
  h := hw;
  go;
  if h = [2, 4] then
    go;
end;

procedure sio(in params: par_t; in changes: l_t);
var copy: par_t;
begin
  copy := params;
  if 5 in changes then
    if copy.flags1 * [0] <> params.flags1 * [0] then
      gw := 1;
end;

procedure read(in line: integer; out s: integer32);
begin
  if line = 0 then begin
    k_get(line, [0], s);
    gw := 1;
  end
  else
    k_get(line, [], s);
end;

procedure read_cond(in line: integer; out s: integer32);
begin
  k_get(line, [0], s);
end;

procedure inq_params(in line: integer; out s: integer32);
begin
  k_get(line, [], s);
  inq(line, [0], s);
  inq(line, [13], s);
end;
