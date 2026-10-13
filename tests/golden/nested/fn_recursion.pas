{ Golden: codegen-patterns 6 ("Static link in A1"), 3 ("Result
  variable"), 1.1 (bead domain-pascal-5aw.18): recursion through a nested
  function.

  - `count' calls itself, so it takes the static link in A1 (6, "The
    rule"): it copies A1 to a callee-saved register at entry and passes
    that copy on, `movea.l A2,A1; bsr' (0xE397D0, 0xE39BC2, 0xE4F3BA,
    0xE50D5E pass their own copy of A1); the enclosing routine passes its
    frame, `movea.l A6,A1' (0xE13928).  The link is loaded after the
    entry load of the parameter (6, "Where the callee loads the link":
    0xE397D0, 0xE62CEA load a parameter, then copy A1).
  - The parameter and the link are both used after the recursive call, so
    both live in callee-saved registers, D2 and A2 (12.4 R3).
  - The result variable is D0 (no call follows an assignment whose value
    is still needed, 3): `move.w (-2,A2),D0', then the recursive call's D0
    plus the parameter and the local.
  - The recursive call is not popped: a join and the epilogue follow it
    (3, "Function calls in expressions", 0xE04700; 1.1). }
module nfr;

procedure nfr_$rec(n: integer); options(val_param);
var
  k: integer;

  function count(x: integer): integer;
  begin
    count := k;
    if x > 0 then
      count := count(x - 1) + x + k;
  end;

begin
  k := n;
  k := count(3);
end;
