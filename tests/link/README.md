# Link test against ../source

CLAUDE.md section 6, item 4 (milestone 7; beads domain-pascal-xdh.1,
domain-pascal-b0k4).  `../source` builds the SR10.2 SAU2 kernel from its
Domain Pascal tree with dpc: its C sources and C build are gone
(`../source` 428dab68 "Remove the kernel's C sources and the C build",
4c9e9b8c "Build the kernel from Pascal by default").  A module file dpc
refuses is therefore a module the kernel cannot link without.  The link
test builds that tree with our dpc, in a **scratch copy**, the way
`../source`'s Makefile does, and says how far the kernel is from linking.
`../source` is only read.

    make linktest                     # ../source HEAD, target/stable/bin/dpc
    make linktest LINKTEST_DPC=target/debug/dpc LINKTEST_REV=<rev>
    python3 tools/linktest.py run [--rev REV | --working-tree] [--dpc DPC] [-j N]
                                  [--only MODULE ...] [--update] [--link-anyway]
                                  [--no-make] [--make-var VAR=VALUE ...] [--top N]

A run takes a few seconds (the 109 module files compile in about 2 s in
parallel), so it is not part of `make test`, which runs the harness's
unit tests (`tools/tests/test_linktest.py`: a fake `../source` git
repository and a fake dpc, no kernel needed).

## How ../source builds the kernel

Read from `../source/domain_os/Makefile` and `tools/pascal_modules.py`
(2026-10-17, `../source` 992f96b1).

- **Module files.**  `PAS_MODULES := $(shell python3
  tools/pascal_modules.py)`: every `<subsystem>/<subsystem>.pas`, and every
  other `.pas` of a subsystem (not an `.ins.pas`) with a module heading
  (`pbu/ec2_pbu.pas`).  The other `.pas` files are fragments the module
  files `%include`.
- **Compile.**  From `domain_os/`: `$(DPC) -o build/sau2/pascal/<m>.s
  <m>.pas`, no other option; then `m68k-elf-as -m68010` (PAS_ASFLAGS).  A
  module's `.s` depends on every `.pas` of the tree and on `$(DPC)`.
  `DPC ?= ~/src/domainos-archeology/domain-pascal/target/debug/dpc`.  A
  module dpc refuses fails the build (`../source` bead source-zrkj).
- **Runtime.**  `m_arith.o` and `pas_set_build.o` are extracted from the
  archive `$(DPC) --print-runtime=m68k` names (`runtime/README.md`).
- **Kernel.**  The Pascal objects, the hand-written assembly
  (`<subsystem>/sau2/*.s`, `arch/m68k/sau2/*.s`) and the runtime objects,
  laid out in map order by `tools/gen_layout_ld.py` (`layout.ld`),
  linked with `sau2.ld` into `build/sau2/domain_os.elf`; `make domain_os`
  makes the RFC image `dist/sau2/domain_os` from it.
- **`make check`** = `check-layout` (static layout checks, a scratch link
  with unresolved symbols ignored, map order of every placed symbol),
  `check-asm` (`tools/asm_compare.py` over the hand-written objects and
  M$ARITH, relocation-aware against the image) and `check-rfc` (the load
  image, fixup table and a dry-run of sysboot + COLD against the
  original).  It reads the map (`SAU2_MAP`) and the image (`SAU2_IMAGE`)
  from `$HOME`.
- `make pascal-report` compiles every module, keeps going, and lists the
  refused ones with their first error; `tools/pascal_check.sh` runs `dpc
  --check-only` over a copy with missing fragments dropped.

## What the harness does (tools/linktest.py)

1. **Export.**  `git archive` of `../source`'s `domain_os/` at `--rev`
   (default HEAD) into `tmp/linktest/export/`, then `rsync -rlpc
   --delete` into `tmp/linktest/source/domain_os/` with `build/`, `dist/`
   and `__pycache__` left out, so the scratch build survives between runs
   and make rebuilds only what changed.  Other agents' uncommitted edits
   in `../source` are not seen; the commit id is printed and recorded.
   `--working-tree` copies the working tree instead (not reproducible).
   From a git worktree of this repository the default `--source` is the
   `../source` beside the main checkout.
