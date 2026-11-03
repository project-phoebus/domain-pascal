.section ".text.BE_X_SECT.BX_SECT_PROC","ax",@progbits
.globl BX_SECT_$SET
BX_SECT_$SET:
link.w %fp,#0
pea (%a5)
lea (.Ldata.BX_SECT_DATA).l,%a5
movea.l (8,%fp),%a0
move.l (%a0),(%a5)
move.l (%a0),%d0
addq.l #1,%d0
move.l %d0,(4,%a5)
clr.w (8,%a5)
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl BX_SECT_$SHOW
BX_SECT_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata.BX_SECT_DATA).l,%a5
pea (0xa,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (0xc,%a5)
jsr (EXEC_$LOG_W).l
addq.w #4,%sp
pea (0xe,%a5)
jsr (EXEC_$LOG_W).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".text.BE_X_SECT.BX_SECT_INIT_PROC","ax",@progbits
.globl BX_SECT_$BRANCHY
BX_SECT_$BRANCHY:
link.w %fp,#0
movea.l #.Ldata.BX_SECT_DATA,%a0
moveq #7,%d0
move.l %d0,(%a0)
movea.l (8,%fp),%a1
tst.w (%a1)
ble.s 1f
moveq #9,%d0
move.l %d0,(4,%a0)
bra.s 2f
1:
movea.l #.Ldata.BX_SECT_DATA,%a0
move.w #3,(8,%a0)
2:
movea.l #.Ldata.BX_SECT_DATA,%a0
addq.l #1,(%a0)
move.l #.Ldata.BX_SECT_DATA,-(%sp)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
movea.l #.Ldata.BX_SECT_DATA,%a0
pea (4,%a0)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
movea.l #.Ldata.BX_SECT_DATA,%a0
pea (8,%a0)
jsr (EXEC_$LOG_W).l
unlk %fp
rts
.globl BX_SECT_$ELEM
BX_SECT_$ELEM:
link.w %fp,#0
movea.l #.Ldata.BX_SECT_DATA,%a0
move.w (8,%fp),%d0
move.w %d0,%d1
add.w %d1,%d1
move.w (0xa,%fp),(8,%a0,%d1.w)
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_X_SECT.BX_SECT_DATA","aw",@nobits
.balign 4
.Ldata.BX_SECT_DATA:
.space 20
