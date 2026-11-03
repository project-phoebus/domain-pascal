.text
FORNC_$FIND_IN:
link.w %fp,#-0xc
move.l %d2,-(%sp)
move.l (8,%fp),%d0
move.w (0x10,%fp),%d1
move.w %d1,%d2
subq.w #1,%d2
bmi.s 1f
move.w %d2,%d1
clr.w %d2
movea.l (0xc,%fp),%a0
lea (%a0),%a1
.balignw 4,0x2048
2:
movea.l %a1,%a0
cmp.l (%a0),%d0
blt.s 4f
cmp.l (4,%a0),%d0
bgt.s 4f
move.w %d2,%d0
bra.s 5f
4:
addq.w #1,%d2
lea (0xc,%a1),%a1
dbf %d1,2b
1:
moveq #-1,%d0
5:
move.l (-0x10,%fp),%d2
unlk %fp
rts
FORNC_$FIND1_IN:
link.w %fp,#-0xc
movem.l %d2-%d3,-(%sp)
move.l (8,%fp),%d0
move.l (0xc,%fp),%d1
move.w (0x14,%fp),%d2
move.w %d2,%d3
subq.w #1,%d3
bmi.s 1f
move.w %d3,%d2
moveq #1,%d3
movea.l (0x10,%fp),%a0
lea (0xc,%a0),%a1
.balignw 4,0x2048
2:
movea.l %a1,%a0
cmp.l (-8,%a0),%d1
bgt.s 4f
cmp.l (-0xc,%a0),%d0
blt.s 4f
move.w %d3,%d0
bra.s 5f
4:
addq.w #1,%d3
lea (0xc,%a1),%a1
dbf %d2,2b
1:
clr.w %d0
5:
movem.l (-0x14,%fp),%d2-%d3
unlk %fp
rts
.globl FORNC_$SETUP
FORNC_$SETUP:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.w %d0
bra.s 2f
.balignw 4,0x2048
1:
move.w %d0,%d1
ext.l %d1
add.l %d1,%d1
move.l %d1,%d2
lsl.l #2,%d2
add.l %d2,%d1
move.w %d0,%d2
lsl.w #2,%d2
move.w %d2,%d3
add.w %d3,%d3
add.w %d3,%d2
move.l %d1,(0,%a5,%d2.w)
move.w %d0,%d1
ext.l %d1
add.l %d1,%d1
move.l %d1,%d2
lsl.l #2,%d2
add.l %d2,%d1
move.w %d0,%d2
addq.l #5,%d1
lsl.w #2,%d2
move.w %d2,%d3
add.w %d3,%d3
add.w %d3,%d2
move.l %d1,(4,%a5,%d2.w)
move.w %d0,%d1
ext.l %d1
addq.l #1,%d1
add.l %d1,%d1
move.l %d1,%d2
lsl.l #2,%d2
add.l %d2,%d1
move.w %d0,%d2
lsl.w #2,%d2
move.w %d2,%d3
add.w %d3,%d3
add.w %d3,%d2
lea (0,%a5,%d2.w),%a0
move.l %d1,(0xf0,%a0)
move.w %d0,%d1
ext.l %d1
addq.l #1,%d1
add.l %d1,%d1
move.l %d1,%d2
lsl.l #2,%d2
add.l %d2,%d1
move.w %d0,%d2
addq.l #5,%d1
lsl.w #2,%d2
move.w %d2,%d3
add.w %d3,%d3
add.w %d3,%d2
lea (0,%a5,%d2.w),%a0
move.l %d1,(0xf4,%a0)
addq.w #1,%d0
2:
cmpi.w #0x14,%d0
blt.s 1b
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FORNC_$FIND
FORNC_$FIND:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
pea (%a5)
move.l (0xa,%fp),-(%sp)
bsr.w FORNC_$FIND_IN
lea (0xc,%sp),%sp
move.w %d0,(-2,%fp)
subq.l #2,%sp
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FORNC_$FIND1
FORNC_$FIND1:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (0xa,%fp),%d2
subq.l #2,%sp
move.l %d2,%d0
move.w (8,%fp),-(%sp)
addq.l #1,%d0
pea (0xf0,%a5)
move.l %d0,-(%sp)
move.l %d2,-(%sp)
bsr.w FORNC_$FIND1_IN
lea (0x10,%sp),%sp
move.w %d0,(-2,%fp)
subq.l #2,%sp
move.w %d0,-(%sp)
jsr (EXEC_$MARK).l
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
FORNC_$FILL_IN:
link.w %fp,#-0x18
move.l %d2,-(%sp)
move.w (8,%fp),%d0
move.l (0xa,%fp),%d1
movea.l (0xe,%fp),%a0
clr.l (-0x18,%fp)
clr.l (-0x14,%fp)
clr.l (-0x10,%fp)
clr.l (-0xc,%fp)
move.w %d0,%d2
subq.w #1,%d2
bmi.s 1f
move.w %d2,%d0
lea (4,%fp),%a1
.balignw 4,0x2048
2:
move.l %d1,(-0x1c,%a1)
addq.l #4,%a1
dbf %d0,2b
1:
move.l (-0x18,%fp),(%a0)
move.l (-0x14,%fp),(0xc,%a0)
move.l (-0x10,%fp),(0x18,%a0)
move.l (-0xc,%fp),(0x24,%a0)
move.l (-0x1c,%fp),%d2
unlk %fp
rts
.globl FORNC_$FILLS
FORNC_$FILLS:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
subq.l #2,%sp
pea (0xf0,%a5)
move.l (0xa,%fp),-(%sp)
move.w %d2,-(%sp)
bsr.s FORNC_$FILL_IN
lea (0xc,%sp),%sp
move.l (0xf0,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
cmpi.w #1,%d2
blt.s 1f
move.l (0xfc,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
1:
cmpi.w #2,%d2
blt.s 2f
move.l (0x108,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
2:
cmpi.w #3,%d2
blt.s 3f
move.l (0x114,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
3:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FORNC","aw",@nobits
.balign 4
.Ldata:
.space 480
