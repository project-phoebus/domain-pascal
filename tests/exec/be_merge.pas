{ Execution test (bead domain-pascal-5kgg): the best-effort paths of the
  store-merge refusals in a unit that does not merge (codegen-patterns 13,
  "Store merging"; `dpc_m68k::isel::merge`): stores the image does not
  settle are kept apart, in the source's order.

  - Two adjacent words copied from adjacent words (merged at
    ROUTE_$SHORT_PORT 0xE69C16, apart at RIP_$TABLE_D 0xE68ECA).
  - Two byte constants to adjacent bytes off A5.
  - Two word constants to another unit's adjacent words.

  call: BE_MERGE_$COPY
  call: BE_MERGE_$BYTES
  call: BE_MERGE_$EXT
  link: be_merge_data.mod.pas
  warns: best effort (not byte-identical): a store adjacent to the previous statement's store, both copying
  warns: best effort (not byte-identical): a store adjacent to the previous statement's store, of two constants
  expect:
    5
    6
    97
    98
    7
    8
}
module be_merge;
%include 'exec.ins.pas';
type
  pair_t = record a: integer; b: integer; end;
  bytes_t = record x: char; y: char; end;
var
  src, dst: pair_t;
  bb: bytes_t;
  bm_$ext: extern pair_t;

procedure be_merge_$copy;
begin
  src.a := 5;
  exec_$clobber;
  src.b := 6;
  exec_$clobber;
  dst.a := src.a;
  dst.b := src.b;
  exec_$log_w(dst.a);
  exec_$log_w(dst.b);
end;

procedure be_merge_$bytes;
begin
  bb.x := 'a';
  bb.y := 'b';
  exec_$log_c(bb.x);
  exec_$log_c(bb.y);
end;

procedure be_merge_$ext;
begin
  bm_$ext.a := 7;
  bm_$ext.b := 8;
  exec_$log_w(bm_$ext.a);
  exec_$log_w(bm_$ext.b);
end;
