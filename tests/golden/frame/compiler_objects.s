.text
.globl PKT_$NEXT_LONG_ID
PKT_$NEXT_LONG_ID:
link.w %fp,#-8
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
pea (%a5)
jsr (ML_$SPIN_LOCK).l
addq.w #4,%sp
move.w %d0,(-6,%fp)
move.l (4,%a5),%d2
addq.l #1,(4,%a5)
subq.l #2,%sp
move.w %d0,-(%sp)
pea (%a5)
jsr (ML_$SPIN_UNLOCK).l
move.l %d2,%d0
movem.l (-0x10,%fp),%d2/%a5
unlk %fp
rts
.globl PKT_$NEXT_ID
PKT_$NEXT_ID:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
pea (%a5)
jsr (ML_$SPIN_LOCK).l
addq.w #4,%sp
move.w %d0,(-2,%fp)
move.w (8,%a5),%d2
addq.w #1,(8,%a5)
subq.l #2,%sp
move.w %d0,-(%sp)
pea (%a5)
jsr (ML_$SPIN_UNLOCK).l
move.w %d2,%d0
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl MSG_$SEND
MSG_$SEND:
link.w %fp,#-0x34
movem.l %a2-%a4,-(%sp)
movea.l (8,%fp),%a0
move.l (%a0),(-0x28,%fp)
movea.l (0xc,%fp),%a1
move.w (%a1),(-0x34,%fp)
movea.l (0x10,%fp),%a2
move.w (%a2),(-0x32,%fp)
movea.l (0x14,%fp),%a3
move.w (%a3),(-0x30,%fp)
movea.l (0x18,%fp),%a4
move.w (%a4),(-0x2e,%fp)
movea.l (0x1c,%fp),%a0
move.w (%a0),(-0x2c,%fp)
movea.l (0x20,%fp),%a1
move.w (%a1),(-0x2a,%fp)
pea (-0x24,%fp)
move.w (-0x2a,%fp),-(%sp)
move.w (-0x2c,%fp),-(%sp)
move.w (-0x2e,%fp),-(%sp)
move.w (-0x30,%fp),-(%sp)
move.w (-0x32,%fp),-(%sp)
move.w (-0x34,%fp),-(%sp)
move.l (-0x28,%fp),-(%sp)
pea (-0x20,%fp)
jsr (MSG_$SENDI).l
movem.l (-0x40,%fp),%a2-%a4
unlk %fp
rts
.globl XPD_$SET_PTRACE_OPTS
XPD_$SET_PTRACE_OPTS:
link.w %fp,#-0x1c
movea.l (8,%fp),%a0
move.l (%a0)+,(-0x18,%fp)
move.l (%a0)+,(-0x14,%fp)
movea.l (0xc,%fp),%a0
lea (-0x10,%fp),%a1
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.l (%a0)+,(%a1)+
move.w (%a0)+,(%a1)+
clr.l (-0x1c,%fp)
pea (-0x1c,%fp)
pea (-0x10,%fp)
pea (-0x18,%fp)
jsr (XPD_$TAKE).l
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.COMPILER_OBJECTS","aw",@nobits
.balign 4
.Ldata:
.space 12
