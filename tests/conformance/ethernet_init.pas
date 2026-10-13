{ ETHERNET_$INIT, 0xE78004, 0x10 bytes: the whole ETHERNET module (map `I
  E78004 ETHERNET size = 10'; tools/imgdis.sh E78004 10).  Semantics: a
  stub that returns status_$io_controller_not_in_system (0x100002, "OS /
  I/O manager: controller not in system" in the 10.2 status-code
  database) in D0; ../source/domain_os/mac_os/mac_os.h describes it, and
  has no C routine for it (the link test adds the unit, as for IO_$ALLOC).
  The same shape as SCSI_$CINIT (scsi.pas).

  A whole map module (bead domain-pascal-xdh.2): `link #-4' for the result
  variable, `move.l #0x100002,D0', and the module-end filler 0x2048
  (codegen-patterns 2, 3, 7).

  Reconstructed (not known from the original): no parameters (none is
  read); integer32 as the result type (as scsi.pas: a status in D0).  The
  status name is ../source's (io/io.h). }
module ethernet;
const
  status_$io_controller_not_in_system = 16#00100002;

function ethernet_$init: integer32;
begin
  ethernet_$init := status_$io_controller_not_in_system;
end;
