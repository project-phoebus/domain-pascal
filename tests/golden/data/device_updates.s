.text
.globl ENABLE_DEV
ENABLE_DEV:
link.w %fp,#0
clr.w %d0
move.w (8,%fp),%d1
bset %d1,%d0
or.w %d0,(PBU_CTL+2).l
unlk %fp
rts
.globl DISABLE_DEV
DISABLE_DEV:
link.w %fp,#0
clr.w %d0
move.w (8,%fp),%d1
bset %d1,%d0
not.w %d0
and.w %d0,(PBU_CTL+2).l
unlk %fp
rts
.globl VME_INT
VME_INT:
link.w %fp,#0
move.w (8,%fp),%d0
clr.w %d1
bset %d0,%d1
not.w %d1
and.w %d1,(PBU_CTL+2).l
andi.w #-0x4001,(PBU_CTL).l
ori.w #0x40,(PBU_CTL+4).l
clr.w %d1
bset %d0,%d1
or.w %d1,(PBU_CTL+2).l
unlk %fp
rts
.globl COMMAND
COMMAND:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
andi.b #0xfc,(SM_$CB+0x31).l
move.b %d0,%d2
andi.b #3,%d2
or.b %d2,(SM_$CB+0x31).l
move.b %d1,(SM_$CB+0x30).l
move.l (-4,%fp),%d2
unlk %fp
rts
.globl COUNTERS
COUNTERS:
link.w %fp,#0
addq.w #1,(SM_$CB+0x46).l
ori.b #0x80,(SM_$CB+0x44).l
andi.b #0xfc,(SM_$CB+0x31).l
ori.b #2,(SM_$CB+0x31).l
unlk %fp
rts
.globl SHUTDOWN
SHUTDOWN:
link.w %fp,#0
ori.w #0x700,%sr
andi.b #0xf7,(VME_CTL+0x31).l
unlk %fp
rts
.balignw 4,0x2048
