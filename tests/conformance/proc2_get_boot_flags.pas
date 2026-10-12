{ PROC2_$GET_BOOT_FLAGS, 0xE41B56, 0x1C bytes (tools/imgdis.sh E41B56 1C).
  Semantics: ../source/domain_os/proc2/get_boot_flags.c; data layout:
  proc2_$unwired_data_t in ../source/domain_os/proc2/proc2.h (its
  _Static_asserts: proc_dir_uid 0, system_uid 8, uid[0..57] 0x10,
  info_alloc_ptr 0x1E0, free_list_head 0x1E2, boot_flags 0x1E4; map
  `D E7BE84 PROC2 size = 1E8').

  A fragment of the PROC2 module (code segment 0xE3E748), compiled with
  `dpc --fragment-origin 0xE41B56'; the data area `.bss.moddata.PROC2' is
  placed at 0xE7BE84 by the manifest.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2  default convention: the out parameter's address at (8,A6);
  - 4    `pea (A5)', `lea <PROC2 data>.l,A5', restore `movea.l
         (-4,A6),A5';
  - 13   a one-statement body storing a module variable through the
         parameter's address: `movea.l (8,A6),A0' where the store needs
         it, then memory to memory `move.w (0x1e4,A5),(A0)'.

  Reconstructed (not known from the original): the names; the field
  types from the C (uid_t as a record of two integer32, the two
  uint16_t words as 0..65535, boot_flags as integer); `out' as the mode
  (`var' compiles the same, ref 5.2 Table 5-1).  The data area declares
  the fields up to boot_flags only (next_upid at 0x1E6, initialised to
  0x41 in the image, is left out); they are declared to put boot_flags at
  0x1E4 (dpc-sema lays statics out in declaration order). }
module proc2;
type
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
var
  proc_dir_uid: uid_$t;                 { 0x000 }
  system_uid: uid_$t;                   { 0x008 }
  uid: array[0..57] of uid_$t;          { 0x010, per ASID }
  info_alloc_ptr: 0..65535;             { 0x1E0 }
  free_list_head: 0..65535;             { 0x1E2 }
  boot_flags: integer;                  { 0x1E4 }

procedure proc2_$get_boot_flags(out flags: integer);
begin
  flags := boot_flags;
end;
