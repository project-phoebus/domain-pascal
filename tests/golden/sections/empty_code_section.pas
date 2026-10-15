{ Golden: codegen-patterns 11 (named sections), 4 (one data base per data
  section); bead domain-pascal-zzru.
  - `module name, , data_section;' leaves the code section name empty:
    the code goes to the default `.text', the data to the named section
    (ref 7.1.1, p. 7-2, 2.2.1, p. 2-10; owner ruling 2026-10-15,
    reference-clarifications.md "Module heading with an empty code
    section").  ../source pchist/pchist.pas is `module pchist, ,
    wired_data;'.
  - A routine of the default code section loads the named data section's
    block into A5: PCHIST_$CNTL 0xE5CDBE `lea 0xE2C204,A5', the PCHIST
    WIRED_DATA segment (`D E2C204 PCHIST'). }
module pchist, , wired_data;
var
  lock: integer32;              { WIRED_DATA +0 }
  hits: integer32;              { +4 }

procedure pchist_$hits(out n: integer32);
begin
  n := hits;
end;
