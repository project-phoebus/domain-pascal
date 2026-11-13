| tools/exectest/stubs.s - the logging stubs of the execution tests
| (tests/exec/README.md).  Each label is an entry point the runner
| (tools/exectest/runner.c) recognises by its address and runs in C; the
| body, an `illegal', is never executed.

	.section	".text.exec_stubs","ax",@progbits
	.balign	2
	.irp	n,LOG_L,LOG_W,LOG_UW,LOG_C,LOG_BOOL,LOG_X,LOG_MEM,LOGV_L,LOGV_W,LOGV_C,ID_L,ID_W,ID_BOOL,ID_P,CLOBBER,MARK,NOSAVE,NOSAVE_W,NOSAVE_ID_L
	.globl	EXEC_$\n
EXEC_$\n:
	illegal
	.endr

| The floating point interpreter (runner.c, run_fpp).
	.globl	FPP_$
FPP_$:
	illegal

| The Pascal run time's WRITE and WRITELN (runner.c, run_pas_write).
	.globl	PAS_$WRITE
PAS_$WRITE:
	illegal
	.globl	PAS_$WRITELN
PAS_$WRITELN:
	illegal

| A "device register" (runner.c): each word read returns the next value.
	.section	".data.exec_stubs","aw",@progbits
	.balign	2
| The standard output file variable (its address passed; not read).
	.globl	PAS_$OUTPUT
PAS_$OUTPUT:
	.short	0
	.globl	EXEC_$COUNTER
EXEC_$COUNTER:
	.short	0
