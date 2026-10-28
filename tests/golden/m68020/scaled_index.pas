{ codegen-patterns 14 ("Scaled index"); bead domain-pascal-vzzh.  With
  `-cpu mathchip' (MC68020 code, the SAU6..SAU9 kernels'), an array
  element's index is scaled by the operand's scale field instead of a
  shift or an add, and a displacement beyond a byte goes into the full
  extension word instead of `lea (0,An,Dx),Am; (d,Am)':

  - a longword element indexed by a word static: `move.w
    PROC1_$AS_ID,D1; move.l (0x3c,A5,D1.w*4),D0' (SAU9 FIM_$GET_FIM_ADDR
    0x3C40C390; the image's index register is D1, not the D0 dpc gives:
    open, bead domain-pascal-tbiu);
  - a word element indexed by a parameter's register, beyond a byte:
    `move.w (0xc42,A5,D2.w*2),(A0)' (SAU9 PROC1_$GET_TYPE 0x3C4175D4);
  - an 8-byte element copied out: `lea (0x3e0,A5,D0.w*8),A0; move.l
    (A0)+,(A1); move.l (A0)+,(4,A1)' (SAU9 NAME_$GET_NDIR_UID 0x3C45FD4E);
  - a byte element beyond a byte, scale 1: `tst.b (0xd47,A5,D0.w)' (SAU9
    BAT_$ALLOCATE 0x3C442FD4). }
module scaled_index;
type
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
var
  fim_pad: array[1..15] of integer32;                    { 0x00..0x3B }
  fim_$user_fim_addr: array[0..57] of integer32;         { 0x3C..0x123 }
  name_pad: array[1..175] of integer32;                  { 0x124..0x3DF }
  name_$ndir_uid: array[0..63] of uid_$t;                { 0x3E0..0x5DF }
  proc1_pad: array[1..817] of integer;                   { 0x5E0..0xC41 }
  proc1_$type: array[0..64] of integer;                  { 0xC42..0xCC3 }
  bat_pad: array[1..131] of integer;                     { 0xCC4..0xD46 }
  bat_$busy: array[0..31] of boolean;                    { 0xD47..0xD66 }
  proc1_$as_id: extern integer;
function fim_$get_fim_addr: integer32;
begin
  fim_$get_fim_addr := fim_$user_fim_addr[proc1_$as_id];
end;
procedure proc1_$get_type(pid: integer; out type_ret: integer); options(val_param);
begin
  type_ret := proc1_$type[pid];
end;
procedure name_$get_ndir_uid(out uid: uid_$t);
begin
  uid := name_$ndir_uid[proc1_$as_id];
end;
function bat_$is_busy(vol: integer): boolean; options(val_param);
begin
  bat_$is_busy := bat_$busy[vol];
end;
