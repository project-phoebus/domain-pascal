{ Golden: codegen-patterns 1.5 "Constant by address", "To a `univ'
  parameter"; beads domain-pascal-9286, -c5u.

  A constant passed by address to a `univ' parameter keeps a cell of its
  own size, not extended to the parameter's nor cut to it:
  - a string: its characters, an odd count followed by one zero byte
    (the module-local 0xE2FF7C's format for VFMT_$WRITE2, cell 0xE2FFD4,
    37 characters and 00; 0xE30020, 36 characters, for a 4-byte
    parameter);
  - an integer constant for a non-integer parameter: a longword, the
    integer32 (PKT_$LIKELY_TO_ANSWER 0xE12A92 `pea (0x120,PC)', cell
    0xE12BB4 `0000 0000', for a 1024-byte buffer; XPD's 0xE7504E, cell
    0xE7508C, for a 4-byte status); for a parameter of more than 4
    bytes only for the value 0 (any value for a 4-byte one:
    calls/univ_small_const);
  - every 4-byte integer cell is keyed as integer32, so the 0 for an
    `[aligned(0)] integer32' parameter and the 0 for a non-integer one
    share a cell (0xE2FFFC, first named for VFMT_$WRITE2's a2 at
    0xE2FF8A, then for a1 and a2 at 0xE2FFA0, and by LOG_$INIT 0xE30214
    for LOG_$ADD's data), apart from a word 0 (RIP_$SERVER 0xE68E26,
    0xE68E28);
  - a character for a larger array: a word with it in the high byte
    (KBD_$INIT 0xE3338E, cell 0xE333DA `3200'; Reported). }
module ucon;
type
  fmt_t = array[1..200] of char;
  bytes_t = array[1..1024] of char;
  status_t = record all: integer32; end;
  arg_t = [aligned(0)] integer32;
  str4_t = array[1..4] of char;

procedure ext_$write2(in fmt: univ fmt_t; in a1, a2: univ arg_t); extern;
procedure ext_$send(in data: univ bytes_t; in len: integer; in s: univ status_t); extern;
procedure ext_$put(in c: univ str4_t; in n: integer); extern;

procedure ucon_$report;
begin
  ext_$write2('%/%/Warning: Status %lh, Unable to %$', 0, 0);
  ext_$write2(' %a -- error logging disabled.%.', '`node_data/system_logs/sys_error_log', 4);
end;

procedure ucon_$send;
begin
  ext_$send(0, 0, 0);
  ext_$put('2', 1);
end;
