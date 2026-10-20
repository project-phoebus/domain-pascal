{ codegen-patterns 3, "`nil` results"; bead domain-pascal-bdi0.  `nil' as
  a pointer function's result (ref 3.13.4, p. 3-51) takes the form of the
  result variable's other assignments:

  - last_only: `f := nil' the only assignment and the last statement of
    a body of several statements: `movea.l #0,A0', the frame slot left
    alone (TIME_$RTE_INT 0xE163D6 after its calls, TIME_$VT_INT
    0xE16426);
  - first_nil: an assignment that is not last (`f := nil' first, `f :=
    x; return' in a `for') makes the result variable a register's, A0
    where no call follows: `suba.l A0,A0', the loop's count after it
    (DISK_$GET_DRTE 0xE3DA36 `suba.l A0,A0; moveq #31,D2');
  - locked: a call after the assignments: the variable in A2, `suba.l
    A2,A2' on the `nil' path and moved to A0 at the end
    (EC2_$REGISTER_EC1 0xE4299C `suba.l A2,A2; bra.s', ML_$UNLOCK after
    the join, 0xE429CE `movea.l A2,A0');
  - returned: `f := x; return' and a final `f := nil', no call:
    `suba.l A0,A0' falling into the epilogue (ROUTE_$FIND_PORTP 0xE15B84,
    0xE15B7A `movea.l A1,A0; bra.s' on the other path, its `f := port;
    return' in a loop; the AST module-local 0xE0209E, 0xE020EE).
  Refused, in driver tests: every assignment last on several paths, all
  `nil' (no site; with memory on another path the result slot,
  NET_$FIND_HANDLER 0xE5A178, golden `results/pointer_slot'), a body of
  the one statement `f := nil', `nil' right before a call's pad (bead
  domain-pascal-ncac). }
module nilres;
type
  status_$t = record case integer of 1: (all: integer32); end;
  rec_t = record
    active: integer;
    kind: integer;
    next: ^rec_t;
  end;
  rec_ptr = ^rec_t;
var
  tab: array[0..7] of rec_ptr;
  busy: boolean;
  count: integer;
  devices: extern array[0..7] of record
    flags: integer;
    kind: integer;
    table: rec_ptr;
  end;
procedure ml_$lock(id: integer); val_param; extern;
procedure ml_$unlock(id: integer); val_param; extern;
procedure scan(var s: status_$t); extern;

function last_only: rec_ptr;
var
  status: status_$t;
begin
  scan(status);
  busy := false;
  last_only := nil;
end;

function first_nil(k: integer): rec_ptr; val_param;
var
  i: integer;
begin
  first_nil := nil;
  for i := 0 to 7 do
    if devices[i].kind = k then begin
      first_nil := devices[i].table;
      return;
    end;
end;

function locked(k: integer): rec_ptr; val_param;
begin
  ml_$lock(6);
  if tab[k] = nil then begin
    count := 1;
    locked := nil;
  end
  else
    locked := tab[k];
  ml_$unlock(6);
end;

function returned(k: integer): rec_ptr; val_param;
begin
  if k <> 0 then begin
    count := k;
    returned := tab[k];
    return;
  end;
  count := 0;
  returned := nil;
end;
