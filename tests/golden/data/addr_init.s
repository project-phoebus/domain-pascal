.text
.globl ADDRI_$INIT
ADDRI_$INIT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0),(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.ADDRI","aw",@progbits
.balign 4
.Ldata:
.space 16
.long ADDRI_$INIT
.space 4
.long .Ldata
.long .Ldata+8
.long OTHER
.long EXT+8
.long ADDRI_$EXT
.byte 0x00,0x07
.space 2
.globl DD
DD:
.long .Ldata+12
