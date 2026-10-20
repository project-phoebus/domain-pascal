{ Execution test (bead domain-pascal-kplu): constants passed by address
  from more than 32K away, in routines with the unit's data block in A5.

  The image's form (DIR, codegen-patterns 4, "What A5 points at"): the
  constant cell's address is kept in a header in front of the data block,
  below A5, and the far reference reads it from there.  DIR's block
  0xE7DBF8 starts with 0xE4B33C and 0xE4B230, A5 = 0xE7DC00, and
  DIR_$CLEANUP pushes them with `move.l (-4,A5),-(SP)' (0xE5365A) and
  `move.l (-8,A5),-(SP)' (0xE539DC).

  - The header cells are numbered by the constant cells' addresses, the
    lowest at -4 (DIR: 0xE4B230 at -4, 0xE4B33C at -8).  The constants
    are placed after BF5_$FIRST, the word 8 first, then 777777, the
    texts and 888888, so 8 is at -4, 777777 at -8, 'hgfedcba' at -12 and
    888888 at -16, whatever the order of their far uses.
  - far_worker, internal, inherits A5 (it uses `c'): 888888 is pushed
    from -16; the text constant copied into `t' by address comes from
    -12 (`lea' becomes `movea.l (-12,A5),A0': best effort, not seen);
    exec_$log_mem's 8 from -4.
  - BF5_$FAR loads A5 (it uses `c'): 777777 from -8, twice from the one
    cell, and 888888 from far_worker's -16 cell.
  - bf5_$nodata uses no static data, so it has no data base: its far
    constant is copied after it instead (bead domain-pascal-pxov).
  - The block: four header cells, then the variables at their usual
    offsets from A5; BF5_$FIRST checks that `c' and `t' are where the
    other routines find them.

  call: BF5_$FIRST
  call: BF5_$FAR
  call: BF5_$FIRST
  warns: best effort (not byte-identical): a constant's address beyond the reach of a 16-bit displacement loaded into an address register
  warns: best effort (not byte-identical): a constant cell beyond the reach of a 16-bit displacement
  expect:
    777777
    888888
    mem 61 62 63 64 65 66 67 68
    0
    777777
    888888
    777777
    1
    888888
    mem 68 67 66 65 64 63 62 61
    2
    999999
    3
    777777
    888888
    mem 68 67 66 65 64 63 62 61
    3
}
module be_far_a5;
%include 'exec.ins.pas';

type
  bigset = set of 0..1023;
  text8 = array[1..8] of char;
const
  cs0 = [0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448, 480, 512, 544, 576, 608, 640, 672, 704, 736, 768, 800, 832, 864, 896, 928, 960, 992];
  cs1 = [1, 33, 65, 97, 129, 161, 193, 225, 257, 289, 321, 353, 385, 417, 449, 481, 513, 545, 577, 609, 641, 673, 705, 737, 769, 801, 833, 865, 897, 929, 961, 993];
var
  c: integer;
  t: text8;
  s: bigset;

procedure bf5_$first;
begin
  exec_$log_l(777777);
  exec_$log_l(888888);
  if c = 0 then
    t := 'abcdefgh';
  if c = 99 then begin
    t := 'hgfedcba';
    exec_$log_l(999999);
  end;
  exec_$log_mem(t, 8);
  exec_$logv_w(c);
end;

procedure bf5_$fill;
begin
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

procedure far_worker;
  options(internal);
begin
  exec_$log_l(888888);
  t := 'hgfedcba';
  exec_$log_mem(t, 8);
  c := c + 1;
  exec_$logv_w(c);
end;

procedure bf5_$nodata;
begin
  exec_$log_l(999999);
end;

procedure bf5_$far;
begin
  exec_$log_l(777777);
  exec_$log_l(888888);
  exec_$log_l(777777);
  c := c + 1;
  exec_$logv_w(c);
  far_worker;
  bf5_$nodata;
  c := c + 1;
  exec_$logv_w(c);
end;
