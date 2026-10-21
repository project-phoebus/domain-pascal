{ Golden: codegen-patterns 1.5 "Constant by address", "To a `univ'
  parameter"; bead domain-pascal-45yc.

  An integer constant passed by address to a 4-byte non-integer `univ'
  parameter is a 4-byte cell holding its integer32 value, big-endian and
  sign-extended (ref 5.3.3 p. 5-11: the expression is extended to the
  size of the univ parameter; 3.2.3 p. 3-5: an integer constant is a
  32-bit integer).  SAU5 MBUS_$UNDEF_INT 0xE0AA0C: `link.w A6,#-4; pea
  (0xe,PC); jsr CRASH_SYSTEM; clr.b D0; unlk; rts', cell 0xE0AA20 `0000
  002a'.  The cells of equal values are shared with an integer32
  parameter's (every 4-byte integer cell is keyed as integer32). }
module usmall;
type
  status_$t = packed record
    case integer of
      1: (fail: boolean; subsys: 0..127; modc: 0..255; code: integer);
      2: (all: integer32);
  end;

const
  undef = 16#2A;

procedure crash_system(in status: univ status_$t); extern;
procedure ext_$put(in v: integer32); extern;

function mbus_$undef_int(v: integer32): boolean; options(val_param);
begin
  crash_system(16#2A);
  mbus_$undef_int := false;
end;

procedure usmall_$more;
begin
  crash_system(undef);
  crash_system(-1);
  crash_system(-32768);
  ext_$put(undef);
end;
