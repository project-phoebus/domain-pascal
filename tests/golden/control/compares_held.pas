{ Golden: codegen-patterns 7.1 ("A variable in memory against a
  register-held one", "Two values held in registers").  Bead
  domain-pascal-5xnz; survey `tools/cmpsurvey.py held'.

  - Two values held in data registers: the left one's register is the
    `cmp' destination, `cmp.<s> Dright,Dleft', the relation as written,
    for `=' and `<>' too (audit_$log_dir_op 0xE4BE36 `clr.w D2; cmp.w
    D1,D2; bge.b', ../source `len := 0; if len < name_len'; DISK_$SPIN_DOWN
    0xE3DB34; dir_$old_chain_unlink 0xE54D88 and dir_$old_free_slot
    0xE551C8, the two orders of `=').
  - A variable in memory against a held one, `=' or `<>': `cmp.<s>
    M,Dheld', the operands exchanged (XPD_$FIND_DEBUGGER_INDEX 0xE5BAF8
    `cmp.w (0x484,A1),D2; bne.s', ../source `xpd_$debuggers[slot].asid =
    asid'; KBD_$GET_MODE 0xE1CA70).
  - The same with an ordering relation: the memory operand loaded into a
    new data register, `cmp.<s> Dheld,Dn', the relation as written
    (name_$parse_component 0xE4A036 `move.w (A1),D1; cmp.w D0,D1; bhi.b',
    `next_pos <= path_len', unsigned; its `path_len >= next_pos' at
    0xE4A028, the held one on the left, `cmp.w (A1),D0; bcs.b').  Its
    `then' part does not read next_pos: the image keeps the loaded value
    for it (0xE4A040 `move.w D1,(A3)'), dpc does not (bead
    domain-pascal-n3u8). }
module cmpheld;
type
  pinteger = 0..65535;
  entry_t = record flags: integer; asid: integer; end;
var
  g: integer;
  r: integer;
  tbl: array[1..6] of entry_t;

procedure clamp(n: integer); options(val_param);
var len: integer;
begin
  len := 0;
  if len < n then len := n;
  g := len;
end;

procedure both_held(a, b: integer; u, v: pinteger); options(val_param);
begin
  if a = b then r := 1;
  if b <> a then r := 2;
  if a > b then r := 3;
  if u <= v then r := 4;
end;

function find(asid: integer): integer; options(val_param);
var slot: integer;
begin
  find := 0;
  for slot := 1 to 6 do
    if tbl[slot].asid = asid then begin
      find := slot;
      return;
    end;
end;

procedure parse(path_len: pinteger; out next_pos: pinteger);
  internal;
begin
  if path_len >= next_pos then r := 1;
  if next_pos <= path_len then r := 2;
end;
