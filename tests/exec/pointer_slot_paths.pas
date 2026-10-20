{ yqsm: pointer results through the result slot (codegen-patterns 3, "Every assignment last"): nil, several paths, case, with, frame locals, the Off fallback.  From the yqsm review.

  call: YS_SLOT_$INIT
  call: YS_SLOT_$T_FIND w:0
  call: YS_SLOT_$T_FIND w:1
  call: YS_SLOT_$T_FIND w:2
  call: YS_SLOT_$T_FIND w:3
  call: YS_SLOT_$T_CASE w:0
  call: YS_SLOT_$T_CASE w:1
  call: YS_SLOT_$T_CASE w:2
  call: YS_SLOT_$T_CASE w:3
  call: YS_SLOT_$T_CASE w:7
  call: YS_SLOT_$T_WITH w:2
  call: YS_SLOT_$T_WITH w:5
  call: YS_SLOT_$T_CSE w:2
  call: YS_SLOT_$T_CSE w:1
  call: YS_SLOT_$T_ESC w:0
  call: YS_SLOT_$T_ESC w:1
  call: YS_SLOT_$T_IDX w:1
  call: YS_SLOT_$T_IDX w:6
  expect:
    -1
    -1
    30
    -1
    20
    30
    10
    10
    20
    30
    -1
    30
    -1
    20
    10
    30
    -1
}
module ys_slot;
%include 'exec.ins.pas';

type
  ip = ^integer32;
  rec_t = record
    a: integer32;
    alt: ip;
    nxt: ip;
  end;
  recp = ^rec_t;
var
  v1, v2, v3, v4: integer32;
  r1, r2: rec_t;
  tab: array[0..7] of recp;
  ptab: array[0..7] of ip;
  gp: ip;
  st: integer32;
  res: ip;

procedure ys_slot_$init;
var i: integer;
begin
  v4 := 40; v3 := 30; v2 := 20; v1 := 10;
  r1.a := 1; r1.alt := addr(v3); r1.nxt := addr(v1);
  r2.a := 2; r2.alt := nil; r2.nxt := addr(v2);
  i := 0;
  while i <= 7 do begin tab[i] := nil; ptab[i] := nil; i := i + 1; end;
  tab[1] := addr(r2);
  tab[2] := addr(r1);
  ptab[1] := addr(v1);
  ptab[2] := addr(v2);
  ptab[7] := addr(v3);
  gp := addr(v2);
end;

procedure show;
begin
  if res = nil then exec_$logv_l(-1) else exec_$logv_l(res^);
end;

{ nil on two paths, memory on the third }
function find(k: integer; var s: integer32): ip; val_param;
begin
  if tab[k] = nil then begin
    s := 1;
    find := nil;
  end
  else if tab[k]^.alt = nil then begin
    s := 2;
    find := nil;
  end
  else begin
    s := 0;
    find := tab[k]^.alt;
  end;
end;

procedure ys_slot_$t_find(k: integer); val_param;
begin
  res := find(k, st);
  show;
end;

{ case with otherwise; memory, nil, uplevel-free }
function fcase(k: integer): ip; val_param;
begin
  st := k;
  case k of
    0: fcase := gp;
    1: fcase := tab[2]^.alt;
    2: fcase := ptab[1];
    3: fcase := r1.nxt;
    otherwise fcase := r2.nxt;
  end;
end;

procedure ys_slot_$t_case(k: integer); val_param;
begin
  res := fcase(k);
  show;
end;

{ assigned inside a with }
function fwith(k: integer): ip; val_param;
begin
  st := 0;
  if (k >= 0) and (k <= 7) and (tab[k] <> nil) then
    with tab[k]^ do
      fwith := alt
  else
    fwith := nil;
end;

procedure ys_slot_$t_with(k: integer); val_param;
begin
  res := fwith(k);
  show;
end;

{ same element read in the test and assigned }
function fcse(k: integer): ip; val_param;
begin
  st := 0;
  if ptab[k] <> nil then
    fcse := ptab[k]
  else
    fcse := nil;
end;

procedure ys_slot_$t_cse(k: integer); val_param;
begin
  res := fcse(k + 5);
  show;
end;

procedure setp(var p: ip; k: integer); val_param;
begin
  if k = 0 then p := addr(v2) else p := addr(v1);
end;

{ a local whose address is taken }
function fesc(k: integer): ip; val_param;
var q: ip;
begin
  setp(q, k);
  st := 1;
  fesc := q;
end;

procedure ys_slot_$t_esc(k: integer); val_param;
begin
  res := fesc(k);
  show;
end;

{ computed index }
function fidx(k: integer): ip; val_param;
begin
  st := 0;
  if k > 5 then return;
  fidx := ptab[k + 6];
end;

procedure ys_slot_$t_idx(k: integer); val_param;
var p: ip;
begin
  p := gp;
  p := fidx(k);
  res := p;
  if k > 5 then exec_$logv_l(-1) else show;
end;
