.arch 68020
.text
.globl P
P:
link.w %fp,#0
movem.l %d2/%a2,-(%sp)
movea.l (8,%fp),%a2
move.w (0xc,%fp),%d2
subq.l #2,%sp
move.w %d2,-(%sp)
pea (%a2)
jsr (Q).l
addq.w #8,%sp
subq.l #2,%sp
move.w %d2,-(%sp)
pea (%a2)
jsr (Q).l
move.w %d2,(2,%a2)
movem.l (-8,%fp),%d2/%a2
unlk %fp
rts
.globl STORES
STORES:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w (0xc,%fp),%d0
move.w %d0,(%a0)
move.w %d0,(2,%a0)
unlk %fp
rts
.balignw 4,0x2048
