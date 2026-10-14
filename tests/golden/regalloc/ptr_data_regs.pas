{ Golden: codegen-patterns 12.2, "Pointers in data registers" (bead
  domain-pascal-pze): a pointer parameter, or a by-address parameter,
  that the body also uses as data is held in a data register from the
  entry and copied to an address register where it is dereferenced.
  - init: EC2_$INIT 0xE42C60 (ec/ec2_init.c; `out ec' in ec2.ins.pas):
    `move.l (8,A6),D0', the address compared (0xE42C68), one `movea.l
    D0,A0' for the two stores (0xE42C70..0xE42C74), the compare of a
    non-negative 4-byte subrange unsigned (`bls', 0xE42C6E; bead
    domain-pascal-5j9).  The image's `link #-4' is not reproduced here
    (tests/conformance/ec2_init.pas has the whole routine).
  - free_stack: PROC1_$FREE_STACK 0xE1511A (proc1/free_stack.c): a
    pointer value parameter used in arithmetic (0xE15130 `subq.l #4,D0')
    and copied for the store through it (0xE15136).  In wake and joins
    the address is likewise used in arithmetic: a store of the whole
    pointer alone does not put it in a data register (stored: EC_$INIT
    0xE15202..0xE1520C holds its pointer in A0 and stores A0).
  - wake: the address passed on to a `var' parameter from the data
    register, `move.l Dn,-(SP)' (EC2_$ADVANCE 0xE42CDA; OSINFO_$GET_MMAP
    0xE5C72C, 0xE5C860); after the call the copy is made again
    (OSINFO_$GET_MMAP 0xE5C870 after the `jsr' at 0xE5C864) and then
    shared by the later statements.
  - joins: a copy made in an `if' condition is read again after the
    `if' and its join while no instruction writes its register
    (OSINFO_$GET_MMAP 0xE5C8C8 and 0xE5C952 read A1, the copy of D3 made
    at 0xE5C870); the slot loaded at entry keeps A0, the copy takes A1
    (12.4 R4(b): 0xE5C6B4 A1 after the copy in A0).
  - stored: a pointer value parameter dereferenced and stored whole
    stays in an address register and is stored from it (the AST
    module-local 0xE00F7C: `movea.l (8,A6),A1' at 0xE00F86, `move.l
    A1,(0x3ec,A5)' at 0xE00FA0; EC_$INIT 0xE15208). }
module ptr_data_regs;
type
  ec_t = record
    value: integer32;
    awaiters: integer;
  end;
  ec_ptr_t = ^ec_t;
  addr_t = 0..16#7fffffff;
  link_t = ^addr_t;
  status_$t = integer32;
var
  g: addr_t;
  free_list: addr_t;

procedure wakeup(in out ec: ec_t; out st: status_$t); extern;

procedure init(out ec: ec_t);
begin
  if addr_t(addr(ec)) > 1000 then begin
    ec.value := 0;
    ec.awaiters := 0;
  end;
end;

procedure free_stack(p: link_t); options(val_param);
begin
  g := addr_t(p) - 4;
  p^ := free_list;
end;

procedure wake(in out ec: ec_t; out st: status_$t);
begin
  g := addr_t(addr(ec)) - 4;
  ec.value := 0;
  wakeup(ec, st);
  ec.awaiters := 0;
  ec.value := 2;
  st := 0;
end;

procedure joins(in out ec: ec_t; out st: status_$t);
begin
  g := addr_t(addr(ec)) - 4;
  if ec.value = 0 then ec.awaiters := 1;
  ec.value := 2;
  st := 0;
  if ec.value = 1 then st := 1 else g := 0;
  ec.value := 3;
end;

procedure stored(p: link_t); options(val_param);
begin
  g := addr_t(p);
  p^ := free_list;
end;
