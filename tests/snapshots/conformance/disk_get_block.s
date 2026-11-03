.set __dpc_fragment_origin,0xe3bb80
.text
.globl DISK_$GET_BLOCK
DISK_$GET_BLOCK:
link.w %fp,#-4
pea (%a2)
subq.l #2,%sp
move.w #0xf,-(%sp)
jsr (ML_$LOCK).l
addq.w #4,%sp
subq.l #2,%sp
move.l (0x1a,%fp),-(%sp)
move.w (0x18,%fp),-(%sp)
move.w (0x16,%fp),-(%sp)
move.l (0x12,%fp),-(%sp)
move.l (0xe,%fp),-(%sp)
move.l (0xa,%fp),-(%sp)
move.w (8,%fp),-(%sp)
jsr (DBUF_$GET_BLOCK).l
lea (0x18,%sp),%sp
movea.l %a0,%a2
subq.l #2,%sp
move.w #0xf,-(%sp)
jsr (ML_$UNLOCK).l
movea.l %a2,%a0
movea.l (-8,%fp),%a2
unlk %fp
rts
