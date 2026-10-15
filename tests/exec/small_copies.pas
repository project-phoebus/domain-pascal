{ Execution test: 6- and 8-byte record assignments (codegen-patterns
  5.2, "6 and 8 bytes"; bead domain-pascal-fbg).  8 bytes: the source
  post-incremented, the destination by displacement (data area to an
  `out' parameter, `in' parameter to the data area, frame to frame,
  into a field through a pointer); 6 bytes: `move.l' and `move.w' memory
  to memory.  Expected values from ref "Assignment" (p. 4-25): the whole
  record is copied, the bytes around it untouched.

  call: SC_$MAIN w:3
  expect:
    mem 00 00 00 03 ff ff ff fd
    mem 00 00 00 03 ff ff ff fd
    mem 11 11 11 11 00 00 00 03 ff ff ff fd 22 22 22 22
    mem 00 00 00 03 ff ff ff fd
    mem 00 00 00 03 00 07
    mem 33 33 00 00 00 03 00 07 44 44
    3
}
module sc;
%include 'exec.ins.pas';

type
  uid_$t = record
    high: integer32;
    low: integer32;
  end;
  clock_t = record
    high: integer32;
    low: integer;
  end;
  holder = record
    before: integer32;
    u: uid_$t;
    after: integer32;
  end;
  holderp = ^holder;
  wrapped = record
    a: integer;
    c: clock_t;
    b: integer;
  end;
var
  root: uid_$t;
  node: uid_$t;
  h: holder := [before := 16#11111111, after := 16#22222222];
  w: wrapped := [a := 16#3333, b := 16#4444];

procedure get_root(out u: uid_$t);
begin
  u := root;
end;

procedure set_node(in u: uid_$t);
begin
  node := u;
end;

procedure put_clock(out c: clock_t; in d: clock_t);
begin
  c := d;
end;

procedure sc_$main(k: integer); options(val_param);
var
  t, t2: uid_$t;
  p: holderp;
  c, c2: clock_t;
begin
  root.high := k;
  root.low := -k;
  get_root(t);
  exec_$log_mem(t, 8);
  set_node(t);
  exec_$log_mem(node, 8);
  p := exec_$id_p(addr(h));
  p^.u := node;
  exec_$log_mem(h, 16);
  t2 := p^.u;
  exec_$log_mem(t2, 8);
  c.high := k;
  c.low := 7;
  put_clock(c2, c);
  exec_$log_mem(c2, 6);
  w.c := c2;
  exec_$log_mem(w, 10);
  exec_$logv_l(t2.high);
end;
