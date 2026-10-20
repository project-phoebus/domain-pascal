{ Execution test (bead domain-pascal-pxov): an array indexed by a
  boolean (`array[boolean]', bounds false..true, ref 3.5.3: false is 0,
  true 0xFF).  The index is made its ordinal by its top bit, `lsr.b #7',
  then widened, as SIO2681_$SET_LINE 0xE1D2F0..0xE1D2FC does for
  `baud_masks[chip^.config1.w < 0]' (`smi D1; clr.w D5; lsr.b #7,D1;
  move.b D1,D5; add.w D5,D5').  A boolean variable, a comparison, an
  element read, written and updated, word and longword elements.

  call: BI_$PICK w:7 -> w
  call: BI_$PICK w:-2 -> w
  call: BI_$PICK w:3 -> w
  warns: best effort (not byte-identical): an array index of boolean type
  expect:
    22
    100000
    => 56
    11
    200000
    => 56
    11
    100000
    => 57
}
module be_bool_index;
%include 'exec.ins.pas';

var
  tab: array[boolean] of integer;
  masks: array[false..true] of integer32;

function bi_$pick(w: integer): integer;
  options(val_param);
var
  b: boolean;
begin
  tab[false] := 11;
  tab[true] := 22;
  masks[false] := 100000;
  masks[true] := 200000;
  b := w > 5;
  exec_$logv_w(tab[b]);
  exec_$logv_l(masks[w < 0]);
  tab[w = 3] := tab[w = 3] + 1;
  bi_$pick := tab[true] * 2 + tab[false];
end;
