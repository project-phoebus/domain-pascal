{ Constants for reaching definitions (codegen-patterns 7.1; bead
  domain-pascal-pvy8; ref 6.4.24 pp. 6-27..6-28, `-opt' 1 to 3), with
  `dpc --substitute-constants', a per-unit setting.  The shape of SAU3
  PBU_$SETUP 0xE594E0 (../source finding 607): `bus', `first' and `last'
  have one constant definition reaching the rest, so the tests of `bus'
  fold and the parts they skip go (0xE595AA..0xE595D2), `req.bus := bus'
  stores the constant (0xE596E4), `case bus' selects on `moveq #2' with
  its table kept (0xE596F0), the `for' counts from constants with
  `first' and `last' stored nowhere (0xE595D8), and the packed field
  store reads `bus' (0xE597D4), so its store stays (0xE5957C). }
module const_subst;
type
  req_t = record tag: integer; bus: integer; vec: integer end;
  info_t = packed record bus: 0..7; rest: 0..31 end;
var
  present: boolean;
  csr: integer32;
  n: integer;
  r: req_t;
  info: info_t;
  st: integer32;
procedure work(i: integer); extern;
define p;
procedure p(unit: integer);
label done;
var bus: 0..7; first, last, sub, vector: integer;
begin
  if not present then begin st := 1; goto done; end;
  bus := 2;
  first := 0;
  last := 7;
  if bus = 1 then csr := 16#C000;
  if bus = 2 then csr := csr div 1024 + 16#1C0;
  for sub := first to last do begin
    if sub = unit then goto done;
    work(sub);
    r.tag := 3;
    r.bus := bus;
    case bus of 1, 2, 3, 4: r.vec := sub; end;
    info.bus := bus;
  end;
  if bus = 1 then n := n + 1;
done:
  st := 0;
end;
