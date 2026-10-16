{ Execution test: slot loads that run out of address registers reuse the
  one written least recently (codegen-patterns 12.4 R4(a), R4(b); bead
  domain-pascal-ei42; golden regalloc/slot_lru): seven value parameters
  copied at entry, A0-A4 then A0 and A1 again (MSG_$SEND 0xE59A0A..
  0xE59A38); seven `in' words read once by one argument list (MSG_$SENDI
  0xE59AB6..0xE59AF2).  Each value reaches the callee in its place
  (ref 5.2, p. 5-4: arguments matched to parameters by position).

  call: SLRU_$COPIES @w:1 @w:2 @w:3 @w:4 @w:5 @w:6 @w:7
  call: SLRU_$ARGS @w:11 @w:12 @w:13 @w:14 @w:15 @w:16 @w:17
  expect:
    1
    2
    3
    4
    5
    6
    7
    11
    12
    13
    14
    15
    16
    17
}
module slru;
%include 'exec.ins.pas';

procedure slru_$take(a, b, c, d, e, f, g: integer);
  options(val_param);
begin
  exec_$logv_w(a);
  exec_$logv_w(b);
  exec_$logv_w(c);
  exec_$logv_w(d);
  exec_$logv_w(e);
  exec_$logv_w(f);
  exec_$logv_w(g);
end;

procedure slru_$copies(a, b, c, d, e, f, g: integer);
begin
  slru_$take(a, b, c, d, e, f, g);
end;

procedure slru_$args(in a, b, c, d, e, f, g: integer);
begin
  slru_$take(a, b, c, d, e, f, g);
end;
