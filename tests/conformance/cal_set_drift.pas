{ CAL_$SET_DRIFT, 0xE68580..0xE6859A (0x1A bytes; tools/imgdis.sh
  E68580 1A).  Semantics: ../source domain_os/cal/set_drift.c; the
  record layout: cal_$timezone_rec_t in ../source domain_os/cal/cal.h
  (drift, a clock_t of 6 bytes, at 0x06 of the 12-byte record;
  _Static_asserts).  CAL_$TIMEZONE is the map's name at 0xE7B030 (`D
  E7B030 OS_CAL_WIRED'), reached by absolute address from this code
  section.

  Patterns exercised (docs/codegen-patterns.md):
  - 5.2  a 6-byte record assignment: two moves memory to memory, `move.l
         (A0),abs.l; move.w (4,A0),abs+4.l' (0xE68588, 0xE6858E), the
         `in' parameter's slot loaded into A0 (bead domain-pascal-fbg);
  - 2    `link.w A6,#0', nothing saved.

  Reconstructed (not known from the original): the names of the types
  and fields (../source's); the parameter's mode, `in' (read through the
  slot and never copied: a value parameter would be copied into the
  frame, codegen-patterns 1.2); CAL_$TIMEZONE declared `extern' here,
  as another unit's object (the CAL module that defines it is not
  compiled). }
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

procedure cal_$set_drift(in drift: cal_$clock_t);
begin
  cal_$timezone.drift := drift;
end;
