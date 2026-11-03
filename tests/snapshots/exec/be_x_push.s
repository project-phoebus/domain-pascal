.text
.globl TWO
TWO:
link.w %fp,#0
subq.l #2,%sp
movea.l (8,%fp),%a0
move.w (%a0),%d0
add.w %d0,%d0
move.w %d0,%d1
movea.l (0xc,%fp),%a1
lsl.w #2,%d1
add.w %d1,%d0
add.w (%a1),%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
unlk %fp
rts
.globl BX_PUSH_$TWICE
BX_PUSH_$TWICE:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
moveq #2,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
move.w %d3,(-2,%fp)
move.w %d3,(-2,%fp)
pea (-2,%fp)
pea (-2,%fp)
bsr.s TWO
addq.w #8,%sp
addq.w #1,%d3
dbf %d2,2b
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl BW
BW:
link.w %fp,#0
subq.l #2,%sp
move.b (8,%fp),-(%sp)
jsr (EXEC_$LOGV_C).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0xa,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
unlk %fp
rts
.globl BX_PUSH_$BYTES
BX_PUSH_$BYTES:
link.w %fp,#0
move.w #5,-(%sp)
move.w #0x6100,-(%sp)
bsr.s BW
unlk %fp
rts
.globl KEEP
KEEP:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),(0x10,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_PUSH_$ADDRP
BX_PUSH_$ADDRP:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
lea (%a5),%a0
move.l %a0,(8,%a5)
clr.l (0x10,%a5)
move.l (8,%a5),-(%sp)
bsr.s KEEP
addq.w #4,%sp
move.l (0x10,%a5),%d0
cmp.l (8,%a5),%d0
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_PUSH_$CHAIN
BX_PUSH_$CHAIN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #7,(2,%a5)
lea (%a5),%a0
move.l %a0,(8,%a5)
lea (8,%a5),%a0
move.l %a0,(0xc,%a5)
movea.l (0xc,%a5),%a0
movea.l (%a0),%a1
pea (2,%a1)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl KEEPIN
KEEPIN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0),(0x10,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_PUSH_$ADDRSYM
BX_PUSH_$ADDRSYM:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
pea (.Lc0,%pc)
bsr.s KEEPIN
addq.w #4,%sp
move.l (0x10,%a5),(-4,%fp)
move.l (-4,%fp),-(%sp)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
movea.l (-4,%fp),%a0
move.w #0x2c,(%a0)
move.l #EB_$K,-(%sp)
jsr (EXEC_$LOG_W).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.long EB_$K
.balignw 4,0x2048
.section ".bss.moddata.BE_X_PUSH","aw",@nobits
.balign 4
.Ldata:
.space 20
