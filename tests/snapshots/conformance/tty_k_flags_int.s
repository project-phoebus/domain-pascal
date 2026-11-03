.set __dpc_fragment_origin,0xe67618
.text
.globl TTY_$K_INQ_FUNC_ENABLED
TTY_$K_INQ_FUNC_ENABLED:
link.w %fp,#-8
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0x10,%fp),%a2
subq.l #2,%sp
pea (%a2)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (TTY_$I_GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a0
movea.l (0xc,%fp),%a1
move.l (0x20,%a0),(%a1)
1:
movem.l (-0x10,%fp),%a2/%a5
unlk %fp
rts
.globl TTY_$K_SET_INPUT_FLAG
TTY_$K_SET_INPUT_FLAG:
link.w %fp,#-8
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a2
movea.l (0x14,%fp),%a3
subq.l #2,%sp
pea (%a3)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (TTY_$I_GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a3)
bne.s 1f
movea.l (0x10,%fp),%a1
movea.l %d0,%a0
tst.b (%a1)
bpl.s 2f
clr.l %d1
move.w (%a2),%d2
bset %d2,%d1
or.l %d1,(0x14,%a0)
bra.s 3f
2:
clr.l %d1
move.w (%a2),%d2
bset %d2,%d1
not.l %d1
and.l %d1,(0x14,%a0)
3:
1:
movem.l (-0x18,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
.globl TTY_$K_INQ_INPUT_FLAGS
TTY_$K_INQ_INPUT_FLAGS:
link.w %fp,#-8
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0x10,%fp),%a2
subq.l #2,%sp
pea (%a2)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (TTY_$I_GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a0
movea.l (0xc,%fp),%a1
move.l (0x14,%a0),(%a1)
1:
movem.l (-0x10,%fp),%a2/%a5
unlk %fp
rts
.globl TTY_$K_SET_OUTPUT_FLAG
TTY_$K_SET_OUTPUT_FLAG:
link.w %fp,#-8
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a2
movea.l (0x14,%fp),%a3
subq.l #2,%sp
pea (%a3)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (TTY_$I_GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a3)
bne.s 1f
movea.l (0x10,%fp),%a1
movea.l %d0,%a0
tst.b (%a1)
bpl.s 2f
clr.l %d1
move.w (%a2),%d2
bset %d2,%d1
or.l %d1,(0xc,%a0)
bra.s 3f
2:
clr.l %d1
move.w (%a2),%d2
bset %d2,%d1
not.l %d1
and.l %d1,(0xc,%a0)
3:
1:
movem.l (-0x18,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
.globl TTY_$K_INQ_OUTPUT_FLAGS
TTY_$K_INQ_OUTPUT_FLAGS:
link.w %fp,#-8
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0x10,%fp),%a2
subq.l #2,%sp
pea (%a2)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (TTY_$I_GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a0
movea.l (0xc,%fp),%a1
move.l (0xc,%a0),(%a1)
1:
movem.l (-0x10,%fp),%a2/%a5
unlk %fp
rts
.globl TTY_$K_SET_ECHO_FLAG
TTY_$K_SET_ECHO_FLAG:
link.w %fp,#-8
movem.l %d2/%a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a2
movea.l (0x14,%fp),%a3
subq.l #2,%sp
pea (%a3)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (TTY_$I_GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a3)
bne.s 1f
movea.l (0x10,%fp),%a1
movea.l %d0,%a0
tst.b (%a1)
bpl.s 2f
clr.l %d1
move.w (%a2),%d2
bset %d2,%d1
or.l %d1,(0x1c,%a0)
bra.s 3f
2:
clr.l %d1
move.w (%a2),%d2
bset %d2,%d1
not.l %d1
and.l %d1,(0x1c,%a0)
3:
1:
movem.l (-0x18,%fp),%d2/%a2-%a3/%a5
unlk %fp
rts
.globl TTY_$K_INQ_ECHO_FLAGS
TTY_$K_INQ_ECHO_FLAGS:
link.w %fp,#-8
movem.l %a2/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0x10,%fp),%a2
subq.l #2,%sp
pea (%a2)
movea.l (8,%fp),%a0
move.w (%a0),-(%sp)
jsr (TTY_$I_GET_DESC).l
addq.w #8,%sp
move.l %a0,%d0
tst.l (%a2)
bne.s 1f
movea.l %d0,%a0
movea.l (0xc,%fp),%a1
move.l (0x1c,%a0),(%a1)
1:
movem.l (-0x10,%fp),%a2/%a5
unlk %fp
rts
.section ".bss.moddata.TTY","aw",@nobits
.balign 4
.Ldata:
.space 4
