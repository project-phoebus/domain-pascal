{ ASKNODE_$INFO, 0xE64598..0xE645C4 (0x26 bytes of code, the word cell
  0xE645BE and the longword cell 0xE645C0; tools/imgdis.sh E64598 2C),
  the first routine of the ASKNODE code segment.  Semantics:
  ../source/domain_os/asknode/info.c.  ASKNODE_$INTERNET_INFO 0xE645EA,
  of the same module, lies outside the unit
  (`module-routines=ASKNODE_$INTERNET_INFO', a forward `bsr.s').

  Patterns exercised (docs/codegen-patterns.md):
  - 1.5  two constants passed by address (bead domain-pascal-5aw.9) in
         one call, among by-address parameters passed on: the longword -1
         pushed first (`pea (0x18,PC)' at 0xE645A4 -> 0xE645C0), the word
         0x98 second (0xE645AC -> 0xE645BE); laid out word first, then the
         longword (the 2 mod 4 group first), here 4-aligned without a
         filler.

  Reconstructed (not known from the original): the names; the parameter
  types (../source: an `int32_t' -1 and a `uint16_t' 0x98); modes. }
module asknode;
type
  status_$t = integer32;
  uid_$t = record high, low: integer32; end;

procedure asknode_$internet_info(in req_type: integer; in node_id: integer32;
                                 in req_len: integer32; in param: uid_$t;
                                 in resp_len: integer; out result: integer32;
                                 out status: status_$t); extern;

procedure asknode_$info(in req_type: integer; in node_id: integer32; in param: uid_$t;
                        out result: integer32; out status: status_$t);
begin
  asknode_$internet_info(req_type, node_id, -1, param, 16#98, result, status);
end;
