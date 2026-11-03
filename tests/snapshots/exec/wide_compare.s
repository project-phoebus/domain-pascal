.text
.globl WIDE_COMPARE_$MAIN
WIDE_COMPARE_$MAIN:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d2
move.w %d0,(%a5)
move.w (%a5),%d0
move.w %d2,%d1
ext.l %d0
ext.l %d1
add.l %d1,%d0
cmpi.l #5,%d0
ble.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
1:
move.w (%a5),%d0
move.w %d2,%d1
ext.l %d0
ext.l %d1
sub.l %d1,%d0
cmpi.l #-0x8000,%d0
bge.s 2f
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
move.w (%a5),%d0
move.w %d2,%d1
ext.l %d0
ext.l %d1
add.l %d1,%d0
cmpi.l #1,%d0
bge.s 3f
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
3:
move.w (%a5),%d0
add.w %d2,%d0
bpl.s 4f
subq.l #2,%sp
move.w #4,-(%sp)
jsr (EXEC_$MARK).l
4:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.WIDE_COMPARE","aw",@nobits
.balign 4
.Ldata:
.space 4
