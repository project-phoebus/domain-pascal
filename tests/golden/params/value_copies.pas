{ Value parameters under the default convention: passed by address, copied
  by the callee on entry (ref 5.2 Table 5-1, p. 5-5; bead
  domain-pascal-5aw.12).  codegen-patterns 1.2, "Value parameters passed by
  address", and 2, "Where the compiler's objects go" (with no locals the
  copies are the whole frame, the larger first, those of one size at
  rising addresses in parameter order; 8 bytes or more at multiples of
  8).

  - vc_$scalars: byte, word and longword copies, `move.<s> (An),(d,A6)'
    (FILE_$INVALIDATE 0xE75166..0xE75186: A0, A1, A2 for three slots, each
    a register no earlier copy left a value in, 12.4 R4(b)); the copies
    pushed by value from the frame (0xE751B4..0xE751BC).
  - vc_$assign: an assignment to a value parameter goes to the copy, never
    through the slot, so the caller's variable is not changed (ref 5.2
    p. 5-4); `addr' of the parameter and passing it to a `var' parameter
    name the copy.
  - vc_$uid: an 8-byte record, `move.l (A0)+,(d,A6)' twice (124 sites,
    DIR_$GET_ENTRYU 0xE4D512..0xE4D51A), after the entry load of a slot
    used more than once (0xE4D50E `movea.l (0x18,A6),A2'); passed on as
    `pea (-8,A6)' (0xE4D55E); a slot used once in the `if' condition
    before the first branch reuses A0 after the `(A0)+' (0xE4D51E).
  - vc_$clock: 6 bytes, `move.l (A0),(d,A6); move.w (4,A0),(d+4,A6)'
    (TONE_$TIME 0xE17300..0xE17308).
  - vc_$blocks: 14 bytes unrolled and 24 bytes in a `dbf' loop, the
    block copy of 5.2 from the slot (XPD_$SET_PTRACE_OPTS
    0xE5AFB0..0xE5AFBE; OS_$INIT 0xE33808). }
module vc;
type
  status_$t = integer32;
  uid_$t = record high, low: integer32; end;
  clock_t = record high: integer32; low: integer; end;
  opts_t = record a, b, c: integer32; d: integer; end;
  big_t = array[1..6] of integer32;
  int_ptr = ^integer;

procedure vc_$take(b: boolean; w: integer; l: integer32); options(val_param, extern);
procedure vc_$var(var w: integer); extern;
procedure vc_$ptr(in p: int_ptr); extern;
procedure vc_$uid_op(in u: uid_$t; out status: status_$t); extern;
procedure vc_$clock_op(in c: clock_t); extern;
procedure vc_$opts_op(in o: opts_t; in b: big_t); extern;

procedure vc_$scalars(b: boolean; w: integer; l: integer32);
begin
  vc_$take(b, w, l);
end;

procedure vc_$assign(w: integer; var out_w: integer);
begin
  w := w + 1;
  out_w := w;
  vc_$var(w);
  vc_$ptr(addr(w));
end;

procedure vc_$uid(u: uid_$t; in n: integer; out status: status_$t);
begin
  if n = 0 then
    status := 1
  else begin
    vc_$uid_op(u, status);
    if status <> 0 then
      vc_$uid_op(u, status);
  end;
end;

procedure vc_$clock(c: clock_t);
begin
  vc_$clock_op(c);
end;

procedure vc_$blocks(o: opts_t; b: big_t);
begin
  vc_$opts_op(o, b);
end;
