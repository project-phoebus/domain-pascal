{ Golden: codegen-patterns 7.5, `goto' and labels (ref "Goto",
  pp. 4-95..4-97; bead domain-pascal-o4q).
  - forward: `goto' is a `bra' to the label, `if c then goto l' one
    conditional branch on `c' to it (7.1: no conditional branch over a
    `bra' in compiled code); a forward-only label is a join, not
    aligned (7, "Other branch targets");
  - backward: the label heads a loop and gets the 0x2048 filler in an
    aligning routine (the backward-`goto' targets of ../source's C in
    aligning routines: NETWORK_$READ_AHEAD 0xE0FDA0, the REM_FILE
    module-local 0xE60FD8 at 0xE61130 and 0xE611E0 with the filler,
    BAT_$ALLOCATE 0xE3B294, FLP_DO_IO 0xE3DE64, PROC2_$WAIT 0xE3FE84
    4-aligned);
  - `else goto': a branch on the negated condition to the label, then
    the `then' part (TTY_$I_RCV 0xE1BBCE..0xE1BBD2 `btst #3,D1; beq.s
    0xE1BC38', its crlf arm's `else goto normal_char', a label on
    another arm of the `case', Appendix D p. D-4);
  - out of a `while' and a `for' loop; with `and then'; to a label
    before the epilogue, where a function's result register is still
    the one its last assignment on each path set. }
module gt;
var
  w: extern integer;
  b: extern boolean;
  r: extern integer;

procedure ext_$p(x: integer); options(val_param, extern);

procedure gt_$forward; internal;
label done;
begin
  w := 0;
  if b then goto done;
  r := 1;
  ext_$p(2);
done:
  r := 3;
end;

procedure gt_$backward; internal;
label again;
begin
  w := 0;
again:
  ext_$p(w);
  if r <> 0 then goto again;
  w := 1;
end;

procedure gt_$else_goto; internal;
label tail;
begin
  if w = 1 then
    if b then
      r := 5
    else
      goto tail
  else begin
    r := 6;
    ext_$p(1);
  end;
  w := 2;
tail:
  r := 7;
end;

procedure gt_$arms; internal;
label common;
begin
  case w of
    0: r := 1;
    1: if b then r := 2 else goto common;
    2: r := 3;
    3: common: begin r := 4; ext_$p(5) end;
  end;
end;

procedure gt_$out_while; internal;
label found;
begin
  r := 0;
  while w < 10 do begin
    if r = w then goto found;
    w := w + 1;
  end;
  r := 1;
found:
  ext_$p(r);
end;

procedure gt_$out_for; internal;
label bad;
var i: integer;
begin
  for i := 1 to 10 do begin
    ext_$p(i);
    if b then goto bad;
  end;
  r := 1;
  return;
bad:
  r := 2;
end;

procedure gt_$and_then; internal;
label skip;
begin
  if (w = 1) and then b then goto skip;
  r := 1;
skip:
  ext_$p(2);
end;

function gt_$result: integer; internal;
label done;
begin
  gt_$result := 1;
  if b then goto done;
  ext_$p(3);
  gt_$result := 2;
done:
end;
