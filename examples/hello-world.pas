{ examples/hello-world.pas - Domain Pascal, compiled by dpc to run
  natively on an Apple-silicon Mac (docs/aarch64.md).  Without a field
  width, enumerations and booleans print in 15 columns (Apollo's default
  widths); `:1' prints them as they are.

  From the top of the repository, after `make build':

    target/debug/dpc --target=aarch64-apple-darwin -o tmp/hello-world.s examples/hello-world.pas
    clang -arch arm64 tmp/hello-world.s \
      $(target/debug/dpc --print-runtime=aarch64-apple-darwin) -o tmp/hello-world
    tmp/hello-world
}
program hello_world;

type
  colour = (red, green, blue);

var
  i: integer;
  total: integer32;
  c: colour;
  name: array [1..10] of char;

function factorial(n: integer): integer32;
begin
  if n <= 1 then
    factorial := 1
  else
    factorial := n * factorial(n - 1);
end;

begin
  writeln('Hello, world, from Domain Pascal!');

  total := 0;
  for i := 1 to 10 do
    total := total + i;
  writeln('1 + 2 + ... + 10 = ', total:1);

  writeln('10! = ', factorial(10):1);

  name := 'Apollo';
  c := blue;
  writeln(name:-1, ' likes ', c:1, ': ', (c = blue):1);
end.
