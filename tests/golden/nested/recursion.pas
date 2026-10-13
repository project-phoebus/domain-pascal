{ Golden: codegen-patterns 6, nested routines (bead domain-pascal-5aw.3).

  - A nested routine that calls itself passes its own static link
    (0xE397D0, 0xE39BC2, 0xE4F3BA, 0xE50D5E: recursive, so A1 in every
    call, `movea.l An,A1' with An copied from A1 at entry); the enclosing
    routine passes its frame (`movea.l A6,A1').
  - A nested function called only by its enclosing routine reads the
    enclosing local through the dynamic link `(A6)' (0xE13616); its frame
    has the link slot before its result variable.
  - Nested routines inherit A5: the enclosing exported routine loads it
    because `rec' uses the unit's data (codegen-patterns 4; 0xE17778
    reads `(0x14,A5)' for LOG_$READ). }
module nb;
var
  total: integer32;
  cnt: integer;
procedure ext_$p(v: integer32); options(val_param); extern;

procedure nb_$walk(depth: integer); options(val_param);
var
  k: integer;

  procedure rec;
  begin
    total := total + 1;
    k := k - 1;
    if k > 0 then
      rec;
    ext_$p(total);
    k := 0;
  end;

  function twice(x: integer): integer;
  begin
    twice := x + k;
  end;

begin
  k := depth;
  rec;
  cnt := twice(3);
end;
