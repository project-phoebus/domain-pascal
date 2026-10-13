{ XPD_$RESET_PTRACE_OPTS, 0xE5B156, 0x1E bytes: 0xE5B156..0xE5B174
  (tools/imgdis.sh E5B156 1E).  Semantics:
  ../source/domain_os/xpd/ptrace_opts.c; layout: xpd_$ptrace_opts_t in
  ../source/domain_os/xpd/xpd.h (14 bytes: longwords at 0, 4, 8, bytes
  at 0xC and 0xD).

  The unit is a fragment of the XPD module, so tools/conform.py compiles
  it with `dpc --fragment-origin 0xE5B156'.

  Patterns exercised (docs/codegen-patterns.md):
  - 12.2 a by-address slot used more than once, loaded at entry into A0;
  - 13   `clr.b'/`clr.l' through A0, `(A0)' with no displacement;
  - 12.4 R10 (bead domain-pascal-g5q.8): stores are not reordered by the
         scheduler, so the image's field order 0xC, 0, 0xD, 4, 8
         (0xE5B15E..0xE5B16C) is taken to be the source's order
         (Uncertain: the source is not known; the C reconstruction has
         the same order).

  Reconstructed: the field names (from the C), `char' for the two flag
  bytes (a byte field cleared with `clr.b'; the C has uint8_t). }
module xpd_reset_ptrace_opts;
type
  xpd_$ptrace_opts_t = record
    signal_mask: integer32;     { 0x00 }
    trace_range_lo: integer32;  { 0x04 }
    trace_range_hi: integer32;  { 0x08 }
    flags: char;                { 0x0C }
    flags2: char;               { 0x0D }
  end;

procedure xpd_$reset_ptrace_opts(var opts: xpd_$ptrace_opts_t);
begin
  opts.flags := chr(0);
  opts.signal_mask := 0;
  opts.flags2 := chr(0);
  opts.trace_range_lo := 0;
  opts.trace_range_hi := 0;
end;
