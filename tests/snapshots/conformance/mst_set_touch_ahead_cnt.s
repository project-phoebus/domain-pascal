.set __dpc_fragment_origin,0xe446fc
.text
.globl MST_$SET_TOUCH_AHEAD_CNT
MST_$SET_TOUCH_AHEAD_CNT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (0x18,%fp),-(%sp)
move.l (0x14,%fp),-(%sp)
move.l (0x10,%fp),-(%sp)
move.l (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
pea (.Lc0,%pc)
bsr.w MST_$PRIV_SET_TOUCH_AHEAD_CNT
movea.l (-4,%fp),%a5
unlk %fp
rts
.Lc0:
.short 0x0000
.section ".bss.moddata.MST_","aw",@nobits
.balign 4
.Ldata:
.space 4
