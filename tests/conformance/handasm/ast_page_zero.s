| ast_page_zero.s - HARNESS FIXTURE, NOT COMPILER OUTPUT.
|
| Hand-written by transcribing the image bytes of the module-local routine
| at 0xE00EB0 (map: no name; ../source/domain_os/misc/zero_page.c) and of
| AST_$PAGE_ZERO at 0xE00EEC, 0xE00EB0..0xE00F16
| (tools/imgdis.sh E00EB0 66), to test tools/conform.py on a unit of two
| contiguous routines with external calls, an address constant and a local
| bsr.  It says nothing about what the compiler emits.
|
| Every external is referenced by its map name; `(sym).l' and `#sym' keep
| the absolute-long forms of the image.

	.text
zero_page:					| module-local at 0xE00EB0
	link.w	%fp,#-12			| 4e56 fff4
	pea	(0x16).w			| 4878 0016
	move.l	#AST_$ZERO_BUFF,-(%sp)		| 2f3c 00ff 8c00
	move.l	(8,%fp),-(%sp)			| 2f2e 0008
	jsr	(MMU_$INSTALL).l		| 4eb9 00e2 4048
	lea	(12,%sp),%sp			| 4fef 000c
	movea.l	#AST_$ZERO_BUFF,%a0		| 207c 00ff 8c00
	move.w	#0xff,%d0			| 303c 00ff
	movea.l	%a0,%a0				| 2048 (loop-head alignment filler)
1:	clr.l	(%a0)+				| 4298
	dbf	%d0,1b				| 51c8 fffc
	move.l	(8,%fp),-(%sp)			| 2f2e 0008
	jsr	(MMU_$REMOVE).l			| 4eb9 00e2 3d64
	unlk	%fp				| 4e5e
	rts					| 4e75

	.globl	AST_$PAGE_ZERO
AST_$PAGE_ZERO:
	link.w	%fp,#0				| 4e56 0000
	subq.l	#2,%sp				| 558f
	move.w	#0x14,-(%sp)			| 3f3c 0014
	jsr	(ML_$LOCK).l			| 4eb9 00e2 0b12
	addq.w	#4,%sp				| 584f
	move.l	(8,%fp),-(%sp)			| 2f2e 0008
	bsr.s	zero_page			| 61ac
	addq.w	#4,%sp				| 584f
	subq.l	#2,%sp				| 558f
	move.w	#0x14,-(%sp)			| 3f3c 0014
	jsr	(ML_$UNLOCK).l			| 4eb9 00e2 0b62
	unlk	%fp				| 4e5e
	rts					| 4e75
