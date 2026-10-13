{ Golden: predeclared functions of chapter 4 (bead domain-pascal-0wc);
  codegen-patterns 8, 13.
  - folded to constants: `sizeof' of a type, a variable and a string
    (p. 4-181), `firstof'/`lastof' of enumerations, subranges, arrays,
    char, boolean, integer (pp. 4-87, 4-106), `succ'/`pred' of constants
    (pp. 4-191, 4-147), `max'/`min' (pp. 4-111, 4-113), `in_range'
    (p. 4-104); the stores are the constant stores of section 13;
  - `succ'/`pred' of a variable: `x + 1' / `x - 1', updated in place
    (`addq.w #1,abs.l', 0xE00B3E). }
module bi;
type
  color = (red, green, blue);
  small = -7..7;
  rec = record a: integer32; b: integer; c: char; end;
  vec = array[2..9] of integer;
const
  n = sizeof(rec);
var
  w: extern integer;
  l: extern integer32;
  c: extern color;
  ch: extern char;
  b: extern boolean;
  v: extern vec;

procedure consts;
begin
  w := sizeof(rec);
  w := sizeof(v);
  w := sizeof('wildebeest');
  w := n;
  l := sizeof(integer32);
  c := lastof(color);
  c := succ(red);
  c := pred(blue);
  w := firstof(small);
  w := lastof(small);
  w := firstof(v);
  w := lastof(vec);
  w := lastof(integer);
  l := lastof(integer32);
  ch := lastof(char);
  b := firstof(boolean);
  b := succ(false);
  w := max(3, 9);
  w := min(-3, 9);
  b := in_range(small(5));
end;

procedure computed;
begin
  w := succ(w);
  l := pred(l);
end;

procedure en;
begin
  c := succ(c);
end;
