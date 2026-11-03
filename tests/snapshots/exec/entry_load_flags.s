.text
.globl ENTRY_LOAD_FLAGS_$WEQ
ENTRY_LOAD_FLAGS_$WEQ:
link.w %fp,#0
move.w (8,%fp),%d0
movea.l (0xa,%fp),%a0
bne.s 1f
moveq #1,%d0
bra.s 3f
1:
moveq #2,%d0
3:
move.l %d0,(%a0)
2:
unlk %fp
rts
.globl ENTRY_LOAD_FLAGS_$WNEG
ENTRY_LOAD_FLAGS_$WNEG:
link.w %fp,#0
move.w (8,%fp),%d0
bpl.s 1f
subq.l #2,%sp
move.w #-1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
unlk %fp
rts
.globl ENTRY_LOAD_FLAGS_$WGE
ENTRY_LOAD_FLAGS_$WGE:
link.w %fp,#0
move.w (8,%fp),%d0
bmi.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 3f
1:
subq.l #2,%sp
clr.w -(%sp)
3:
jsr (EXEC_$LOGV_W).l
2:
unlk %fp
rts
.globl ENTRY_LOAD_FLAGS_$LEQ
ENTRY_LOAD_FLAGS_$LEQ:
link.w %fp,#0
move.l (8,%fp),%d0
bne.s 1f
pea (0xa).w
bra.s 3f
1:
pea (0x14).w
3:
jsr (EXEC_$LOGV_L).l
2:
unlk %fp
rts
.globl ENTRY_LOAD_FLAGS_$LNEG
ENTRY_LOAD_FLAGS_$LNEG:
link.w %fp,#0
move.l (8,%fp),%d0
bpl.s 1f
pea (-1).w
bra.s 3f
1:
clr.l -(%sp)
3:
jsr (EXEC_$LOGV_L).l
2:
unlk %fp
rts
.globl ENTRY_LOAD_FLAGS_$CEQ
ENTRY_LOAD_FLAGS_$CEQ:
link.w %fp,#0
move.b (8,%fp),%d0
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
unlk %fp
rts
.globl ENTRY_LOAD_FLAGS_$BOOLP
ENTRY_LOAD_FLAGS_$BOOLP:
link.w %fp,#0
move.b (8,%fp),%d0
movea.l (0xa,%fp),%a0
bpl.s 1f
moveq #3,%d0
bra.s 3f
1:
moveq #4,%d0
3:
move.l %d0,(%a0)
2:
unlk %fp
rts
.globl ENTRY_LOAD_FLAGS_$FIRST
ENTRY_LOAD_FLAGS_$FIRST:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
tst.w %d0
bne.s 1f
move.w %d1,%d0
addq.w #5,%d0
move.w %d0,(%a5)
bra.s 2f
1:
move.w %d1,(%a5)
2:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ENTRY_LOAD_FLAGS_$SHOW
ENTRY_LOAD_FLAGS_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ENTRY_LOAD_FLAGS_$MULTI
ENTRY_LOAD_FLAGS_$MULTI:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
bpl.s 1f
move.w %d2,(%a5)
bra.s 2f
1:
clr.w (%a5)
2:
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
move.w %d2,(%a5)
subq.l #2,%sp
move.w (%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ENTRY_LOAD_FLAGS","aw",@nobits
.balign 4
.Ldata:
.space 4
