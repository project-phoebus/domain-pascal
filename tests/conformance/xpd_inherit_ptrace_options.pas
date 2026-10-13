{ XPD_$INHERIT_PTRACE_OPTIONS, 0xE5B174, 0x14 bytes: 0xE5B174..0xE5B188
  (tools/imgdis.sh E5B174 14).  Semantics:
  ../source/domain_os/xpd/ptrace_opts.c ("bit 3 = inherit on fork" of
  the byte at 0xD); layout: xpd_$ptrace_opts_t in
  ../source/domain_os/xpd/xpd.h (14 bytes: longwords at 0, 4, 8, bytes
  at 0xC and 0xD).

  The unit is a fragment of the XPD module, so tools/conform.py compiles
  it with `dpc --fragment-origin 0xE5B174'.

  Patterns exercised (docs/codegen-patterns.md):
  - 5.3  a boolean field of a packed record (ref 3.10.7) as a value:
         `btst #3,(13,A0); sne D0' (0xE5B17C);
  - 3    a boolean function result straight into D0, `link #-4';
  - 12.2 a by-address parameter used once, loaded into A0.

  Reconstructed: the record is declared packed with the byte at 0xD
  spelled as eight booleans, the fifth (bit 3, ref 3.10.7: fields fill a
  byte from its most significant bit) named after the C's comment; the
  other names are placeholders.  The original's declaration is not
  known: the C notes `btst.l D1,D0' with an event code on the byte at
  0xC, so those bytes may be sets in a packed record (`k in s' with a
  constant member tests the same bit), which dpc does not take as
  operands yet. }
module xpd_inherit_ptrace_options;
type
  xpd_$ptrace_opts_t = packed record
    signal_mask: integer32;     { 0x00 }
    trace_range_lo: integer32;  { 0x04 }
    trace_range_hi: integer32;  { 0x08 }
    flags: char;                { 0x0C }
    flags2_b7, flags2_b6, flags2_b5, flags2_b4: boolean;  { 0x0D, bits 7..4 }
    inherit: boolean;           { 0x0D, bit 3 }
    flags2_b2, flags2_b1, flags2_b0: boolean;
  end;

function xpd_$inherit_ptrace_options(var opts: xpd_$ptrace_opts_t): boolean;
begin
  xpd_$inherit_ptrace_options := opts.inherit;
end;
