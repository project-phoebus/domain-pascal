{ Execution test (bead domain-pascal-5kgg): best-effort forms of kept
  bases, M$ARITH calls and holds: another unit's element reached again
  after a branch from before the first base's load (a base of its own,
  bead domain-pascal-lnzy), an M$ARITH call with a word argument and a
  computed operand (the pad after the computation, codegen-patterns 8),
  and the static link last used by a call's arguments in a body with
  branches (held through the call, 12.4 R4(c)).  Values of
  elem_bases_data.mod.pas: eb_$longs[k] = 100000k, eb_$words[i] = 64i -
  1000.

  call: EB_$FILL
  call: BX_MISC_$KEPT w:2 w:1 -> l
  call: BX_MISC_$KEPT w:2 w:0 -> l
  call: BX_MISC_$PAD l:100 w:7
  call: BX_MISC_$OUTER w:4
  call: BX_MISC_$OUTER w:-4
  link: elem_bases_data.mod.pas
  warns: best effort (not byte-identical): an element of another unit's array that an earlier statement since the last call reached too
  warns: best effort (not byte-identical): an M$ARITH call with a word argument and a computed operand
  warns: best effort (not byte-identical): in a body with branches, a call whose arguments are the last use of the static link
  expect:
    => 500000
    => 200000
    14
    mark 4
    5
    -4
}
module be_x_misc;
%include 'exec.ins.pas';
var
  eb_$longs: extern array[0..9] of integer32;
  eb_$words: extern array[1..8] of integer;
  gl: integer32;
  gw: integer;

function bx_misc_$kept(k, c: integer): integer32; options(val_param);
var x: integer32;
begin
  x := 0;
  if c > 0 then x := eb_$longs[k + 1];
  bx_misc_$kept := x + eb_$longs[k];
end;

procedure bx_misc_$pad(a: integer32; c: integer); options(val_param);
begin
  gw := c;
  gl := (a + 1) div gw;
  exec_$logv_l(gl);
end;

procedure bx_misc_$outer(n: integer); options(val_param);
var t: integer;
  procedure inner(k: integer); options(val_param);
  begin
    if k > 0 then begin
      exec_$mark(k);
      exec_$logv_w(t + 1);
    end
    else
      exec_$logv_w(k);
  end;
begin
  t := n;
  inner(n);
end;
