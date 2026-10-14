{ Execution test (review of bead domain-pascal-pze): pointers held in
  data registers (codegen-patterns 12.2, "Pointers in data registers")
  whose copies `reuse_pointer_copies' must not reuse across a call in a
  loop, across a call inside `with', or past a join where only one path
  made the copy; copies made in both parts of an `if' and reused after
  it; a by-address parameter compared and stepped around a call; a
  pointer only stored (held in an address register).

  call: PTR_DREGS_FLOW_$MAIN
  expect:
    3
    9
    13
    12
    11
    21
    22
    23
    13
    1
    5
}
module ptr_dregs_flow;
%include 'exec.ins.pas';

type
  rec = record
    value: integer32;
    awaiters: integer;
    pad: integer;
    more: integer32;
  end;
  rp = ^rec;
  addr_t = 0..16#7fffffff;

var
  r: rec;
  g: addr_t;
  gp: rp;

{ the copy is made again on each iteration: the call writes A0 }
procedure loopcall(p: rp); options(val_param);
var i: integer;
begin
  g := addr_t(p) - 4;
  i := 0;
  while i < 3 do begin
    p^.value := p^.value + 1;
    exec_$clobber;
    i := i + 1;
  end;
  p^.awaiters := 9;
end;

{ a copy in each part of the `if', read after its join }
procedure botharms(p: rp; c: integer); options(val_param);
begin
  g := addr_t(p) - 4;
  if c = 1 then p^.value := 11 else p^.value := 13;
  p^.awaiters := 12;
end;

{ `with' through the pointer around a call }
procedure withp(p: rp); options(val_param);
begin
  g := addr_t(p) - 4;
  with p^ do begin
    value := 21;
    exec_$clobber;
    awaiters := 22;
  end;
  p^.more := 23;
end;

{ a by-address parameter compared, then stepped around a call }
procedure cmpcall(in out ec: rec);
begin
  if addr_t(addr(ec)) > 1000 then begin
    ec.value := ec.value + 1;
    exec_$clobber;
    ec.value := ec.value + 1;
  end;
end;

{ stored whole and dereferenced }
procedure stored(p: rp); options(val_param);
begin
  gp := p;
  p^.awaiters := 1;
end;

procedure ptr_dregs_flow_$main;
begin
  r.value := 0;
  r.more := 0;
  r.awaiters := 0;
  loopcall(addr(r));
  exec_$log_l(r.value);
  exec_$log_w(r.awaiters);
  botharms(addr(r), 0);
  exec_$log_l(r.value);
  exec_$log_w(r.awaiters);
  botharms(addr(r), 1);
  exec_$log_l(r.value);
  withp(addr(r));
  exec_$log_l(r.value);
  exec_$log_w(r.awaiters);
  exec_$log_l(r.more);
  r.value := 11;
  cmpcall(r);
  exec_$log_l(r.value);
  stored(addr(r));
  exec_$log_w(r.awaiters);
  gp^.more := 5;
  exec_$log_l(r.more);
end;
