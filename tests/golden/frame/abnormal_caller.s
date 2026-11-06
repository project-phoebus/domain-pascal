.text
WORKER:
link.w %fp,#-0x4c
movem.l %d2-%d7/%a2-%a4,-(%sp)
movea.l (8,%fp),%a0
clr.l (%a0)
pea (-0x48,%fp)
jsr (CLEANUP).l
addq.w #4,%sp
move.l %d0,(-0x4c,%fp)
pea (-0x4c,%fp)
jsr (NOTE).l
addq.w #4,%sp
pea (-0x48,%fp)
jsr (RLS_CLEANUP).l
movem.l (-0x70,%fp),%d2-%d7/%a2-%a4
unlk %fp
rts
.globl ENTRY
ENTRY:
link.w %fp,#-0x44
movem.l %d2-%d7/%a2-%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
clr.l (%a0)
clr.l (%a5)
pea (-0x40,%fp)
jsr (CLEANUP).l
addq.w #4,%sp
move.l %d0,(-0x44,%fp)
pea (-0x44,%fp)
jsr (NOTE).l
addq.w #4,%sp
pea (-0x40,%fp)
jsr (RLS_CLEANUP).l
addq.w #4,%sp
move.l (8,%fp),-(%sp)
bsr.w WORKER
movem.l (-0x6c,%fp),%d2-%d7/%a2-%a5
unlk %fp
rts
.globl PLAIN
PLAIN:
link.w %fp,#0
movea.l (8,%fp),%a0
clr.l (%a0)
movea.l (0xc,%fp),%a1
clr.l (%a1)
moveq #1,%d0
move.l %d0,(%a0)
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.ABNORMAL_CALLER","aw",@nobits
.balign 4
.Ldata:
.space 4
