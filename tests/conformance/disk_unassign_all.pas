{ DISK_$UNASSIGN_ALL, 0xE6BE1C..0xE6BE4A (0x2E bytes; tools/imgdis.sh
  E6BE1C 2E), a fragment of the DISK_ module (code segment 0xE6B6DC,
  which aligns its loop heads up to DISK_$LV_MOUNT 0xE6CA3A:
  tools/loopalign.py switch DISK_).  Semantics:
  ../source/domain_os/disk/unassign_all.c (DISK_$UNASSIGN for volumes
  1..10, the status never looked at).

  Patterns (docs/codegen-patterns.md): 7.4 (a `for' loop with constant
  bounds and its index used in the body: the `dbf' count loaded first,
  `moveq #9,D2', then the index, `moveq #1,D3'; the index stepped with
  `addq.w #1' before `dbf'; the head 0xE6BE28 already 4-aligned, so no
  filler; the index's frame slot never written), 12.4 R3 (the count and
  the index live across the call: D2, D3, saved with `movem'), 1.1/1.6
  (two by-address arguments, popped with `addq.w #8,SP' inside the loop;
  `bsr.w' to DISK_$UNASSIGN, the routine before it in the module, outside
  this fragment: the manifest's `module-routines=' token, dpc
  --module-routine).

  Reconstructed (not known from the original): the names `status', `vol'
  and `i'; `vol := i' as a statement of its own (the C's `vol_idx', a
  frame word at -6 stored from D3 before each call: `move.w D3,(-6,A6)'
  at 0xE6BE28).  The image cannot tell this from the index itself being
  stored to its frame slot before it is passed by address; dpc keeps a
  `for' index in a register only and refuses its address.  The frame:
  `link #-8' fits the three locals here (4 + 2 + 2 bytes), with `vol' at
  -6 and `status' at -4 as the image addresses them. }
module disk_;
type
  status_$t = integer32;

procedure disk_$unassign(var vol: integer; var status: status_$t); extern;

procedure disk_$unassign_all;
var
  status: status_$t;
  vol: integer;
  i: integer;
begin
  for i := 1 to 10 do begin
    vol := i;
    disk_$unassign(vol, status);
  end;
end;
