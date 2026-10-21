{ Execution test: a first call's pad placed above the last parameter
  loaded at entry (codegen-patterns 12.4 R11, "The pad and the last
  entry load"; bead domain-pascal-dfjh; golden calls/pad_entry_load):
  two by-value words, the second loaded after the pad (PMAP_$PURGE_WS
  0xE146C2); a by-value word loaded after the pad, the first push from
  another parameter's slot (IO_$DEVICE_INTERRUPTING 0xE1A756); two
  by-address slots, the second loaded after the pad; a nested routine's
  static link loaded after the pad (PMAP_$FLUSH's nested 0xE1360C); and
  a load the first push reads, which stays above the pad.  The loaded
  values must survive the call in their callee-saved registers.

  call: PAD_ENTRY_LOAD_$TWO w:4 w:7 -> w
  call: PAD_ENTRY_LOAD_$IO_LIKE w:21 w:-3
  call: PAD_ENTRY_LOAD_$SLOTS @w:5! @w:6!
  call: PAD_ENTRY_LOAD_$READS w:9 w:100
  call: PAD_ENTRY_LOAD_$OUTER w:40
  expect:
    mark 1
    => 47
    -3
    42
    mark 2
    out 16
    out 11
    100
    109
    mark 3
    mark 3
    42
}
module pad_entry_load;

%include 'exec.ins.pas';

function pad_entry_load_$two(a: integer; b: integer): integer;
  options(val_param);
begin
  exec_$mark(1);
  pad_entry_load_$two := a * 10 + b;
end;

procedure pad_entry_load_$io_like(u: integer; w: integer);
  options(val_param);
begin
  exec_$logv_w(w);
  exec_$logv_w(u * 2);
end;

procedure pad_entry_load_$slots(var x: integer; var y: integer);
begin
  exec_$mark(2);
  y := y + x;
  x := x + y;
end;

procedure pad_entry_load_$reads(a: integer; b: integer);
  options(val_param);
begin
  exec_$logv_w(b);
  exec_$logv_w(a + b);
end;

procedure pad_entry_load_$outer(n: integer);
  options(val_param);
var
  c: integer;

  procedure inner;
  begin
    exec_$mark(3);
    c := c + 1;
  end;

begin
  c := n;
  inner;
  inner;
  exec_$logv_w(c);
end;
