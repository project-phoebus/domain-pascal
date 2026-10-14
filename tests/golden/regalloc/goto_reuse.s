.text
.globl FW_FWD
FW_FWD:
link.w %fp,#-4
tst.b (B).l
bmi.s 1f
move.l #LOCK,-(%sp)
jsr (ML_$SPIN_LOCK).l
addq.w #4,%sp
move.w %d0,(-2,%fp)
move.w #1,(COUNTER).l
1:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
move.l #LOCK,-(%sp)
jsr (ML_$SPIN_UNLOCK).l
unlk %fp
rts
.globl FW_BACK
FW_BACK:
link.w %fp,#-4
move.l #LOCK,-(%sp)
jsr (ML_$SPIN_LOCK).l
addq.w #4,%sp
move.w %d0,(-2,%fp)
.balignw 4,0x2048
1:
subq.l #2,%sp
move.w (-2,%fp),-(%sp)
move.l #LOCK,-(%sp)
jsr (ML_$SPIN_UNLOCK).l
addq.w #8,%sp
tst.b (B).l
bpl.s 2f
jsr (TICK).l
bra.s 1b
2:
move.w #2,(COUNTER).l
unlk %fp
rts
.globl FW_AFTER
FW_AFTER:
link.w %fp,#-4
move.l #LOCK,-(%sp)
jsr (ML_$SPIN_LOCK).l
addq.w #4,%sp
move.w %d0,(-2,%fp)
tst.b (B).l
bmi.s 1f
move.w #1,(COUNTER).l
1:
subq.l #2,%sp
move.w %d0,-(%sp)
move.l #LOCK,-(%sp)
jsr (ML_$SPIN_UNLOCK).l
unlk %fp
rts
.globl CSE_BACK
CSE_BACK:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (%a5),%d0
lsl.w #2,%d0
move.l (4,%a5,%d0.w),(0x2c,%a5)
.balignw 4,0x2048
1:
move.w (%a5),%d0
lsl.w #2,%d0
move.l (4,%a5,%d0.w),(0x30,%a5)
addq.w #1,(%a5)
cmpi.w #5,(%a5)
blt.s 1b
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CSE_FWD
CSE_FWD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
tst.b (B).l
bmi.s 1f
move.w (%a5),%d0
lsl.w #2,%d0
move.l (4,%a5,%d0.w),(0x2c,%a5)
1:
move.w (%a5),%d0
lsl.w #2,%d0
move.l (4,%a5,%d0.w),(0x30,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl CSE_AFTER
CSE_AFTER:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (%a5),%d0
lsl.w #2,%d0
move.l (4,%a5,%d0.w),(0x2c,%a5)
tst.b (B).l
bmi.s 1f
move.w #3,(COUNTER).l
1:
move.l (4,%a5,%d0.w),(0x30,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl RL_SKIP
RL_SKIP:
link.w %fp,#-4
move.l %d2,-(%sp)
tst.b (B).l
bmi.s 1f
move.w (COUNTER).l,%d2
jsr (TICK).l
1:
move.w %d2,%d0
addq.w #1,%d0
move.w %d0,(COUNTER).l
jsr (TICK).l
move.w %d2,(COUNTER).l
move.l (-8,%fp),%d2
unlk %fp
rts
.globl RL_LOOP
RL_LOOP:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (COUNTER).l,%d2
.balignw 4,0x2048
1:
jsr (TICK).l
move.w %d2,(COUNTER).l
tst.b (B).l
bmi.s 1b
move.l (-8,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.GOTO_REUSE","aw",@nobits
.balign 4
.Ldata:
.space 52
