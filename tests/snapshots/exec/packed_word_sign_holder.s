.text
.globl VIA_PTR
VIA_PTR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (6,%a0),(0x14,%a5)
subq.l #2,%sp
move.w (0x14,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl VIA_VAR
VIA_VAR:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
move.w (6,%a2),(0x14,%a5)
subq.l #2,%sp
move.w (0x14,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
lea (%a2),%a0
move.l (%a0)+,(%a5)
move.l (%a0)+,(4,%a5)
pea (3,%a5)
jsr (EXEC_$LOG_C).l
addq.w #4,%sp
tst.w (4,%a2)
smi %d0
move.b %d0,(0x16,%a5)
pea (0x16,%a5)
jsr (EXEC_$LOG_BOOL).l
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl TWO_BYTE
TWO_BYTE:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
tst.b (%a2)
smi %d0
move.b %d0,(0x16,%a5)
pea (0x16,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
clr.w (0x14,%a5)
tst.b (%a2)
bpl.s 1f
move.w #1,(0x14,%a5)
1:
subq.l #2,%sp
move.w (0x14,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl PACKED_WORD_SIGN_HOLDER_$RUN
PACKED_WORD_SIGN_HOLDER_$RUN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.b #0x78,(8,%a5)
move.b #0x79,(0xb,%a5)
move.w #0x4d2,(0xe,%a5)
andi.b #0x80,(9,%a5)
ori.b #3,(9,%a5)
andi.b #0x80,(0xc,%a5)
ori.b #3,(0xc,%a5)
st (0x16,%a5)
andi.b #0x7f,(0xc,%a5)
move.b (0x16,%a5),%d0
andi.b #0x80,%d0
or.b %d0,(0xc,%a5)
pea (8,%a5)
bsr.w VIA_PTR
addq.w #4,%sp
pea (8,%a5)
bsr.w VIA_VAR
addq.w #4,%sp
move.b #0x70,(0x10,%a5)
move.b #0x71,(0x13,%a5)
andi.b #0x80,(0x11,%a5)
ori.b #7,(0x11,%a5)
andi.b #0x7f,(0x11,%a5)
move.b (0x16,%a5),%d0
andi.b #0x80,%d0
or.b %d0,(0x11,%a5)
pea (0x11,%a5)
bsr.w TWO_BYTE
addq.w #4,%sp
tst.b (0x11,%a5)
smi %d0
move.b %d0,(0x16,%a5)
pea (0x16,%a5)
jsr (EXEC_$LOG_BOOL).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PACKED_WORD_SIGN_HOLDER","aw",@nobits
.balign 4
.Ldata:
.space 24
