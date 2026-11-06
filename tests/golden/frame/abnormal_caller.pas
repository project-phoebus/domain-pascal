{ Golden: codegen-patterns 9, `abnormal`: a routine calling one saves
  D2-D7/A2-A4 (MAC_$SEND 0xE0BB12, the module-local 0xE45A60), and A5 when it
  loads it, and reads its parameters from their slots (XNS_IDP_$SEND 0xE18A66:
  `(0x14)` at 0xE18A76 and again after the call).  A routine that calls none
  loads them at entry, as it always does. }
module abnormal_caller;

type rec_t = record w: array[1..16] of integer32; end;
     status_t = integer32;

var count: integer32;

function cleanup(out r: rec_t): status_t; options(abnormal, extern);
procedure rls_cleanup(in out r: rec_t); extern;
procedure note(in s: status_t); extern;

procedure worker(var a: integer32); internal;
var r: rec_t; st: status_t;
begin
  a := 0;
  st := cleanup(r);
  note(st);
  rls_cleanup(r);
end;

procedure entry(var a: integer32);
var r: rec_t; st: status_t;
begin
  a := 0;
  count := 0;
  st := cleanup(r);
  note(st);
  rls_cleanup(r);
  worker(a);
end;

procedure plain(var a, b: integer32);
begin
  a := 0;
  b := 0;
  a := 1;
end;
