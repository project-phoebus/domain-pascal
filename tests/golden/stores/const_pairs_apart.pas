{ Golden: codegen-patterns 13, "Store merging" (bead domain-pascal-qb9q):
  in a unit built without --merge-constant-stores, consecutive constant
  stores to adjacent objects are made one by one, as the image's units
  without a merge make them (`tools/storesurvey.py units`).  init_bufs has
  the shape of TTY_$I_INIT 0xE3332E..0xE33352 (seven words through the
  by-address descriptor, `move.w #1,(0x2ca,A4)' .., ascending: the words
  TTY_$K_RESET 0xE67322 merges in its own unit); clear_down of AREA_$INIT
  0xE2F3C0 (`clr.w (1506,A0); clr.w (1504,A0)', descending); flags of
  PKT_$BLD_INTERNET_HDR 0xE1217E (two bytes); cntl of NETLOG_$CNTL 0xE719A4
  (`clr.w (112,A5); clr.w (114,A5)', off A5). }
module apart;
type
  buf_t = record
    head, tail, size: integer;
  end;
  desc_t = record
    pad: array [1..357] of integer;   { 0x2CA bytes }
    input_head: integer;               { 0x2CA }
    input: buf_t;                      { 0x2CC }
    gap: array [1..128] of integer;
    output: buf_t;                     { 0x3D2 }
  end;
  area_t = record
    a: array [1..752] of integer;
    lo, hi: integer;                   { 1504, 1506 }
  end;
  hdr_t = record
    w: integer;
    f1, f2: boolean;                   { 2, 3 }
  end;
var
  cnt: array [1..56] of integer;
  c1, c2: integer;                     { 112, 114 }
procedure init_bufs(var d: desc_t);
begin
  d.input_head := 1;
  d.input.head := 1;
  d.input.tail := 1;
  d.input.size := 256;
  d.output.head := 1;
  d.output.tail := 1;
  d.output.size := 256;
end;
procedure clear_down(var a: area_t);
begin
  a.hi := 0;
  a.lo := 0;
end;
procedure flags(var h: hdr_t);
begin
  h.f1 := false;
  h.f2 := true;
end;
procedure cntl;
begin
  c1 := 0;
  c2 := 0;
end;
