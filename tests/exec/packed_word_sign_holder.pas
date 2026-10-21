{ Execution test: records that only inherit a field's `[aligned(0)]'
  (ref 3.15.6.2 pp. 3-67..3-68) through a pointer and a var parameter
  (bead domain-pascal-epjn review).  One of 4 bytes or more is word
  aligned (ref 3.10.5.6), so its word field and its copy compile (d5b's
  `q^.x', `h.x; gh := h') and a boolean at an even offset of it is the
  word's sign, `tst.w' (codegen-patterns 5.3).  A 2-byte one may lie at an
  odd offset of a record holding it (dpc's layout of a byte-aligned field
  that crosses one word boundary), so its boolean stays `tst.b'.

  call: PACKED_WORD_SIGN_HOLDER_$RUN
  expect:
    1234
    1234
    121
    true
    true
    1
    true
}
module packed_word_sign_holder;
%include 'exec.ins.pas';
type
  pb_t = packed record
      a: boolean;
      b: 0..127;
      end;
  ab_t = [aligned(0)] pb_t;
  holder_t = record
      c: char;
      f: ab_t;
      d: char;
      e: ab_t;
      x: integer;
      end;
  hp = ^holder_t;
  w_t = record
      f: ab_t;
      end;
  wp = ^w_t;
  outer_t = record
      c: char;
      w: w_t;
      d: char;
      end;
var
  gh: holder_t;
  h: holder_t;
  o: outer_t;
  g: integer;
  ok: boolean;

procedure via_ptr(q: hp); options(val_param);
begin
  g := q^.x;
  exec_$logv_w(g);
end;

procedure via_var(var r: holder_t);
begin
  g := r.x;
  exec_$logv_w(g);
  gh := r;
  exec_$log_c(gh.d);
  ok := r.e.a;
  exec_$log_bool(ok);
end;

procedure two_byte(p: wp); options(val_param);
begin
  ok := p^.f.a;
  exec_$log_bool(ok);
  g := 0;
  if p^.f.a then
    g := 1;
  exec_$logv_w(g);
end;

procedure packed_word_sign_holder_$run;
begin
  h.c := 'x';
  h.d := 'y';
  h.x := 1234;
  h.f.b := 3;
  h.e.b := 3;
  ok := true;
  h.e.a := ok;
  via_ptr(addr(h));
  via_var(h);
  o.c := 'p';
  o.d := 'q';
  o.w.f.b := 7;
  o.w.f.a := ok;
  two_byte(addr(o.w));
  ok := o.w.f.a;
  exec_$log_bool(ok);
end;
