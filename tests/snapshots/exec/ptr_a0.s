.text
.globl PTR_A0_$GETN
PTR_A0_$GETN:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a0
clr.l (%a0)
move.l (0x40,%a5),(-4,%fp)
movea.l (-4,%fp),%a0
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PTR_A0_$RESET
PTR_A0_$RESET:
link.w %fp,#0
movem.l %d2-%d5/%a5,-(%sp)
lea (.Ldata).l,%a5
moveq #0xa,%d0
move.l %d0,(PARK_INT_$DESC+4).l
moveq #7,%d1
move.l %d1,(PARK_INT_$DESC).l
moveq #4,%d2
move.l %d2,(8,%a5)
clr.l (4,%a5)
lea (4,%a5),%a0
move.l %a0,(0xc,%a5)
moveq #3,%d0
move.l %d0,(%a5)
moveq #1,%d1
move.l %d1,(0x28,%a5)
lea (0x30,%a5),%a0
move.l %a0,(0x2c,%a5)
moveq #2,%d0
move.l %d0,(0x30,%a5)
lea (0x38,%a5),%a0
move.l %a0,(0x34,%a5)
moveq #3,%d0
move.l %d0,(0x38,%a5)
moveq #0x2c,%d1
move.l %d1,(0x50,%a5)
moveq #0x21,%d2
move.l %d2,(0x4c,%a5)
moveq #0x16,%d3
move.l %d3,(0x48,%a5)
moveq #0xb,%d4
move.l %d4,(0x44,%a5)
moveq #0x42,%d5
move.l %d5,(0x14,%a5)
movem.l (-0x14,%fp),%d2-%d5/%a5
unlk %fp
rts
.globl PTR_A0_$SHOW
PTR_A0_$SHOW:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
move.l (PARK_INT_$DESC).l,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (PARK_INT_$DESC+4).l,-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (4,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (8,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (0x28,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (0x30,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (0x38,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
move.l (0x48,%a5),-(%sp)
jsr (EXEC_$LOGV_L).l
addq.w #4,%sp
bsr.w PTR_A0_$RESET
movea.l (-4,%fp),%a5
unlk %fp
rts
.globl PTR_A0_$OUTP
PTR_A0_$OUTP:
link.w %fp,#-4
pea (%a2)
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
movea.l (0x10,%fp),%a1
move.l (%a0),(%a1)
movea.l (0x14,%fp),%a2
move.l (4,%a0),(%a2)
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PTR_A0_$VARREC
PTR_A0_$VARREC:
link.w %fp,#-4
pea (%a2)
movea.l (0x10,%fp),%a2
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
moveq #1,%d0
move.l %d0,(%a2)
add.l (%a0),%d0
move.l %d0,(%a2)
add.l (4,%a0),%d0
move.l %d0,(%a0)
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PTR_A0_$IFELSE
PTR_A0_$IFELSE:
link.w %fp,#-4
pea (%a2)
movea.l (0xc,%fp),%a2
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
tst.l (%a2)
bne.s 1f
addq.l #1,(%a0)
bra.s 2f
1:
clr.l (4,%a0)
2:
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PTR_A0_$WITH
PTR_A0_$WITH:
link.w %fp,#-4
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
move.l (4,%a0),%d0
addq.l #3,%d0
move.l %d0,(%a0)
move.l %d0,(4,%a0)
unlk %fp
rts
.globl PTR_A0_$ARRINDEX
PTR_A0_$ARRINDEX:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (0xc,%fp),%d2
move.l (0xe,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
move.w %d2,%d0
lsl.w #2,%d0
move.l (%a0),(0x40,%a5,%d0.w)
move.l (0x44,%a5,%d0.w),(4,%a0)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl PTR_A0_$TWOQ
PTR_A0_$TWOQ:
link.w %fp,#-8
movem.l %a2-%a3,-(%sp)
movea.l (0xc,%fp),%a3
pea (%a3)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
movea.l %a0,%a2
pea (%a3)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
move.l (4,%a0),%d0
addq.l #1,%d0
move.l %d0,(%a2)
move.l %d0,(4,%a0)
movem.l (-0x10,%fp),%a2-%a3
unlk %fp
rts
.globl PTR_A0_$FUNC
PTR_A0_$FUNC:
link.w %fp,#-8
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
move.l (%a0),%d0
add.l (4,%a0),%d0
unlk %fp
rts
.globl PTR_A0_$PTRFUNC
PTR_A0_$PTRFUNC:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
moveq #1,%d0
move.l %d0,(%a0)
move.l (0xc,%a5),(-8,%fp)
movea.l (-8,%fp),%a0
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl PTR_A0_$ADDROF
PTR_A0_$ADDROF:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
lea (4,%a5),%a1
move.l (%a0),(%a1)
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl PTR_A0_$WITHOTHER
PTR_A0_$WITHOTHER:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
movea.l (0xc,%a5),%a1
move.l (%a0),(%a1)
moveq #2,%d0
move.l %d0,(4,%a1)
moveq #5,%d1
move.l %d1,(4,%a0)
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PTR_A0_$CHAIN2
PTR_A0_$CHAIN2:
link.w %fp,#-4
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
bsr.w PTR_A0_$GETN
moveq #7,%d0
movea.l (4,%a0),%a1
move.l %d0,(%a1)
moveq #8,%d1
move.l %d1,(%a0)
unlk %fp
rts
.globl PTR_A0_$CHAIN3
PTR_A0_$CHAIN3:
link.w %fp,#-4
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
bsr.w PTR_A0_$GETN
movea.l (4,%a0),%a1
moveq #7,%d0
movea.l (4,%a1),%a1
move.l %d0,(%a1)
moveq #8,%d1
move.l %d1,(%a0)
unlk %fp
rts
.globl PTR_A0_$HPSTORE
PTR_A0_$HPSTORE:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
move.l (4,%a0),%d0
movea.l (0xc,%a5),%a1
move.l %d0,(%a1)
moveq #2,%d1
movea.l (0xc,%a5),%a1
move.l %d1,(4,%a1)
move.l %d0,%d1
addq.l #1,%d1
move.l %d1,(%a0)
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl PTR_A0_$MARITH3
PTR_A0_$MARITH3:
link.w %fp,#-4
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (0xc,%fp),%d2
move.l (0x10,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
moveq #1,%d0
move.l %d0,(4,%a0)
move.l %d2,-(%sp)
move.l (%a5),-(%sp)
jsr (M$MIS$LLL).l
move.l %d0,(%a5)
movem.l (-0xc,%fp),%d2/%a5
unlk %fp
rts
.globl PTR_A0_$SETSMALL
PTR_A0_$SETSMALL:
link.w %fp,#-8
move.l %d2,-(%sp)
move.w (0xc,%fp),%d2
move.l (0xe,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
clr.l %d0
bset %d2,%d0
btst %d2,%d0
beq.s 1f
moveq #1,%d0
move.l %d0,(%a0)
1:
moveq #2,%d0
move.l %d0,(4,%a0)
move.l (-0xc,%fp),%d2
unlk %fp
rts
.globl PTR_A0_$CASEW
PTR_A0_$CASEW:
link.w %fp,#-4
move.l %d2,-(%sp)
move.w (0xc,%fp),%d2
move.l (0xe,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
move.w %d2,%d0
cmpi.w #4,%d0
bcc.s 1f
add.w %d0,%d0
move.w (6f,%pc,%d0.w),%d0
jmp (6f,%pc,%d0.w)
6:
.short 2f-6b
.short 3f-6b
.short 4f-6b
.short 5f-6b
2:
moveq #0xa,%d0
8:
move.l %d0,(%a0)
bra.s 1f
3:
moveq #0x14,%d0
bra.s 7f
4:
moveq #0x1e,%d0
bra.s 8b
5:
moveq #0x28,%d0
7:
move.l %d0,(4,%a0)
1:
move.l (-8,%fp),%d2
unlk %fp
rts
.globl PTR_A0_$WHILEQ
PTR_A0_$WHILEQ:
link.w %fp,#-8
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
moveq #1,%d0
bra.s 2f
.balignw 4,0x2048
1:
move.w %d0,%d1
move.l (%a0),%d2
lsl.w #2,%d1
add.l (0x40,%a5,%d1.w),%d2
move.l %d2,(%a0)
addq.w #1,%d0
2:
cmpi.w #4,%d0
ble.s 1b
movem.l (-0x10,%fp),%d2/%a5
unlk %fp
rts
.globl PTR_A0_$GLOBCOPY
PTR_A0_$GLOBCOPY:
link.w %fp,#-0x20
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
lea (0x10,%a5),%a1
lea (-0x20,%fp),%a2
moveq #5,%d0
.balignw 4,0x2048
1:
move.l (%a1)+,(%a2)+
dbf %d0,1b
move.l (-0x1c,%fp),(%a0)
lea (-0x20,%fp),%a1
lea (0x10,%a5),%a2
moveq #5,%d0
.balignw 4,0x2048
2:
move.l (%a1)+,(%a2)+
dbf %d0,2b
moveq #3,%d0
move.l %d0,(4,%a0)
movem.l (-0x28,%fp),%a2/%a5
unlk %fp
rts
INNER:
link.w %fp,#-4
moveq #1,%d0
move.l %d0,(%a5)
unlk %fp
rts
.globl PTR_A0_$NESTED
PTR_A0_$NESTED:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
moveq #9,%d0
move.l %d0,(%a0)
bsr.s INNER
movea.l (-8,%fp),%a5
unlk %fp
rts
.globl PTR_A0_$DISABLE
PTR_A0_$DISABLE:
link.w %fp,#-4
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
ori.w #0x700,%sr
moveq #9,%d0
move.l %d0,(%a0)
andi.w #0xf8ff,%sr
unlk %fp
rts
.globl PTR_A0_$SHORTC
PTR_A0_$SHORTC:
link.w %fp,#-4
pea (%a2)
movea.l (0xc,%fp),%a2
pea (%a2)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
tst.l (%a2)
bne.s 1f
tst.l (%a0)
ble.s 1f
move.l (%a0),(4,%a0)
1:
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PTR_A0_$DEREF2
PTR_A0_$DEREF2:
link.w %fp,#-8
pea (%a5)
lea (.Ldata).l,%a5
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
movea.l (0xc,%a5),%a1
move.l (4,%a0),(%a1)
move.l (4,%a1),(%a0)
movea.l (-0xc,%fp),%a5
unlk %fp
rts
.globl PTR_A0_$BYADDR
PTR_A0_$BYADDR:
link.w %fp,#-4
pea (%a2)
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
addq.w #8,%sp
movea.l %a0,%a2
pea (%a2)
jsr (EXEC_$LOG_L).l
movea.l (-8,%fp),%a2
unlk %fp
rts
.globl PTR_A0_$WHOLE
PTR_A0_$WHOLE:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.l (0xc,%fp),-(%sp)
pea (8,%fp)
jsr (PARK_INT_$GETD).l
move.l %a0,%d0
move.l %d0,(0xc,%a5)
moveq #3,%d0
movea.l (0xc,%a5),%a0
move.l %d0,(%a0)
movea.l (-8,%fp),%a5
unlk %fp
rts
.balignw 4,0x2048
.section ".moddata.PTR_A0","aw",@progbits
.balign 4
.Ldata:
.space 12
.long .Ldata+4
.space 48
.long .Ldata+40
.space 16
