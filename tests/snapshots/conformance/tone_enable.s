.set __dpc_fragment_origin,0xe1ace8
.text
.globl TONE_$ENABLE
TONE_$ENABLE:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
pea (-4,%fp)
move.l (8,%fp),-(%sp)
lea (0x1268,%a5),%a0
move.l %a0,(-8,%fp)
pea (-8,%fp)
jsr (SIO2681_$TONE).l
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.OS_TERM_INIT","aw",@nobits
.balign 4
.Ldata:
.space 4716
