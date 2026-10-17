{ Golden: codegen-patterns 5.2 ("Base kept across statements"), 12.4 R7.
  Bead domain-pascal-lnzy; survey `tools/basesurvey.py pairs'.

  An element of another unit's array reached by an earlier statement as
  an operand `(d,An,Dx)', no call, loop or other address register's
  value since, the base's load dominating: a later statement's element
  of the same array reads the base's register again, its index formed
  as before without a base load (across statements 21 kept, none loaded
  again).  What each routine reproduces of the image, and what not:
  - clear_two: the one `movea.l #FIM_$TRACE_STS' for two `clr.l
    (0,An,Dx.w)' of separate statements, each index loaded and shifted
    (the module-local 0xE5B704 at 0xE5B744..0xE5B756); there the indices
    come from by-value slots into D2 and D3 and the base is A1 (the
    routine has other values live), here from other units' variables
    into D0.
  - handler: an element tested in an `if' whose `then' part returns,
    the same element read after it through the same base and the same
    ×56 index (R7) (SIO_$I_GET_DESC 0xE667E8..0xE66808 `movea.l
    #DTTE,A0; ...; tst.l (0x28,A0,D0.w); bne.s; ...; move.l
    (0x28,A0,D0.w),(-4,A6)'; there the index is a local held in a
    register and the result is stored to its slot).
  - across_if: the base loaded for an `if''s condition, read again after
    the `if''s join (as the module-local 0xE3ECEA at 0xE3ED20/0xE3ED8A,
    FIM_$QUIT_INH; there an `if ... else' whose parts load another
    array, here a `then' part alone). }
module kept_base;
type
  dtte_t = record
    ecs: array[1..9] of integer32;
    handler_ptr: univ_ptr;
    tty_handler: univ_ptr;
    alt_handler: univ_ptr;
    ptr_30: univ_ptr;
    discipline: integer;
    flags: boolean;
    pad_37: char;
  end;
var
  fim_$trace_sts: extern array[0..57] of integer32;
  fim_$quit_inh: extern array[0..57] of boolean;
  dtte: extern array[0..3] of dtte_t;
  last: extern univ_ptr;
  count: extern integer;
  as_one, as_two: extern integer;

procedure clear_two;
begin
  fim_$trace_sts[as_one] := 0;
  fim_$trace_sts[as_two] := 0;
end;

procedure handler(line: integer);
  options(val_param);
begin
  if dtte[line].tty_handler = nil then begin
    count := 0;
    return;
  end;
  last := dtte[line].tty_handler;
end;

procedure across_if(as_id: integer);
  options(val_param);
begin
  if not fim_$quit_inh[as_id] then
    count := count + 1;
  fim_$quit_inh[as_id] := true;
end;
