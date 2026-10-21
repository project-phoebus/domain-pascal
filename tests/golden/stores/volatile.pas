{ Golden: volatile, atomic and device references (ref 3.15.1-3.15.3,
  3.15.7; codegen-patterns 13 "device registers", 12.4 R7, 12.5).
  - acknowledge: golden regalloc/cse's FIM_$ACKNOWLEDGE shape
    (0xE0A96C) with `proc1_$as_id' volatile: no computed value is reused
    across statements (12.4 R7 is for non-volatile variables); each
    statement reads the variable again and scales it again.
  - devs: device objects are cleared with `move.<s> #0' and set true with
    `move.b #0xff', never `clr'/`st', which read their operand on the
    68000 (WIN_ 0xE19112, RING 0xE758E2, 0xE0A33E); two adjacent device
    words are stored one by one (ref 3.15.3 p. 3-57: device suppresses
    merging); a device word is updated in place, `addq' to memory, as
    any other (the statement reads it, so the read is not "gratuitous",
    ref 3.15.3; SM_$CB's counters at sm_$check_status, the sau5 image;
    golden data/device_updates, bead domain-pascal-eaw7); a volatile
    boolean that is not device is still set with `st'.
  - local_v: a volatile local (ref 3.15.1's `i: [volatile] integer') is
    not held in a register (12.5) and is read again after its store
    (no test of the store's flags, 7.1).
  - atomic: `x := x + c' as one instruction (ref 3.15.2, Table 3-6). }
module volatile;
type
  ec_t = record
    value: integer32;
    head: integer32;
    tail: integer32;
  end;
  dev_word = [device] integer16;
  dev_flag = [device] boolean;
var
  proc1_$as_id: extern [volatile] integer;
  quit_ec: array[0..57] of ec_t;
  quit_inh: array[0..57] of boolean;
  deliv_ec: array[0..57] of ec_t;
  last_value: integer32;
  dw: extern dev_word;
  dw2: extern dev_word;
  df: extern dev_flag;
  vb: extern [volatile] boolean;
  a: extern [atomic] integer;
  b: extern [atomic] integer32;

procedure ec_$advance(var ec: ec_t); extern;

procedure acknowledge;
begin
  last_value := quit_ec[proc1_$as_id].value;
  quit_inh[proc1_$as_id] := false;
  ec_$advance(deliv_ec[proc1_$as_id]);
end;

procedure devs;
begin
  dw := 0;
  dw2 := 0;
  df := true;
  vb := true;
  dw := dw + 1;
end;

procedure local_v;
var i: [volatile] integer; k: integer32; j: integer;
begin
  i := 0;
  j := i;
  i := j + 1;
  if i = 0 then dw := j;
end;

procedure atomic;
begin
  a := a + 2;
  b := b - 100;
end;
