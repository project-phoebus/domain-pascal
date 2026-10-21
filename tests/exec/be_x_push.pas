{ Execution test (bead domain-pascal-5kgg): best-effort argument pushes
  (codegen-patterns 1.5): a `for' index passed by address twice (stored
  to its slot once), a byte constant next to a word constant (each in
  its own slot), `addr(p^)' by value (the pointer's value pushed), an
  object reached through a chain of pointers, and `addr' of another
  unit's object passed by address (once stored into a frame temporary
  with `lea', a best effort; now a code cell holding the address, as in
  the image: bead domain-pascal-68tc, golden calls/univ_symbol_addr).

  call: EB_$FILL
  call: BX_PUSH_$TWICE
  call: BX_PUSH_$BYTES
  call: BX_PUSH_$ADDRP
  call: BX_PUSH_$CHAIN
  call: BX_PUSH_$ADDRSYM
  link: elem_bases_data.mod.pas
  warns: best effort (not byte-identical): a `for` index passed by address to a call with a function call among its arguments, or passed twice
  warns: best effort (not byte-identical): a byte constant argument other than `true` and 0 next to a word or byte constant
  warns: best effort (not byte-identical): `addr(p^)` by value of a pointer in memory
  warns: best effort (not byte-identical): an object reached through a pointer that is itself reached through a pointer
  expect:
    11
    22
    33
    97
    5
    1
    7
    3
    44
}
module be_x_push;
%include 'exec.ins.pas';
type
  rp = ^rec;
  rec = record a, b: integer; next: rp; end;
  pint = ^integer;
var
  gr: rec;
  gp: rp;
  gpp: ^rp;
  kept: univ_ptr;
  eb_$k: extern integer;

procedure two(in a: integer; in b: integer);
begin
  exec_$logv_w(a * 10 + b);
end;

procedure bx_push_$twice;
var i: integer;
begin
  for i := 1 to 3 do two(i, i);
end;

procedure bw(c: char; w: integer); options(val_param);
begin
  exec_$logv_c(c);
  exec_$logv_w(w);
end;

procedure bx_push_$bytes;
begin
  bw('a', 5);
end;

procedure keep(p: univ_ptr); options(val_param);
begin
  kept := p;
end;

procedure bx_push_$addrp;
begin
  gp := addr(gr);
  kept := nil;
  keep(addr(gp^));
  if kept = gp then exec_$logv_w(1) else exec_$logv_w(0);
end;

procedure bx_push_$chain;
begin
  gr.b := 7;
  gp := addr(gr);
  gpp := addr(gp);
  exec_$log_w(gpp^^.b);
end;

procedure keepin(in p: univ_ptr);
begin
  kept := p;
end;

procedure bx_push_$addrsym;
var q: pint;
begin
  keepin(addr(eb_$k));
  q := kept;
  exec_$log_w(q^);
  q^ := 44;
  exec_$log_w(eb_$k);
end;
