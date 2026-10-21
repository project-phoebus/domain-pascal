{ Execution test: a boolean in bit 7 of byte 0 of a byte-aligned
  (`[aligned(0)]', ref 3.15.6.3 p. 3-69) packed record at an odd address,
  reached through a var parameter, a pointer and `with p^': read as its
  byte, `tst.b' (codegen-patterns 5.3, "Boolean field, test"; bead
  domain-pascal-g9kx review).  The word form, `tst.w', would raise an
  address error on the 68010.

  call: PACKED_WORD_SIGN_ODD_$RUN
  expect:
    true
    1
    true
    1
    true
    1
}
module packed_word_sign_odd;
%include 'exec.ins.pas';
type
  pb_t = packed record
      a: boolean;
      b: 0..127;
      end;
  ab_t = [aligned(0)] pb_t;
  abp = ^ab_t;
  bo_t = record
      c: char;
      f: ab_t;
      d: char;
      end;
var
  bo: bo_t;
  ok: boolean;
  n: integer;

procedure via_var(var r: ab_t);
begin
  ok := r.a;
  exec_$log_bool(ok);
  n := 0;
  if r.a then n := 1;
  exec_$logv_w(n);
end;

procedure via_ptr(q: abp); options(val_param);
begin
  ok := q^.a;
  exec_$log_bool(ok);
  n := 0;
  if q^.a then n := 1;
  exec_$logv_w(n);
end;

procedure via_with(q: abp); options(val_param);
begin
  with q^ do begin
    ok := a;
    exec_$log_bool(ok);
    n := 0;
    if a then n := 1;
    exec_$logv_w(n);
  end;
end;

procedure packed_word_sign_odd_$run;
var q: abp;
begin
  bo.c := 'x';
  bo.d := 'y';
  bo.f.b := 5;
  ok := true;
  bo.f.a := ok;
  via_var(bo.f);
  q := addr(bo.f);
  via_ptr(q);
  via_with(q);
end;
