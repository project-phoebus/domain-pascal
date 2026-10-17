{ Execution test: fields and elements of a type transfer of a variable
  (ref "Type Transfer Functions", p. 4-197: no bits changed; bead
  domain-pascal-w0rj): the bytes of the variable itself, big-endian, at
  the view's offsets.  A longword `val_param' read as two words, twice
  (from its slot, never loaded whole: PROC1_$CREATE_P 0xE15156,
  0xE15162); a UID local read as words and bytes; a `var' longword read
  as bytes; an integer32 local as an array of words; an element of a
  table of UIDs at a run-time index; a transfer of a field of a
  transfer.

  call: TF_$MAIN l:262154 @l:305419896 w:3
  expect:
    4
    10
    4
    10
    -21555
    4660
    52
    86
    -1
    -2
    104
    -3
    360
}
module tf;
%include 'exec.ins.pas';

type
  args_t = record
    stack_size: integer;
    proc_type: integer;
  end;
  bytes_t = record
    b0, b1, b2, b3: char;
  end;
  uid_t = record
    high, low: integer32;
  end;
  view_t = record
    case integer of
      1: (uid: uid_t);
      2: (w0, w1: integer;
          w2: 0..65535;
          w3: integer);
      3: (b0, b1, b2, b3: char);
  end;
  words_t = array[1..2] of integer;
var
  tbl: array[1..4] of uid_t;

procedure tf_$main(a: integer32; var r: integer32; i: integer);
  options(val_param);
var
  u: uid_t;
  l: integer32;
  k: integer;
begin
  exec_$logv_w(args_t(a).stack_size);
  exec_$logv_w(args_t(a).proc_type);
  exec_$logv_w(args_t(a).stack_size);
  exec_$logv_w(args_t(a).proc_type);
  u.high := 16#12345678;
  u.low := 16#ABCDEF00;
  exec_$logv_w(view_t(u).w2);
  exec_$logv_w(view_t(u).w0);
  exec_$logv_w(ord(view_t(u).b1));
  exec_$logv_w(ord(bytes_t(r).b2));
  l := -2;
  exec_$logv_w(words_t(l)[1]);
  exec_$logv_w(words_t(l)[2]);
  for k := 1 to 4 do begin
    tbl[k].high := 16#78 * k;
    tbl[k].low := -k;
  end;
  exec_$logv_w(ord(view_t(tbl[i]).b3));
  exec_$logv_w(view_t(tbl[i]).w3);
  exec_$logv_w(words_t(view_t(tbl[i]).uid.high)[2]);
end;
