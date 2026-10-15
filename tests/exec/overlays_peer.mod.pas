{ The second unit of overlays.pas: it declares the same two overlay
  sections under other names (ref 7.2.4, Method 4, p. 7-7: "There's no
  requirement that the variables in Sub2 have the same names"), gives
  the first its initial values (as Method 4's Program Math does), and
  reads and writes them. }
module overlays_peer;
%include 'exec.ins.pas';

var (ovl_$init)
  pa: integer := 5;
  pb: integer32 := 16#12345678;
  pc: char := 'Q';
var (ovl_$zero)
  za: integer;
  zb: array[1..3] of integer32;
define ovp_$show, ovp_$set;

procedure ovp_$show;
begin
  exec_$log_w(pa);
  exec_$log_l(pb);
  exec_$log_c(pc);
  exec_$log_w(za);
  exec_$log_l(zb[2]);
  exec_$log_l(zb[3]);
end;

procedure ovp_$set;
begin
  pa := -7;
  zb[3] := 99;
end;
