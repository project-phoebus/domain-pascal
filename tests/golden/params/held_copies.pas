{ Value parameters passed by address whose copy is held in a data register
  (codegen-patterns 1.2, "Held in a register instead"; bead
  domain-pascal-gfnj).  The copy's frame slot stays reserved (the `link'
  sizes count it); the value is loaded through the argument slot, where a
  copy would be made, into a register the body reads.

  - hc_$refcnt: FILE_$SET_REFCNT 0xE5E3F4: `link #-0x44' with the copy's
    place -0x40 never addressed; 0xE5E40A `movea.l (0xc,A6),A0; move.l
    (A0),D0' after the entry loads of the other slots, then the tests
    against D0 (0xE5E410 `cmpi.l #-1,D0', 0xE5E41E) and the store of its
    low word (0xE5E426 `move.w D0,(-0x38,A6)').
  - hc_$server: a byte live across a call, in D2 (PROC2_$SET_SERVER
    0xE41476..0xE4147C `movea.l (0xc,A6),A0; subq.l #2,SP; move.b
    (A0),D2' before the ML_$LOCK call).
  - hc_$mode: a word stored into the unit's data from D0
    (SMD_$SET_TP_REPORTING 0xE6E4BC..0xE6E4C2 `movea.l (8,A6),A0; move.w
    (A0),D0; move.w D0,(0xde,A5)').
  - Copied as before: hc_$pushed, every read pushes it
    (SMD_$SET_TP_CURSOR 0xE6E976, 0xE6E984); hc_$loop, read in a loop
    (DISK_$REGISTER 0xE3D9AC, EC2_$WAIT 0xE42358, SMD_$FREE_HDM 0xE6DA3A
    copy theirs: Uncertain, these save six or more registers); hc_$ptr, a
    pointer (SET_LITES_LOC 0xE0C4EC copies it and tests the copy);
    hc_$assigned, assigned (no such copy decided by the survey); hc_$zero,
    tested against zero by the first statement (the load's flags or a
    `tst' right after the load: not settled, codegen-patterns 7.1). }
module hc;
type
  status_$t = integer32;
  int_ptr = ^integer;

var
  mode_v: integer;
  last: integer;
  p_v: int_ptr;

procedure hc_$set(in w: integer; out status: status_$t); extern;
procedure hc_$lock(w: integer); options(val_param, extern);
procedure hc_$take(w: integer); options(val_param, extern);

procedure hc_$refcnt(refcnt: integer32; out status: status_$t);
var
  w: integer;
begin
  if refcnt = -1 then
    w := 0
  else if refcnt < 16#FFF5 then
    w := refcnt
  else
    w := -11;
  hc_$set(w, status);
end;

procedure hc_$server(in n: integer; flag: boolean);
begin
  hc_$lock(4);
  if flag then
    last := n;
end;

procedure hc_$mode(mode: integer; out status: status_$t);
begin
  mode_v := mode;
  status := 0;
end;

procedure hc_$pushed(w: integer);
begin
  hc_$take(w);
  hc_$take(w);
end;

procedure hc_$loop(n: integer);
var
  i: integer;
begin
  i := 0;
  while i < n do
    i := i + 1;
  last := i;
end;

procedure hc_$ptr(p: int_ptr);
begin
  if p <> nil then
    p_v := p;
end;

procedure hc_$assigned(w: integer);
begin
  w := w + 1;
  last := w;
end;

procedure hc_$zero(w: integer);
begin
  if w > 0 then
    last := w;
end;
