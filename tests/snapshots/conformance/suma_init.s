.section ".text.BITPAD.OS_INIT_PROC","ax",@progbits
.globl SUMA_$INIT
SUMA_$INIT:
link.w %fp,#0
movea.l #.Ldata.WIRED_DATA,%a0
move.l #TERM_$TPAD_BUFFER,(0x24,%a0)
clr.w (0x28,%a0)
move.b #1,(0x1e,%a0)
move.w #0x200,(0x2a,%a0)
unlk %fp
rts
.balignw 4,0x2048
.section ".bss.moddata.BITPAD.WIRED_DATA","aw",@nobits
.balign 4
.Ldata.WIRED_DATA:
.space 44
