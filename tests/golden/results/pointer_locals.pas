{ codegen-patterns 3 ("Callers of pointer functions"), 12.4 R4(c), 12.5
  ("Pointer locals assigned from calls"); bead domain-pascal-500.  A
  pointer local assigned from pointer functions' A0:

  - held: never pushed by value, so held in an address register,
    `movea.l A0,An' (108 sites; KBD_$SET_KBD_TYPE 0xE7253E `movea.l
    A0,A2'), tested with `cmpa.w #0,An' (AST_ 0xE02F56 `movea.l A0,A3;
    cmpa.w #0,A3'), assigned again on one path, dereferenced after the
    join and passed by address last (`pea (A2)', 0xE72550), held through
    that call in a body with branches (12.4 R4(c): 13 such sites, all
    callee-saved);
  - framed: pushed by value later, so kept in its frame slot (AST_$RESERVE
    0xE0677E: `aote' is passed to ast_$lookup_aste); `= nil' right after
    the store branches on the store's flags (0xE067BA `move.l
    A0,(-28,A6); bne.s'), dereferenced through A0 on the branch's path
    (0xE067E2 `bset #6,(191,A0)') and after the join where both paths
    left it in A0 (0xE067E8 `tst.b (185,A0)');
  - uplevel: reached by a nested routine, so in the frame; `= nil' right
    after the store tests A0 again (AST_$INVALIDATE 0xE0666A `move.l
    A0,(-20,A6); cmpa.w #0,A0'), A0 is the base on the branch's path
    (0xE066A8) and the slot is reloaded at the join (0xE066AE `movea.l
    (-20,A6),A0').
  - escaped: its address passed, so in the frame (bead domain-pascal-pe9)
    and reloaded after the call that may change it. }
module plocal;
type
  status_$t = record case integer of 1: (all: integer32); end;
  rec_t = record
    count: integer;
    flags: integer;
    next: ^rec_t;
  end;
  rec_ptr = ^rec_t;
var
  seen: integer;
  gstatus: status_$t;

function lookup(key: integer; var status: status_$t): rec_ptr; val_param; extern;
function force(key: integer; var status: status_$t): rec_ptr; val_param; extern;
procedure use_rec(var r: rec_t); val_param; extern;
procedure use_ptr(p: rec_ptr); val_param; extern;
procedure set_ptr(var p: rec_ptr); val_param; extern;

procedure held(key: integer); val_param;
var
  p: rec_ptr;
begin
  p := lookup(key, gstatus);
  if p = nil then p := force(key, gstatus);
  seen := p^.count;
  use_rec(p^);
end;

procedure framed(key: integer); val_param;
var
  p: rec_ptr;
begin
  p := lookup(key, gstatus);
  if p = nil then begin
    p := force(key, gstatus);
    if p = nil then return;
  end else
    p^.flags := 1;
  seen := p^.count;
  use_ptr(p);
end;

procedure uplevel(key: integer); val_param;
var
  p: rec_ptr;
  procedure inner;
  begin
    seen := p^.count;
  end;
begin
  p := lookup(key, gstatus);
  if p = nil then begin
    p := force(key, gstatus);
    if p = nil then return;
  end else
    p^.flags := 1;
  seen := p^.flags;
  inner;
end;

procedure escaped(key: integer); val_param;
var
  p: rec_ptr;
begin
  p := lookup(key, gstatus);
  set_ptr(p);
  seen := p^.count;
end;
