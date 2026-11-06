{ Golden: codegen-patterns 2, "Frame layout": a record of under 8 bytes with a
  longword field is placed at a multiple of 4, also when `[aligned(1)]` makes
  it word aligned (Apollo's time_$clock_t: /com/calendar's clk at -0x8D4 under a
  word; the kernel's 18 clock locals, TIME_$CLOCK's callers).  Other objects of
  that size are not (WIN_$CINIT 0xE30350 `pea (-0xA,A6)` under a longword). }
module clock_aligned;

type clock_t = [aligned(1)] packed record
       case integer of
         1: (high: 0..2147483647; low: 0..65535);
         2: (high16: 0..65535; low32: 0..2147483647);
     end;
     words_t = record a, b, c: integer; end;

procedure get_clock(out c: clock_t); extern;
procedure get_words(out w: words_t); extern;

procedure p;
var n: integer;
    c: clock_t;
begin
  n := 1;
  get_clock(c);
  n := n + 1;
end;

procedure q;
var n: integer32;
    w: words_t;
begin
  n := 1;
  get_words(w);
  n := n + 1;
end;
