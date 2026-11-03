.section ".moddata.USA_DATA","aw",@progbits
.balign 4
.Ldata:
.globl USA_$CTL
USA_$CTL:
.byte 0x12,0x34,0x56,0x78
.globl USA_$LONG
USA_$LONG:
.byte 0x00,0x0a,0xbc,0xde
