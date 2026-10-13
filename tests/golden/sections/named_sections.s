.section ".text.SECT.WIRED_PROC","ax",@progbits
SECT_$COUNT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
addq.l #1,(%a5)
movea.l (-8,%fp),%a5
unlk %fp
rts
SECT_$HITS:
link.w %fp,#-8
pea (%a5)
lea (.Ldata.WIRED_DATA).l,%a5
move.l (%a5),%d0
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl SECT_$TICK
SECT_$TICK:
link.w %fp,#0
bsr.s SECT_$COUNT
unlk %fp
rts
.balignw 4,0x2048
.section ".text.SECT.INIT_PROC","ax",@progbits
.globl SECT_$INIT
SECT_$INIT:
link.w %fp,#0
pea (%a5)
lea (.Ldata.INIT_DATA).l,%a5
jsr (SECT_$COUNT).l
jsr (SECT_$HITS).l
move.l %d0,(%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SECT.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 4
.section ".bss.moddata.SECT.INIT_DATA","aw",@nobits
.balign 4
.Ldata.INIT_DATA:
.space 4
