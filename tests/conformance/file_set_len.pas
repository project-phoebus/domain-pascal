{ FILE_$SET_LEN, 0xE73F90..0xE73FB0 (0x1A bytes of code, the filler
  0x2048 at 0xE73FAA and the longword cell 0xE73FAC; tools/imgdis.sh
  E73F90 20), the first routine of the FILE_ code segment at 0xE73F90.
  Semantics: ../source/domain_os/file/set_len.c.  FILE_$TRUNCATE_D
  0xE73FE4, of the same module, lies outside the unit
  (`module-routines=FILE_$TRUNCATE_D'; a forward `bsr.s', which dpc
  writes as its encoding `613e', gas not relocating an 8-bit
  displacement to an undefined symbol).

  Patterns exercised (docs/codegen-patterns.md):
  - 1.5  a longword constant 0 passed by address (bead
         domain-pascal-5aw.9) between by-address parameters passed on:
         `pea (0x12,PC)' at 0xE73F98, the cell 4-aligned after the
         0x2048 filler (an aligning routine);
  - 1.1  no pop before `unlk'.

  Reconstructed (not known from the original): the names; the third
  parameter of FILE_$TRUNCATE_D as an `integer32' value (../source calls
  the cell `file_$truncate_nil_context'); `in'/`var' modes. }
module file_;
type
  status_$t = integer32;
  uid_$t = record high, low: integer32; end;

procedure file_$truncate_d(in uid: uid_$t; in length: integer32; context: integer32;
                           out status: status_$t); extern;

procedure file_$set_len(in uid: uid_$t; in length: integer32; out status: status_$t);
begin
  file_$truncate_d(uid, length, 0, status);
end;
