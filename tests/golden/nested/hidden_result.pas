{ codegen-patterns 3 ("Record results over 4 bytes") and 6: a nested
  function whose 8-byte record result goes through a hidden address, and
  that takes no static link: the result temporary's address pushed last,
  at (8,A6) in the callee, the parameters after it, as at the top level
  (SAU7 FIM_FP's quiet_nan 0x3C46D3AA, called `move.b D5,-(SP); pea
  (-16,A2); pea (-20,A6); bsr.w'; ../source finding 1033). }
module nested_hidden;
type v = record hi, lo: integer32 end;
procedure t(var r: v);
  function q(in x: v; s: boolean): v;
  var w: v;
  begin w.lo := x.lo; w.hi := x.hi; q := w; end;
begin
  r := q(r, true);
end;
