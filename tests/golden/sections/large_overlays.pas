{ Golden: codegen-patterns 5.2, "Overlay sections of more than 0x8000
  bytes" (bead domain-pascal-7pcf).  Two sections laid out like the
  image's two such sections, ACL_$DATA (0xE88834, size 0xAD98: the image
  cache at +0, per-process tables from +0x7C00; ../source acl/acl.pas)
  and FILE_$LOT_DATA (0xE935CC: the lock table from entry 1 at +0, 0xC400
  bytes, the per-process tables after it):
  - whole variables and fields absolute, at any offset (FILE_$LOCK_INIT
    0xE327AC `clr.w (0xe9f9c4).l', FILE_$LOT_DATA+0xC3F8), by address
    `pea (sym+d).l';
  - an element of an array at the section's start from `movea.l #sym,An'
    (FILE_$CHECK_PROT 0xE5D1C8 `movea.l #0xe935cc,A1', `move.b
    (-2,A1,D0.l),D4'; ACL_$MIN_RIGHTS 0xE469B8 for ACL_$DATA's cache);
  - an array at 0x7C00 or beyond from the section's start plus 60000,
    `movea.l #sym+60000,An', its displacements less 60000 (ACL_$IN_SUBSYS
    0xE470AA `movea.l #0xe97294,A0', `(-15706,A1)' for the array at
    +0xAD08; ACL_$INIT 0xE3112A..0xE31132 a pointer stepped from that base
    in a `for' loop, `addq.l #8,A2', `(-0x4d60,A2)'; FILE_$EXPORT_LK
    0xE74200 `movea.l #0xea202c,A1'). }
module lovl;
type
  slot_t = record
    head: integer;
    rest: array[1..16#1FF] of integer;
  end;
  sid_t = record
    a, b: integer32;
    rest: array[1..14] of integer;
  end;
  entry_t = record
    next: integer;
    pad: array[1..12] of integer;
    kind: char;
    flag: boolean;
  end;
var (lovl_$data)
  cache: array[0..30] of slot_t;              { +0x0000 }
  original: array[1..64] of sid_t;            { +0x7C00 }
  current: array[1..64] of sid_t;             { +0x8500 }
  level: array[1..64] of integer;             { +0x8E00 }
var (lovl_$lot)
  lot: array[1..1792] of entry_t;             { +0x0000 }
  cell: integer;                              { +0xC400 }
define lovl_$a, lovl_$b, lovl_$c, lovl_$d;

procedure lovl_$use(var x: integer); extern;

procedure lovl_$a;
begin
  level[3] := 0;
  cell := 0;
  lovl_$use(level[1]);
  current[1].a := 0;
end;

function lovl_$b(in n: integer32): char;
begin
  lovl_$b := lot[n].kind;
end;

procedure lovl_$c(in n: integer);
var i: integer;
begin
  for i := 1 to 64 do
    level[i] := n;
end;

procedure lovl_$d(in n: integer32);
begin
  cache[n].head := 0;
end;
