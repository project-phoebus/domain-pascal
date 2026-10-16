{ Golden: codegen-patterns 5.2, "Block addresses", "Elements" (bead
  domain-pascal-dsr5): a record copied into an element of the unit's
  array indexed `.l' (the array's bounds times the element size do not
  fit in 16 bits; "Word indices used as `.l'"): the index's load, the
  source's `lea' in its stall (12.4 R11), the `ext.l' and the scaling,
  then the element's `lea (d,A5,Dx.l)' and an unrolled copy (KBD_$RCV
  0xE1CDD0..0xE1CDE6 `move.w (88,A5),D0; lea (28,A2),A0; ext.l D0; lsl.l
  #4,D0; lea (92,A5,D0.l),A1; move.l (A0)+,(A1)+' x4; here the record in
  A0, so A1 and A2). }
module kbdlike;
type
  sample_t = record a, b, c, d: integer32; end;
  kbd_t = record pad: array[1..14] of integer; s: sample_t; end;
var
  pad: array[1..44] of integer;
  head: integer;                          { 0x58 }
  pad2: integer;
  samples: array[0..4095] of sample_t;    { 0x5c }

procedure rcv(in out k: kbd_t);
begin
  samples[head] := k.s;
  k.pad[1] := 0;
end;
