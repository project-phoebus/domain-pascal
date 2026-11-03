.text
.globl BH_A_$MUL26
BH_A_$MUL26:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d0
move.w %d0,%d2
add.w %d2,%d2
move.w %d2,%d0
lsl.w #2,%d0
add.w %d0,%d2
add.w %d0,%d0
add.w %d0,%d2
move.w %d2,(-2,%fp)
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl BH_A_$MUL13
BH_A_$MUL13:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
move.w %d2,%d3
move.w %d3,%d0
lsl.w #2,%d0
add.w %d0,%d3
add.w %d0,%d0
add.w %d0,%d3
move.w %d3,(-2,%fp)
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w %d2,%d3
bpl.s 1f
addq.w #3,%d3
1:
asr.w #2,%d3
lsl.w #2,%d3
move.w %d3,%d0
lsl.w #2,%d0
add.w %d0,%d3
add.w %d0,%d0
add.w %d0,%d3
move.w %d3,(-2,%fp)
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl BH_A_$MULL13
BH_A_$MULL13:
link.w %fp,#-4
move.l (8,%fp),%d1
subq.l #2,%sp
move.w #0xd,-(%sp)
move.l %d1,-(%sp)
jsr (M$MIS$LLW).l
addq.w #8,%sp
move.l %d0,(-4,%fp)
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
unlk %fp
rts
.globl BH_A_$IDX84
BH_A_$IDX84:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
move.w (0xc,%fp),%d0
moveq #0x3f,%d1
and.l %d2,%d1
lsl.l #2,%d1
move.l %d1,%d3
lsl.l #2,%d3
add.l %d3,%d1
lsl.l #2,%d3
add.l %d3,%d1
move.w %d0,%d3
ext.l %d3
move.l %d3,(0x50,%a5,%d1.l)
jsr (EXEC_$CLOBBER).l
moveq #0x3f,%d0
and.l %d2,%d0
lsl.l #2,%d0
move.l %d0,%d1
lsl.l #2,%d1
add.l %d1,%d0
lsl.l #2,%d1
add.l %d1,%d0
move.l (0x50,%a5,%d0.l),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BH_A_$DIVC
BH_A_$DIVC:
link.w %fp,#-4
movem.l %d2-%d4,-(%sp)
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
move.w %d2,%d4
move.w %d3,%d0
ext.l %d4
addq.w #1,%d0
divs.w %d0,%d4
move.w %d4,(-2,%fp)
subq.l #2,%sp
move.w %d4,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w %d2,%d4
move.w %d3,%d0
ext.l %d4
addq.w #1,%d0
divs.w %d0,%d4
swap %d4
move.w %d4,(-2,%fp)
subq.l #2,%sp
move.w %d4,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0x10,%fp),%d2-%d4
unlk %fp
rts
.globl BH_A_$UDIVC
BH_A_$UDIVC:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
clr.l %d0
clr.l %d1
move.w %d2,%d0
move.w %d3,%d1
addq.l #1,%d1
move.l %d1,-(%sp)
move.l %d0,-(%sp)
jsr (M$DIS$LLL).l
addq.w #8,%sp
move.w %d0,(-2,%fp)
clr.l %d0
move.w (-2,%fp),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
clr.l %d0
clr.l %d1
move.w %d2,%d0
move.w %d3,%d1
addq.l #1,%d1
move.l %d1,-(%sp)
move.l %d0,-(%sp)
jsr (M$OIS$LLL).l
addq.w #8,%sp
move.w %d0,(-2,%fp)
clr.l %d0
move.w (-2,%fp),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl BH_A_$DIVP
BH_A_$DIVP:
link.w %fp,#-8
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d0
move.w %d0,(0x1504,%a5)
lea (0x1504,%a5),%a0
move.l %a0,(-4,%fp)
move.w %d2,%d3
movea.l (-4,%fp),%a0
ext.l %d3
divs.w (%a0),%d3
move.w %d3,(-6,%fp)
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w %d2,%d3
movea.l (-4,%fp),%a0
ext.l %d3
divs.w (%a0),%d3
swap %d3
move.w %d3,(-6,%fp)
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0x14,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BH_A_$UMAX
BH_A_$UMAX:
link.w %fp,#-4
move.w (8,%fp),%d0
moveq #0x21,%d1
cmp.w %d0,%d1
bhi.s 1f
move.w %d0,%d1
1:
move.w %d1,(-2,%fp)
clr.l %d0
move.w (-2,%fp),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
unlk %fp
rts
.globl BH_A_$UMAX2
BH_A_$UMAX2:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d2
move.w (0xa,%fp),%d0
move.w %d2,%d1
cmp.w %d0,%d1
bhi.s 1f
move.w %d0,%d1
1:
move.w %d1,(-2,%fp)
clr.l %d0
move.w (-2,%fp),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w %d2,%d0
move.w %d0,(-2,%fp)
clr.l %d0
move.w (-2,%fp),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl BH_A_$ORX
BH_A_$ORX:
link.w %fp,#-8
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.w (0xc,%fp),%d2
move.l %d0,(0x1500,%a5)
move.l (0x1500,%a5),%d3
ori.l #0x12000000,%d3
move.l %d3,(-4,%fp)
move.l %d3,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w %d2,%d3
ori.w #0x42,%d3
move.w %d3,(-6,%fp)
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w %d2,%d3
eori.w #0xff,%d3
move.w %d3,(-6,%fp)
subq.l #2,%sp
move.w %d3,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.l (0x1500,%a5),%d2
eori.l #0x12000000,%d2
move.l %d2,(-4,%fp)
move.l %d2,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x14,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BH_A_$LINT
BH_A_$LINT:
link.w %fp,#0
move.l %d2,-(%sp)
move.l (8,%fp),%d2
cmpi.l #-1,%d2
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 7f
1:
subq.l #2,%sp
clr.w -(%sp)
7:
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
cmpi.l #-0x80000000,%d2
bcs.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 8f
3:
subq.l #2,%sp
clr.w -(%sp)
8:
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
cmpi.l #-0x80000000,%d2
bcc.s 5f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 9f
5:
subq.l #2,%sp
clr.w -(%sp)
9:
jsr (EXEC_$MARK).l
6:
move.l (-4,%fp),%d2
unlk %fp
rts
.globl BH_A_$LNEG
BH_A_$LNEG:
link.w %fp,#0
move.l %d2,-(%sp)
move.l (8,%fp),%d2
cmpi.l #-1,%d2
ble.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 9f
1:
subq.l #2,%sp
clr.w -(%sp)
9:
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
cmpi.l #-5,%d2
blt.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 10f
3:
subq.l #2,%sp
clr.w -(%sp)
10:
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
cmpi.l #-1,%d2
bge.s 5f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 11f
5:
subq.l #2,%sp
clr.w -(%sp)
11:
jsr (EXEC_$MARK).l
addq.w #4,%sp
6:
moveq #-1,%d0
cmp.l %d2,%d0
bge.s 7f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 12f
7:
subq.l #2,%sp
clr.w -(%sp)
12:
jsr (EXEC_$MARK).l
8:
move.l (-4,%fp),%d2
unlk %fp
rts
.globl BH_A_$UOUT
BH_A_$UOUT:
link.w %fp,#0
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
clr.l %d0
move.w %d2,%d0
cmpi.l #-1,%d0
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 9f
1:
subq.l #2,%sp
clr.w -(%sp)
9:
jsr (EXEC_$MARK).l
addq.w #4,%sp
2:
clr.l %d0
move.w %d2,%d0
cmpi.l #0x11170,%d0
bge.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 10f
3:
subq.l #2,%sp
clr.w -(%sp)
10:
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
move.w %d3,%d0
ext.l %d0
cmpi.l #0x9c40,%d0
ble.s 5f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 11f
5:
subq.l #2,%sp
clr.w -(%sp)
11:
jsr (EXEC_$MARK).l
addq.w #4,%sp
6:
move.w %d3,%d0
ext.l %d0
cmpi.l #-0x9c40,%d0
ble.s 7f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 12f
7:
subq.l #2,%sp
clr.w -(%sp)
12:
jsr (EXEC_$MARK).l
8:
movem.l (-8,%fp),%d2-%d3
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_H4BA_ARITH","aw",@nobits
.balign 4
.Ldata:
.space 5384
