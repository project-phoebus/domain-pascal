.text
.globl HANDLER
HANDLER:
link.w %fp,#0
movea.l (8,%fp),%a0
addq.l #1,(%a0)
subq.l #2,%sp
movea.l (0xc,%fp),%a1
move.w (%a1),-(%sp)
jsr (EXEC_$MARK).l
unlk %fp
rts
.globl VHANDLER
VHANDLER:
link.w %fp,#0
move.l (8,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
pea (0xc,%fp)
jsr (EXEC_$LOG_BOOL).l
unlk %fp
rts
.globl POINTERS_$INSTALL
POINTERS_$INSTALL:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l #HANDLER,(%a5)
move.l #VHANDLER,(6,%a5)
lea (0xa,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(0x12,%a5)
lea (0x10,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
move.l %a0,(0x18,%a5)
lea (0x16,%a5),%a0
move.l %a0,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$ID_P).l
move.l %a0,(0x1c,%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl POINTERS_$CALL
POINTERS_$CALL:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.l %d0,(-4,%fp)
pea (.Lc0,%pc)
pea (-4,%fp)
movea.l (%a5),%a0
jsr (%a0)
addq.w #8,%sp
move.l (-4,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
pea (.Lc0,%pc)
pea (-4,%fp)
movea.l (%a5),%a0
jsr (%a0)
addq.w #8,%sp
subq.l #2,%sp
st -(%sp)
move.l (-4,%fp),-(%sp)
movea.l (6,%a5),%a0
jsr (%a0)
movea.l (-8,%fp),%a5
unlk %fp
rts
.Lc0:
.short 0x0007
.globl POINTERS_$WALK
POINTERS_$WALK:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
clr.w (0x20,%a5)
move.l (0x1c,%a5),(-4,%fp)
bra.s 2f
.balignw 4,0x2048
1:
subq.l #2,%sp
movea.l (-4,%fp),%a0
move.w (%a0),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
movea.l (-4,%fp),%a0
move.w (0x20,%a5),%d0
add.w (%a0),%d0
move.w %d0,(0x20,%a5)
movea.l (-4,%fp),%a0
move.l (2,%a0),(-4,%fp)
2:
tst.l (-4,%fp)
bne.s 1b
move.w (0x20,%a5),%d0
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.POINTERS","aw",@progbits
.balign 4
.Ldata:
.byte 0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x0a,0x00,0x00,0x00,0x00
.byte 0x00,0x14,0x00,0x00,0x00,0x00,0x00,0x1e
.space 12
