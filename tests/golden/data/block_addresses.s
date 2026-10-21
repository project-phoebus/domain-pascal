.text
.globl GET_INFO
GET_INFO:
link.w %fp,#0
movea.l #CAL_$TIMEZONE,%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
unlk %fp
rts
.globl GET_86
GET_86:
link.w %fp,#0
movea.l #DISK_$ERROR_INFO,%a0
movea.l (8,%fp),%a1
moveq #0x14,%d0
.balignw 4,0x2048
1:
move.l (%a0)+,(%a1)+
dbf %d0,1b
move.w (%a0)+,(%a1)+
unlk %fp
rts
.globl PUT_BIG
PUT_BIG:
link.w %fp,#-0x210
clr.l (-0x210,%fp)
lea (-0x210,%fp),%a0
movea.l #OTHER_BIG,%a1
move.w #0x82,%d0
.balignw 4,0x2048
1:
move.l (%a0)+,(%a1)+
dbf %d0,1b
unlk %fp
rts
.globl CHECK
CHECK:
link.w %fp,#0
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
lea (%a0),%a2
movea.l (0x10,%fp),%a3
move.l (%a2)+,(%a3)
move.l (%a2)+,(4,%a3)
clr.w (%a1)
lea (%a0),%a2
movea.l #UID_$NIL,%a3
moveq #1,%d0
cmpm.l (%a3)+,(%a2)+
bne.s 1f
cmpm.l (%a3)+,(%a2)+
bne.s 1f
move.w #1,(%a1)
1:
tst.l (4,%a0)
bne.s 3f
move.w #2,(%a1)
3:
movem.l (-8,%fp),%a2-%a3
unlk %fp
rts
.globl STATS
STATS:
link.w %fp,#0
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
lea (%a5),%a1
lea (%a0),%a2
moveq #4,%d0
.balignw 4,0x2048
1:
move.l (%a1)+,(%a2)+
dbf %d0,1b
move.w (%a1)+,(%a2)+
move.w #1,(0x14,%a0)
movem.l (-8,%fp),%a2/%a5
unlk %fp
rts
.globl SCAN
SCAN:
link.w %fp,#-8
movem.l %d2/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d2
movea.l (0xc,%fp),%a2
move.l %d2,%d0
subq.l #4,%d0
move.l %d0,(0x18,%a5)
move.l (%a2),(-8,%fp)
move.w (4,%a2),(-4,%fp)
lea (-8,%fp),%a0
movea.l %d2,%a1
moveq #2,%d0
.balignw 4,0x2048
3:
cmpm.w (%a1)+,(%a0)+
dbne %d0,3b
beq.s 1f
pea (-8,%fp)
move.l %d2,-(%sp)
jsr (SUB48).l
addq.w #8,%sp
1:
pea (%a2)
move.l %d2,-(%sp)
jsr (SUB48).l
movem.l (-0x14,%fp),%d2/%a2/%a5
unlk %fp
rts
.globl FIND
FIND:
link.w %fp,#-0x10
movem.l %d2-%d3/%a2,-(%sp)
movea.l (8,%fp),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
clr.w %d0
moveq #5,%d1
movea.l #DISK_$DVTBL,%a0
moveq #1,%d2
lea (0x48,%a0),%a0
.balignw 4,0x2048
2:
cmpi.w #3,(-0x28,%a0)
bne.s 4f
tst.l (-0x40,%a0)
beq.s 4f
lea (-0x48,%a0),%a1
lea (-8,%fp),%a2
moveq #1,%d3
cmpm.l (%a2)+,(%a1)+
bne.s 4f
cmpm.l (%a2)+,(%a1)+
bne.s 4f
move.w %d2,%d0
bra.s 1f
4:
addq.w #1,%d2
lea (0x48,%a0),%a0
dbf %d1,2b
1:
movem.l (-0x1c,%fp),%d2-%d3/%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BLKADDR","aw",@nobits
.balign 4
.Ldata:
.space 28
