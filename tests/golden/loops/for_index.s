.text
.globl FLI_$ALL
FLI_$ALL:
link.w %fp,#-4
movem.l %d2-%d3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (2,%a5),%d0
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
move.w %d3,(-2,%fp)
pea (-2,%fp)
jsr (EXT_$PUT).l
addq.w #4,%sp
addq.w #1,%d3
dbf %d2,2b
1:
movem.l (-0x10,%fp),%d2-%d3/%a5
unlk %fp
rts
.globl FLI_$UNASSIGN_ALL
FLI_$UNASSIGN_ALL:
link.w %fp,#-8
movem.l %d2-%d3,-(%sp)
moveq #9,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
move.w %d3,(-6,%fp)
pea (-4,%fp)
pea (-6,%fp)
jsr (EXT_$UNASSIGN).l
addq.w #8,%sp
addq.w #1,%d3
dbf %d2,2b
movem.l (-0x10,%fp),%d2-%d3
unlk %fp
rts
.globl FLI_$ALLOC
FLI_$ALLOC:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
moveq #0x45,%d1
movea.l #PGROUPS,%a0
moveq #1,%d0
addq.l #8,%a0
.balignw 4,0x2048
2:
tst.w (-8,%a0)
beq.s 1f
addq.w #1,%d0
addq.l #8,%a0
dbf %d1,2b
1:
cmpi.w #0x46,%d0
ble.s 4f
move.l (8,%fp),-(%sp)
jsr (CRASH_SYSTEM).l
bra.s 5f
4:
move.w %d0,(0x13a,%a5)
5:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl FLI_$CLASSES
FLI_$CLASSES:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #0x11,%d0
clr.w %d1
movea.l %a5,%a0
.balignw 4,0x2048
2:
clr.w %d2
move.b (4,%a5,%d1.w),%d2
add.w %d2,%d2
move.w (0x116,%a0),(0x16,%a5,%d2.w)
addq.w #1,%d1
addq.l #2,%a0
dbf %d0,2b
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.FLI","aw",@nobits
.balign 4
.Ldata:
.space 316
