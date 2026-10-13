.text
.globl M_$ONE
M_$ONE:
link.w %fp,#-4
pea (-2,%fp)
jsr (M_$PROBE1).l
unlk %fp
rts
.globl M_$THREE
M_$THREE:
link.w %fp,#-8
pea (-8,%fp)
pea (-4,%fp)
pea (-2,%fp)
jsr (M_$PROBE3).l
unlk %fp
rts
.balignw 4,0x2048
