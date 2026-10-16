{ DISK_$GET_ERROR_INFO, 0xE6C08C..0xE6C0A8 (0x1C bytes; tools/imgdis.sh
  E6C08C 1C).  Semantics: ../source domain_os/disk/get_error_info.c; the
  record layout: disk_$error_info_t in ../source
  domain_os/disk/disk_internal.h (86 bytes; _Static_asserts).
  DISK_$ERROR_INFO is the map's name at 0xE7AC60, reached by absolute
  address from this code section (disk_vol.pas's .TEXT).

  Patterns exercised (docs/codegen-patterns.md):
  - 5.2  "Block addresses": another unit's record as a block copy's
         source, `movea.l #DISK_$ERROR_INFO,A0' (0xE6C090), the source
         first, then the by-address parameter's slot, `movea.l
         (8,A6),A1'; an 86-byte copy, a `dbf' loop of 21 longwords
         (`moveq #0x14,D0') and a final `move.w (A0)+,(A1)+' (bead
         domain-pascal-21xu);
  - 7    the loop head at 0xE6C09C, 4-aligned without a filler;
  - 2    `link.w A6,#0', nothing saved.

  Reconstructed (not known from the original): the names of the types
  and fields (../source's); the parameter's mode, `out' (written whole,
  never read); DISK_$ERROR_INFO declared `extern' here, as another unit's
  object. }
module disk;
type
  disk_$error_info_t = record
    timestamp: integer32;                 { 0x00 }
    daddr: integer32;                     { 0x04 }
    geometry: integer32;                  { 0x08 }
    info: array[1..8] of integer32;       { 0x0c }
    header: array[1..8] of integer32;     { 0x2c }
    ppn: integer32;                       { 0x4c }
    status: integer32;                    { 0x50 }
    vol_idx: integer;                     { 0x54 }
  end;
var
  disk_$error_info: extern disk_$error_info_t;

procedure disk_$get_error_info(out buffer: disk_$error_info_t);
begin
  buffer := disk_$error_info;
end;
