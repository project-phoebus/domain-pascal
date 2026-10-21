{ The other unit of univ_symbol_addr.pas: a record and a longword it
  reaches as another unit's objects (map symbols), whose addresses it
  passes by address. }
module usa_data;

type
  ctl_t = record
    csr: integer;
    mask: integer;
  end;
var
  usa_$ctl: ctl_t := [csr := 16#1234, mask := 16#5678];
  usa_$long: integer32 := 16#ABCDE;
define usa_$ctl;
define usa_$long;
