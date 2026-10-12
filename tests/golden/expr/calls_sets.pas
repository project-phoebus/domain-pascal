{ Golden: codegen-patterns 3, 1.1, 5.1, 7.1.
  - a function call in an expression: the call sequence of a procedure
    call, the result read from D0 (0xE04700..0xE04704 `bsr.w; move.l
    D0,(A3)', no pop before the epilogue); copied before arithmetic
    (0xE01BA2..0xE01BA4 `addq.w #8,SP; move.w D0,D1'); a boolean result
    tested with `tst.b D0' after the pop (0xE0082C..0xE00830); the tail
    wrapper `f := g(x)', the call followed by the epilogue (ACL_$IS_SUSER);
  - `k in s' for a constant k and a one-word set: `move.w; btst #k,Dn;
    sne Dn' (REM_NAME_SERVER_LOCAL 0xE4A412..0xE4A41A), and in a condition
    `btst #k,Dn; beq' (0xE013BA..0xE013C2);
  - a comparison as a boolean result: the compare, then `s<cc>'
    (AUDIT_$IS_PROCESS_AUDITED 0xE70DA8..0xE70DAC; ACL_$IN_SUBSYS
    0xE470B6..0xE470BA `sgt'). }
module calls_sets;
type
  flags_t = set of 0..15;
  rec = record
    pad: array[1..11] of integer;
    flags: flags_t;
  end;
var
  w: extern integer;
  l: extern integer32;
  r: extern integer;
  rp: extern ^rec;
function f(x: integer): integer; options(val_param, extern);
function g(x: integer32): integer32; options(val_param, extern);
function b(x: integer): boolean; options(val_param, extern);
procedure store_result; internal;
begin
  l := g(l);
end;
procedure arith_on_result; internal;
begin
  w := f(r) + 1;
end;
procedure test_result; internal;
begin
  if b(w) then r := 0;
  r := 1;
end;
function wrapper(x: integer32): integer32; internal;
begin
  wrapper := g(x);
end;
function bit13: boolean; internal;
begin
  bit13 := 13 in rp^.flags;
end;
procedure bit4; internal;
begin
  if 4 in rp^.flags then r := 0;
  r := 1;
end;
function positive: boolean; internal;
begin
  positive := w > 0;
end;
function not_two: boolean; internal;
begin
  not_two := w <> 2;
end;
