.text
.globl BX_ARITH_$CMINUS
BX_ARITH_$CMINUS:
link.w %fp,#0
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
moveq #0x64,%d2
move.w %d0,%d3
muls.w %d1,%d3
sub.w %d3,%d2
move.w %d2,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0xc,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl BX_ARITH_$VMINUS
BX_ARITH_$VMINUS:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w #0x3e8,(2,%a5)
move.w %d0,%d2
move.w (2,%a5),%d0
muls.w %d1,%d2
sub.w %d2,%d0
move.w %d0,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BX_ARITH_$UMOD
BX_ARITH_$UMOD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
clr.l %d1
move.w %d0,%d1
divu.w #0x10,%d1
swap %d1
move.w %d1,(2,%a5)
subq.l #2,%sp
move.w (2,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_ARITH_$LUMOD
BX_ARITH_$LUMOD:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
subq.l #2,%sp
move.l %d0,%d1
move.w #0x40,-(%sp)
move.l %d1,-(%sp)
jsr (M$OIU$WLW).l
addq.w #8,%sp
clr.l %d1
move.w %d0,%d1
move.l %d1,(4,%a5)
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BX_ARITH_$ADD9
BX_ARITH_$ADD9:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
moveq #0x14,%d0
add.w %d2,%d0
move.w %d0,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
moveq #0x1e,%d0
add.w %d2,%d0
move.w %d0,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BX_ARITH_$BOTH
BX_ARITH_$BOTH:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.l (0xc,%fp),%d1
move.l %d0,%d2
move.l %d1,%d0
addq.l #1,%d2
addq.l #2,%d0
move.l %d0,-(%sp)
move.l %d2,-(%sp)
jsr (M$MIS$LLL).l
addq.w #8,%sp
move.l %d0,(4,%a5)
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BX_ARITH_$WMODL
BX_ARITH_$WMODL:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.l (0xa,%fp),%d1
move.w %d0,%d2
move.l %d1,-(%sp)
ext.l %d2
move.l %d2,-(%sp)
jsr (M$OIS$LLL).l
addq.w #8,%sp
move.l %d0,(4,%a5)
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BX_ARITH_$UNPOP
BX_ARITH_$UNPOP:
link.w %fp,#-4
pea (8,%fp)
jsr (EXEC_$ID_L).l
move.l %d0,%d1
cmpi.l #3,%d1
bge.s 1f
moveq #3,%d1
1:
move.l %d1,%d0
unlk %fp
rts
.globl BX_ARITH_$SETL2
BX_ARITH_$SETL2:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
moveq #7,%d0
move.l %d0,(8,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_ARITH_$CSTORE2
BX_ARITH_$CSTORE2:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
moveq #5,%d0
move.l %d0,(0xc,%a5)
move.l (8,%a5),%d0
addq.l #5,%d0
move.l %d0,(4,%a5)
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (0xc,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_ARITH_$NEG
BX_ARITH_$NEG:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d0
move.w %d0,%d2
jsr (EXEC_$CLOBBER).l
addi.w #-5,%d2
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (EXEC_$LOGV_W).l
move.l (-8,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_X_ARITH","aw",@nobits
.balign 4
.Ldata:
.space 24
