.text
.globl M_$INIT
M_$INIT:
rts
.globl M_$RELEASE
M_$RELEASE:
rts
.globl M_$PUTC
M_$PUTC:
rts
.globl M_$SET
M_$SET:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w #1,(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl M_$LAST
M_$LAST:
rts
.balignw 4,0x2048
.section ".bss.moddata.M","aw",@nobits
.balign 4
.Ldata:
.space 4
