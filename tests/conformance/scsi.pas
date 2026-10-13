{ The whole SCSI_ module: two code sections and a data section.
  Map (SAU2): `I E34F04 SCSI_ size = 10' in OS_INIT_PROC (SCSI_$CINIT),
  `I E88800 SCSI_ size = 4' in SCSI_WIRED_PROC (SCSI_$FREE_ASID) and
  `D E88804 SCSI_ size = 4' in SCSI_WIRED_DATA (SCSI_$PRESENT).
  Image: tools/imgdis.sh E34F04 10; tools/imgdis.sh E88800 8.
  Semantics: ../source/domain_os/scsi/cinit.c, scsi/free_asid.c.

  Patterns exercised (docs/codegen-patterns.md):
  - 11   named sections: the module heading names the code and data
         sections (ref 7.1.1, p. 7-2), a routine attribute moves one
         routine to another code section (ref 5.7.1, p. 5-24); dpc emits
         one output section per Pascal section, `.text.SCSI_.<SECTION>'
         and `.bss.moddata.SCSI_.SCSI_WIRED_DATA', each placed by the
         manifest at its map segment; each code section is a whole map
         segment, with its own module-end padding (0x2048 after the
         `rts' of either);
  - 2, 3 SCSI_$CINIT: `link.w A6,#-4' for the result variable, the
         result in D0 (`move.l #0x100002,D0');
  - 2    SCSI_$FREE_ASID: an empty procedure is a bare `rts'.

  Reconstructed (not known from the original): the section names as
  identifiers (the map's, folded; OS_INIT_DATA as SCSI_$CINIT's data
  section is the map's pairing of OS_INIT_PROC, codegen-patterns 11; it
  allocates nothing there); the order of the bodies; integer32 for
  status_$t and SCSI_$PRESENT (4 bytes, the whole data segment; nothing
  in these routines reads it); SCSI_$FREE_ASID's parameter (PROC2_$DELETE
  passes the ASID by address, ../source proc2/cleanup_handlers_internal.c;
  its type is not seen).  The status name is ../source's
  (io/io.h: status_$io_controller_not_in_system = 0x00100002). }
module scsi_, scsi_wired_proc, scsi_wired_data;
const
  status_$io_controller_not_in_system = 16#00100002;
var
  scsi_$present: integer32;
define
  scsi_$present;

[section(os_init_proc, os_init_data)]
function scsi_$cinit: integer32;
begin
  scsi_$cinit := status_$io_controller_not_in_system;
end;

procedure scsi_$free_asid(in asid: integer);
begin
end;
