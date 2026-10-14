.text
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
move.l (0x14,%a0),(%a1)
1:
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl SETF
SETF:
link.w %fp,#-4
movem.l %d2/%a2-%a3,-(%sp)
movea.l (0xc,%fp),%a2
movea.l (0x14,%fp),%a3
subq.l #2,%sp
pea (%a3)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a3)
bne.s 1f
movea.l (0x10,%fp),%a1
movea.l %d0,%a0
tst.b (%a1)
bpl.s 2f
clr.l %d1
move.w (%a2),%d2
bset %d2,%d1
or.l %d1,(0x14,%a0)
bra.s 3f
2:
clr.l %d1
move.w (%a2),%d2
bset %d2,%d1
not.l %d1
and.l %d1,(0x14,%a0)
3:
1:
movem.l (-0x10,%fp),%d2/%a2-%a3
unlk %fp
rts
.globl PUT
PUT:
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
move.w (%a1),(0x18,%a0)
1:
movea.l (-8,%fp),%a2
unlk %fp
rts
.balignw 4,0x2048
