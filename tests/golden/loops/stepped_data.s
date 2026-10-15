.text
.globl SD_$INVALIDATE
SD_$INVALIDATE:
link.w %fp,#-8
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (0x70,%a5),%d0
subq.w #1,%d0
bmi.s 1f
move.w %d0,%d2
lea (0xc,%a5),%a2
.balignw 4,0x2048
2:
movea.l %a2,%a3
tst.l (0xc,%a3)
bne.s 4f
move.l (4,%a3),-(%sp)
jsr (EXT_$PUT).l
addq.w #4,%sp
clr.w (8,%a3)
4:
lea (0xc,%a2),%a2
dbf %d2,2b
1:
movem.l (-0x18,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
.globl SD_$FIND
SD_$FIND:
link.w %fp,#-8
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
clr.w %d0
moveq #7,%d1
moveq #1,%d2
lea (0xc,%a5),%a0
.balignw 4,0x2048
2:
movea.l %a0,%a1
cmpi.w #1,(0xa,%a1)
bne.s 4f
move.w %d2,%d0
bra.s 5f
4:
addq.w #1,%d2
lea (0xc,%a0),%a0
dbf %d1,2b
5:
movem.l (-0x10,%fp),%d2/%a5
unlk %fp
rts
.globl SD_$CLEAR
SD_$CLEAR:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l %a5,%a0
moveq #0x27,%d0
lea (0x14,%a0),%a0
.balignw 4,0x2048
2:
move.l (%a5),(0xa0,%a0)
lea (0x14,%a0),%a0
dbf %d0,2b
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl SD_$FRAME
SD_$FRAME:
link.w %fp,#-0xb4
pea (%a5)
lea (.Ldata).l,%a5
movea.l %fp,%a0
moveq #0x23,%d0
addq.l #4,%a0
.balignw 4,0x2048
2:
move.l (4,%a5),(-0xb4,%a0)
addq.l #4,%a0
dbf %d0,2b
moveq #7,%d0
lea (4,%fp),%a0
.balignw 4,0x2048
5:
move.l (8,%a5),(-0x24,%a0)
addq.l #4,%a0
dbf %d0,5b
move.l (-0xb0,%fp),%d0
add.l (-0x20,%fp),%d0
move.l %d0,-(%sp)
jsr (EXT_$PUT).l
movea.l (-0xb8,%fp),%a5
unlk %fp
rts
.globl SD_$INIT_EXT
SD_$INIT_EXT:
link.w %fp,#-4
movem.l %d2-%d3/%a2,-(%sp)
moveq #0x3f,%d2
movea.l #EXT_TAB,%a2
moveq #1,%d3
lea (0xc,%a2),%a2
.balignw 4,0x2048
2:
pea (-0xc,%a2)
jsr (EXT_$INIT).l
addq.w #4,%sp
move.w %d3,(-6,%a2)
addq.w #1,%d3
lea (0xc,%a2),%a2
dbf %d2,2b
movem.l (-0x10,%fp),%d2-%d3/%a2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.SD","aw",@nobits
.balign 4
.Ldata:
.space 980
