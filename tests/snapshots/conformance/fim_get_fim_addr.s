.set __dpc_fragment_origin,0xe0aa04
.text
.globl FIM_$GET_FIM_ADDR
FIM_$GET_FIM_ADDR:
link.w %fp,#-4
pea (%a5)
lea (.Ldata).l,%a5
move.w (PROC1_$AS_ID).l,%d0
lsl.w #2,%d0
move.l (0x3c,%a5,%d0.w),%d0
movea.l (-8,%fp),%a5
unlk %fp
rts
.section ".moddata.FIM_","aw",@progbits
.balign 4
.Ldata:
.space 292
.byte 0x08,0x08,0x0c,0x00,0x10,0x00,0x00,0x00,0x3a,0x14,0x20,0x5c
.space 4
