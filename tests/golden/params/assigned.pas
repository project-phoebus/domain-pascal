{ Assignments to by-value parameters (bead domain-pascal-jyba).
  codegen-patterns 12.2, "Assigned parameters": a parameter loaded at
  entry is assigned in its register, as a register-held local is (12.5);
  one in its slot is assigned there, as a frame object is.

  - pa_$round: `x := (x + 16#7FFF) & (-16#8000)' computed in the entry
    register (area_$internal_create 0xE077FE `addi.l #0x7fff,D2; andi.l
    #-0x8000,D2'), read from it after.
  - pa_$pages: a word `(size + 16#3FF) & 16#FC00' in place (PROC1_$ALLOC_
    STACK 0xE15040 `addi.w #0x3ff,D2; andi.w #-0x400,D2'), then counted
    down to 0 in a loop, the test on the subtraction's flags (0xE150EA
    `subi.w #0x400,D2; bne').
  - pa_$prot: `prot := prot ! 2' (mst_$set_mstes 0xE43E36 `ori.w
    #2,D0').
  - pa_$clamp: constants in an `if' (audit_$log_link_op 0xE4BD78 `tst.w
    D2; bpl.b; clr.w D2'; RIP_$UPDATE_INT 0xE15944 `cmpi.w #0x11,D0; bls.b;
    moveq #0x11,D0').
  - pa_$strip: `ch := chr(ord(ch) & 16#7F)' at the character's width
    (tty_$i_rcv 0xE1B966 `andi.b #0x7f,D3').
  - pa_$compress: `rshft' and `+ 1' in place (pacct_$compress 0xE5A9E4
    `lsr.l #3,D0', 0xE5A9F2 `addq.l #1,D0').
  - pa_$take: `count := count - share', `x := min(x, y)' (MMAP_$ALLOC_PURE
    0xE0D7F0 `sub.w D6,D2'; MSG_$$RCV_INTERNAL 0xE5961C `cmp.w (2,A2),D2;
    bls; move.w (2,A2),D2').
  - pa_$resize: read by a nested routine, so in its slot: loaded,
    computed and stored back (area_$resize 0xE0883C..0xE0884C `move.l
    (0xe,A6),D0; addi.l #0x7fff,D0; andi.l #-0x8000,D0; move.l
    D0,(0xe,A6)').
  - pa_$pushed: only pushed, so in its slot: a constant stored there
    (AST_$GET_SEG_MAP 0xE06BC6 `clr.l (0x10,A6)', pushed at 0xE06BD2). }
module pa;
type
  pinteger = 0..65535;
var
  g: integer32;
  gw: integer;
  gc: char;

procedure pa_$round(x: integer32); options(val_param);
begin
  x := (x + 16#7FFF) & (-16#8000);
  g := x;
  gw := 1;
end;

procedure pa_$pages(size: pinteger); options(val_param);
begin
  size := (size + 16#3FF) & 16#FC00;
  gw := size;
  repeat
    g := 0;
    size := size - 16#400;
  until size = 0;
end;

procedure pa_$prot(prot: integer; seg: integer); options(val_param);
begin
  prot := prot ! 2;
  gw := seg;
  gw := prot;
end;

procedure pa_$clamp(len: integer; hop: pinteger); options(val_param);
begin
  if len < 0 then
    len := 0;
  if hop > 17 then
    hop := 17;
  gw := len;
  gw := hop;
end;

procedure pa_$strip(ch: char; strip: boolean); options(val_param);
begin
  if strip then
    ch := chr(ord(ch) & 16#7F);
  gc := ch;
end;

procedure pa_$compress(value: integer32); options(val_param);
begin
  while value >= 16#2000 do
    value := rshft(value, 3);
  value := value + 1;
  g := value;
end;

procedure pa_$take(count: integer; share: integer; m: integer); options(val_param);
begin
  count := count - share;
  count := min(count, m);
  gw := count;
end;

procedure pa_$resize(virt_size: integer32); options(val_param);
  procedure show;
  begin
    g := virt_size;
  end;
begin
  virt_size := (virt_size + 16#7FFF) & (-16#8000);
  show;
end;

procedure pa_$log(v: integer32); options(val_param); extern;

procedure pa_$pushed(location: integer32; delete: boolean); options(val_param);
begin
  if delete then
    location := 0;
  pa_$log(location);
end;
