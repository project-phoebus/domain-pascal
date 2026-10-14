{ Execution test: pointers and routine pointers (ref 3.13, 3.13.3;
  codegen-patterns 1.6): a procedure called through a pointer variable
  and through a record field (`jsr (An)'), by-value and by-address
  arguments through the pointer; a linked list walked through `^.next'.

  call: POINTERS_$INSTALL
  call: POINTERS_$CALL l:41
  call: POINTERS_$WALK -> w
  expect:
    mark 7
    42
    mark 7
    43
    true
    mark 30
    mark 20
    mark 10
    => 60
}
module pointers;
%include 'exec.ins.pas';

type
  plain_t = ^procedure (var n: integer32; in w: integer);
  vp_t = ^procedure (v: integer32; f: boolean); options(val_param);
  node_p = ^node;
  node = record
    val: integer;
    next: node_p;
  end;
  holder = record
    pad: integer;
    vp: vp_t;
  end;
var
  q: plain_t;
  h: holder;
  n1: node := [val := 10, next := nil];
  n2: node := [val := 20];
  n3: node := [val := 30];
  head: node_p;
  total: integer;

procedure handler(var n: integer32; in w: integer);
begin
  n := n + 1;
  exec_$mark(w);
end;

procedure vhandler(v: integer32; f: boolean); options(val_param);
begin
  exec_$logv_l(v);
  exec_$log_bool(f);
end;

procedure pointers_$install;
begin
  q := addr(handler);
  h.vp := addr(vhandler);
  n2.next := exec_$id_p(addr(n1));
  n3.next := exec_$id_p(addr(n2));
  head := exec_$id_p(addr(n3));
end;

procedure pointers_$call(a: integer32); options(val_param);
var
  n: integer32;
begin
  n := a;
  q^(n, 7);
  exec_$logv_l(n);
  q^(n, 7);
  h.vp^(n, true);
end;

function pointers_$walk: integer;
var
  p: node_p;
begin
  total := 0;
  p := head;
  while p <> nil do begin
    exec_$mark(p^.val);
    total := total + p^.val;
    p := p^.next;
  end;
  pointers_$walk := total;
end;
