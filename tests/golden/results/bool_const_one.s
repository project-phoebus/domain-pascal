.text
.globl YES
YES:
link.w %fp,#-4
st %d0
unlk %fp
rts
.globl NO
NO:
link.w %fp,#-4
clr.b %d0
unlk %fp
rts
.balignw 4,0x2048
