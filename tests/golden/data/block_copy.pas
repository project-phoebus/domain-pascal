{ Golden: codegen-patterns 5.2, record and array assignment as a block
  copy with `move.l (A0)+,(A1)+', source address first:
  - 12 bytes from the data area to a by-address parameter: `lea (A5),A0;
    movea.l (8,A6),A1' and three moves (PROC1_$GET_LOADAV 0xE14BC6);
  - 14 bytes, a final word with `move.w (A0)+,(A1)+' (0xE419B0..0xE419BE),
    into the data area at `lea (d,A5),A1';
  - 144 bytes as a dbf loop: count after both addresses, loop head
    4-aligned with the 0x2048 filler (AST_ 0xE01494..0xE014A2; not
    every loop in the image is aligned, codegen-patterns 5.2);
  - 22 bytes: the loop and a final word (0xE19DA2..0xE19DAA). }
module blk;
type
  avg_t = array[1..3] of integer32;
  r14 = record
    a: array[1..3] of integer32;
    b: integer;
  end;
  r144 = array[1..36] of integer32;
  r22 = record
    a: array[1..5] of integer32;
    b: integer;
  end;
var
  avg: avg_t;
  small: r14;
  big: r144;
  mid: r22;

procedure blk_$get_avg(out x: avg_t);
begin
  x := avg;
end;

procedure blk_$put_small(in x: r14);
begin
  small := x;
end;

procedure blk_$get_big(out x: r144);
begin
  x := big;
end;

procedure blk_$get_mid(out x: r22);
begin
  x := mid;
end;
