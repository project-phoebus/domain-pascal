{ Golden: codegen-patterns 13, "Store merging" (bead domain-pascal-qb9q):
  a unit built with --merge-constant-stores (the .args file) stores two
  word constants to adjacent words, ascending, as one longword.  reset has
  the shape of TTY_$K_RESET 0xE67322..0xE67344 (A2 the descriptor):
  `move.l #0x10001,(0x3d2,A2); move.w #0x100,(0x3d6,A2)' for a run of
  three words (the first two merged, as KBD_$INIT 0xE333B6 does), then
  `clr.l (0x56,A2)' for two zeros; log_init of LOG_$INIT 0xE3017C (`moveq
  #1,D0; move.l D0,(A0)' for head := 0; tail := 1, the merged value in
  moveq's range; whether the original loads the pointer before or after
  the moveq is not shown, LOG_$INIT's A0 being loaded before the branch to
  0xE3017C); open of APP 0xE00BA8 (`move.l #0x498fc02,(-40,A6)', the
  two words of a frame record at 0 mod 4).  A byte and a word, and a
  longword below a word, stay two stores (not one reference, or the same
  two stores either way): pair_kinds.  The shapes the merging units keep
  apart are two stores too: globals has SMD's (0xE6D824 `clr.w
  (204,A5); clr.w (206,A5)', words off A5, in the routine that merges
  through a pointer at 0xE6D80E; 0xE6D838 `st (5950,A5); st (5951,A5)',
  bytes); flags of ACL_ 0xE44EC6 (`clr.b (40,A2); clr.b (41,A2)'); down of
  FLP_DO_IO 0xE3DE4A (`move.w #25,(306,A5); move.w #500,(304,A5)',
  descending). }
module merged;
type
  buf_t = record
    head, tail, size: integer;
  end;
  desc_t = record
    pad: array [1..43] of integer;     { 0x56 bytes }
    saved_flags: integer;              { 0x56 }
    column: integer;                   { 0x58 }
    gap: array [1..444] of integer;
    output: buf_t;                     { 0x3D2 }
  end;
  log_t = record
    head, tail: integer;
  end;
  opt_t = record
    socket, flags: integer;
  end;
  kinds_t = record
    b: boolean;                        { 0 }
    w: integer;                        { 2 }
    l: integer32;                      { 4 }
    w2: integer;                       { 8 }
  end;
  hdr_t = record
    w: integer;
    f1, f2: boolean;                   { 2, 3 }
  end;
procedure reset(var d: desc_t);
begin
  d.output.head := 1;
  d.output.tail := 1;
  d.output.size := 256;
  d.saved_flags := 0;
  d.column := 0;
end;
procedure log_init(var l: log_t);
begin
  l.head := 0;
  l.tail := 1;
end;
procedure use_opt(var o: opt_t); extern;
procedure open;
var
  pad: array [1..18] of integer;
  opt: opt_t;                          { -40 }
begin
  opt.socket := 1176;
  opt.flags := -1022;
  use_opt(opt);
end;
procedure pair_kinds(var k: kinds_t);
begin
  k.b := true;
  k.w := 0;
  k.w2 := 7;
  k.l := 5;
end;
var
  cursor: record
    line, column: integer;            { 0, 2 }
  end;
  hi, lo: integer;                    { 4, 6 }
  modes: record
    f1, f2: boolean;                  { 8, 9 }
  end;
procedure globals;
begin
  cursor.line := 0;
  cursor.column := 0;
  modes.f1 := true;
  modes.f2 := true;
end;
procedure flags(var h: hdr_t);
begin
  h.f1 := false;
  h.f2 := false;
end;
procedure down;
begin
  lo := 25;
  hi := 500;
end;
