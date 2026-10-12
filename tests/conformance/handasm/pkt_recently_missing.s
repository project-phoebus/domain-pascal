| pkt_recently_missing.s - HARNESS FIXTURE, NOT COMPILER OUTPUT.
|
| Hand-written by transcribing the image bytes of PKT_$RECENTLY_MISSING,
| 0xE128BA..0xE128F6 (tools/imgdis.sh E128BA 3C), a compiled routine of
| the PKT module (code 0xE1202C..0xE12D38) that starts at 2 mod 4 and has
| a loop-head alignment filler (0x2048 at 0xE128DA, loop head 0xE128DC).
| It tests tools/conform.py on the fragment form that
| `dpc --fragment-origin' emits (crates/dpc-m68k/src/emit.rs): the
| origin symbol, the `.space 2' that makes section-relative alignment
| absolute, and the filler written as `.balignw 4,0x2048' (what the
| Align4 pseudo-instruction emits) rather than as an instruction.  It says
| nothing about what the compiler emits.

	.set	__dpc_fragment_origin,0xe128ba

	.text
	.space	2				| not part of the unit
	.globl	PKT_$RECENTLY_MISSING
PKT_$RECENTLY_MISSING:
	link.w	%fp,#-8				| 4e56 fff8
	movem.l	%d2/%a5,-(%sp)			| 48e7 2004
	lea	(PKT_$MISSING_NODE).l,%a5	| 4bf9 00e2 4c9c (module data)
	move.l	(8,%fp),%d1			| 222e 0008
	move.w	(0x58,%a5),%d2			| 342d 0058 (PKT_$N_MISSING)
	clr.b	%d0				| 4200
	subq.w	#1,%d2				| 5342
	bmi.s	3f				| 6b16
	lea	(8,%a5),%a0			| 41ed 0008
	.balignw	4,0x2048		| 2048 (loop-head alignment filler)
1:	cmp.l	(-8,%a0),%d1			| b2a8 fff8
	bne.s	2f				| 6604
	st	%d0				| 50c0
	bra.s	3f				| 6006
2:	addq.l	#8,%a0				| 5088
	dbf	%d2,1b				| 51ca fff2
3:	movem.l	(-16,%fp),%d2/%a5		| 4cee 2004 fff0
	unlk	%fp				| 4e5e
	rts					| 4e75
