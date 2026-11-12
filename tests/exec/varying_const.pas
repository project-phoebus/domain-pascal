{ Execution test (bead domain-pascal-338r, domain-c's gaps 17 and 18):
  string and character constants assigned to varying strings and passed
  to an `in' varying parameter ("Variable-Length String Operations",
  pp. 4-204..4-205: the length word, the characters and a trailing null,
  the rest of the body left as it was; best effort, no kernel site).
  Read back through the `length' and `body' fields.

  call: VARYING_CONST_$RUN
  expect:
    10
    45
    111
    0
    120
    1
    90
    0
    5
    72
    111
}
module varying_const;
%include 'exec.ins.pas';

type
  v_t = varying [40] of char;
var
  s: varying [1024] of char;
  t: v_t;

procedure show(in v: v_t);
begin
  exec_$logv_w(v.length);
  exec_$logv_w(ord(v.body[1]));
  exec_$logv_w(ord(v.body[v.length]));
end;

procedure varying_const_$run;
begin
  s.body[14] := 'x';
  s := '-D__apollo';
  exec_$logv_w(s.length);
  exec_$logv_w(ord(s.body[1]));
  exec_$logv_w(ord(s.body[10]));
  exec_$logv_w(ord(s.body[11]));
  exec_$logv_w(ord(s.body[14]));
  t := 'Z';
  exec_$logv_w(t.length);
  exec_$logv_w(ord(t.body[1]));
  exec_$logv_w(ord(t.body[2]));
  show('Hello');
end;
