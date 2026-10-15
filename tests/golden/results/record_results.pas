{ codegen-patterns 3 ("Record results"), 7.1 ("A test right after a
  store"); bead domain-pascal-gcpo.  A function whose result is a 4-byte
  record (base.ins.pas's `status_$t') returns it in D0 as a longword, with
  no storage of the caller's.

  - cleanup: the caller pushes the arguments only and reads D0 after the
    pop, `move.l D0,<status>', then a compare of the status with a
    nonzero constant right after the store compares D0 (the 21 compiled
    calls of FIM_$CLEANUP that store into the frame, MAC_OS_$SEND
    0xE0B5F8..0xE0B608 `pea (-0x18,A6); jsr FIM_$CLEANUP; addq.w #4,SP;
    move.l D0,(-0x20,A6); cmpi.l #0x120035,D0'); the status passed on by
    address;
  - field: a field of another offset is compared in memory (no site);
  - into_static: the result stored to the unit's data;
  - dropped: `discard' of the call (VOLX_$SHUTDOWN called twice in a row
    at 0xE6D594, the first result unread);
  - get_status: the callee copies the record into its result variable's
    slot, below the locals, and loads that into D0 (PEB_$GET_STATUS
    0xE5AE52 `move.l (-8,A6),(-0x10,A6); move.l (-0x10,A6),D0';
    VOLX_$SHUTDOWN 0xE6B5B2); the record through a by-address parameter
    (`pass_on'). }
module rres;
type
  status_$t = packed record case integer of
    1: (fail: boolean; subsys: 0..127; modc: 0..255; code: integer);
    2: (all: integer32);
  end;
  cleanup_rec_t = record
    pad: array[1..6] of integer32;
  end;
var
  last: status_$t;
  count: integer;

function fim_$cleanup(out rec: cleanup_rec_t): status_$t;
  options(abnormal, extern);
procedure fim_$rls_cleanup(in out rec: cleanup_rec_t); extern;
procedure report(in st: status_$t); extern;
function probe(in unit: integer): status_$t; options(extern, val_param);

procedure cleanup;
var
  rec: cleanup_rec_t;
  st: status_$t;
begin
  st := fim_$cleanup(rec);
  if st.all = 16#120035 then begin
    count := count + 1;
    fim_$rls_cleanup(rec);
  end;
  report(st);
end;

procedure field;
var
  st: status_$t;
begin
  st := probe(3);
  if st.code = 5 then
    count := 0;
  report(st);
end;

procedure into_static;
begin
  last := probe(count);
end;

procedure dropped;
begin
  discard(probe(1));
  last := probe(2);
end;

function get_status(in unit: integer): status_$t;
var
  st: status_$t;
begin
  st := probe(unit);
  if st.all <> 0 then
    st.code := 1;
  get_status := st;
end;

function pass_on(var st: status_$t): status_$t;
begin
  pass_on := st;
end;
