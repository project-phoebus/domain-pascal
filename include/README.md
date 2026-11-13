# dpc's include files

Insert files that dpc ships, for programs to `%include` (ref 4.7,
"%include"); put this directory on the search path with `-I include`.

| File | What |
|---|---|
| `pointerint.ins.pas` | `PointerInt`, a signed integer the size of a pointer: `integer32` on the m68k, dpc's `integer64` elsewhere (docs/aarch64.md, "integer64 and PointerInt") |
