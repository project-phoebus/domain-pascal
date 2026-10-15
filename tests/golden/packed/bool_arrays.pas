{ codegen-patterns 5.2 "Packed arrays of booleans" (ref 3.11.4.2 Table
  3-5 p. 3-47, 3.15.5 p. 3-62; bead domain-pascal-51e), PCHIST's
  `pchist_$proc_bitmap: packed array[1..64] of boolean' at +0x18 of the
  module's data (PCHIST's WIRED_DATA block 0xE2C204), its neighbours as
  ../source's pchist/pchist.pas lays them out.

  - count: `if bitmap[pid] then', pid a local held in a register, its
    low byte copied, `move.b D0,D1' (PCHIST_$COUNT 0xE1A156..0xE1A170,
    pid in D2 there: `move.b D2,D0; clr.w D1; subq.b #1,D0; move.b
    D0,D1; lsr.w #3,D1; lea (0,A5,D1.w),A1; moveq #7,D1; moveq #7,D3;
    and.b D0,D1; sub.b D1,D3; btst D3,(0x18,A1); beq');
  - fork: another unit's word read by its low byte (0xE5CC44 `move.b
    (0x00e20609).l,D0'), an `in' parameter through its address
    (0xE5CC9E `move.b (0x1,A2),D0') under `not' (`bne'), the element set
    from the word, `move.l #0x80; lsr.l; or.b' (PCHIST_$UNIX_PROFIL_FORK
    0xE5CCDC..0xE5CCF4, the byte offset in the count's register D1);
  - unprofile: cleared, `move.l #-0x81; lsr.l; and.b'
    (PCHIST_$UNIX_PROFIL_CNTL 0xE5CBF2..0xE5CC0C, which takes D3 for
    the offset: not reproduced, codegen-patterns 5.2);
  - used: a function result, `btst; sne D0' (ACL_$USED_SUSER
    0xE492DA..0xE492FE, whose bitmap is another unit's);
  - init_all: a `for' index held in a register (ACL_$INIT 0xE310E4
    `move.w D3,D0').
  The FIM_ frame size table (FIM_'s block 0xE2126C+0x124, `08 08 0C 00
  10 00 00 00 3A 14 20 5C 00 00 00 00') as a packed array of 0..255: one
  byte per element, initialised. }
module pchist;
var
  pchist_$lock: array[1..9] of integer;                    { +0x000 }
  unused_12: array[1..3] of integer;                       { +0x012 }
  pchist_$proc_bitmap: packed array[1..64] of boolean;     { +0x018 }
  pchist_$addr: array[1..64] of integer32;                 { +0x020 }
  pchist_$sys_profiling_count: integer;                    { +0x120 }
  pchist_$proc_profiling_count: integer;                   { +0x122 }
  frame_size: packed array[0..15] of 0..255 := [
    16#08, 16#08, 16#0C, 0, 16#10, 0, 0, 0, 16#3A, 16#14, 16#20, 16#5C, * of 0];
  proc1_$current: extern integer;

procedure count(in mode: integer);
var pid: integer;
begin
  pid := proc1_$current;
  if mode = 1 then
    if pchist_$proc_bitmap[pid] then
      pchist_$addr[pid] := 0;
end;

procedure fork(in child: integer);
begin
  if pchist_$proc_bitmap[proc1_$current] then begin
    if not pchist_$proc_bitmap[child] then
      pchist_$proc_profiling_count := pchist_$proc_profiling_count + 1;
    pchist_$proc_bitmap[child] := true;
  end;
end;

procedure unprofile(in pid: integer);
begin
  pchist_$proc_bitmap[pid] := false;
end;

function used: boolean;
begin
  used := pchist_$proc_bitmap[proc1_$current];
end;

procedure init_all;
var i: integer;
begin
  for i := 1 to 64 do
    pchist_$proc_bitmap[i] := true;
end;
