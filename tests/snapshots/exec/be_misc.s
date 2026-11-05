.text
.globl BE_MISC_$DIV
BE_MISC_$DIV:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
move.w (0xc,%fp),%d2
move.w %d0,(%a5)
move.w %d2,(8,%a5)
move.w %d1,(4,%a5)
ext.l %d0
ext.l %d1
add.l %d1,%d0
divs.w (8,%a5),%d0
ext.l %d0
move.l %d0,(0xc,%a5)
pea (0xc,%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
move.w (%a5),%d0
move.w (4,%a5),%d1
ext.l %d0
ext.l %d1
subq.l #2,%sp
add.l %d1,%d0
move.w (8,%a5),-(%sp)
move.l %d0,-(%sp)
jsr (M$OIS$WLW).l
addq.w #8,%sp
ext.l %d0
move.l %d0,(0xc,%a5)
pea (0xc,%a5)
jsr (EXEC_$LOG_L).l
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl BE_MISC_$ADDR
BE_MISC_$ADDR:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (8,%fp),%d0
lea (ADDRV_$DRV+0xa).l,%a0
move.l %a0,(0x10,%a5)
move.l %d0,(%a0)
pea (ADDRV_$DRV+0xa).l
jsr (EXEC_$LOG_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BE_MISC_$ODD
BE_MISC_$ODD:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(2,%a5)
btst #0,%d0
sne %d0
move.b %d0,(0x14,%a5)
pea (0x14,%a5)
jsr (EXEC_$LOG_BOOL).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_MISC","aw",@nobits
.balign 4
.Ldata:
.space 24
