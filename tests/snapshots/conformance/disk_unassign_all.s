.set __dpc_fragment_origin,0xe6be1c
.text
.globl DISK_$UNASSIGN_ALL
DISK_$UNASSIGN_ALL:
link.w %fp,#-8
movem.l %d2-%d3,-(%sp)
moveq #9,%d2
moveq #1,%d3
.balignw 4,0x2048
2:
move.w %d3,(-6,%fp)
pea (-4,%fp)
pea (-6,%fp)
bsr.w DISK_$UNASSIGN
addq.w #8,%sp
addq.w #1,%d3
dbf %d2,2b
movem.l (-0x10,%fp),%d2-%d3
unlk %fp
rts
