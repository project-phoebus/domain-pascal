.text
.globl SET_RANGE_ODD_BYTE_$RUN
SET_RANGE_ODD_BYTE_$RUN:
link.w %fp,#0
movem.l %d2-%d5/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
move.w (0xc,%fp),%d4
moveq #-1,%d0
move.w %d2,%d1
ext.l %d1
move.w %d3,%d5
lsl.l %d1,%d0
moveq #-1,%d1
ext.l %d5
addq.l #1,%d5
lsl.l %d5,%d1
not.l %d1
and.l %d1,%d0
move.l %d0,(%a5)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
moveq #-1,%d0
move.w %d3,%d1
ext.l %d1
move.w %d2,%d5
lsl.l %d1,%d0
moveq #-1,%d1
ext.l %d5
addq.l #1,%d5
lsl.l %d5,%d1
not.l %d1
and.l %d1,%d0
move.l %d0,(%a5)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
moveq #-1,%d0
move.w %d4,%d1
moveq #-1,%d4
ext.l %d1
lsl.l %d1,%d0
move.l %d1,%d5
addq.l #1,%d5
lsl.l %d5,%d4
not.l %d4
and.l %d4,%d0
move.l %d0,(%a5)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w %d2,%d0
subq.w #3,%d0
clr.l %d1
bset %d0,%d1
clr.l %d0
bset %d3,%d0
or.l %d0,%d1
ori.l #1,%d1
move.l %d1,(%a5)
move.l %d1,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.b #7,(4,%a5)
move.b #8,(5,%a5)
clr.w %d0
move.b (4,%a5),%d0
btst #0,%d0
beq.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 5f
1:
subq.l #2,%sp
clr.w -(%sp)
5:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
2:
clr.w %d0
move.b (5,%a5),%d0
btst #0,%d0
beq.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 6f
3:
subq.l #2,%sp
clr.w -(%sp)
6:
jsr (EXEC_$LOGV_W).l
4:
movem.l (-0x14,%fp),%d2-%d5/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SET_RANGE_ODD_BYTE","aw",@nobits
.balign 4
.Ldata:
.space 8
