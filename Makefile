# Top-level build.  Build output goes to target/ (cargo).  tmp/ is shared
# scratch space and is left alone.

CARGO   ?= cargo
PYTHON  ?= python3

.PHONY: all build test conform linktest clean

all: build

build:
	$(CARGO) build

test:
	$(CARGO) test
	@if [ -d tools/tests ]; then $(PYTHON) -m unittest discover -s tools/tests; fi

# Image conformance (CLAUDE.md section 6.3): every unit listed in the
# manifest is linked at its image address and compared with the image.
conform: build
	$(PYTHON) tools/conform.py manifest tests/conformance/manifest.txt

# Link test (CLAUDE.md section 6.4; tests/link/README.md): the units of
# tests/link/manifest.txt swapped for their C objects in a scratch copy of
# ../source (tmp/linktest/), whose `make check' must pass.  About 20 s, so
# not part of `make test'.
linktest: build
	$(PYTHON) tools/linktest.py run tests/link/manifest.txt

clean:
	$(CARGO) clean
