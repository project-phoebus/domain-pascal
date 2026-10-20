{ Golden: boolean values formed from conditions other than a comparison
  (bead domain-pascal-2lse; codegen-patterns 3, "Other conditions as
  boolean values").  `not' of a boolean variable, a field or a function's
  result: the value in a data register of its own, then `not.b' (0xE19610
  `move.b D5,D0; not.b D0; move.b D0,(118,A5)', a register-held variable;
  0xE0B616 `move.b (40,A4),D1; not.b D1', a field; 0xE12A26 `bsr; not.b
  D0', a function's result); of a plain `or': `or.b; not.b' (0xE0AED6);
  of a one-bit field: the opposite `s<cc>' (0xE725EC `btst #8,D2; seq
  D2').  Best effort (not byte-identical): `not' of a comparison as the
  opposite `s<cc>'; `and then' / `or else' as jumping code to `st Dn; bra;
  clr.b Dn' (the shape of PKT_$LIKELY_TO_ANSWER 0xE129F4..0xE12A18). }
module not_values;
type
  rec = record
    mode: integer;
    ready: boolean;
  end;
  rec_ptr = ^rec;
  bits = packed record
    pad: 0..127;
    cts: boolean;
  end;
var
  flag: boolean;
  other: boolean;
  bt: bits;
function busy(n: integer): boolean; options(val_param, extern);
procedure nots(p: rec_ptr; wait: boolean; n: integer); options(val_param);
begin
  flag := not wait;
  other := not p^.ready;
  flag := not busy(n);
  other := not (wait or p^.ready);
  flag := not bt.cts;
  other := not (n = 3);
end;
procedure shorts(p: rec_ptr; n: integer); options(val_param);
begin
  flag := (n = 0) and then (p^.mode = 4);
  other := (n > 2) or else p^.ready;
end;
