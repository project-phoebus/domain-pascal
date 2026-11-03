{ Golden: `addr(x)' of another unit's object compared (ref "Addr", p.
  4-13; codegen-patterns 5.2, "Comparisons"; bead domain-pascal-awzk;
  ../source pascal-translation-notes finding 665):
  - a variable against the address: the variable loaded into a data
    register, then the address as the compare's immediate, `cmp.l
    #sym,Dn' with the immediate-EA encoding `b0bc' + the register
    (FIM_$BUILD_DF 0xE0A58A `move.l (-6,A6),D2; cmp.l #GLOBAL_B,D2;
    bhi'; SAU5 0xE0A482, SAU8 0x3C0B228, 0x3C0B286), the branch unsigned
    for an unsigned type;
  - a pointer against a routine's address: `movea.l', then `cmpa.l
    #sym,An' (FIM_$BUILD_DF 0xE0A54C `movea.l (0x8).l,A3; cmpa.l
    #FIM_$COLD_BUS_ERR,A3; beq').
}
module addr_compare;
type
  uaddr = 0..16#7FFFFFFF;
  info_t = record
    pad: integer;
    addr: uaddr;
  end;
var
  global_b: extern integer32;
  bus_err_vec: extern univ_ptr;
  n: extern integer;
procedure cold_bus_err; extern;
procedure work; extern;
define build_df;
procedure build_df;
var
  info: info_t;
begin
  work;
  if info.addr > uaddr(addr(global_b)) then n := 1;
  if bus_err_vec = addr(cold_bus_err) then n := 2;
end;
