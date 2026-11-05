.text
.globl GET_ROOT
GET_ROOT:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a1
lea (%a5),%a0
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
movea.l (-4,%fp),%a5
unlk %fp
rts
FROM_LOCAL:
link.w %fp,#-0x10
move.w (0xc,%fp),%d0
movea.l #UID_$NIL,%a0
move.l (%a0)+,(-0x10,%fp)
move.l (%a0)+,(-0xc,%fp)
move.w %d0,%d1
ext.l %d1
move.l %d1,(-0xc,%fp)
lea (-0x10,%fp),%a0
movea.l (8,%fp),%a1
move.l (%a0)+,(%a1)
move.l (%a0)+,(4,%a1)
unlk %fp
rts
CLK:
link.w %fp,#-0xc
movea.l (0xc,%fp),%a0
move.l (%a0),%d0
add.l %d0,%d0
move.l %d0,(-0xc,%fp)
move.w (4,%a0),(-8,%fp)
movea.l (8,%fp),%a1
move.l %d0,(%a1)
move.w (-8,%fp),(4,%a1)
unlk %fp
rts
.globl PICK
PICK:
link.w %fp,#0
movea.l (8,%fp),%a0
movea.l (0xc,%fp),%a1
tst.w (%a1)
bne.s 1f
movea.l #ACL_$FNDWRX,%a1
move.l (%a1)+,(%a0)
move.l (%a1)+,(4,%a0)
bra.s 2f
1:
cmpi.w #3,(%a1)
bne.s 3f
movea.l #UID_$NIL,%a1
move.l (%a1)+,(%a0)
move.l (%a1)+,(4,%a0)
3:
2:
unlk %fp
rts
.globl ASSIGN_LOCAL
ASSIGN_LOCAL:
link.w %fp,#-0x10
pea (%a5)
lea (.Ldata).l,%a5
pea (-0x10,%fp)
bsr.w GET_ROOT
lea (-0x10,%fp),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
lea (-8,%fp),%a0
move.l (%a0)+,(8,%a5)
move.l (%a0)+,(0xc,%a5)
movea.l (-0x14,%fp),%a5
unlk %fp
rts
.globl ASSIGN_FIELD
ASSIGN_FIELD:
link.w %fp,#-0xc
movea.l (8,%fp),%a0
move.l (%a0),(-4,%fp)
pea (-0xc,%fp)
bsr.w GET_ROOT
lea (-0xc,%fp),%a0
movea.l (-4,%fp),%a1
move.l (%a0)+,(4,%a1)
move.l (%a0)+,(8,%a1)
unlk %fp
rts
.globl ASSIGN_SELF
ASSIGN_SELF:
link.w %fp,#-8
pea (%a2)
movea.l (8,%fp),%a2
pea (%a2)
pea (-8,%fp)
bsr.w CLK
move.l (-8,%fp),(%a2)
move.w (-4,%fp),(4,%a2)
movea.l (-0xc,%fp),%a2
unlk %fp
rts
.globl WORD_ARG
WORD_ARG:
link.w %fp,#-0x14
pea (%a5)
lea (.Ldata).l,%a5
movea.l (8,%fp),%a0
move.w (%a0),(-0xa,%fp)
move.w (-0xa,%fp),-(%sp)
pea (-0x14,%fp)
bsr.w FROM_LOCAL
lea (-0x14,%fp),%a0
move.l (%a0)+,(-8,%fp)
move.l (%a0)+,(-4,%fp)
lea (-8,%fp),%a0
move.l (%a0)+,(8,%a5)
move.l (%a0)+,(0xc,%a5)
movea.l (-0x18,%fp),%a5
unlk %fp
rts
.globl PASS_ON
PASS_ON:
link.w %fp,#-0xc
pea (-4,%fp)
move.l (8,%fp),-(%sp)
pea (.Lc0,%pc)
pea (-0xc,%fp)
jsr (DEFAULT_ACL).l
addq.w #8,%sp
pea (-0xc,%fp)
jsr (SET_ACL).l
unlk %fp
rts
.Lc0:
.short 0x0003
.globl PASS_TWO
PASS_TWO:
link.w %fp,#-0x10
move.l (0xc,%fp),-(%sp)
pea (-8,%fp)
jsr (TO_CLOCK).l
addq.w #8,%sp
pea (-8,%fp)
move.l (8,%fp),-(%sp)
pea (-0x10,%fp)
jsr (TO_CLOCK).l
addq.w #8,%sp
pea (-0x10,%fp)
pea (.Lc1,%pc)
jsr (SET_BOTH).l
unlk %fp
rts
.Lc1:
.short 0x0001
.balignw 4,0x2048
.section ".bss.moddata.HRES","aw",@nobits
.balign 4
.Ldata:
.space 16
