{ Golden: `with' (ref "With", pp. 4-211..4-214; C.3.18) and `addr' as a
  value (ref "Addr", p. 4-13); codegen-patterns 5.2, "`with`", 1.5.

  - over a variable or a `var' parameter (the alias form too): the
    references as written, no code;
  - over `p^' for a pointer in the unit's data: the pointer loaded once
    into an address register, which the body's references use, past an
    assignment to `p' (p. 4-212: "changing p does not affect access to
    the record");
  - over an element of an array a pointer points to: the scaled index,
    the pointer, `lea (0,An,Dx.w),Am', the lower bound folded into each
    reference's displacement (NETLOG_$LOG_IT 0xE71BB6..0xE71BFC: ×26
    into D1, `movea.l (116,A5),A0; lea (0,A0,D1.w),A0', the field stores
    at `(-26,A0)' .. `(-2,A0)'); held in a callee-saved register across a
    call (codegen-patterns 12.4 R3);
  - over `q^' for a by-value pointer parameter the body does not change:
    through `q' as written (loaded at entry, 12.2);
  - `addr(x)' stored: `lea; move.l An,<dst>' (0xE00964 `lea (24,A5),A0;
    move.l A0,(-108,A6)'); passed by value: `pea' (1.5). }
module withs;

type
  rec = record
    kind: char;
    pid: char;
    t: integer32;
    u: integer;
  end;
  rec_p = ^rec;
  recs = array[1..39] of rec;

var
  g: rec;
  p: rec_p;
  bufp: ^recs;
  n: integer;
  c: char;
  l: integer32;

procedure take(v: rec_p); options(val_param); extern;
procedure poke(var w: integer); extern;

procedure w_$var(var v: rec);
begin
  with g do begin
    kind := c;
    u := 3;
  end;
  with v : r do
    r.t := l;
end;

procedure w_$ptr;
begin
  with p^ do begin
    t := l;
    p := addr(g);
    u := 3;
  end;
end;

procedure w_$elem(k: char); options(val_param);
begin
  with bufp^[n] do begin
    kind := k;
    t := l;
    pid := c;
    poke(u);
    u := 5;
  end;
end;

procedure w_$param(q: rec_p); options(val_param);
begin
  with q^ do begin
    t := l;
    u := 3;
  end;
  take(addr(g));
end;
