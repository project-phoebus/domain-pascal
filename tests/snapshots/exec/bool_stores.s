.text
.globl TAKE
TAKE:
link.w %fp,#0
pea (8,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
pea (0xa,%fp)
jsr (EXEC_$LOG_BOOL).l
unlk %fp
rts
.globl BOOL_STORES_$CMP
BOOL_STORES_$CMP:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,(2,%a5)
cmpi.w #3,%d2
seq %d0
move.b %d0,(%a5)
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
cmpi.w #2,(2,%a5)
sge %d0
move.b %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
lea (4,%a5),%a0
move.l %a0,(0x18,%a5)
move.w (2,%a5),%d0
addq.w #1,%d0
movea.l (0x18,%a5),%a0
btst #0,%d0
sne %d0
move.b %d0,(0x10,%a0)
pea (0x14,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
tst.w %d2
smi %d0
movea.l (0x18,%a5),%a0
move.b %d0,(0x11,%a0)
pea (0x15,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.w %d2,%d0
addq.w #1,%d0
btst #0,%d0
sne %d0
move.b %d0,(%a5)
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BOOL_STORES_$LOOP
BOOL_STORES_$LOOP:
link.w %fp,#-8
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
clr.w %d2
clr.w %d3
.balignw 4,0x2048
1:
cmpi.w #2,%d2
seq %d0
move.b %d0,(-6,%fp)
bpl.s 2f
addq.w #1,%d3
2:
tst.b (-6,%fp)
bmi.s 3f
addq.w #2,%d3
3:
pea (-6,%fp)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
addq.w #1,%d2
cmp.w (2,%a5),%d2
blt.s 1b
subq.l #2,%sp
move.w %d3,%d0
subq.w #4,%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0x14,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BOOL_STORES_$PUSH
BOOL_STORES_$PUSH:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
cmpi.w #1,%d3
bne.s 1f
move.w #8,(0x1c,%a5)
bra.s 2f
1:
move.w #0x10,(0x1c,%a5)
2:
btst #3,(0x1d,%a5)
sne %d0
move.b %d0,-(%sp)
tst.w %d2
sgt %d0
move.b %d0,-(%sp)
bsr.w TAKE
addq.w #4,%sp
btst #0,%d3
sne %d0
move.b %d0,-(%sp)
cmpi.w #5,%d2
seq %d0
move.b %d0,-(%sp)
bsr.w TAKE
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BOOL_STORES_$RANGE
BOOL_STORES_$RANGE:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
move.w #1,(2,%a5)
cmpi.w #4,(%a2)
bhi.s 2f
cmpi.w #2,(%a2)
bls.s 1f
2:
clr.w (2,%a5)
1:
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
cmpi.w #4,(%a2)
sls %d0
move.b %d0,(%a5)
pea (%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BOOL_STORES","aw",@nobits
.balign 4
.Ldata:
.space 32
