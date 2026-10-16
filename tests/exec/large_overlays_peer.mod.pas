{ The second unit of large_overlays.pas: the same section, its words
  read and written absolutely (constant indices), whatever their offset
  (FILE_$LOCK_INIT 0xE327AC `clr.w (0xe9f9c4).l'), and an element of the
  array at the start from `movea.l #sym,An'. }
module large_overlays_peer;
%include 'exec.ins.pas';

var (lov_$data)
  pfirst: array[1..16#3E00] of integer;    { +0x0000 }
  ptable: array[1..64] of integer;         { +0x7C00 }
  pbig: array[1..16#400] of integer;       { +0x7C80 }
  plast: array[1..64] of integer;          { +0x8480 }
define lov_$show, lov_$set;

procedure lov_$show;
begin
  exec_$log_w(pfirst[3]);
  exec_$log_w(ptable[1]);
  exec_$log_w(ptable[64]);
  exec_$log_w(plast[1]);
  exec_$log_w(plast[64]);
end;

procedure lov_$set;
begin
  pbig[16#400] := -1;
  plast[64] := -2;
  exec_$log_w(pbig[16#400]);
  exec_$log_w(plast[64]);
end;
