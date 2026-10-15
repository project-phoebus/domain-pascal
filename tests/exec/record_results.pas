{ Execution test: functions returning a 4-byte record, base.ins.pas's
  `status_$t' (codegen-patterns 3, "Record results"; bead
  domain-pascal-gcpo).  The result comes back in D0 as a longword: the
  callee copies its record into the result variable's frame slot and
  loads it; the caller stores D0, compares the status right after the
  store (codegen-patterns 7.1), reads its fields (0xC5240007: fail set,
  subsys 16#45, modc 16#24, code 7), assigns it to a variable of the
  unit and to an `out' parameter, and passes it on by address.  The runner's `-> l' prints the
  result register of a direct call.

  call: RECORD_RESULTS_$MAKE l:0x00120035 -> l
  call: RECORD_RESULTS_$MAKE l:-1 -> l
  call: RECORD_RESULTS_$FIELDS l:0xC5240007
  call: RECORD_RESULTS_$FIELDS l:0
  call: RECORD_RESULTS_$CHAIN l:0x00120035 @l:99!
  call: RECORD_RESULTS_$CHAIN l:0x00240003 @l:99!
  call: RECORD_RESULTS_$CHAIN l:7 @l:99!
  call: RECORD_RESULTS_$LAST
  expect:
    => 1179701
    => -1
    -987496441
    1
    69
    7
    0
    0
    0
    0
    -5
    1179701
    out 1179701
    2359299
    5
    out 2359299
    7
    out 7
    7
    3
}
module record_results;
%include 'exec.ins.pas';

type
  status_$t = packed record case integer of
    1: (fail: boolean; subsys: 0..127; modc: 0..255; code: integer);
    2: (all: integer32);
  end;
var
  last: status_$t;
  calls: integer32 := 0;

function record_results_$make(v: integer32): status_$t; options(val_param);
var
  st: status_$t;
begin
  st.all := v;
  record_results_$make := st;
end;

{ the status by address (the default convention), returned }
function pass_on(in st: status_$t): status_$t;
begin
  pass_on := st;
end;

procedure log_status(in st: status_$t);
begin
  exec_$log_l(st.all);
end;

procedure record_results_$fields(v: integer32); options(val_param);
var
  st: status_$t;
  b: integer32;
begin
  st := record_results_$make(v);
  exec_$log_l(st.all);
  if st.fail then b := 1 else b := 0;
  exec_$log_l(b);
  b := st.subsys;
  exec_$log_l(b);
  exec_$log_w(st.code);
end;

procedure record_results_$chain(v: integer32; out res: status_$t);
  options(val_param);
var
  st, again: status_$t;
begin
  st := record_results_$make(v);
  if st.all = 16#120035 then
    exec_$logv_l(-5);
  again := pass_on(st);
  log_status(again);
  if again.code = 3 then
    exec_$logv_l(5);
  last := pass_on(again);
  calls := calls + 1;
  res := last;
end;

procedure record_results_$last;
begin
  exec_$log_l(last.all);
  exec_$log_l(calls);
end;
