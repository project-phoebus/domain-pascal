{ Golden: `in_range' of an enumeration in a condition (ref "In_range",
  p. 4-104; codegen-patterns 10.1; bead domain-pascal-k1sv): one unsigned
  compare against the last ordinal, `cmpi.w #31; bhi', then the guard's
  second compare `cmpi.w #17; bls', as TTY_$K_SET_FUNC_CHAR 0xE67502 and
  TTY_$K_INQ_FUNC_CHAR 0xE67588 (../source tty/k_func.pas: `not
  in_range(func) or else (func > tty_$func_reserved_18)'); as a value,
  `s<cc>' of the same compare (`sls'). }
module in_range;
type
  func_t = (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11, f12, f13, f14,
            f15, f16, f17, f18, f19, f20, f21, f22, f23, f24, f25, f26, f27,
            f28, f29, f30, f31);
var
  slots: array[0..17] of char;
  ok: boolean;
procedure set_slot(in func: func_t; in ch: char; out st: integer32);
begin
  if not in_range(func) or else (func > f17) then begin
    st := 16#350002;
    return;
  end;
  st := 0;
end;
procedure check(in func: func_t);
begin
  ok := in_range(func);
end;
