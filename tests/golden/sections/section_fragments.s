.set __dpc_fragment_origin.text.SFRAG.WIRED_PROC,0x1002
.set __dpc_fragment_origin.text,0x20000
.section ".text.SFRAG.WIRED_PROC","ax",@progbits
.space 2
.globl SFRAG_$PUT
SFRAG_$PUT:
link.w %fp,#0
.short 0x61e8
jsr (SFRAG_$GET_DESC).l
jsr (SFRAG_$INIT).l
unlk %fp
rts
.text
.globl SFRAG_$RESET
SFRAG_$RESET:
link.w %fp,#0
jsr (SFRAG_$LOCK).l
bsr.w SFRAG_$GET_DESC
jsr (SFRAG_$INIT).l
jsr (SFRAG_$PUT).l
unlk %fp
rts
.balignw 4,0x2048
