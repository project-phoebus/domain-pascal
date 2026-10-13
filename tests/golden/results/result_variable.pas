{ codegen-patterns 3 ("Result variable"), 12.4 R5, 12.5 ("Stored locals
  pushed from the register"), 1.4, 7.1.  A function body of several
  statements holds its result variable in one data register up to the
  epilogue, where it is moved into D0 (dropped when it is D0 itself).
  Externals: ML_$SPIN_LOCK 0xE20BB6, ML_$SPIN_UNLOCK 0xE20BBE (the
  spin-lock token returned in D0 and passed back by value).

  - next_id: assigned from memory before a call, so in a callee-saved
    register (D2), `move.w D2,D0' at the end; the token stored to its
    slot and pushed from D0, which nothing wrote since (PKT_$NEXT_ID
    0xE1248E, also a conformance unit);
  - rtnva: a call's result copied into D2 across the next call (`move.l
    D0,D2'), the token reloaded from its slot after that call
    (NETBUF_$RTNVA 0xE0ED26);
  - chk_io: no call: the variable in D0, constants on three paths
    (`moveq', `clr.w'), and the entry loads in the order of R5: the
    parameter whose last use comes later shares D0 with the result
    (PARITY_$CHK_IO 0xE0B180..0xE0B1AA: `(8)'->D1, `(0xc)'->D0; the
    original's shared `else' part and its test of the result are not
    reproduced);
  - early: `f := c; return' on one path, `st'/`clr.b' for a boolean
    (PROC1_$SUSPENDP 0xE14886 `st D0'; 0xE4120C `st D2', `clr.b D2');
  - in_if: assigned only inside an `if', then a call: D2, and on the
    other path the register is as it was (PROC2_$GET_PID 0xE40CBA);
  - second_cell: a pointer result (A0, ref 5.5.10 p. 5-20) read from
    memory as the only statement: stored into the result variable's
    slot, then loaded from it into A0 (module-local 0xE6AA38, also a
    conformance unit; SIO_$I_GET_DESC 0xE66808). }
module results;
type
  intp = ^integer;
var
  lock: integer32;
  counter: 0..65535;
  err_ppn: integer32;
  dma_err: boolean;
  table: array[1..10] of integer;
  cells: array[1..4] of intp;

function ml_$spin_lock(var l: integer32): integer; extern;
procedure ml_$spin_unlock(var l: integer32; token: integer); options(extern, val_param);
function worker(p: integer32): integer32; options(extern, val_param);
procedure tick; extern;

function next_id: integer;
var
  token: integer;
begin
  token := ml_$spin_lock(lock);
  next_id := counter;
  counter := counter + 1;
  if counter > 16#FA00 then counter := 1;
  ml_$spin_unlock(lock, token);
end;

function rtnva(va: integer32): integer32; options(val_param);
var
  token: integer;
begin
  token := ml_$spin_lock(lock);
  rtnva := worker(va);
  ml_$spin_unlock(lock, token);
end;

function chk_io(ppn1, ppn2: integer32): integer; options(val_param);
begin
  if not dma_err then chk_io := 0
  else if ppn1 = err_ppn then chk_io := 1
  else if ppn2 = err_ppn then chk_io := 2
  else chk_io := 0;
  dma_err := false;
end;

function early(n: integer): boolean; options(val_param);
begin
  early := true;
  if n = table[1] then return;
  table[2] := n;
  early := false;
end;

function in_if(i: integer): integer; options(val_param);
begin
  if i > 3 then in_if := table[i];
  tick;
end;

function second_cell: intp;
begin
  second_cell := cells[2];
end;
