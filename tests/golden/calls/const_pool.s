.text
.globl CPOOL_$ORDER
CPOOL_$ORDER:
link.w %fp,#0
pea (.Lc0,%pc)
pea (.Lc1,%pc)
jsr (EXT_$TWO).l
addq.w #8,%sp
pea (.Lc2,%pc)
pea (.Lc3,%pc)
jsr (EXT_$TWO).l
addq.w #8,%sp
pea (.Lc2,%pc)
pea (.Lc4,%pc)
jsr (EXT_$TWO).l
unlk %fp
rts
.Lc0:
.short 0x1046
.Lc4:
.short 0x0003
.Lc3:
.short 0x0002
.Lc2:
.short 0xffff
.Lc1:
.short 0x0001
.globl CPOOL_$GROUPS
CPOOL_$GROUPS:
link.w %fp,#0
pea (.Lc5,%pc)
pea (.Lc6,%pc)
jsr (EXT_$LONG).l
addq.w #8,%sp
pea (.Lc7,%pc)
move.l (%sp),-(%sp)
jsr (EXT_$TWO).l
unlk %fp
rts
.Lc5:
.short 0x0004
.Lc7:
.short 0x0007
.balignw 4,0x2048
.Lc6:
.short 0x0000,0x0000
.globl CPOOL_$REUSE
CPOOL_$REUSE:
link.w %fp,#0
pea (.Lc7,%pc)
pea (.Lc1,%pc)
jsr (EXT_$TWO).l
addq.w #8,%sp
pea (.Lc8,%pc)
jsr (EXT_$COLOR).l
unlk %fp
rts
.Lc8:
.short 0x0001
.globl CPOOL_$TEMP
CPOOL_$TEMP:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
pea (.Lc9,%pc)
lea (%a5),%a0
move.l %a0,(-8,%fp)
pea (-8,%fp)
jsr (EXT_$PTR).l
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.Lc9:
.short 0x0005
.balignw 4,0x2048
.section ".bss.moddata.CPOOL","aw",@nobits
.balign 4
.Ldata:
.space 4
