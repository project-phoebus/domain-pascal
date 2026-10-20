{ Execution test (bead domain-pascal-5kgg): set shapes on the
  lowering's best-effort path (codegen-patterns 5.1).  A one-byte set,
  `[byte] set of 0..7', holds member k in bit k of its byte (ref 3.15.5
  p. 3-61; reference-clarifications "A size attribute on a set type");
  a one-word set narrowed into one keeps members 0..7.

  - a one-word set local and a one-word set value parameter assigned to
    a one-byte set;
  - an integer32 member of a one-word set constructor `[k]` (ref 3.9.3:
    members 0..1023, the element's low word);
  - `in` with a `0..65535` element on a one-word constant set of a
    declared type.

  call: BE_L_SETS_$LOCAL w:3
  call: BE_L_SETS_$LOCAL w:1
  call: BE_L_SETS_$PARAM w:0x0228 w:0
  call: BE_L_SETS_$WIDE l:9
  call: BE_L_SETS_$WIDE l:4
  call: BE_L_SETS_$DECL w:9
  call: BE_L_SETS_$DECL w:2
  warns: best effort (not byte-identical): assigning a one-word set local or value parameter to a one-byte set
  warns: best effort (not byte-identical): `in` on a one-word constant set of a declared type
  warns: best effort (not byte-identical): a set member of a 4-byte type
  expect:
    true
    true
    true
    false
    true
    true
    true
    false
    true
    false
}
module be_l_sets;
%include 'exec.ins.pas';
type
  e8 = 0..7;
  bs_t = [byte] set of e8;
  ws_t = set of 0..15;
  uw = 0..65535;
var
  g: bs_t;

procedure be_l_sets_$local(k: integer); options(val_param);
var w: ws_t;
begin
  w := [1, 3, 9];
  if k > 2 then w := [1, 3, 4, 9];
  g := w;
  exec_$log_bool(3 in g);
  exec_$log_bool(4 in g);
end;

procedure be_l_sets_$param(w: ws_t; k: integer); options(val_param);
begin
  g := w;
  exec_$log_bool(3 in g);
  exec_$log_bool(5 in g);
end;

procedure be_l_sets_$wide(k: integer32); options(val_param);
var s: ws_t;
begin
  s := [k];
  exec_$log_bool(9 in s);
end;

procedure be_l_sets_$decl(k: uw); options(val_param);
begin
  exec_$log_bool(k in ws_t([1, 9]));
end;
