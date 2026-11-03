# Snapshots

dpc's assembly for every `.pas` unit of the image-conformance manifest
(`tests/conformance/manifest.txt`) and every execution test program
(`tests/exec/`), kept here so that a machine without the SR10.2 image, its
link map or Musashi still checks the compiler side of those suites.  CI is
such a machine: it depends on nothing outside this repository (bead
domain-pascal-rz9q).

- `conformance/NAME.s`: the unit `tests/conformance/NAME.pas`, compiled as
  `make conform` compiles it.
- `exec/NAME.s`: the program `tests/exec/NAME.pas`; `exec/NAME+MODULE.s`:
  its `link:` module `MODULE.pas`, compiled with the program's options.
- `NAME.args`: the command line, `SRC OPTION...`, run from the repository
  root.  The options the manifest derives from the map (`--fragment-origin`,
  `--module-routine`, ...) are recorded resolved, which is what lets the
  check run without the map.

The `.s` files are normalised as the golden tests normalise theirs: `|`
comments removed, blanks collapsed, empty lines dropped.

## Checking

`make snapshot` (part of `make test`; about ten seconds) runs
`tools/snapshot.py check`: each snapshot's source is compiled with its
options and the output compared with the `.s`.  It also fails when a unit
or program has no snapshot, or a snapshot has no unit.  It needs dpc only.

A difference means dpc's output for that unit changed.  The snapshots
match what the image and the emulator accepted when they were written, so a
changed conformance snapshot is a change to code that matched the image, and
a changed exec snapshot is a change to code that computed the right results.
Whether the new output is still right is for `make conform` and `make
exectest` to say.

## Updating

`make snapshot-update` (`tools/snapshot.py update [--suite
conformance|exec]`) rewrites a suite's snapshots, and only when the suite
passes:

- conformance: every `.pas` unit of the manifest compiles and matches the
  image, as `make conform` checks it (needs the image, the map and the
  m68k-elf tools);
- exec: `make exectest` passes (needs Musashi and the m68k-elf tools).

It writes nothing for a suite that fails or cannot run, so a snapshot only
ever records output that passed.  As with a golden file, a snapshot change
needs a reason in the commit message.
