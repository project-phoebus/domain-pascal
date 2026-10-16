{ Golden: codegen-patterns 8, "Array elements" (`x[i] := x[i] op c';
  bead domain-pascal-9pkq).
  - a word of a word array of the unit's data indexed by another unit's
    word variable, at a displacement within a byte: a load, `addq.w' or
    `subq.w' on a data register, a store through the same operand
    (AUDIT_$CONTROL 0xE715CC..0xE715DA `move.w (-2,A5,D0.w),D1; addq.w
    #1,D1; move.w D1,(-2,A5,D0.w)'; AUDIT_$LOG_EVENT 0xE710B0 `subq.w');
  - the same beyond a byte: `lea (0,A5,D0.w),A0', then `addq.w'/`subq.w'
    to the element in place (ACL_$ENTER_SUPER 0xE46FA4..0xE46FA8 `addq.w
    #1,(0xb76,A0)'; ACL_$EXIT_SUPER 0xE46FE8; ACL_$SET_RE_ALL_SIDS
    0xE48430 `addq.w', 0xE48458 `subq.w');
  - a field of an element of another unit's array, within a byte: in
    place, the base between the index's value and its scaling (PMAP_
    0xE13146..0xE13154 `move.w PROC1_$CURRENT,D0; movea.l
    #PROC1_$STATS,A0; lsl.w #4,D0; addq.l #1,(-4,A0,D0.w)'); a byte
    element (AST_ 0xE00E2C `subq.b #1,(-4,A0,D0.l)', there with a `.l'
    index). }
module element_update;
type
  sbyte = [byte] -128..127;
  stats_t = record
    a: integer32;
    b: integer32;
    c: integer32;
    d: integer32;
  end;
var
  counts: array[1..64] of integer;         { +0x000 }
  pad: array[1..1000] of integer;          { +0x080 }
  far_counts: array[1..64] of integer;     { +0x850 }
  proc1_$current: extern integer;
  proc1_$stats: extern array[1..64] of stats_t;
  bvals: extern array[1..64] of sbyte;

procedure near_up; internal;
begin
  counts[proc1_$current] := counts[proc1_$current] + 1;
end;

procedure near_down; internal;
begin
  counts[proc1_$current] := counts[proc1_$current] - 1;
end;

procedure far_up; internal;
begin
  far_counts[proc1_$current] := far_counts[proc1_$current] + 1;
end;

procedure far_down; internal;
begin
  far_counts[proc1_$current] := far_counts[proc1_$current] - 1;
end;

procedure other_unit_field; internal;
begin
  proc1_$stats[proc1_$current].d := proc1_$stats[proc1_$current].d + 1;
end;

procedure other_unit_byte; internal;
begin
  bvals[proc1_$current] := bvals[proc1_$current] - 1;
end;
