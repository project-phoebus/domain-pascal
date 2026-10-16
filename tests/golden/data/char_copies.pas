{ Golden: codegen-patterns 5.2, "Arrays of char" (bead domain-pascal-40r2):
  an array of char is copied byte by byte, a record by longwords even
  when it holds an array of char.
  - read_tz: a 4-character array (Apollo's cal_$tz_name_t, `array [1..4]
    of char') into the unit's record, four `move.b (A0)+,(A1)+' after
    `lea (d,A6),A0; lea (2,A5),A1' (CAL_$READ_TIMEZONE 0xE3E592..0xE3E5A0
    `lea (0xe2,A4),A0; lea (0x2,A5),A1', from a held pointer's field);
  - write_tz: the record holding it (cal_$timezone_rec_t) by longwords,
    `move.l (A0)+,(A1)+' three times (CAL_$WRITE_TIMEZONE 0xE3E626..
    0xE3E62E), then the array alone by bytes (0xE3E68C..0xE3E69A);
  - copy_ids: 24 characters into a `with' record held in A0, `lea
    (A0),A2; moveq #0x17,D0; move.b (A1)+,(A2)+; dbf' (ACL_ 0xE45130..
    0xE45136; conformance unit acl_pci_copy_entry.pas);
  - get_name: another unit's 32 characters into a by-address parameter,
    `movea.l #sym,A0; movea.l (8,A6),A1; moveq #0x1f,D0' (the address
    forms of "Block addresses"; the count as REM_NAME_ 0xE4A634);
  - small: 8 and 10 characters in the frame, `moveq #7' and `moveq #9'
    loops (PROC1_$BIND 0xE14DB6, 0xE40C0C), each loop head aligned
    (codegen-patterns 7; 0xE12252, 0xE40C0E `movea.l A0,A0'). }
module charcopy;
type
  tz_name_t = array[1..4] of char;
  clock_t = record
    high: integer32;
    low: integer;
  end;
  { Apollo's cal_$timezone_rec_t (/sys/ins/cal.ins.pas, the m68k arm) }
  timezone_rec_t = packed record
    utc_delta: integer;
    tz_name: tz_name_t;
    drift: [aligned(1)] clock_t;
  end;
  ids_t = array[1..24] of char;
  entry_t = record
    ids: ids_t;
    reserved: integer32;
  end;
  name_t = array[1..32] of char;
  label_t = record
    utc_delta: integer;
    tz_name: tz_name_t;
  end;
var
  timezone: timezone_rec_t;
  names: extern name_t;

define read_tz, write_tz, copy_ids, get_name, small;

procedure read_tz(out n: integer);
var
  lbl: label_t;
begin
  n := 0;
  timezone.tz_name := lbl.tz_name;
end;

procedure write_tz(in tz_rec: timezone_rec_t; out lbl: label_t);
begin
  timezone := tz_rec;
  with lbl do begin
    utc_delta := timezone.utc_delta;
    tz_name := timezone.tz_name;
  end;
end;

procedure copy_ids(in src: entry_t; out dst: entry_t);
begin
  with dst do begin
    ids := src.ids;
    reserved := 0;
  end;
end;

procedure get_name(out name: name_t);
begin
  name := names;
end;

procedure small(out n: integer);
var
  a, b: array[1..8] of char;
  c, d: array[1..10] of char;
begin
  a := b;
  c := d;
  n := 0;
end;
