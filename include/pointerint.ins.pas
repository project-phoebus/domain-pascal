{ include/pointerint.ins.pas - PointerInt, a signed integer the size of a
  pointer: a portable spelling for code that keeps an address in an
  integer (owner, 2026-11-12; docs/aarch64.md, "integer64 and
  PointerInt").  A type transfer between it and any pointer type is
  always of equal sizes (ref "Type Transfer Functions", p. 4-197).

  integer32 on the m68k, whose pointers are 4 bytes; dpc's integer64 (a
  64-bit integer, not in the reference) elsewhere.  _ISP__M68K is a
  predeclared conditional variable (ref 4.7, p. 4-46), true on the m68k
  only. }

%if _ISP__M68K %then
type
  PointerInt = integer32;
%else
type
  PointerInt = integer64;
%endif
