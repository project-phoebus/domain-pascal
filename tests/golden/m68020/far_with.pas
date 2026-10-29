{ codegen-patterns 14 ("Symbols far from an array's first element");
  bead domain-pascal-2ys5.  With `-cpu mathchip', a `with' over an
  element of another unit's array indexed from far beyond 16 bits (the
  SAU9 MMAP, `array[16#10000..16#1FFFF]' of 16-byte entries) biases the
  symbol by the multiple of 60000 nearest the element's offset and keeps
  the rest in the fields' displacements: SAU9 AST_$PTOV 0x3C408712
  `movea.l #MMAP-1020000,A0; lsl.l #4,D1; lea (0,A0,D1.l),A4; tst.b
  (-28571,A4)', then `(-28567,A4)', `(-28574,A4)' (-1020000 = -0x100000
  + 28576).  The MC68010 code folds the whole offset into the
  displacements (MMAP_$AVAIL 0xE0CC76) and refuses one beyond 16 bits. }
module far_with;
type
  entry_t = packed record
    pad: 0..255;
    kind: 0..255;
    count: integer;
    link: integer32;
    stamp: integer32;
    flag: boolean;
    ok: boolean;
    rest: integer;
  end;
var
  m: extern array[16#10000..16#1FFFF] of entry_t;

procedure t(vpn: integer32; out r: integer); options(val_param);
begin
  with m[vpn] do
    if flag and then (not ok) and then (count <> 0) then r := count;
end;
