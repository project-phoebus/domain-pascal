.text
.globl GET
GET:
link.w %fp,#-8
movem.l %d2/%a2,-(%sp)
movea.l (0xc,%fp),%a2
clr.w %d2
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
pea (%a2)
jsr (LOCK).l
move.w (0x24,%a2),%d2
1:
move.w %d2,%d0
movem.l (-0x10,%fp),%d2/%a2
unlk %fp
rts
.globl SETFC
SETFC:
link.w %fp,#-4
movem.l %d2/%a2-%a3,-(%sp)
movea.l (0xc,%fp),%a2
movea.l (0x10,%fp),%a3
subq.l #2,%sp
pea (%a3)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a3)
bne.s 1f
cmpi.w #0x11,(%a2)
ble.s 2f
move.l #0x350002,(%a3)
bra.s 1f
2:
movea.l %d0,%a3
move.w (%a2),%d1
move.l (0x20,%a3),%d2
btst %d1,%d2
beq.s 3f
move.w #0x12,(0x26,%a3)
3:
tst.b (0x28,%a3)
bmi.s 4f
subq.l #2,%sp
st -(%sp)
pea (%a3)
jsr (SET_DFL).l
4:
1:
movem.l (-0x10,%fp),%d2/%a2-%a3
unlk %fp
rts
.balignw 4,0x2048
