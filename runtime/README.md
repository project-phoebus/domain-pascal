# dpc's run-time library

Code compiled by dpc is linked with a small run-time library, the way gcc
output is linked with libgcc.  For m68k that library is **M$ARITH**, the
Domain Pascal compiler's arithmetic routines: 32-bit multiply, `div` and
`mod` (CLAUDE.md 3.11; `docs/codegen-patterns.md` section 8).  It is the
only runtime Apollo's compiled kernel code calls.

## Layout

| Path | What |
|---|---|
| `runtime/<target>/` | the library's source, per target |
| `runtime/m68k/m_arith.s` | M$ARITH, the whole map module `I E0ABD4 M$ARITH size = 1DC` |
| `target/runtime/<target>/` | the built library (`make build` / `make runtime`) |
| `target/runtime/m68k/m_arith.o` | the assembled module |
| `target/runtime/m68k/libm_arith.a` | the archive to link with (one member, `m_arith.o`) |

`runtime/m68k/m_arith.s` is `../source/domain_os/math/sau2/arith.s`
copied verbatim (its header comment names the `../source` commit), an
instruction-for-instruction transcription of the image.  It keeps that
file's section, `.text.M$ARITH`, and its thirteen global names: twelve
entry points, M$OIS$LLL and M$OIU$LLL being one routine at 0xE0AD7C.  The
module is shipped whole and in image order, M$OIS$WWL included, which no
compiled kernel code calls: the kernel linked the module whole.  Do not
"clean it up"; to change it, re-copy a newer verified transcription and
update the header.

## Building

`make build` runs `make runtime` after `cargo build`: `m68k-elf-as
-m68010` assembles the module and `m68k-elf-ar rcsD` (deterministic: no
timestamps, uids or modes) archives it, so the output is reproducible.
Without `m68k-elf-as` or `m68k-elf-ar` the step is skipped with a notice.

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

## The selector names only what the module has

The m68k instruction selector forms M$ARITH names only through
`dpc_m68k::runtime::marith_entry`, which returns a name from the module's
entry list (`dpc_m68k::runtime::ENTRIES`); an operation the module has no
routine for is refused, not given an invented name.  Unit tests check
that the list equals the `.global` names of `runtime/m68k/m_arith.s` and
that every combination the selector asks for is in it.

## Checks

- `make conform`: `target/runtime/m68k/m_arith.o` as built, linked at
  0xE0ABD4 and compared with the image, all 0x1DC bytes
  (`tests/conformance/manifest.txt`).
- `make exectest`: the execution tests link the archive
  (`tests/exec/README.md`).
- `make linktest`: the kernel in a scratch copy of `../source` is linked
  with this runtime instead of `../source`'s own `arith.o`
  (`tests/link/README.md`).
