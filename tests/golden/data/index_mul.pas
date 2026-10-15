{ Golden: codegen-patterns 5.2 ("Index scaling"), 8 ("Multiplication by
  a constant"); bead domain-pascal-r3yy, survey `tools/arithsurvey.py
  scale'.

  A scaling or product by a constant of 1..127 done with a multiply
  (an odd part that is not 2^k +- 1): the constant first, `moveq #c,Dn;
  mul<s>.w x,Dn', the variable the source operand.
  - with_unit: TPAD_$SET_PUNCH_IMPACT 0xE69B0E..0xE69B1E `moveq #44,D2;
    mulu.w (A3),D2; lea (0,A5,D2.w),A0; move.w (-4,A0),D0; movea.l
    (12,A6),A1; move.w (A1),(-4,A0)' (`with tpad_$unit_configs[unitp]',
    unitp by address; here not held, so its slot is loaded after the
    `moveq', TPAD_$INQUIRE_UNIT 0xE6999C..0xE699A2; the other slots'
    loads placed by the scheduler, 12.4 R11).
  - by_unit_var: TPAD_$DATA 0xE691CA..0xE691D0 `moveq #44,D0; ...; muls.w (380,A5),D0'
    (there a `.l' index; here a `.w' one, `mulu.w').
  - by_value: module-local 0xE383B0 at 0xE383C8..0xE383D2 `move.w
    (12,A6),D2; moveq #100,D0; mulu.w D2,D0; lea (0,A5,D0.w),A3'.
  - other_unit: RIP_$TABLE 0xE6904C..0xE69056 `moveq #92,D1; movea.l
    #sym,A0; mulu.w D0,D1; lea (0,A0,D1.w),A4' (the base after the
    `moveq'), and PKT_$BLD_INTERNET_HDR 0xE121E6..0xE121F2 (the
    parameter's slot loaded after the `moveq').
  - word_product: module-local 0xE4519C at 0xE458CC..0xE458D4 `moveq
    #44,D3; muls.w D2,D3; addi.w #52,D3' (a signed word).
  - long_product: TPAD_$DATA 0xE691D0 (`muls.w', the 32-bit product).
  - by_268: ×268 (odd part 67, three bits), `mulu.w #c' on the loaded
    value (0xE6D8EE; 261 of 261 constants of 128 and more). }
module im;

type
  cfg_t = record
    x: integer;
    pad: array[1..20] of integer;
    impact: integer;
  end;
  r100 = record a: integer32; pad: array[1..48] of integer; end;
  r92 = record a: integer32; pad: array[1..44] of integer; end;
  r268 = record a: integer; pad: array[1..133] of integer; end;
var
  cfgs: array[1..8] of cfg_t;
  vt: array[0..7] of r100;
  big: array[0..7] of r268;
  unit_no: integer;
  rt: extern array[0..19] of r92;

procedure with_unit(in u: integer; in v: integer; out old: integer);
begin
  with cfgs[u] do begin
    old := impact;
    impact := v;
  end;
end;

function by_unit_var: integer;
begin
  by_unit_var := cfgs[unit_no].x;
end;

function by_value(i: integer): integer32;
  options(val_param);
begin
  by_value := vt[i].a;
end;

function other_unit(in i: integer): integer32;
begin
  other_unit := rt[i].a;
end;

function word_product(i: integer): integer;
  options(val_param);
begin
  word_product := i * 44 + 52;
end;

function long_product: integer32;
begin
  long_product := unit_no * 44;
end;

function by_268(i: integer): integer;
  options(val_param);
begin
  by_268 := big[i].a;
end;
