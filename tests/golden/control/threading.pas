{ Golden: codegen-patterns 7.1 ("Branch threading"), 7.2 (bead
  domain-pascal-o6p).  Compiled code has no branch to a `bra'
  (tools/threadsurvey.py: none of 0xE00800..0xE88834), and 845 forward
  branches go past a `bra' to its target.

  set_flag: the shape of TTY_$K_SET_FLAG 0xE67454..0xE67482:

    tst.b (A1); bpl.b 0xE6747C     if value then begin
    bset ...                         state_flags + [...]
    cmp.w ...; beq.b 0xE67482        if ... then  (past the `bra')
    ...; jsr 0xE1B824                  tty_$i_signal(...)
    bra.b 0xE67482                 end  (the epilogue)
    0xE6747C: bclr ...             else state_flags - [...]

  else_while: an `else' part that starts with a `while' is entered at
  the loop's test, and the loop's opening `bra' after the `then' part's
  `bra' is dropped (TTY_$K_GET 0xE1C44A `bpl.w 0xE1C4FC', the head
  0xE1C4D6 right after the `then' part); the head is still aligned (the
  module-local 0xE4AFA8: `bra.s' at 0xE4B0F4, the filler at 0xE4B0F6,
  the head 0xE4B0F8).

  nested: the inner `then' part's `bra' goes to the outer join, and an
  inner `else' part ending in `return' leaves the outer `bra' unreached:
  dropped (no unreached `bra' in the image). }
module thr;
type
  flags_t = set of 0..15;
var
  flags: extern flags_t;
  head, tail: extern integer;
  w, r: extern integer;

procedure sig(code: integer); options(val_param, extern);

procedure set_flag(in value: boolean);
begin
  if value then begin
    flags := flags + [4];
    if head <> tail then
      sig(26);
  end
  else flags := flags - [4];
end;

procedure else_while;
begin
  if w = 0 then r := 0
  else while r > 0 do r := r - 1;
  sig(1);
end;

procedure nested;
begin
  if w = 1 then
    if r = 1 then w := 2 else begin r := 3; return end
  else r := 30;
  sig(2);
end;
