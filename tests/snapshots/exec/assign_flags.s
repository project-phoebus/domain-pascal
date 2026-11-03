.text
.globl ASSIGN_FLAGS_$WEQ
ASSIGN_FLAGS_$WEQ:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),%d2
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
move.w %d2,(%a5)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl ASSIGN_FLAGS_$WNEG
ASSIGN_FLAGS_$WNEG:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),%d2
bpl.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
move.w %d2,(%a5)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl ASSIGN_FLAGS_$WPOS
ASSIGN_FLAGS_$WPOS:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),%d2
ble.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
move.w %d2,(%a5)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl ASSIGN_FLAGS_$LEQ
ASSIGN_FLAGS_$LEQ:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0),%d2
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
move.l %d2,(4,%a5)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl ASSIGN_FLAGS_$LNEG
ASSIGN_FLAGS_$LNEG:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0),%d2
bpl.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
move.l %d2,(4,%a5)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl ASSIGN_FLAGS_$BOR
ASSIGN_FLAGS_$BOR:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b (8,%fp),%d0
move.b (0xa,%fp),%d2
move.b %d0,%d1
or.b %d2,%d1
move.b %d1,%d3
bpl.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
move.b %d3,(8,%a5)
move.b %d2,%d3
move.b %d3,(8,%a5)
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl ASSIGN_FLAGS_$CRES
ASSIGN_FLAGS_$CRES:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_W).l
addq.w #4,%sp
move.w %d0,%d2
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
move.w %d2,(%a5)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl ASSIGN_FLAGS_$CRESL
ASSIGN_FLAGS_$CRESL:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),-(%sp)
jsr (EXEC_$ID_L).l
addq.w #4,%sp
move.l %d0,%d2
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
move.l %d2,(4,%a5)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl ASSIGN_FLAGS_$SHOW
ASSIGN_FLAGS_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ASSIGN_FLAGS","aw",@nobits
.balign 4
.Ldata:
.space 12
