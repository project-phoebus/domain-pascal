.text
REMAINDER:
link.w %fp,#-8
clr.l %d0
move.w (%a5),%d0
clr.l %d1
divu.w #0x65,%d0
swap %d0
move.w %d0,%d1
move.l %d1,%d0
unlk %fp
rts
.globl STORES
STORES:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.l %d0,%d1
addq.l #1,%d1
move.l %d1,(0x10,%a5)
clr.l %d0
move.w (%a5),%d0
move.l %d0,(4,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl POP_PAD
POP_PAD:
link.w %fp,#0
subq.l #2,%sp
move.w #5,-(%sp)
jsr (TAKE).l
addq.w #4,%sp
subq.l #2,%sp
movea.l (8,%fp),%a0
move.w (4,%a0),-(%sp)
jsr (TAKE).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SCHEDULE","aw",@nobits
.balign 4
.Ldata:
.space 20
