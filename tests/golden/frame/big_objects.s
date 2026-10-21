.text
.globl FILE_$SET_TROUBLE
FILE_$SET_TROUBLE:
link.w %fp,#-0x38
st (-0x38,%fp)
pea (-0x38,%fp)
pea (.Lc0,%pc)
jsr (SET_ATTRIBUTE).l
unlk %fp
rts
.Lc0:
.short 0x0002
.globl FILE_$SET_TYPE
FILE_$SET_TYPE:
link.w %fp,#-0x40
movea.l (0xc,%fp),%a0
move.l (%a0)+,(-0x40,%fp)
move.l (%a0)+,(-0x3c,%fp)
lea (-0x40,%fp),%a0
move.l (%a0)+,(-0x38,%fp)
move.l (%a0)+,(-0x34,%fp)
pea (-0x38,%fp)
pea (.Lc1,%pc)
move.l (8,%fp),-(%sp)
jsr (SET_ATTRIBUTE_UID).l
unlk %fp
rts
.Lc1:
.short 0x0004
I_WAIT:
link.w %fp,#-0x60
pea (-0x60,%fp)
pea (-0x58,%fp)
pea (-0x50,%fp)
pea (-0x40,%fp)
pea (-0x30,%fp)
pea (-0x10,%fp)
jsr (WAITN).l
unlk %fp
rts
.globl OPEN
OPEN:
link.w %fp,#-0x2c
pea (-0x2c,%fp)
pea (-0x28,%fp)
jsr (USE_OPEN).l
unlk %fp
rts
.balignw 4,0x2048
