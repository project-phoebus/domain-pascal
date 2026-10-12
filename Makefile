# Top-level build.  Build output goes to target/ (cargo).  tmp/ is shared
# scratch space and is left alone.

CARGO   ?= cargo
PYTHON  ?= python3

.PHONY: all build test clean

all: build

build:
	$(CARGO) build

test:
	$(CARGO) test
	@if [ -d tools/tests ]; then $(PYTHON) -m unittest discover -s tools/tests; fi

clean:
	$(CARGO) clean
