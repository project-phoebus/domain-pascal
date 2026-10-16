{ CAL_$GET_INFO, 0xE68568..0xE68580 (0x18 bytes; tools/imgdis.sh E68568
  18).  Semantics: ../source domain_os/cal/get_info.c; the record layout:
  cal_$timezone_rec_t in ../source domain_os/cal/cal.h (12 bytes;
  _Static_asserts).  CAL_$TIMEZONE is the map's name at 0xE7B030 (`D
  E7B030 OS_CAL_WIRED'), reached by absolute address from this code
  section.

  Patterns exercised (docs/codegen-patterns.md):
  - 5.2  "Block addresses": another unit's record as a block copy's
         source, `movea.l #CAL_$TIMEZONE,A0' (0xE6856C), then the
         by-address parameter's slot, `movea.l (8,A6),A1', and three
         `move.l (A0)+,(A1)+' (bead domain-pascal-21xu);
  - 2    `link.w A6,#0', nothing saved.

  Reconstructed (not known from the original): the names of the types
  and fields (../source's); the parameter's mode, `out' (written whole,
  never read); CAL_$TIMEZONE declared `extern' here, as another unit's
  object (the CAL module that defines it is not compiled). }
module cal;
type
  cal_$clock_t = record
    high: integer32;
    low: integer;
  end;
  cal_$timezone_rec_t = record
    utc_delta: integer;                   { 0x00 }
    tz_name: array[0..3] of char;         { 0x02 }
    drift: cal_$clock_t;                  { 0x06 }
  end;
var
  cal_$timezone: extern cal_$timezone_rec_t;

procedure cal_$get_info(out tz_rec: cal_$timezone_rec_t);
begin
  tz_rec := cal_$timezone;
end;
