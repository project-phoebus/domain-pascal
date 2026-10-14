.text
.globl FLUSH
FLUSH:
link.w %fp,#-8
pea (%a2)
movea.l (0xc,%fp),%a2
subq.l #2,%sp
pea (%a2)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a2
jsr (LOCK).l
move.w %d0,(-6,%fp)
pea (%a2)
jsr (FLUSH_DESC).l
addq.w #4,%sp
subq.l #2,%sp
move.w (-6,%fp),-(%sp)
jsr (UNLOCK).l
1:
movea.l (-0xc,%fp),%a2
unlk %fp
rts
.globl INQ
INQ:
link.w %fp,#-4
pea (%a2)
movea.l (0x10,%fp),%a2
subq.l #2,%sp
pea (%a2)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a0
movea.l (0xc,%fp),%a1
move.l (4,%a0),(%a1)
1:
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl SETRAW
SETRAW:
link.w %fp,#-4
movem.l %a2-%a3,-(%sp)
movea.l (0xc,%fp),%a3
subq.l #2,%sp
pea (%a3)
move.w (8,%fp),-(%sp)
jsr (GET_DESC).l
addq.w #8,%sp
movea.l %a0,%a2
tst.l (%a3)
bne.s 1f
subq.l #2,%sp
move.b (0xa,%fp),-(%sp)
pea (%a2)
jsr (SET_RAW_MODE).l
1:
movem.l (-0xc,%fp),%a2-%a3
unlk %fp
rts
.globl INQRAW
INQRAW:
link.w %fp,#-4
pea (%a2)
movea.l (0xe,%fp),%a2
subq.l #2,%sp
pea (%a2)
move.w (8,%fp),-(%sp)
jsr (GET_DESC).l
addq.w #8,%sp
tst.l (%a2)
bne.s 1f
movea.l (0xa,%fp),%a1
move.b (8,%a0),(%a1)
1:
movea.l (-8,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
