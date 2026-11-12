.text
.globl ONE
ONE:
link.w %fp,#0
movem.l %d2/%a5,-(%sp)
lea (.Ldata).l,%a5
move.w (8,%fp),%d2
move.b %d2,(%a5)
clr.l %d0
move.b %d2,%d0
move.l #0xc000,%d1
btst %d0,%d1
beq.s 1f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 5f
1:
subq.l #2,%sp
clr.w -(%sp)
5:
jsr (EXEC_$LOGV_W).l
addq.w #4,%sp
2:
move.w %d2,%d0
ext.l %d0
move.l %d0,(4,%a5)
move.l #0xe002,%d1
btst %d0,%d1
beq.s 3f
subq.l #2,%sp
move.w #1,-(%sp)
bra.s 6f
3:
subq.l #2,%sp
clr.w -(%sp)
6:
jsr (EXEC_$LOGV_W).l
4:
movem.l (-8,%fp),%d2/%a5
unlk %fp
rts
.globl IN_WORD_CONST_BYTE_$RUN
IN_WORD_CONST_BYTE_$RUN:
link.w %fp,#0
subq.l #2,%sp
move.w (8,%fp),-(%sp)
bsr.s ONE
addq.w #4,%sp
subq.l #2,%sp
move.w (0xa,%fp),-(%sp)
bsr.s ONE
addq.w #4,%sp
subq.l #2,%sp
move.w (0xc,%fp),-(%sp)
bsr.w ONE
addq.w #4,%sp
subq.l #2,%sp
move.w (0xe,%fp),-(%sp)
bsr.w ONE
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.IN_WORD_CONST_BYTE","aw",@nobits
.balign 4
.Ldata:
.space 8
