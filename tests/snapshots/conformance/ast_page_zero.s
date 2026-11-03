.set __dpc_fragment_origin,0xe00eb0
.text
ZERO_PAGE:
link.w %fp,#-0xc
pea (0x16).w
move.l #AST_$ZERO_BUFF,-(%sp)
move.l (8,%fp),-(%sp)
jsr (MMU_$INSTALL).l
lea (0xc,%sp),%sp
movea.l #AST_$ZERO_BUFF,%a0
move.w #0xff,%d0
.balignw 4,0x2048
1:
clr.l (%a0)+
dbf %d0,1b
move.l (8,%fp),-(%sp)
jsr (MMU_$REMOVE).l
unlk %fp
rts
.globl AST_$PAGE_ZERO
AST_$PAGE_ZERO:
link.w %fp,#0
subq.l #2,%sp
move.w #0x14,-(%sp)
jsr (ML_$LOCK).l
addq.w #4,%sp
move.l (8,%fp),-(%sp)
bsr.s ZERO_PAGE
addq.w #4,%sp
subq.l #2,%sp
move.w #0x14,-(%sp)
jsr (ML_$UNLOCK).l
unlk %fp
rts
