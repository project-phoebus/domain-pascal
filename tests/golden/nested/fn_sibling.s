.text
GET:
link.w %fp,#-8
pea (%a2)
movea.l %a1,%a2
subq.l #2,%sp
move.w (8,%fp),-(%sp)
jsr (EXT_$F).l
move.w %d0,%d1
add.w (-2,%a2),%d1
move.w %d1,%d0
movea.l (-0xc,%fp),%a2
unlk %fp
rts
USE:
link.w %fp,#-4
pea (%a2)
subq.l #2,%sp
movea.l (%fp),%a2
move.w #2,-(%sp)
movea.l %a2,%a1
bsr.s GET
move.w %d0,(-4,%a2)
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl NFS_$SIB
NFS_$SIB:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w %d0,(-2,%fp)
bsr.s USE
subq.l #2,%sp
move.w #5,-(%sp)
movea.l %fp,%a1
bsr.s GET
addq.w #4,%sp
move.w %d0,(-4,%fp)
subq.l #2,%sp
move.w %d0,-(%sp)
jsr (EXT_$P).l
unlk %fp
rts
.balignw 4,0x2048
