.text
.globl PUT1
PUT1:
link.w %fp,#0
pea (.Lc0,%pc)
move.l (8,%fp),-(%sp)
jsr (EXEC_$LOG_MEM).l
unlk %fp
rts
.Lc0:
.short 0x0001
.globl PUT16
PUT16:
link.w %fp,#0
move.l (8,%fp),-(%sp)
jsr (EXEC_$LOG_L).l
unlk %fp
rts
.globl OGF8_ARGS_$UNIV
OGF8_ARGS_$UNIV:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
pea (.Lc1,%pc)
bsr.s PUT1
addq.w #4,%sp
pea (.Lc2,%pc)
bsr.s PUT1
addq.w #4,%sp
pea (.Lc3,%pc)
bsr.s PUT1
addq.w #4,%sp
pea (.Lc4,%pc)
pea (.Lc5,%pc)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
move.l %d2,(0xc,%a5)
move.l %d2,%d0
lsl.l #8,%d0
lsl.l #2,%d0
move.l %d0,(-4,%fp)
pea (-4,%fp)
bsr.s PUT16
addq.w #4,%sp
subq.l #2,%sp
move.w #0x155,-(%sp)
move.l (0xc,%a5),-(%sp)
jsr (M$MIS$LLW).l
addq.w #8,%sp
neg.l %d0
subq.l #1,%d0
move.l %d0,(-4,%fp)
pea (-4,%fp)
bsr.w PUT16
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.Lc4:
.short 0x0000
.balignw 4,0x2048
.Lc1:
.short 0xffff,0xffff
.Lc5:
.Lc3:
.short 0x0000,0x0000
.Lc2:
.short 0x1234,0x5678
.globl WALK
WALK:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (0xc,%fp),%d2
bra.s 2f
.balignw 4,0x2048
1:
move.l (8,%fp),-(%sp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
movea.l (8,%fp),%a0
cmpi.l #0xa,(%a0)
bne.s 4f
clr.l (8,%fp)
bra.s 5f
4:
move.l (8,%fp),%d0
subq.l #4,%d0
move.l %d0,(8,%fp)
5:
subq.w #1,%d2
2:
tst.w %d2
bgt.s 1b
tst.l (8,%fp)
bne.s 6f
clr.l -(%sp)
jsr (EXEC_$LOGV_L).l
6:
move.l (-4,%fp),%d2
unlk %fp
rts
.globl OGF8_ARGS_$PTR
OGF8_ARGS_$PTR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
pea (8,%a5)
bsr.s WALK
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl OGF8_ARGS_$CMP
OGF8_ARGS_$CMP:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
move.l (0xc,%fp),%d0
move.l %d2,(-4,%fp)
move.l %d0,(0xc,%a5)
pea (-4,%fp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
cmp.l (0xc,%a5),%d0
bne.s 1f
subq.l #2,%sp
move.w #0x6500,-(%sp)
bra.s 8f
1:
subq.l #2,%sp
move.w #0x6e00,-(%sp)
8:
jsr (EXEC_$LOGV_C).l
addq.w #4,%sp
2:
pea (-4,%fp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
cmp.l (0xc,%a5),%d0
ble.s 3f
subq.l #2,%sp
move.w #0x6700,-(%sp)
bra.s 9f
3:
subq.l #2,%sp
move.w #0x6c00,-(%sp)
9:
jsr (EXEC_$LOGV_C).l
addq.w #4,%sp
4:
pea (-4,%fp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
cmp.l (0xc,%a5),%d0
blt.s 5f
subq.l #2,%sp
move.w #0x4700,-(%sp)
bra.s 10f
5:
subq.l #2,%sp
move.w #0x4c00,-(%sp)
10:
jsr (EXEC_$LOGV_C).l
addq.w #4,%sp
6:
clr.w (0x10,%a5)
move.w %d2,(0x12,%a5)
.balignw 4,0x2048
7:
addq.w #1,(0x10,%a5)
pea (0x10,%a5)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
cmp.w (0x12,%a5),%d0
blt.s 7b
subq.l #2,%sp
move.w (0x10,%a5),-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.OGF8_ARGS","aw",@progbits
.balign 4
.Ldata:
.byte 0x00,0x00,0x00,0x0a,0x00,0x00,0x00,0x14,0x00,0x00,0x00,0x1e
.space 8
