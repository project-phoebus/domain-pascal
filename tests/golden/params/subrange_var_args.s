.text
.globl SA_$EXACT
SA_$EXACT:
link.w %fp,#-0x14
movea.l (8,%fp),%a0
move.l (%a0),(-4,%fp)
pea (-6,%fp)
jsr (SA_$P16).l
addq.w #4,%sp
pea (-8,%fp)
jsr (SA_$PPI).l
addq.w #4,%sp
pea (-0xc,%fp)
jsr (SA_$PLI).l
addq.w #4,%sp
pea (-0xe,%fp)
jsr (SA_$PIO).l
addq.w #4,%sp
pea (-0x14,%fp)
pea (-6,%fp)
movea.l (-4,%fp),%a0
jsr (%a0)
unlk %fp
rts
.globl SA_$MIXED
SA_$MIXED:
link.w %fp,#-0x14
movea.l (8,%fp),%a0
move.l (%a0),(-4,%fp)
pea (-0xe,%fp)
jsr (SA_$P16).l
addq.w #4,%sp
pea (-6,%fp)
jsr (SA_$PPI).l
addq.w #4,%sp
pea (-0xc,%fp)
jsr (SA_$PLI).l
addq.w #4,%sp
pea (-6,%fp)
jsr (SA_$PIO).l
addq.w #4,%sp
pea (-0x14,%fp)
pea (-8,%fp)
movea.l (-4,%fp),%a0
jsr (%a0)
unlk %fp
rts
.globl SA_$CONSTS
SA_$CONSTS:
link.w %fp,#-0xc
move.w #0x7fff,(-4,%fp)
move.b #0x7e,(-7,%fp)
move.w #0x1170,(-0xa,%fp)
pea (-6,%fp)
jsr (SA_$PIO).l
addq.w #4,%sp
pea (-0xa,%fp)
jsr (SA_$PPI).l
addq.w #4,%sp
subq.l #2,%sp
pea (-8,%fp)
move.w #-0x63c0,-(%sp)
jsr (SA_$VAL).l
unlk %fp
rts
.globl SA_$REG
SA_$REG:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d0
move.w #0x1170,%d2
move.w %d0,%d1
subq.w #1,%d1
bmi.s 1f
move.w %d1,%d3
.balignw 4,0x2048
2:
subq.l #2,%sp
move.w %d2,-(%sp)
jsr (SA_$USE).l
addq.w #4,%sp
addq.w #1,%d2
dbf %d3,2b
1:
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.globl SA_$MORE
SA_$MORE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
pea (.Lc0,%pc)
jsr (SA_$BYREF).l
addq.w #4,%sp
subq.l #2,%sp
move.w #0x2c00,-(%sp)
jsr (SA_$BYTE).l
andi.b #0xf,(4,%a5)
ori.b #0x10,(4,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.Lc0:
.short 0x1170
.balignw 4,0x2048
.section ".moddata.SA","aw",@progbits
.balign 4
.Ldata:
.byte 0x11,0x70,0x02
.space 5
