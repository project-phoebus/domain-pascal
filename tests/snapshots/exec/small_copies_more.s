.text
.globl SAME_SLOT
SAME_SLOT:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(8,%a1)
move.l (%a0)+,(0xc,%a1)
unlk %fp
rts
.globl TWO_VARS
TWO_VARS:
link.w %fp,#0
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
unlk %fp
rts
.globl VIA_PTR_LOCALS
VIA_PTR_LOCALS:
link.w %fp,#-0xc
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
lea (0x20,%a5),%a0
move.l %a0,(-0xc,%fp)
pea (-0xc,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
movea.l %a0,%a2
lea (0x10,%a5),%a0
move.l %a0,(-0xc,%fp)
pea (-0xc,%fp)
jsr (EXEC_$ID_P).l
lea (%a0),%a1
move.l (%a1)+,(%a2)
move.l (%a1)+,(4,%a2)
movem.l (-0x14,%fp),%a2/%a5
unlk %fp
rts
.globl KEEP_PTR
KEEP_PTR:
link.w %fp,#0
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a3
lea (%a3),%a0
move.l (%a0)+,(%a2)
move.l (%a0)+,(4,%a2)
jsr (EXEC_$CLOBBER).l
lea (%a3),%a0
move.l (%a0)+,(8,%a2)
move.l (%a0)+,(0xc,%a2)
movem.l (-8,%fp),%a2-%a3
unlk %fp
rts
.globl ONE
ONE:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a1
lea (%a5),%a0
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl TWO
TWO:
link.w %fp,#0
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a3
lea (%a3),%a0
move.l (%a0)+,(%a2)
move.l (%a0)+,(4,%a2)
jsr (EXEC_$CLOBBER).l
lea (%a2),%a0
move.l (%a0)+,(%a3)
move.l (%a0)+,(4,%a3)
movem.l (-8,%fp),%a2-%a3
unlk %fp
rts
.globl THREE
THREE:
link.w %fp,#0
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
move.l (%a0),(%a1)
move.w (4,%a0),(4,%a1)
unlk %fp
rts
.globl OVERLAP
OVERLAP:
link.w %fp,#-0xc
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
lea (0x48,%a5),%a0
move.l %a0,(-0xc,%fp)
pea (-0xc,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
movea.l %a0,%a2
lea (0x44,%a5),%a0
move.l %a0,(-0xc,%fp)
pea (-0xc,%fp)
jsr (EXEC_$ID_P).l
lea (%a0),%a1
move.l (%a1)+,(%a2)
move.l (%a1)+,(4,%a2)
movem.l (-0x14,%fp),%a2/%a5
unlk %fp
rts
.globl P1
P1:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0),(-8,%fp)
move.w (4,%a0),(-4,%fp)
move.l (-8,%fp),(0x50,%a5)
move.w (-4,%fp),(0x54,%a5)
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl P2
P2:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0),(-8,%fp)
move.w (4,%a0),(-4,%fp)
move.l (-8,%fp),(0x58,%a5)
move.w (-4,%fp),(0x5c,%a5)
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl P3
P3:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
lea (-8,%fp),%a0
move.l (%a0)+,(0x60,%a5)
move.l (%a0)+,(0x64,%a5)
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl P4
P4:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
lea (-8,%fp),%a0
move.l (%a0)+,(0x68,%a5)
move.l (%a0)+,(0x6c,%a5)
subq.l #2,%sp
move.w (0xc,%fp),-(%sp)
jsr (EXEC_$LOGV_W).l
movea.l (-0xc,%fp),%a5
unlk %fp
rts
INNER:
link.w %fp,#-4
movea.l (%fp),%a0
lea (8,%a5),%a1
move.l (%a1)+,(-0x10,%a0)
move.l (%a1)+,(-0xc,%a0)
unlk %fp
rts
.globl SM_$MAIN
SM_$MAIN:
link.w %fp,#-0x20
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w %d2,%d0
ext.l %d0
move.l %d0,(%a5)
move.l %d0,%d1
addq.l #1,%d1
move.l %d1,(4,%a5)
pea (%a5)
bsr.w SAME_SLOT
addq.w #4,%sp
pea (.Lc0,%pc)
pea (%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
moveq #2,%d0
move.l %d0,(0x14,%a5)
moveq #1,%d1
move.l %d1,(0x10,%a5)
lea (0x10,%a5),%a0
move.l (%a0)+,(0x18,%a5)
move.l (%a0)+,(0x1c,%a5)
pea (.Lc0,%pc)
pea (0x10,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
moveq #8,%d0
move.l %d0,(-4,%fp)
moveq #7,%d1
move.l %d1,(-8,%fp)
pea (-8,%fp)
pea (0x10,%a5)
bsr.w TWO_VARS
addq.w #8,%sp
pea (.Lc1,%pc)
pea (0x10,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
bsr.w VIA_PTR_LOCALS
pea (.Lc1,%pc)
pea (0x20,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
move.w #0xa,(-0x14,%fp)
moveq #9,%d0
move.l %d0,(-0x18,%fp)
move.l %d0,(0x28,%a5)
move.w (-0x14,%fp),(0x2c,%a5)
move.l %d0,(0x2e,%a5)
move.w (0x2c,%a5),(0x32,%a5)
pea (.Lc2,%pc)
pea (0x28,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
lea (%a5),%a0
move.l %a0,(-0x20,%fp)
pea (-0x20,%fp)
jsr (EXEC_$ID_P).l
addq.w #4,%sp
movea.l %a0,%a2
lea (%a2),%a0
move.l (%a0)+,(8,%a2)
move.l (%a0)+,(0xc,%a2)
pea (.Lc1,%pc)
pea (8,%a2)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (%a5)
pea (%a5)
bsr.w KEEP_PTR
addq.w #8,%sp
pea (.Lc1,%pc)
pea (8,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (0x34,%a5)
bsr.w ONE
addq.w #4,%sp
pea (.Lc1,%pc)
pea (0x34,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (0x34,%a5)
pea (0x3c,%a5)
bsr.w TWO
addq.w #8,%sp
pea (.Lc1,%pc)
pea (0x3c,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc1,%pc)
pea (0x34,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
bsr.w INNER
pea (.Lc1,%pc)
pea (-0x10,%fp)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
clr.l (0x4c,%a5)
moveq #2,%d0
move.l %d0,(0x48,%a5)
moveq #1,%d1
move.l %d1,(0x44,%a5)
bsr.w OVERLAP
pea (.Lc2,%pc)
pea (0x44,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
move.w %d2,%d0
ext.l %d0
move.l %d0,(-0x18,%fp)
move.w #4,(-0x14,%fp)
pea (-0x18,%fp)
bsr.w P1
addq.w #4,%sp
pea (.Lc3,%pc)
pea (0x50,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (-0x18,%fp)
bsr.w P2
addq.w #4,%sp
pea (.Lc3,%pc)
pea (0x58,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
move.w %d2,%d0
ext.l %d0
move.l %d0,(-8,%fp)
moveq #4,%d0
move.l %d0,(-4,%fp)
pea (-8,%fp)
bsr.w P3
addq.w #4,%sp
pea (.Lc1,%pc)
pea (0x60,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
subq.l #2,%sp
move.w #9,-(%sp)
pea (-8,%fp)
bsr.w P4
addq.w #8,%sp
pea (.Lc1,%pc)
pea (0x68,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (0x50,%a5)
pea (0x70,%a5)
bsr.w THREE
addq.w #8,%sp
pea (.Lc4,%pc)
pea (0x70,%a5)
jsr (EXEC_$LOG_MEM).l
movem.l (-0x2c,%fp),%d2/%a2/%a5
unlk %fp
rts
.Lc0:
.short 0x0010
.Lc4:
.short 0x000a
.Lc3:
.short 0x0006
.Lc2:
.short 0x000c
.Lc1:
.short 0x0008
.balignw 4,0x2048
.section ".bss.moddata.SM","aw",@nobits
.balign 4
.Ldata:
.space 124
