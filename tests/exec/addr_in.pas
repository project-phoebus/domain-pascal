{ Execution test: `addr' of an `in' parameter or of a field of one (bead
  domain-pascal-mbtx; golden params/addr_in): by address, the address the
  slot holds, stored from the frame after a call and from the register
  holding the slot, pushed by value; a field's address; under `val_param'
  the by-value slot's address.  Each address is read back through a
  pointer, so the values show the right object was reached.

  call: ADDRIN_$SETUP
  call: ADDRIN_$PUT
  call: ADDRIN_$INIT
  call: ADDRIN_$PASS
  call: ADDRIN_$VAL
  expect:
    77
    7
    5
    77
    1
    77
    41
    42
}
module addrin;
%include 'exec.ins.pas';

type
  info_t = record
    version: integer32;
    port: integer32;
  end;
  info_p = ^info_t;
  ec_t = record
    value: integer32;
    head, tail: univ_ptr;
  end;
  ec_p = ^ec_t;
  long_p = ^integer32;
var
  r: info_t;
  e: ec_t;
  g: univ_ptr;

procedure addrin_$setup;
begin
  { the higher field first: a small longword constant stored below the
    next statement's nonzero one is refused (codegen-patterns 13; bead
    domain-pascal-53xi) }
  r.port := 77;
  r.version := 1;
  e.value := 5;
end;

procedure put(in info: info_t);
var
  src: info_p;
begin
  exec_$clobber;
  src := addr(info);
  exec_$log_l(src^.port);
end;

procedure addrin_$put;
begin
  put(r);
end;

procedure init(in out ec: ec_t);
begin
  ec.value := 7;
  ec.head := addr(ec);
  ec.tail := addr(ec);
end;

procedure addrin_$init;
var
  p: ec_p;
begin
  init(e);
  p := e.head;
  exec_$log_l(p^.value);
  e.value := 5;
  p := e.tail;
  exec_$log_l(p^.value);
end;

procedure show(in p: univ_ptr); options(val_param);
var
  q: info_p;
begin
  q := p;
  exec_$log_l(q^.port);
  exec_$log_l(q^.version);
end;

procedure pass(in info: info_t);
var
  l: long_p;
begin
  show(addr(info));
  g := addr(info.port);
  l := g;
  exec_$log_l(l^);
end;

procedure addrin_$pass;
begin
  pass(r);
end;

procedure val(in x: integer32; in y: integer32); options(val_param);
var
  l: long_p;
begin
  g := addr(x);
  l := g;
  exec_$log_l(l^);
  exec_$log_l(y + 1);
end;

procedure addrin_$val;
begin
  val(41, 41);
end;
