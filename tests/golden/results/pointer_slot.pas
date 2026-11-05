{ codegen-patterns 3, "Pointer results", "Every assignment last"; bead
  domain-pascal-yqsm.  A pointer function body of several statements
  whose every assignment of the result is last (nothing but the epilogue
  after it: the last statement of the body, or of a part of a final `if'
  or `case') and one of them reads memory keeps the result variable in
  its frame slot: each assignment stores into it, and the epilogue starts
  with `movea.l (-k,A6),A0', where a `return' branches to as well.

  - get_desc: one assignment, the last statement, after `return's: the
    element moved into the slot, `move.l (0x28,A0,D0.w),(-4,A6)', and
    reloaded at the epilogue that both `return's reach with the slot
    unwritten (SIO_$I_GET_DESC 0xE66808/0xE6680E; KBD_$GET_DESC
    0xE1AA92/0xE1AA98, TTY_$I_GET_DESC 0xE667B0/0xE667B6);
  - find: `nil' on two paths of a final `if', memory on the third: `clr.l'
    of the slot, `move.l (A1),(-k,A6)', the reload at the join
    (NET_$FIND_HANDLER 0xE5A178/0xE5A180/0xE5A184);
  - out_local: a local in its frame slot (its address passed to a call),
    copied into the result slot and reloaded (MST_$MAP_AREA 0xE43BF0
    `move.l (-0x14,A6),(-0x20,A6)', 0xE43BF6);
  - framed: `nil' on two paths and a local in its frame slot (a nested
    routine reaches it; tested and dereferenced) on the third: found in the frame when
    the local's assignment is selected, the body selected again with the
    result slot, `clr.l' and `move.l (-4,A6),(-8,A6)' (the shape of
    NET_$FIND_HANDLER 0xE5A178 and MST_$MAP_AREA 0xE43BF0; whether the
    original keeps such a local in the frame is another matter);
  - held: a register-held local as the only assignment is moved from its
    register into A0, no slot (AST_$ACTIVATE_ASTE_CANNED 0xE2F398 `movea.l
    A2,A0'), as before.
  In driver tests, shapes with no site: a pointer function's result
  where every assignment is last (the slot as the only statement,
  MST_$MAP 0xE438BE) and a register-held local beside memory keep dpc's
  earlier register-held variable (best effort, not byte-identical); a
  register-held local beside `nil' and a parameter beside memory are
  refused, as before.  Not reproduced here: the slot's place (dpc puts it
  below the locals; SIO_$I_GET_DESC has it above real_line's slot,
  codegen-patterns 2, "Frame size", Uncertain) and the element offset
  that SIO_$I_GET_DESC reuses from the test (bead domain-pascal-gg57). }
module ptrslot;
type
  status_$t = record case integer of 1: (all: integer32); end;
  rec_t = record
    pad: array[1..10] of integer32;
    handler: univ_ptr;
    alt: univ_ptr;
    ptr_30: univ_ptr;
    flags: integer32;
  end;
  rec_ptr = ^rec_t;
  univ_ptr_ptr = ^univ_ptr;
var
  dtte: extern array[0..3] of rec_t;
  tab: array[0..7] of rec_ptr;

function real_line(line: integer; out status: status_$t): integer;
  val_param; extern;
procedure scan(var s: status_$t); extern;
procedure fill(out p: univ_ptr; out s: status_$t); extern;
function make(k: integer): rec_ptr; val_param; extern;

function get_desc(line: integer; out status: status_$t): univ_ptr;
  val_param;
var
  k: integer;
begin
  k := real_line(line, status);
  if status.all <> 0 then
    return;
  if dtte[k].handler = nil then begin
    status.all := 16#B000D;
    return;
  end;
  get_desc := dtte[k].handler;
end;

function find(k: integer; var status: status_$t): univ_ptr; val_param;
begin
  if tab[k] = nil then begin
    status.all := 16#2B0003;
    find := nil;
  end
  else if tab[k]^.alt = nil then begin
    status.all := 16#11001D;
    find := nil;
  end
  else begin
    status.all := 0;
    find := univ_ptr_ptr(tab[k]^.alt)^;
  end;
end;

function framed(k: integer; var status: status_$t): rec_ptr; val_param;
var
  p: rec_ptr;
  procedure forget; begin p := nil; end;
begin
  p := tab[k];
  if p = nil then begin
    status.all := 1;
    framed := nil;
  end
  else if p^.alt = nil then begin
    status.all := 2;
    framed := nil;
  end
  else
    framed := p;
end;

function out_local(var status: status_$t): univ_ptr;
var
  va: univ_ptr;
begin
  fill(va, status);
  scan(status);
  out_local := va;
end;

function held(k: integer; var status: status_$t): rec_ptr; val_param;
var
  p: rec_ptr;
begin
  p := make(k);
  p^.flags := 0;
  scan(status);
  held := p;
end;
