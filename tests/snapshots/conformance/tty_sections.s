.set __dpc_fragment_origin.text.TTY.WIRED_PROC,0xe1b3b4
.set __dpc_fragment_origin.text,0xe67986
.section ".text.TTY.WIRED_PROC","ax",@progbits
TTY_$I_XMIT_CHAR:
link.w %fp,#-4
move.l #0x1000c,-(%sp)
pea (0xc,%fp)
move.l (8,%fp),-(%sp)
bsr.w TTY_$I_PUT_CHARS
unlk %fp
rts
.text
.space 2
.globl TTY_$K_SET_SESSION_ID
TTY_$K_SET_SESSION_ID:
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
move.w (%a1),(0x54,%a0)
1:
movem.l (-0x10,%fp),%a2/%a5
unlk %fp
rts
.globl TTY_$K_INQ_SESSION_ID
TTY_$K_INQ_SESSION_ID:
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
move.w (0x54,%a0),(%a1)
1:
movem.l (-0x10,%fp),%a2/%a5
unlk %fp
rts
.section ".bss.moddata.TTY","aw",@nobits
.balign 4
.Ldata:
.space 4
