# dpc's run-time library

Code compiled by dpc is linked with a small run-time library, the way gcc
output is linked with libgcc.  For m68k that library holds two map
segments of the image, the Domain Pascal compiler's run-time support:

- **M$ARITH**, the arithmetic routines: 32-bit multiply, `div` and `mod`
  (CLAUDE.md 3.11; `docs/codegen-patterns.md` section 8).  It is the only
  runtime Apollo's compiled kernel code calls, and the only one dpc calls.
- **PAS_$SET_BUILD**, the set builder (codegen-patterns 5.1; bead
  domain-pascal-jzm5).  No code in the image calls it, and dpc never
  does: a set constructor with a run-time range is refused.  It is
  shipped so that a kernel linked with dpc's runtime (`../source`) gets
  the image's bytes for that segment from it too.

## Layout

| Path | What |
|---|---|
| `runtime/<target>/` | the library's source, per target |
| `runtime/m68k/m_arith.s` | M$ARITH, the whole map module `I E0ABD4 M$ARITH size = 1DC` |
| `runtime/m68k/pas_set_build.s` | PAS_$SET_BUILD, the whole map segment `I E11FA8 PAS_$SET_BUILD size = 4C` |
| `target/runtime/<target>/` | the built library (`make build` / `make runtime`) |
| `target/runtime/m68k/m_arith.o`, `pas_set_build.o` | the assembled segments |
| `target/runtime/m68k/libm_arith.a` | the archive to link with (members `m_arith.o`, `pas_set_build.o`) |

The archive keeps the name `libm_arith.a` from when M$ARITH was its only
member: `dpc --print-runtime` prints it, `../source` extracts
`m_arith.o` from whatever path that prints, and renaming it would change
the target description (`dpc_m68k::runtime::LIBRARY`) and the driver's
tests for no gain.  The members are listed in
`dpc_m68k::runtime::MEMBERS` (object, map segment, address, size).

`runtime/m68k/m_arith.s` is `../source/domain_os/math/sau2/arith.s`
copied verbatim (its header comment names the `../source` commit), an
instruction-for-instruction transcription of the image.  It keeps that
file's section, `.text.M$ARITH`, and its thirteen global names: twelve
entry points, M$OIS$LLL and M$OIU$LLL being one routine at 0xE0AD7C.  The
module is shipped whole and in image order, M$OIS$WWL included, which no
compiled kernel code calls: the kernel linked the module whole.  Do not
"clean it up"; to change it, re-copy a newer verified transcription and
update the header.

`runtime/m68k/pas_set_build.s` was transcribed here, instruction for
instruction, from the image (`tools/imgdis.sh E11FA8 4C`): `../source`
has no assembly for it, only a C stand-in (`pas/set_build.c`).  It is
hand-written assembly (no frame, `movem.l` push and pop, arguments read
off SP).  Its section is `.text.PAS_$SET_BUILD`, the map segment's name,
as M$ARITH's is `.text.M$ARITH` (and as `../source`'s C object names
it).  The segment's 0x4C bytes end with the zero word after the `rts`
(0xE11FF2), which is transcribed too.  The `or.w #0xf,D0` at 0xE11FD6 is
encoded `807c 000f`; gas would choose `ori` (`0040 000f`), so it is a
`.short` with the mnemonic in its comment (CLAUDE.md 4.4).  The
arguments, by the body: the destination (0x10,SP) and the template
(0x14,SP) by address, then the words lo (0x18), hi (0x1A) and a third
word n (0x1C) by value; it copies (n >> 4) + 1 template words and sets
members max(lo, 0) .. min(hi, n).  With no caller in the image, what a
compiler passed as n is not attested.

## Building

`make build` runs `make runtime` after `cargo build`: `m68k-elf-as
-m68010` assembles each source of `RUNTIME_SRCS` (Makefile) and
`m68k-elf-ar rcsD` (deterministic: no timestamps, uids or modes) archives
them, in that order, so the output is reproducible.  Without
`m68k-elf-as` or `m68k-elf-ar` the step is skipped with a notice.  `make
stable` builds the same runtime into `target/stable/runtime/m68k/`.

## Finding it: `dpc --print-runtime`

`dpc --print-runtime` (or `--print-runtime=m68k`), given alone, prints the
archive's path, like `gcc -print-libgcc-file-name`, and exits 0; 1 when
the file is not there.  dpc looks in

1. `$DPC_RUNTIME_DIR/<target>/`, when that variable is set;
2. otherwise `runtime/<target>/` beside the directory holding the dpc
   executable: for `target/debug/dpc` (or `target/release/dpc`),
   `target/runtime/<target>/`, where `make build` puts it.  `cargo run`
   works the same way.  A symlink to dpc is followed to the executable
   itself.

`make build` always writes `target/runtime/`, so with a `cargo install`ed
dpc (`~/.cargo/bin/dpc`, which would look in `~/.cargo/runtime/`) or a
`CARGO_TARGET_DIR` elsewhere, set `DPC_RUNTIME_DIR` to the repository's
`target/runtime`.

The file name comes from the target description
(`dpc_target::Target::runtime_library`; m68k: `dpc_m68k::runtime`).

Link with it last, like libgcc:

    m68k-elf-ld ... prog.o other.o $(dpc --print-runtime)

An archive link pulls in only the members that define a symbol something
already references.  dpc output never references PAS_$SET_BUILD, so a
program linked this way gets M$ARITH (when it uses it) and never
`pas_set_build.o`.  To place a segment whatever references it, as the
kernel link does, extract the member and link the object by name:

    ar x "$(dpc --print-runtime=m68k)" m_arith.o pas_set_build.o

## The selector names only what the module has

The m68k instruction selector forms M$ARITH names only through
`dpc_m68k::runtime::marith_entry`, which returns a name from the module's
entry list (`dpc_m68k::runtime::ENTRIES`); an operation the module has no
routine for is refused, not given an invented name.  Unit tests check
that the list equals the `.global` names of `runtime/m68k/m_arith.s` and
that every combination the selector asks for is in it.  PAS_$SET_BUILD is
in a separate list, `dpc_m68k::runtime::UNCALLED`, that the selector
does not use; a unit test checks that it is `pas_set_build.s`'s only
global and that the selector's source does not name it.

## Checks

- `make conform`: `target/runtime/m68k/m_arith.o` as built, linked at
  0xE0ABD4 and compared with the image, all 0x1DC bytes, and
  `target/runtime/m68k/pas_set_build.o` at 0xE11FA8, all 0x4C bytes
  (`tests/conformance/manifest.txt`).
- `make exectest`: the execution tests link the archive
  (`tests/exec/README.md`).
- `make linktest`: once every module file of `../source`'s Pascal tree
  compiles (or with `--link-anyway`), its kernel is linked in a scratch
  copy with both members, extracted from the archive of the dpc under
  test by `../source`'s own Makefile (`tests/link/README.md`).

## For `../source`

`../source`'s Makefile extracts both members by name from the archive `dpc
--print-runtime=m68k` names (`RUNTIME_OBJS`: `m_arith.o`,
`pas_set_build.o`; read 2026-10-17), and links them like hand-written
routines; its `check-asm` compares M$ARITH with the image.
