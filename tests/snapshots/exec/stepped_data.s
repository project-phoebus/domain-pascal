.text
.globl SDX_$SETUP
SDX_$SETUP:
link.w %fp,#-0xc
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w #8,(0x70,%a5)
moveq #2,%d0
move.l %d0,(%a5)
clr.l %d0
moveq #7,%d1
moveq #1,%d2
lea (0xc,%a5),%a0
.balignw 4,0x2048
2:
movea.l %a0,%a1
addi.l #0xa,%d0
move.w #7,(8,%a1)
moveq #0x64,%d3
move.l %d3,(4,%a1)
move.w %d2,(0xa,%a1)
move.l %d0,(0xc,%a1)
addq.w #1,%d2
lea (0xc,%a0),%a0
dbf %d1,2b
moveq #7,%d0
lea (0xc,%a5),%a0
.balignw 4,0x2048
5:
move.w (0xa,%a0),%d1
ext.l %d1
add.l (4,%a0),%d1
move.l %d1,(4,%a0)
lea (0xc,%a0),%a0
dbf %d0,5b
movea.l %a5,%a0
moveq #0x27,%d0
lea (0x14,%a0),%a0
.balignw 4,0x2048
8:
move.l (%a5),(0xa0,%a0)
lea (0x14,%a0),%a0
dbf %d0,8b
clr.l %d0
moveq #0xb,%d1
movea.l #SDX_$EXT,%a0
moveq #1,%d2
addq.l #6,%a0
.balignw 4,0x2048
11:
addi.l #0xd,%d0
move.l %d0,(-6,%a0)
move.w %d2,(-2,%a0)
addq.w #1,%d2
addq.l #6,%a0
dbf %d1,11b
movem.l (-0x18,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl SDX_$SUM
SDX_$SUM:
link.w %fp,#-0x10
movem.l %d2-%d3/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.l %d2
move.w (0x70,%a5),%d0
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d3
lea (0xc,%a5),%a2
.balignw 4,0x2048
2:
movea.l %a2,%a0
add.l (4,%a0),%d2
move.w (8,%a0),%d0
ext.l %d0
add.l %d0,%d2
add.l (0xc,%a0),%d2
jsr (EXEC_$CLOBBER).l
lea (0xc,%a2),%a2
dbf %d3,2b
1:
move.l %d2,%d0
movem.l (-0x20,%fp),%d2-%d3/%a2/%a5
unlk %fp
rts
.globl SDX_$FIND
SDX_$FIND:
link.w %fp,#-8
movem.l %d2-%d3/%a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
clr.w %d0
moveq #7,%d1
moveq #1,%d2
lea (0xc,%a5),%a1
.balignw 4,0x2048
2:
movea.l %a1,%a2
move.w (0xa,%a2),%d3
cmp.w (%a0),%d3
bne.s 4f
move.w %d2,%d0
bra.s 5f
4:
addq.w #1,%d2
lea (0xc,%a1),%a1
dbf %d1,2b
5:
movem.l (-0x18,%fp),%d2-%d3/%a2/%a5
unlk %fp
rts
.globl SDX_$CLEAR_FLAGS
SDX_$CLEAR_FLAGS:
link.w %fp,#-8
movem.l %d2/%a2-%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
move.w (0x70,%a5),%d0
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d2
lea (0xc,%a5),%a3
.balignw 4,0x2048
2:
movea.l %a3,%a4
move.w (0xa,%a4),%d0
cmp.w (%a2),%d0
bne.s 4f
move.l (4,%a4),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.w #1,(8,%a4)
bra.s 1f
4:
clr.w (8,%a4)
jsr (EXEC_$CLOBBER).l
lea (0xc,%a3),%a3
dbf %d2,2b
1:
movem.l (-0x1c,%fp),%d2/%a2-%a5
unlk %fp
rts
.globl SDX_$FAR
SDX_$FAR:
link.w %fp,#-0xc
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l %a5,%a0
moveq #0x25,%d0
lea (0x3c,%a0),%a0
.balignw 4,0x2048
2:
move.l (0xa0,%a0),%d1
add.l %d1,%d1
move.l %d1,%d2
lsl.l #2,%d2
add.l %d2,%d1
move.l %d1,(0xa0,%a0)
lea (0x14,%a0),%a0
dbf %d0,2b
clr.l %d0
movea.l %a5,%a0
moveq #0x27,%d1
lea (0x14,%a0),%a0
.balignw 4,0x2048
5:
add.l (0xa0,%a0),%d0
lea (0x14,%a0),%a0
dbf %d1,5b
movem.l (-0x14,%fp),%d2/%a5
unlk %fp
rts
.globl SDX_$FRAME
SDX_$FRAME:
link.w %fp,#-0xbc
pea (%a5)
lea (.Ldata).l,%a5
movea.l %fp,%a0
moveq #0x23,%d0
addq.l #4,%a0
.balignw 4,0x2048
2:
move.l (0x30,%a5),(-0xb4,%a0)
addq.l #4,%a0
dbf %d0,2b
moveq #7,%d0
lea (4,%fp),%a0
.balignw 4,0x2048
5:
move.l (0x1c,%a5),(-0x24,%a0)
addq.l #4,%a0
dbf %d0,5b
clr.l %d0
movea.l %fp,%a0
moveq #0x23,%d1
addq.l #4,%a0
.balignw 4,0x2048
8:
add.l (-0xb4,%a0),%d0
addq.l #4,%a0
dbf %d1,8b
moveq #7,%d1
lea (4,%fp),%a0
.balignw 4,0x2048
11:
add.l (-0x24,%a0),%d0
addq.l #4,%a0
dbf %d1,11b
movea.l (-0xc0,%fp),%a5
unlk %fp
rts
.globl SDX_$EXT_SUM
SDX_$EXT_SUM:
link.w %fp,#-0xc
move.l %d2,-(%sp)
clr.l %d0
movea.l #SDX_$EXT,%a0
moveq #0xb,%d1
addq.l #6,%a0
.balignw 4,0x2048
2:
add.l (-6,%a0),%d0
addq.l #6,%a0
dbf %d1,2b
movea.l #SDX_$EXT,%a0
moveq #0xb,%d1
addq.l #6,%a0
.balignw 4,0x2048
5:
move.w (-2,%a0),%d2
addq.l #6,%a0
ext.l %d2
sub.l %d2,%d0
dbf %d1,5b
move.l (-0x10,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SDX","aw",@nobits
.balign 4
.Ldata:
.space 980
