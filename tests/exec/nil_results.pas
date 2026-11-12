{ Execution test (bead domain-pascal-338r, domain-c's gap 11): `nil' as
  a pointer function's result (codegen-patterns 3, "`nil` results";
  best effort, no kernel site): a body of the one statement `f := nil',
  `movea.l #0,A0' (the DOMAIN C compiler's make_temp, reported: cc.m68k
  0x09F9BC); and `nil' where every assignment of the result is the last
  statement of its path, another path assigning a register-held local
  (arg_option_parameter, reported: 0x0B27F2 `suba.l A0,A0').

  call: NIL_RESULTS_$RUN
  expect:
    1
    7
    1
    9
}
module nil_results;
%include 'exec.ins.pas';

type
  p_t = ^integer;
var
  argv: array [1..4] of p_t;
  v: array [1..4] of integer;
  i, argc: integer;

function none(n: integer): p_t; options(val_param);
begin
  none := nil;
end;

function aop(required: boolean): p_t; options(val_param);
var
  p: p_t;
begin
  p := argv[i + 1];
  if (i + 1 < argc) and (p^ <> 0) then begin
    i := i + 1;
    aop := p;
  end
  else if not required then
    aop := nil
  else
    exec_$logv_w(-1);
end;

procedure nil_results_$run;
var
  q: p_t;
begin
  if none(3) = nil then exec_$logv_w(1);
  v[1] := 0; v[2] := 7; v[3] := 0; v[4] := 9;
  argv[1] := addr(v[1]); argv[2] := addr(v[2]);
  argv[3] := addr(v[3]); argv[4] := addr(v[4]);
  argc := 4;
  i := 1;
  q := aop(false);
  exec_$logv_w(q^);
  q := aop(false);
  if q = nil then exec_$logv_w(1);
  i := 3;
  argc := 5;
  q := aop(true);
  exec_$logv_w(q^);
end;
