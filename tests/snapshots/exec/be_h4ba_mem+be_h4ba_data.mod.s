.text
.globl BH_$DUMP
BH_$DUMP:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.Lc0:
.short 0x0008
.balignw 4,0x2048
.section ".bss.moddata.BE_H4BA_DATA","aw",@nobits
.balign 4
.Ldata:
.globl BH_$BITS
BH_$BITS:
.space 8
.globl BH_$RECS
BH_$RECS:
.space 240
.globl BH_$K
BH_$K:
.space 4
