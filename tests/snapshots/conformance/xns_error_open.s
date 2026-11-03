.set __dpc_fragment_origin,0xe178aa
.section ".text.XNS_ERROR.WIRED_PROC","ax",@progbits
.space 2
.globl MAYBE_OPEN_ERROR_SOCKET
MAYBE_OPEN_ERROR_SOCKET:
link.w %fp,#-0x28
movem.l %a2/%a5,-(%sp)
lea (.Ldata.WIRED_DATA).l,%a5
movea.l (8,%fp),%a2
clr.l (%a2)
move.l #XNS_ERROR_$CLIENT_MUTEX,-(%sp)
jsr (ML_$EXCLUSION_START).l
addq.w #4,%sp
tst.w (0x74,%a5)
bne.s 1f
move.l #0x30008,(-0x28,%fp)
clr.l (-0x24,%fp)
pea (%a2)
pea (-0x28,%fp)
jsr (XNS_IDP_$OS_OPEN).l
addq.w #8,%sp
tst.l (%a2)
bne.s 2f
move.w (-0x26,%fp),(0x76,%a5)
1:
addq.w #1,(0x74,%a5)
2:
move.l #XNS_ERROR_$CLIENT_MUTEX,-(%sp)
jsr (ML_$EXCLUSION_STOP).l
movem.l (-0x30,%fp),%a2/%a5
unlk %fp
rts
.section ".bss.moddata.XNS_ERROR.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 118
.globl XNS_ERROR_$STD_IDP_CHANNEL
XNS_ERROR_$STD_IDP_CHANNEL:
.space 2
