{ Golden: codegen-patterns 4, 1.2, 1.5, 1.1: LOG_$READ 0xE17800.

    link.w A6,#0; pea (A5); lea 0xE2B280,A5   ; A5 for the worker
    move.l (0x10,A6),-(SP)                    ; actual_len, passed on
    movea.l (0xc,A6),A0; move.w (A0),-(SP)    ; max_len^, by value
    clr.w -(SP)                               ; offset 0
    move.l (8,A6),-(SP)                       ; buffer, passed on
    bsr.w 0xE17778
    movea.l (-4,A6),A5; unlk A6; rts          ; no pop

  An exported routine loads A5 when it uses its unit's data directly or
  through a module-local (internal) callee, which inherits A5: LOG_$READ
  has no A5 operand of its own, its worker 0xE17778 reads (0x14,A5).
  A by-address parameter passed on by address is its slot, `move.l
  (d,A6),-(SP)'.  The worker here is a stand-in: dpc cannot compile
  0xE17778 yet (tests/conformance/pending/log_read.pas). }
module log_;
type
  pinteger = 0..65535;
  log_$buf_t = array[1..512] of integer;
var
  pad: array[1..5] of integer32;
  last_len: pinteger;

procedure read_internal(var buffer: log_$buf_t; offset: pinteger;
                        max_len: pinteger; var actual_len: pinteger);
  internal;
begin
  actual_len := last_len;
end;

procedure log_$read(var buffer: log_$buf_t; in max_len: pinteger;
                    out actual_len: pinteger);
begin
  read_internal(buffer, 0, max_len, actual_len);
end;
