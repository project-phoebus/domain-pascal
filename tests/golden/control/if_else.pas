{ Golden: codegen-patterns 7 (`if', `return', branch sizes), 1.2, 12.2.
  The shapes of LOG_$READ's worker 0xE17778 (the module-local routine
  of ../source/domain_os/log/read_internal.c), 0xE1778C..0xE177CC:

    tst.l (0x14,A5); bne.s 1f; clr.w (A0); bra.s <epilogue>
                                 `if ptr = nil then begin ...; return end'
    move.w #0x400,(A0)
    cmpi.w #0x400,D1; bcc.s 3f   `if max_len < 0x400 then' (unsigned)
    ...
    cmpi.w #0x400,D0; bls.s 4f; clr.w (A0); bra.s 5f; 4: ...; 5:
                                 `if offset > 0x400 then ... else ...'

  The `then' part branches over the `else' part to the join; a `then'
  part that ends in `return' branches to the epilogue instead (here the
  `unlk', 0xE177F6 in the image is the register restore).  The
  parameters are loaded at entry (0xE17780..0xE17788); unlike the
  original the length is a by-value pointer here (the original's is a
  by-address slot, bead domain-pascal-b1v).  Branches are `.s' when the
  target is in reach (`if_far' needs `.w'). }
module ctl;
type
  pinteger = 0..65535;
  pinteger_ptr = ^pinteger;
var
  pad: array[1..5] of integer32;
  logfile_ptr: univ_ptr;
  b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13: extern integer32;

procedure if_head(offset: pinteger; max_len: pinteger; actual_len: pinteger_ptr);
  internal;
begin
  if logfile_ptr = nil then begin
    actual_len^ := 0;
    return;
  end;
  actual_len^ := 16#400;
  if max_len < 16#400 then
    actual_len^ := 2;
  if offset > 16#400 then actual_len^ := 0
  else actual_len^ := 5;
  if offset > 16#400 then
    if max_len > 16#400 then actual_len^ := 0
    else actual_len^ := 1;
end;

procedure if_far; internal;
begin
  if logfile_ptr = nil then begin
    b1 := 16#12345; b2 := 16#12345; b3 := 16#12345; b4 := 16#12345;
    b5 := 16#12345; b6 := 16#12345; b7 := 16#12345; b8 := 16#12345;
    b9 := 16#12345; b10 := 16#12345; b11 := 16#12345; b12 := 16#12345;
    b13 := 16#12345;
  end;
  b1 := 0;
end;
