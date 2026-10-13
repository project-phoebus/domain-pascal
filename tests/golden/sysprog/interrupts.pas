{ Golden: codegen-patterns 10 (interrupt priority), 12.5 (a local held
  in a register), 1.1.  Beads domain-pascal-5aw.7, -0wc.
  - `disable' / `enable' (ref Appendix E, pp. E-3, E-4) as `ori.w
    #0x700,SR' / `andi.w #0xF8FF,SR' (TIME_$GET_ADJUST 0xE16AB0, 0xE16ABA);
  - `oldsr := set_sr(newsr)' (ref p. E-5): `move.w SR,Dn; move.w
    newsr,SR' (PROC1_$SET_VT 0xE149B6..0xE149B8), then the store from Dn
    (the reference's expansion; no image site stores to memory);
  - `discard(set_sr(old))' (0xE149F2..0xE149F4, PROC1_$TS_END_CALLBACK
    0xE14AF8): the old word read into a register nothing reads;
  - a local assigned once from `set_sr' held in its register, its frame
    slot still allocated (PROC1_$SET_VT 0xE149B6 `move.w SR,D3');
  - `discard' of a function call: a call statement, its result unread
    (ref p. 4-62);
  - TIME_$GET_ADJUST 0xE16AA8 itself: tests/conformance/time_get_adjust.pas. }
module intr;
type
  pinteger = 0..65535;
var
  delta: extern integer32;
  saved: extern integer;
  w: extern integer;
  u: extern pinteger;

function ext_$f(x: integer): integer; options(val_param, extern);

procedure crit(out d: integer32);
begin
  disable;
  d := delta;
  enable;
end;

procedure save_restore;
begin
  saved := set_sr(16#2700);
  w := 1;
  discard(set_sr(saved));
end;

procedure held_sr;
var
  old: integer;
begin
  old := set_sr(16#2700);
  w := 1;
  discard(set_sr(old));
end;

procedure discard_call;
begin
  discard(ext_$f(3));
  w := 0;
end;

procedure unsigned_sr;
begin
  u := set_sr(16#F8FF);
end;
