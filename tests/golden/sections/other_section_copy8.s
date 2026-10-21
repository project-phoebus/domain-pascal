.section ".text.LOG.OS_INIT_PROC","ax",@progbits
.globl LOG_$INIT
LOG_$INIT:
link.w %fp,#-0x18
movea.l #.Ldata.WIRED_DATA,%a0
move.l (%a0)+,(-0x10,%fp)
move.l (%a0)+,(-0xc,%fp)
pea (-0x18,%fp)
jsr (GET_ATTRIBUTES).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.LOG.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 12
