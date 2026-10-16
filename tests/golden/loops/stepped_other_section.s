.section ".text.HINT.OS_INIT_PROC","ax",@progbits
.globl HINT_$INIT_CACHE
HINT_$INIT_CACHE:
link.w %fp,#-4
movea.l #.Ldata.WIRED_DATA,%a0
moveq #1,%d0
lea (0xc,%a0),%a0
.balignw 4,0x2048
2:
clr.l (-4,%a0)
clr.b (-8,%a0)
clr.l (-0xc,%a0)
lea (0xc,%a0),%a0
dbf %d0,2b
unlk %fp
rts
.globl HINT_$INIT_Q
HINT_$INIT_Q:
link.w %fp,#-4
movem.l %d2-%d3/%a2,-(%sp)
moveq #0x3f,%d2
movea.l #.Ldata.WIRED_DATA,%a2
moveq #1,%d3
lea (0xc,%a2),%a2
.balignw 4,0x2048
2:
pea (0x10a8,%a2)
move.w %d3,-(%sp)
st -(%sp)
jsr (Q_$INIT_QUEUE).l
addq.w #8,%sp
addq.w #1,%d3
lea (0xc,%a2),%a2
dbf %d2,2b
movem.l (-0x10,%fp),%d2-%d3/%a2
unlk %fp
rts
.globl HINT_$INIT_BUS
HINT_$INIT_BUS:
link.w %fp,#-4
moveq #4,%d0
movea.l #.Ldata.WIRED_DATA,%a0
.balignw 4,0x2048
2:
clr.l (0x60,%a0)
lea (0x24,%a0),%a0
dbf %d0,2b
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.HINT.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 5044
