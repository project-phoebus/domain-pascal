{ Golden: type transfer functions (ref "Type Transfer Functions",
  pp. 4-197..4-199) and pointer arithmetic through them (codegen-patterns
  5.2, "Pointer arithmetic").  A transfer between types of one size emits
  no code; the integer arithmetic is that of section 8, and the sum moves
  to an address register only where it is used as one:
  - LOG_$READ's worker 0xE177A6..0xE177DA: a `0..65535' offset
    zero-extended, the pointer added from memory (`add.l (0x14,A5),D3'),
    `movea.l D3,A1' for the use as a pointer (here a stored `univ_ptr');
  - a constant above 8 loaded first, the pointer added (0xE00AB4 `moveq
    #30,D0; add.l (-64,A6),D0; movea.l D0,A2'; 0xE196DE, 0xE1CF0A,
    0xE7140E); 1..8 with `addq'/`subq' after a copy of the pointer
    (0xE4D5A2 `move.l A2,D0; addq.l #1,D0'; 0xE028DE `move.l A2,D0;
    subq.l #4,D0');
  - a sign-extended word plus a pointer, then `movea.l' (0xE4EFCA..
    0xE4EFD2 `move.w (14,A3),D4; ext.l D4; add.l A3,D4; movea.l D4,A0';
    0xE538B8..0xE538BE); here into a list walk's pointer register;
  - between integer types a transfer converts like an assignment (p. 4-197,
    "integer subranges are always compatible regardless of their
    sizes"): a `0..65535' word zero-extended (0xE177A6); between an
    enumeration and an integer (p. 4-199, `CAR_MANUFACTURERS(ordinal...)')
    nothing. }
module tt;
type
  pint = 0..65535;
  colour = (red, green, blue);
  rec = record
    a, b: integer32;
    next: ^rec;
  end;
  rptr = ^rec;
var
  base: rptr;
  q: univ_ptr;
  l: integer32;
  w: integer;
  u: pint;
  c: colour;

procedure tt_$offset(offset: pint); internal;
begin
  q := univ_ptr(integer32(base) + offset);
end;

procedure tt_$consts;
begin
  q := univ_ptr(integer32(base) + 30);
  q := univ_ptr(integer32(base) + 1);
  q := univ_ptr(integer32(base) - 4);
  l := integer32(base);
  base := rptr(l);
  l := integer32(u);
  w := integer(u);
  c := colour(w);
  w := integer(c);
end;

procedure tt_$walk;
var
  p: rptr;
begin
  p := univ_ptr(integer32(base) + w);
  while p <> nil do
    p := p^.next;
end;
