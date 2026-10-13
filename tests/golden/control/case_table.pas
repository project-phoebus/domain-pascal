{ Golden: codegen-patterns 7.3 (`case', jump table), 1.1, 12.2.

  A `case' of 4 or more labels whose range r is at most 19/7 (2.71)
  times the number of labels n is a jump table (every table of 4 or more
  labels in compiled code has r/n <= 2.71, module-local 0xE63376 19/7):

    move.w <selector>,D0                OSINFO_$GET_MMAP 0xE5C6C0
    subq.w #lo,D0                       labels from lo (PROC1_$CREATE_P
                                        0xE15178 `subq.w #3'; ACL_$SERVER
                                        0xE495CE `subi.w #100'; MST_$PRIV_
                                        SET_TOUCH_AHEAD_CNT 0xE44534
                                        `addq.w #4' for -4)
    cmpi.w #r,D0; bcc <otherwise/join>  0xE5C6C4 (unsigned: below lo too)
    add.w D0,D0
    move.w (T,PC,D0.w),D0; jmp (T,PC,D0.w)   `303b 0006' / `4efb 0002'
  T: .short ARM-T ...                   0xE5C6D6, right after the `jmp'

  Arms follow the table in source order; each ends in a `bra' to the join
  but the last, which falls through (0xE5C77A..0xE5C798); an `otherwise'
  part comes last and falls through, the last arm branching over it
  (MAC_OS_$OPEN 0xE0B3C8..0xE0B3DC).  A label without an arm, and the
  `bcc', go to the `otherwise' part or the join (MAC_OS_$OPEN 0xE0B3AE:
  labels 1, 2); an empty arm goes to the join (KBD_$RCV 0xE1CCFE).  A
  `char' selector is widened with `clr.w; move.b' (ACL_$SERVER 0xE495C8).
  `osinfo' loads its by-address slot at entry (two uses, 12.2). }
module cst;
type
  info_t = record
    pad: array[1..19] of integer;
    op: integer;        { 0x26 }
    value: integer32;   { 0x28 }
    a: integer;         { 0x2c }
    b: integer;         { 0x2e }
  end;
var
  x1, x2, x3: extern integer;
  y1: extern integer32;
  k: extern integer;
  c: extern char;

procedure osinfo(var info: info_t);
begin
  case info.op of
    0: x3 := 7;
    1: y1 := info.value;
    2: x2 := 1;
    4: x1 := 0;
    3: x2 := 2;
  end;
end;

procedure create_p;
begin
  case k of
    3, 4, 5, 10: x1 := 5;
    8: x1 := 6;
    9: ;
    otherwise x1 := 0;
  end;
end;

procedure server;
begin
  case c of
    'd': x1 := 1;
    'f', 'h': x1 := 2;
    'l': x1 := 3;
    otherwise x2 := 0;
  end;
end;

procedure touch_ahead;
begin
  case k of
    -1: x1 := 1;
    -2: x1 := 2;
    -3: x1 := 3;
    -4: x1 := 4;
  end;
end;
