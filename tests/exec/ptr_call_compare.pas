{ Execution test (bead domain-pascal-338r, domain-c's gap 12): a
  pointer function's result `=' / `<>' a pointer variable, in a condition and as a value (codegen-patterns 3; best
  effort, no kernel site): the call, then `cmpa.l <other>,A0'.  With
  the call on the right, the existing form (the variable loaded first).  The
  DOMAIN C compiler's append_root_rec_to_bucket (reported: cc.m68k
  0x06CF9C) `bsr.w get_bucket; cmpa.l A3,A0; bne'.

  call: PTR_CALL_COMPARE_$RUN w:0 w:1
  expect:
    1
    0
    1
    0
    0
    1
    0
    1
}
module ptr_call_compare;
%include 'exec.ins.pas';

type
  r_t = record a: integer; end;
  r_p = ^r_t;
var
  x, y: r_t;
  g: r_p;
  b: boolean;

function pick(i: integer): r_p; options(val_param);
begin
  if i = 0 then pick := addr(x) else pick := addr(y);
end;

procedure one(i: integer; q: r_p); options(val_param);
var
  r: integer;
begin
  r := 0;
  if pick(i) = q then r := 1;
  exec_$log_w(r);
  r := 0;
  if g <> pick(i) then r := 1;
  exec_$log_w(r);
  b := pick(i) = g;
  if b then exec_$log_w(1) else exec_$log_w(0);
  b := q <> pick(i);
  if b then exec_$log_w(1) else exec_$log_w(0);
end;

procedure ptr_call_compare_$run(i: integer; j: integer); options(val_param);
begin
  g := addr(x);
  one(i, addr(x));
  one(j, addr(x));
end;
