.text
INIT_UNIT:
link.w %fp,#-8
movea.l (%fp),%a0
moveq #0x2e,%d0
movea.l #UNITS,%a1
mulu.w (-2,%a0),%d0
lea (0,%a1,%d0.w),%a0
addq.w #1,(2,%a0)
clr.w (0x2c,%a0)
unlk %fp
rts
INIT_BIG:
link.w %fp,#-4
movea.l (%fp),%a0
moveq #0x2e,%d0
movea.l #BIG,%a1
muls.w (-2,%a0),%d0
lea (0,%a1,%d0.l),%a0
adda.l #0x8fec,%a0
move.w #1,(%a0)
unlk %fp
rts
.globl UI_$ACQUIRE
UI_$ACQUIRE:
link.w %fp,#-4
movea.l (8,%fp),%a0
move.w (%a0),(-2,%fp)
bsr.s INIT_UNIT
bsr.s INIT_BIG
unlk %fp
rts
.balignw 4,0x2048
