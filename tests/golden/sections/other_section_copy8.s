.section ".text.LOG.OS_INIT_PROC","ax",@progbits
.globl LOG_$INIT
LOG_$INIT:
link.w %fp,#-0x14
movea.l #.Ldata.WIRED_DATA,%a0
move.l (%a0)+,(-0xc,%fp)
move.l (%a0)+,(-8,%fp)
pea (-0x14,%fp)
jsr (GET_ATTRIBUTES).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.LOG.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 12
