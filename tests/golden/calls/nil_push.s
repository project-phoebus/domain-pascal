.text
.globl NIO_$PUT_INT
NIO_$PUT_INT:
link.w %fp,#0
subq.l #2,%sp
move.w (0x12,%fp),-(%sp)
move.l (0xe,%fp),-(%sp)
st -(%sp)
move.l (0xa,%fp),-(%sp)
move.w (8,%fp),-(%sp)
clr.l -(%sp)
jsr (NIO_$COMMON).l
unlk %fp
rts
.balignw 4,0x2048
