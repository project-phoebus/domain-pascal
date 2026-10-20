{ Execution test (bead domain-pascal-pxov): a unit whose code is longer
  than a 16-bit displacement reaches (DIR's .TEXT, 0x8FD0 bytes).

  - A call of a routine of the unit more than 32K away: `jsr abs.l', as
    DIR's own (DIR_$CLEANUP 0xE535FE `jsr 0xe4b340', dir_$map_page), both
    forwards (BF_$FIRST to far_end) and backwards (far_end to early).
  - A conditional branch over more than 32K inside one routine (the
    `then' part of bf_$big): inverted around `jmp abs.l' (no site).
  - A constant cell passed by address more than 32K from the routine
    that first placed it: copied after the far routine (best effort).
  - A `for' loop whose body is longer than 32K: its `dbf' to a `jmp'
    after it (`dbf Dn,1f; bra.s 2f; 1: jmp (head).l; 2:'), so that the
    layout of the calls that span it stays exact (the review's
    rv_drift: gas lengthened a far `dbf' unseen and a `bsr.w' wrapped).

  The filler is set constants of 1024 elements stored a longword at a
  time (`move.l #imm', codegen-patterns 5.1).

  call: BF_$FIRST
  call: BF_$BIG w:0
  call: BF_$BIG w:1
  call: BF_$LOOP
  warns: best effort (not byte-identical): a `dbf` beyond the reach of a 16-bit displacement
  warns: best effort (not byte-identical): a branch beyond the reach of a 16-bit displacement
  warns: best effort (not byte-identical): a constant cell beyond the reach of a 16-bit displacement
  expect:
    424242
    424242
    10
    1
    2
    7
    3
    424242
    6
}
module be_far;
%include 'exec.ins.pas';

type
  bigset = set of 0..1023;
const
  cs0 = [0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 480, 512, 544, 576, 608, 640, 672, 704, 736, 768, 800, 832, 864, 896, 928, 960, 992];
  cs1 = [1, 33, 65, 97, 129, 161, 193, 225, 257, 289, 321, 353, 385, 417, 449, 481, 513, 545, 577, 609, 641, 673, 705, 737, 769, 801, 833, 865, 897, 929, 961, 993];
var
  s: bigset;
  c: integer;

procedure far_end(k: integer); options(val_param); forward;

procedure early(k: integer);
  options(val_param);
begin
  exec_$logv_w(k * 2);
end;

procedure bf_$first;
begin
  exec_$log_l(424242);
  far_end(5);
end;

procedure bf_$big(k: integer);
  options(val_param);
begin
  s := [];
  if k = 0 then begin
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    if 993 in s then exec_$logv_w(1);
  end;
  exec_$logv_w(k * 5 + 2);
end;

procedure bf_$loop;
var
  i: integer;
begin
  c := 0;
  for i := 1 to 3 do begin
    c := c + 1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
    s := cs0;
    s := cs1;
  end;
  exec_$logv_w(c);
  far_end(3);
end;

procedure far_end;
begin
  exec_$log_l(424242);
  early(k);
end;
