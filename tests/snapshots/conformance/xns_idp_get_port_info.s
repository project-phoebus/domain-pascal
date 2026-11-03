.set __dpc_fragment_origin,0xe18fb8
.text
.globl XNS_IDP_$GET_PORT_INFO
XNS_IDP_$GET_PORT_INFO:
link.w %fp,#0
pea (%a5)
lea (.Ldata).l,%a5
movea.l (0x10,%fp),%a0
move.l #0x3a0001,(%a0)
movea.l (-4,%fp),%a5
unlk %fp
rts
.section ".bss.moddata.XNS_IDP","aw",@nobits
.balign 4
.Ldata:
.space 4
