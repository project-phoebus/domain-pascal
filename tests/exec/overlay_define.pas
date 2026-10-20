{ Execution test: `define'd variables in overlay sections (ref 3.14,
  p. 3-52: a `define' of an `extern' variable of a `var (name)' part "will
  be stored in section_name"; bead domain-pascal-m15q).  The peer
  (overlay_define_peer.mod.pas) defines OVD_$TAIL and OVD_$ARR in ovd_$bss
  and OVD_$K, with an initial value, in ovd_$data, and so defines both
  sections; this unit declares the same sections positionally (ref 7.2.4,
  Method 4, p. 7-7), as common symbols, and reaches the defined variables
  also as plain `extern's (../source's ROUTE_$PORT and NETWORK_$ME
  declarations).  Each view sees the other's stores: the symbol and the
  section's offset are one object (codegen-patterns 11, "Overlay
  sections"; best effort, the image does not show which unit defined
  NET_PORT_TABLE).

  link: overlay_define_peer.mod.pas
  call: OVERLAY_DEFINE_$RUN w:2
  expect:
    305419896
    305419896
    3
    11
    100000
    true
    true
    -5
    100000
    11
    305419896
    77
    -2
    9
    4
}
module overlay_define;
%include 'exec.ins.pas';

var (ovd_$bss)
  mpad: integer32;
  mtail: integer32;
  marr: array[1..3] of integer;
var (ovd_$data)
  mdpad: integer32;
  mk: integer32;
var
  ovd_$arr: extern array[1..3] of integer;
  ovd_$tail: extern integer32;
  ovd_$k: extern integer32;

procedure ovd_$show; extern;
procedure ovd_$set; extern;

procedure overlay_define_$run(in k: integer);
options(val_param);
begin
  exec_$log_l(mk);
  exec_$log_l(ovd_$k);
  exec_$log_l(mdpad);
  marr[k] := 11;
  exec_$log_w(ovd_$arr[2]);
  ovd_$tail := 100000;
  exec_$log_l(mtail);
  exec_$log_bool(addr(ovd_$arr) = addr(marr));
  exec_$log_bool(addr(ovd_$k) = addr(mk));
  mpad := -5;
  ovd_$show;
  ovd_$set;
  exec_$log_w(marr[3]);
  exec_$log_l(mk);
  exec_$log_l(ovd_$tail);
  exec_$log_l(mdpad);
end;
