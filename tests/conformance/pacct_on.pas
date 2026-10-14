{ PACCT_$ON, 0xE5A9A4, 0x26 bytes (tools/imgdis.sh E5A9A4 26), a fragment
  of the PACCT module's code segment 0xE5A6C0.  Semantics:
  ../source/domain_os/pacct/on.c (accounting is on when the owner UID is
  not UID_$NIL); data layout: pacct_state_t in
  ../source/domain_os/pacct/pacct_internal.h (owner, a UID, at +0 of the
  PACCT data block, map `D E817EC PACCT size = 20').  UID_$NIL is
  another unit's object (map 0xE1737C, UID_LIST).

  Patterns exercised (docs/codegen-patterns.md):
  - 7.1  `<>' of two 8-byte records (ref Table 4-1, p. 4-2: `=' and `<>'
         for all types): the left record's address in A0 (`lea (A5),A0'),
         the right one's in A1 (`movea.l #UID_$NIL,A1', another unit's
         object), the count `moveq #1,D0', two `cmpm.l (A1)+,(A0)+' with a
         `bne' to the end after the first, then the result `sne' into the
         count's register;
  - 3    a boolean function result in D0 (`link #-4');
  - 4    A5 saved with `pea (A5)', loaded, restored.

  Reconstructed (not known from the original): the names (the C's);
  uid_$t as an unpacked record of two integer32s (base.ins.pas declares
  it packed, which lays out the same 8 bytes, ref 3.10.7); only `owner'
  of the 0x20-byte data block is declared (it is at offset 0). }
module pacct;
type
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
var
  pacct_owner: uid_$t;                    { +0 of the PACCT data block }
  uid_$nil: extern uid_$t;

function pacct_$on: boolean;
begin
  pacct_$on := pacct_owner <> uid_$nil;
end;
