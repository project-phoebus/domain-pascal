{ Golden: codegen-patterns 12.4 R4(b) (bead domain-pascal-i24w).  A
  by-address parameter loaded for one use after the body's first branch or
  call passes over the free address registers that hold a dead parameter
  slot load on every path to it, to the first one that no instruction
  wrote (or that a block copy used up); `tools/regalloc.py slotpass', 41
  loads of that shape, 41 hits.
  - inq_pgroup: the status, loaded at entry into A2 (held across the
    call), is dead after its test; the destination's slot passes over A2
    to A3, A0 holding the descriptor and A1 the copy's source
    (TTY_$K_INQ_PGROUP 0xE6796E, TTY_$K_INQ_INPUT_BREAK_MODE 0xE678E8);
  - after_test: two slots read once before the `if' in A0 and A1
    (IIC_$RECEIVE's straight-line rule, 12.2), the one after it in A2
    (0xE699B0 A3 past A1; PBU_$ADVANCE_EC_INT 0xE88432 A3 past A1, A2
    busy). }
module lsp;
const
  status_$ok = 0;
type
  status_$t = record
    all: integer32;
  end;
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
  desc_t = record
    pad: array[0..37] of integer;
    pgroup_uid: uid_$t;
  end;
  desc_ptr_t = ^desc_t;
var
  w: extern integer;

function lsp_$get_desc(in line: integer; out status: status_$t): integer32;
  options(extern, val_param, a0_return);

procedure lsp_$inq_pgroup(in line: integer; out uid: uid_$t;
                          out status: status_$t);
var
  tty: integer32;
begin
  tty := lsp_$get_desc(line, status);
  if status.all <> status_$ok then
    return;
  uid := desc_ptr_t(tty)^.pgroup_uid;
end;

procedure lsp_$after_test(out a, c, x: integer);
begin
  a := 1;
  c := 2;
  if w = 3 then
    x := 0;
end;
