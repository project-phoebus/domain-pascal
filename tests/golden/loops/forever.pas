{ Golden: codegen-patterns 7.2, loops with no test (`repeat ... until
  false', ref p. 4-42, the `close' example).  The head is entered by falling
  through and aligned like any loop head; the bottom is a `bra' back to
  it (MEM_LITES 0xE0C3D2..0xE0C410: the filler, `beq.b' to the code after
  the loop for the `if ... then exit', `bra.b 0xE0C3D4').  `next' goes to
  the head (PKT_$SAR_INTERNET 0xE720C0 `bne.w 0xE71FE8').  An `if c then
  exit' that ends the body is one branch back on the negated condition
  (0xE72152 `cmp.w D4w,D2w; bne.w 0xE71FE8', then the code after the
  loop); an `exit' that ends it leaves no `bra' (MSG_$SARI 0xE59F76..
  0xE59F9E: the last arm's code, then the code after the loop).  A loop
  with no way out ends the routine: no epilogue (DXM_$HELPER_COMMON
  0xE1721C `bra.b 0xE171FC', then the next routine at 0xE1721E). }
module fev;
var
  w: extern integer;
  b: extern boolean;
  r: extern integer;

procedure ext_$p(x: integer); options(val_param, extern);

procedure fev_$mid; internal;
begin
  w := 0;
  repeat
    ext_$p(1);
    if b then exit;
    ext_$p(2);
  until false;
  r := 1;
end;

procedure fev_$last_if; internal;
begin
  repeat
    ext_$p(1);
    if not b then next;
    r := 2;
    if w = 3 then exit;
  until false;
  r := 1;
end;

procedure fev_$last_exit; internal;
begin
  repeat
    ext_$p(1);
    if w = 3 then next;
    r := 2;
    exit;
  until false;
  r := 1;
end;

procedure fev_$no_way_out; internal;
begin
  w := 0;
  repeat
    ext_$p(1);
    w := w + 1;
  until false;
end;
