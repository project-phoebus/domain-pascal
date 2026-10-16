.text
.globl SELFA_$TOUCH
SELFA_$TOUCH:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
clr.l (%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.SELFA","aw",@progbits
.balign 4
.Ldata:
.space 8
.long .Ldata+4
.long .Ldata+4
.space 8
.long .Ldata+20
.long .Ldata+20
.globl GLOB_EC
GLOB_EC:
.byte 0x00,0x00,0x00,0x01
.long .Ldata+32
.long .Ldata+40
.long .Ldata+44
.space 4
.long .Ldata+48
.long .Ldata+48
.section ".data.overlay.SELFA_OV","aw",@progbits
.balign 4
.globl SELFA_OV
.type SELFA_OV,@object
.size SELFA_OV,16
SELFA_OV:
.space 8
.long SELFA_OV+4
.long SELFA_OV+4
