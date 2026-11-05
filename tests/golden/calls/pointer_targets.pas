{ Golden: codegen-patterns 1.5 ("An object reached through a pointer",
  bead domain-pascal-s9j): arguments passed by address that lie in or
  are the target of an object a pointer points to.
  - `p.f^', the record held in an address register since the entry: the
    pointer field pushed, `move.l (d,A2),-(SP)' (0xE1CC52 in KBD_$RCV's
    nested 0xE1CC10;
    TTY_$I_OUTPUT_BUFFER_DRAINED 0xE1B39E..0xE1B3B2, whose shape
    tty_$drained has: `bclr', the push, `bsr.w' to an internal routine,
    no pop before the epilogue);
  - a field through a by-address parameter's slot not held: `movea.l
    (8,A6),A0; pea (d,A0)' (KBD_$OUTPUT_BUFFER_DRAINED 0xE1CE96..
    0xE1CEAA, the whole routine);
  - a field of the enclosing routine's by-address parameter, from a
    nested routine: the slot reloaded through the static link for each
    call, `movea.l (8,A2),A0; pea (d,A0)' (0xE1AFB8, 0xE1AFCC in the
    nested routine at 0xE1AFA2 of TTY_$I_PUT_CHARS);
  - through a pointer local in the frame (here a nested routine reaches
    it; a pointer local is otherwise held in a register, codegen-patterns
    12.5): `movea.l (d,A6),A0; pea
    (d,A0)' (AST_$PURIFY 0xE0599E); its own target `q^' is the pointer
    pushed, `move.l (d,A6),-(SP)' (module-local 0xE53A18's 0xE53DB0
    to FILE_$FW_FILE);
  - through a pointer in the unit's data: `movea.l (d,A5),A0; pea
    (d,A0)' (AUDIT_$LOG_EVENT_S 0xE70E88). }
module ptargets;
type
  ec_t = record
    value: integer32;
    head, tail: univ_ptr;
  end;
  ec_p = ^ec_t;
  flag_t = (f_output_wait, f_input_wait, f_2, f_3);
  desc_t = record
    pad1: array[1..4] of integer;
    flags: set of flag_t;                 { 0x08 }
    pad2: array[1..16] of integer32;      { 0x0a }
    pad3: integer;                        { 0x4a }
    ec: ec_t;                             { 0x4c }
    pad4: array[1..9] of integer32;       { 0x58 }
    key_ec: ec_p;                         { 0x7c }
  end;
  desc_p = ^desc_t;
  node_t = record
    next: desc_p;
    pad: integer32;
  end;
  node_p = ^node_t;
var
  current: desc_p;

procedure ec_$advance_without_dispatch(var ec: ec_t); extern;
procedure ec_$put(in w: integer; var ec: ec_t); extern;

procedure advance(var ec: ec_t); internal;
begin
  ec_$advance_without_dispatch(ec);
end;

procedure ptargets_$drained(in out d: desc_t);
begin
  d.flags := d.flags - [f_output_wait];
  advance(d.key_ec^);
end;

procedure ptargets_$buffer_drained(var d: desc_t);
begin
  ec_$advance_without_dispatch(d.ec);
end;

procedure ptargets_$outer(var d: desc_t; w: integer);
  procedure put_two;
  begin
    ec_$put(0, d.ec);
    ec_$put(0, d.ec);
  end;
begin
  if w > 0 then
    put_two;
end;

procedure ptargets_$local(var n: node_t);
var
  p: desc_p;
  q: ec_p;
  procedure forget; begin p := nil; q := nil; end;
begin
  p := n.next;
  ec_$advance_without_dispatch(p^.ec);
  q := p^.key_ec;
  ec_$advance_without_dispatch(q^);
end;

procedure ptargets_$static;
begin
  ec_$advance_without_dispatch(current^.ec);
end;
