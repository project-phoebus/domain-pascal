.text
.globl BX_REGS_$LOCAL
BX_REGS_$LOCAL:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.b (0xa,%fp),%d1
move.w %d0,(4,%a5)
move.b %d1,(2,%a5)
move.w (4,%a5),%d2
tst.b (2,%a5)
bpl.s 1f
move.w %d2,%d0
addq.w #1,%d0
move.w %d0,(4,%a5)
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
1:
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BX_REGS_$LATE
BX_REGS_$LATE:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
lea (8,%a5),%a1
move.l %a1,(0x10,%a5)
tst.w (%a0)
ble.s 1f
move.w (%a0),%d0
movea.l (0x10,%a5),%a1
ext.l %d0
move.l %d0,(%a1)
bra.s 2f
1:
move.w (%a0),(%a5)
2:
move.w (%a0),%d0
movea.l (0xc,%fp),%a1
addq.w #1,%d0
move.w %d0,(%a1)
move.w (%a0),%d0
movea.l (0x10,%fp),%a2
addq.w #2,%d0
move.w %d0,(%a2)
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl BX_REGS_$SHOW
BX_REGS_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (8,%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_REGS_$LATE3
BX_REGS_$LATE3:
link.w %fp,#0
pea (%a2)
movea.l (0xc,%fp),%a2
clr.l (%a2)
movea.l (8,%fp),%a0
cmpi.w #3,(%a0)
ble.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
moveq #5,%d0
move.l %d0,(%a2)
1:
moveq #6,%d0
movea.l (0x10,%fp),%a0
move.l %d0,(%a0)
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl BX_REGS_$LATE1
BX_REGS_$LATE1:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0x10,%fp),%a0
move.w #0x46,(0x1e,%a5)
tst.w (%a0)
ble.s 1f
movea.l (8,%fp),%a1
move.w (%a0),(%a1)
1:
movea.l (0xc,%fp),%a1
move.w (0x1e,%a5),(%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_REGS_$LATE4
BX_REGS_$LATE4:
link.w %fp,#0
pea (%a2)
movea.l (0x14,%fp),%a2
movea.l (8,%fp),%a0
move.w (%a2),(%a0)
jsr (EXEC_$CLOBBER).l
movea.l (0xc,%fp),%a0
move.w #1,(%a0)
tst.w (%a2)
bne.s 1f
movea.l (0x10,%fp),%a1
move.w #2,(%a1)
1:
movea.l (-4,%fp),%a2
unlk %fp
rts
.globl BX_REGS_$TWICE
BX_REGS_$TWICE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),(0x18,%a5)
clr.w (0x14,%a5)
movea.l (0x18,%a5),%a0
move.w (%a0),%d0
beq.s 2f
cmpi.w #0xff,%d0
ble.s 1f
2:
move.w #1,(0x14,%a5)
1:
move.w %d0,(0x16,%a5)
pea (0x14,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BX_REGS_$CALL
BX_REGS_$CALL:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.w (0x14,%a5)
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d1
bne.s 1f
move.w #1,(0x14,%a5)
1:
move.w %d1,(0x16,%a5)
pea (0x14,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_X_REGS","aw",@nobits
.balign 4
.Ldata:
.space 36
