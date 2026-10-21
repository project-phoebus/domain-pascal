{ Golden: codegen-patterns 13, "Pointer constants" (bead
  domain-pascal-gw19).  A pointer constant other than `nil' that needs a
  register is formed in an address register with a long immediate,
  `movea.l #n,An', never `moveq' into a data register: stored to memory
  from it, PBU_$ACQUIRE_INT SAU5 0xE588D4 `movea.l #-1,A1; move.l
  A1,(-0x48,A0)' (`ddf.handlers[i] := univ_ptr(pbu_no_handler)', the
  handlers `univ_ptr's, a frame record, in a `dbf' loop); into a pointer
  local held in an address register, PEB_$GET_STATUS 0xE5ADD6 `movea.l
  #0x7000,A0'.  `nil' into such a local is `suba.l An,An' (the 10 of the
  24 `suba.l' that are not results, DISK_ 0xE3C620 `suba.l A2,A2').
  Outside `moveq''s range a pointer constant is stored directly,
  `move.l #n,<ea>' (PEB_$LOAD_REGS 0xE5AE6A `move.l #0x7000,(-4,A6)').
  Compared in memory: `cmpi.l #n,<ea>' (PBU_$SETUP SAU5 0xE59842
  `cmpi.l #-1,(0x40,A3)'); held in an address register: `cmpa.l #n,An'
  (FIM_$BUILD_DF 0xE0A552 `cmpa.l #sym,A3' with an address; no compiled
  site with a literal), `nil' `cmpa.w #0,An' as before.  The frame and
  the registers beyond those named are dpc's rules (codegen-patterns
  12), not compared with those routines. }
module ptrk;
type
  handler_t = univ_ptr;
  ddf_t = record
    unit: integer;
    stack: univ_ptr;
    handlers: array[0..15] of handler_t;
  end;
  ddf_ptr_t = ^ddf_t;
  node_p = ^node_t;
  node_t = record
    next: node_p;
    v: integer32;
  end;
const
  no_handler = -1;

procedure ptrk_$setup(in ddf: ddf_t); extern;

procedure ptrk_$acquire;
var
  i: integer;
  ddf: ddf_t;
begin
  for i := 0 to 15 do
    ddf.handlers[i] := univ_ptr(no_handler);
  ddf.unit := 7;
  ptrk_$setup(ddf);
end;

procedure ptrk_$set(in d: ddf_ptr_t);
begin
  d^.handlers[3] := univ_ptr(5);
end;

procedure ptrk_$set_stack(in d: ddf_ptr_t);
begin
  d^.stack := univ_ptr(16#7000);
end;

function ptrk_$unused(in d: ddf_ptr_t): boolean;
begin
  ptrk_$unused := d^.handlers[2] = univ_ptr(no_handler);
end;

function ptrk_$sum(in head: node_p): integer32;
var
  r: node_p;
  s: integer32;
begin
  s := 0;
  r := head;
  while r <> node_p(16#800000) do begin
    s := s + r^.v;
    r := r^.next;
    if s > 100 then
      r := node_p(16#800000)
    else if s < 0 then
      r := nil;
  end;
  ptrk_$sum := s;
end;
