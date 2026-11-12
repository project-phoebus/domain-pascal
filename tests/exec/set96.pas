{ Execution test (bead domain-pascal-338r, domain-c's gaps 29 and 38):
  a `set of 0..95' variable tested with longword elements whose type
  reaches beyond its bits (a range test, then the bit test; the DOMAIN C
  compiler's sched.pas, reported: `cmpi #95' first), in a condition and
  as a value, and members added with computed elements (best effort, no
  site).

  call: SET96_$RUN l:5 l:60 l:95 l:96
  call: SET96_$RUN l:-1 l:63 l:0 l:200
  expect:
    1
    0
    0
    1
    1
    0
    0
    0
    0
    0
    0
    1
    0
    0
    0
    0
}
module set96;
%include 'exec.ins.pas';

type
  regset = set of 0..95;
var
  regs: regset;
  b: boolean;

procedure one(n: integer32);
begin
  if n in regs then exec_$logv_w(1) else exec_$logv_w(0);
  b := n + 32 in regs;
  if b then exec_$logv_w(1) else exec_$logv_w(0);
end;

procedure set96_$run(a: integer32; c: integer32; d: integer32; e: integer32);
  options(val_param);
var
  m: 0..95;
begin
  regs := [];
  m := 2;
  regs := regs + [m + 3];
  m := 95;
  regs := regs + [m];
  regs := regs + [92];
  one(a);
  one(c);
  one(d);
  one(e);
end;
