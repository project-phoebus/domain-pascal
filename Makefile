# Top-level build.  Build output goes to target/ (cargo).  tmp/ is shared
# scratch space and is left alone.

CARGO   ?= cargo
PYTHON  ?= python3

.PHONY: all build test exectest corpus conform linktest srcsurvey clean

all: build

build:
	$(CARGO) build

test:
	$(CARGO) test
	@if [ -d tools/tests ]; then $(PYTHON) -m unittest discover -s tools/tests; fi
	$(MAKE) exectest
	$(MAKE) corpus

# Execution tests (tests/exec/README.md): dpc output run on the Musashi
# 68010 emulator, built from MUSASHI_DIR (default
# ~/src/domainos-archeology/otterlater/musashi; not part of this repo),
# linked with M$ARITH from ../source.  Skipped with a notice when Musashi,
# the m68k-elf tools or arith.s are missing.  Under a second.
exectest: build
	$(PYTHON) tools/exectest.py

# Front-end corpus test (bead domain-pascal-qia): dpc --check-only over
# Apollo's SR10.2 inserts (sys/ins/*.ins.pas) and Pascal programs, from
# CORPUS_DIR (default ~/src/domainos-archeology/sr10.2-install/install/
# ri.apollo.os.v.10.2; not part of this repo, not redistributable),
# compared with tests/corpus/baseline.txt.  Skipped with a notice when the
# install is missing.  About a second.
corpus: build
	$(PYTHON) tools/corpus.py

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

# Source survey (tools/srcsurvey.py; tests/conformance/README.md): dpc over
# ../source's Pascal modules, each compiled routine compared with the
# image, the per-routine statuses compared with tests/srcsurvey/baseline.txt
# (exit 1 when a routine got worse).  At the baseline's ../source commit
# unless SRCSURVEY_REV is given (e.g. HEAD), so only dpc moves the result.
# About 10 s and it depends on another repository, so not part of `make
# test'; skipped with a notice when ../source is missing.  The full report
# is in tmp/srcsurvey/report.txt; refresh the baseline with
# `python3 tools/srcsurvey.py run --update'.
SRCSURVEY_REV ?=

srcsurvey: build
	@if [ -e ../source/.git ]; then \
	  mkdir -p tmp/srcsurvey; \
	  $(PYTHON) tools/srcsurvey.py run --baseline $(if $(SRCSURVEY_REV),--rev $(SRCSURVEY_REV)) \
	    > tmp/srcsurvey/report.txt 2>&1; rc=$$?; \
	  sed -n '/^== summary/,$$p' tmp/srcsurvey/report.txt; \
	  if [ $$rc -ne 0 ]; then tail -5 tmp/srcsurvey/report.txt; fi; \
	  exit $$rc; \
	else echo "srcsurvey: ../source not found; skipped"; fi

clean:
	$(CARGO) clean
