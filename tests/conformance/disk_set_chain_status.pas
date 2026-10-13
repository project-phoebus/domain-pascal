{ The module-local routine at 0xE3CCCE, 0x20 bytes, just before
  DISK_$WRITE_MULTI (tools/imgdis.sh E3CCCE 20), in the DISK_ code segment
  0xE3BB80, whose routines do not align their loop heads from 0xE3BC40 on
  (tools/loopalign.py switch DISK_; the head 0xE3CCDE is at 2 mod 4 with
  no filler).  Semantics: ../source/domain_os/disk/write_multi.c,
  `disk_$set_chain_status': store *status into every block of a chain
  linked through +0x00.

  Patterns (docs/codegen-patterns.md): 7.4 (`while' with the test at the
  bottom, entered by `bra.s' to it; the test `cmpa.w #0,A1; bne.s' back to
  the head), 7.4/12.5 (the walk pointer a local held in an address
  register, A1, for the whole body: `movea.l D0,A1', then `movea.l
  (A1),A1' for `r := r^.next'), 12.2 (entry loads in parameter order: the
  by-value pointer, not dereferenced, in D0; the status slot, used once
  but inside the loop, in A0), 13 (`move.l (A0),(0xc,A1)', memory to
  memory).

  Reconstructed (not known from the original): every name; `internal'
  (the pointer is read by value at (8,A6)); the record layout beyond the
  link at 0 and the status at 0xC (the C's disk_io_req_t); the walk
  pointer as a local `r' rather than the parameter itself (the parameter
  is loaded into D0 and copied once, to A1, which is then updated: two
  values); `unknown', a stand-in for the other 4 bytes of the untouched
  8-byte frame (codegen-patterns 2). }
module disk_;
type
  status_$t = integer32;
  disk_$req_ptr_t = ^disk_$req_t;
  disk_$req_t = record
    next: disk_$req_ptr_t;            { 0x00 }
    pad: array[1..4] of integer;      { 0x04 }
    status: status_$t;                { 0x0C }
  end;

procedure set_chain_status(req: disk_$req_ptr_t; var status: status_$t);
  internal;
var
  r: disk_$req_ptr_t;
  unknown: integer32;
begin
  r := req;
  while r <> nil do begin
    r^.status := status;
    r := r^.next;
  end;
end;
