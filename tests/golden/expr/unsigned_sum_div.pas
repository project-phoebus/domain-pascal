{ Golden: codegen-patterns 8 ("The sign"); ../source finding 787.  A
  `div' of an unsigned sum or product (of unsigned values and
  non-negative constants) is unsigned, though the sum's language type is
  a signed integer (Uncertain: 9 sites, 2 against, see codegen-patterns):
  - an unsigned sum divided: `M$DIU' (PMAP_$PURIFIER_L 0xE1404E..
    0xE1405E `(page_count + 11) div 12', `M$DIU$LLW'; here by a longword
    constant, as the word divisor's pad scheduled into a computed
    operand is best effort, codegen-patterns 12.4 R9);
  - an unsigned product divided: `M$DIU' (MST_$INIT 0xE30C9E..0xE30CAC
    `(pages * 10) div 100');
  - `(word + 3) div 4' of a `0..65535' word: `lsr.l' (DXM_$ADD_CALLBACK);
  - a `mod' keeps its operands' own types: signed for an unsigned sum
    (SMD_$LOC_EVENT 0xE6E9EC..0xE6E9FE `(head + 255) mod 256',
    `M$OIS$WLW'). }
module unsigned_sum_div;
type
  linteger = 0..2147483647;
  uword = 0..65535;
var
  count: extern linteger;
  pages: extern linteger;
  size: extern uword;
  head: extern uword;
  q: extern linteger;
  n: extern uword;
procedure purify;
begin
  q := (count + 11) div 100000;
end;
procedure ten_percent;
begin
  q := (pages * 10) div 100000;
end;
procedure words;
begin
  n := (size + 3) div 4;
end;
procedure previous;
begin
  q := (head + 255) mod 100000;
end;
