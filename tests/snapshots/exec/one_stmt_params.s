.text
.globl ONE_STMT_PARAMS_$ENABLE
ONE_STMT_PARAMS_$ENABLE:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b (0xc,%fp),%d2
move.b (0xe,%fp),%d0
bpl.s 1f
move.b %d2,(%a5)
clr.w %d0
move.b %d2,%d0
add.w %d0,%d0
move.w #0xff,(2,%a5,%d0.w)
bra.s 3f
1:
clr.w %d0
move.b %d2,%d0
add.w %d0,%d0
clr.w (2,%a5,%d0.w)
clr.b (%a5)
3:
subq.l #2,%sp
move.b (%a5),-(%sp)
jsr (EXEC_$LOGV_C).l
addq.w #4,%sp
subq.l #2,%sp
clr.w %d0
move.b %d2,%d0
add.w %d0,%d0
move.w (2,%a5,%d0.w),-(%sp)
jsr (EXEC_$LOGV_W).l
2:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl ONE_STMT_PARAMS_$PICK
ONE_STMT_PARAMS_$PICK:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d2
cmpi.w #5,%d0
bge.s 1f
move.w %d0,(0x202,%a5)
subq.l #2,%sp
move.w #1,-(%sp)
jsr (EXEC_$MARK).l
move.w (0x202,%a5),%d0
add.w %d2,%d0
bra.w 1f
1:
move.w %d0,(0x202,%a5)
2:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl ONE_STMT_PARAMS_$SHOW
ONE_STMT_PARAMS_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (0x202,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ONE_STMT_PARAMS_$XLATE
ONE_STMT_PARAMS_$XLATE:
link.w %fp,#-4
move.b (8,%fp),%d0
clr.w %d1
move.b %d0,%d1
subi.w #0x91,%d1
cmpi.w #9,%d1
bcc.s 4f
add.w %d1,%d1
move.w (5f,%pc,%d1.w),%d1
jmp (5f,%pc,%d1.w)
5:
.short 2f-5b
.short 4f-5b
.short 4f-5b
.short 4f-5b
.short 4f-5b
.short 4f-5b
.short 3f-5b
.short 3f-5b
.short 3f-5b
2:
moveq #0x18,%d0
bra.s 1f
3:
moveq #9,%d0
bra.s 1f
4:
clr.b %d0
1:
unlk %fp
rts
.globl ONE_STMT_PARAMS_$COUNT
ONE_STMT_PARAMS_$COUNT:
link.w %fp,#-4
movem.l %d2-%d4,-(%sp)
move.w (8,%fp),%d2
move.w (0xa,%fp),%d0
move.w %d0,%d1
subq.w #1,%d1
bmi.s 1f
move.w %d1,%d3
moveq #1,%d4
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d4,%d0
muls.w %d2,%d0
move.w %d0,-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
addq.w #1,%d4
dbf %d3,2b
1:
movem.l (-0x10,%fp),%d2-%d4
unlk %fp
rts
.globl ONE_STMT_PARAMS_$GROW
ONE_STMT_PARAMS_$GROW:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
bra.s 2f
.balignw 4,0x2048
1:
move.w (0x202,%a5),%d0
add.w %d2,%d0
move.w %d0,(0x202,%a5)
subq.l #2,%sp
move.w (0x202,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
2:
cmpi.w #0x14,(0x202,%a5)
blt.s 1b
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl ONE_STMT_PARAMS_$UPTO
ONE_STMT_PARAMS_$UPTO:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
.balignw 4,0x2048
1:
move.w (0x202,%a5),%d0
sub.w %d2,%d0
move.w %d0,(0x202,%a5)
subq.l #2,%sp
move.w (0x202,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
tst.w (0x202,%a5)
bpl.s 1b
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ONE_STMT_PARAMS","aw",@nobits
.balign 4
.Ldata:
.space 516
