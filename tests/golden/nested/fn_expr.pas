{ Golden: codegen-patterns 3 ("Function calls in expressions"), 6 (bead
  domain-pascal-5aw.18): a nested function called by its enclosing
  routine inside expressions.

  - `get' is called only by its enclosing routine: the dynamic link
    `(A6)' is its static link (6, 0xE13616), its caller loads no A1
    (0xE13856).  It makes no call, so the link goes to A0 (6, "Which
    register": A0 in 8 routines without calls).
  - Its result, D0, in the enclosing routine's arithmetic: popped, copied
    before the arithmetic, `move.w D0,D1; addq.w #3,D1' (3, 0xE01BA2..
    0xE01BA4 `addq.w #8,SP; move.w D0,D1; lsl.w #2,D1'), stored to the
    local the nested function reaches, which stays in the frame (6, "The
    parent's side");
  - and in a comparison: popped, then compared in place, `cmpi.w #3,D0;
    ble' (3, 0xE0082C `addq.w #8,SP; tst.b D0; bmi'; 7.1). }
module nfe;
procedure ext_$p(v: integer); options(val_param); extern;

procedure nfe_$expr(a: integer); options(val_param);
var
  k: integer;

  function get(x: integer): integer;
  begin
    get := x + k;
  end;

begin
  k := a;
  k := get(2) + 3;
  if get(1) > 3 then
    ext_$p(k);
end;
