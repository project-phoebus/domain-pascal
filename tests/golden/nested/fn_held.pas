{ Golden: codegen-patterns 3 ("Result variable"), 6, 12.4 R3 (bead
  domain-pascal-5aw.18): a nested function whose result is held across a
  call.

  - `get := ext_$f(x)' and then a call: the result variable is held in
    the lowest free callee-saved register, D2, `move.w D0,D2' after the
    first call, and moved to D0 at the start of the epilogue, `move.w
    D2,D0' (3: NETBUF_$RTNVA 0xE0ED4E `move.l D0,D2', PKT_$NEXT_ID
    0xE124D0 `move.w D2,D0'; 66 sites of `list reg').
  - The static link lives across the first call too: the dynamic link
    `(A6)' (only the enclosing routine calls `get') loaded into A2 after
    the save (6, "Which register": A2 in 44 routines with calls; 0xE1360C
    `movea.l (A6),A4', 0xE4120C `movea.l (A6),A2'), both saved with one
    `movem.l' (2, "Register save").
  - The enclosing routine stores the call's D0 into its local (3,
    0xE04700 `move.l D0,(A3)'), unpopped because nothing follows on the
    way to the epilogue (1.1). }
module nfh;
procedure ext_$p(v: integer); options(val_param); extern;
function ext_$f(v: integer): integer; options(val_param); extern;

procedure nfh_$held(a: integer); options(val_param);
var
  k: integer;

  function get(x: integer): integer;
  begin
    get := ext_$f(x);
    ext_$p(k);
  end;

begin
  k := a;
  k := get(3);
end;
