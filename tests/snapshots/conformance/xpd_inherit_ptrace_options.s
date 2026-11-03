.set __dpc_fragment_origin,0xe5b174
.text
.globl XPD_$INHERIT_PTRACE_OPTIONS
XPD_$INHERIT_PTRACE_OPTIONS:
link.w %fp,#-4
movea.l (8,%fp),%a0
btst #3,(0xd,%a0)
sne %d0
unlk %fp
rts
