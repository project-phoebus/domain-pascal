{ Golden: codegen-patterns 11 (named sections), 4 (one data base per data
  section), 1.6 (calls between code sections).
  - The module heading names the code and data sections (ref 7.1.1,
    p. 7-2); `[section(c,d)]' moves a routine (ref 5.7.1, p. 5-24).  One
    output section per Pascal section: `.text.<UNIT>.<SECTION>' and the
    NOBITS `.bss.moddata.<UNIT>.<SECTION>', routines grouped by code
    section in body order, each code section with its own module-end
    padding (SCSI_ 0xE34F12 and 0xE88802).
  - A routine's static data is in its data section (ref 5.7.1), whose
    block its A5 points at: `lea (.Ldata.<SECTION>).l,A5' (the image's
    one A5 value per segment, PROC2 0xE7BE84 in .DATA vs. 0xE86054 in
    PROC2_CREATE_DAT).
  - A call into another code section is `jsr abs.l', from a statement or
    from an expression (PROC1_$INIT 0xE2F9EC `jsr 0xE14B12'); an internal
    routine so called by a routine of another data section loads its own
    A5 (0xE14B12 `lea 0xE254E8,A5'), while a call within the section is a
    `bsr' to a worker that inherits A5 (0xE17778). }
module sect, wired_proc, wired_data;
var
  hits: integer32;              { WIRED_DATA +0 }

procedure sect_$count; internal;
begin
  hits := hits + 1;
end;

function sect_$hits: integer32; internal;
begin
  sect_$hits := hits;
end;

procedure sect_$tick; options(val_param);
begin
  sect_$count;
end;

[section(init_proc, init_data)]
procedure sect_$init;
var
  boots: static integer32;      { INIT_DATA +0 }
begin
  sect_$count;
  boots := sect_$hits;
end;
