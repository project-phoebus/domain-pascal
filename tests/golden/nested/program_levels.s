.text
THREE:
link.w %fp,#-0x10
pea (-0x10,%fp)
jsr (USE).l
unlk %fp
rts
TWO:
link.w %fp,#-0xc
bsr.s THREE
pea (-0xc,%fp)
jsr (USE).l
unlk %fp
rts
ONE:
link.w %fp,#-8
bsr.s TWO
pea (-8,%fp)
jsr (USE).l
unlk %fp
rts
.globl PROGRAM_LEVELS
PROGRAM_LEVELS:
link.w %fp,#0
bsr.s ONE
unlk %fp
rts
.balignw 4,0x2048
