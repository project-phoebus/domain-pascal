{ Execution test (bead domain-pascal-5kgg): best-effort stores
  (codegen-patterns 12.2, 12.4, 5.1, 10.1): a by-value parameter assigned
  and never read, a parameter not loaded at entry stored, an enclosing
  routine's parameter assigned and `addr' of its by-address parameter
  stored, `[k]' with a run-time member assigned to a set, `max' and a
  call's value stored through a pointer (the value's instructions first,
  not rotated), a constant minus a computed value, a call through a
  routine pointer whose slot the arguments read too, a pointer in a data
  register whose part is passed by address, and another unit's element
  stored from another part of the same variable.

  call: EB_$FILL
  call: BX_STORES_$UNREAD w:4
  call: BX_STORES_$ONE w:-9
  call: BX_STORES_$SHOWG
  call: BX_STORES_$OUTER w:5
  call: BX_STORES_$ADDROUT
  call: BX_STORES_$SINGLE w:3
  call: BX_STORES_$SINGLE w:4
  call: BX_STORES_$MAXSTORE w:3 w:8
  call: BX_STORES_$MAXSTORE w:9 w:-8
  call: BX_STORES_$CALLSTORE w:41
  call: BX_STORES_$SUB4 w:3 l:0
  call: BX_STORES_$IND
  call: BX_STORES_$DPART w:1
  call: BX_STORES_$DPART w:0
  call: BX_STORES_$SAME w:4
  call: BX_STORES_$WORD w:4 -> w
  link: elem_bases_data.mod.pas
  warns: best effort (not byte-identical): assigning to a by-value parameter the routine never reads
  warns: best effort (not byte-identical): storing a parameter that was not loaded at entry
  warns: best effort (not byte-identical): assigning to an enclosing routine's parameter
  warns: best effort (not byte-identical): `addr` of an enclosing routine's parameter passed by address, stored
  warns: best effort (not byte-identical): assigning `[x]` with a run-time member to a set
  warns: best effort (not byte-identical): a store whose destination needs a register of a `max` or `min` value
  warns: best effort (not byte-identical): a store whose destination and value both need a register, with a call on either side
  warns: best effort (not byte-identical): a constant minus a computed value
  warns: best effort (not byte-identical): a call through a routine pointer reached through a parameter slot the arguments also read
  warns: best effort (not byte-identical): passing by address a part of an object reached through a pointer held in a data register
  warns: best effort (not byte-identical): a store reaching an element of another unit's array and another part of the same variable
  expect:
    mark 1
    -9
    15
    5
    mark 3
    mark 0
    8
    9
    42
    6
    77
    20
    10
    10
    10
    => -935
}
module be_x_stores;
%include 'exec.ins.pas';

type
  wset = set of 0..15;
  rp = ^rec;
  rec = record a, b: integer; next: rp; end;
  proc_t = ^procedure(in v: integer);
  hp = ^holder;
  holder = record f: proc_t; w: integer; end;
var
  g: integer;
  gr: rec;
  gp: rp;
  s: wset;
  hold: holder;
  eb_$words: extern array[1..8] of integer;

procedure bx_stores_$unread(n: integer); options(val_param);
begin
  n := 3;
  exec_$mark(1);
end;

procedure bx_stores_$one(n: integer); options(val_param);
begin
  g := n;
end;

procedure bx_stores_$showg;
begin
  exec_$log_w(g);
end;

procedure bx_stores_$outer(n: integer); options(val_param);
  procedure bump;
  begin
    n := n + 10;
  end;
begin
  bump;
  exec_$logv_w(n);
end;

procedure outer2(var r: rec);
var q: rp;
  procedure inner;
  begin
    q := addr(r);
  end;
begin
  inner;
  q^.a := 5;
end;

procedure bx_stores_$addrout;
begin
  gr.a := 0;
  outer2(gr);
  exec_$log_w(gr.a);
end;

procedure bx_stores_$single(k: integer); options(val_param);
begin
  s := [k];
  if 3 in s then exec_$mark(3) else exec_$mark(0);
end;

procedure bx_stores_$maxstore(x, y: integer); options(val_param);
begin
  gp := addr(gr);
  gp^.b := max(x, y);
  exec_$log_w(gp^.b);
end;

procedure bx_stores_$callstore(x: integer); options(val_param);
begin
  gp := addr(gr);
  gp^.b := exec_$id_w(x) + 1;
  exec_$log_w(gp^.b);
end;

procedure bx_stores_$sub4(x: integer; y: integer32); options(val_param);
begin
  x := 10 - (x + 1);
  g := x;
  exec_$log_w(g);
end;

procedure call_ind(h: hp); options(val_param);
begin
  h^.f^(h^.w);
end;

procedure bx_stores_$ind;
begin
  hold.f := addr(exec_$log_w);
  hold.w := 77;
  call_ind(addr(hold));
end;

procedure dpart(p: rp); options(val_param);
begin
  if p <> nil then exec_$log_w(p^.b);
  if p = gp then exec_$log_w(p^.a);
end;

procedure bx_stores_$dpart(k: integer); options(val_param);
begin
  gr.a := 10;
  gr.b := 20;
  gp := addr(gr);
  if k = 1 then dpart(gp) else dpart(nil);
  exec_$log_w(gr.a);
end;

procedure bx_stores_$same(i: integer); options(val_param);
begin
  eb_$words[i] := eb_$words[1] + 1;
end;

function bx_stores_$word(i: integer): integer; options(val_param);
begin
  bx_stores_$word := eb_$words[i];
end;
