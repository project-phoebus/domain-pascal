/*
 * runtime/m68k/pas_set_build.s - dpc's m68k run-time library: PAS_$SET_BUILD,
 * the Domain Pascal run-time routine that builds a set from a template and
 * a range (the set constructor with a run-time range), shipped with dpc as
 * a prebuilt object beside M$ARITH (runtime/README.md; bead
 * domain-pascal-jzm5).
 *
 * Transcribed by: Claude Opus 5.5 (bead domain-pascal-jzm5, 2026-10-15)
 *
 * SAU2 map: `I E11FA8 PAS_$SET_BUILD size = 4C' (0xE11FA8 .. 0xE11FF4,
 * in WIRED_PROC), one entry point.  Transcribed instruction for instruction
 * from the SR10.2 SAU2 image (tools/imgdis.sh E11FA8 4C); the image address
 * of each instruction is in its comment.  The 0x4C bytes include the zero
 * word after the `rts' (0xE11FF2), so it is transcribed too.  Assembled
 * with `m68k-elf-as -m68010' its section .text.PAS_$SET_BUILD equals the
 * image's 0x4C bytes at 0xE11FA8 (`make conform',
 * tests/conformance/manifest.txt).  Do not "clean it up".
 *
 * This is hand-written assembly, not compiled Pascal (the signs of
 * codegen-patterns 0): no link frame (only a movem push and pop of
 * D2-D3/A2), the arguments read straight off SP, and the OR form with an
 * immediate effective address `807c' where compiled code uses ori
 * (0xE03498 `ori.w #0x2000,D1', codegen-patterns 8).  The zero word after
 * the `rts' pads the segment to a longword (the next one, PEB_PARITY,
 * starts at 0xE11FF4); non-aligning compiled code ends `rts; 0x0000' too
 * (codegen-patterns 7), so that word is no sign either way.
 *
 * Arguments (after the 12 bytes of saved registers and the return
 * address), Domain Pascal slots, pushed right to left:
 *
 *   (0x10,SP)  destination set, by address    -> A1 (and A2 for the copy)
 *   (0x14,SP)  template set, by address       -> A0
 *   (0x18,SP)  lo, word by value              -> D1w
 *   (0x1A,SP)  hi, word by value              -> D2w
 *   (0x1C,SP)  n,  word by value              -> D0w
 *
 * The body copies (n >> 4) + 1 words of the template to the destination,
 * clamps lo to 0 and hi to n (signed compares), and sets members lo..hi:
 * member k is bit k & 7 of byte ((n | 0xF) - k) >> 3, the big-endian set
 * layout of codegen-patterns 5.1.  The image has no caller of the
 * routine, so what a compiler passed as n is not attested; the C stand-in
 * in ../source (pas/set_build.c) calls it total_bits.
 *
 * dpc never emits a call to it (a set constructor with a run-time range
 * is refused); it is shipped so that a kernel linked with dpc's runtime
 * has the image's bytes for this map segment.
 */

        .section ".text.PAS_$SET_BUILD","ax",@progbits
        .balign 2

        .global PAS_$SET_BUILD

PAS_$SET_BUILD:
        movem.l %d2-%d3/%a2, -(%sp)     /* 00e11fa8 */
        movea.l (0x14,%sp), %a0         /* 00e11fac  template */
        movea.l (0x10,%sp), %a1         /* 00e11fb0  destination */
        move.w  (0x1c,%sp), %d0         /* 00e11fb4  n */
        movea.l %a1, %a2                /* 00e11fb8 */
        move.w  %d0, %d1                /* 00e11fba */
        lsr.w   #4, %d1                 /* 00e11fbc  (n >> 4) + 1 words */
1:      move.w  (%a0)+, (%a2)+          /* 00e11fbe  copy the template */
        dbf     %d1, 1b                 /* 00e11fc0 */
        move.w  (0x18,%sp), %d1         /* 00e11fc4  lo */
        bge.s   2f                      /* 00e11fc8 */
        clr.w   %d1                     /* 00e11fca  lo < 0: 0 */
2:      move.w  (0x1a,%sp), %d2         /* 00e11fcc  hi */
        cmp.w   %d2, %d0                /* 00e11fd0 */
        bge.s   3f                      /* 00e11fd2 */
        move.w  %d0, %d2                /* 00e11fd4  hi > n: n */
3:      .short  0x807c, 0x000f          /* 00e11fd6  or.w #0xf,%d0 (gas: ori) */
4:      cmp.w   %d1, %d2                /* 00e11fda */
        blt.s   5f                      /* 00e11fdc  lo > hi: done */
        move.w  %d0, %d3                /* 00e11fde */
        sub.w   %d1, %d3                /* 00e11fe0 */
        lsr.w   #3, %d3                 /* 00e11fe2  byte ((n | 0xF) - lo) >> 3 */
        bset    %d1, (0,%a1,%d3.w)      /* 00e11fe4  bit lo & 7 */
        addq.w  #1, %d1                 /* 00e11fe8 */
        bra.s   4b                      /* 00e11fea */
5:      movem.l (%sp)+, %d2-%d3/%a2     /* 00e11fec */
        rts                             /* 00e11ff0 */
        .short  0x0000                  /* 00e11ff2  segment end */

        .end
