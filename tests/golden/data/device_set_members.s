.text
.globl ENABLE_GPU_INT
ENABLE_GPU_INT:
link.w %fp,#0
andi.w #-5,(KOLOR_CTL+0xc).l
ori.w #0x80,(KOLOR_CTL+0xc).l
unlk %fp
rts
.globl DISABLE_GPU_INT
DISABLE_GPU_INT:
link.w %fp,#0
andi.w #-0x81,(KOLOR_CTL+0xc).l
unlk %fp
rts
.globl BUSY_WAIT
BUSY_WAIT:
link.w %fp,#0
ori.w #1,(COLOR3_$CR_CONTENTS).l
andi.w #-2,(COLOR3_$CR_CONTENTS).l
ori.w #8,(COLOR3_$CR_CONTENTS).l
unlk %fp
rts
.globl INQ_DONE
INQ_DONE:
link.w %fp,#0
ori.w #2,(COLOR_SUP+0xa0).l
andi.w #-3,(COLOR_SUP+0xa0).l
unlk %fp
rts
.globl WIDTHS
WIDTHS:
link.w %fp,#0
ori.b #8,(OTHER).l
andi.b #0x7f,(OTHER).l
ori.l #0x100000,(OTHER+2).l
andi.l #-3,(OTHER+2).l
unlk %fp
rts
.globl NOT_DEVICE
NOT_DEVICE:
link.w %fp,#0
bset #7,(PLAIN+1).l
bclr #4,(PLAIN).l
unlk %fp
rts
.balignw 4,0x2048
