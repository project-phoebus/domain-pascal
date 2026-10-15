{ Module-local routine at 0xE46498 (in the map's ACL_ module), 0x20
  bytes: 0xE46498..0xE464B8 (tools/imgdis.sh E46498 20).  Semantics:
  ../source/domain_os/acl/is_process_type_2.c, where it is called
  acl_$is_process_type_2 (a C name; the map does not name the routine):
  whether the process's type, PROC1_$TYPE[pid] (another unit's 1-based
  array of words), is other than 2.

  The unit is a fragment of ACL_, so tools/conform.py compiles it with
  `dpc --fragment-origin 0xE46498'.  No loops: no loop-alignment option.

  Patterns exercised (docs/codegen-patterns.md):
  - 1.2  internal: the word parameter by value at (8,A6);
  - 2    link.w A6,#-8 (the 4-byte slot of a routine that is not a
         global symbol, then the 1-byte result variable, rounded up);
  - 5.2  an element of another unit's array: `movea.l #PROC1_$TYPE,A0',
         the immediate the symbol and the lower bound in the
         displacement, `cmpi.w #2,(-2,A0,D1.w)'; the base loaded after
         the parameter and before its copy and scaling (bead
         domain-pascal-rlnm, `tools/boolsurvey.py ebase');
  - 12.2 the by-value parameter loaded into D0, then copied into D1 for
         the scaling `add.w D1,D1' (0xE4649C..0xE464A8);
  - 3    the comparison as a boolean result, `sne D1; move.b D1,D0'.

  Reconstruction notes: the routine is `internal' because the map does
  not name it; the array's bound (64) is a stand-in (any upper bound
  whose products fit a word index gives the same code); the C's int8_t
  result -1/0 is the boolean's representation (ref 3.5.3). }
module acl_;
var
  proc1_$type: extern array[1..64] of integer;

function acl_$is_process_type_2(pid: integer): boolean; internal;
begin
  acl_$is_process_type_2 := proc1_$type[pid] <> 2;
end;
