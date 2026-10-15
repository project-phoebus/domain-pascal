.text
.globl OVL_$CNTL
OVL_$CNTL:
link.w %fp,#0
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
tst.w (PCHIST_UNWIRED_D+0x936).l
beq.s 1f
clr.w (PCHIST_UNWIRED_D+0x936).l
1:
clr.b (PCHIST_UNWIRED_D+0x50e).l
pea (PCHIST_UNWIRED_D+0x500).l
pea (PCHIST_UNWIRED_D+0x936).l
jsr (OVL_$WIRE).l
addq.w #8,%sp
move.l #PCHIST_UNWIRED_D,-(%sp)
jsr (OVL_$LOCK).l
st (%a5)
move.w (OVL_$INIT_SEC).l,(OVL_$INIT_SEC+8).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl OVL_$COUNT
OVL_$COUNT:
link.w %fp,#0
movea.l (8,%fp),%a0
move.w (%a0),%d0
movea.l #PCHIST_UNWIRED_D,%a0
lsl.w #2,%d0
move.w %d0,%d1
lsl.w #2,%d1
add.w %d1,%d0
clr.l (-8,%a0,%d0.w)
clr.l (OVL_$INIT_SEC+4).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.OVL.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 4
.comm PCHIST_UNWIRED_D,2360,4
.section ".data.overlay.OVL_$INIT_SEC","aw",@progbits
.balign 4
.globl OVL_$INIT_SEC
.type OVL_$INIT_SEC,@object
.size OVL_$INIT_SEC,12
OVL_$INIT_SEC:
.byte 0x00,0x03
.space 10
