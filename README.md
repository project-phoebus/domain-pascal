# domain-pascal

This project is a compiler for **Domain Pascal**, the Pascal dialect that
Apollo Computer documented in the *Domain Pascal Language Reference*
(000792-A01, December 1990).  It generates m68k code, and that code is meant
to **follow the same patterns as Apollo's own compiler**.  The reference
point is the compiled Pascal in the SR10.2 Domain/OS kernel.

It belongs to the Domain/OS archaeology work.  The sibling repository
`../source` reconstructs the SR10.2 SAU2 kernel in C.  This compiler is meant
to let that kernel be translated back into Pascal, its original language,
and built with the original ABI.  The output links against the kernel's
hand-written assembly and against the `M$ARITH` runtime.

There is no code yet.  To find your way around:

- `CLAUDE.md`: project rules, language summary, ABI summary, architecture,
  test strategy, milestones.
- `docs/codegen-patterns.md`: what Apollo's compiler emitted, with kernel
  image addresses as evidence.
- `docs/000792-A01_Domain_Pascal_Language_Reference_Dec90.txt`: the
  language reference, as OCR text.
- `docs/reference-clarifications.md`: the owner's rulings on passages that
  are unclear or badly OCR'd.
- `tools/imgdis.sh`: disassembles a range of the SR10.2 SAU2 kernel image.

Only m68k is targeted for now; aarch64 may follow later for native test
runs.  LLVM is not used.
