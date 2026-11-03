.set __dpc_fragment_origin,0xe673aa
.text
.space 2
.globl TTY_$I_SET_RAW
TTY_$I_SET_RAW:
link.w %fp,#-4
movem.l %a2-%a3/%a5,-(%sp)
lea (.Ldata).l,%a5
movea.l (0xc,%fp),%a3
subq.l #2,%sp
pea (%a3)
move.w (8,%fp),-(%sp)
jsr (TTY_$I_GET_DESC).l
addq.w #8,%sp
movea.l %a0,%a2
tst.l (%a3)
bne.s 1f
subq.l #2,%sp
move.b (0xa,%fp),-(%sp)
pea (%a2)
jsr (TTY_$I_SET_RAW_MODE).l
1:
movem.l (-0x10,%fp),%a2-%a3/%a5
unlk %fp
rts
.section ".bss.moddata.TTY","aw",@nobits
.balign 4
.Ldata:
.space 4
