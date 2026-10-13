{ Golden: codegen-patterns 6 ("Uplevel accesses"), 3 (bead
  domain-pascal-5aw.18): a nested function reading its enclosing
  routine's by-value parameter, by-address parameter and local.

  - Through the link `(A6)' in A0 (6; no call): the by-value parameter as
    `(8,A0)' and the local as `(-2,A0)' (0xE135AA `btst #0,(0x15,A2)';
    0xE1362E `(-0x50,A4)'); the by-address parameter's slot read through
    the link, then dereferenced, `movea.l (0xa,A0),A1; add.w (4,A1)'
    (0xE135DA `movea.l (8,A2),A0; move.w (0xc,A0),-(SP)').
  - The sum is built in D0, the result register (3, "Result assignment
    as the last statement"), its own parameter first; the slot load of
    `r' is moved ahead of the first `add' by the scheduler (12.4 R11).
  - The enclosing routine keeps the local in its frame (6, "The parent's
    side"); its by-address `cnt', used twice, is loaded at entry (12.2)
    and held in A2 across the call (12.4 R3). }
module nfu;
type
  rec_t = record
    a: integer32;
    n: integer;
  end;

procedure nfu_$up(v: integer; var r: rec_t; var cnt: integer); options(val_param);
var
  k: integer;

  function sum(x: integer): integer;
  begin
    sum := x + v + r.n + k;
  end;

begin
  k := cnt;
  cnt := sum(1);
end;
