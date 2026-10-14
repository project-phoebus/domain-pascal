{ Golden: codegen-patterns 5 ("Signedness"), 7.1, 8 (bead
  domain-pascal-5j9).  A non-negative subrange stored in 4 bytes
  (`linteger = 0..2147483647', base.ins.pas) is unsigned (ref 3.4 p. 3-9;
  4.3.3 pp. 4-6..4-7; "Max", p. 4-111):
  - compared with a constant: `cmpi.l; bls' (EC2_$INIT 0xE42C68);
  - two of them: `move.l; cmp.l; bhi' (PMAP_$PURIFIER_L 0xE13EB0..
    0xE13EB8, two working-set counts);
  - against an integer32 or an integer: signed (ref 4.3.3), the word
    widened with `ext.l' (Uncertain: no image site identified);
  - `div' by a word constant: `M$DIU$LLW' (CAL_$DECODE_TIME 0xE68794);
    of two: `M$DIU$LLL' and `M$OIU$LLL' (AST_$GET_SEG_MAP 0xE06E16,
    0xE06E26); by a power of two: `lsr.l' (AST_$GET_SEG_MAP 0xE06E2E
    `lsr.l #8; lsr.l #2'); `*': `M$MIU$LLL'
    (PCHIST_$UNIX_PROFIL_ADDUPC 0xE5D008);
  - the type transfers of ref 4.3.3's example: `u_type(s32) DIV u32'
    unsigned, `s32 DIV integer32(u32)' signed;
  - `u := u - 1' (a signed `-', ref 4.3.3 not listing it) updated in
    place, `subq.l #1,<ea>', as for an integer32 (codegen-patterns 8:
    1076 `addq'/`subq' to memory in compiled code, e.g. 0xE00B3E). }
module ulong;
type
  linteger = 0..2147483647;
var
  u, v: extern linteger;
  l: extern integer32;
  w: extern integer;
  c: extern integer;

procedure compares;
begin
  if u > 1000 then c := 1;
  if u <= v then c := 2;
  if l < u then c := 3;
  if w < u then c := 4;
end;

procedure arith;
begin
  u := v div 60;
  u := v div u;
  u := v mod u;
  u := v div 1024;
  u := v * u;
  u := linteger(l) div v;
  l := l div integer32(v);
  u := u - 1;
end;
