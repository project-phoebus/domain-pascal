{ Golden: codegen-patterns 12.4 R11, "The pad and the last entry load"
  (bead domain-pascal-dfjh): when the first statement is a call with a
  pad, the pad goes above the last parameter loaded at entry if the
  first push is a `move' to -(SP) that does not read that load's
  register (`tools/padsurvey.py rule': 46 sites, none against).

  - io_$device_interrupting: IO_$DEVICE_INTERRUPTING 0xE1A756 `subq.l
    #2,SP; move.w (8,A6),D2; move.l (10,A6),-(SP); move.w D2,-(SP)'
    (and the five other IO_ routines; AREA_$FREE_ASID 0xE07E8E with
    `move.w #14,-(SP)').
  - pmap_$purge_ws: PMAP_$PURGE_WS 0xE146C2 `move.w (8,A6),D2; subq.l
    #2,SP; move.b (10,A6),D3; move.w #20,-(SP)': only the last load.
  - tty_$k_flush_input: the push reads the load's register, the pad
    stays below it (TTY_$K_FLUSH_INPUT 0xE1C092 `movea.l (12,A6),A2;
    subq.l #2,SP; pea (A2)').
  - old_dir_lookup: the first push is a `pea', the pad stays below the
    load (module-local 0xE56A04, 0xE56A10 `movea.l (24,A6),A3; subq.l
    #2,SP; pea (-50,A6)').
  - pmap_$flush's nested flush_batch: the static link, PMAP_$FLUSH's
    nested 0xE1360C `subq.l #2,SP; movea.l (A6),A4; move.w
    #20,-(SP)'. }
module pad_entry_load;
type
  status_$t = integer32;

procedure ml_$lock(n: integer);
  options(extern, val_param);
procedure ml_$unlock(n: integer);
  options(extern, val_param);
function io_$check_unit(unit: integer; out status: status_$t): boolean;
  options(extern, val_param);
procedure mmap_$purge(n: integer);
  options(extern, val_param);
procedure tty_$i_flush_input(line: integer; out status: status_$t);
  options(extern, val_param);
function dir_$find(key: integer32; n: integer;
                   out a: integer32; out b: integer32): boolean;
  options(extern, val_param);

var
  ready: array[0..7] of integer;
  done: integer;

function io_$device_interrupting(unit: integer;
                                 out status: status_$t): boolean;
  options(val_param);
begin
  if io_$check_unit(unit, status) then
    io_$device_interrupting := ready[unit] <> 0;
end;

procedure pmap_$purge_ws(index: integer; flag: boolean);
  options(val_param);
begin
  ml_$lock(16#14);
  if flag then
    mmap_$purge(index)
  else
    ready[index] := 0;
  ml_$unlock(16#14);
end;

procedure tty_$k_flush_input(in line: integer; out status: status_$t);
begin
  tty_$i_flush_input(line, status);
  if status = 0 then
    done := 0;
end;

procedure old_dir_lookup(key: integer32; n: integer; x: integer;
                         out status: status_$t);
  options(val_param);
var
  a, b: integer32;
begin
  if dir_$find(key, n, a, b) then
    status := 0
  else
    status := a + b;
end;

procedure pmap_$flush(n: integer);
  options(val_param);
var
  count: integer;

  procedure flush_batch;
  begin
    ml_$lock(16#14);
    count := count + 1;
    done := count;
    ml_$unlock(16#14);
  end;

begin
  count := n;
  flush_batch;
  flush_batch;
end;
