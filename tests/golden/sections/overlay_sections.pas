{ Golden: codegen-patterns 11, "Overlay sections" (bead domain-pascal-raxl).
  `var (name)' with a name that is none of the unit's data sections is an
  overlay section (ref 3.14, p. 3-52; 7.2.4, Method 4, p. 7-7), laid out
  like PCHIST_UNWIRED_D (0xE85718, size 0x938; ../source
  pchist/pchist.pas): its variables are offsets into the section's
  symbol, never through A5, as another unit's object:
  - whole variables absolute: PCHIST_$CNTL 0xE5CD1A `tst.w (0xe8604e).l',
    0xE5CD56 `clr.w (0xe8604e).l' (+0x936), a field 0xE5CDA8 `clr.b
    (0x00e85c26).l' (+0x50E);
  - by address, the section's start `move.l #sym,-(SP)' (XPD_$INIT
    0xE32334, XPD_$DATA), a variable after it `pea (sym+d).l' (PCHIST_$CNTL
    0xE5CEF6 +0x936, 0xE5CF00 +0x500);
  - an element of an overlay array from `movea.l #sym,An', the section's
    start, its offset in the displacement (PCHIST_$UNIX_PROFIL_FORK
    0xE5CC76 `movea.l #0xe85718,A0', `(-0x14,A0,D0.w)');
  - the unit's own block through A5 meanwhile (PCHIST_$CNTL 0xE5CF28
    `st (0x124,A5)');
  - the section a common block, `.comm SYM,size,4'; one with initial
    values defined by the unit; a routine's `var (name)' part static in
    that section. }
module ovl, , wired_data;
type
  proc_t = record
    buffer, bufsize, offset, scale, ovf: integer32;
  end;
  hist_t = record
    enabled: integer;
    doalign: boolean;
    pid_filter: integer;
    rest: array[1..16#212] of integer;
  end;
var
  enabled: boolean;
var (pchist_unwired_d)
  proc_data: array[1..64] of proc_t;     { +0x000 }
  wire_pages: array[1..3] of integer32;  { +0x500 }
  histogram: hist_t;                     { +0x50C }
  wired_count: integer;                  { +0x936 }
var (ovl_$init_sec)
  limit: integer := 3;
  base: integer32;
define ovl_$cntl, ovl_$count;

procedure ovl_$wire(var count: integer; var pages: integer32); extern;
procedure ovl_$lock(var p: proc_t); extern;

procedure ovl_$cntl;
var (ovl_$init_sec) calls: integer;
begin
  if wired_count <> 0 then
    wired_count := 0;
  histogram.doalign := false;
  ovl_$wire(wired_count, wire_pages[1]);
  ovl_$lock(proc_data[1]);
  enabled := true;
  calls := limit;
end;

procedure ovl_$count(in pid: integer);
begin
  proc_data[pid].scale := 0;
  base := 0;
end;
