.text
.globl ELEM_PTR_CALLS_$SETUP
ELEM_PTR_CALLS_$SETUP:
link.w %fp,#-8
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #3,%d0
movea.l %a5,%a0
.balignw 4,0x2048
2:
movea.l %a0,%a1
move.l #EXEC_$LOGV_L,(0x18,%a1)
move.l #EXEC_$ID_L,(0x1c,%a1)
move.l #EXEC_$ID_BOOL,(0x20,%a1)
move.l #EXEC_$LOG_W,(0xc4,%a1)
lea (0xb0,%a0),%a0
dbf %d0,2b
moveq #3,%d0
clr.w %d1
movea.l %a5,%a0
.balignw 4,0x2048
5:
move.w %d1,%d2
muls.w #0x3e8,%d2
move.l %d2,(0x2d8,%a0)
addq.w #1,%d1
addq.l #4,%a0
dbf %d0,5b
moveq #3,%d0
lea (%a5),%a0
.balignw 4,0x2048
8:
move.w #1,(0x10,%a0)
addq.l #2,%a0
dbf %d0,8b
movem.l (-0x10,%fp),%d2/%a5
unlk %fp
rts
.globl ELEM_PTR_CALLS_$CALL
ELEM_PTR_CALLS_$CALL:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #2,%d1
lea (0,%a5,%d1.w),%a0
move.w %d0,%d1
move.l (0x2d8,%a0),-(%sp)
mulu.w #0xb0,%d1
lea (0,%a5,%d1.w),%a1
movea.l (0x18,%a1),%a2
jsr (%a2)
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl ELEM_PTR_CALLS_$FAR
ELEM_PTR_CALLS_$FAR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (8,%fp)
move.w (8,%fp),%d0
move.w %d0,%d1
mulu.w #0xb0,%d1
lea (0,%a5,%d1.w),%a0
movea.l (0xc4,%a0),%a1
jsr (%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
CHECK:
link.w %fp,#-8
move.l %d2,-(%sp)
move.w (8,%fp),%d0
movea.l (0xa,%fp),%a0
clr.l (%a0)
cmpi.w #1,%d0
slt %d1
cmpi.w #3,%d0
sgt %d2
or.b %d2,%d1
bpl.s 1f
moveq #1,%d0
move.l %d0,(%a0)
bra.s 4f
1:
move.w %d0,%d1
add.w %d1,%d1
tst.w (0x10,%a5,%d1.w)
bne.s 3f
moveq #2,%d0
move.l %d0,(%a0)
3:
2:
tst.l (%a0)
4:
seq %d0
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl ELEM_PTR_CALLS_$ID
ELEM_PTR_CALLS_$ID:
link.w %fp,#-4
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),%d2
move.l (0xa,%fp),-(%sp)
move.w %d2,-(%sp)
bsr.s CHECK
addq.w #8,%sp
tst.b %d0
bpl.s 1f
move.w %d2,%d0
lsl.w #2,%d0
lea (0,%a5,%d0.w),%a0
move.w %d2,%d0
pea (0x2d8,%a0)
mulu.w #0xb0,%d0
lea (0,%a5,%d0.w),%a1
movea.l (0x1c,%a1),%a2
jsr (%a2)
1:
movem.l (-0x10,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl ELEM_PTR_CALLS_$TEST
ELEM_PTR_CALLS_$TEST:
link.w %fp,#-8
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),%d2
move.l (0xc,%fp),-(%sp)
move.w %d2,-(%sp)
bsr.w CHECK
addq.w #8,%sp
tst.b %d0
bpl.s 1f
move.w %d2,%d0
mulu.w #0xb0,%d0
lea (0,%a5,%d0.w),%a0
pea (0xa,%fp)
move.w %d2,%d0
add.w %d0,%d0
move.w (0x10,%a5,%d0.w),%d0
mulu.w #0xb0,%d0
lea (0,%a5,%d0.w),%a0
movea.l (0x20,%a0),%a1
jsr (%a1)
1:
movem.l (-0x10,%fp),%d2/%a5
unlk %fp
rts
.globl ELEM_PTR_CALLS_$CHECKED
ELEM_PTR_CALLS_$CHECKED:
link.w %fp,#-4
move.w (8,%fp),%d0
movea.l (0xa,%fp),%a0
cmpi.w #0x1e,%d0
ble.s 1f
moveq #4,%d0
6:
move.l %d0,(%a0)
bra.s 5f
1:
cmpi.w #1,%d0
bge.s 3f
moveq #3,%d0
bra.s 6b
3:
clr.l (%a0)
4:
2:
tst.l (%a0)
5:
seq %d0
unlk %fp
rts
.globl ELEM_PTR_CALLS_$FILL
ELEM_PTR_CALLS_$FILL:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.w (0x14,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ELEM_PTR_CALLS_$FIND
ELEM_PTR_CALLS_$FIND:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
moveq #7,%d0
move.l %d0,(%a1)
moveq #3,%d0
clr.w %d1
lea (%a5),%a2
.balignw 4,0x2048
2:
tst.w (0x10,%a2)
bne.s 4f
move.w %d1,(%a0)
clr.l (%a1)
bra.s 6f
4:
addq.w #1,%d1
addq.l #2,%a2
dbf %d0,2b
1:
tst.l (%a1)
bne.s 5f
6:
subq.l #2,%sp
move.w #2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
5:
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl ELEM_PTR_CALLS_$CHAIN
ELEM_PTR_CALLS_$CHAIN:
link.w %fp,#0
movem.l %d2/%a2,-(%sp)
move.w (8,%fp),%d2
movea.l (0xa,%fp),%a2
clr.l (%a2)
subq.l #2,%sp
move.w #3,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
cmpi.w #1,%d2
bne.s 1f
moveq #5,%d0
move.l %d0,(%a2)
bra.s 5f
1:
cmpi.w #2,%d2
bne.s 3f
moveq #6,%d0
move.l %d0,(%a2)
3:
2:
tst.l (%a2)
5:
bne.s 4f
subq.l #2,%sp
move.w #4,-(%sp)
jsr (EXEC_$MARK).l
4:
movem.l (-8,%fp),%d2/%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ELEM_PTR_CALLS","aw",@nobits
.balign 4
.Ldata:
.space 744
