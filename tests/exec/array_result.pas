{ Execution test (bead domain-pascal-338r, domain-c's gap 33): a
  function returning a `packed array [1..4] of char', in D0 as a 4-byte
  record is (codegen-patterns 3; best effort, no kernel site; the DOMAIN
  C compiler's dmp_$three_char_lr_name_from_lr, reported: `move.l
  (-8,A6),D0').

  call: ARRAY_RESULT_$RUN w:33
  expect:
    97
    98
    99
    33
}
module array_result;
%include 'exec.ins.pas';

type
  n_t = packed array [1..4] of char;
var
  m: n_t;

function f(i: integer): n_t; options(val_param);
var
  r: n_t;
begin
  r[1] := 'a'; r[2] := 'b'; r[3] := 'c'; r[4] := chr(i);
  f := r;
end;

procedure array_result_$run(k: integer); options(val_param);
begin
  m := f(k);
  exec_$logv_w(ord(m[1]));
  exec_$logv_w(ord(m[2]));
  exec_$logv_w(ord(m[3]));
  exec_$logv_w(ord(m[4]));
end;
