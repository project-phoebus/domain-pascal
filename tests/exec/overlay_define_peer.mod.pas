{ The second unit of overlay_define.pas: it `define's variables in two
  overlay sections (ref 3.14, p. 3-52: a `define' of an `extern' variable
  of a `var (name)' part "will be stored in section_name"; ROUTE_$PORT's
  shape in ../source route/route.ins.pas and route_wired.pas), so it
  defines both sections (codegen-patterns 11, "Overlay sections"; bead
  domain-pascal-m15q): ovd_$bss with no initial values (NOBITS
  `.bss.overlay.OVD_$BSS'), ovd_$data with them (`.data.overlay.OVD_$DATA').
  Allocation is in order: pad +0, ovd_$tail +4 (`define' in its
  declaration), ovd_$arr +8 (allocated by the `define' below); dpad +0,
  ovd_$k +4. }
module overlay_define_peer;
%include 'exec.ins.pas';

var (ovd_$bss)
  pad: integer32;
  ovd_$tail: define integer32;
  ovd_$arr: extern array[1..3] of integer;
var (ovd_$data)
  dpad: integer32 := 3;
  ovd_$k: extern integer32;

define ovd_$arr;
define ovd_$k := 16#12345678;
define ovd_$show, ovd_$set;

procedure ovd_$show;
begin
  exec_$log_l(pad);
  exec_$log_l(ovd_$tail);
  exec_$log_w(ovd_$arr[2]);
  exec_$log_l(ovd_$k);
end;

procedure ovd_$set;
begin
  ovd_$arr[3] := 77;
  ovd_$k := -2;
  ovd_$tail := 9;
  dpad := 4;
end;
