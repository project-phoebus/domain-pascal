{ Execution test (bead domain-pascal-338r, domain-c's gap 2): a `case'
  whose labels are between the sparsest jump table and the densest
  compare chain of the kernels (codegen-patterns 7.3), 12 labels over 35
  here as in the DOMAIN C compiler's errorlst (reported: cc.m68k
  0x0498B0, a table).  Best effort: a table; labels outside the range
  and missing ones go to `otherwise'.

  call: CASE_BAND_$RUN w:48 w:57 w:58 w:76
  call: CASE_BAND_$RUN w:82 w:83 w:47 w:-1
  expect:
    1
    1
    4
    2
    3
    4
    4
    4
}
module case_band;
%include 'exec.ins.pas';

function sel(k: integer): integer; options(val_param);
begin
  case k of
    48, 49, 50, 51, 52, 53, 54, 55, 56, 57: sel := 1;
    76: sel := 2;
    82: sel := 3;
    otherwise sel := 4;
  end;
end;

procedure case_band_$run(i: integer; j: integer; k: integer; l: integer);
  options(val_param);
begin
  exec_$log_w(sel(i));
  exec_$log_w(sel(j));
  exec_$log_w(sel(k));
  exec_$log_w(sel(l));
end;
