{ Golden: codegen-patterns 5.1 "Values" and "Set comparisons" (bead
  domain-pascal-61fd): register-sized set operations as values.
  - TRY: `flags - [2] + [1]' as `moveq #-5,D0; and.w (84,A0),D0; ori.w
    #2,D0; move.w D0,(84,A0)' (PROC1_$TRY_TO_SUSPEND 0xE1474C..0xE14756,
    there through A2; SIO's 0xE1C89E..0xE1C8AC `moveq #-2,D0; ...; ori.w
    #16,D0' alike);
  - SIG: `(s - a) + b' on longwords, `move.l; not.l; and.l; or.l; move.l'
    (PROC2_$SET_SIG_MASK 0xE3F850..0xE3F85E);
  - OVF: an array element updated in place, `bclr #7' on the element's
    high byte (DIR_$OLD_FIND_OVERFLOW_SLOT 0xE5500A `bclr #7,(868,A4)'
    after its `lea'; dpc's index form is its own, with the member
    number unreduced as at the images' indexed sites, sau2
    MMAP_$FREE_WSL 0xE0D1B2 `bclr #15,(0x2c,A5,D0.w)'; bead
    domain-pascal-fozq);
  - SUSP: `k in f' with `f' held in a register, as a function result,
    `btst #1,D2; sne D0' (PROC1_$SUSPEND 0xE14842 `btst #1,D1; sne D0':
    the held register differs);
  best effort (not byte-identical, no site known):
  - UNION: `x + y' of two variables (the integer `!''s form) and `x +
    [3]' of a variable (loaded, then `ori');
  - CDIFF: `[0, 5] - x' (`not', then `andi');
  - SUB: subset `x <= y' as `x - y = []', superset as `y - x = []';
  - CLEFT: `[2] = x' as `x = [2]'. }
module set_values;
type
  w_t = set of 0..15;
  l_t = set of 0..31;
  pcb_t = record
    pad: array[1..42] of integer;
    flags: w_t;          { 84 }
  end;
  sig_t = record
    blocked: l_t;
    clear: l_t;
    setm: l_t;
  end;
  slot_t = record
    flags: w_t;
    pad: integer;
  end;
var
  slots: array[1..8] of slot_t;
  gw: w_t;
  gb: boolean;

procedure try(var pcb: pcb_t);
begin
  pcb.flags := pcb.flags - [2] + [1];
end;

procedure sig(var s: sig_t);
begin
  s.blocked := (s.blocked - s.clear) + s.setm;
end;

procedure ovf(j: integer); options(val_param);
begin
  slots[j].flags := slots[j].flags - [15];
end;

procedure clobber; extern;

function susp(in s: w_t): boolean;
var
  f: w_t;
begin
  f := s;
  clobber;
  susp := 1 in f;
end;

procedure union(in x, y: w_t);
begin
  gw := x + y;
  gw := x + [3];
end;

procedure cdiff(in x: w_t);
begin
  gw := [0, 5] - x;
end;

procedure sub(in x, y: w_t);
begin
  gb := x <= y;
  gb := x >= y;
end;

procedure cleft(in x: w_t);
begin
  gb := [2] = x;
end;
