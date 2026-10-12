# Image conformance tests

This is the main gate (CLAUDE.md section 6, item 3).  Each test is a Pascal
version of a real routine from the SR10.2 SAU2 kernel.  The harness:

1. compiles and assembles it;
2. links it standalone at the routine's image address;
3. compares the bytes with the image.

Relocations are matched symbolically, as `../source`'s `asm_compare.py`
does.  The harness keeps a table of external symbols and their image
addresses, taken from the SAU2 link map.

The first candidates are listed in CLAUDE.md section 6 (PROC1_$TST_LOCK,
TIME_$Q_INIT_QUEUE, PROC1_$GET_LOADAV, AST_$PAGE_ZERO, LOG_$READ).  No
compiled tests yet.

The harness is `tools/conform.py` (map reader: `tools/sau2map.py`).  Units
are listed in `manifest.txt` and run with `make conform`; one unit can be
checked with

    python3 tools/conform.py check FILE.s --routine NAME [--addr 0xADDR[:LEN]]

`handasm/` holds harness fixtures transcribed by hand from the image.  They
test the harness itself and are **not** compiler output.
