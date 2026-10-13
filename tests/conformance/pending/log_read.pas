{ PENDING: not in manifest.txt.  dpc cannot compile the worker yet; the
  first diagnostic is the type transfer `univ_ptr(integer32(...) +
  offset)'; past it, the `for' loop's (an index of an array of words:
  the stepped pointers of codegen-patterns 7.4 are not implemented; then
  its run-time count without a call in the body).  What is missing is
  listed in docs/codegen-patterns.md section 12.3 (bead
  domain-pascal-g5q.4).  When it compiles, move it to
  tests/conformance/log_read.pas and add to the manifest

    tests/conformance/log_read.pas  0xE17778 LOG_$READ
        --section-addr .bss.moddata.LOG_=0xE2B280

  Module-local routine at 0xE17778, 0x88 bytes, and LOG_$READ, 0xE17800,
  0x28 bytes (tools/imgdis.sh E17778 B0).  Semantics:
  ../source/domain_os/log/read_internal.c (the worker, called
  log_$read_internal there) and log/read.c; data layout: log_state_t in
  ../source/domain_os/log/log.h (logfile_uid 0, current_entry_ptr 8,
  spin_lock 0xC, wired_handle 0x10, logfile_ptr 0x14, dirty_flag 0x18;
  map `D E2B280 LOG_ size = 1C', whose only map name is LOG_$LOGFILE_UID
  at 0xE2B280); LOG_BUFFER_SIZE 0x400 (log/log.h).  The unit is a
  fragment of the LOG_ module (code segment 0xE1758C), which aligns its
  loop heads (codegen-patterns 7).

  Patterns (docs/codegen-patterns.md): 1.1 (no pop before the A5
  restore), 1.2 (LOG_$READ by address, the worker by value), 1.5
  (`clr.w -(SP)' for the 0 offset, a by-address parameter passed on as
  its slot), 4 (A5 loaded by LOG_$READ for the worker), 5 (unsigned
  0..65535 words: zero-extension, `bcc'/`bls', `lsr.w'), 5.2 (1-based
  arrays), 7 and 7.1 (`if', `return', a run-time `dbf' count with its
  `subq.w #1; bmi' guard), 12.1/12.2 (entry loads, D2/D3/A2).

  Reconstructed (not known from the original): every name except
  LOG_$READ and LOG_$LOGFILE_UID; the parameter modes (`var buffer',
  `in max_len', `out'/`var actual_len'; LOG_$READ dereferences max_len
  itself, so it is not a callee-copied value parameter); the types (the
  C has uint16_t, here 0..65535; the buffer as words, the worker copies
  words); `& 16#fffe' (the C's `& 0xFFFE'; `moveq #-2'); the pointer sum
  spelled with type transfers (ref p. 4-197); the copy as a `for' loop
  over two 1-based arrays; `unknown', a stand-in for the rest of the
  worker's untouched 16-byte frame (codegen-patterns 2). }
module log_;
const
  log_buffer_size = 16#400;
type
  pinteger = 0..65535;
  log_$buf_t = array[1..512] of integer;
  log_$buf_ptr_t = ^log_$buf_t;
var
  log_$logfile_uid: define array[1..2] of integer32;  { 0x00 }
  current_entry_ptr: univ_ptr;                         { 0x08 }
  spin_lock: integer;                                  { 0x0C }
  pad_0e: integer;                                     { 0x0E }
  wired_handle: integer32;                             { 0x10 }
  logfile_ptr: log_$buf_ptr_t;                         { 0x14 }

procedure read_internal(var buffer: log_$buf_t; offset: pinteger;
                        max_len: pinteger; var actual_len: pinteger);
  internal;
var
  src: log_$buf_ptr_t;
  i: integer;
  unknown: array[1..5] of integer;
begin
  if logfile_ptr = nil then begin
    actual_len := 0;
    return;
  end;
  actual_len := log_buffer_size;
  if max_len < log_buffer_size then
    actual_len := max_len & 16#fffe;
  if offset + actual_len > log_buffer_size then
    if offset > log_buffer_size then actual_len := 0
    else actual_len := log_buffer_size - offset;
  src := univ_ptr(integer32(logfile_ptr) + offset);
  for i := 1 to actual_len div 2 do
    buffer[i] := src^[i];
end;

procedure log_$read(var buffer: log_$buf_t; in max_len: pinteger;
                    out actual_len: pinteger);
begin
  read_internal(buffer, 0, max_len, actual_len);
end;
