{ Golden: codegen-patterns 12.4 R7, computed values reused across
  statements (bead domain-pascal-g5q.7; isel.rs `cse_value').
  - acknowledge: FIM_$ACKNOWLEDGE 0xE0A96C (fim/acknowledge.c) with the
    tables in the unit's data instead of other units' arrays (dpc does
    not index those yet).  `proc1_$as_id * 12', the scaled index of the
    first statement, `move.w; lsl.w #2; move.w; add.w; add.w' in D2
    (0xE0A972..0xE0A97E), is reused by the third (0xE0A9B0 `pea
    (0,A1,D2.w)'), past the store to another array (0xE0A9A6); the
    second statement re-reads the variable for its byte index (R8,
    0xE0A99A).  D2 is callee-saved: the value's last use is the call's
    argument (R4(c)).  Not reproduced: the extern tables' `movea.l #'
    bases and their order (R9).
  - read_shape: LOG_$READ's worker 0xE17778 (log/read_internal.c) up to
    0xE177CA, then `base_addr + offset'.  The zero-extended `offset'
    (0xE177A6 `clr.l D1; move.w D0,D1') is reused after the `if' and
    the stores through `actual_len' (0xE177D0 `move.l D1,D3'); the sum
    with `actual_len' goes into the other operand (0xE177B4 `add.l
    D1,D2'), and the sum with the variable copies the held value and
    adds the variable to it (0xE177D0..0xE177D4 `move.l D1,D3; add.l
    (0x14,A5),D3').  Not reproduced: the order 0xE177A6..0xE177AE (R9),
    the constant held in D3 (0xE177AE, R8) and D3 for `0x400 - offset'
    (0xE177C4, R6). }
module cse;
const
  log_buffer_size = 16#400;
type
  pinteger = 0..65535;
  ec_t = record
    value: integer32;
    head: integer32;
    tail: integer32;
  end;
var
  proc1_$as_id: extern integer;
  quit_ec: array[0..57] of ec_t;
  quit_inh: array[0..57] of boolean;
  deliv_ec: array[0..57] of ec_t;
  last_value: integer32;
  base_addr: integer32;
  src_addr: integer32;

procedure ec_$advance(var ec: ec_t); extern;

procedure acknowledge;
begin
  last_value := quit_ec[proc1_$as_id].value;
  quit_inh[proc1_$as_id] := false;
  ec_$advance(deliv_ec[proc1_$as_id]);
end;

procedure read_shape(offset: pinteger; max_len: pinteger;
                     var actual_len: pinteger);
  internal;
begin
  actual_len := log_buffer_size;
  if max_len < log_buffer_size then
    actual_len := max_len & 16#fffe;
  if offset + actual_len > log_buffer_size then
    if offset > log_buffer_size then actual_len := 0
    else actual_len := log_buffer_size - offset;
  src_addr := base_addr + offset;
end;
