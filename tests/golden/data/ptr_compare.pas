{ codegen-patterns 7.1 ("Two variables in memory"); bead
  domain-pascal-jqp0.  A pointer held in an address register against a
  variable: `cmpa.l M,An' (SAU9 PROC1_$SET_VT 0x3C416C7A `cmpa.l
  PROC1_$CURRENT_PCB,A2'; the instruction of SAU2's 20 `movea.l M1,An;
  cmpa.l M2,An' sites, 0xE01A7E).  The local here is assigned from a
  pointer function (section 3, "Pointer locals"). }
module ptr_compare;
type
  pcb_t = record vtimer: integer32; end;
  pcb_ptr_t = ^pcb_t;
var
  current_pcb: pcb_ptr_t;
function find_pcb(in n: integer): pcb_ptr_t; extern;
procedure stop_timer(in n: integer); extern;
procedure p_$set(in n: integer);
var
  pcb: pcb_ptr_t;
begin
  pcb := find_pcb(n);
  if pcb = current_pcb then
    stop_timer(n);
  pcb^.vtimer := 0;
end;
