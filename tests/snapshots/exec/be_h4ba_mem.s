.text
.globl BH_M_$BITS
BH_M_$BITS:
link.w %fp,#0
movem.l %d2-%d4,-(%sp)
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
beq.s 1f
move.w %d2,%d0
moveq #7,%d1
subq.w #1,%d0
move.l #0x80,%d4
and.w %d0,%d1
lsr.l %d1,%d4
move.w %d0,%d1
movea.l #BH_$BITS,%a0
lsr.w #3,%d1
lea (0,%a0,%d1.w),%a0
or.b %d4,(%a0)
bra.s 2f
1:
move.w %d2,%d0
moveq #7,%d1
subq.w #1,%d0
move.l #-0x81,%d4
and.w %d0,%d1
lsr.l %d1,%d4
move.w %d0,%d1
movea.l #BH_$BITS,%a0
lsr.w #3,%d1
lea (0,%a0,%d1.w),%a0
and.b %d4,(%a0)
2:
move.b %d2,%d0
clr.w %d1
subq.b #1,%d0
move.b %d0,%d1
lsr.w #3,%d1
movea.l #BH_$BITS,%a0
lea (0,%a0,%d1.w),%a0
moveq #7,%d1
moveq #7,%d4
and.b %d0,%d1
sub.b %d1,%d4
btst %d4,(%a0)
beq.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 11f
3:
subq.l #2,%sp
clr.w -(%sp)
11:
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
move.w %d2,%d0
moveq #7,%d1
subq.w #1,%d0
move.l #0x80,%d4
and.w %d0,%d1
lsr.l %d1,%d4
move.w %d0,%d1
movea.l #BH_$BITS,%a0
lsr.w #3,%d1
lea (0,%a0,%d1.w),%a0
or.b %d4,(%a0)
move.b %d2,%d0
clr.w %d1
subq.b #1,%d0
move.b %d0,%d1
lsr.w #3,%d1
movea.l #BH_$BITS,%a0
lea (0,%a0,%d1.w),%a0
moveq #7,%d1
moveq #7,%d4
and.b %d0,%d1
sub.b %d1,%d4
btst %d4,(%a0)
beq.s 5f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 12f
5:
subq.l #2,%sp
clr.w -(%sp)
12:
jsr (EXEC_$MARK).l
addq.w #4,%sp
6:
tst.w %d3
bpl.s 7f
move.w %d2,%d0
moveq #7,%d1
subq.w #1,%d0
move.l #0x80,%d3
and.w %d0,%d1
lsr.l %d1,%d3
move.w %d0,%d1
movea.l #BH_$BITS,%a0
lsr.w #3,%d1
lea (0,%a0,%d1.w),%a0
or.b %d3,(%a0)
bra.s 8f
7:
move.w %d2,%d0
moveq #7,%d1
subq.w #1,%d0
move.l #-0x81,%d3
and.w %d0,%d1
lsr.l %d1,%d3
move.w %d0,%d1
movea.l #BH_$BITS,%a0
lsr.w #3,%d1
lea (0,%a0,%d1.w),%a0
and.b %d3,(%a0)
8:
move.b %d2,%d0
clr.w %d1
subq.b #1,%d0
move.b %d0,%d1
lsr.w #3,%d1
movea.l #BH_$BITS,%a0
lea (0,%a0,%d1.w),%a0
moveq #7,%d1
moveq #7,%d2
and.b %d0,%d1
sub.b %d1,%d2
btst %d2,(%a0)
beq.s 9f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 13f
9:
subq.l #2,%sp
clr.w -(%sp)
13:
jsr (EXEC_$MARK).l
10:
movem.l (-0xc,%fp),%d2-%d4
unlk %fp
rts
.globl BH_M_$SET
BH_M_$SET:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (8,%fp),%d0
moveq #7,%d1
subq.w #1,%d0
move.l #0x80,%d2
and.w %d0,%d1
lsr.l %d1,%d2
move.w %d0,%d1
movea.l #BH_$BITS,%a0
lsr.w #3,%d1
lea (0,%a0,%d1.w),%a0
or.b %d2,(%a0)
move.l (-4,%fp),%d2
unlk %fp
rts
.globl BH_M_$CLR
BH_M_$CLR:
link.w %fp,#0
move.l %d2,-(%sp)
move.w (8,%fp),%d0
moveq #7,%d1
subq.w #1,%d0
move.l #-0x81,%d2
and.w %d0,%d1
lsr.l %d1,%d2
move.w %d0,%d1
movea.l #BH_$BITS,%a0
lsr.w #3,%d1
lea (0,%a0,%d1.w),%a0
and.b %d2,(%a0)
move.l (-4,%fp),%d2
unlk %fp
rts
.globl BH_M_$CMP4
BH_M_$CMP4:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w #1,(%a5)
move.w #2,(2,%a5)
move.w #1,(4,%a5)
move.w #2,(6,%a5)
cmpi.w #1,%d0
bne.s 1f
move.w %d1,(%a5)
bra.s 2f
1:
move.w %d1,(2,%a5)
2:
lea (%a5),%a0
lea (4,%a5),%a1
moveq #0,%d0
.balignw 4,0x2048
5:
cmpm.l (%a1)+,(%a0)+
dbne %d0,5b
bne.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 11f
3:
subq.l #2,%sp
clr.w -(%sp)
11:
jsr (EXEC_$MARK).l
addq.w #4,%sp
6:
lea (%a5),%a0
lea (4,%a5),%a1
moveq #0,%d0
.balignw 4,0x2048
9:
cmpm.l (%a1)+,(%a0)+
dbne %d0,9b
beq.s 7f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 12f
7:
subq.l #2,%sp
clr.w -(%sp)
12:
jsr (EXEC_$MARK).l
10:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BH_M_$CMP10
BH_M_$CMP10:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w #1,(8,%a5)
move.w #2,(0xa,%a5)
move.w #3,(0xc,%a5)
move.w #4,(0xe,%a5)
move.w #5,(0x10,%a5)
move.w #1,(0x12,%a5)
move.w #2,(0x14,%a5)
move.w #3,(0x16,%a5)
move.w #4,(0x18,%a5)
move.w #5,(0x1a,%a5)
move.w %d0,%d2
add.w %d2,%d2
move.w %d1,(6,%a5,%d2.w)
lea (8,%a5),%a0
lea (0x12,%a5),%a1
moveq #4,%d0
.balignw 4,0x2048
3:
cmpm.w (%a1)+,(%a0)+
dbne %d0,3b
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 9f
1:
subq.l #2,%sp
clr.w -(%sp)
9:
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
lea (8,%a5),%a0
lea (0x12,%a5),%a1
moveq #4,%d0
.balignw 4,0x2048
7:
cmpm.w (%a1)+,(%a0)+
dbne %d0,7b
beq.s 5f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 10f
5:
subq.l #2,%sp
clr.w -(%sp)
10:
jsr (EXEC_$MARK).l
8:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BH_M_$CMP3
BH_M_$CMP3:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.b #0x41,(0x1c,%a5)
clr.b (0x1d,%a5)
move.b #0x41,(0x1e,%a5)
clr.b (0x1f,%a5)
move.b %d1,(0x1b,%a5,%d0.w)
lea (0x1c,%a5),%a0
lea (0x1e,%a5),%a1
moveq #0,%d0
.balignw 4,0x2048
3:
cmpm.w (%a1)+,(%a0)+
dbne %d0,3b
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 9f
1:
subq.l #2,%sp
clr.w -(%sp)
9:
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
lea (0x1c,%a5),%a0
lea (0x1e,%a5),%a1
moveq #0,%d0
.balignw 4,0x2048
7:
cmpm.w (%a1)+,(%a0)+
dbne %d0,7b
beq.s 5f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 10f
5:
subq.l #2,%sp
clr.w -(%sp)
10:
jsr (EXEC_$MARK).l
8:
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BH_M_$CMP12
BH_M_$CMP12:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.l (0xa,%fp),%d1
move.l #0x186a0,(0x20,%a5)
move.l #-0x80000000,(0x24,%a5)
move.l #0x7fffffff,(0x28,%a5)
move.l #0x186a0,(0x2c,%a5)
move.l #-0x80000000,(0x30,%a5)
move.l #0x7fffffff,(0x34,%a5)
move.w %d0,%d2
lsl.w #2,%d2
move.l %d1,(0x1c,%a5,%d2.w)
lea (0x20,%a5),%a0
lea (0x2c,%a5),%a1
moveq #2,%d0
.balignw 4,0x2048
3:
cmpm.l (%a1)+,(%a0)+
dbne %d0,3b
bne.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 9f
1:
subq.l #2,%sp
clr.w -(%sp)
9:
jsr (EXEC_$MARK).l
addq.w #4,%sp
4:
lea (0x20,%a5),%a0
lea (0x2c,%a5),%a1
moveq #2,%d0
.balignw 4,0x2048
7:
cmpm.l (%a1)+,(%a0)+
dbne %d0,7b
beq.s 5f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 10f
5:
subq.l #2,%sp
clr.w -(%sp)
10:
jsr (EXEC_$MARK).l
8:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BH_M_$LEA8
BH_M_$LEA8:
link.w %fp,#-4
movem.l %d2-%d4/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.l (0xa,%fp),%d3
lea (0x38,%a5),%a0
move.l %a0,(0x1e8,%a5)
jsr (EXEC_$CLOBBER).l
move.w %d2,%d4
movea.l (0x1e8,%a5),%a0
lsl.w #3,%d4
lea (0,%a0,%d4.w),%a0
lea (0x18c,%a0),%a0
move.l %d3,(%a0)
lea (0,%a5,%d4.w),%a0
move.l (0x1c4,%a0),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x14,%fp),%d2-%d4/%a5
unlk %fp
rts
.globl BH_M_$FAR
BH_M_$FAR:
link.w %fp,#-4
movem.l %d2-%d4/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
move.w %d2,%d4
add.w %d4,%d4
lea (0,%a5,%d4.w),%a0
move.w #7,(0x1ec,%a0)
lea (0x1ec,%a5),%a0
move.l %a0,%d0
subi.l #0x1ff80,%d0
movea.l %d0,%a2
lea (0,%a2,%d4.w),%a0
adda.l #0x1ff80,%a0
clr.l %d0
move.w (%a0),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w %d2,%d4
add.w %d4,%d4
lea (0,%a2,%d4.w),%a0
adda.l #0x1ff80,%a0
move.w %d3,(%a0)
lea (0,%a5,%d4.w),%a0
clr.l %d0
move.w (0x1ec,%a0),%d0
move.l %d0,-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0x18,%fp),%d2-%d4/%a2/%a5
unlk %fp
rts
.globl BH_M_$ADDC
BH_M_$ADDC:
link.w %fp,#-4
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
move.l %d0,(0x40,%a5)
move.l %d0,%d1
addq.l #1,%d1
move.l %d1,(0x48,%a5)
lea (0x38,%a5),%a0
move.l %a0,%d0
addi.l #8,%d0
movea.l %d0,%a2
move.l (%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
lea (0x38,%a5),%a0
move.l %a0,%d0
addi.l #0x10,%d0
movea.l %d0,%a2
move.l (%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
lea (0x50,%a5),%a0
move.l %a0,%d0
subi.l #0x10,%d0
movea.l %d0,%a2
move.l (%a2),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-0xc,%fp),%a2/%a5
unlk %fp
rts
.globl BH_M_$WIDE
BH_M_$WIDE:
link.w %fp,#0
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
move.w (0xa,%fp),%d3
move.w %d2,%d0
movea.l #WI_$R228,%a0
mulu.w #0xe4,%d0
move.l (0,%a0,%d0.l),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w %d3,%d0
movea.l #WI_$R24,%a0
ext.l %d0
lsl.l #3,%d0
move.l %d0,%d1
add.l %d1,%d1
add.l %d1,%d0
lea (0,%a0,%d0.l),%a1
move.l (0x5da8,%a1),-(%sp)
jsr (EXEC_$LOGV_L).l
movem.l (-8,%fp),%d2-%d3
unlk %fp
rts
.globl BH_M_$KEPT
BH_M_$KEPT:
link.w %fp,#-4
movem.l %d2-%d3,-(%sp)
move.w (8,%fp),%d2
movea.l #BH_$RECS,%a0
move.w %d2,%d3
add.w %d3,%d3
move.w %d3,%d0
add.w %d0,%d0
add.w %d0,%d3
move.w #5,(-6,%a0,%d3.w)
lea (0,%a0,%d3.w),%a0
move.w (-6,%a0),%d0
addq.w #1,%d0
move.w %d0,(-4,%a0)
move.w (-6,%a0),%d0
addq.w #2,%d0
move.w %d0,(-2,%a0)
subq.l #2,%sp
movea.l #BH_$RECS,%a0
move.w (-4,%a0,%d3.w),-(%sp)
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
subq.l #2,%sp
move.w %d2,%d0
movea.l #BH_$RECS,%a0
add.w %d0,%d0
move.w %d0,%d1
add.w %d1,%d1
add.w %d1,%d0
move.w (-2,%a0,%d0.w),-(%sp)
jsr (EXEC_$LOGV_W).l
movem.l (-0xc,%fp),%d2-%d3
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_H4BA_MEM","aw",@nobits
.balign 4
.Ldata:
.space 508
