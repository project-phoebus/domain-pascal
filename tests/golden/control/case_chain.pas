{ Golden: codegen-patterns 7.3 (`case', compare chain).

  A `case' of 2 or 3 labels, however dense, and one of 4 or more labels
  over a range of at least 13/3 (4.33) times their number, is a chain of
  compares in source label order, then a `bra' to the `otherwise' part
  or the join:

    move.w (4,A0),D0; beq.s L0          SYSBUS_$INIT 0xE0AB64: label 0
    cmpi.w #1,D0; beq.s L1              first uses the flags of the load
    cmpi.w #2,D0; beq.s L2
    bra.s JOIN                          0xE0AB76

  Label 0 elsewhere is `tst.w Dn; beq' (TIME_$GET_EC 0xE16762), and
  first after a `char' widening too (0xE556E6 `clr.w D0; move.b
  (39,A0),D0; tst.w D0').  A sparse `char' case: module-local 0xE1CC64
  (labels 0xC0, 0x91, ... in source order, three labels for one arm,
  `bra' to the `otherwise' part, which comes after the arms).  A held
  selector is copied first (TTY_$I_SIGNAL 0xE1B830 `move.w D0,D1').  A
  longword selector is compared with `cmpi.l' (0xE5D2FE).  Arms as for
  the jump table (case_table.pas). }
module cch;
type
  dev_t = record
    pad: array[1..2] of integer;
    kind: integer;      { 4 }
  end;
var
  x1, x2, x3: extern integer;
  st: extern integer32;
  k: extern integer;
  c: extern char;
  dev: extern ^dev_t;

procedure sysbus;
begin
  case dev^.kind of
    0: x1 := 1;
    1: x2 := 2;
    2: x3 := 3;
  end;
end;

procedure get_ec;
begin
  case k of
    1: x1 := 1;
    0: x1 := 2;
    otherwise st := 16#d0005;
  end;
end;

procedure key(var res: integer);
begin
  case c of
    chr(16#c0): res := 22;
    chr(16#91): res := 24;
    chr(16#92): res := 4;
    chr(16#95): res := 8;
    chr(16#96): res := 13;
    chr(16#99), chr(16#98), chr(16#97): res := 9;
    otherwise res := 0;
  end;
end;

procedure status_chain;
begin
  case st of
    16#f0010: x1 := 1;
    16#f0003: x1 := 2;
    16#f0011: x1 := 3;
  end;
end;

procedure signal(sig: integer); internal;
begin
  x1 := sig;
  case sig of
    3: x2 := 0;
    2: x2 := 1;
    21: x2 := 2;
    1: x2 := 3;
    26: x2 := 4;
  end;
end;
