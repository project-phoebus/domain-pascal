{ Golden: codegen-patterns 7.3 ("Which form"); ../source finding 950.  A
  `case' of 5 labels over a range of 17 (r/n 3.4) is a compare chain, in
  source order: SAU7 WIN7_$ERROR_QUE 0x3C41C30C `cmpi.w #2; beq; cmpi.w
  #3; beq; cmpi.w #17; beq; cmpi.w #18; beq; cmpi.w #16; beq; bra', the
  densest chain seen (tables reach r/n 2.71).  The image compares a frame
  copy of the selector, `cmpi.w #2,(-52,A6)'; dpc compares a register
  (not settled, bead domain-pascal-15zp). }
module case_chain_dense;
var q: extern integer;
procedure error_que(x: integer); options(val_param);
begin
  case x of
    2: q := 1;
    3: q := 2;
    17, 18, 16: q := 3;
  end;
end;
