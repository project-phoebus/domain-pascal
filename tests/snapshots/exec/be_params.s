.text
.globl BP_$CMPMEM
BP_$CMPMEM:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d1
move.w (0xa,%fp),%d0
move.w %d1,(%a5)
move.w %d0,%d2
addq.w #1,%d2
cmp.w %d2,%d1
bge.s 1f
moveq #1,%d0
bra.s 2f
1:
clr.w %d0
2:
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl BP_$NYS
BP_$NYS:
link.w %fp,#-4
movea.l (8,%fp),%a0
clr.w %d0
cmpa.w #0,%a0
beq.s 1f
move.w (%a0),%d0
addq.w #1,%d0
1:
unlk %fp
rts
.globl BP_$NYS_T
BP_$NYS_T:
link.w %fp,#-8
move.l %d2,-(%sp)
move.w (8,%fp),%d0
move.w %d0,(-2,%fp)
pea (-2,%fp)
bsr.s BP_$NYS
addq.w #4,%sp
move.w %d0,%d2
clr.l -(%sp)
bsr.s BP_$NYS
move.w %d0,%d1
move.w %d2,%d0
add.w %d1,%d0
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl BP_$R0AG
BP_$R0AG:
link.w %fp,#-4
move.w (8,%fp),%d0
move.w (0xa,%fp),%d1
cmpi.w #3,%d1
ble.s 1f
moveq #7,%d0
1:
add.w %d1,%d0
unlk %fp
rts
.globl BP_$M2W3
BP_$M2W3:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.l %a0,(4,%a5)
move.w (%a0),%d0
ext.l %d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl BP_$M2W3_T
BP_$M2W3_T:
link.w %fp,#-8
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,(-6,%fp)
pea (-6,%fp)
bsr.s BP_$M2W3
addq.w #4,%sp
move.l %d0,%d2
move.l (4,%a5),-(%sp)
addq.l #1,%d2
jsr (EXEC_$LOG_W).l
move.l %d2,%d0
movem.l (-0x10,%fp),%d2/%a5
unlk %fp
rts
INNER:
link.w %fp,#-0x14
movea.l (8,%fp),%a0
move.l (%a0)+,(-0x10,%fp)
move.l (%a0)+,(-0xc,%fp)
move.l (-0x10,%fp),%d0
add.l (-0xc,%fp),%d0
unlk %fp
rts
.globl BP_$NEST
BP_$NEST:
link.w %fp,#-0xc
movea.l (8,%fp),%a0
move.l (%a0),(-8,%fp)
moveq #5,%d0
move.l %d0,(-4,%fp)
pea (-8,%fp)
bsr.s INNER
unlk %fp
rts
.globl BP_$UNUSED
BP_$UNUSED:
link.w %fp,#-8
movea.l (8,%fp),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
moveq #9,%d0
movea.l (0xc,%fp),%a0
move.l %d0,(%a0)
unlk %fp
rts
.globl BP_$UNUSED_T
BP_$UNUSED_T:
link.w %fp,#-0x10
move.l #0x12c,(-8,%fp)
move.l #0x190,(-4,%fp)
pea (-0xc,%fp)
pea (-8,%fp)
bsr.s BP_$UNUSED
move.l (-0xc,%fp),%d0
unlk %fp
rts
.globl BP_$CHARS6
BP_$CHARS6:
link.w %fp,#0
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
moveq #5,%d0
.balignw 4,0x2048
1:
move.b (%a0)+,(%a1)+
dbf %d0,1b
unlk %fp
rts
.globl BP_$CHARS3
BP_$CHARS3:
link.w %fp,#0
movea.l (0xc,%fp),%a0
movea.l (8,%fp),%a1
moveq #2,%d0
.balignw 4,0x2048
1:
move.b (%a0)+,(%a1)+
dbf %d0,1b
unlk %fp
rts
.globl BP_$CHARS_T
BP_$CHARS_T:
link.w %fp,#-0x14
move.b #0x61,(-6,%fp)
move.b #0x62,(-5,%fp)
move.b #0x63,(-4,%fp)
move.b #0x64,(-3,%fp)
move.b #0x65,(-2,%fp)
move.b #0x66,(-1,%fp)
pea (-6,%fp)
pea (-0xc,%fp)
bsr.s BP_$CHARS6
addq.w #8,%sp
pea (-0xc,%fp)
jsr (EXEC_$LOG_C).l
addq.w #4,%sp
pea (-7,%fp)
jsr (EXEC_$LOG_C).l
addq.w #4,%sp
move.b #0x78,(-0x10,%fp)
move.b #0x79,(-0xf,%fp)
move.b #0x7a,(-0xe,%fp)
pea (-0x10,%fp)
pea (-0x14,%fp)
bsr.s BP_$CHARS3
addq.w #8,%sp
pea (-0x14,%fp)
jsr (EXEC_$LOG_C).l
addq.w #4,%sp
pea (-0x12,%fp)
jsr (EXEC_$LOG_C).l
unlk %fp
rts
.globl BP_$NEG
BP_$NEG:
link.w %fp,#-8
move.w (8,%fp),%d0
move.w %d0,%d1
clr.w %d0
bra.s 2f
.balignw 4,0x2048
1:
addq.w #2,%d0
addi.w #-3,%d1
2:
tst.w %d1
bgt.s 1b
unlk %fp
rts
.globl BP_$CWEB
BP_$CWEB:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (8,%fp),%d0
moveq #-1,%d2
tst.w %d0
ble.s 1f
moveq #3,%d2
1:
jsr (EXEC_$CLOBBER).l
move.w %d2,%d0
move.l (-8,%fp),%d2
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BE_PARAMS","aw",@nobits
.balign 4
.Ldata:
.space 8
