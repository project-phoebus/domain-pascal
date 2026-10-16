{ Golden: codegen-patterns 5.2 ("Word indices used as `.l'"); bead
  domain-pascal-wkg, survey `tools/arithsurvey.py wordidx'.

  Into an array whose bounds times the element size do not fit in 16
  bits, a variable index whose type's range does not fit either is
  widened by its sign, scaled at a longword's width and used as a `.l'
  index; the arrays are other units' (`movea.l #sym,An' after the index's
  value, before its scaling).
  - by_area_id: AREA_$DELETE 0xE07C34..0xE07C5E: `area_id' (`pinteger',
    AREAS `array[1..16#540]' of 48 bytes, ../source area/area.ins.pas)
    loaded into D0 and tested, then `clr.l D1; movea.l #0xD94C00,A0;
    move.w D0,D1; subq.l #2,SP; lsl.l #4,D1; move.l D1,D0; add.l D0,D0;
    add.l D0,D1' (the base between the `clr.l' and the copy: 44 of 44
    such zero-extensions of a word held in a register).  A
    zero-extension read from memory (a parameter's slot, through a
    pointer) is refused: the base's place there is not settled.
  - by_local: the same from a register local (k, held in D1).
  - by_id: AREA_'s module-local 0xE075CA at 0xE076DC..0xE076E8 `move.w
    D2,D3; ext.l D3; lsl.l #4,D3; ...', an `integer'.
  - by_228 / by_150: `muls.w #228' of a signed word (module-local
    0xE3EB8C at 0xE3EBB2..0xE3EBBA, its parameter held in D2), `mulu.w
    #150' of an unsigned one (0xE54D76..0xE54D7C), each on a copy of the
    loaded value.
  - by_char: no scaling, the zero-extended word itself (DXM_WIRED_
    0xE17088..0xE1708C `clr.l D0; move.w D1,D0; move.b
    (-1,A1,D0.l),D6'), from a register local. }
module wi;

type
  pinteger = 0..65535;
  area_t = record z: integer32; pad: array[1..22] of integer; end;
  r228 = record z: integer32; pad: array[1..112] of integer; end;
  r150 = record z: integer; pad: array[1..74] of integer; end;
var
  areas: extern array[1..16#540] of area_t;
  n_areas: extern pinteger;
  t228: extern array[1..300] of r228;
  t150: extern array[0..300] of r150;
  chars: extern array[1..40000] of char;

function by_area_id(area_id: pinteger): integer32;
  options(val_param);
begin
  if (area_id = 0) or (area_id > n_areas) then
    by_area_id := 0
  else
    by_area_id := areas[area_id].z;
end;

function by_local(i: pinteger): integer32;
  options(val_param);
var
  k: pinteger;
begin
  k := i;
  by_local := areas[k].z;
end;

function by_id(id: integer): integer32;
  options(val_param);
begin
  by_id := areas[id].z;
end;

function by_228(i: integer): integer32;
  options(val_param);
begin
  by_228 := t228[i].z;
end;

function by_150(i: pinteger): integer;
  options(val_param);
begin
  by_150 := t150[i].z;
end;

function by_char(i: pinteger): char;
  options(val_param);
var
  k: pinteger;
begin
  k := i;
  by_char := chars[k];
end;
