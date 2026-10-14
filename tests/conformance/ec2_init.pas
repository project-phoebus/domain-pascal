{ EC2_$INIT, 0xE42C60, 0x1C bytes: 0xE42C60..0xE42C7C (tools/imgdis.sh
  E42C60 1C), the first routine of the EC2U module.  Semantics:
  ../source/domain_os/ec/ec2_init.c (zero the eventcount, but only when
  its address is above 1000: smaller values are EC2 indices, not
  addresses).  Declarations: the customer insert files (codegen-patterns
  0.1): `ec2_$init(OUT ec: ec2_$eventcount_t)' (ec2.ins.pas), and in
  base.ins.pas `ec2_$eventcount_t' (a [volatile] record, `value:
  [aligned(1)] integer32; awaiters: pinteger' for SR10 68k), `pinteger =
  0..65535' and `linteger = 0..2147483647'.

  Patterns exercised (docs/codegen-patterns.md):
  - 5, 7.1  a 4-byte non-negative subrange (`linteger') is unsigned: the
         compare with 1000 branches `bls' (0xE42C6E; bead
         domain-pascal-5j9);
  - 12.2 a by-address parameter whose address is also used as data is
         held in a data register (`move.l (8,A6),D0', 0xE42C64) and
         copied once to A0 for the two stores (0xE42C70..0xE42C74;
         bead domain-pascal-pze);
  - 2    `link.w A6,#-4' with a frame no instruction addresses.

  Reconstructed (not known from the original): the type transfer
  `linteger(addr(ec))' (the C compares the raw pointer value); and
  `unused_4', a stand-in for the 4 frame bytes no instruction addresses
  (as in tests/conformance/acl_clear_super.pas).  EC2_$READ (`link #-12',
  8 bytes used) and EC2_$ADVANCE (`link #-4', none used) have 4 such
  bytes too; a local held in a register (codegen-patterns 12.5), e.g. the
  address in D0/D2, would explain them, but that is not settled (bead
  domain-pascal-kwu). }
module ec2u;
type
  pinteger = 0..65535;
  linteger = 0..2147483647;
  ec2_$eventcount_t = [volatile] record
    value: [aligned(1)] integer32;
    awaiters: pinteger;
  end;

procedure ec2_$init(out ec: ec2_$eventcount_t);
var
  unused_4: integer32;                    { (-4,A6): stand-in }
begin
  if linteger(addr(ec)) > 1000 then begin
    ec.value := 0;
    ec.awaiters := 0;
  end;
end;
