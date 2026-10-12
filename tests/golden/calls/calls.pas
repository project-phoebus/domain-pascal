{ Golden: codegen-patterns 1 (calls).
  - 1.1 arguments right to left; the caller pops with `addq.w #n,SP' up
    to 8 bytes (AST_$PAGE_ZERO 0xE00EFC) and `lea (n,SP),SP' above
    (0xE00EC8); no pop after the last statement's call (0xE00F0C), nor
    when only statements without calls follow (PROC2_$UPGID_TO_UID
    0xE41050);
  - 1.3 a byte in the high half of a 2-byte slot (`move.b', `st');
  - 1.4 `subq.l #2,SP' pad when the argument bytes are 2 mod 4
    (0xE00EF0);
  - 1.5 long constants: `clr.l' for 0, `pea (n).w' within 16 bits
    (0xE00EB4), `move.l #n' beyond; word constants `clr.w' / `move.w #n'
    (0xE135D2, 0xE00EF2); two adjacent word constants merged into one
    longword (0xE00EB4); `true' as `st -(SP)' (0xE1365C), `false' as
    `clr.w -(SP)' (0xE688BC); another unit's
    variable by address and `addr' of it as `move.l #sym,-(SP)'
    (0xE01824, 0xE00EB8); a frame object by address `pea (d,A6)'
    (0xE14DF4);
  - 1.6 `jsr (sym).l' (0xE00EF6); `bsr.s' within the unit when in reach
    (0xE00F02), else `bsr.w' (0xE03636);
  - 12.2 a by-value parameter only pushed is pushed from the frame. }
module calls;
type
  uid_t = record
    high, low: integer32;
  end;
var
  uid_$nil: extern uid_t;
  buf: extern array[1..4] of integer32;
  count: extern integer;

procedure ext_$long(a: integer32); options(val_param, extern);
procedure ext_$word(w: integer); options(val_param, extern);
procedure ext_$pair(a, b: integer; c: integer32); options(val_param, extern);
procedure ext_$flag(b: boolean; p: univ_ptr); options(val_param, extern);
procedure ext_$uid(in u: uid_t); options(val_param, extern);
procedure ext_$three(a, b, c: integer32); options(val_param, extern);

procedure worker(x: integer32); internal;
begin
  ext_$long(x);
end;

{ Near: `bsr.s'.  calls_$consts below is too far: `bsr.w'. }
procedure calls_$near(x: integer32); options(val_param);
begin
  worker(x);
end;

procedure calls_$consts(x: integer32; w: integer; b: boolean); options(val_param);
begin
  ext_$long(0);
  ext_$long(16#22);
  ext_$long(-2);
  ext_$long(16#12345);
  ext_$word(0);
  ext_$word(-5);
  ext_$pair(1, 16#20, x);
  ext_$flag(true, addr(buf));
  ext_$flag(false, addr(buf));
  ext_$flag(b, addr(buf));
  ext_$word(w);
  ext_$uid(uid_$nil);
  ext_$three(x, x, x);
  worker(x);
end;

{ The call's arguments are not popped: no push follows before `unlk'. }
procedure calls_$tail(x: integer32); options(val_param);
begin
  ext_$long(x);
  count := 3;
end;
