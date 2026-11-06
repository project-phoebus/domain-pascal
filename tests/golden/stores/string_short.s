.text
.globl P
P:
link.w %fp,#-0xc
pea (%a5)
lea (.Ldata).l,%a5
move.w #0x2020,(%a5)
move.l #0x31323030,(2,%a5)
move.l #0x53657665,(6,%a5)
move.w #0x6e20,(0xa,%a5)
move.l #0x20202020,(-4,%fp)
move.l #0x61626320,(-0xa,%fp)
move.l #0x78202020,(2,%a5)
move.w #0x7920,(%a5)
clr.w (-6,%fp)
movea.l (-0x10,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.STRING_SHORT","aw",@nobits
.balign 4
.Ldata:
.space 12
