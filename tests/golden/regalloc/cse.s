.text
.globl ACKNOWLEDGE
ACKNOWLEDGE:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (PROC1_$AS_ID).l,%d2
lsl.w #2,%d2
move.w %d2,%d0
add.w %d0,%d0
add.w %d0,%d2
move.l (0,%a5,%d2.w),(0x5ac,%a5)
move.w (PROC1_$AS_ID).l,%d0
lea (0,%a5,%d0.w),%a0
clr.b (0x2b8,%a0)
lea (0,%a5,%d2.w),%a0
pea (0x2f4,%a0)
jsr (EC_$ADVANCE).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
READ_SHAPE:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
movea.l (0xc,%fp),%a0
move.w #0x400,(%a0)
cmpi.w #0x400,%d1
bcc.s 1f
moveq #-2,%d2
and.w %d1,%d2
move.w %d2,(%a0)
1:
clr.l %d1
clr.l %d2
move.w %d0,%d1
move.w (%a0),%d2
add.l %d1,%d2
cmpi.l #0x400,%d2
bls.s 2f
cmpi.w #0x400,%d0
bls.s 3f
clr.w (%a0)
bra.s 4f
3:
move.w #0x400,%d2
sub.w %d0,%d2
move.w %d2,(%a0)
4:
2:
move.l %d1,%d0
add.l (0x5b0,%a5),%d0
move.l %d0,(0x5b4,%a5)
move.l (-8,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.CSE","aw",@nobits
.balign 4
.Ldata:
.space 1464
