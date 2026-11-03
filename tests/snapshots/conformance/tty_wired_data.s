.set __dpc_fragment_origin,0xe1aed0
.section ".text.TTY.WIRED_PROC","ax",@progbits
TTY_$I_LOCK:
link.w %fp,#-4
subq.l #2,%sp
move.w #3,-(%sp)
jsr (ML_$LOCK).l
unlk %fp
rts
TTY_$I_UNLOCK:
link.w %fp,#-4
subq.l #2,%sp
move.w #3,-(%sp)
jsr (ML_$UNLOCK).l
unlk %fp
rts
.section ".moddata.TTY.WIRED_DATA","aw",@progbits
.balign 4
.Ldata.WIRED_DATA:
.space 27
.byte 0x01,0x00,0x00,0x37,0x00,0x00,0x00,0x00,0x1f,0x00,0x00,0x00,0x3c
