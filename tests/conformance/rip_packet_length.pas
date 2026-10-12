{ Module-local routine at 0xE68864 (the first routine of the map's
  RIP_UNWIRED module), 0x16 bytes: 0xE68864..0xE6887A
  (tools/imgdis.sh E68864 16).  Semantics: ../source/domain_os/rip/server.c,
  where it is called RIP_$PACKET_LENGTH (a C name; the map does not name the
  routine): entry_count * 6 + 2, computed in 16 bits.

  The unit is a fragment of RIP_UNWIRED (its first 0x16 bytes), so
  tools/conform.py compiles it with `dpc --fragment-origin 0xE68864'.  No
  loops, no module end: no loop-alignment option.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2  internal: the word parameter by value at (8,A6);
  - 2    link.w A6,#-8 (see below), no register save;
  - 3    the integer16 result computed straight into D0;
  - 8    multiplication by the constant 6 = 2 x 3 with shifts and adds:
         `add.w D0,D0' for the factor 2, then a copy, `add.w D1,D1' and
         `add.w D1,D0' for the factor 3 (0xE6886C..0xE68872); `+ 2' as
         `addq.w #2,D0' (0xE68874);
  - 12.4 the partial product in a new register (D1, R1/R8).

  Reconstruction notes: the routine is `internal' here because the map does
  not name it and every call reaches it with `bsr' (RIP_$PROCESS_REQUEST,
  RIP_$SERVER).  The frame is 8 bytes and nothing in it is addressed
  (codegen-patterns 2, untouched frames): `stand_in' stands for whatever
  the original declared (the result variable takes 2 bytes, rounded to 4). }
module rip_unwired;
function rip_$packet_length(entry_count: integer): integer; internal;
var
  stand_in: integer32;
begin
  rip_$packet_length := entry_count * 6 + 2;
end;
