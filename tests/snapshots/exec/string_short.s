.text
LOG:
link.w %fp,#-4
move.b (8,%fp),%d0
bpl.s 1f
pea (.Lc0,%pc)
bra.s 3f
1:
pea (.Lc1,%pc)
3:
jsr (EXEC_$LOG_L).l
2:
unlk %fp
rts
.balignw 4,0x2048
.Lc0:
.short 0x0000,0x0001
.Lc1:
.short 0x0000,0x0000
.globl THROUGH
THROUGH:
link.w %fp,#0
lea (.Lc2,%pc),%a0
movea.l (8,%fp),%a1
move.b (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
move.b (%a0)+,(%a1)+
unlk %fp
rts
.balignw 4,0x2048
.Lc2:
.short 0x7778,0x797a
.globl STRSHORT_$RUN
STRSHORT_$RUN:
link.w %fp,#-0x18
pea (%a5)
lea (.Ldata).l,%a5
move.w #0x6162,(%a5)
subq.l #2,%sp
cmpi.b #0x61,(%a5)
seq %d0
cmpi.b #0x62,(1,%a5)
seq %d1
and.b %d1,%d0
move.b %d0,-(%sp)
bsr.s LOG
addq.w #4,%sp
move.l #0x63642020,(2,%a5)
subq.l #2,%sp
cmpi.b #0x63,(2,%a5)
seq %d0
cmpi.b #0x64,(3,%a5)
seq %d1
and.b %d1,%d0
cmpi.b #0x20,(4,%a5)
seq %d1
and.b %d1,%d0
cmpi.b #0x20,(5,%a5)
seq %d1
and.b %d1,%d0
move.b %d0,-(%sp)
bsr.w LOG
addq.w #4,%sp
move.l #0x53657665,(6,%a5)
move.w #0x6e20,(0xa,%a5)
subq.l #2,%sp
cmpi.b #0x53,(6,%a5)
seq %d0
cmpi.b #0x65,(9,%a5)
seq %d1
and.b %d1,%d0
cmpi.b #0x6e,(0xa,%a5)
seq %d1
and.b %d1,%d0
cmpi.b #0x20,(0xb,%a5)
seq %d1
and.b %d1,%d0
move.b %d0,-(%sp)
bsr.w LOG
addq.w #4,%sp
move.w #0x1234,(-8,%fp)
move.w #0x5678,(-2,%fp)
move.l #0x65666768,(-6,%fp)
subq.l #2,%sp
cmpi.b #0x65,(-6,%fp)
seq %d0
cmpi.b #0x68,(-3,%fp)
seq %d1
and.b %d1,%d0
cmpi.w #0x1234,(-8,%fp)
seq %d1
and.b %d1,%d0
cmpi.w #0x5678,(-2,%fp)
seq %d1
and.b %d1,%d0
move.b %d0,-(%sp)
bsr.w LOG
addq.w #4,%sp
move.w #0x6920,(-0xa,%fp)
subq.l #2,%sp
cmpi.b #0x69,(-0xa,%fp)
seq %d0
cmpi.b #0x20,(-9,%fp)
seq %d1
and.b %d1,%d0
move.b %d0,-(%sp)
bsr.w LOG
addq.w #4,%sp
move.w #0x1234,(-0x18,%fp)
move.w #0x5678,(-0x12,%fp)
pea (-0x16,%fp)
bsr.w THROUGH
addq.w #4,%sp
subq.l #2,%sp
cmpi.b #0x77,(-0x16,%fp)
seq %d0
cmpi.b #0x7a,(-0x13,%fp)
seq %d1
and.b %d1,%d0
cmpi.w #0x1234,(-0x18,%fp)
seq %d1
and.b %d1,%d0
cmpi.w #0x5678,(-0x12,%fp)
seq %d1
and.b %d1,%d0
move.b %d0,-(%sp)
bsr.w LOG
movea.l (-0x1c,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.STRSHORT","aw",@nobits
.balign 4
.Ldata:
.space 12
