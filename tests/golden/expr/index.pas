{ Golden: codegen-patterns 5.2 (array elements).
  - an array in the unit's data with a word index read from memory:
    scaled in place (`add.w Dn,Dn' for 2-byte elements,
    AUDIT_$IS_PROCESS_AUDITED 0xE70DA6; `lsl.w #2' for 4, FIM_$GET_FIM_ADDR
    0xE0AA16), the lower bound folded into an 8-bit displacement,
    `(-2,A5,D0.w)', `(0x3c,A5,D0.w)';
  - a displacement beyond 8 bits: `lea (0,A5,Dn.w),An' and `(d,An)'
    (BAT_$GET_BAT_STEP 0xE3BB6E);
  - a by-value parameter as the index: loaded, copied, scaled (0xE3BB64..
    0xE3BB6A); one loaded at entry copied (FIM_$FREE_ASID 0xE0AA8A);
  - a store to an element (FIM_$FREE_ASID 0xE0AA8E `clr.l
    (0x3c,A5,D0.w)'); an element passed by address, `pea (d,An,Dn.w)'
    (DISK_$REVALIDATE 0xE6C07A);
  - a frame array, `(d,A6,Dn.w)'; a constant index folded into the
    displacement; a longword index, `.l'. }
module index;
type
  rec = record
    a: integer32;
    b: integer;
    pad: array[1..45] of integer;
  end;
var
  words: array[1..64] of integer;
  longs: array[0..15] of integer32;
  recs: array[1..4] of rec;
  i: extern integer;
  j: extern integer32;
  w: extern integer;
procedure p(var x: integer); options(extern);
function word_elem: integer; internal;
begin
  word_elem := words[i];
end;
function long_elem: integer32; internal;
begin
  long_elem := longs[i];
end;
function rec_elem(k: integer): integer; internal;
begin
  rec_elem := recs[k].b;
end;
function const_elem: integer; internal;
begin
  const_elem := words[3];
end;
function long_index: integer32; internal;
begin
  long_index := longs[j];
end;
function frame_elem: integer; internal;
var
  local: array[1..4] of integer;
begin
  frame_elem := local[i];
end;
procedure stores(k: integer); internal;
begin
  w := k;
  longs[k] := 0;
end;
procedure by_address; internal;
begin
  p(words[i]);
end;
