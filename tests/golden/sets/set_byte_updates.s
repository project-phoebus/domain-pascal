.text
.globl HI2
HI2:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (%a0),%a0
ori.b #0xa0,(4,%a0)
unlk %fp
rts
.globl HI3
HI3:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (%a0),%a0
andi.b #0xc7,(4,%a0)
unlk %fp
rts
.globl HI2D
HI2D:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (%a0),%a0
andi.b #0xe7,(4,%a0)
unlk %fp
rts
.globl LO3
LO3:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (%a0),%a0
andi.w #-0xe1,(4,%a0)
unlk %fp
rts
.globl BOTH
BOTH:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (%a0),%a0
ori.w #0x180,(4,%a0)
unlk %fp
rts
.globl ABSOLUTE
ABSOLUTE:
link.w %fp,#0
bclr #13,(S+0x32).l
bset #15,(S+0x32).l
bset #3,(S+0x33).l
ori.b #0x42,(S+0x32).l
unlk %fp
rts
.globl LONG
LONG:
link.w %fp,#0
ori.l #0x60000000,(L).l
bclr #4,(L+1).l
unlk %fp
rts
.balignw 4,0x2048
