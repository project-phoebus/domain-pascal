{ NAME_CONVERT_ACL_STATUS, 0xE5861C, 0x3A bytes (tools/imgdis.sh E5861C
  3A).  Semantics: ../source/domain_os/misc/name_convert_acl_status.c;
  status values and names: ../source/domain_os/acl/acl.h and
  name/name.h (texts in the 10.2 status-code database: 0x230001 "no
  right to perform operation", 0x230002 "insufficient rights to perform
  operation", 0x230004 is acl.h's "wrong type - operation illegal on
  system objects"; 0xE0013 "no rights", 0xE0014 "insufficient rights",
  0xE000E "branch is not a directory").

  A fragment of the NAME module (code segment 0xE58488), compiled with
  `dpc --fragment-origin 0xE5861C'.  The routine is a map name without
  `$' and is called from DIR and NAME; it does not use the module's data,
  and this unit declares none (codegen-patterns 4).

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2   default convention: the status by address at (8,A6);
  - 7     `if ... then ... else if ...': each test skips its `then' part
          with the negated condition (`cmpi.l #n,(A0); bne.s'), each
          `then' part branches to the join with `bra.s', the joins of the
          three `if's coincide at the `unlk'; branch sizes `.s';
  - 12.2  a by-address parameter used more than once is loaded into A0
          once, at entry (0xE58620), and A0 holds it for the whole body;
  - 13    `cmpi.l', `move.l #imm,(A0)'.

  Reconstructed (not known from the original): the names (the routine's
  from the map, the status constants from ../source); status_$t as
  integer32; `var' as the mode (`in out' compiles the same, ref 5.2
  Table 5-1); the `else if' chain (three independent `if's would compile
  differently: each `then' part would fall through to the next test). }
module name;
const
  status_$no_right_to_perform_operation = 16#00230001;
  status_$insufficient_rights_to_perform_operation = 16#00230002;
  status_$acl_wrong_type = 16#00230004;
  status_$naming_no_rights = 16#000E0013;
  status_$naming_insufficient_rights = 16#000E0014;
  status_$naming_branch_is_not_a_directory = 16#000E000E;
type
  status_$t = integer32;

procedure name_convert_acl_status(var status: status_$t);
begin
  if status = status_$no_right_to_perform_operation then
    status := status_$naming_no_rights
  else if status = status_$insufficient_rights_to_perform_operation then
    status := status_$naming_insufficient_rights
  else if status = status_$acl_wrong_type then
    status := status_$naming_branch_is_not_a_directory;
end;
