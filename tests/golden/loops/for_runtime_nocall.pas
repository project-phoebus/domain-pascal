{ Golden: codegen-patterns 7.4 ("The count's register"), `for' loops
  from 0 or 1 to a run-time upper bound with no call in the body (bead
  domain-pascal-fbah; tools/loopalign.py counters).  The counter takes
  the lowest data register free over the loop, the count's dying
  register counting as free; the count is copied where that register
  lies below the count's, which a bound variable dying at the bound
  forces (R2 keeps the count out of its register).
  - From 0 to `count - 1', the bound a value parameter loaded into a
    register: the MAC_OS_ module-local 0xE0B202 at 0xE0B20C..0xE0B218
    `move.w (16,A6),D1; move.w D1,D2; subq.w #1,D2; bmi.s; move.w D2,D1;
    clr.w D2', D0 holding the other parameter; the index then in the
    count's register.
  - From 1 to `count', the same: the RING_ module-local 0xE76422 at
    0xE76432..0xE76442 `move.w (20,A6),D2; move.w D2,D3; subq.w #1,D3;
    bmi.s; move.w D3,D2; ...; moveq #1,D3', D0 and D1 holding the
    others (also 0xE7630C at 0xE76316..0xE76326). }
module flrn;
var
  ext_$tab: extern array[1..100] of integer32;

type
  pair = record
    lo, hi: integer32;
    port: integer32;
  end;
  pairs = array[0..19] of pair;
  pairs1 = array[1..20] of pair;

function flrn_$find(key: integer32; in tab: pairs; count: integer): integer;
  internal;
var
  i: integer;
begin
  for i := 0 to count - 1 do
    with tab[i] do
      if (key >= lo) and then (key <= hi) then begin
        flrn_$find := i;
        return;
      end;
  flrn_$find := -1;
end;

function flrn_$find1(a, b: integer32; in tab: pairs1; count: integer): integer;
  internal;
var
  i: integer;
begin
  for i := 1 to count do
    with tab[i] do
      if (b <= hi) and then (a >= lo) then begin
        flrn_$find1 := i;
        return;
      end;
  flrn_$find1 := 0;
end;

{ No index to set up: another unit's array stepped in place, its
  `movea.l' before the count's copy, its adjustment after (SHOW_CURSOR
  0xE6E2E2..0xE6E2E8 `bmi; movea.l A5,A1; move.w D1,D0; addq.l #8,A1'). }
procedure flrn_$fill(n: integer; k: integer32); internal;
var
  i: integer;
begin
  for i := 1 to n do
    ext_$tab[i] := k;
end;
