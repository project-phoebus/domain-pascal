| proc1_tst_lock.s - HARNESS FIXTURE, NOT COMPILER OUTPUT.
|
| Hand-written by transcribing the image bytes of PROC1_$TST_LOCK,
| 0xE148CA..0xE148E6 (tools/imgdis.sh E148CA 1C), to test tools/conform.py.
| It exists only to prove the harness accepts a correct unit and rejects a
| perturbed one; it says nothing about what the compiler emits.
|
| PROC1_$CURRENT_PCB is referenced by name (the map has it at 0xE1EAC8);
| `(sym).l' forces the absolute-long form, 2079 00e1 eac8.

	.text
	.globl	PROC1_$TST_LOCK
PROC1_$TST_LOCK:
	link.w	%fp,#-4				| 4e56 fffc
	movea.l	(PROC1_$CURRENT_PCB).l,%a0	| 2079 00e1 eac8
	move.w	(8,%fp),%d0			| 302e 0008
	move.l	(0x40,%a0),%d1			| 2228 0040
	btst	%d0,%d1				| 0101
	sne	%d1				| 56c1
	move.b	%d1,%d0				| 1001
	unlk	%fp				| 4e5e
	rts					| 4e75
