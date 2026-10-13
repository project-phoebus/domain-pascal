{ Golden: codegen-patterns 6 (static link), 3 (function results), 12.4
  (bead domain-pascal-5aw.18): a nested *function* called by a sibling
  nested routine and by its enclosing routine.

  - `get' is called by its sibling `use' as well as by the enclosing
    routine, so it takes the static link in A1 (6, "The rule", 86 of 86):
    the sibling passes the link it holds, `movea.l An,A1; bsr' (0xE13718
    `movea.l A4,A1; bsr.w 0xE1359C'), the enclosing routine its own frame,
    `movea.l A6,A1; bsr' (0xE13928).  `use' is called by the enclosing
    routine only: it takes the dynamic link `(A6)' (0xE13616) and its
    caller loads no A1 (0xE13856).
  - In `get' the link lives across the call, so it is copied to a
    callee-saved register at entry, `movea.l A1,A2' (0xE135A4), and the
    enclosing local is read through it after the call, `add.w (d,A2)'
    (0xE1362E reads `(-0x50,A4)').
  - The result of a body of one statement is computed in a scratch
    register and moved to D0 (3, "Result assignment as the last
    statement", PROC1_$TST_LOCK 0xE148E0); the call's D0 is copied before
    the arithmetic (3, "Function calls in expressions", 0xE01BA2..0xE01BA4).
  - Nested frames: the 4-byte link slot, then the function's result
    variable (`link #-8`: 6 "Frame", 2 "Frame size": the 11 nested
    functions all have `link #-8` or more); `use', a procedure with no
    locals, `link #-4' (0xE1359C).
  - The enclosing routine keeps `k' and `r', which its nested routines
    reach, in its frame (6, "The parent's side"), and pushes the stored
    `r' from the D0 that still holds it (12.5 "Stored locals pushed from the register", PKT_$NEXT_ID
    0xE124A8, 0xE124C4).
  - The call whose path to the epilogue has no push, call or branch is
    not popped (1.1; `calls/pop_branches'). }
module nfs;
procedure ext_$p(v: integer); options(val_param); extern;
function ext_$f(v: integer): integer; options(val_param); extern;

procedure nfs_$sib(a: integer); options(val_param);
var
  k: integer;
  r: integer;

  function get(x: integer): integer;
  begin
    get := ext_$f(x) + k;
  end;

  procedure use;
  begin
    r := get(2);
  end;

begin
  k := a;
  use;
  r := get(5);
  ext_$p(r);
end;
