{ Golden: codegen-patterns 12.4 R4(c) in bodies with branches (bead
  domain-pascal-9j1z; `tools/regalloc.py callparams'): a parameter loaded
  at entry (12.2) whose last use on some path is an argument of a call is
  held through that call, so it takes a callee-saved register, wherever
  the call lies.
  - in_then: the call in an `if' part, the parameter dead after it: D2
    (AREA_$ASSOC 0xE096B0 D2, `move.w D2,-(SP)' at 0xE096D2).
  - both_parts: a call in each part of an `if', A2 (XPD_$GET_FP_INT
    0xE5C560, `move.w (A2),-(SP)' at 0xE5C574, `pea (A2)' at 0xE5C57E).
  - other_path: the parameter also used on the other path: A2
    (SIO_$I_INHIBIT_XMIT 0xE1C9D4, `bset' through A2 in one part, `pea
    (A2)' at 0xE1C9EC in the other).
  - after_inner_join: the call after a join inside an `if' part: A2
    (SOCK_$PUT_INT 0xE161A2, `pea (A2)' at 0xE161E4; the nested 0xE1CC10
    of KBD_$RCV, `kbd' in A2, `move.l (0x48,A2),-(SP)' at 0xE1CC52).
  - before_branch: the call before the routine's first branch: A2
    (VOLX_$GET_INFO 0xE6B5D4 A3, 0xE6B5D8 A4).
  Refused (driver tests): a call on every path after a join (bead
  domain-pascal-cs2n), a pointer parameter held in a data register (bead
  domain-pascal-b3wz), a register-held local (bead domain-pascal-ig8d). }
module regalloc;
type
  rec_t = record
    a: integer;
    b: integer;
    c: integer32;
  end;
  rec_p = ^rec_t;
var
  flag: extern boolean;
  count: extern integer;

procedure tick(n: integer); options(val_param, extern);
procedure take(in r: rec_t); extern;
procedure give(in n: integer32); extern;

procedure in_then(p: rec_p; n: integer); options(val_param);
begin
  p^.a := n;
  if flag then
    tick(n);
end;

procedure both_parts(in r: rec_t; out s: integer32);
begin
  s := 0;
  if flag then
    tick(r.a)
  else
    take(r);
end;

procedure other_path(var r: rec_t);
begin
  if flag then
    r.b := 1
  else begin
    r.b := 0;
    take(r);
  end;
end;

procedure after_inner_join(var r: rec_t);
begin
  if r.a > 0 then begin
    if flag then
      r.b := 1;
    take(r);
  end;
end;

procedure before_branch(var r: rec_t);
begin
  r.a := 0;
  take(r);
  if flag then
    count := 0;
end;
