.text
.globl BU_$ADDR_A
BU_$ADDR_A:
link.w %fp,#-8
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #3,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #5,%d0
movea.l #BU_$BLKS,%a0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a1
lea (-0xf8,%a1),%a0
move.l (%a0),%d0
add.l (0xf4,%a0),%d0
unlk %fp
rts
.globl BU_$ADDR_THRU
BU_$ADDR_THRU:
link.w %fp,#-8
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #3,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #5,%d0
movea.l (BU_$BP).l,%a0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a0
lea (-0xf8,%a0),%a1
move.l (0xf4,%a1),%d0
unlk %fp
rts
.globl BU_$ADDR_STORE
BU_$ADDR_STORE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #3,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #5,%d0
movea.l #BU_$BLKS,%a0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a1
lea (-0xf8,%a1),%a0
move.l %a0,(%a5)
movea.l #BU_$BLKS,%a0
lea (0,%a0,%d1.w),%a0
move.l %a0,(6,%a5)
movea.l (%a5),%a0
movea.l (6,%a5),%a1
move.l (0xf4,%a0),%d0
movea.l (%a5),%a0
add.l (%a1),%d0
sub.l (%a0),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl LOG_Z
LOG_Z:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (8,%fp),%a1
move.l (0xf4,%a0),%d0
add.l (0xf4,%a1),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
unlk %fp
rts
.globl BU_$ADDR_ARG
BU_$ADDR_ARG:
link.w %fp,#0
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #3,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #5,%d0
movea.l #BU_$BLKS,%a0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a1
pea (-0xf8,%a1)
bsr.s LOG_Z
unlk %fp
rts
.globl IS_SECOND
IS_SECOND:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.w %d0
movea.l (0x24,%a5),%a0
move.l (8,%fp),%d2
move.l (%a0),%d1
cmp.l %d2,%d1
bne.s 1f
moveq #1,%d0
1:
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BU_$PCMP
BU_$PCMP:
link.w %fp,#-0x1c
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
lea (0xc,%a5),%a0
move.l %a0,(0x24,%a5)
lea (0x14,%a5),%a0
move.l %a0,(0xc,%a5)
lea (0x1c,%a5),%a0
move.l %a0,(0x14,%a5)
clr.l (0x1c,%a5)
move.w %d2,%d0
lsl.w #3,%d0
pea (4,%a5,%d0.w)
bsr.s IS_SECOND
addq.w #4,%sp
move.w %d0,%d1
lea (-0x10,%fp),%a0
move.l %a0,%d0
lea (-0x10,%fp),%a0
move.l %a0,%d3
cmp.l %d3,%d0
bne.s 1f
addi.w #0xa,%d1
1:
move.w %d2,%d0
lsl.w #3,%d0
lea (4,%a5,%d0.w),%a0
move.l %a0,(-0x18,%fp)
move.l (-0x18,%fp),%d0
lea (0x14,%a5),%a0
move.l %a0,%d2
cmp.l %d2,%d0
beq.s 2f
addi.w #0x64,%d1
2:
move.w %d1,%d0
movem.l (-0x28,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BU_$ADDR_ARITH
BU_$ADDR_ARITH:
link.w %fp,#-0x10
movem.l %d2-%d6,-(%sp)
move.w (8,%fp),%d3
move.l (0xa,%fp),%d2
move.l #BU_$BLKS,%d4
move.w %d3,%d0
lsl.w #3,%d0
move.w %d0,%d1
neg.w %d0
lsl.w #5,%d1
movea.l #BU_$BLKS,%a0
add.w %d1,%d0
move.l %d2,%d5
lea (0,%a0,%d0.w),%a1
lea (-0xf8,%a1),%a0
add.l %a0,%d5
movea.l #BU_$BLKS,%a0
sub.l %d4,%d5
lea (0,%a0,%d0.w),%a0
move.l %d2,%d6
lea (-0xf8,%a0),%a1
move.l %d5,%d0
add.l %a1,%d6
sub.l %d4,%d6
sub.l %d6,%d0
addi.l #0x3e8,%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w %d3,%d0
lsl.w #3,%d0
move.w %d0,%d1
neg.w %d0
lsl.w #5,%d1
movea.l #BU_$BLKS,%a0
add.w %d1,%d0
move.l %d2,%d5
lea (0,%a0,%d0.w),%a1
lea (-0xf8,%a1),%a0
sub.l %a0,%d5
movea.l #BU_$BLKS,%a0
add.l %d4,%d5
lea (0,%a0,%d0.w),%a0
lea (-0xf8,%a0),%a1
move.l %a1,%d6
move.l %d5,%d0
sub.l %d2,%d6
sub.l %d4,%d6
add.l %d6,%d0
addi.l #0x1f5,%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
move.l %d6,%d0
addi.l #0x1fe,%d0
movem.l (-0x24,%fp),%d2-%d6
unlk %fp
rts
.globl BU_$ADDR_MASK
BU_$ADDR_MASK:
link.w %fp,#-8
move.l %d2,-(%sp)
move.w (8,%fp),%d1
move.l (0xa,%fp),%d0
move.w %d1,%d2
lsl.w #3,%d2
move.w %d2,%d1
neg.w %d2
lsl.w #5,%d1
movea.l #BU_$BLKS,%a0
add.w %d1,%d2
move.l %d0,%d1
lea (0,%a0,%d2.w),%a1
andi.l #0x3ff,%d1
lea (-0xf8,%a1),%a0
add.l %a0,%d1
move.l %d1,%d0
.short 0x90bc
.long BU_$BLKS
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl BU_$ADDR_BITS
BU_$ADDR_BITS:
link.w %fp,#-0xc
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l (-4,%fp)
lea (-4,%fp),%a0
lea (-4,%fp),%a1
move.l %a1,%d2
lea (-4,%fp),%a1
andi.l #1,%d2
move.l %a1,%d0
ori.l #1,%d0
add.l %d0,%d2
sub.l %a0,%d2
move.l %d2,(-8,%fp)
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
lea (4,%a5),%a0
lea (6,%a5),%a1
move.l %a1,%d2
addi.l #4,%d2
sub.l %a0,%d2
move.l %d2,(-8,%fp)
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
lea (-4,%fp),%a0
lea (-4,%fp),%a1
move.l %a1,%d0
eori.l #1,%d0
sub.l %a0,%d0
add.l (-4,%fp),%d0
movem.l (-0x14,%fp),%d2/%a5
unlk %fp
rts
.globl OWN_STATIC
OWN_STATIC:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
lea (%a5),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl ELEM_OF
ELEM_OF:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #3,%d1
move.w %d1,%d0
neg.w %d1
lsl.w #5,%d0
movea.l #BU_$BLKS,%a0
add.w %d0,%d1
lea (0,%a0,%d1.w),%a1
lea (-0xf8,%a1),%a0
unlk %fp
rts
.globl SAME_PTR
SAME_PTR:
link.w %fp,#-4
move.l (8,%fp),%d0
move.l %d0,(-4,%fp)
movea.l %d0,%a0
unlk %fp
rts
.globl NEXT_BLK
NEXT_BLK:
link.w %fp,#-4
move.l (8,%fp),%d0
addi.l #0xf8,%d0
move.l %d0,(-4,%fp)
movea.l %d0,%a0
unlk %fp
rts
.globl FIELD_OF
FIELD_OF:
link.w %fp,#-4
movea.l (8,%fp),%a0
movea.l (%a0),%a0
lea (%a0),%a0
unlk %fp
rts
.globl BU_$PRESULT
BU_$PRESULT:
link.w %fp,#-0x14
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l %d2
bsr.w OWN_STATIC
movea.l %a0,%a2
lea (%a5),%a0
move.l %a2,%d0
move.l %a0,%d1
cmp.l %d1,%d0
bne.s 1f
moveq #1,%d2
1:
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.w ELEM_OF
addq.w #4,%sp
movea.l %a0,%a2
add.l (0xf4,%a2),%d2
pea (%a2)
bsr.s SAME_PTR
addq.w #4,%sp
movea.l %a0,%a2
add.l (%a2),%d2
pea (%a2)
bsr.s NEXT_BLK
addq.w #4,%sp
movea.l %a0,%a2
lea (0x1c,%a5),%a0
add.l (%a2),%d2
move.l %a0,(0xc,%a5)
move.l #0x3e8,(0x20,%a5)
pea (0xc,%a5)
bsr.s FIELD_OF
add.l (4,%a0),%d2
move.l %d2,%d0
movem.l (-0x20,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl CK
CK:
link.w %fp,#-4
move.w (8,%fp),%d0
addq.w #1,%d0
unlk %fp
rts
.globl FIND
FIND:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #2,%d1
move.l (0x30,%a5,%d1.w),(-4,%fp)
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BU_$CALLCMP
BU_$CALLCMP:
link.w %fp,#-0x10
movem.l %d2-%d4/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.l (0xa,%fp),%d0
moveq #-2,%d1
bra.s 2f
.balignw 4,0x2048
1:
move.w %d1,%d3
lsl.w #2,%d3
clr.l (0x30,%a5,%d3.w)
addq.w #1,%d1
2:
cmpi.w #5,%d1
ble.s 1b
lea (0xc,%a5),%a0
move.l %a0,(0x3c,%a5)
lea (0xc,%a5),%a0
move.l %a0,(0x40,%a5)
move.w %d0,(-2,%fp)
lea (-2,%fp),%a2
clr.w %d3
tst.w %d2
ble.s 4f
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.w CK
addq.w #4,%sp
move.w %d0,%d1
ext.l %d1
move.l %d1,%d0
clr.l %d1
move.w (%a2),%d1
cmp.l %d1,%d0
sne %d4
tst.b %d4
bpl.s 4f
addq.w #1,%d3
4:
cmpi.w #5,%d2
bgt.s 6f
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.w FIND
addq.w #4,%sp
move.l %a0,%d0
tst.l %d0
seq %d4
tst.b %d4
bpl.s 5f
6:
addi.w #0xa,%d3
5:
cmpi.w #7,%d2
beq.s 7f
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.w CK
addq.w #4,%sp
move.w %d0,%d1
ext.l %d1
move.l %d1,%d0
move.w %d2,%d1
ext.l %d1
add.l %d1,%d1
subq.l #1,%d1
cmp.l %d1,%d0
seq %d4
tst.b %d4
bpl.s 7f
addi.w #0x64,%d3
7:
tst.w %d2
bpl.s 8f
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.w CK
addq.w #4,%sp
move.w %d0,%d1
ext.l %d1
move.l %d1,%d0
clr.l %d1
move.w (%a2),%d1
cmp.l %d1,%d0
sne %d2
tst.b %d2
bpl.s 8f
addi.w #0x3e8,%d3
8:
move.w %d3,%d0
movem.l (-0x24,%fp),%d2-%d4/%a2/%a5
unlk %fp
rts
.globl BU_$INSET
BU_$INSET:
link.w %fp,#-0x10
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w #-0x7ff6,%d1
move.w #0x81,(0x48,%a5)
lea (0x48,%a5),%a0
clr.w %d2
moveq #0xf,%d3
cmp.w %d0,%d3
bcs.s 1f
btst %d0,%d1
bls.s 1f
addq.w #1,%d2
1:
moveq #0xf,%d3
cmp.w %d0,%d3
bcs.s 4f
move.w (0x48,%a5),%d3
btst %d0,%d3
bhi.s 3f
4:
addi.w #0xa,%d2
3:
moveq #0xf,%d3
cmp.w %d0,%d3
btst %d0,%d1
shi %d1
tst.b %d1
bpl.s 5f
addi.w #0x64,%d2
5:
moveq #0xf,%d1
cmp.w %d0,%d1
bcs.s 6f
move.w (%a0),%d1
btst %d0,%d1
bls.s 6f
addi.w #0x3e8,%d2
6:
move.w %d2,%d0
movem.l (-0x1c,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BU_$UZERO
BU_$UZERO:
link.w %fp,#-8
move.l %d2,-(%sp)
move.w (8,%fp),%d0
clr.w %d1
move.w %d0,%d2
ble.s 1f
addq.w #1,%d1
1:
tst.w %d2
bgt.s 2f
addq.w #2,%d1
2:
tst.w %d2
bpl.s 3f
addq.w #4,%d1
3:
tst.w %d2
bmi.s 4f
addq.w #8,%d1
4:
clr.l %d2
move.w %d0,%d2
moveq #6,%d0
and.l %d2,%d0
beq.s 5f
addi.w #0x10,%d1
5:
moveq #6,%d0
and.l %d2,%d0
bne.s 6f
addi.w #0x20,%d1
6:
moveq #6,%d0
and.l %d2,%d0
tst.l %d0
bcc.s 7f
addi.w #0x40,%d1
7:
moveq #6,%d0
and.l %d2,%d0
tst.l %d0
bcs.s 8f
addi.w #0x80,%d1
8:
move.w %d1,%d0
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl BU_$SZERO
BU_$SZERO:
link.w %fp,#-0xc
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d1
move.l (0xa,%fp),%d0
clr.w %d2
move.w %d1,%d3
move.l %d0,%d1
tst.w %d3
ble.s 1f
addq.w #1,%d2
1:
tst.w %d3
bgt.s 2f
addq.w #2,%d2
2:
tst.w %d3
bpl.s 3f
addq.w #4,%d2
3:
tst.w %d3
bmi.s 4f
addq.w #8,%d2
4:
tst.l %d1
ble.s 5f
addi.w #0x10,%d2
5:
tst.l %d1
bgt.s 6f
addi.w #0x20,%d2
6:
tst.l %d1
bpl.s 7f
addi.w #0x40,%d2
7:
tst.l %d1
bmi.s 8f
addi.w #0x80,%d2
8:
move.l %d1,%d0
subq.l #1,%d0
bpl.s 9f
addi.w #0x100,%d2
9:
move.w %d2,%d0
movem.l (-0x14,%fp),%d2-%d3
unlk %fp
rts
.globl BU_$STRS
BU_$STRS:
link.w %fp,#-0x16c
lea (.Lc0,%pc),%a0
lea (-0x10,%fp),%a1
moveq #8,%d0
.balignw 4,0x2048
1:
move.b (%a0)+,(%a1)+
dbf %d0,1b
lea (.Lc1,%pc),%a0
lea (-0x10,%fp),%a1
moveq #8,%d0
.balignw 4,0x2048
2:
move.b (%a0)+,(%a1)+
dbf %d0,2b
pea (.Lc2,%pc)
pea (-0x10,%fp)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
lea (.Lc3,%pc),%a0
lea (-0x140,%fp),%a1
move.w #0x12b,%d0
.balignw 4,0x2048
3:
move.b (%a0)+,(%a1)+
dbf %d0,3b
pea (.Lc4,%pc)
pea (-0x140,%fp)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (-0x15,%fp)
jsr (EXEC_$LOG_C).l
addq.w #4,%sp
moveq #1,%d0
bra.s 5f
.balignw 4,0x2048
4:
move.w %d0,%d1
lsl.w #2,%d1
lea (0,%fp,%d1.w),%a0
moveq #0xb,%d1
muls.w %d0,%d1
move.l %d1,(-0x16c,%a0)
addq.w #1,%d0
5:
cmpi.w #4,%d0
ble.s 4b
lea (-0x160,%fp),%a0
move.l (%a0)+,(-0x158,%fp)
move.l (%a0)+,(-0x154,%fp)
move.l (-0x158,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (-0x154,%fp),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
lea (-0x168,%fp),%a0
move.l (%a0)+,(-0x150,%fp)
move.l (%a0)+,(-0x14c,%fp)
move.l (-0x150,%fp),%d0
add.l (-0x14c,%fp),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
unlk %fp
rts
.Lc0:
.short 0x7878,0x7878,0x7878,0x7878,0x7800
.Lc4:
.short 0x0006
.Lc2:
.short 0x0009
.Lc1:
.short 0x6162,0x6320,0x2020,0x2020,0x2000
.balignw 4,0x2048
.Lc3:
.short 0x2f63,0x6f6d,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020
.globl BU_$BOOLS
BU_$BOOLS:
link.w %fp,#-8
movem.l %d2-%d4,-(%sp)
move.w (8,%fp),%d0
sgt %d1
cmpi.w #5,%d0
sgt %d2
clr.w %d3
cmp.b %d2,%d1
bne.s 1f
addq.w #1,%d3
1:
cmp.b %d2,%d1
beq.s 2f
addq.w #2,%d3
2:
cmp.b %d2,%d1
ble.s 3f
addq.w #4,%d3
3:
cmp.b %d2,%d1
bge.s 4f
addq.w #8,%d3
4:
cmp.b %d2,%d1
blt.s 5f
addi.w #0x10,%d3
5:
cmp.b %d2,%d1
bgt.s 6f
addi.w #0x20,%d3
6:
cmpi.b #0xff,%d1
bne.s 7f
addi.w #0x40,%d3
7:
tst.b %d2
bne.s 8f
addi.w #0x80,%d3
8:
tst.w %d0
sgt %d1
cmpi.w #5,%d0
sgt %d4
cmp.b %d4,%d1
bne.s 9f
addi.w #0x100,%d3
9:
cmpi.w #3,%d0
slt %d0
cmp.b %d2,%d0
beq.s 10f
addi.w #0x200,%d3
10:
cmpi.b #0xff,%d2
ble.s 11f
addi.w #0x400,%d3
11:
move.w %d3,%d0
movem.l (-0x14,%fp),%d2-%d4
unlk %fp
rts
.globl BU_$MUL3
BU_$MUL3:
link.w %fp,#-8
movem.l %d2-%d4,-(%sp)
move.l (8,%fp),%d3
move.l (0xc,%fp),%d4
move.l (0x10,%fp),%d2
move.l %d4,-(%sp)
move.l %d3,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d2,%d1
addq.l #1,%d1
move.l %d1,-(%sp)
move.l %d0,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,(-4,%fp)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l %d4,-(%sp)
move.l %d3,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,%d1
move.l %d3,-(%sp)
move.l %d2,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,%d2
move.l %d2,-(%sp)
move.l %d1,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
movem.l (-0x14,%fp),%d2-%d4
unlk %fp
rts
.globl BU_$CASE_EMPTY
BU_$CASE_EMPTY:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d0
moveq #5,%d1
move.w %d0,%d2
cmpi.w #1,%d2
beq.s 2f
cmpi.w #2,%d2
beq.s 3f
bra.s 1f
2:
moveq #0xa,%d1
bra.s 1f
3:
moveq #0x14,%d1
1:
move.w %d1,%d0
move.l (-8,%fp),%d2
unlk %fp
rts
.globl BU_$WIDEN
BU_$WIDEN:
link.w %fp,#-0xc
movem.l %d2-%d4,-(%sp)
move.b (8,%fp),%d1
move.b (0xa,%fp),%d0
move.b %d1,%d2
move.b %d0,%d3
clr.l %d4
move.b %d2,%d4
move.l %d4,(-4,%fp)
move.l %d4,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.b %d3,%d4
ext.w %d4
ext.l %d4
move.l %d4,(-4,%fp)
move.l %d4,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l %d4
move.b %d2,%d4
lsr.l #2,%d4
andi.l #0xf,%d4
addi.l #0x186a0,%d4
move.l %d4,(-4,%fp)
move.l %d4,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.b %d3,%d0
move.l #0x186a0,-(%sp)
ext.w %d0
ext.l %d0
move.l %d0,-(%sp)
jsr (M$MIS$LLL).l
movem.l (-0x18,%fp),%d2-%d4
unlk %fp
rts
.globl BU_$LMOD
BU_$LMOD:
link.w %fp,#-8
movem.l %d2-%d4,-(%sp)
move.l (8,%fp),%d2
move.w (0xc,%fp),%d3
subq.l #2,%sp
move.w %d3,-(%sp)
move.l %d2,-(%sp)
jsr (M$OIS$WLW).l
addq.w #8,%sp
move.w %d0,%d4
ext.l %d4
move.l %d4,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d3,-(%sp)
move.l %d2,-(%sp)
jsr (M$OIS$WLW).l
addq.w #8,%sp
move.w %d0,%d1
ext.l %d1
tst.l %d1
beq.s 1f
addi.l #0xf4240,%d4
1:
move.l %d4,%d0
movem.l (-0x14,%fp),%d2-%d4
unlk %fp
rts
.globl BU_$FORRT
BU_$FORRT:
link.w %fp,#-0x20
movem.l %d2-%d5,-(%sp)
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
moveq #9,%d0
moveq #1,%d1
lea (2,%fp),%a0
.balignw 4,0x2048
2:
move.w %d1,%d4
move.w %d4,%d5
add.w %d5,%d5
add.w %d5,%d4
move.w %d4,(-0x22,%a0)
addq.w #1,%d1
addq.l #2,%a0
dbf %d0,2b
clr.l %d4
move.w %d3,%d0
sub.w %d2,%d0
blt.s 4f
move.w %d0,%d1
move.w %d2,%d0
.balignw 4,0x2048
5:
move.w %d0,%d5
add.w %d5,%d5
move.w (-0x22,%fp,%d5.w),%d5
addq.w #1,%d0
ext.l %d5
add.l %d5,%d4
dbf %d1,5b
4:
move.l %d4,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l %d4
move.w %d3,%d0
move.w %d0,%d1
sub.w %d2,%d1
blt.s 7f
.balignw 4,0x2048
8:
move.l %d4,%d3
move.w %d0,%d4
add.l %d3,%d3
ext.l %d4
subq.w #1,%d0
add.l %d4,%d3
move.l %d3,%d4
dbf %d1,8b
7:
move.l %d4,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l %d4
moveq #0xa,%d0
sub.w %d2,%d0
blt.s 10f
.balignw 4,0x2048
11:
addq.l #1,%d4
dbf %d0,11b
10:
move.l %d4,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l %d4
moveq #5,%d0
sub.w %d2,%d0
blt.s 13f
move.w %d0,%d3
move.w %d2,%d5
.balignw 4,0x2048
14:
jsr (EXEC_$CLOBBER).l
move.w %d5,%d0
addq.w #1,%d5
ext.l %d0
add.l %d0,%d4
dbf %d3,14b
13:
move.l %d4,%d0
movem.l (-0x30,%fp),%d2-%d5
unlk %fp
rts
.globl BU_$FORU
BU_$FORU:
link.w %fp,#-0x20
movem.l %d2-%d4,-(%sp)
move.w (8,%fp),%d0
moveq #9,%d1
clr.w %d2
lea (%fp),%a0
.balignw 4,0x2048
2:
move.w %d2,%d3
move.w %d3,%d4
neg.w %d3
lsl.w #3,%d4
add.w %d4,%d3
move.w %d3,(-0x20,%a0)
addq.w #1,%d2
addq.l #2,%a0
dbf %d1,2b
clr.l %d1
tst.w %d0
beq.s 4f
move.w %d0,%d2
subq.w #1,%d2
lea (2,%fp),%a0
move.w %d2,%d0
.balignw 4,0x2048
5:
move.w (-0x20,%a0),%d2
addq.l #2,%a0
ext.l %d2
add.l %d2,%d1
dbf %d0,5b
4:
move.l %d1,%d0
movem.l (-0x2c,%fp),%d2-%d4
unlk %fp
rts
.globl BU_$LBYTE
BU_$LBYTE:
link.w %fp,#-8
movem.l %d2-%d3,-(%sp)
subq.l #2,%sp
move.l (8,%fp),%d2
move.w #0x3e8,-(%sp)
move.l %d2,-(%sp)
jsr (M$OIS$WLW).l
addq.w #8,%sp
move.w %d0,%d3
ext.l %d3
move.b %d3,%d0
ext.w %d0
ext.l %d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
tst.l %d2
ble.s 1f
subq.l #2,%sp
move.w #0x2710,-(%sp)
move.l %d2,-(%sp)
jsr (M$DIS$LLW).l
bra.s 3f
1:
subq.l #2,%sp
move.w #0x2710,-(%sp)
move.l %d2,-(%sp)
jsr (M$DIS$LLW).l
neg.l %d0
3:
move.b %d0,%d1
2:
move.b %d3,%d0
clr.w %d2
ext.w %d0
move.b %d1,%d2
add.w %d2,%d0
movem.l (-0x10,%fp),%d2-%d3
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_U_KINDS","aw",@nobits
.balign 4
.Ldata:
.space 76
