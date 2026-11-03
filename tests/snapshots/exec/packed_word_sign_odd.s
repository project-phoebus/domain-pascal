.text
.globl VIA_VAR
VIA_VAR:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
tst.b (%a2)
smi %d0
move.b %d0,(4,%a5)
pea (4,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
clr.w (6,%a5)
tst.b (%a2)
bpl.s 1f
move.w #1,(6,%a5)
1:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl VIA_PTR
VIA_PTR:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
tst.b (%a2)
smi %d0
move.b %d0,(4,%a5)
pea (4,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
clr.w (6,%a5)
tst.b (%a2)
bpl.s 1f
move.w #1,(6,%a5)
1:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl VIA_WITH
VIA_WITH:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
tst.b (%a2)
smi %d0
move.b %d0,(4,%a5)
pea (4,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
clr.w (6,%a5)
tst.b (%a2)
bpl.s 1f
move.w #1,(6,%a5)
1:
subq.l #2,%sp
move.w (6,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl PACKED_WORD_SIGN_ODD_$RUN
PACKED_WORD_SIGN_ODD_$RUN:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.b #0x78,(%a5)
move.b #0x79,(3,%a5)
andi.b #0x80,(1,%a5)
ori.b #5,(1,%a5)
st (4,%a5)
andi.b #0x7f,(1,%a5)
move.b (4,%a5),%d0
andi.b #0x80,%d0
or.b %d0,(1,%a5)
pea (1,%a5)
bsr.w VIA_VAR
addq.w #4,%sp
lea (1,%a5),%a2
move.l %a2,(-4,%fp)
pea (%a2)
bsr.w VIA_PTR
addq.w #4,%sp
move.l (-4,%fp),-(%sp)
bsr.w VIA_WITH
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PACKED_WORD_SIGN_ODD","aw",@nobits
.balign 4
.Ldata:
.space 8
