{ codegen-patterns 5.3 (packed records, ref 3.10.7), 9, 3, 12.2.
  status_$t as Apollo's base.ins.pas declares it (codegen-patterns 0.1,
  9): `fail' bit 7 of the first byte, `code' the low word.

  - fork_fail: `if status.code <> 0 then status.fail := true', `tst.w
    (2,A0); beq; bset #7,(A0)' (PROC2_$FORK 0xE72CC2..0xE72CC8);
  - normalise: after a call that sets the status, `status.fail :=
    status.all <> status_$ok', the status held in A2 across the call,
    `tst.l (A2); sne D0; andi.b #127,(A2); andi.b #-128,D0; or.b D0,(A2)'
    (OS_$GET_EC 0xE6D6CA..0xE6D6EA);
  - inherit: a boolean field as a function result, `btst #3,(13,A0); sne
    D0' (XPD_$INHERIT_PTRACE_OPTIONS 0xE5B17C);
  - route_state / set_state / clear_state: a 2-bit enumeration field at
    the top of a byte, read `move.w #192,D0; and.b (16,A0),D0; lsr.w
    #6,D0' and set `andi.b #63,(16,A0); ori.b #-128,(16,A0)', cleared
    with the `andi' alone (RIP 0xE158B6..0xE158D4, 0xE1565E);
  - set_depth: a `0..255' variable (a word: its low byte is read) into
    an 8-bit field across the bytes 0x27 and 0x28, a longword unit,
    `move.b ...,D0; andi.l #255,D0; andi.l #-0x7f8001,(38,A0); lsl.l
    #8,D0; lsl.l #7,D0; or.l D0,(38,A0)' (APP_$RECEIVE 0xE00864..0xE0087A,
    `move.b (21,A4),D0'). }
module packed_fields;
const
  status_$ok = 0;
type
  status_$t = packed record case integer of
      1:  (fail:  boolean;
           subsys: 0..127;
           modc: 0..255;
           code: integer);
      2:  (all: integer32);
      end;
  opts_t = packed record
    signal_mask: integer32;
    trace_range_lo: integer32;
    trace_range_hi: integer32;
    flags: char;
    f7, f6, f5, f4: boolean;
    inherit: boolean;
    f2, f1, f0: boolean;
  end;
  state_t = (invalid, valid, aging, spare);
  route_t = packed record
    pad: array[1..8] of integer;
    state: state_t;
    rest: 0..63;
  end;
  binteger = 0..255;
  rec_t = packed record
    pad: array[1..19] of integer;     { 0x00..0x25 }
    x: binteger;                      { 0x26 }
    b: boolean;                       { 0x27, bit 7 }
    depth: binteger;                  { 0x27..0x28, bits 22..15 of 0x26 }
    low: 0..32767;
  end;
var
  queue_depth: extern binteger;

procedure ext_$op(out status: status_$t); extern;

procedure fork_fail(out status: status_$t);
begin
  if status.code <> 0 then status.fail := true;
end;

procedure normalise(out status: status_$t);
begin
  ext_$op(status);
  status.fail := status.all <> status_$ok;
end;

function inherit(var opts: opts_t): boolean;
begin
  inherit := opts.inherit;
end;

function route_state(var r: route_t): integer;
begin
  route_state := ord(r.state);
end;

procedure set_state(var r: route_t);
begin
  r.state := aging;
end;

procedure clear_state(var r: route_t);
begin
  r.state := invalid;
end;

procedure set_depth(var r: rec_t);
begin
  r.depth := queue_depth;
end;
