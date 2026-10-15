{ Golden: codegen-patterns 5.2 ("Another unit's array", "Arrays through a
  pointer"); 12.4 R11.  Bead domain-pascal-rlnm; survey
  `tools/boolsurvey.py ebase'.

  An element of another unit's array: `movea.l #sym,An', the immediate
  the variable's symbol, the lower bound and field offsets in the
  displacement; the base generated after the index's value and before
  its scaling (a scaling with `neg' first, the scheduler then moving the
  base above its last `add').
  - is_two: module-local 0xE4649C..0xE464AA `move.w (8,A6),D0; movea.l
    #PROC1_$TYPE,A0; move.w D0,D1; add.w D1,D1; cmpi.w #2,(-2,A0,D1.w);
    sne D1; move.b D1,D0' (a 1-based array of words; byte-identical).
  - quit_ec_value: TTY_$I_WAIT 0xE1C22E `move.w PROC1_$AS_ID,D0; movea.l
    #FIM_$QUIT_EC,A0; lsl.w #2,D0; move.w D0,D1; add.w D1,D1; add.w
    D1,D0' (×12).
  - big: 0xE1AA50..0xE1AA62 `move.w D2,D3; lsl.w #3,D3; move.w D3,D0;
    neg.w D3; lsl.w #3,D0; movea.l #0xE2DC90,A1; add.w D0,D3; tst.l
    (0x2c,A1,D3.w)' (×56).
  - ktt: KBD_$FETCH_KEY 0xE1CBA0..0xE1CBB0 (`clr.w D0; move.b (A2),D0;
    ...; movea.l #SMD_$KTT,A0; move.b (0,A0,D0.w),(A2)', there in a
    loop and after a compare of the widened byte; here alone, the base in
    the zero-extension's stall).
  A 1-byte element through a pointer loaded from memory: the index, then
  the pointer (KBD_$FETCH_KEY 0xE1CB8A `clr.w D0; movea.l (0x34,A4),A1;
  move.b (A2),D0; move.b (0,A1,D0.w),(A2)'): through_ptr. }
module oui;
type
  ec_t = record value: integer32; a, b: integer32; end;
  big_t = record x: integer32; pad: array[1..13] of integer32; end;
  ktt_t = array[char] of char;
  ktt_ptr_t = ^ktt_t;
var
  proc1_$as_id: extern integer;
  proc1_$type: extern array[1..64] of integer;
  fim_$quit_ec: extern array[0..57] of ec_t;
  bigs: extern array[1..20] of big_t;
  smd_$ktt: extern ktt_t;
  ktt_ptr: extern ktt_ptr_t;

function is_two(i: integer): boolean;
  options(val_param);
begin
  is_two := proc1_$type[i] <> 2;
end;

function quit_ec_value: integer32;
begin
  quit_ec_value := fim_$quit_ec[proc1_$as_id].value;
end;

function big(i: integer): integer32;
  options(val_param);
begin
  big := bigs[i].x;
end;

function ktt(c: char): char;
  options(val_param);
begin
  ktt := smd_$ktt[c];
end;

function through_ptr(c: char): char;
  options(val_param);
begin
  through_ptr := ktt_ptr^[c];
end;
