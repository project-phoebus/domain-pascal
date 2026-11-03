.set __dpc_fragment_origin,0xe1a7e8
.section ".text.IO_.WIRED_PROC","ax",@progbits
.globl IO_$DISABLE_DEVICE
IO_$DISABLE_DEVICE:
link.w %fp,#-4
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata.WIRED_DATA).l,%a5
subq.l #2,%sp
move.w (8,%fp),%d2
move.l (0xa,%fp),-(%sp)
move.w %d2,-(%sp)
bsr.w IO_$CHECK_UNIT
addq.w #8,%sp
tst.b %d0
bpl.s 1f
move.w %d2,%d0
lsl.w #5,%d0
lea (0,%a5,%d0.w),%a3
move.w (0x12e,%a3),%d0
lsl.w #2,%d0
move.w %d0,%d1
lsl.w #3,%d1
add.w %d1,%d0
lea (0,%a5,%d0.w),%a2
tst.l (0x74,%a2)
beq.s 2f
move.l (0x118,%a3),-(%sp)
movea.l (0x74,%a2),%a0
jsr (%a0)
2:
1:
movem.l (-0x14,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
.globl IO_$ENABLE_DEVICE
IO_$ENABLE_DEVICE:
link.w %fp,#-4
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata.WIRED_DATA).l,%a5
subq.l #2,%sp
move.w (8,%fp),%d2
move.l (0xa,%fp),-(%sp)
move.w %d2,-(%sp)
bsr.w IO_$CHECK_UNIT
addq.w #8,%sp
tst.b %d0
bpl.s 1f
move.w %d2,%d0
lsl.w #5,%d0
lea (0,%a5,%d0.w),%a3
move.w (0x12e,%a3),%d0
lsl.w #2,%d0
move.w %d0,%d1
lsl.w #3,%d1
add.w %d1,%d0
lea (0,%a5,%d0.w),%a2
tst.l (0x70,%a2)
beq.s 2f
move.l (0x118,%a3),-(%sp)
movea.l (0x70,%a2),%a0
jsr (%a0)
2:
1:
movem.l (-0x14,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
.section ".bss.moddata.IO_.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 1304
