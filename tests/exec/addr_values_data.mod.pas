{ The other unit of addr_values.pas: a record and a longword that
  addr_values.pas reaches as another unit's objects (map symbols, so
  their addresses are immediates, codegen-patterns 5.2 "`addr(x)` as a
  value"). }
module addrv_data;

type
  drv_t = record
    a: integer32;
    b: integer32;
    c: integer;
    d: integer32;
  end;
var
  addrv_$drv: drv_t;
  addrv_$long: integer32;
define addrv_$drv;
define addrv_$long;
