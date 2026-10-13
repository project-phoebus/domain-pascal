{ Golden: codegen-patterns 12.4 R9-R11, the scheduler inside basic
  blocks (bead domain-pascal-g5q.8; crates/dpc-m68k/src/sched.rs, the
  default policy R11 with a window of 3).
  - remainder: `u mod 101' used as a longword (0xE382B4..0xE382CC): the
    `clr.l' of the result's zero-extension is moved into the stall after
    the dividend's load, before the `divu.w' that reads it (0xE382BE
    `clr.l D2').
  - stores: a store is ordered against everything (12.4 R10; 0xE3F2DE/
    0xE3F2E2): the `clr.l' of the next statement's zero-extension stays
    after the store although the `addq' stalls on the parameter's load.
  - pop_pad: a call's pop stays right after it, the next call's pad right
    after the pop, and the argument's address load after the pad
    (0xE1AFC8 `addq.w #8,SP; subq.l #2,SP; movea.l (8,A2),A0'). }
module schedule;
type
  pinteger = 0..65535;
  rec = record
    a: integer32;
    b: integer;
  end;
var
  u: pinteger;
  r: integer32;
  g: rec;
  k: integer32;

procedure take(x: integer); options(val_param, extern);

function remainder: integer32; internal;
begin
  remainder := u mod 101;
end;

procedure stores(p: integer32); options(val_param);
begin
  k := p + 1;
  r := u;
end;

procedure pop_pad(var y: rec);
begin
  take(5);
  take(y.b);
end;
