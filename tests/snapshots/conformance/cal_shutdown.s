.set __dpc_fragment_origin,0xe3e6e4
.text
.globl CAL_$SHUTDOWN
CAL_$SHUTDOWN:
link.w %fp,#-4
movem.l %a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a2
subq.l #2,%sp
pea (%a2)
clr.l -(%sp)
clr.l -(%sp)
move.l #LV_LABEL_$UID,-(%sp)
clr.l -(%sp)
move.w (0x10,%a5),-(%sp)
jsr (DBUF_$GET_BLOCK).l
lea (0x18,%sp),%sp
movea.l %a0,%a3
tst.l (%a2)
bne.s 1f
move.l (TIME_$CLOCKH).l,(0xb0,%a3)
move.l (0xb0,%a3),(0xe6,%a3)
subq.l #2,%sp
pea (%a2)
move.w #0xb,-(%sp)
pea (%a3)
jsr (DBUF_$SET_BUFF).l
1:
movem.l (-0x10,%fp),%a2-%a3/%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.OS_CAL_WIRED","aw",@nobits
.balign 4
.Ldata:
.space 20
