{ Execution test (codegen-patterns 5.2, "String constants"): a string constant
  assigned to an array of 2, 4 or 6 bytes in the data and the frame, padded with
  blanks, and through a pointer, with the words around it left alone.

  call: STRSHORT_$RUN
  expect:
    1
    1
    1
    1
    1
    1
}
module strshort;
%include 'exec.ins.pas';

type name2 = array[1..2] of char;
     name4 = array[1..4] of char;
     name6 = array[1..6] of char;
     holder = record before: integer; name: name4; after: integer; end;

var two: name2;
    four: name4;
    six: name6;

procedure log(ok: boolean); internal;
begin
  if ok then exec_$log_l(1) else exec_$log_l(0);
end;

procedure through(out n: name4);
begin
  n := 'wxyz';
end;

procedure strshort_$run;
var h: holder;
    l: name2;
    k: holder;
begin
  two := 'ab';
  log((two[1] = 'a') and (two[2] = 'b'));
  four := 'cd';
  log((four[1] = 'c') and (four[2] = 'd') and (four[3] = ' ') and (four[4] = ' '));
  six := 'Seven';
  log((six[1] = 'S') and (six[4] = 'e') and (six[5] = 'n') and (six[6] = ' '));
  h.before := 16#1234;
  h.after := 16#5678;
  h.name := 'efgh';
  log((h.name[1] = 'e') and (h.name[4] = 'h') and (h.before = 16#1234) and (h.after = 16#5678));
  l := 'i';
  log((l[1] = 'i') and (l[2] = ' '));
  k.before := 16#1234;
  k.after := 16#5678;
  through(k.name);
  log((k.name[1] = 'w') and (k.name[4] = 'z') and (k.before = 16#1234) and (k.after = 16#5678));
end;
