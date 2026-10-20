{ The other unit of be_h4ba_mem.pas: a packed array of booleans and an
  array of records that be_h4ba_mem.pas reaches as another unit's
  (codegen-patterns 5.2), and a routine that logs the booleans' bytes. }
module be_h4ba_data;
%include 'exec.ins.pas';

type
  rec6 = record a, b, c: integer; end;
var
  bh_$bits: packed array[1..64] of boolean;
  bh_$recs: array[1..40] of rec6;
  bh_$k: integer;
define bh_$bits, bh_$recs, bh_$k;

procedure bh_$dump;
begin
  exec_$log_mem(bh_$bits, 8);
end;
