.text
.globl SHOW
SHOW:
link.w %fp,#0
movea.l (8,%fp),%a0
move.l (4,%a0),-(%sp)
jsr (EXEC_$LOGV_L).l
unlk %fp
rts
.globl PARK_INT_CALLS_$AFTER_CALL
PARK_INT_CALLS_$AFTER_CALL:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a2
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (4,%a2),%d0
add.l %d2,%d0
move.l %d0,(4,%a2)
move.l (4,%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
1:
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
.globl PARK_INT_CALLS_$CASE
PARK_INT_CALLS_$CASE:
link.w %fp,#-4
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
move.w (%a5),%d1
cmpi.w #4,%d1
bcc.s 6f
add.w %d1,%d1
move.w (7f,%pc,%d1.w),%d1
jmp (7f,%pc,%d1.w)
7:
.short 2f-7b
.short 3f-7b
.short 4f-7b
.short 5f-7b
2:
move.w #3,(%a5)
bra.s 1f
3:
move.w #4,(%a5)
bra.s 1f
4:
move.w #5,(%a5)
bra.s 1f
5:
move.w #6,(%a5)
bra.s 1f
6:
clr.w (%a5)
1:
movea.l %d0,%a2
move.w (%a5),%d0
ext.l %d0
subq.l #3,%d0
add.l %d2,%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (4,%a2),%d0
add.l %d2,%d0
move.l %d0,(4,%a2)
move.l (4,%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x10,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl PARK_INT_CALLS_$BY_ADDR
PARK_INT_CALLS_$BY_ADDR:
link.w %fp,#-4
movem.l %d2/%a2,-(%sp)
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a2
move.l (%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (4,%a2),%d0
add.l %d2,%d0
move.l %d0,(4,%a2)
pea (%a2)
bsr.w SHOW
1:
movem.l (-0xc,%fp),%d2/%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PARK_INT_CALLS","aw",@progbits
.balign 4
.Ldata:
.space 4
