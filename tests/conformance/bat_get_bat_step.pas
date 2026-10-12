{ BAT_$GET_BAT_STEP, 0xE3BB58, 0x26 bytes: 0xE3BB58..0xE3BB7E
  (tools/imgdis.sh E3BB58 26).  Semantics: ../source/domain_os/bat/
  get_bat_step.c (`return bat_$volumes[vol_idx].bat_step'); layout:
  bat/bat_internal.h (records of 0x234 bytes, the one for volume v at the
  BAT_ data block + (v-1) * 0x234, bat_step at +0x14).

  The unit is a fragment of the BAT_ module, so tools/conform.py compiles
  it with `dpc --fragment-origin 0xE3BB58'.  The data block is the map's
  `D E79478 BAT_' (manifest --section-addr).

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2  val_param: vol_idx by value in a word slot at (8,A6);
  - 2, 4 link.w A6,#-4 (the result variable), `pea (A5)', `lea
         <BAT_ data>,A5', restore from (-8,A6);
  - 5.2  an element of a 1-based array of 0x234-byte records in the
         unit's data: the index scaled with `mulu.w #0x234' (an odd part,
         0x8D, of four bits), `lea (0,A5,D1.w),A0' because the
         displacement, 0x14 - 0x234 = -0x220 (the lower bound folded in),
         does not fit in 8 bits, then `move.w (-0x220,A0),D0';
  - 8, 12.2 the by-value parameter loaded into D0, then copied into D1
         before the multiplication (0xE3BB64..0xE3BB6A);
  - 3    the word result loaded straight into D0.

  Reconstruction notes: the record is reduced to the field read; the
  other 0x232 bytes are padding here.  The C has `uint16_t'
  (bat_$volume_t.bat_step), so the field and the result are 0..65535
  (2 bytes, ref 5.3.4.1 p. 5-13). }
module bat_;
type
  bat_$step_t = 0..65535;
  bat_$volume_t = record
    header: array[1..10] of integer;     { 0x00..0x13 }
    bat_step: bat_$step_t;               { 0x14 }
    rest: array[1..271] of integer;      { 0x16..0x233 }
  end;
var
  bat_$volumes: array[1..6] of bat_$volume_t;   { 0x000..0xD37 }
function bat_$get_bat_step(vol_idx: integer): bat_$step_t; options(val_param);
begin
  bat_$get_bat_step := bat_$volumes[vol_idx].bat_step;
end;
