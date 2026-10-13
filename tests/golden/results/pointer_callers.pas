{ codegen-patterns 3 ("Callers of pointer functions", "Pointer results"),
  12.4 R4(a), 12.5; beads domain-pascal-ae9.  A function whose result is
  a pointer returns it in A0 (ref 5.5.10, p. 5-20); its caller reads A0
  right after the call and its pop (`tools/resultsurvey.py callers').

  - wrap: `f := g(...)' for a pointer result: the call's A0 stored into
    the result variable's slot and left in A0 (MST_$MAPS 0xE439C8 `move.l
    A0,(-4,A6)', also a conformance unit);
  - locked: a pointer result variable in a body of several statements,
    held in A2 across the next call and moved to A0 at the end
    (DISK_$GET_BLOCK 0xE3BBBC / 0xE3BBCA, also a conformance unit);
  - stores: A0 stored to the unit's data (TIME_$GET_EC 0xE16734 `move.l
    A0,(0x1620,A5)'), to another unit's variable (PEB_$LOAD_WCS 0xE32072
    `move.l A0,abs.l'); a pointer local that is only dereferenced is
    held in an address register, `movea.l A0,A2', and its object passed
    by address with `pea (A2)' (KBD_$SET_KBD_TYPE 0xE7253E, 0xE72550;
    bead domain-pascal-500; locals kept in their frame slot are in
    pointer_locals);
  - tests: A0 against `nil', `cmpa.w #0,A0' (AST_$LOAD_AOTE 0xE023BA);
    A0 as the base of a load (0xE4E990 `tst.w (A0)'; VFMT_$MAIN 0xE6AD10
    `move.w (2,A0),D7');
  - held: A0 copied into a local held in a data register, `move.l A0,D0'
    (TTY_$K_INQ_FUNC_ENABLED 0xE6763C);
  - args: by-address parameters read once inside one argument list, each
    in a new address register, A0, A1, A2 (MST_$MAP 0xE43886..0xE438AA,
    12.4 R4(a)). }
module pcall;
type
  status_$t = record case integer of 1: (all: integer32); end;
  rec_t = record
    count: integer;
    next: ^rec_t;
  end;
  rec_ptr = ^rec_t;
var
  cached: rec_ptr;
  seen: integer;
  gstatus: status_$t;
  ext_cell: extern rec_ptr;

function lookup(key: integer; var status: status_$t): rec_ptr; val_param; extern;
procedure ml_$lock(id: integer); val_param; extern;
procedure ml_$unlock(id: integer); val_param; extern;
procedure take(var a: integer; var b: integer; var c: integer); extern;
procedure take_values(a: integer; b: integer; c: integer); val_param; extern;

function wrap(key: integer; var status: status_$t): rec_ptr; val_param;
begin
  wrap := lookup(key, status);
end;

function locked(key: integer; var status: status_$t): rec_ptr; val_param;
begin
  ml_$lock(15);
  locked := lookup(key, status);
  ml_$unlock(15);
end;

procedure stores(key: integer; var status: status_$t); val_param;
var
  local: rec_ptr;
begin
  cached := lookup(key, status);
  ext_cell := lookup(key, status);
  local := lookup(key, status);
  take(local^.count, local^.count, local^.count);
end;

procedure tests;
begin
  if lookup(seen, gstatus) = nil then seen := 1;
  seen := lookup(seen, gstatus)^.count;
end;

procedure held;
var
  p: rec_ptr;
begin
  p := lookup(seen, gstatus);
  seen := 0;
  cached := p;
end;

procedure args(var a: integer; var b: integer; var c: integer);
begin
  take_values(a, b, c);
end;
