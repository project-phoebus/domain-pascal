.text
.globl PBU_$RELEASE_INT
PBU_$RELEASE_INT:
link.w %fp,#0
move.l (0xc,%fp),-(%sp)
pea (.Lc0,%pc)
move.l (8,%fp),-(%sp)
jsr (PBU_$SETDOWN).l
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x0000
.globl PBU_$SHUTDOWN
PBU_$SHUTDOWN:
link.w %fp,#0
movem.l %a2-%a3,-(%sp)
movea.l (8,%fp),%a2
movea.l (0xc,%fp),%a3
pea (%a3)
pea (.Lc0,%pc)
pea (%a2)
jsr (PBU_$SETDOWN).l
lea (0xc,%sp),%sp
pea (.Lc0,%pc)
jsr (PBU_$COUNT).l
addq.w #4,%sp
pea (%a3)
pea (.Lc0,%pc)
pea (%a2)
jsr (PBU_$SETDOWN).l
movem.l (-8,%fp),%a2-%a3
unlk %fp
rts
.balignw 4,0x2048
