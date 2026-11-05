.text
.globl ADDRARGS_$STOP
ADDRARGS_$STOP:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (%a5),-(%sp)
jsr (EC_$ADVANCE).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl ADDRARGS_$INIT_DTTE
ADDRARGS_$INIT_DTTE:
link.w %fp,#-4
pea (%a2)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a0
move.l (%a0),(-4,%fp)
pea (0xc,%a2)
jsr (EC_$INIT).l
addq.w #4,%sp
pea (%a2)
jsr (EC_$INIT).l
addq.w #4,%sp
move.l #CLOCKH,-(%sp)
jsr (EC_$READ).l
move.l %d0,(-4,%fp)
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl ADDRARGS_$CHANS
ADDRARGS_$CHANS:
link.w %fp,#0
subq.l #2,%sp
move.w #3,-(%sp)
pea (CHANS+0x24).l
jsr (CHAN_INIT).l
addq.w #8,%sp
subq.l #2,%sp
clr.w -(%sp)
move.l #CHANS,-(%sp)
jsr (CHAN_INIT).l
unlk %fp
rts
.globl ADDRARGS_$WAIT_QUIT
ADDRARGS_$WAIT_QUIT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
clr.l -(%sp)
move.l #CLOCKH,-(%sp)
move.w (AS_ID).l,%d0
lsl.w #2,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
pea (4,%a5,%d0.w)
jsr (EC_$WAIT).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl ADDRARGS_$WAIT_HELD
ADDRARGS_$WAIT_HELD:
link.w %fp,#-4
pea (%a2)
movea.l (8,%fp),%a2
clr.w (0xc,%a2)
pea (1).w
move.l #CLOCKH,-(%sp)
pea (0xe,%a2)
jsr (EC_$WAIT).l
move.w %d0,%d1
move.w %d1,(0xc,%a2)
movea.l (-8,%fp),%a2
unlk %fp
rts
FORGET:
link.w %fp,#-4
movea.l (%fp),%a0
clr.l (-4,%a0)
unlk %fp
rts
.globl ADDRARGS_$WAIT_LOCAL
ADDRARGS_$WAIT_LOCAL:
link.w %fp,#-8
move.l (OTHR).l,(-4,%fp)
pea (1).w
move.l #CLOCKH,-(%sp)
movea.l (-4,%fp),%a0
pea (0xe,%a0)
jsr (EC_$WAIT).l
move.w %d0,%d1
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ADDRARGS","aw",@nobits
.balign 4
.Ldata:
.space 700
