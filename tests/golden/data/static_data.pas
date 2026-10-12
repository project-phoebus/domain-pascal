{ Golden: codegen-patterns 4 (module data off A5) and 13 (stores).
  - An exported routine that uses the unit's data saves A5 with
    `pea (A5)', loads it with the data area's address and restores it
    with `movea.l (-4,A6),A5' (PROC1_$GET_LOADAV 0xE14BBE..0xE14BD2);
    offset 0 is `(A5)', others `(d,A5)' (XNS_IDP_$GET_STATS 0xE18FE6,
    0xE18FE8); memory to memory from the data area through a by-address
    parameter (PROC2_$GET_BOOT_FLAGS 0xE41B62..0xE41B66).
  - An internal routine inherits A5 from its caller and does not load
    it (module-local 0xE17778 reads (0x14,A5)).
  - The data area: a NOBITS block `.bss.moddata.<UNIT>', 4-aligned, its
    size rounded to 4; a `define'd variable is a global symbol in it. }
module stat;
var
  count: integer32;          { 0 }
  flags: integer;            { 4 }
  level: integer;            { 6 }
define level;

procedure stat_$get_flags(out f: integer);
begin
  f := flags;
end;

procedure stat_$reset; options(val_param);
begin
  count := 0;
end;

procedure stat_$bump; internal;
begin
  level := 3;
end;
