{ Execution test: a by-address parameter whose address the body also
  uses as data is held in a data register and copied to an address
  register where it is dereferenced (codegen-patterns 12.2, "Pointers in
  data registers"; bead domain-pascal-pze): a copy made in an `if'
  condition read again after the `if', copies renamed to the one still
  held, a copy made again after a call, the address passed on from the
  data register.

  call: PTR_DREGS_$MAIN
  expect:
    5
    1
    7
    0
    6
}
module ptr_dregs;
%include 'exec.ins.pas';

type
  rec = record
    value: integer32;
    awaiters: integer;
  end;
  addr_t = 0..16#7fffffff;

var
  r: rec;
  g: addr_t;

procedure bump(in out ec: rec; out st: integer32);
begin
  ec.value := ec.value + 1;
  st := 0;
end;

procedure work(in out ec: rec; out st: integer32);
begin
  g := addr_t(addr(ec)) - 4;
  if ec.value = 0 then ec.awaiters := 1;
  ec.value := 2;
  st := 0;
  if ec.value = 2 then st := 7 else g := 0;
  ec.value := ec.value + 3;
end;

procedure again(in out ec: rec; out st: integer32);
begin
  g := addr_t(addr(ec)) - 4;
  bump(ec, st);
  ec.awaiters := 6;
end;

procedure ptr_dregs_$main;
var
  st: integer32;
begin
  r.value := 0;
  r.awaiters := 0;
  work(r, st);
  exec_$log_l(r.value);
  exec_$log_w(r.awaiters);
  exec_$log_l(st);
  r.value := -1;
  again(r, st);
  exec_$log_l(r.value);
  exec_$log_w(r.awaiters);
end;