2. **Module list.**  The scratch copy's own `tools/pascal_modules.py`.
3. **Compile.**  Each module file with `DPC -o build/sau2/pascal/<m>.s
   <m>.pas` from the scratch `domain_os/` (the Makefile's rule and flags,
   nothing added), then `m68k-elf-as -m68010`, in parallel (`-j`, default
   the CPU count).  The `.s` and `.o` land where the Makefile's would, so
   the make in step 6 links them as they are; a refused module leaves no
   `.s`/`.o` behind (as `make pascal-report` does), so make never takes a
   stale one.  dpc's stderr is kept in `<m>.s.log`.  The default dpc is
   the main checkout's `target/stable/bin/dpc`, the stable build of its
   committed HEAD (`make stable`; `target/stable/COMMIT` is printed and
   recorded), also when run from a git worktree, which has no stable build
   of its own (the Makefile's `LINKTEST_DPC` default does the same).
4. **Per module.**  The status:
   - `OK`: compiled and assembled;
   - `REFUSED`: dpc reported errors (its "not supported yet" refusals, or
     any other error, e.g. `index out of range` in `rem_file/server.pas`);
   - `FAILED`: dpc stopped without an error diagnostic (a panic, a signal,
     the 300 s timeout);
   - `ASM`: gas rejected dpc's output;

   with the error count and the distinct error kinds.  A kind is the
   message normalised by `tools/srcsurvey.py`'s refusal histogram
   (`srcsurvey.kind_of`: names in backquotes, quoted text and numbers
   folded), after one more folding: in the messages that spell a type out
   (dpc's `TypeTable::display`, `crates/dpc-sema/src/types.rs`: a type's
   name, or `set of 0..7`, `[byte] set of ...`, `array[1..4] of char`,
   `^foo_t`, `(a, b)`), the whole type phrase becomes `T`.  The templates
   are listed in `linktest.TYPE_PHRASES` (`functions returning T are not
   supported yet`, `a packed field of type T that is not a whole byte ...`,
   the initial-value refusals, the type-mismatch errors), so one refusal is
   one kind whatever the type.  dpc reports at most one refusal per statement and
   may stop early, so a module's kinds are the ones it reported this run,
   not necessarily all it has.
5. **Ranking.**  Per kind: *only*, the modules where it is the only kind
   left (fixing it lets those modules compile, as far as dpc reported),
   and *total*, the modules it appears in; sorted by only, then total.
   The top `--top` (20) are printed; all are in the report.
6. **Baseline** `tests/link/baseline.txt`: a header with the `../source`
   commit and the dpc (its stable commit, or its path), then one line per
   module, `status<TAB>module<TAB>first kind` (`-` for OK).  Statuses rank
   OK > ASM > REFUSED > FAILED.  Regressions, exit 1:
   - `WORSE`: a lower rank than in the baseline (OK -> anything,
     REFUSED -> FAILED, ...);
   - `GONE`: a module that was OK there and is no module file of
     `../source` now (removed or renamed: a renamed module also shows up
     as `new`; refresh the baseline then).

   Listed, not counted: `FIXED` (now OK), `better` (a higher rank, not
   OK), `diff` (the same status, another first kind), `new`, and `gone`
   for a module that did not compile.  When the baseline's `../source`
   commit or dpc is not the run's, a note says so.  A malformed baseline
   line is an error naming the file and line, exit 2.  `--update` rewrites
   the baseline (say why in the commit); it is refused (exit 2) with
   `--only`, with `--working-tree` and without `m68k-elf-as`, so the
   baseline always records a complete run at a commit.  With `--only`,
   only those modules are compared and no make is run.
7. **make.**  When every module compiles: `make -jN DPC=<our dpc,
   absolute> domain_os check` in the scratch copy (after removing
   `domain_os.elf`, `layout.ld` and `layout-check.elf`, so it always
   relinks), log `tmp/linktest/make.log`; its failure is exit 1 and its
   error lines are printed.  `--make-var VAR=VALUE` passes more variables
   (`SAU2_MAP`, `SAU2_IMAGE`).  With `--link-anyway`, when some modules are
   refused, `make -k ... PAS_MODULES='<the compiled ones>' domain_os check`
   instead (a command-line variable overrides the Makefile's `:=`): the
   first link errors and the undefined symbols, informational only.  No
   make without `m68k-elf-as`, with `--only` or with `--no-make`.

`tmp/linktest/report.txt` has the header, every module with its kinds,
the full ranking, the baseline comparison, the make result and the
summary line.

## Result (2026-10-17)

`../source` 992f96b1, dpc stable b23a505d (bead domain-pascal-bdi0's
`nil` function results included): **23 of 109 module files compile; 86
refused by dpc**, 125 error kinds.  Top of the ranking (only / total):

| only | total | kind |
|---:|---:|---|
| 4 | 6 | this predeclared function is not supported yet |
| 3 | 7 | a packed field of type T that is not a whole byte, word or longword |
| 3 | 4 | a `for` index variable that is not a word |
| 3 | 4 | a set comparison other than `s = []` / `s <> []` |
| 2 | 8 | a block through a pointer that is not held in a register (bead domain-pascal-dsr5) |
| 2 | 7 | a pointer or by-address parameter used once after a branch or a call, its lowest free register taken on some path (codegen-patterns 12.4 R4(b)) |
| 2 | 4 | a 2-byte set constant passed by address |
| 2 | 2 | a `for` index variable that is not a local of the routine |

With bdi0 the `nil` function result refusal (10 modules, 5 of them only
blocked by it, at dpc e189765e) is gone, but no module compiles yet
because of it: each of those modules now stops at another refusal.

`--link-anyway` (the 23 modules only): `check-asm` passes (109 objects,
133 sections identical modulo relocations) and `gen_layout_ld.py`'s
static checks pass; the link stops at `sau2.ld` line 264, `undefined
symbol 'MMAP_$MMAPE' referenced in expression`, a symbol a refused module
(mmap) defines, and the `the fixup table must start past the split`
assertion.

## History

Until ../source removed its C build (2026-10-17), this harness swapped
dpc objects of `tests/link/manifest.txt`'s units for the C objects of the
same routines in a scratch `../source` build and ran its `make check`
(28 units in place of 42 C objects, every byte checked again by
`../source`'s `asm_compare.py`; the C data owners, `dpc --data-base`,
whole-module units).  The manifest is retired: the whole tree is Pascal
now, and the per-routine conformance of `../source`'s Pascal is
`tools/srcsurvey.py`'s job.  `tools/modsurvey.py` ranked map segments as
candidates for that swap and reads C objects from the scratch build; it
has nothing to rank any more.  The old harness and README are in git
history (before bead domain-pascal-b0k4).
