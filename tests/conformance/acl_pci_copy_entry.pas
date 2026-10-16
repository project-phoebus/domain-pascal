{ The nested routine at 0xE45122, 0x32 bytes (tools/imgdis.sh E45122 32),
  of the ACL_ module-local routine at 0xE4519C (acl_$prim_create_internal
  in ../source).  Semantics: ../source domain_os/acl/prim_create_internal.c
  and prim_create_internal.pas (`acl_$pci_copy_entry'); the record
  layouts: acl/acl_internal.h, acl/acl.ins.pas and acl/acl.pas
  (acl_$cache_entry_t, acl_$v4_entry_t).  UID_$NIL is the map's name at
  0xE1737C.

  The unit is the nested routine alone (README.md, "Nested routines";
  `enclosing=PRIM_CREATE_INTERNAL' in the manifest).  In the image the
  routine after it, at 0xE45154, is another nested routine of the same
  parent (../source's acl_$pci_subsys_rights); the stand-in enclosing
  routine is linked there and not compared.  The nested code reaches
  nothing of its parent (no static link: its two operands are its own
  by-address parameters), so the parent's heading and body do not
  matter.

  Patterns exercised (docs/codegen-patterns.md):
  - 5.2  "Arrays of char": `ids := src.ids', 24 characters copied byte by
         byte, `lea (A0),A2; moveq #0x17,D0; move.b (A1)+,(A2)+; dbf'
         (0xE45130..0xE45136), the destination a pointer held in A0 (the
         `with' record, `lea (An),Am', "Block addresses"), the source's
         slot in A1, the count after both addresses (bead
         domain-pascal-40r2);
  - 7    the loop head at 0xE45134, 4-aligned without a filler (aligning
         code);
  - 5.2  "6 and 8 bytes": `subsys := uid_$nil', `movea.l #UID_$NIL,A1;
         move.l (A1)+,(0x18,A0); move.l (A1)+,(0x1c,A0)';
  - 2    `link.w A6,#-8', `pea (A2)', the restore `movea.l (-0xc,A6),A2'.

  Reconstructed (not known from the original): every name in the unit;
  `ids' as an array of char in both records (a byte copy is the form of
  one, codegen-patterns 5.2; the bytes are three UIDs); the `with'; dst
  `out'.  The frame: `link #-8'; the frame-size model of codegen-patterns
  2 explains 4 bytes (the slot of a routine that is not a global symbol,
  as for ring_discard_packet.pas), the other 4 are the stand-in local
  `unexplained' (../source reads them as the `with' temporary). }
module acl;
type
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
  acl_$cache_entry_t = record
    ids: array[1..24] of char;            { 0x00 }
    reserved_18: integer;                 { 0x18 }
    rights: integer;                      { 0x1A }
    entry_1c: array[1..2] of integer;     { 0x1C }
  end;
  acl_$v4_entry_t = record
    case integer of
      1: (person: uid_$t;                 { 0x00 }
          group: uid_$t;                  { 0x08 }
          org: uid_$t;                    { 0x10 }
          subsys: uid_$t;                 { 0x18 }
          reserved_20: integer32;         { 0x20 }
          reserved_24: integer32;         { 0x24 }
          rights: integer32);             { 0x28 }
      2: (ids: array[1..24] of char);     { 0x00 }
  end;
var
  uid_$nil: extern uid_$t;

procedure prim_create_internal(var e: acl_$cache_entry_t;
                               var v: acl_$v4_entry_t); internal;

  procedure pci_copy_entry(in src: acl_$cache_entry_t;
                           out dst: acl_$v4_entry_t);
  var
    unexplained: integer32;   { stand-in: 4 frame bytes the model leaves unexplained }
  begin
    with dst do begin
      ids := src.ids;
      reserved_20 := 0;
      subsys := uid_$nil;
    end;
  end;

begin
  { stand-in body (not compared) }
  pci_copy_entry(e, v);
end;
