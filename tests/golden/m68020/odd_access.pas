{ codegen-patterns 14 ("Odd addresses"); bead domain-pascal-gkrx.  With
  `-cpu mathchip' a word or longword reference to a byte-aligned object
  (`aligned(0)', ref 3.15.6.3 p. 3-70) at an address not known to be
  even is generated as the plain reference, which the MC68010 build
  refuses (the 68010 traps): SAU9 SCSI_WAIT 0x3C4778F2 `move.l
  (35,A5),(-116,A6)'; ETHERNET_$SEND_OS 0x3C483F88 `move.w (3,A3),(3,A0)';
  RING_8025_$INIT 0x3C487148 `clr.w (3,A2); move.w #-248,(5,A2)'.
  - through a pointer to a byte-aligned word (ref 3.15.6.7);
  - a byte-aligned word field after a char, at offset 1;
  - a `for' fill of another unit's byte-aligned array (`clr.w (A0)+'). }
module odd_access;
type
  bw = [aligned(0)] integer16;
  r_t = record
    c: char;
    w: bw;
  end;
  oa = [aligned(0)] array [1..4] of integer16;
var
  bp: extern ^bw;
  r: extern r_t;
  x: extern oa;
  i: integer;

procedure p;
var k: integer;
begin
  i := bp^;
  i := r.w;
  r.w := 7;
  for k := 1 to 4 do
    x[k] := 0;
end;
