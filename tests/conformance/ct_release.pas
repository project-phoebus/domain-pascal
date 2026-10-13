{ CT_$RELEASE, 0xE70AF4, 4 bytes: the whole CT module (map `I E70AF4 CT
  size = 4', where the map also puts CT_$PROC_START; tools/imgdis.sh
  E70AF4 4).  Semantics: ../source/domain_os/ct/release.c: the cartridge
  tape driver is configured out and CT_$RELEASE does nothing.  The same
  shape as MT_$RELEASE (mt_release.pas).

  A whole map module (bead domain-pascal-xdh.2): `rts' and the module-end
  filler 0x2048 (codegen-patterns 2, 7).  CT_$PROC_START, the other map
  name at 0xE70AF4, is not a routine of the unit: ../source defines no
  such symbol, and the unit does not either.

  Reconstructed (not known from the original): the parameters (an empty
  procedure's are not visible; ../source's three, by address: `in' for
  the 16-bit unit and mode, `out' for the status); status_$t as a record
  with the integer32 field `all' (ref 9.1.1-9.1.2, pp. 9-1..9-3). }
module ct;
type
  status_$t = record
    all: integer32;
  end;

procedure ct_$release(in unit: integer; in mode: integer; out status: status_$t);
begin
end;
