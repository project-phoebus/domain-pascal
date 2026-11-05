.text
.globl A0F
A0F:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #2,%d1
move.l (-4,%a5,%d1.w),(-4,%fp)
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl A0G
A0G:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
suba.l %a0,%a0
cmpi.w #3,%d0
bgt.s 1f
move.w %d0,%d1
lsl.w #2,%d1
move.l (-4,%a5,%d1.w),%d0
movea.l %d0,%a0
1:
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl OGF8_RESULTS_$A0
OGF8_RESULTS_$A0:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s A0F
addq.w #4,%sp
move.l %a0,(0x18,%a5)
pea (0x18,%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s A0F
addq.w #4,%sp
move.l %a0,%d0
move.l %d0,%d1
subi.l #0x64,%d1
move.l %d1,(0x18,%a5)
pea (0x18,%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.w A0F
addq.w #4,%sp
move.l %a0,%d0
move.l %d0,%d1
addq.l #1,%d1
move.l %d1,-(%sp)
jsr (EXEC_$LOGV_L).l
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl OGF8_RESULTS_$A0_DIRECT
OGF8_RESULTS_$A0_DIRECT:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.w %d0,%d1
lsl.w #2,%d1
move.l (-4,%a5,%d1.w),(-4,%fp)
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl OGF8_RESULTS_$A0_MULTI
OGF8_RESULTS_$A0_MULTI:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
suba.l %a0,%a0
cmpi.w #3,%d0
bgt.s 1f
move.w %d0,%d1
lsl.w #2,%d1
move.l (-4,%a5,%d1.w),%d0
movea.l %d0,%a0
1:
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl SF
SF:
link.w %fp,#-4
movea.l (8,%fp),%a0
clr.w %d0
move.b (%a0),%d0
unlk %fp
rts
.globl SG
SG:
link.w %fp,#-4
clr.w %d0
move.w (8,%fp),%d1
bset %d1,%d0
ori.w #-0x8000,%d0
unlk %fp
rts
.globl SH
SH:
link.w %fp,#-4
movea.l (8,%fp),%a0
moveq #-2,%d0
and.l (%a0),%d0
unlk %fp
rts
.globl OGF8_RESULTS_$SETS
OGF8_RESULTS_$SETS:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
clr.b %d0
bset %d2,%d0
move.b %d0,(0x22,%a5)
move.b #1,(0x23,%a5)
pea (0x22,%a5)
bsr.s SF
addq.w #4,%sp
move.w %d0,(0x1c,%a5)
pea (.Lc0,%pc)
pea (0x1c,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
moveq #7,%d0
move.l %d0,(0x1e,%a5)
pea (0x1e,%a5)
bsr.s SH
addq.w #4,%sp
move.l %d0,(0x1e,%a5)
pea (.Lc1,%pc)
pea (0x1e,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
tst.w %d2
bne.s 1f
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.w SG
addq.w #4,%sp
move.w %d0,(0x1c,%a5)
pea (.Lc0,%pc)
pea (0x1c,%a5)
jsr (EXEC_$LOG_MEM).l
addq.w #8,%sp
pea (.Lc2,%pc)
bsr.w SH
addq.w #4,%sp
move.l %d0,(0x1e,%a5)
pea (.Lc1,%pc)
pea (0x1e,%a5)
jsr (EXEC_$LOG_MEM).l
1:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.Lc0:
.short 0x0002
.Lc1:
.short 0x0004
.balignw 4,0x2048
.Lc2:
.short 0x8000,0x0007
.globl OGF8_RESULTS_$SET_DIRECT
OGF8_RESULTS_$SET_DIRECT:
link.w %fp,#-4
clr.w %d0
move.w (8,%fp),%d1
bset %d1,%d0
unlk %fp
rts
.globl RF
RF:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
bpl.s 1f
move.l #0x4000e,(-4,%fp)
move.l (-4,%fp),%d0
bra.s 2f
1:
move.w %d0,%d1
ext.l %d1
move.l %d1,(0x18,%a5)
pea (0x18,%a5)
jsr (EXEC_$ID_L).l
move.l %d0,(-4,%fp)
2:
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl RG
RG:
link.w %fp,#-4
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s RF
move.l %d0,(-4,%fp)
unlk %fp
rts
.globl OGF8_RESULTS_$RECS
OGF8_RESULTS_$RECS:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
subq.l #2,%sp
move.w %d2,-(%sp)
bsr.s RF
addq.w #4,%sp
move.l %d0,(0x24,%a5)
pea (0x24,%a5)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
tst.w %d2
bpl.s 1f
subq.l #2,%sp
move.w #-5,-(%sp)
bsr.s RG
addq.w #4,%sp
move.l %d0,(0x24,%a5)
subi.l #0x40013,(0x24,%a5)
pea (0x24,%a5)
jsr (EXEC_$LOG_L).l
1:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl PF
PF:
link.w %fp,#-0xc
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d0
move.l (0xa,%fp),%d1
tst.w %d0
beq.s 1f
cmpi.w #3,%d0
bgt.s 1f
move.w %d0,%d2
lsl.w #2,%d2
lea (8,%a5,%d2.w),%a0
cmp.l (%a0),%d1
bne.s 2f
movea.l %a0,%a1
bra.s 3f
2:
1:
cmpi.l #0x63,%d1
bne.s 4f
suba.l %a1,%a1
bra.s 5f
4:
lea (0x14,%a5),%a0
move.l %a0,(-0xc,%fp)
pea (-0xc,%fp)
jsr (EXEC_$ID_P).l
movea.l %a0,%a1
5:
3:
movea.l %a1,%a0
movem.l (-0x14,%fp),%d2/%a5
unlk %fp
rts
.globl OGF8_RESULTS_$PTR
OGF8_RESULTS_$PTR:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (0xa,%fp),%d2
subq.l #2,%sp
move.w %d2,%d0
ext.l %d0
move.l %d0,-(%sp)
move.w (8,%fp),-(%sp)
bsr.s PF
addq.w #8,%sp
move.l %a0,(0x28,%a5)
tst.l (0x28,%a5)
beq.s 1f
pea (%a0)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
movea.l (0x28,%a5),%a0
addq.l #1,(%a0)
pea (0x10,%a5)
bra.s 3f
1:
pea (.Lc3,%pc)
jsr (EXEC_$LOG_L).l
addq.w #4,%sp
pea (.Lc3,%pc)
3:
jsr (EXEC_$LOG_L).l
2:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.balignw 4,0x2048
.Lc3:
.short 0x0000,0x0000
.balignw 4,0x2048
.section ".moddata.OGF8_RESULTS","aw",@progbits
.balign 4
.Ldata:
.byte 0x00,0x00,0x03,0xe8,0x00,0x00,0x07,0xd0,0x00,0x00,0x0b,0xb8,0x00,0x00,0x00,0x0a
.byte 0x00,0x00,0x00,0x14,0x00,0x00,0x00,0x1e
.space 20
