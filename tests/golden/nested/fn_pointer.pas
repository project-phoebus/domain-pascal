{ Golden: codegen-patterns 3 ("Pointer results", "Callers of pointer
  functions"), 6 (bead domain-pascal-5aw.18): a nested function returning
  a pointer in A0.

  - `cur := slot', a pointer read from the enclosing routine's local
    through the link: the memory form of a one-statement pointer
    function, through the result variable's slot below the reserved 4
    bytes, `move.l <src>,(-8,A6); movea.l (-8,A6),A0' (3: the module-local
    0xE6AA38, `link #-8'; SIO_$I_GET_DESC 0xE66808).  The link, `(A6)'
    (only the enclosing routine calls `cur', 6), is in A0 before that (6,
    "Which register": A0 without calls).
  - The caller dereferences the A0 the call returns, `move.w D2,(A0)' (3,
    "Callers of pointer functions": A0 as a base, `clr.w (A0)' 0xE4EA0E).
  - The enclosing routine's by-value parameter is held in D2 across the
    call (12.4 R3); its local `slot', which `cur' reaches, stays in the
    frame (6, "The parent's side"). }
module nfp;
type
  int_ptr = ^integer;

procedure nfp_$ptr(a: integer; tbl: int_ptr); options(val_param);
var
  slot: int_ptr;

  function cur: int_ptr;
  begin
    cur := slot;
  end;

begin
  slot := tbl;
  cur^ := a;
end;
