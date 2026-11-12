{ Execution test (bead domain-pascal-338r, domain-c's gap 23): `k in c'
  for a one-word constant set c and a char or `0..255' element, or an
  `integer32' one (codegen-patterns 5.1; best effort, no kernel site):
  the constant a longword, `btst Dk,Dn', as the DOMAIN C compiler's
  alias_ptr_node (reported: cc.m68k 0x073BC8 `move.l #0xC000,D1; ...;
  btst D0,D1').  The elements are within 0..31 here; one above 31 is
  tested modulo 32, as by the original's register `btst'.

  call: IN_WORD_CONST_BYTE_$RUN w:14 w:15 w:3 w:13
  expect:
    1
    1
    1
    1
    0
    0
    0
    1
}
module in_word_const_byte;
%include 'exec.ins.pas';

type
  r_t = packed record c: 0..255; f: boolean; end;
var
  r: r_t;
  l: integer32;

procedure one(k: integer); options(val_param);
begin
  r.c := k;
  if r.c in [14, 15] then exec_$logv_w(1) else exec_$logv_w(0);
  l := k;
  if l in [1, 13..15] then exec_$logv_w(1) else exec_$logv_w(0);
end;

procedure in_word_const_byte_$run(i: integer; j: integer; k: integer; m: integer);
  options(val_param);
begin
  one(i);
  one(j);
  one(k);
  one(m);
end;
