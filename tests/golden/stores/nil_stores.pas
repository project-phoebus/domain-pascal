{ Golden: codegen-patterns 13, "Constants stored": `nil' stored is the
  longword 0, `clr.l <ea>' (bead domain-pascal-j4ip; tools/nilsurvey.py
  store: 1160 longword zeros stored to memory in compiled code, all
  `clr.l', no `move.l #0,<ea>').  Sites: a frame local, BAT_$ALLOC_VTOCE
  0xE3AEDA `clr.l (-0x14,A6)'; a variable of the unit, BAT_$ALLOCATE
  0xE3B2EE `clr.l (0xd38,A5)'; another unit's variable, HINT_$SHUTDN
  0xE49924 `clr.l abs.l'; a field through a pointer, MAC_OS_$CLOSE
  0xE0B506 `clr.l (0x7a4,A2); clr.l (0x7a0,A2)' (two adjacent
  longwords, the lower a routine pointer, not merged); a local held in a data
  register, IO_$GET_DCTE 0xE1A466 `clr.l D2'.  Which register holds the
  local, and the frame, are dpc's rules (codegen-patterns 12), not
  compared with those routines. }
module nils;
type
  node_p = ^node;
  node = record
    val: integer;
    next: node_p;
    cb: ^procedure;
  end;
var
  head: node_p;
  other_head: extern node_p;

procedure set_ptr(var q: node_p); extern;

procedure frame_local;
var
  q: node_p;
begin
  set_ptr(q);
  q := nil;
  set_ptr(q);
end;

procedure unit_var;
begin
  head := nil;
end;

procedure other_unit_var;
begin
  other_head := nil;
end;

procedure through_pointer(n: node_p); options(val_param);
begin
  n^.cb := nil;
  n^.next := nil;
end;

procedure in_register;
var
  p: node_p;
begin
  p := nil;
  if head <> nil then
    p := head;
  other_head := p;
end;
