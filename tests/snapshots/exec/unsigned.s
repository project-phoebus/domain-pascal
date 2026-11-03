.text
.globl UNSIGNED_$CMP
UNSIGNED_$CMP:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d0
move.w %d0,(%a5)
cmp.w (%a5),%d2
bls.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
cmp.w (%a5),%d2
bcc.s 2f
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
cmp.w (%a5),%d2
bhi.s 3f
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
3:
cmp.w (%a5),%d2
bcs.s 4f
subq.l #2,%sp
move.w #4,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
cmp.w (%a5),%d2
bne.s 5f
subq.l #2,%sp
move.w #5,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
5:
cmp.w (%a5),%d2
beq.s 6f
subq.l #2,%sp
move.w #6,-(%sp)
jsr (EXEC_$MARK).l
6:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl UNSIGNED_$WIDEN
UNSIGNED_$WIDEN:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
clr.l %d0
move.w %d2,%d0
move.l %d0,(4,%a5)
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l %d0
move.w %d2,%d0
addq.l #1,%d0
move.l %d0,(4,%a5)
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl UNSIGNED_$BYTES
UNSIGNED_$BYTES:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w %d0,%d2
move.w %d1,%d3
move.w %d2,%d0
ext.l %d0
move.l %d0,(4,%a5)
move.w %d3,%d0
ext.l %d0
add.l (4,%a5),%d0
move.l %d0,(4,%a5)
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w %d3,(8,%a5)
cmp.w (8,%a5),%d2
ble.s 1f
subq.l #2,%sp
move.w #7,-(%sp)
jsr (EXEC_$MARK).l
1:
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.UNSIGNED","aw",@nobits
.balign 4
.Ldata:
.space 12
