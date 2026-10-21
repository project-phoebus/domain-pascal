.text
.globl NOT_PRESENT
NOT_PRESENT:
link.w %fp,#0
tst.w (PBU_$MCB).l
bmi.s 1f
jsr (EXT_$FAIL).l
1:
unlk %fp
rts
.globl PRESENT
PRESENT:
link.w %fp,#-4
tst.w (PBU_$MCB).l
smi %d0
unlk %fp
rts
.globl NOT_WIRED
NOT_WIRED:
link.w %fp,#0
move.w (PBU_$MCB).l,%d0
btst #14,%d0
bne.s 1f
jsr (EXT_$FAIL).l
1:
unlk %fp
rts
.globl EITHER
EITHER:
link.w %fp,#0
tst.w (PBU_$MCB).l
smi %d0
move.w (PBU_$MCB).l,%d1
btst #12,%d1
sne %d1
or.b %d1,%d0
bpl.s 1f
jsr (EXT_$FAIL).l
1:
unlk %fp
rts
.globl SET_PRESENT
SET_PRESENT:
link.w %fp,#0
andi.b #0x7f,(PBU_$MCB).l
move.b (MBUS_$PRESENT).l,%d0
andi.b #0x80,%d0
or.b %d0,(PBU_$MCB).l
unlk %fp
rts
.globl QUEUED
QUEUED:
link.w %fp,#0
movea.l (8,%fp),%a0
tst.w (2,%a0)
bpl.s 1f
jsr (EXT_$FAIL).l
1:
unlk %fp
rts
.globl FAILED
FAILED:
link.w %fp,#0
movea.l (8,%fp),%a0
tst.w (%a0)
bpl.s 1f
jsr (EXT_$FAIL).l
1:
unlk %fp
rts
.globl LOW_FLAG
LOW_FLAG:
link.w %fp,#-4
movea.l (8,%fp),%a0
tst.b (1,%a0)
smi %d0
unlk %fp
rts
.balignw 4,0x2048
