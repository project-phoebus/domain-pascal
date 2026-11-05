.text
.globl ABS_$LT_SIGNED
ABS_$LT_SIGNED:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d1
move.w (0xa,%fp),%d0
move.w %d1,%d2
bpl.s 2f
neg.w %d2
2:
cmp.w %d0,%d2
bge.s 1f
moveq #1,%d0
bra.s 3f
1:
clr.w %d0
3:
move.l (-8,%fp),%d2
unlk %fp
rts
.globl ABS_$GT_CONST
ABS_$GT_CONST:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,%d1
bpl.s 2f
neg.w %d1
2:
cmpi.w #5,%d1
bls.s 1f
moveq #1,%d0
bra.s 3f
1:
clr.w %d0
3:
unlk %fp
rts
.globl ABS_$SUM
ABS_$SUM:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d0
bpl.s 1f
neg.w %d0
1:
clr.l %d1
move.w %d0,%d1
move.w (0xa,%fp),%d0
bpl.s 2f
neg.w %d0
2:
clr.l %d2
move.w %d0,%d2
add.l %d2,%d1
move.l %d1,%d0
move.l (-8,%fp),%d2
unlk %fp
rts
.globl ABS_$PLUS10
ABS_$PLUS10:
link.w %fp,#-4
move.w (8,%fp),%d0
bpl.s 1f
neg.w %d0
1:
clr.l %d1
move.w %d0,%d1
addi.l #0xa,%d1
move.l %d1,%d0
unlk %fp
rts
.globl ABS_$PRODUCTS
ABS_$PRODUCTS:
link.w %fp,#-4
move.w (8,%fp),%d0
muls.w (0xa,%fp),%d0
bpl.s 1f
neg.w %d0
1:
move.w (0xc,%fp),%d1
muls.w (0xe,%fp),%d1
bpl.s 2f
neg.w %d1
2:
add.w %d1,%d0
unlk %fp
rts
.globl ABS_$QUOT
ABS_$QUOT:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d1
move.w (0xa,%fp),%d0
move.w %d1,%d2
ext.l %d2
divs.w %d0,%d2
tst.w %d2
bpl.s 2f
neg.w %d2
2:
cmpi.w #5,%d2
bls.s 1f
moveq #6,%d0
bra.s 3f
1:
moveq #5,%d0
3:
move.l (-8,%fp),%d2
unlk %fp
rts
.globl ABS_$L_CONST
ABS_$L_CONST:
link.w %fp,#-4
move.l (8,%fp),%d0
move.l %d0,%d1
bpl.s 2f
neg.l %d1
2:
cmpi.l #0x1f40,%d1
bls.s 1f
moveq #1,%d0
bra.s 3f
1:
clr.w %d0
3:
unlk %fp
rts
.globl ABS_$L_DIFF
ABS_$L_DIFF:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.l (8,%fp),%d2
move.l (0xc,%fp),%d1
move.l (0x10,%fp),%d0
move.l %d2,%d3
sub.l %d1,%d3
bpl.s 2f
neg.l %d3
2:
cmp.l %d0,%d3
bls.s 1f
moveq #1,%d0
bra.s 3f
1:
clr.w %d0
3:
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl ABS_$ARG
ABS_$ARG:
link.w %fp,#0
subq.l #2,%sp
move.w (8,%fp),%d0
bpl.s 1f
neg.w %d0
1:
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
unlk %fp
rts
.globl ABS_$UNSIGNED
ABS_$UNSIGNED:
link.w %fp,#-4
clr.l %d0
move.w (8,%fp),%d0
unlk %fp
rts
.globl ABS_$BYTE
ABS_$BYTE:
link.w %fp,#-4
move.b (8,%fp),%d0
ext.w %d0
bpl.s 1f
neg.w %d0
1:
unlk %fp
rts
.globl ABS_$STORE
ABS_$STORE:
link.w %fp,#-4
move.w (8,%fp),%d0
bpl.s 1f
neg.w %d0
1:
unlk %fp
rts
.globl ABS_$L_STORE
ABS_$L_STORE:
link.w %fp,#-4
move.l (8,%fp),%d0
bpl.s 1f
neg.l %d0
1:
unlk %fp
rts
.globl ABS_$GLOBAL
ABS_$GLOBAL:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
bpl.s 1f
neg.w %d1
1:
move.w %d1,(%a5)
subq.l #2,%sp
move.w %d1,-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ABS_$C_MINUS
ABS_$C_MINUS:
link.w %fp,#-4
move.w (8,%fp),%d0
sub.w (0xa,%fp),%d0
bpl.s 1f
neg.w %d0
1:
moveq #-1,%d1
sub.w %d0,%d1
move.w %d1,%d0
unlk %fp
rts
.globl ABS_$W_DIFF
ABS_$W_DIFF:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
move.w (0xa,%fp),%d1
move.l (0xc,%fp),%d0
move.w %d2,%d3
move.w %d1,%d2
ext.l %d3
ext.l %d2
sub.l %d2,%d3
bpl.s 2f
neg.l %d3
2:
cmp.l %d0,%d3
ble.s 1f
moveq #1,%d0
bra.s 3f
1:
clr.w %d0
3:
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl ABS_$QUOT_L
ABS_$QUOT_L:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
move.w (0xa,%fp),%d1
move.l (0xc,%fp),%d0
move.w %d2,%d3
ext.l %d3
divs.w %d1,%d3
tst.w %d3
bpl.s 2f
neg.w %d3
2:
clr.l %d1
move.w %d3,%d1
cmp.l %d0,%d1
ble.s 1f
moveq #1,%d0
bra.s 3f
1:
clr.w %d0
3:
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ABS","aw",@nobits
.balign 4
.Ldata:
.space 4
