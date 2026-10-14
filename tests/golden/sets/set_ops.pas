{ Golden: codegen-patterns 5.1 (set operators on register-sized sets,
  `[]', the member test of a set larger than a longword), 8 (`&' of a
  held value and a constant `moveq' cannot load).  Mirrors the TTY
  routines of the SR10.2 image:
  - DRAINED: `s := s - [0]' on a word set, `bclr #0' on its low byte
    (TTY_$I_OUTPUT_BUFFER_DRAINED 0xE1B39E);
  - SET_FLAG: `s + [k]' / `s - [k]' with a run-time member of a longword
    set, `clr.l; bset Dk,Dn; or.l' and `...; not.l; and.l' (TTY_$K_SET_
    INPUT_FLAG 0xE6768E..0xE676A2);
  - SEVERAL: `- [5, 6, 7]' as `andi.w #-0xe1' (TTY_$I_RCV 0xE1B98E); one
    member of a longword set on its byte; `bset #7' (0xE1B68E);
  - MASKS: `x * y' with the right operand first (0xE1BF92), `s - y'
    (0xE1BF9E), `s + y' (0xE1C034), `:= []' (0xE1C044);
  - SAVED: `s + (t * [7])' with t held, copy and `andi' then `or' into
    the set (TTY_$I_RCV 0xE1BACE);
  - ANY_OF: `s * [0, 1, 2] <> []' on the flags of the `and' (0xE1BFFE);
  - PENDING: `s <> []' as `tst.w' (TTY_$K_GET 0xE1C5D4);
  - IN_CONST: `k in [c..d]', the constant in a register (TTY_$K_SET_DELAY
    0xE67A18..0xE67A1E);
  - SEP: `ch in s' for a 32-byte `set of char' (the TTY module-local
    0xE1B740..0xE1B754: `clr.w; move.w #0xff; move.b; lea; sub.w;
    lsr.w #3; btst'). }
module set_ops;
type
  flag_t = (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11, f12, f13, f14, f15,
            f16, f17, f18, f19, f20, f21, f22, f23, f24, f25, f26, f27, f28, f29, f30, f31);
  flags_t = set of flag_t;
  w_t = set of 0..15;
  desc = record
    pad: array[1..4] of integer;
    state: w_t;          { 8 }
    pend: w_t;           { 10 }
    pad2: integer;
    pad3: integer;
    inf: flags_t;        { 16 }
    saved: flags_t;      { 20 }
  end;
var
  mask: extern flags_t;
  seps: set of char;
  buf: array[1..10] of char;
procedure drained(in out t: desc);
begin
  t.state := t.state - [0];
end;
procedure set_flag(in out t: desc; in flag: flag_t; in value: boolean);
begin
  if value then
    t.inf := t.inf + [flag]
  else
    t.inf := t.inf - [flag];
end;
procedure several(in out t: desc);
begin
  t.state := t.state - [5, 6, 7];
  t.inf := t.inf + [f9];
  t.state := t.state + [7];
end;
procedure masks(in out t: desc);
begin
  t.saved := t.inf * mask;
  t.inf := t.inf - mask;
  t.inf := t.inf + t.saved;
  t.saved := [];
end;
procedure saved(var t: desc; s: w_t); options(val_param);
begin
  t.saved := [];
  t.state := t.state + (s * [7]);
end;
function any_of(in out t: desc): boolean;
begin
  any_of := false;
  if t.state * [0, 1, 2] <> [] then any_of := true;
end;
function pending(in out t: desc): boolean;
begin
  pending := false;
  if t.pend <> [] then pending := true;
end;
function in_const(in d: flag_t): boolean;
begin
  in_const := false;
  if not (d in [f0..f4]) then in_const := true;
end;
function sep(i: integer): boolean; options(val_param);
begin
  sep := false;
  if buf[i] in seps then sep := true;
end;
