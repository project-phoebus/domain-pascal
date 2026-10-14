{ Golden: computed booleans stored and pushed (bead domain-pascal-k1sv;
  codegen-patterns 3, "Comparisons as boolean results").  A comparison
  stored to memory: `s<cc>' into a data register, then `move.b' (no
  `s<cc>' to memory in compiled code; TTY_$K_GET 0xE1C506 `cmp.w (A3),D3;
  seq D1; move.b D1,(-0x1e,A6)'; 0xE5CF30 `cmpi.w #3,(A0); seq D1; move.b
  D1,(294,A5)').  A comparison assigned to a register-held local:
  `s<cc>' into the local's register (TTY_$K_GET 0xE1C42E `tst.w
  (0x38,A2); seq D5').  A comparison and a set member passed by value:
  formed right before the push (TTY_$K_GET 0xE1C54E `btst.b
  #1,(0x17,A2); sne D1; move.b D1,-(SP)'; AREA_$DEACTIVATE_ASTE 0xE0A196
  `tst.l D0; seq D1; move.b D1,-(SP)'). }
module bool_stores;
type
  rec = record
    mode: integer;
    count: integer;
    flags: set of 0..15;
  end;
  rec_ptr = ^rec;
var
  done: boolean;
  limit: integer;
procedure take(a: boolean; b: boolean); options(val_param, extern);
procedure q(in b: boolean); extern;
procedure stores(p: rec_ptr; n: integer); options(val_param);
var
  eol: boolean;
  i: integer;
begin
  done := p^.count = 3;
  i := 0;
  repeat
    eol := p^.mode = 0;
    if eol then i := i + 1;
    if not eol then i := i + 2;
  until i > limit;
end;
procedure pushes(p: rec_ptr); options(val_param);
begin
  take(p^.count > 3, 1 in p^.flags);
end;
procedure frame_store(p: rec_ptr); options(val_param);
var
  w: boolean;
begin
  w := p^.mode < 5;
  q(w);
end;
