.text
.globl MOUNT
MOUNT:
link.w %fp,#-4
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
pea (8,%fp)
jsr (GET_BLOCK).l
addq.w #4,%sp
moveq #0x64,%d0
lea (0x4c,%a0),%a1
mulu.w %d2,%d0
lea (0x5c,%a5,%d0.w),%a2
moveq #0x18,%d0
.balignw 4,0x2048
1:
move.l (%a1)+,(%a2)+
dbf %d0,1b
clr.w (%a0)
movem.l (-0x10,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl DISMOUNT
DISMOUNT:
link.w %fp,#-4
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
pea (8,%fp)
jsr (GET_BLOCK).l
addq.w #4,%sp
moveq #0x64,%d0
mulu.w %d2,%d0
lea (0x5c,%a5,%d0.w),%a1
lea (0x4c,%a0),%a2
moveq #0x18,%d0
.balignw 4,0x2048
1:
move.l (%a1)+,(%a2)+
dbf %d0,1b
clr.w (%a0)
movem.l (-0x10,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl VDM
VDM:
link.w %fp,#-8
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
pea (-8,%fp)
jsr (GET_UID).l
addq.w #4,%sp
move.w %d2,%d0
lea (-8,%fp),%a0
lsl.w #5,%d0
lea (-0xc,%a5,%d0.w),%a1
moveq #1,%d0
cmpm.l (%a1)+,(%a0)+
bne.s 2f
cmpm.l (%a1)+,(%a0)+
beq.s 1f
2:
pea (-8,%fp)
jsr (GET_UID).l
addq.w #4,%sp
1:
move.w %d2,%d0
lsl.w #5,%d0
lea (-0x1c,%a5,%d0.w),%a0
movea.l #UID_$NIL,%a1
moveq #1,%d0
cmpm.l (%a1)+,(%a0)+
bne.s 3f
cmpm.l (%a1)+,(%a0)+
bne.s 3f
pea (-8,%fp)
jsr (GET_UID).l
3:
movem.l (-0x10,%fp),%d2/%a5
unlk %fp
rts
.globl SET_WDIRUS
SET_WDIRUS:
link.w %fp,#0
movem.l %a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.w (CUR).l,%d0
lea (%a0),%a3
lsl.w #3,%d0
lea (0,%a5,%d0.w),%a2
moveq #1,%d0
lea (0x4b4,%a2),%a2
cmpm.l (%a2)+,(%a3)+
bne.s 1f
cmpm.l (%a2)+,(%a3)+
bne.s 1f
clr.l (%a1)
bra.s 3f
1:
move.w (CUR).l,%d0
lsl.w #3,%d0
lea (0,%a5,%d0.w),%a2
move.l (%a0),(0x4b4,%a2)
move.l (4,%a0),(%a1)
3:
movem.l (-0xc,%fp),%a2-%a3/%a5
unlk %fp
rts
.globl FM_WRITE
FM_WRITE:
link.w %fp,#0
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
move.w (0x10,%fp),%d0
move.w %d0,%d1
lea (%a1),%a3
muls.w #0x160,%d1
lea (0,%a0,%d1.l),%a2
moveq #0x1f,%d1
lea (0xd8,%a2),%a2
.balignw 4,0x2048
1:
move.l (%a3)+,(%a2)+
dbf %d1,1b
move.w %d0,%d1
muls.w #0x160,%d1
clr.w (0,%a0,%d1.l)
move.w (6,%a1),(2,%a0)
movem.l (-8,%fp),%a2-%a3
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BLKELEM","aw",@nobits
.balign 4
.Ldata:
.space 1716
