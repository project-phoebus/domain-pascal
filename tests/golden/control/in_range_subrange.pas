{ Golden: `in_range' of integer subranges stored in words (ref
  "In_range", p. 4-104; codegen-patterns 10.1; bead domain-pascal-bzkf),
  SIO_$K_SET_PARAM's checks (../source sio/k_set_param.pas: `if not
  in_range(params.parity)'): 0..3 as one unsigned compare against 3 in
  memory, `cmpi.w #0x3,(0x14,A2); bhi' (0xE68252, 0xE682A6); 1..3 widened
  with zeros, the lower bound subtracted, `clr.l D1; move.w (0x12,A2),D1;
  subq.l #0x1,D1; cmpi.l #0x2,D1; bhi' (0xE68278..0xE68286).  The
  checks' error stores are one copy, each check's `bhi.s 0xE682D4'
  going straight to it (cross-jumping, codegen-patterns 7.1 "Identical
  ends", bead domain-pascal-k1da): here the inner check's store is the
  outer's, `bhi.s 4b'. }
module in_range_subrange;
type
  stop_bit_t = 1..3;
  parity_t = 0..3;
  params_t = record
    pad: array[1..9] of integer;
    stop_bits: stop_bit_t;
    parity: parity_t;
  end;
  status_t = integer32;
var
  last: parity_t;
procedure set_param(in params: params_t; out status: status_t);
begin
  status := 0;
  if not in_range(params.parity) then begin
    status := 16#360002;
    return;
  end;
  if not in_range(params.stop_bits) then begin
    status := 16#360002;
    return;
  end;
  last := params.parity;
end;
