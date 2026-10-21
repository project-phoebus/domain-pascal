{ Execution test (bead domain-pascal-do4s): calls through routine
  pointers in elements of the unit's own array, the element's address
  first and the pointer through it into another register (IO_$MAP_CSR
  0xE1A8C0 `lea (0,A5,D0.w),A0; movea.l (0x64,A0),A1; jsr (A1)'), a field
  within a byte and one beyond it; a function result assigned on one path
  only, left in D0 (IO_$DEVICE_INTERRUPTING 0xE1A788, codegen-patterns 3,
  "Result variable"); a `bra' after a status store landing past the
  status's `tst' (io_$check_unit 0xE1A4BE) and, after a `clr', past the
  branch that cannot be taken (IO_$ACQUIRE 0xE1A566; codegen-patterns
  7.1, "A branch past the test of a store").

  call: ELEM_PTR_CALLS_$SETUP
  call: ELEM_PTR_CALLS_$CALL w:1
  call: ELEM_PTR_CALLS_$CALL w:2
  call: ELEM_PTR_CALLS_$FAR w:2
  call: ELEM_PTR_CALLS_$ID w:1 @l:-1! -> l
  call: ELEM_PTR_CALLS_$ID w:2 @l:-1! -> l
  call: ELEM_PTR_CALLS_$ID w:9 @l:-1!
  call: ELEM_PTR_CALLS_$ID w:0 @l:-1!
  call: ELEM_PTR_CALLS_$TEST w:1 b:0 @l:-1! -> b
  call: ELEM_PTR_CALLS_$TEST w:2 b:255 @l:-1! -> b
  call: ELEM_PTR_CALLS_$TEST w:31 b:255 @l:-1!
  call: ELEM_PTR_CALLS_$CHECKED w:1 @l:-1! -> b
  call: ELEM_PTR_CALLS_$CHECKED w:0 @l:-1! -> b
  call: ELEM_PTR_CALLS_$CHECKED w:40 @l:-1! -> b
  call: ELEM_PTR_CALLS_$FIND @w:-1! @l:-1!
  call: ELEM_PTR_CALLS_$FILL
  call: ELEM_PTR_CALLS_$FIND @w:-1! @l:-1!
  call: ELEM_PTR_CALLS_$CHAIN w:1 @l:-1!
  call: ELEM_PTR_CALLS_$CHAIN w:2 @l:-1!
  call: ELEM_PTR_CALLS_$CHAIN w:3 @l:-1!
  expect:
    1000
    2000
    2
    out 0
    => 1000
    out 0
    => 2000
    out 1
    out 1
    out 0
    => false
    out 0
    => true
    out 1
    out 0
    => true
    out 3
    => false
    out 4
    => false
    mark 1
    out -1
    out 7
    mark 2
    mark 1
    out 2
    out 0
    mark 3
    out 5
    mark 3
    out 6
    mark 3
    mark 4
    out 0
}
module elem_ptr_calls;
%include 'exec.ins.pas';

type
  status_t = record
    all: integer32;
  end;
  vec_t = record
    log: ^procedure(v: integer32); options(val_param);
    idl: ^function(in v: integer32): integer32;
    idb: ^function(in v: boolean): boolean;
    pad: array[0..39] of integer32;
    far_log: ^procedure(in v: integer);
  end;

var
  filler: array[0..3] of integer32;
  tag: array[0..3] of integer;
  table: array[0..3] of vec_t;
  big: array[0..3] of integer32;

procedure elem_ptr_calls_$setup;
var
  i: integer;
begin
  for i := 0 to 3 do
    with table[i] do begin
      log := addr(exec_$logv_l);
      idl := addr(exec_$id_l);
      idb := addr(exec_$id_bool);
      far_log := addr(exec_$log_w);
    end;
  for i := 0 to 3 do
    big[i] := i * 1000;
  for i := 0 to 3 do
    tag[i] := 1;
end;

procedure elem_ptr_calls_$call(i: integer); options(val_param);
begin
  table[i].log^(big[i]);
end;

procedure elem_ptr_calls_$far(i: integer); options(val_param);
begin
  table[i].far_log^(i);
end;

{ The status cleared, then an error for a unit out of range or not in use;
  true when it is still clear (io_$check_unit's shape). }
function check(i: integer; out status: status_t): boolean;
  internal;
begin
  status.all := 0;
  if (i < 1) or (i > 3) then
    status.all := 1
  else if tag[i] = 0 then
    status.all := 2;
  check := status.all = 0;
end;

function elem_ptr_calls_$id(i: integer; out status: status_t): integer32;
  options(val_param);
begin
  if check(i, status) then
    elem_ptr_calls_$id := table[i].idl^(big[i]);
end;

function elem_ptr_calls_$test(i: integer; b: boolean;
                              out status: status_t): boolean;
  options(val_param);
begin
  if check(i, status) then
    with table[i] do
      elem_ptr_calls_$test := table[tag[i]].idb^(b);
end;

function elem_ptr_calls_$checked(i: integer; out status: status_t): boolean;
  options(val_param);
begin
  if i > 30 then
    status.all := 4
  else if i < 1 then
    status.all := 3
  else
    status.all := 0;
  elem_ptr_calls_$checked := status.all = 0;
end;

procedure elem_ptr_calls_$fill;
begin
  tag[2] := 0;
end;

{ The first free slot (IO_$ACQUIRE's first loop): `exit' after a cleared
  status, then the status tested. }
procedure elem_ptr_calls_$find(out unit: integer; out status: status_t);
label
  done;
var
  i: integer;
begin
  status.all := 7;
  for i := 0 to 3 do
    if tag[i] = 0 then begin
      unit := i;
      status.all := 0;
      exit;
    end;
  if status.all <> 0 then
    goto done;
  exec_$mark(2);
done:
  exec_$mark(1);
end;

{ IO_$ACQUIRE's second loop: one of two errors, then the status tested. }
procedure elem_ptr_calls_$chain(i: integer; out status: status_t);
  options(val_param);
label
  done;
begin
  status.all := 0;
  exec_$mark(3);
  if i = 1 then
    status.all := 5
  else if i = 2 then
    status.all := 6;
  if status.all <> 0 then
    goto done;
  exec_$mark(4);
done:
end;
