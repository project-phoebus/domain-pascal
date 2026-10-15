{ Golden: codegen-patterns 5.2, "6 and 8 bytes" (bead domain-pascal-fbg).
  8 bytes: the source's address in an address register, post-incremented,
  the destination by displacement:
  - from the data area into an `out' parameter: the slot first, into A1,
    then `lea (d,A5),A0' (NAME_$GET_ROOT_UID 0xE589C8..0xE589D2);
  - from an `in' parameter into the data area: `movea.l (8,A6),A0;
    move.l (A0)+,(d,A5)' twice (0xE00BF0);
  - from the frame into an `out' parameter: the source first, `lea
    (-8,A6),A0; movea.l (8,A6),A1' (UID_$GEN 0xE1A0E2..0xE1A0EC);
  - from another unit's object: `movea.l #sym,A0' (0xE00F80, UID_$NIL).
  6 bytes: `move.l' then `move.w' four bytes on, memory to memory
  (TONE_$TIME 0xE17304; CAL_$SET_DRIFT 0xE68588).
  sc_$inq: the source through an integer local held in D0 and moved to
  A0 first, then the destination's slot, coloured after the source's
  register, then `lea (0x4c,A0),A1' (TTY_$K_INQ_PGROUP 0xE6796C..
  0xE67978, where A2 holds the status and the slot takes A3; that
  routine's status test makes dpc refuse the slot load, 12.4 R4(b)). }
module sc;
type
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
  clock_t = record
    high: integer32;
    low: integer;
  end;
  desc_t = record
    pad: array[0..37] of integer;
    pgroup_uid: uid_$t;
  end;
  desc_ptr_t = ^desc_t;
var
  node_uid: uid_$t;
  root_uid: uid_$t;
  now: clock_t;
  uid_$nil: extern uid_$t;

procedure sc_$get_root(out u: uid_$t);
begin
  u := root_uid;
end;

procedure sc_$set_node(in u: uid_$t);
begin
  node_uid := u;
end;

procedure sc_$local(out u: uid_$t);
var
  t: uid_$t;
begin
  t := node_uid;
  u := t;
end;

procedure sc_$nil(out u: uid_$t);
begin
  u := uid_$nil;
end;

procedure sc_$get_now(out c: clock_t);
begin
  c := now;
end;

procedure sc_$set_now(in c: clock_t);
begin
  now := c;
end;

function sc_$get_desc(line: integer): integer32;
  options(extern, val_param, a0_return);

procedure sc_$inq(in line: integer; out uid: uid_$t);
var
  tty: integer32;
begin
  tty := sc_$get_desc(line);
  uid := desc_ptr_t(tty)^.pgroup_uid;
end;
