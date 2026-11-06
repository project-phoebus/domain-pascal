.text
BUMP:
link.w %fp,#-4
move.w (8,%fp),%d0
ext.l %d0
add.l (%a5),%d0
move.l %d0,(%a5)
unlk %fp
rts
KEEP:
link.w %fp,#-0x10
move.l %d2,-(%sp)
move.w (0xc,%fp),%d0
movea.l (8,%fp),%a0
move.l (%a0)+,(-0x10,%fp)
move.l (%a0)+,(-0xc,%fp)
lea (-0x10,%fp),%a0
move.l (%a0)+,(4,%a5)
move.l (%a0)+,(8,%a5)
move.l (0x18,%a5),%d1
move.w %d0,%d2
add.l %d1,%d1
ext.l %d2
movea.l (-4,%a5),%a0
add.l %d2,%d1
subq.l #2,%d1
move.l %d1,(%a0)
pea (.Lc0,%pc)
move.l (-8,%a5),-(%sp)
jsr (PM_$ADD).l
move.l (-0x14,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x0004
.globl PROGRAM_MAIN
PROGRAM_MAIN:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w #2,-(%sp)
bsr.s BUMP
addq.w #4,%sp
movea.l (-0xc,%a5),%a0
lea (0xc,%a5),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
subq.l #2,%sp
move.w #2,-(%sp)
move.l (-8,%a5),-(%sp)
bsr.w KEEP
movea.l (-0x10,%a5),%a0
lea (4,%a5),%a1
move.l (%a1)+,(%a0)
move.l (%a1)+,(4,%a0)
movea.l (-0x14,%a5),%a0
move.l (%a5),%d0
add.l (%a0),%d0
movea.l (-0x14,%a5),%a0
move.l %d0,(%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PROGRAM_MAIN","aw",@progbits
.balign 4
.long COUNT
.long TARGET
.long BLOCK
.long SOURCE
.long LOG
.Ldata:
.byte 0x00,0x00,0x00,0x28
.space 24
