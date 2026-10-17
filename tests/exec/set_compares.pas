{ Execution test (bead domain-pascal-2i6f): `=' and `<>' of register-
  sized sets (codegen-patterns 5.1, "Set comparisons"), compared as
  integers: a word intersection against a constant (PROC1_$SET_PRIORITY
  0xE152AC `moveq #11,D0; and.w (84,A4),D0; cmpi.w #8,D0; bne.s') in a
  condition and as a value (OS_$INIT 0xE339EC..0xE33A10 `andi.w
  #-0x7ffc,D0; cmpi.w #4,D0; seq D0'), a held word set against a constant
  (RING_$SENDP 0xE75C38 `cmpi.w #0x14,D7'), two longword intersections
  (SIO_$K_SET_PARAM 0xE680F6..0xE68102 `and.l; and.l; cmp.l D0,D1'), a
  set variable against another; and set constants passed by address,
  their code-section cells (codegen-patterns 1.5, "Set constants": TERM_$
  READ's 0xE66896 `0001', 0xE66898 `0000'; TERM_$INQUIRE's longword
  masks 0xE671AC..).

  call: SET_COMPARES_$RUN w:8 w:4 l:1 l:1
  call: SET_COMPARES_$RUN w:9 w:5 l:3 l:2
  call: SET_COMPARES_$RUN w:-32760 w:20 l:-1 l:-2
  call: SET_COMPARES_$RUN w:3 w:-32764 l:0 l:0
  call: SET_COMPARES_$CONSTS
  expect:
    true
    true
    false
    false
    true
    true
    false
    true
    false
    true
    false
    true
    true
    true
    true
    true
    false
    true
    false
    false
    false
    false
    true
    true
    mem 00 01
    mem 00 00
    mem 00 08
    mem 00 00 00 01
    mem 80 00 00 00
    mem 00 00 00 10
    mem 00 01
}
module set_compares;
%include 'exec.ins.pas';

type
  w_t = set of 0..15;
  l_t = set of 0..31;
  rec = record
    pad: integer;
    flags: w_t;
  end;
var
  b: boolean;

function masked(in r: rec): boolean;
begin
  masked := false;
  if r.flags * [0, 1, 3] = [3] then masked := true;
end;

function masked_value(in w: w_t): boolean;
begin
  masked_value := w * [2, 15] = [2];
end;

procedure held(in w: w_t);
var h: w_t;
begin
  h := w;
  exec_$clobber;
  if h = [2, 4] then b := true else b := false;
  exec_$log_bool(b);
end;

procedure two(in x, y: l_t);
begin
  if x * [0] <> y * [0] then b := true else b := false;
  exec_$log_bool(b);
  if x = y then b := true else b := false;
  exec_$log_bool(b);
end;

procedure set_compares_$run(i: integer; j: integer; k: integer32; l: integer32);
  options(val_param);
var
  r: rec;
begin
  r.flags := w_t(i);
  exec_$log_bool(masked(r));
  exec_$log_bool(masked_value(w_t(j)));
  held(w_t(j));
  two(l_t(k), l_t(l));
  if w_t(i) <> w_t(j) then b := true else b := false;
  exec_$log_bool(b);
end;

procedure word_cell(in o: w_t);
begin
  exec_$log_mem(o, 2);
end;

procedure long_cell(in o: l_t);
begin
  exec_$log_mem(o, 4);
end;

procedure set_compares_$consts;
begin
  word_cell([0]);
  word_cell([]);
  word_cell([3]);
  long_cell([0]);
  long_cell([31]);
  long_cell([4]);
  word_cell([0]);
end;
