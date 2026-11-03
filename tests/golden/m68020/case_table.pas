{ Golden: `case' jump tables on the MC68020 (codegen-patterns 7.3; SAU8
  pbu_map_dma_error 0x3CA56F2..0x3CA5712, ../source
  pascal-translation-notes finding 701):
  - the index doubled by the scaled index `(T,PC,D0.w*2)', no `add.w
    D0,D0' (all 61 dispatches of the SAU8 and SAU9 images, `303b 0206';
    the SAU2's 50 use `add.w' and an unscaled index);
  - a longword selector less its lowest label, `subi.l #lo,D0', then
    `cmpi.l #n,D0; bcc' to the `otherwise' part (0x3CA56FC
    `subi.l #0x2E0001,D0; cmpi.l #5,D0; bcc').
}
module case_table;
var
  s: extern integer32;
  w: extern integer;
define map_status, pick;
procedure map_status(in st: integer32);
begin
  case st of
    16#2E0001: s := 1;
    16#2E0002: s := 2;
    16#2E0003: s := 3;
    16#2E0004: s := 4;
    16#2E0005: s := 5;
    otherwise
      s := 0;
  end;
end;

procedure pick;
begin
  case w of
    0: s := 10;
    1: s := 11;
    2: s := 12;
    3: s := 13;
  end;
end;
