.text
.globl BUMP
BUMP:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
addq.w #1,(%a5)
move.w (%a5),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
WID:
link.w %fp,#-8
move.w (8,%fp),%d0
unlk %fp
rts
CID:
link.w %fp,#-8
move.b (8,%fp),%d0
unlk %fp
rts
THREE:
link.w %fp,#-4
move.l %d2,-(%sp)
subq.l #2,%sp
move.b (0xa,%fp),%d2
move.w (8,%fp),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
clr.w %d0
move.b %d2,%d0
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (0xc,%fp),-(%sp)
jsr (EXEC_$MARK).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl TWO_CALLS_$ADD
TWO_CALLS_$ADD:
link.w %fp,#-4
move.l %d2,-(%sp)
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.l %d0,%d2
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.l %d0,%d1
add.w %d1,%d2
move.w %d2,(-2,%fp)
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl TWO_CALLS_$SUB
TWO_CALLS_$SUB:
link.w %fp,#-4
move.l %d2,-(%sp)
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d0,%d2
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d0,%d1
sub.l %d1,%d2
move.l %d2,(-4,%fp)
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl TWO_CALLS_$LT
TWO_CALLS_$LT:
link.w %fp,#-4
move.l %d2,-(%sp)
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.l %d0,%d2
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.l %d0,%d1
cmp.w %d1,%d2
bge.s 1f
move.w #1,(-2,%fp)
bra.s 2f
1:
clr.w (-2,%fp)
2:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl TWO_CALLS_$LT_VALUE
TWO_CALLS_$LT_VALUE:
link.w %fp,#-4
move.l %d2,-(%sp)
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d0,%d2
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d0,%d1
cmp.l %d1,%d2
slt %d1
move.b %d1,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$LOG_BOOL).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl TWO_CALLS_$NESTED
TWO_CALLS_$NESTED:
link.w %fp,#-4
subq.l #2,%sp
subq.l #2,%sp
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d1
addq.w #1,%d1
move.w %d1,-(%sp)
bsr.w WID
addq.w #4,%sp
move.w %d0,(-2,%fp)
pea (-2,%fp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d1
addq.w #1,%d1
move.w %d1,-(%sp)
jsr (EXEC_$LOGV_W).l
unlk %fp
rts
.globl TWO_CALLS_$BY_ADDRESS
TWO_CALLS_$BY_ADDRESS:
link.w %fp,#-4
move.l %d2,-(%sp)
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d0,%d2
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.l %d0,%d1
ext.l %d1
add.l %d1,%d2
move.l %d2,(-4,%fp)
pea (-4,%fp)
jsr (EXEC_$LOG_L).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl TWO_CALLS_$MIXED
TWO_CALLS_$MIXED:
link.w %fp,#0
subq.l #2,%sp
movea.l (0x10,%fp),%a0
subq.l #2,%sp
move.w (%a0),-(%sp)
bsr.w WID
addq.w #4,%sp
move.w %d0,-(%sp)
movea.l (0xc,%fp),%a1
subq.l #2,%sp
move.b (1,%a1),-(%sp)
bsr.w CID
addq.w #4,%sp
move.b %d0,-(%sp)
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,-(%sp)
bsr.w THREE
unlk %fp
rts
.globl TWO_CALLS_$INDEX
TWO_CALLS_$INDEX:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.l %d0,%d2
subq.l #2,%sp
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
bsr.w WID
addq.w #4,%sp
move.l %d0,%d1
sub.w %d1,%d2
move.w %d2,(6,%a5)
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl TWO_CALLS_$PRODUCTS
TWO_CALLS_$PRODUCTS:
link.w %fp,#-4
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0),(0x14,%a5)
movea.l (0xc,%fp),%a1
move.l (%a1),(0x18,%a5)
movea.l (0x10,%fp),%a2
move.l (%a2),(0x1c,%a5)
movea.l (0x14,%fp),%a3
move.l (%a3),(0x20,%a5)
move.l (0x18,%a5),-(%sp)
move.l (0x14,%a5),-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,%d2
move.l (0x20,%a5),-(%sp)
move.l (0x1c,%a5),-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,%d1
add.l %d1,%d2
move.l %d2,(-4,%fp)
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x14,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
.globl TWO_CALLS_$CALL_PRODUCT
TWO_CALLS_$CALL_PRODUCT:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
move.l (%a0),(0x18,%a5)
movea.l (0x10,%fp),%a1
move.l (%a1),(0x1c,%a5)
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d0,%d2
move.l (0x1c,%a5),-(%sp)
move.l (0x18,%a5),-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,%d1
add.l %d1,%d2
move.l %d2,(-4,%fp)
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl TWO_CALLS_$PRODUCT_CALL
TWO_CALLS_$PRODUCT_CALL:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
move.l (%a0),(0x18,%a5)
movea.l (0x10,%fp),%a1
move.l (%a1),(0x1c,%a5)
move.l (0x1c,%a5),-(%sp)
move.l (0x18,%a5),-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,%d2
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d0,%d1
add.l %d1,%d2
move.l %d2,(-4,%fp)
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl TWO_CALLS_$LOADAV
TWO_CALLS_$LOADAV:
link.w %fp,#-4
movea.l (0xc,%fp),%a0
move.w (%a0),%d1
movea.l (8,%fp),%a0
muls.w #0x16b,%d1
move.l (%a0),%d0
bpl.s 1f
addi.l #0xff,%d0
1:
asr.l #8,%d0
move.l #0xfe95,-(%sp)
move.l %d0,-(%sp)
jsr (M$MIS$LLL).l
tst.l %d0
bpl.s 2f
addi.l #0xff,%d0
2:
asr.l #8,%d0
add.l %d0,%d1
move.l %d1,%d0
unlk %fp
rts
.globl TWO_CALLS_$OR_ELSE
TWO_CALLS_$OR_ELSE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.w (%a5)
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
cmpi.w #1,%d0
beq.s 2f
bsr.w BUMP
cmpi.w #1,%d0
bne.s 1f
2:
move.w #1,(-2,%fp)
bra.s 3f
1:
clr.w (-2,%fp)
3:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl TWO_CALLS_$AND_THEN
TWO_CALLS_$AND_THEN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.w (%a5)
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
cmpi.w #1,%d0
bne.s 1f
bsr.w BUMP
cmpi.w #7,%d0
bne.s 1f
move.w #1,(-2,%fp)
bra.s 2f
1:
clr.w (-2,%fp)
2:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl TWO_CALLS_$BUMPS
TWO_CALLS_$BUMPS:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w #1,(%a5)
bsr.w BUMP
move.l %d0,%d2
bsr.w BUMP
move.l %d0,%d1
add.w %d1,%d2
move.w %d2,(-2,%fp)
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$MARK).l
addq.w #4,%sp
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl TWO_CALLS_$BOOLS
TWO_CALLS_$BOOLS:
link.w %fp,#-4
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_BOOL).l
addq.w #4,%sp
tst.b %d0
bpl.s 1f
move.l (0xc,%fp),-(%sp)
jsr (EXEC_$ID_BOOL).l
addq.w #4,%sp
tst.b %d0
bpl.s 1f
move.w #1,(-2,%fp)
bra.s 2f
1:
clr.w (-2,%fp)
2:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
jsr (EXEC_$MARK).l
unlk %fp
rts
.globl TWO_CALLS_$RESULT
TWO_CALLS_$RESULT:
link.w %fp,#-4
move.l %d2,-(%sp)
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.l %d0,%d2
subq.l #2,%sp
movea.l (0xc,%fp),%a0
move.w (%a0),-(%sp)
bsr.w WID
addq.w #4,%sp
move.l %d0,%d1
add.w %d1,%d2
move.w %d2,%d0
move.l (-8,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.TWO_CALLS","aw",@progbits
.balign 4
.Ldata:
.space 36
