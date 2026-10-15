{ Execution test: by-address parameters loaded for one use after a branch
  into a register past the ones holding dead slot loads (codegen-patterns
  12.4 R4(b), bead domain-pascal-i24w): the out parameter after the
  status test, in A3 past A2 (TTY_$K_INQ_PGROUP 0xE6796E's shape); a third
  slot after an `if', in A2 past A0 and A1.  Each runs on both sides of
  its test.  Expected values from ref "Assignment" (p. 4-25) and the `if'
  statement (p. 4-112).

  link: late_slot_pass_desc.mod.pas
  call: LSP_$RUN l:-1
  call: LSP_$RUN l:2
  call: LSP_$AFTER_TEST w:3 @w:9! @w:9! @w:9!
  call: LSP_$AFTER_TEST w:4 @w:9! @w:9! @w:9!
  expect:
    1
    7
    8
    0
    5
    6
    out 1
    out 2
    out 0
    out 1
    out 2
    out 9
}
module lsp;
%include 'exec.ins.pas';

type
  uid_$t = record
    high, low: integer32;
  end;
  desc_t = record
    pad: integer32;
    uid: uid_$t;
  end;
  desc_ptr_t = ^desc_t;
var
  u: uid_$t := [high := 7, low := 8];
  w: integer;

function lsp_$getd(in k: integer32; out st: integer32): integer32;
  options(extern, a0_return);

procedure lsp_$inq(k: integer32; out uid: uid_$t; out st: integer32);
  options(val_param);
var
  d: integer32;
begin
  d := lsp_$getd(k, st);
  if st <> 0 then
    return;
  uid := desc_ptr_t(d)^.uid;
end;

procedure lsp_$run(k: integer32);
  options(val_param);
var
  st: integer32;
begin
  lsp_$inq(k, u, st);
  exec_$logv_l(st);
  exec_$logv_l(u.high);
  exec_$logv_l(u.low);
end;

procedure lsp_$after_test(k: integer; out a, c, x: integer);
  options(val_param);
begin
  w := k;
  a := 1;
  c := 2;
  if w = 3 then
    x := 0;
end;
