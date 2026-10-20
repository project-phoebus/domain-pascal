{ Execution test (bead domain-pascal-5kgg, best effort): forms whose
  image form is not settled: a word `div'/`mod' of a sum used as a
  longword (the word quotient widened, codegen-patterns 8), the address
  of a field of another unit's record as a value (`lea abs.l', 6 sites,
  against `move.l #sym+d': not settled, codegen-patterns 1.5), `odd' of
  a word variable in memory as a boolean value (ref p. 4-128; `lsr.b #1;
  scs' at 0xE052FE: not settled; here loaded and `btst #0').

  link: addr_values_data.mod.pas
  warns: best effort (not byte-identical): a word `div`/`mod` of a sum
  warns: best effort (not byte-identical): the address of a constant or of a part of another unit's object
  warns: best effort (not byte-identical): `odd` of a variable in memory as a boolean value
  call: BE_MISC_$DIV w:100 w:-30 w:7
  call: BE_MISC_$DIV w:-100 w:-30 w:7
  call: BE_MISC_$ADDR l:77
  call: BE_MISC_$ODD w:7
  call: BE_MISC_$ODD w:-4
  call: BE_MISC_$ODD w:-3
  expect:
    10
    0
    -18
    -4
    77
    true
    false
    true
}
module be_misc;
%include 'exec.ins.pas';

type
  drv_t = record
    a: integer32;
    b: integer32;
    c: integer;
    d: integer32;
  end;
var
  addrv_$drv: extern drv_t;
  a, g, b, h, c: integer;
  l: integer32;
  p: ^integer32;
  bo: boolean;

procedure be_misc_$div(x, y, z: integer); options(val_param);
begin
  a := x;
  c := z;
  b := y;
  l := (a + b) div c;
  exec_$log_l(l);
  l := (a + b) mod c;
  exec_$log_l(l);
end;

procedure be_misc_$addr(v: integer32); options(val_param);
begin
  p := addr(addrv_$drv.d);
  p^ := v;
  exec_$log_l(addrv_$drv.d);
end;

procedure be_misc_$odd(x: integer); options(val_param);
begin
  g := x;
  bo := odd(g);
  exec_$log_bool(bo);
end;
