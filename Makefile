# Top-level build.  Build output goes to target/ (cargo).  tmp/ is shared
# scratch space and is left alone.

CARGO   ?= cargo
PYTHON  ?= python3

.PHONY: all build test exectest conform linktest clean

all: build

build:
	$(CARGO) build

test:
	$(CARGO) test
	@if [ -d tools/tests ]; then $(PYTHON) -m unittest discover -s tools/tests; fi
	$(MAKE) exectest

# Execution tests (tests/exec/README.md): dpc output run on the Musashi
# 68010 emulator, built from MUSASHI_DIR (default
# ~/src/domainos-archeology/otterlater/musashi; not part of this repo),
# linked with M$ARITH from ../source.  Skipped with a notice when Musashi,
# the m68k-elf tools or arith.s are missing.  Under a second.
exectest: build
	$(PYTHON) tools/exectest.py

# Image conformance (CLAUDE.md section 6.3): every unit listed in the
# manifest is linked at its image address and compared with the image.
conform: build
	$(PYTHON) tools/conform.py manifest tests/conformance/manifest.txt

# Link test (CLAUDE.md section 6.4; tests/link/README.md): the units of
# tests/link/manifest.txt swapped for their C objects in a scratch copy of
# ../source (tmp/linktest/), whose `make check' must pass.  About 20 s, so
# not part of `make test'.  It exports a committed ../source revision
# (LINKTEST_REV, default HEAD) so other agents' uncommitted edits there
# cannot affect it; for the working tree run tools/linktest.py without --rev.
LINKTEST_REV ?= HEAD

linktest: build
	$(PYTHON) tools/linktest.py run --rev $(LINKTEST_REV) tests/link/manifest.txt

clean:
	$(CARGO) clean
