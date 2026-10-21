{ codegen-patterns 2, "Objects of 8 bytes or more at multiples of 8"
  (bead domain-pascal-1edl): a frame object of 8 bytes or more lies at a
  multiple of 8 from the frame pointer, whatever the frame's total; an
  object of fewer bytes keeps its type's alignment.

  - file_$set_trouble: FILE_$SET_TROUBLE 0xE5DC42, a 0x34-byte attribute
    value at -0x38 under 4 bytes nothing addresses, `link #-0x38; st
    (-0x38,A6)' and `pea (-0x38,A6)' (the image passes more arguments);
  - file_$set_type: FILE_$SET_TYPE 0xE5E176, `link #-0x40': the same
    value at -0x38 and the callee's copy of the UID value parameter
    below it at -0x40 (`move.l (A0)+,(-0x40,A6); move.l (A0)+,
    (-0x3c,A6)', 0xE5E17A), copied into the value's first field (`lea
    (-0x40,A6),A0; move.l (A0)+,(-0x38,A6); ...', 0xE5E186);
  - i_wait: the module-local 0xE1C204 (tty_$i_wait, `link #-108'), an
    internal routine: its eventcount (12 bytes) at -0x10 below the
    reserved slot, the time queue element (26 bytes) at -0x30 and the two
    3-element EC_$WAITN lists at -0x40 and -0x50, where longword
    alignment gives -0x2C, -0x3C and -0x4C; the 6-byte clocks below
    longword placed (-0x58, -0x60); the image's 12 bytes below them
    hold other locals, left out here;
  - open: APP_$STD_OPEN 0xE00B92 (`link #-0x2c'), a 36-byte record at
    -0x28 under 4 bytes nothing addresses, a status below it at -0x2C. }
module big_objects;
type
  uid_$t = record high, low: integer32; end;
  value_t = record
    case integer of
      0: (raw: array[1..16#34] of char);
      1: (flag: boolean);
      2: (uid: uid_$t);
  end;
  ec_t = record value, awaiters, tick: integer32; end;
  qelem_t = record
    next: integer32;
    when: record high: integer32; low: integer; end;
    callback: integer32;
    arg: integer32;
    flags: integer32;
    pad1, pad2: integer;
  end;                                 { 26 bytes }
  list_t = array[1..3] of integer32;
  clock_t = record high: integer32; low: integer; end;
  open_t = record
    w: array[1..18] of integer;
  end;

procedure set_attribute(in id: integer; in value: value_t); extern;
procedure set_attribute_uid(in u: uid_$t; in id: integer; in value: value_t); extern;
procedure waitn(var ec: ec_t; var q: qelem_t; var l1, l2: list_t;
                var c1, c2: clock_t); extern;
procedure use_open(var o: open_t; var st: integer32); extern;

procedure file_$set_trouble;
var
  value: value_t;
begin
  value.flag := true;
  set_attribute(2, value);
end;

procedure file_$set_type(in file_uid: uid_$t; type_uid: uid_$t);
var
  value: value_t;
begin
  value.uid := type_uid;
  set_attribute_uid(file_uid, 4, value);
end;

procedure i_wait; internal;
var
  ec: ec_t;
  elem: qelem_t;
  list1: list_t;
  list2: list_t;
  clk1: clock_t;
  clk2: clock_t;
begin
  waitn(ec, elem, list1, list2, clk1, clk2);
end;

procedure open;
var
  opt: open_t;
  status: integer32;
begin
  use_open(opt, status);
end;
