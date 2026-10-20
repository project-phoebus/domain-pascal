{ The record be_merge.pas and be_merge_knob.pas store to as another
  unit's (codegen-patterns 13). }
module be_merge_data;

type
  pair_t = record a: integer; b: integer; end;
var
  bm_$ext: pair_t;
define bm_$ext;
