{ Golden: codegen-patterns 12.4 R1-R3, the allocator of bead
  domain-pascal-g5q.5 (crates/dpc-m68k/src/regs.rs).
  - held_across: parameters loaded at entry (12.2) and live across calls
    take the lowest callee-saved registers, A2 and D2, saved with
    `movem.l'; objects through the held slot are pushed with `pea (d,A2)'
    and `pea (A2)'; module-local 0xE32B76 (sio/init_dtte.c,
    0xE32B7E..0xE32BAA).
  - third_word: a third data value takes D2 without any call, and D2 is
    saved (R1: 0xE177A0 `moveq #-2,D2' while D0, D1 hold parameters).
  - reuse: a parameter's register is free after its last use; the next
    statement's own register is the same A0 (R1: 0xE17778, D1/D0/A0
    reused at 0xE177A6, 0xE177D2, 0xE177E0).
  - dies_before_call: a value whose last use is before the call keeps a
    scratch register (R1/R3; R4(c) holds only values the call's
    arguments read).
  - pushed_last, status_last: in a body without branches, a parameter
    loaded at entry whose last use is an argument of a call is held
    through the call: D2 (R4(c): PROC1_$SET_ASID 0xE148FE, `move.w
    D2,-(SP)' at 0xE1490E), A2 (module-local 0xE1D592, `clr.l (A2); pea
    (A2)' at 0xE1D59C).
  - in_loop: a parameter read inside a `repeat' is live around the back
    edge, and across the call in the body: D2. }
module regalloc;
type
  ec_t = record
    value: integer32;
    head: univ_ptr;
    tail: univ_ptr;
  end;
  blk_t = record
    ec: ec_t;
    input_ec: ec_t;
    count: integer;
    flags: boolean;
  end;
  blk_ptr = ^blk_t;
  rec_t = record
    a: integer;
    pad1: integer;
    b: integer;
    pad2: integer;
    c: integer;
  end;
  rec_ptr = ^rec_t;
var
  other: extern rec_ptr;
  done: extern boolean;

procedure ec_$init(var ec: ec_t); extern;
procedure set_status(var st: integer32); extern;
procedure tick(n: integer); options(val_param, extern);

procedure held_across(var blk: blk_t; count: integer); options(val_param);
begin
  ec_$init(blk.input_ec);
  ec_$init(blk.ec);
  blk.flags := false;
  blk.count := count;
end;

procedure third_word(p: rec_ptr; a, b, c: integer); options(val_param);
begin
  p^.a := a;
  p^.b := b;
  p^.c := c;
end;

procedure reuse(p: rec_ptr); options(val_param);
begin
  p^.a := 0;
  p^.b := 0;
  other^.c := 0;
end;

procedure dies_before_call(p: rec_ptr; n: integer); options(val_param);
begin
  p^.a := n;
  tick(16#10);
end;

procedure in_loop(n: integer; p: rec_ptr); options(val_param);
begin
  p^.pad1 := 0;
  repeat
    p^.a := n;
    tick(16#10);
  until done;
end;

procedure pushed_last(p: rec_ptr; n: integer); options(val_param);
begin
  p^.a := n;
  tick(n);
end;

procedure status_last(var st: integer32);
begin
  st := 0;
  set_status(st);
end;
