#!/usr/bin/env python3
"""strsurvey.py - string constants assigned to arrays of char, in SR10.2's /com.

Prints each site where blanks are loaded (`move.l #0x20202020,Dn` or `moveq
#32,Dn`) and soon stored through a post-incremented register, with the
instructions around it.  Evidence for docs/codegen-patterns.md 5.2, "String
constants" (bead domain-pascal-vq0h).  Reads the install, writes nothing.
"""
import os
import re
import struct
import subprocess
import tempfile

base=os.path.expanduser('~/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2/com/')
rows=[]
for n in sorted(os.listdir(base)):
    d=open(base+n,'rb').read()
    if len(d)<48 or d[:2]!=b'\x01\x97': continue
    nscns,_,_,_,opt,_=struct.unpack('>HIIIHH',d[2:20])
    off=20+opt; secs={}
    for i in range(nscns):
        nm=d[off:off+8].rstrip(b'\0').decode('latin1'); pa,va,sz,sc=struct.unpack('>IIII',d[off+8:off+24]); secs[nm]=(va,sz,sc); off+=40
    va,sz,sc=secs['.text']
    with tempfile.NamedTemporaryFile(suffix='.text') as tf:
        tf.write(d[sc:sc+sz])
        tf.flush()
        t=subprocess.run(['m68k-elf-objdump','-D','-b','binary','-m','m68k:68020','--adjust-vma=%#x'%va,tf.name],capture_output=True,text=True).stdout
    seq=[]
    for l in t.split('\n'):
        x=l.split('\t')
        if len(x)>=3:
            m=re.match(r'\s*([0-9a-f]+):',x[0])
            if m: seq.append((int(m.group(1),16),x[2].strip()))
    for i,(a,tx) in enumerate(seq):
        m=re.match(r'(movel #538976288|moveq #32),(%d\d)',tx)
        if not m: continue
        r=m.group(2)
        win=seq[i+1:i+6]
        if not any(re.search(r'^move[bwl] '+r+r',%a\d@\+',u) for _,u in win): continue
        rows.append((n,a,[u for _,u in seq[max(0,i-9):i+9]]))
print(len(rows))
for n,a,w in rows: print(n,hex(a),' ; '.join(w).replace('%',''))
