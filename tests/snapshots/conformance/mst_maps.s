.set __dpc_fragment_origin,0xe4386c
.text
.globl MST_$MAP
MST_$MAP:
link.w %fp,#-4
movem.l %a2-%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.l (0x24,%fp),-(%sp)
move.l (0x20,%fp),-(%sp)
clr.w -(%sp)
movea.l (0x1c,%fp),%a0
move.b (%a0),-(%sp)
move.w (MST_$TOUCH_COUNT).l,-(%sp)
movea.l (0x14,%fp),%a1
move.w (%a1),-(%sp)
move.w (PROC1_$AS_ID).l,-(%sp)
movea.l (0x18,%fp),%a2
move.l (%a2),-(%sp)
movea.l (0x10,%fp),%a3
move.l (%a3),-(%sp)
movea.l (0xc,%fp),%a4
move.l (%a4),-(%sp)
move.l (8,%fp),-(%sp)
move.l #0x7fffffff,-(%sp)
bsr.w ALLOC_SEGS
move.l %a0,(-4,%fp)
movem.l (-0x14,%fp),%a2-%a5
unlk %fp
rts
.globl MST_$MAP_TOP
MST_$MAP_TOP:
link.w %fp,#-4
movem.l %a2-%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.l (0x24,%fp),-(%sp)
move.l (0x20,%fp),-(%sp)
clr.w -(%sp)
movea.l (0x1c,%fp),%a0
move.b (%a0),-(%sp)
move.w (MST_$TOUCH_COUNT).l,-(%sp)
movea.l (0x14,%fp),%a1
move.w (%a1),-(%sp)
move.w (PROC1_$AS_ID).l,-(%sp)
movea.l (0x18,%fp),%a2
move.l (%a2),-(%sp)
movea.l (0x10,%fp),%a3
move.l (%a3),-(%sp)
movea.l (0xc,%fp),%a4
move.l (%a4),-(%sp)
move.l (8,%fp),-(%sp)
move.l #0x7fffffff,-(%sp)
bsr.w ALLOC_SEGS
move.l %a0,(-4,%fp)
movem.l (-0x14,%fp),%a2-%a5
unlk %fp
rts
.globl MST_$MAP_GLOBAL
MST_$MAP_GLOBAL:
link.w %fp,#-4
movem.l %a2-%a5,-(%sp)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.l (0x24,%fp),-(%sp)
move.l (0x20,%fp),-(%sp)
clr.w -(%sp)
movea.l (0x1c,%fp),%a0
move.b (%a0),-(%sp)
move.w #1,-(%sp)
movea.l (0x14,%fp),%a1
move.w (%a1),-(%sp)
clr.w -(%sp)
movea.l (0x18,%fp),%a2
move.l (%a2),-(%sp)
movea.l (0x10,%fp),%a3
move.l (%a3),-(%sp)
movea.l (0xc,%fp),%a4
move.l (%a4),-(%sp)
move.l (8,%fp),-(%sp)
clr.l -(%sp)
bsr.w ALLOC_SEGS
move.l %a0,(-4,%fp)
movem.l (-0x14,%fp),%a2-%a5
unlk %fp
rts
.globl MST_$MAPS
MST_$MAPS:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.l (0x24,%fp),-(%sp)
move.l (0x20,%fp),-(%sp)
move.b (0xa,%fp),-(%sp)
move.b (0x1e,%fp),-(%sp)
move.w (MST_$TOUCH_COUNT).l,-(%sp)
move.w (0x18,%fp),-(%sp)
move.w (8,%fp),-(%sp)
move.l (0x1a,%fp),-(%sp)
move.l (0x14,%fp),-(%sp)
move.l (0x10,%fp),-(%sp)
move.l (0xc,%fp),-(%sp)
move.l #0x7fffffff,-(%sp)
bsr.w ALLOC_SEGS
move.l %a0,(-4,%fp)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl MST_$REMAP
MST_$REMAP:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (0x1c,%fp),-(%sp)
move.l (0x18,%fp),-(%sp)
move.l (0x14,%fp),-(%sp)
move.l (0x10,%fp),-(%sp)
move.l (0xc,%fp),-(%sp)
move.l (8,%fp),-(%sp)
pea (.Lc0,%pc)
.short 0x610e
move.l %a0,(-4,%fp)
movea.l (-8,%fp),%a5
unlk %fp
rts
.Lc0:
.short 0x0000
.section ".bss.moddata.MST_","aw",@nobits
.balign 4
.Ldata:
.space 4
