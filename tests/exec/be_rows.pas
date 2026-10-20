{ Execution test (bead domain-pascal-yqf6, best effort): an element of an
  array that is itself an element (a row, `a[i][j]`, `a[i, j]`, `a[i].f[j]`)
  or in the enclosing routine's frame: the array's operand formed first,
  its register the base, then the index (codegen-patterns 5.2, "Rows and
  nested elements").  Rows of another unit's array (`ast_pmaps[seg][off]`
  at PMAP_$WRITE_PAGE, `file_$lock_table[asid, slot]` at FILE_$CHECK_PROT),
  of the unit's own data, of a frame array, through a pointer
  (`handle^.blocks[index].slots[sub]` at DIR_$OLD_READ_ENTRIES), a field's
  array in a row (`xns_idp_$channels[channel].port_active[port]`), a set
  updated in place in a row, and a nested routine's element of its
  parent's array (VFMT_$MAIN's `spec[i]`).

  link: be_rows_data.mod.pas
  warns: best effort (not byte-identical): an element of an array that is an element itself
  call: BE_ROWS_$OTHER w:2 w:3 l:1234
  call: BE_ROWS_$OTHER w:5 w:0 l:-9
  call: BE_ROWS_$MODE w:2 w:1
  call: BE_ROWS_$OWN w:3 w:2
  call: BE_ROWS_$FRAME w:1 w:4
  call: BE_ROWS_$PTR w:2 w:1
  call: BE_ROWS_$NESTED w:3
  expect:
    1234
    true
    false
    true
    -9
    true
    false
    true
    21
    32
    14
    -14
    22
    7
    30
}
module be_rows;
%include 'exec.ins.pas';

type
  ent_t = record
    a: integer32;
    flags: set of 0..15;
  end;
  chan_t = record
    id: integer;
    active: array[0..4] of boolean;
    cnt: array[1..3] of integer;
  end;
  blk_t = record
    hdr: integer32;
    slots: array[0..3] of integer;
  end;
  dir_t = record
    n: integer;
    blocks: array[1..3] of blk_t;
  end;
  dir_ptr_t = ^dir_t;
var
  rows_$tbl: extern array[1..5] of array[0..3] of ent_t;
  rows_$mode: extern array[0..2, 0..3] of integer;
  grid: array[1..4, 0..5] of integer;
  chans: array[0..3] of chan_t;
  d: dir_t;
  dp: dir_ptr_t;

procedure be_rows_$other(seg, off: integer; v: integer32); options(val_param);
begin
  rows_$tbl[seg][off].a := v;
  rows_$tbl[seg][off].flags := [];
  exec_$log_l(rows_$tbl[seg, off].a);
  rows_$tbl[seg][off].flags := rows_$tbl[seg][off].flags + [15];
  exec_$log_bool(15 in rows_$tbl[seg][off].flags);
  exec_$log_bool(13 in rows_$tbl[seg][off].flags);
  rows_$tbl[seg][off].flags := rows_$tbl[seg][off].flags - [15];
  exec_$log_bool(rows_$tbl[seg][off].flags = []);
end;

procedure be_rows_$mode(s, m: integer); options(val_param);
var
  r: integer;
begin
  rows_$mode[s, m] := 20 + m;
  r := rows_$mode[s, m];
  exec_$log_w(r);
end;

procedure be_rows_$own(i, j: integer); options(val_param);
begin
  grid[i, j] := i * 10 + j;
  grid[i][j + 1] := 0;
  exec_$log_w(grid[i][j]);
  chans[j].active[i] := true;
  chans[j].cnt[i] := 14;
  exec_$log_w(chans[j].cnt[i]);
end;

procedure be_rows_$frame(i, j: integer); options(val_param);
var
  g: array[0..2, 1..5] of integer;
begin
  g[i, j] := -14;
  exec_$log_w(g[i][j]);
end;

procedure be_rows_$ptr(b, s: integer); options(val_param);
begin
  dp := addr(d);
  dp^.blocks[b].slots[s] := b * 10 + s;
  with dp^.blocks[b] do
    slots[s] := slots[s] + 1;
  exec_$log_w(dp^.blocks[b].slots[s]);
end;

procedure be_rows_$nested(i: integer); options(val_param);
var
  spec: array[1..4] of integer;
  r: integer;

  procedure inner;
  begin
    r := spec[i];
  end;

begin
  spec[1] := 5;
  spec[2] := 6;
  spec[3] := 7;
  spec[4] := 8;
  inner;
  exec_$log_w(r);
  spec[i] := 30;
  inner;
  exec_$log_w(r);
end;
