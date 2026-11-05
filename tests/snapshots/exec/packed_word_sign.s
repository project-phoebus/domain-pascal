.text
.globl PRESENT_OF
PRESENT_OF:
link.w %fp,#-4
movea.l (8,%fp),%a0
tst.w (%a0)
smi %d0
unlk %fp
rts
.globl PACKED_WORD_SIGN_$RUN
PACKED_WORD_SIGN_$RUN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(EXT_MCB).l
move.w %d0,(%a5)
move.w %d0,(4,%a5)
lea (%a5),%a0
move.l %a0,(8,%a5)
tst.w (EXT_MCB).l
smi %d0
move.b %d0,(0xc,%a5)
pea (0xc,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
clr.w (0xe,%a5)
tst.w (EXT_MCB).l
bpl.s 1f
move.w #1,(0xe,%a5)
1:
tst.w (%a5)
bmi.s 2f
addq.w #2,(0xe,%a5)
2:
subq.l #2,%sp
move.w (0xe,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
tst.w (%a5)
smi %d0
move.b %d0,(0xc,%a5)
pea (0xc,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.l (8,%a5),-(%sp)
bsr.w PRESENT_OF
addq.w #4,%sp
move.b %d0,(0xc,%a5)
pea (0xc,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
clr.w (0xe,%a5)
movea.l (8,%a5),%a0
tst.w (%a0)
bpl.s 3f
move.w #1,(0xe,%a5)
bra.s 4f
3:
tst.b (5,%a5)
bmi.s 5f
move.w #2,(0xe,%a5)
5:
4:
subq.l #2,%sp
move.w (0xe,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
tst.b (5,%a5)
smi %d0
move.b %d0,(0xc,%a5)
pea (0xc,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
move.w (EXT_MCB).l,%d0
btst #14,%d0
sne %d0
move.b %d0,(0xc,%a5)
pea (0xc,%a5)
jsr (EXEC_$LOG_BOOL).l
addq.w #4,%sp
clr.w (0xe,%a5)
move.w (EXT_MCB).l,%d0
btst #14,%d0
bne.s 6f
move.w #1,(0xe,%a5)
6:
tst.w %d0
smi %d0
move.w (EXT_MCB).l,%d1
btst #14,%d1
sne %d1
or.b %d1,%d0
bpl.s 7f
addq.w #2,(0xe,%a5)
7:
subq.l #2,%sp
move.w (0xe,%a5),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PACKED_WORD_SIGN_$STORE
PACKED_WORD_SIGN_$STORE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l #-0x7f00edcc,(EXT_MCB).l
clr.b (0xc,%a5)
andi.b #0x7f,(EXT_MCB).l
move.b (0xc,%a5),%d0
andi.b #0x80,%d0
or.b %d0,(EXT_MCB).l
move.l #EXT_MCB,-(%sp)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
st (0xc,%a5)
andi.b #0x7f,(EXT_MCB).l
move.b (0xc,%a5),%d0
andi.b #0x80,%d0
or.b %d0,(EXT_MCB).l
move.l #EXT_MCB,-(%sp)
jsr (EXEC_$LOG_X).l
addq.w #4,%sp
tst.w (EXT_MCB).l
smi %d0
move.b %d0,(0xc,%a5)
pea (0xc,%a5)
jsr (EXEC_$LOG_BOOL).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.PACKED_WORD_SIGN","aw",@nobits
.balign 4
.Ldata:
.space 16
