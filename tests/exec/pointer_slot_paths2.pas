{ yqsm: pointer results through the result slot (codegen-patterns 3, "Every assignment last"): nil, several paths, case, with, frame locals, the Off fallback.  From the yqsm review. 2

  call: YS_SLOT2_$INIT
  call: YS_SLOT2_$T_NEST w:1
  call: YS_SLOT2_$T_NEST w:0
  call: YS_SLOT2_$T_A0L w:1
  call: YS_SLOT2_$T_A0L w:2
  call: YS_SLOT2_$T_GOTO w:0
  call: YS_SLOT2_$T_GOTO w:3
  call: YS_SLOT2_$T_LOOP w:3
  call: YS_SLOT2_$T_LOOP w:9
  call: YS_SLOT2_$T_TEMP w:1
  call: YS_SLOT2_$T_TEMP w:2
  call: YS_SLOT2_$T_FRAMED w:0
  call: YS_SLOT2_$T_FRAMED w:1
  call: YS_SLOT2_$T_FRAMED w:2
  expect:
    10
    -1
    -1
    20
    10
    -1
    20
    -1
    77
    20
    77
    30
    -1
    -1
    30
}
module ys_slot2;
%include 'exec.ins.pas';

type
  ip = ^integer32;
  big_t = record x: array[1..8] of integer32; end;
  rec_t = record
    a: integer32;
    alt: ip;
  end;
  recp = ^rec_t;
var
  v1, v2, v3: integer32;
  r1, r2: rec_t;
  tab: array[0..7] of recp;
  ptab: array[0..7] of ip;
  gp: ip;
  st: integer32;
  res: ip;
  bg: big_t;

procedure ys_slot2_$init;
begin
  v3 := 30; v2 := 20; v1 := 10;
  r1.a := 1; r1.alt := addr(v3);
  r2.a := 2; r2.alt := nil;
  tab[0] := nil;
  tab[1] := addr(r2);
  tab[2] := addr(r1);
  ptab[0] := addr(v2);
  ptab[1] := addr(v1);
  ptab[2] := addr(v3);
  ptab[3] := nil;
  gp := addr(v2);
  bg.x[3] := 77;
end;

procedure show;
begin
  if res = nil then exec_$logv_l(-1) else exec_$logv_l(res^);
end;

{ a nested pointer function reading its parent's locals }
procedure ys_slot2_$t_nest(k: integer); val_param;
var lp: ip; kk: integer;
  function inner: ip;
  begin
    st := 0;
    if kk = 0 then
      inner := nil
    else
      inner := lp;
  end;
begin
  lp := ptab[1];
  kk := k;
  res := inner;
  show;
end;

function getr(k: integer): recp; val_param;
begin
  st := 0;
  getr := tab[k];
end;

{ an A0 pointer local, then nil or memory }
function a0l(k: integer): ip; val_param;
var q: recp;
begin
  q := getr(k);
  st := q^.a;
  if st = 1 then
    a0l := ptab[0]
  else if st = 2 then
    a0l := q^.alt
  else
    a0l := nil;
end;

procedure ys_slot2_$t_a0l(k: integer); val_param;
begin
  res := a0l(k);
  show;
end;

{ goto past the assignment }
function fgoto(k: integer): ip; label 9;
begin
  st := 0;
  if k > 2 then goto 9;
  fgoto := ptab[k + 1];
  9:
end;

procedure ys_slot2_$t_goto(k: integer); val_param;
begin
  res := gp;
  res := fgoto(k);
  if k > 2 then exec_$logv_l(-1) else show;
end;

{ loop with exit before the last assignment }
function floop(k: integer): ip; val_param;
var i: integer;
begin
  i := 0;
  while i < 8 do begin
    if i = k then exit;
    i := i + 1;
  end;
  if i >= 8 then
    floop := nil
  else
    floop := ptab[i - 3];
end;

procedure ys_slot2_$t_loop(k: integer); val_param;
begin
  res := floop(k);
  show;
end;

{ a block copy temporary in the frame beside the slot }
function ftemp(k: integer): ip; val_param;
var e: big_t; t: integer32;
begin
  e := bg;
  t := e.x[3];
  exec_$logv_l(t);
  if k = 1 then
    ftemp := ptab[0]
  else
    ftemp := ptab[2];
end;

procedure ys_slot2_$t_temp(k: integer); val_param;
begin
  if k = 1 then begin
    res := ftemp(k);
    show;
  end
  else begin
    res := ftemp(k);
    { only the result: the 77 was logged above }
    show;
  end;
end;

function framed(k: integer): ip; val_param;
var p: recp;
begin
  p := tab[k];
  if p = nil then begin
    st := 1;
    framed := nil;
  end
  else if p^.alt = nil then begin
    st := 2;
    framed := nil;
  end
  else
    framed := p^.alt;
end;

procedure ys_slot2_$t_framed(k: integer); val_param;
begin
  res := framed(k);
  show;
end;
