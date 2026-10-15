# Top-level build.  Build output goes to target/ (cargo).  tmp/ is shared
# scratch space and is left alone.

CARGO   ?= cargo
PYTHON  ?= python3

.PHONY: all build runtime stable test exectest corpus conform linktest srcsurvey clean

all: build

build:
	$(CARGO) build
	@$(MAKE) --no-print-directory runtime

# The m68k run-time library (runtime/README.md): M$ARITH, Apollo's
# compiler arithmetic routines, assembled from runtime/m68k/m_arith.s into
# target/runtime/m68k/m_arith.o and the archive libm_arith.a (`dpc
# --print-runtime' names it).  Built with `make build' when the m68k-elf
# tools are present; otherwise skipped with a notice.  The archive is
# written deterministically (ar D), so the output is reproducible.
M68K_AS ?= m68k-elf-as
M68K_AR ?= m68k-elf-ar
RUNTIME_SRC := runtime/m68k/m_arith.s
RUNTIME_OUT := target/runtime/m68k
RUNTIME_OBJ := $(RUNTIME_OUT)/m_arith.o
RUNTIME_LIB := $(RUNTIME_OUT)/libm_arith.a

runtime:
	@if command -v $(M68K_AS) >/dev/null 2>&1 && command -v $(M68K_AR) >/dev/null 2>&1; then \
	  $(MAKE) -s --no-print-directory $(RUNTIME_LIB); \
	else echo "runtime: $(M68K_AS) or $(M68K_AR) not found; m68k runtime not built (skipped)"; fi

$(RUNTIME_OBJ): $(RUNTIME_SRC)
	@mkdir -p $(@D)
	$(M68K_AS) -m68010 -o $@ $<

$(RUNTIME_LIB): $(RUNTIME_OBJ)
	@rm -f $@
	$(M68K_AR) rcsD $@ $<
	@echo "runtime: built $@"

# A stable dpc for other repositories (../source): built from the
# committed HEAD only, never from the working tree, so agents' unreviewed
# edits cannot leak into another repository's build.  `make stable'
# exports HEAD with `git archive' into target/stable/src, builds it there,
# and installs target/stable/bin/dpc with the runtime beside it in
# target/stable/runtime/m68k/ (where `dpc --print-runtime' looks:
# <dpc dir>/../runtime/<target>/).  target/stable/COMMIT names the
# commit.  Run it after each commit that other repositories should see.
STABLE := target/stable

stable:
	@rev=$$(git rev-parse --verify HEAD) && \
	rm -rf $(STABLE)/src && mkdir -p $(STABLE)/src $(STABLE)/bin && \
	git archive HEAD | tar -x -C $(STABLE)/src && \
	( cd $(STABLE)/src && $(CARGO) build --release --quiet ) && \
	$(MAKE) --no-print-directory -C $(STABLE)/src runtime RUNTIME_OUT=$(CURDIR)/$(STABLE)/runtime/m68k && \
	cp $(STABLE)/src/target/release/dpc $(STABLE)/bin/dpc.new && \
	mv $(STABLE)/bin/dpc.new $(STABLE)/bin/dpc && \
	echo $$rev > $(STABLE)/COMMIT && \
	echo "stable: $(STABLE)/bin/dpc built from $$rev"; \
	if ! git diff --quiet HEAD -- crates runtime 2>/dev/null; then \
	  echo "stable: note: the working tree has uncommitted changes; they are NOT in the stable build"; fi

test:
	$(CARGO) test
	@if [ -d tools/tests ]; then $(PYTHON) -m unittest discover -s tools/tests; fi
	$(MAKE) exectest
	$(MAKE) corpus

# Execution tests (tests/exec/README.md): dpc output run on the Musashi
# 68010 emulator, built from MUSASHI_DIR (default
# ~/src/domainos-archeology/otterlater/musashi; not part of this repo),
# linked with dpc's M$ARITH runtime (target/runtime/m68k/m_arith.o).
# Skipped with a notice when Musashi or the m68k-elf tools are missing.
# Under a second.
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
